//! TS / Python intra-procedural taint scanner — regex-based v1.
//!
//! Tracks data flow from **sources** (user-controlled values:
//! `req.body[*]`, `req.query[*]`, `process.argv`, `input()`,
//! `request.args[*]`, `sys.argv`, …) into **sinks** (dangerous
//! operations: `child_process.exec`, `os.system`, `subprocess`
//! with `shell=True`, `eval`, `fs.readFile`, `open()`,
//! `db.query` + string concat, `innerHTML =`, etc.). When a
//! tainted value reaches a sink without an intervening
//! **sanitizer**, emit a `SecurityFinding`.
//!
//! ## Algorithm (v1)
//!
//! Regex-based heuristic with two flow patterns:
//!
//! 1. **Direct sink-with-source**: same line contains a sink
//!    call AND a source expression as an argument. Catches the
//!    most common Express-style bug:
//!    ```js
//!    res.send(req.body.userInput);  // direct, obvious
//!    ```
//!
//! 2. **Variable taint within function body**: scan top-down,
//!    track local variables that get assigned from a source:
//!    ```js
//!    function handler(req, res) {
//!      const path = req.body.path;      // line N — taints `path`
//!      const x = sanitize(path);        // line N+1 — `x` is clean
//!      fs.readFile(path, cb);           // line N+2 — finding! `path` still tainted
//!      fs.readFile(x, cb);              // line N+3 — clean (sanitized)
//!    }
//!    ```
//!    Function boundaries reset the tainted-variable set so a
//!    helper function's `path` parameter isn't conflated with
//!    the caller's. This is **intra-procedural** — flow through
//!    function calls is out of scope.
//!
//! ## What this misses (documented limitations)
//!
//! - **Inter-procedural flow** — a source that passes through a
//!   helper before reaching a sink isn't tracked. The
//!   "suspicious parameter name" heuristic (rule 2 in
//!   scanners.md §6) partially mitigates.
//! - **Object property tracking** — `obj.x = tainted; sink(obj.y)`
//!   is treated as `obj` being tainted as a whole.
//! - **Indirect taint via array methods** — `arr.map(req.body.fn)`
//!   isn't recognised.
//! - **Template literals and string concatenation** are matched
//!   conservatively: any concat with a source-shaped expression
//!   taints the whole resulting string.
//!
//! A proper tree-sitter implementation would lift all of these.
//! For v1 the heuristic catches the most common bugs without
//! adding three new workspace deps (tree-sitter,
//! tree-sitter-typescript, tree-sitter-python) with C
//! compilation. Documented in `scanners.md` §6.
//!
//! ## Threat-model invariant
//!
//! The snippet emitted in `SecurityFinding.snippet` carries the
//! sink-line text — that's source code, not runtime user input,
//! so payload-bytes-leak isn't a concern here. The snippet is
//! still bounded by `SecurityFinding::new`'s
//! `SNIPPET_MAX_BYTES` cap.

use crate::security_posture::{Category, Scanner, SecurityFinding, Severity};
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

pub struct TaintScanner;

impl Scanner for TaintScanner {
    fn name(&self) -> &'static str {
        "taint"
    }

    fn scan(&self, workspace: &Path) -> Result<Vec<SecurityFinding>> {
        let mut findings = Vec::new();

        for entry in walkdir::WalkDir::new(workspace)
            .max_depth(8)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();

            // Skip noise dirs (matching the other scanners).
            let skip = path.ancestors().any(|a| {
                a.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| {
                        matches!(
                            n,
                            "node_modules"
                                | "target"
                                | ".git"
                                | "vendor"
                                | ".venv"
                                | "venv"
                                | "__pycache__"
                                | "dist"
                                | "build"
                                | ".next"
                        )
                    })
                    .unwrap_or(false)
            });
            if skip {
                continue;
            }

            let lang = match path.extension().and_then(|e| e.to_str()) {
                Some("ts") | Some("tsx") | Some("js") | Some("jsx") | Some("mjs") | Some("cjs") => {
                    Language::Js
                }
                Some("py") => Language::Python,
                _ => continue,
            };

            let content = match std::fs::read_to_string(path) {
                Ok(c) if c.len() < 524_288 => c, // 512 KiB cap — taint scan is more expensive
                _ => continue,
            };
            let rel = path.strip_prefix(workspace).unwrap_or(path);
            scan_file(&content, rel, lang, &mut findings);
        }

        Ok(findings)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    Js,
    Python,
}

