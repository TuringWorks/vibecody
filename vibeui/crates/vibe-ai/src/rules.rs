//! Rules directory system for `.vibecli/rules/` path-aware injections.
//!
//! Each `.md` file in the rules directory may include optional YAML-style front-matter:
//! ```text
//! ---
//! name: rust-safety
//! path_pattern: "**/*.rs"
//! ---
//! When editing Rust files, always check for unwrap() calls...
//! ```
//!
//! Rules without a `path_pattern` (or with `path_pattern: "*"`) always inject.
//! Rules with a pattern only inject when the open files list includes a matching path.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// ── Rule ──────────────────────────────────────────────────────────────────────

/// A single rule loaded from a `.md` file in the rules directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Display name (from front-matter or derived from file name).
    pub name: String,
    /// Optional glob that must match at least one open file for this rule to apply.
    /// `None` or `"*"` means always apply.
    pub path_pattern: Option<String>,
    /// The rule body (everything after the front-matter block).
    pub content: String,
    /// Source path.
    pub source: PathBuf,
}

impl Rule {
    /// Returns true if this rule applies given the currently open files.
    pub fn matches_open_files(&self, open_files: &[String]) -> bool {
        let pattern = match &self.path_pattern {
            None => return true,
            Some(p) if p == "*" || p.is_empty() => return true,
            Some(p) => p.as_str(),
        };
        open_files.iter().any(|f| glob_match(pattern, f))
    }
}

// ── RulesLoader ───────────────────────────────────────────────────────────────

/// Loads all rule files from a directory.
pub struct RulesLoader;

impl RulesLoader {
    /// Load all `.md` rule files from `dir`. Returns empty vec if dir doesn't exist.
    pub fn load(dir: &Path) -> Vec<Rule> {
        if !dir.is_dir() {
            return vec![];
        }
        WalkDir::new(dir)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file()
                    && e.path().extension().and_then(|x| x.to_str()) == Some("md")
            })
            .filter_map(|e| Self::parse_file(e.path()))
            .collect()
    }

    /// Load rules from workspace and global directories, deduplicated by name.
    pub fn load_for_workspace(workspace_root: &Path) -> Vec<Rule> {
        let mut rules = Self::load(&workspace_root.join(".vibecli").join("rules"));
        // Global rules (lower priority, skip names already seen)
        if let Ok(home) = std::env::var("HOME") {
            let global_dir = PathBuf::from(home).join(".vibecli").join("rules");
            let seen: std::collections::HashSet<String> =
                rules.iter().map(|r| r.name.clone()).collect();
            for r in Self::load(&global_dir) {
                if !seen.contains(&r.name) {
                    rules.push(r);
                }
            }
        }
        rules
    }

    fn parse_file(path: &Path) -> Option<Rule> {
        let raw = std::fs::read_to_string(path).ok()?;
        let name_default = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("rule")
            .to_string();

        // Parse optional YAML front-matter (--- ... ---)
        if raw.starts_with("---") {
            // Find closing ---
            let after_open = &raw[3..];
            let after_open = after_open.trim_start_matches('\n');
            if let Some(close_pos) = after_open.find("\n---") {
                let fm = &after_open[..close_pos];
                let body = after_open[close_pos..].trim_start_matches("\n---").trim_start().to_string();
                // Simple key: value front-matter parser (no serde_yaml needed)
                let mut name: Option<String> = None;
                let mut path_pattern: Option<String> = None;
                for line in fm.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        let key = k.trim();
                        let val = v.trim().trim_matches('"').trim_matches('\'').to_string();
                        match key {
                            "name" => name = Some(val),
                            "path_pattern" => path_pattern = Some(val),
                            _ => {}
                        }
                    }
                }
                return Some(Rule {
                    name: name.unwrap_or(name_default),
                    path_pattern,
                    content: body,
                    source: path.to_path_buf(),
                });
            }
        }

        // No front-matter — treat entire file as content
        Some(Rule {
            name: name_default,
            path_pattern: None,
            content: raw,
            source: path.to_path_buf(),
        })
    }
}

// ── Glob helper ───────────────────────────────────────────────────────────────

fn glob_match(pattern: &str, path: &str) -> bool {
    glob_match_impl(pattern.as_bytes(), path.as_bytes())
}

fn glob_match_impl(pat: &[u8], text: &[u8]) -> bool {
    let mut pi = 0usize;
    let mut ti = 0usize;
    let mut star_pi = usize::MAX;
    let mut star_ti = 0usize;

    while ti < text.len() {
        if pi < pat.len() && (pat[pi] == b'?' || pat[pi] == text[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < pat.len() && pat[pi] == b'*' {
            if pi + 1 < pat.len() && pat[pi + 1] == b'*' {
                star_pi = pi;
                star_ti = ti;
                pi += 2;
                if pi < pat.len() && pat[pi] == b'/' {
                    pi += 1;
                }
            } else {
                star_pi = pi;
                star_ti = ti;
                pi += 1;
            }
        } else if star_pi != usize::MAX {
            pi = star_pi + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }
    while pi < pat.len() && (pat[pi] == b'*' || pat[pi] == b'/') {
        pi += 1;
    }
    pi == pat.len()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_no_pattern_always_matches() {
        let rule = Rule {
            name: "always".into(),
            path_pattern: None,
            content: "be safe".into(),
            source: PathBuf::from("always.md"),
        };
        assert!(rule.matches_open_files(&[]));
        assert!(rule.matches_open_files(&["src/main.rs".into()]));
    }

    #[test]
    fn rule_pattern_matches_open_file() {
        let rule = Rule {
            name: "rust".into(),
            path_pattern: Some("**/*.rs".into()),
            content: "no unwrap".into(),
            source: PathBuf::from("rust.md"),
        };
        assert!(!rule.matches_open_files(&["src/main.ts".into()]));
        assert!(rule.matches_open_files(&["src/main.rs".into()]));
        assert!(rule.matches_open_files(&[
            "src/main.ts".into(),
            "lib/utils.rs".into(),
        ]));
    }

    #[test]
    fn wildcard_star_matches_all() {
        let rule = Rule {
            name: "all".into(),
            path_pattern: Some("*".into()),
            content: "be safe".into(),
            source: PathBuf::from("all.md"),
        };
        assert!(rule.matches_open_files(&["anything.ts".into()]));
    }
}