/// Sink classifier — drives both the regex and the severity /
/// category of the emitted finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SinkKind {
    CommandInjection,
    PathTraversal,
    SqlInjection,
    DomXss,
    CodeInjection,
}

impl SinkKind {
    fn severity(self) -> Severity {
        match self {
            // Code / command / SQL injection = remote shell-equivalent.
            SinkKind::CodeInjection | SinkKind::CommandInjection | SinkKind::SqlInjection => {
                Severity::Critical
            }
            // Path traversal & DOM XSS still High — direct exploit
            // primitives, just narrower blast radius than RCE.
            SinkKind::PathTraversal | SinkKind::DomXss => Severity::High,
        }
    }

    fn category(self) -> Category {
        match self {
            SinkKind::PathTraversal => Category::PathTraversal,
            SinkKind::CommandInjection => Category::Other("command_injection".into()),
            SinkKind::SqlInjection => Category::Other("sql_injection".into()),
            SinkKind::DomXss => Category::Other("dom_xss".into()),
            SinkKind::CodeInjection => Category::Other("code_injection".into()),
        }
    }

    fn rule_id(self, lang: Language) -> String {
        let lang_tag = match lang {
            Language::Js => "js",
            Language::Python => "py",
        };
        let sink_tag = match self {
            SinkKind::CommandInjection => "command-injection",
            SinkKind::PathTraversal => "path-traversal",
            SinkKind::SqlInjection => "sql-injection",
            SinkKind::DomXss => "dom-xss",
            SinkKind::CodeInjection => "code-injection",
        };
        format!("taint:{lang_tag}:{sink_tag}")
    }

    fn title(self) -> &'static str {
        match self {
            SinkKind::CommandInjection => "Tainted value reaches command-execution sink",
            SinkKind::PathTraversal => "Tainted value reaches filesystem-path sink",
            SinkKind::SqlInjection => "Tainted value reaches SQL-query sink",
            SinkKind::DomXss => "Tainted value reaches DOM sink (potential XSS)",
            SinkKind::CodeInjection => "Tainted value reaches dynamic-code-execution sink",
        }
    }

    fn cwe_ref(self) -> &'static str {
        match self {
            SinkKind::CommandInjection => "https://cwe.mitre.org/data/definitions/78.html",
            SinkKind::PathTraversal => "https://cwe.mitre.org/data/definitions/22.html",
            SinkKind::SqlInjection => "https://cwe.mitre.org/data/definitions/89.html",
            SinkKind::DomXss => "https://cwe.mitre.org/data/definitions/79.html",
            SinkKind::CodeInjection => "https://cwe.mitre.org/data/definitions/94.html",
        }
    }

    fn remediation(self) -> &'static str {
        match self {
            SinkKind::CommandInjection => {
                "Use the array form of `spawn` (never `exec` with a shell), or shell-escape the \
                 argument with a library function. Never interpolate user input into a shell \
                 command string."
            }
            SinkKind::PathTraversal => {
                "Canonicalize the path and assert it stays inside an allowed root \
                 (`path.resolve(root, untrusted)` then check `startsWith(root)`). Reject `..` \
                 and absolute paths before opening."
            }
            SinkKind::SqlInjection => {
                "Use parameterized queries (`?` / `$1` / `:name` placeholders). Never concatenate \
                 user input into a SQL string."
            }
            SinkKind::DomXss => {
                "Use `textContent` instead of `innerHTML`, or run the value through a sanitizer \
                 (DOMPurify with an allow-list). Avoid `dangerouslySetInnerHTML` with user input."
            }
            SinkKind::CodeInjection => {
                "Never `eval`/`Function()`/`exec()` user input. Refactor to a data-driven \
                 dispatch table or a safe parser for the expected input shape."
            }
        }
    }
}

// ── Regex sets ───────────────────────────────────────────────────────

/// Patterns that produce tainted values. Each is searched
/// case-sensitively on a line; a match anywhere in the line means
/// the line contains a source.
fn source_patterns(lang: Language) -> &'static [regex::Regex] {
    static JS: OnceLock<Vec<regex::Regex>> = OnceLock::new();
    static PY: OnceLock<Vec<regex::Regex>> = OnceLock::new();
    match lang {
        Language::Js => JS.get_or_init(|| {
            [
                r"\breq\.(body|query|params|headers|cookies)\b",
                r"\brequest\.(body|query|params|headers|cookies)\b",
                r"\bprocess\.argv\b",
                r"\bprocess\.env\b",
                r"\bJSON\.parse\s*\(\s*req\.",
                r"\bJSON\.parse\s*\(\s*request\.",
                r"\bctx\.(request|params|query|body)\b",
            ]
            .iter()
            .filter_map(|p| regex::Regex::new(p).ok())
            .collect()
        }),
        Language::Python => PY.get_or_init(|| {
            [
                r"\brequest\.(args|form|json|values|data|files|headers|cookies)\b",
                r"\bsys\.argv\b",
                r"\bos\.environ\b",
                r"\binput\s*\(",
                r"\bFlask\.request\.",
                r"\bflask\.request\.",
            ]
            .iter()
            .filter_map(|p| regex::Regex::new(p).ok())
            .collect()
        }),
    }
}

/// Sink patterns paired with the `SinkKind` that classifies them.
fn sink_patterns(lang: Language) -> &'static [(regex::Regex, SinkKind)] {
    static JS: OnceLock<Vec<(regex::Regex, SinkKind)>> = OnceLock::new();
    static PY: OnceLock<Vec<(regex::Regex, SinkKind)>> = OnceLock::new();
    match lang {
        Language::Js => JS.get_or_init(|| {
            [
                // Command injection — `child_process.exec(`, `execSync(`,
                // and shell-running spawn variants.
                (r"\bchild_process\.exec(?:Sync)?\s*\(", SinkKind::CommandInjection),
                (r"\.exec(?:Sync)?\s*\(", SinkKind::CommandInjection),
                (r"\bspawn(?:Sync)?\s*\([^,]*shell\s*:\s*true", SinkKind::CommandInjection),

                // Path traversal — file ops + path-joining a tainted root.
                (r"\bfs\.(readFile|readFileSync|writeFile|writeFileSync|appendFile|unlink|rmdir|rm|stat|open|createReadStream|createWriteStream)\s*\(", SinkKind::PathTraversal),
                (r"\bfsPromises\.(readFile|writeFile|appendFile|unlink|stat|open)\s*\(", SinkKind::PathTraversal),

                // SQL — string-concatenated query.
                (r#"\.(query|exec|run|all|get|prepare)\s*\(\s*[`'"].*\$\{"#, SinkKind::SqlInjection),
                (r#"\.(query|exec|run|all|get|prepare)\s*\(\s*[`'"][^`'"]*[`'"]\s*\+"#, SinkKind::SqlInjection),

                // DOM XSS.
                (r"\.innerHTML\s*=", SinkKind::DomXss),
                (r"\.outerHTML\s*=", SinkKind::DomXss),
                (r"\bdocument\.write\s*\(", SinkKind::DomXss),
                (r"\bdangerouslySetInnerHTML\b", SinkKind::DomXss),
                (r"\.insertAdjacentHTML\s*\(", SinkKind::DomXss),

                // Code injection.
                (r"\beval\s*\(", SinkKind::CodeInjection),
                (r"\bnew\s+Function\s*\(", SinkKind::CodeInjection),
                (r"\bvm\.runInNewContext\s*\(", SinkKind::CodeInjection),
            ]
            .iter()
            .filter_map(|(p, k)| regex::Regex::new(p).ok().map(|r| (r, *k)))
            .collect()
        }),
        Language::Python => PY.get_or_init(|| {
            [
                // Command injection.
                (r"\bos\.system\s*\(", SinkKind::CommandInjection),
                (r"\bos\.popen\s*\(", SinkKind::CommandInjection),
                (r"\bsubprocess\.(call|run|check_call|check_output|Popen)\s*\([^)]*shell\s*=\s*True", SinkKind::CommandInjection),
                (r"\bcommands\.getoutput\s*\(", SinkKind::CommandInjection),

                // Path traversal — open / read / write to a string path.
                (r"\bopen\s*\(", SinkKind::PathTraversal),
                (r"\bpathlib\.Path\s*\([^)]+\)\s*\.(read_text|write_text|read_bytes|write_bytes)", SinkKind::PathTraversal),
                (r"\bshutil\.(copy|move|rmtree)\s*\(", SinkKind::PathTraversal),

                // SQL — f-string / %-formatted query.
                (r#"\.(execute|executemany)\s*\(\s*f[`'"]"#, SinkKind::SqlInjection),
                (r#"\.(execute|executemany)\s*\([^)]*%\s*[a-zA-Z_]"#, SinkKind::SqlInjection),
                (r#"\.(execute|executemany)\s*\(\s*['"][^'"]*['"]\s*\+"#, SinkKind::SqlInjection),

                // Code injection.
                (r"\beval\s*\(", SinkKind::CodeInjection),
                (r"\bexec\s*\(", SinkKind::CodeInjection),
                (r"\bcompile\s*\(", SinkKind::CodeInjection),
                (r"\b__import__\s*\(", SinkKind::CodeInjection),

                // Pickle deserialization — well-known code-execution sink.
                (r"\bpickle\.loads?\s*\(", SinkKind::CodeInjection),
                (r"\byaml\.load\s*\([^)]*\)", SinkKind::CodeInjection),
                (r"\bmarshal\.loads?\s*\(", SinkKind::CodeInjection),
            ]
            .iter()
            .filter_map(|(p, k)| regex::Regex::new(p).ok().map(|r| (r, *k)))
            .collect()
        }),
    }
}

/// Patterns that detoxify a tainted value. If a line contains one of
/// these AND a tainted variable name, the variable is removed from
/// the tainted set.
fn sanitizer_patterns(lang: Language) -> &'static [regex::Regex] {
    static JS: OnceLock<Vec<regex::Regex>> = OnceLock::new();
    static PY: OnceLock<Vec<regex::Regex>> = OnceLock::new();
    match lang {
        Language::Js => JS.get_or_init(|| {
            [
                r"\bencodeURIComponent\s*\(",
                r"\bencodeURI\s*\(",
                r"\bDOMPurify\.sanitize\s*\(",
                r"\bvalidator\.(escape|trim|isLength|isAlphanumeric)\s*\(",
                r"\bpath\.normalize\s*\(",
                r"\bpath\.resolve\s*\(",
                // Parameterized query placeholders — if the call has
                // `?` / `$1` / `:name` style placeholders the sink
                // regex below doesn't match anyway, but document the
                // safe-pattern here for the variable-flow analysis.
                r"\.(prepare|preparedStatement)\s*\(",
            ]
            .iter()
            .filter_map(|p| regex::Regex::new(p).ok())
            .collect()
        }),
        Language::Python => PY.get_or_init(|| {
            [
                r"\bhtml\.escape\s*\(",
                r"\bquote\s*\(", // urllib.parse.quote
                r"\bquote_plus\s*\(",
                r"\bshlex\.quote\s*\(",
                r"\bos\.path\.normpath\s*\(",
                r"\bos\.path\.realpath\s*\(",
                r"\bre\.escape\s*\(",
                r"\bbleach\.clean\s*\(",
            ]
            .iter()
            .filter_map(|p| regex::Regex::new(p).ok())
            .collect()
        }),
    }
}

/// Matches a variable assignment whose RHS contains a source.
/// Returns the LHS variable name on match. Conservative — only
/// handles simple `const x = …` / `let x = …` / `var x = …` /
/// `x = …` forms on a single line. Multi-line / destructured /
/// default-valued assignments fall through (the direct-sink rule
/// still catches them if the source appears in the sink call).
fn assignment_var_name(line: &str, lang: Language) -> Option<String> {
    static JS_ASSIGN: OnceLock<regex::Regex> = OnceLock::new();
    static PY_ASSIGN: OnceLock<regex::Regex> = OnceLock::new();
    let re = match lang {
        Language::Js => JS_ASSIGN.get_or_init(|| {
            regex::Regex::new(r"(?:const|let|var)\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*=")
                .expect("hardcoded JS assign regex compiles")
        }),
        Language::Python => PY_ASSIGN.get_or_init(|| {
            regex::Regex::new(r"^\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*=[^=]")
                .expect("hardcoded Py assign regex compiles")
        }),
    };
    re.captures(line)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// `function name(...)`, `(...)  =>`, `const foo = (...) =>`, `class M {`,
/// Python `def foo(...):`, `class M:` — heuristic function/scope boundary.
fn looks_like_function_boundary(line: &str, lang: Language) -> bool {
    let t = line.trim();
    match lang {
        Language::Js => {
            t.starts_with("function ")
                || t.starts_with("async function ")
                || t.starts_with("class ")
                || t.contains("=> {")
                || t.contains("function(")
                || t.contains("function (")
        }
        Language::Python => {
            t.starts_with("def ") || t.starts_with("async def ") || t.starts_with("class ")
        }
    }
}

// ── Per-file scan ────────────────────────────────────────────────────

fn scan_file(content: &str, file: &Path, lang: Language, findings: &mut Vec<SecurityFinding>) {
    let sinks = sink_patterns(lang);
    let sources = source_patterns(lang);
    let sanitizers = sanitizer_patterns(lang);

    // Tainted-variable set, reset at each function boundary so a
    // helper's `path` parameter doesn't inherit the caller's taint.
    let mut tainted_vars: HashSet<String> = HashSet::new();
    // Dedup: don't emit two findings for the same (file, line, sink_kind).
    let mut emitted: HashSet<(u32, &'static str)> = HashSet::new();

    for (line_idx, line) in content.lines().enumerate() {
        if line.contains("nosectaint:") {
            continue;
        }
        let line_no = (line_idx + 1) as u32;

        // Function boundary → reset.
        if looks_like_function_boundary(line, lang) {
            tainted_vars.clear();
        }

        let line_has_source = sources.iter().any(|re| re.is_match(line));
        let line_has_sanitizer = sanitizers.iter().any(|re| re.is_match(line));

        // ── Rule 1: variable assignment from source → taint the LHS ──
        if line_has_source {
            if let Some(var) = assignment_var_name(line, lang) {
                tainted_vars.insert(var);
            }
        }

        // ── Sanitizer assignment → clean the LHS ──
        if line_has_sanitizer {
            if let Some(var) = assignment_var_name(line, lang) {
                tainted_vars.remove(&var);
            }
        }

        // ── Sink scan ──
        for (re, kind) in sinks {
            if !re.is_match(line) {
                continue;
            }
            // Skip if a sanitizer wraps the sink argument on the same
            // line — `fs.readFile(path.normalize(p), …)` should not
            // emit a finding for `p`.
            if line_has_sanitizer {
                continue;
            }

            // Rule 1: direct sink with literal source on the same line.
            let direct_source = line_has_source;

            // Rule 2: sink with a tainted variable name in its
            // argument list.
            let tainted_var_in_sink = tainted_vars
                .iter()
                .any(|v| line_contains_variable_use(line, v));

            if !direct_source && !tainted_var_in_sink {
                continue;
            }

            let kind_tag = match kind {
                SinkKind::CommandInjection => "command_injection",
                SinkKind::PathTraversal => "path_traversal",
                SinkKind::SqlInjection => "sql_injection",
                SinkKind::DomXss => "dom_xss",
                SinkKind::CodeInjection => "code_injection",
            };
            if !emitted.insert((line_no, kind_tag)) {
                continue;
            }

            findings.push(SecurityFinding::new(
                "taint",
                kind.severity(),
                kind.category(),
                file.to_path_buf(),
                Some(line_no),
                None,
                Some(line.trim().to_string()),
                kind.rule_id(lang),
                kind.title(),
                Some(kind.remediation().to_string()),
                vec![kind.cwe_ref().to_string()],
            ));
        }
    }
}

/// Heuristic: does `line` contain `var` as an identifier (not as a
/// substring of a longer identifier)? Uses simple word-boundary
/// matching — not a real lexer, but good enough to suppress the
/// `path` ↔ `pathname` false-positive class.
fn line_contains_variable_use(line: &str, var: &str) -> bool {
    if var.is_empty() {
        return false;
    }
    let mut start = 0usize;
    while let Some(pos) = line[start..].find(var) {
        let abs = start + pos;
        let before_ok = abs == 0
            || !line.as_bytes()[abs - 1].is_ascii_alphanumeric()
                && line.as_bytes()[abs - 1] != b'_';
        let after = abs + var.len();
        let after_ok = after >= line.len()
            || !line.as_bytes()[after].is_ascii_alphanumeric() && line.as_bytes()[after] != b'_';
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan_js(src: &str) -> Vec<SecurityFinding> {
        let mut f = Vec::new();
        scan_file(src, Path::new("test.ts"), Language::Js, &mut f);
        f
    }

    fn scan_py(src: &str) -> Vec<SecurityFinding> {
        let mut f = Vec::new();
        scan_file(src, Path::new("test.py"), Language::Python, &mut f);
        f
    }

    // ── Rule 1: direct sink+source on same line ──

    #[test]
    fn js_eval_with_req_body_critical() {
        let f = scan_js("eval(req.body.code);");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Critical);
        assert!(f[0].rule_id.contains("code-injection"));
    }

    #[test]
    fn js_fs_readfile_with_req_query() {
        let f = scan_js("fs.readFile(req.query.path, cb);");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::High);
        assert!(f[0].rule_id.contains("path-traversal"));
    }

    #[test]
    fn js_innerHTML_with_req_body() {
        let f = scan_js("el.innerHTML = req.body.html;");
        assert_eq!(f.len(), 1);
        assert!(f[0].rule_id.contains("dom-xss"));
    }

    #[test]
    fn py_os_system_with_request_args() {
        let f = scan_py("os.system(request.args['cmd'])");
        assert_eq!(f.len(), 1);
        assert!(f[0].rule_id.contains("command-injection"));
    }

    #[test]
    fn py_open_with_request_args_path_traversal() {
        let f = scan_py("open(request.args['p'], 'r')");
        assert_eq!(f.len(), 1);
        assert!(f[0].rule_id.contains("path-traversal"));
    }

    #[test]
    fn py_pickle_loads_with_request() {
        let f = scan_py("pickle.loads(request.data)");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Critical);
        assert!(f[0].rule_id.contains("code-injection"));
    }

    // ── Rule 2: variable taint within function body ──

    #[test]
    fn js_taint_propagates_through_local_var() {
        let src = r#"
function handler(req, res) {
  const path = req.body.path;
  fs.readFile(path, cb);
}
"#;
        let f = scan_js(src);
        assert!(
            f.iter().any(|f| f.rule_id.contains("path-traversal")),
            "expected path-traversal finding via tainted var, got: {:?}",
            f.iter().map(|f| &f.rule_id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn js_sanitizer_clears_taint() {
        let src = r#"
function handler(req, res) {
  const raw = req.body.path;
  const safe = path.normalize(raw);
  fs.readFile(safe, cb);
}
"#;
        let f = scan_js(src);
        // The `raw` variable is tainted but never reaches a sink.
        // The `safe` variable is sanitized. No finding should fire
        // on the fs.readFile line.
        let read_findings: Vec<_> = f
            .iter()
            .filter(|f| f.title.contains("filesystem-path") && f.line == Some(5))
            .collect();
        assert!(
            read_findings.is_empty(),
            "sanitizer should have cleared `safe`, no finding expected: got {read_findings:?}"
        );
    }

    #[test]
    fn js_function_boundary_resets_taint() {
        let src = r#"
function tainted_caller(req) {
  const x = req.body.x;
}
function unrelated_helper(x) {
  fs.readFile(x, cb);
}
"#;
        let f = scan_js(src);
        // The `x` parameter in unrelated_helper is a different `x`
        // — function boundary should have cleared the caller's
        // tainted set, so no finding for the readFile line.
        assert!(
            !f.iter()
                .any(|f| f.line == Some(6) && f.rule_id.contains("path-traversal")),
            "function boundary should isolate taint, got: {f:?}"
        );
    }

    #[test]
    fn py_taint_propagates_through_local() {
        let src = r#"
def handler():
    p = request.args['path']
    open(p, 'r')
"#;
        let f = scan_py(src);
        assert!(
            f.iter().any(|f| f.rule_id.contains("path-traversal")),
            "expected path-traversal via tainted var, got: {:?}",
            f.iter().map(|f| &f.rule_id).collect::<Vec<_>>()
        );
    }

    // ── Negative cases ──

    #[test]
    fn js_no_finding_for_clean_code() {
        let f = scan_js("const x = 42; console.log(x);");
        assert!(f.is_empty());
    }

    #[test]
    fn js_no_finding_for_eval_of_literal() {
        // eval of a literal is still bad practice but no taint.
        let f = scan_js(r#"eval("1 + 1");"#);
        assert!(
            f.is_empty(),
            "literal eval has no source, no taint, got: {f:?}"
        );
    }

    #[test]
    fn js_nosectaint_skips_line() {
        let f = scan_js("eval(req.body.code); // nosectaint: known-safe test fixture");
        assert!(f.is_empty(), "nosectaint should suppress the finding");
    }

    #[test]
    fn dedup_same_line_same_sink_kind() {
        // A line with TWO matching sink regexes of the same kind
        // shouldn't double-emit.
        let f = scan_js(r#"eval(req.body.x); /* eval again */"#);
        let code_injection_count = f
            .iter()
            .filter(|f| f.rule_id.contains("code-injection"))
            .count();
        assert!(
            code_injection_count <= 1,
            "dedup expected, got {code_injection_count}"
        );
    }

    // ── Variable-use word-boundary ──

    #[test]
    fn line_contains_variable_use_word_boundary() {
        assert!(line_contains_variable_use("foo(path)", "path"));
        assert!(line_contains_variable_use("path", "path"));
        assert!(line_contains_variable_use("  path  ", "path"));
        // "pathname" should NOT match "path".
        assert!(!line_contains_variable_use("foo(pathname)", "path"));
        assert!(!line_contains_variable_use("_path", "path"));
        assert!(!line_contains_variable_use("path1", "path"));
    }

    #[test]
    fn empty_var_never_matches() {
        assert!(!line_contains_variable_use("anything", ""));
    }

    // ── Scanner name + sink/severity stability ──

    #[test]
    fn scanner_name_stable() {
        assert_eq!(TaintScanner.name(), "taint");
    }

    #[test]
    fn sink_kind_severity_mapping() {
        assert_eq!(SinkKind::CodeInjection.severity(), Severity::Critical);
        assert_eq!(SinkKind::CommandInjection.severity(), Severity::Critical);
        assert_eq!(SinkKind::SqlInjection.severity(), Severity::Critical);
        assert_eq!(SinkKind::PathTraversal.severity(), Severity::High);
        assert_eq!(SinkKind::DomXss.severity(), Severity::High);
    }

    #[test]
    fn rule_id_includes_language_and_sink_tags() {
        assert_eq!(
            SinkKind::PathTraversal.rule_id(Language::Js),
            "taint:js:path-traversal"
        );
        assert_eq!(
            SinkKind::CodeInjection.rule_id(Language::Python),
            "taint:py:code-injection"
        );
    }
}
