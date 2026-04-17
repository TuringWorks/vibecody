//! SonarQube-compatible rule engine with local SQLite rule store.
//!
//! Embeds a representative subset of SonarSource rules (TypeScript, JavaScript,
//! Rust/general) and provides line-level issue detection, matching SonarQube's
//! issue shape: rule key, severity, type, message, why, how-to-fix, effort.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Types ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonarRule {
    pub key: String,
    pub name: String,
    pub description: String,
    pub why: String,
    pub how_to_fix: String,
    pub severity: String,    // BLOCKER | CRITICAL | MAJOR | MINOR | INFO
    pub issue_type: String,  // BUG | VULNERABILITY | CODE_SMELL | SECURITY_HOTSPOT
    pub language: String,
    pub tags: Vec<String>,
    pub effort_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonarIssue {
    pub rule_key: String,
    pub rule_name: String,
    pub file: String,
    pub line: u32,
    pub end_line: u32,
    pub col_start: u32,
    pub message: String,
    pub severity: String,
    pub issue_type: String,
    pub code_snippet: String,
    pub context_before: String,
    pub context_after: String,
    pub why: String,
    pub how_to_fix: String,
    pub effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonarScanResult {
    pub file: String,
    pub issues: Vec<SonarIssue>,
    pub bugs: u32,
    pub vulnerabilities: u32,
    pub code_smells: u32,
    pub security_hotspots: u32,
    pub debt_minutes: u32,
}

// ── Embedded Rule Definitions ──────────────────────────────────────────────────

pub fn builtin_rules() -> Vec<SonarRule> {
    vec![
        // ── SECURITY / VULNERABILITY ────────────────────────────────────────
        SonarRule {
            key: "typescript:S2068".into(),
            name: "Credentials should not be hard-coded".into(),
            description: "Hard-coded credentials in source code are a critical security risk. Anyone with access to the code — including version control history — can extract them.".into(),
            why: "Credentials committed to source control are permanently accessible, even after deletion, via git history. They can be leaked via public repositories, log aggregators, or error messages.".into(),
            how_to_fix: "Use environment variables (`process.env.MY_SECRET`), a secrets manager (AWS Secrets Manager, HashiCorp Vault), or a `.env` file that is excluded from version control.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "typescript".into(),
            tags: vec!["cwe-798".into(), "owasp-a07".into(), "sans-top25".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "typescript:S5332".into(),
            name: "Using http:// is insecure — use https://".into(),
            description: "Cleartext HTTP transmits data without encryption, making it vulnerable to man-in-the-middle attacks, eavesdropping, and data tampering.".into(),
            why: "HTTP traffic can be intercepted by any actor on the network path. Sensitive data (tokens, user info, session cookies) becomes exposed.".into(),
            how_to_fix: "Replace `http://` with `https://`. Ensure your server has a valid TLS certificate. Use HSTS headers to enforce HTTPS.".into(),
            severity: "CRITICAL".into(),
            issue_type: "VULNERABILITY".into(),
            language: "typescript".into(),
            tags: vec!["cwe-319".into(), "owasp-a02".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "typescript:S3649".into(),
            name: "Database queries should not be vulnerable to injection attacks".into(),
            description: "SQL injection occurs when user-controlled data is concatenated directly into a query string, letting attackers alter query logic.".into(),
            why: "An attacker can bypass authentication, dump the entire database, modify records, or execute OS-level commands depending on the database engine.".into(),
            how_to_fix: "Use parameterized queries or prepared statements. For ORMs, use the ORM's built-in query builder. Never concatenate user input into SQL strings.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "typescript".into(),
            tags: vec!["cwe-89".into(), "owasp-a03".into(), "sans-top25".into()],
            effort_minutes: 30,
        },
        SonarRule {
            key: "typescript:S5042".into(),
            name: "Expanding archive files without controlling resource consumption is security-sensitive".into(),
            description: "Uncontrolled archive extraction (zip bomb) can exhaust disk space or memory.".into(),
            why: "A maliciously crafted archive can expand to terabytes, causing a denial-of-service condition on the server.".into(),
            how_to_fix: "Limit the total uncompressed size and the number of entries before extracting. Reject archives that exceed thresholds.".into(),
            severity: "CRITICAL".into(),
            issue_type: "SECURITY_HOTSPOT".into(),
            language: "typescript".into(),
            tags: vec!["cwe-409".into()],
            effort_minutes: 15,
        },
        SonarRule {
            key: "typescript:S6096".into(),
            name: "Cross-site scripting (XSS) — unsanitized data rendered as HTML".into(),
            description: "`innerHTML`, `document.write`, or `dangerouslySetInnerHTML` with user-controlled input allows attackers to inject malicious scripts.".into(),
            why: "XSS lets an attacker execute arbitrary JavaScript in a victim's browser — stealing session cookies, redirecting to phishing pages, or modifying page content.".into(),
            how_to_fix: "Use `textContent` instead of `innerHTML`. If HTML rendering is required, sanitize with DOMPurify. In React, never use `dangerouslySetInnerHTML` with untrusted input.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "typescript".into(),
            tags: vec!["cwe-79".into(), "owasp-a03".into(), "sans-top25".into()],
            effort_minutes: 30,
        },
        // ── BUGS ────────────────────────────────────────────────────────────
        SonarRule {
            key: "typescript:S2259".into(),
            name: "Null pointers should not be dereferenced".into(),
            description: "Accessing a property on a value that may be `null` or `undefined` will throw a `TypeError` at runtime.".into(),
            why: "Null dereferences are one of the most common causes of runtime crashes. TypeScript's type system can catch these, but only when strict null checks are enabled and respected.".into(),
            how_to_fix: "Use optional chaining (`obj?.prop`), nullish coalescing (`obj ?? default`), or an explicit null guard (`if (obj !== null)`) before accessing properties.".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "typescript".into(),
            tags: vec!["cwe-476".into()],
            effort_minutes: 10,
        },
        SonarRule {
            key: "typescript:S6544".into(),
            name: "Promises should not be misused".into(),
            description: "Calling an async function without `await`, or passing an async function as a non-async callback, silently discards the returned Promise and any errors it may throw.".into(),
            why: "Unhandled promise rejections crash Node.js processes in newer versions, and silently swallow errors in older ones. The operation may not have completed when you think it has.".into(),
            how_to_fix: "Add `await` before async calls. If the result is intentionally ignored, add a `.catch()` handler or explicitly cast to `void`.".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "typescript".into(),
            tags: vec!["es2017".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "typescript:S905".into(),
            name: "Non-empty statements should have at least one side-effect".into(),
            description: "A statement that has no side-effect (e.g., a property read whose result is discarded) indicates either dead code or a missing assignment/call.".into(),
            why: "The statement does nothing — it was likely meant to assign a value or call a function. Dead code confuses maintainers and hides bugs.".into(),
            how_to_fix: "Remove the statement if it is dead code, or fix the logic (e.g., change `x === y` to `x = y` if assignment was intended).".into(),
            severity: "MAJOR".into(),
            issue_type: "BUG".into(),
            language: "typescript".into(),
            tags: vec!["cert".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "typescript:S1125".into(),
            name: "Boolean literals should not be redundant".into(),
            description: "Comparing a boolean expression to `true` or `false` is redundant and reduces readability.".into(),
            why: "Code like `if (x === true)` is semantically equivalent to `if (x)` but harder to read and can mask type errors.".into(),
            how_to_fix: "Remove the redundant boolean literal. Change `if (x === true)` → `if (x)`, and `if (x === false)` → `if (!x)`.".into(),
            severity: "MINOR".into(),
            issue_type: "BUG".into(),
            language: "typescript".into(),
            tags: vec!["clumsy".into()],
            effort_minutes: 2,
        },
        // ── CODE SMELLS ─────────────────────────────────────────────────────
        SonarRule {
            key: "typescript:S3776".into(),
            name: "Cognitive complexity of functions should not be too high".into(),
            description: "Cognitive complexity measures how difficult a function is to understand. Functions with high cognitive complexity are hard to test, maintain, and reason about.".into(),
            why: "High complexity correlates strongly with defect density. Functions with complexity > 15 are significantly harder to maintain and understand correctly.".into(),
            how_to_fix: "Decompose the function into smaller, well-named helper functions. Extract complex conditional logic into named predicates. Use early returns to reduce nesting.".into(),
            severity: "CRITICAL".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["brain-overload".into()],
            effort_minutes: 60,
        },
        SonarRule {
            key: "typescript:S1481".into(),
            name: "Unused local variables should be removed".into(),
            description: "A local variable that is declared but never used, or only assigned and never read, adds noise without benefit.".into(),
            why: "Unused variables clutter the code, confuse readers, and may indicate a forgotten step in the logic (e.g., a result that was computed but never applied).".into(),
            how_to_fix: "Remove the unused variable, or use it. If the variable is a destructuring placeholder, use `_` to signal intentional non-use.".into(),
            severity: "MINOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["unused".into()],
            effort_minutes: 2,
        },
        SonarRule {
            key: "typescript:S1854".into(),
            name: "Unused assignments should be removed".into(),
            description: "A value is assigned to a variable, but the variable is then overwritten before it is ever read. The initial assignment is dead code.".into(),
            why: "Dead assignments waste CPU cycles and memory. They often indicate a logic bug where the intended operation (use of the first value) was accidentally omitted.".into(),
            how_to_fix: "Remove the dead assignment. If the first value was supposed to be used, fix the logic to use it before re-assigning.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["unused".into(), "cert".into()],
            effort_minutes: 2,
        },
        SonarRule {
            key: "typescript:S1192".into(),
            name: "String literals should not be duplicated".into(),
            description: "The same string literal appears three or more times in the code. Repeated magic strings are a maintenance hazard.".into(),
            why: "When a duplicated string needs to change, every occurrence must be updated. Missing one creates a subtle bug. Named constants make the intent clear and changes safe.".into(),
            how_to_fix: "Extract the string into a named constant (e.g., `const API_BASE = '/api/v1'`) and reference the constant throughout.".into(),
            severity: "MINOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["design".into()],
            effort_minutes: 10,
        },
        SonarRule {
            key: "typescript:S1128".into(),
            name: "Unnecessary imports should be removed".into(),
            description: "An import statement brings in a module or symbol that is never used in the file.".into(),
            why: "Unused imports increase bundle size, slow down compilation, and mislead readers about a file's dependencies.".into(),
            how_to_fix: "Remove the unused import. In VS Code, use the \"Organize Imports\" action (Shift+Alt+O) to remove all unused imports automatically.".into(),
            severity: "MINOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["unused".into()],
            effort_minutes: 2,
        },
        SonarRule {
            key: "typescript:S2737".into(),
            name: "Empty catch blocks should be handled".into(),
            description: "A `catch` block that does nothing (no logging, no re-throw, no fallback) silently swallows errors, making debugging extremely difficult.".into(),
            why: "Silent error swallowing is one of the most common causes of hard-to-diagnose production issues. The program continues in an undefined state without any indication of what went wrong.".into(),
            how_to_fix: "At minimum, log the error (`console.error(e)` or your logger). If the error is genuinely expected and ignorable, add a comment explaining why. Consider re-throwing if the caller should handle it.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["error-handling".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "typescript:S3358".into(),
            name: "Ternary operators should not be nested".into(),
            description: "Nesting ternary operators creates expressions that are extremely difficult to read and reason about.".into(),
            why: "Nested ternaries are frequently misread and misunderstood — even by the author. They are a common source of subtle logic bugs.".into(),
            how_to_fix: "Replace nested ternaries with `if/else` statements or extract the inner condition into a named variable or function.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["brain-overload".into(), "readability".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "typescript:S3403".into(),
            name: "Strict equality operators should be used with primitives".into(),
            description: "Using `==` or `!=` instead of `===` or `!==` triggers JavaScript's type coercion rules, which produce surprising results (`0 == ''` is `true`).".into(),
            why: "Type coercion makes code unpredictable. `== null` checks are the only widely-accepted use of loose equality; everywhere else `===` should be used.".into(),
            how_to_fix: "Replace `==` with `===` and `!=` with `!==`. The only exception is `x == null` which catches both `null` and `undefined`.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["bad-practice".into()],
            effort_minutes: 2,
        },
        SonarRule {
            key: "typescript:S1186".into(),
            name: "Functions should not be empty".into(),
            description: "An empty function body provides no implementation. It may be a placeholder that was never completed.".into(),
            why: "Empty functions confuse readers — was this intentional (a no-op) or a forgotten implementation? They can cause silent failures when callers expect behavior.".into(),
            how_to_fix: "Implement the function body, or add a comment explaining why it is intentionally empty. For intentional no-ops, consider using a `// no-op: reason` comment.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["suspicious".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "typescript:S1135".into(),
            name: "Track uses of 'TODO' tags".into(),
            description: "TODO and FIXME comments are technical debt markers that should not be left indefinitely in production code.".into(),
            why: "TODO comments represent known incomplete work or known bugs. If they are not tracked, they accumulate and never get resolved.".into(),
            how_to_fix: "Create an issue in your issue tracker for each TODO. Either complete the work or remove the TODO. If the work is deferred, reference the issue number in the comment.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "typescript:S107".into(),
            name: "Functions should not have too many parameters".into(),
            description: "A function with many parameters (> 7) is hard to call correctly and hard to understand. The parameter list becomes an unorganized bag of values.".into(),
            why: "Long parameter lists make function calls error-prone (arguments in wrong order), harder to read, and harder to extend without breaking callers.".into(),
            how_to_fix: "Group related parameters into an options object (`{ a, b, c }`). Apply the Parameter Object or Builder pattern for complex configuration.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["brain-overload".into()],
            effort_minutes: 20,
        },
        SonarRule {
            key: "typescript:S4325".into(),
            name: "'any' type should not be used".into(),
            description: "Using `any` disables TypeScript's type checking for a value, negating the purpose of using a typed language.".into(),
            why: "`any` is a type-safety escape hatch. Values typed as `any` can be used in any context without errors, hiding bugs that would otherwise be caught at compile time.".into(),
            how_to_fix: "Replace `any` with the most specific type you can. Use `unknown` when the type is truly unknown (it forces a type check before use). Use generics when the type is parameterized.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["type-safe".into()],
            effort_minutes: 15,
        },
        SonarRule {
            key: "typescript:S1066".into(),
            name: "Collapsible 'if' statements should be merged".into(),
            description: "When an `if` statement has no `else` and its body contains only another `if` statement, they can be merged with `&&`.".into(),
            why: "Nested single-branch `if` statements add indentation and visual complexity without expressing additional logic. Merging them makes the combined condition clear.".into(),
            how_to_fix: "Merge `if (a) { if (b) { ... } }` into `if (a && b) { ... }`.".into(),
            severity: "MINOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["clumsy".into()],
            effort_minutes: 2,
        },
        SonarRule {
            key: "typescript:S2234".into(),
            name: "Parameters should be passed in the correct order".into(),
            description: "Function call arguments are passed in a different order than the parameters are declared in the function signature.".into(),
            why: "Swapped arguments compile successfully but produce incorrect behavior. This is a particularly subtle bug when parameters have the same type.".into(),
            how_to_fix: "Verify the parameter order in the function signature and match your arguments to it. Consider using named parameters (options object) for functions with multiple same-type parameters.".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "typescript".into(),
            tags: vec!["cert".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "typescript:S4144".into(),
            name: "Functions should not have identical implementations".into(),
            description: "Two or more functions in the same scope have exactly the same body, indicating copy-paste code duplication.".into(),
            why: "Duplicated implementations must be updated in sync. If one is fixed and another is not, subtle behavioral differences arise. It also bloats bundle size.".into(),
            how_to_fix: "Extract the shared implementation into a single function and call it from both places. If the implementations need to vary slightly, parameterize the differences.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["design".into()],
            effort_minutes: 30,
        },
        SonarRule {
            key: "typescript:S1764".into(),
            name: "Identical expressions should not be used on both sides of an operator".into(),
            description: "Using the same expression on both sides of a binary operator (e.g., `x === x`, `a || a`) always produces a constant result and is almost certainly a mistake.".into(),
            why: "These expressions produce constant `true` or `false` regardless of input. They indicate a typo (e.g., intended to compare `x` to `y`) or a missed copy-paste fix.".into(),
            how_to_fix: "If this is a self-comparison (`x === x`) used as a NaN check, use `Number.isNaN(x)` instead. Otherwise, correct the two operands to be different as intended.".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "typescript".into(),
            tags: vec!["cert".into()],
            effort_minutes: 2,
        },
        SonarRule {
            key: "typescript:S6509".into(),
            name: "React hooks should not be called conditionally".into(),
            description: "React Hooks must be called at the top level of a component — never inside loops, conditionals, or nested functions.".into(),
            why: "React relies on the order of Hook calls to correctly preserve state between renders. Conditional calls change the call order and cause unpredictable state corruption.".into(),
            how_to_fix: "Move the hook call to the top level. If conditional logic is needed, place the condition inside the hook (e.g., skip effects with an early return inside `useEffect`).".into(),
            severity: "BLOCKER".into(),
            issue_type: "BUG".into(),
            language: "typescript".into(),
            tags: vec!["react".into()],
            effort_minutes: 10,
        },
        SonarRule {
            key: "typescript:S6478".into(),
            name: "React component functions should not be defined inside other components".into(),
            description: "Defining a component function inside another component causes it to be re-created on every render of the parent.".into(),
            why: "React performs a referential equality check on child components. A new function reference every render causes the child to always unmount and remount, losing state and triggering unnecessary effects.".into(),
            how_to_fix: "Move the inner component definition outside the parent component. If it needs access to parent data, pass it as props.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "typescript".into(),
            tags: vec!["react".into(), "performance".into()],
            effort_minutes: 10,
        },
        SonarRule {
            key: "typescript:S6747".into(),
            name: "useEffect dependency array is missing or incomplete".into(),
            description: "`useEffect` is called without a dependency array, or the dependency array is missing values that are used inside the effect.".into(),
            why: "An effect without a dependency array runs after every render (performance issue). A missing dependency causes stale closures where the effect reads outdated values.".into(),
            how_to_fix: "Add a dependency array listing all variables the effect reads from the component scope. Use the `exhaustive-deps` ESLint rule to catch missing dependencies automatically.".into(),
            severity: "MAJOR".into(),
            issue_type: "BUG".into(),
            language: "typescript".into(),
            tags: vec!["react".into()],
            effort_minutes: 15,
        },
        SonarRule {
            key: "general:S1135".into(),
            name: "Track uses of 'TODO'/'FIXME' tags".into(),
            description: "TODO and FIXME markers indicate known technical debt. Left untracked, they accumulate over time.".into(),
            why: "Research shows TODO comments are rarely acted on unless tracked in an issue system. They represent acknowledged but unfixed problems.".into(),
            how_to_fix: "Create a ticket in your tracker, reference it in the comment, and address it in a dedicated sprint. Remove the comment once resolved.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "general".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "rust:S2068".into(),
            name: "Credentials should not be hard-coded (Rust)".into(),
            description: "Hard-coded credentials in Rust source code are accessible to anyone who can read the binary or the source.".into(),
            why: "Rust binaries can be inspected with tools like `strings`. Credentials in source are also captured in version control history permanently.".into(),
            how_to_fix: "Use `std::env::var(\"SECRET_KEY\")` or a crate like `dotenv` / `config` to read secrets from the environment. Consider `secrecy::Secret<String>` to prevent accidental logging.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "rust".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "rust:S1135".into(),
            name: "Track TODO/FIXME/HACK comments".into(),
            description: "TODO, FIXME, and HACK comments mark acknowledged technical debt in Rust code.".into(),
            why: "Untracked debt accumulates. Teams often discover critical security or correctness TODOs only after incidents.".into(),
            how_to_fix: "Link each TODO to a GitHub issue (`// TODO(#123): ...`). Address in a scheduled sprint.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "rust".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "rust:S3776".into(),
            name: "Function cognitive complexity is too high (Rust)".into(),
            description: "Rust functions with deeply nested match arms, loops, and conditionals become difficult to reason about and test.".into(),
            why: "High complexity correlates with higher defect rates and makes safe Rust patterns (ownership, lifetimes) harder to audit correctly.".into(),
            how_to_fix: "Extract sub-expressions into named helper functions. Use `?` for early returns instead of nested match chains. Apply the Builder pattern for complex initialization.".into(),
            severity: "CRITICAL".into(),
            issue_type: "CODE_SMELL".into(),
            language: "rust".into(),
            tags: vec!["brain-overload".into()],
            effort_minutes: 60,
        },

        // ── PYTHON ──────────────────────────────────────────────────────────
        SonarRule {
            key: "python:S1481".into(),
            name: "Bare `except:` should not be used".into(),
            description: "A bare `except:` clause catches all exceptions including SystemExit and KeyboardInterrupt, preventing the interpreter from shutting down cleanly.".into(),
            why: "Bare except catches SystemExit (raised by sys.exit()), KeyboardInterrupt (Ctrl+C), and GeneratorExit — interfering with normal Python runtime behavior.".into(),
            how_to_fix: "Always specify the exception types you want to catch: `except (ValueError, TypeError):`. If you need a catch-all, use `except Exception:` which does not catch BaseException subclasses.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "python".into(),
            tags: vec!["error-handling".into(), "bad-practice".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "python:S2068".into(),
            name: "Credentials should not be hard-coded (Python)".into(),
            description: "Hard-coded credentials in Python source code are accessible to anyone with access to the file or version control history.".into(),
            why: "Credentials committed to source control are permanently accessible via git history and can be leaked via public repositories or error messages.".into(),
            how_to_fix: "Use environment variables (`os.environ['SECRET']`), python-dotenv, or a secrets manager. Never assign literal passwords or keys in source.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "python".into(),
            tags: vec!["cwe-798".into(), "owasp-a07".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "python:S1135".into(),
            name: "Track TODO/FIXME (Python)".into(),
            description: "TODO and FIXME comments are technical debt markers that should not be left indefinitely in production code.".into(),
            why: "TODO comments represent known incomplete work. If untracked, they accumulate and never get resolved.".into(),
            how_to_fix: "Create an issue in your issue tracker for each TODO. Reference the issue number in the comment and resolve it in a dedicated sprint.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "python".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "python:S3776".into(),
            name: "Cognitive complexity too high (Python)".into(),
            description: "Python functions with deeply nested conditionals and loops are hard to test and maintain.".into(),
            why: "High complexity correlates with higher defect density and makes code harder to reason about.".into(),
            how_to_fix: "Extract nested logic into helper functions. Use early returns to flatten nesting. Apply guard clauses to reduce indentation levels.".into(),
            severity: "CRITICAL".into(),
            issue_type: "CODE_SMELL".into(),
            language: "python".into(),
            tags: vec!["brain-overload".into()],
            effort_minutes: 60,
        },
        SonarRule {
            key: "python:S1186".into(),
            name: "Empty function body (Python)".into(),
            description: "A function body containing only `pass` in a non-abstract function provides no implementation.".into(),
            why: "Empty functions confuse readers — was this intentional (a no-op) or a forgotten implementation? They can cause silent failures when callers expect behavior.".into(),
            how_to_fix: "Implement the function body, add a comment explaining why it is intentionally empty, or raise NotImplementedError if it is meant to be overridden.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "python".into(),
            tags: vec!["suspicious".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "python:S2201".into(),
            name: "Mutable default argument".into(),
            description: "Using a mutable object (list, dict, set) as a default argument value is a classic Python gotcha.".into(),
            why: "Default argument values are evaluated once when the function is defined, not each time the function is called. Mutable defaults are shared across all calls, causing unexpected state accumulation.".into(),
            how_to_fix: "Use `None` as the default value and create the mutable object inside the function body: `def f(x=None): if x is None: x = []`.".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "python".into(),
            tags: vec!["pitfall".into(), "python".into()],
            effort_minutes: 10,
        },
        SonarRule {
            key: "python:S5905".into(),
            name: "`print()` statements in production code".into(),
            description: "Using `print()` for diagnostic output in production code is a code smell — it bypasses the logging infrastructure.".into(),
            why: "print() output cannot be configured, filtered, or redirected without code changes. It clutters stdout and may expose sensitive data in production logs.".into(),
            how_to_fix: "Replace `print()` with the `logging` module: `import logging; logging.debug('message')`. Configure log levels and handlers in your application configuration.".into(),
            severity: "MINOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "python".into(),
            tags: vec!["convention".into()],
            effort_minutes: 5,
        },

        // ── C ────────────────────────────────────────────────────────────────
        SonarRule {
            key: "c:S2068".into(),
            name: "Credentials should not be hard-coded (C)".into(),
            description: "Hard-coded credentials in C source code can be extracted from compiled binaries using tools like `strings`.".into(),
            why: "C string literals are embedded verbatim in the binary. Credentials in source are also captured in version control history permanently.".into(),
            how_to_fix: "Read credentials from environment variables, configuration files excluded from version control, or a secrets management system.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "c".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "c:S1135".into(),
            name: "Track TODO/FIXME (C)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates and may hide critical security or correctness issues.".into(),
            how_to_fix: "Link each TODO to an issue tracker entry and address it in a scheduled sprint.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "c".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "c:S3518".into(),
            name: "Using `gets()` is dangerous — buffer overflow".into(),
            description: "`gets()` reads an unbounded amount of input with no buffer size limit, making buffer overflow inevitable.".into(),
            why: "`gets()` has no bounds checking whatsoever. It was removed from the C11 standard for this reason. Any input longer than the buffer will corrupt memory.".into(),
            how_to_fix: "Use `fgets(buf, sizeof(buf), stdin)` which requires an explicit size limit. For dynamic input, use `getline()` on POSIX systems.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "c".into(),
            tags: vec!["cwe-120".into(), "sans-top25".into()],
            effort_minutes: 15,
        },
        SonarRule {
            key: "c:S1481".into(),
            name: "Unused variable (C)".into(),
            description: "A variable is declared but never used in the function body.".into(),
            why: "Unused variables clutter the code and may indicate a forgotten step in the logic.".into(),
            how_to_fix: "Remove the unused variable, or use it. Compile with `-Wall` to catch these automatically.".into(),
            severity: "MINOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "c".into(),
            tags: vec!["unused".into()],
            effort_minutes: 2,
        },
        SonarRule {
            key: "c:S2182".into(),
            name: "Null pointer dereferenced after malloc without null check".into(),
            description: "`malloc()` and `calloc()` return NULL when allocation fails. Dereferencing without checking causes undefined behavior.".into(),
            why: "In low-memory conditions or with large allocations, malloc can return NULL. Dereferencing NULL is undefined behavior and typically causes a segmentation fault.".into(),
            how_to_fix: "Always check `if (ptr == NULL) { /* handle error */ }` immediately after malloc/calloc before using the pointer.".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "c".into(),
            tags: vec!["cwe-476".into()],
            effort_minutes: 10,
        },

        // ── C++ ──────────────────────────────────────────────────────────────
        SonarRule {
            key: "cpp:S2068".into(),
            name: "Credentials should not be hard-coded (C++)".into(),
            description: "Hard-coded credentials in C++ source code can be extracted from compiled binaries.".into(),
            why: "C++ string literals are embedded in the binary. Credentials in source are captured in version control permanently.".into(),
            how_to_fix: "Use environment variables or a configuration file excluded from version control. Consider using `std::getenv()` or a secrets management library.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "cpp".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "cpp:S1135".into(),
            name: "Track TODO/FIXME (C++)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates and may hide critical security or correctness issues.".into(),
            how_to_fix: "Link each TODO to an issue tracker entry and address it in a scheduled sprint.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "cpp".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "cpp:S1232".into(),
            name: "Classes with virtual methods should have a virtual destructor".into(),
            description: "A class with virtual methods but a non-virtual destructor causes undefined behavior when deleting via a base class pointer.".into(),
            why: "When `delete` is called on a base class pointer pointing to a derived object, only the base destructor runs if it is not virtual. The derived destructor is skipped, leaking resources and corrupting state.".into(),
            how_to_fix: "Add `virtual ~MyClass() = default;` (or a defined destructor) to any class with virtual methods.".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "cpp".into(),
            tags: vec!["cwe-1041".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "cpp:S5445".into(),
            name: "Using `new` without RAII/smart pointer causes memory leaks".into(),
            description: "Raw `new` allocations require a matching `delete`. Any early return, exception, or missed code path will leak the allocation.".into(),
            why: "Manual memory management with raw `new`/`delete` is error-prone. Exceptions in particular make it very hard to guarantee `delete` is always called.".into(),
            how_to_fix: "Use `std::unique_ptr<T>` for exclusive ownership or `std::shared_ptr<T>` for shared ownership. Prefer `std::make_unique<T>()` and `std::make_shared<T>()` factory functions.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "cpp".into(),
            tags: vec!["memory".into(), "raii".into()],
            effort_minutes: 15,
        },
        SonarRule {
            key: "cpp:S3518".into(),
            name: "Using `strcpy`/`gets`/`sprintf` without bounds checking".into(),
            description: "These C standard library functions perform no bounds checking and are trivially exploitable for buffer overflow attacks.".into(),
            why: "Buffer overflows are among the most exploited vulnerability classes (CWE-120). These functions copy data until a null terminator without respecting destination buffer size.".into(),
            how_to_fix: "Use bounds-checked alternatives: `strncpy` → `strlcpy`, `strcpy` → `strncpy`, `sprintf` → `snprintf`, `gets` → `fgets`. Better yet, use C++ `std::string`.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "cpp".into(),
            tags: vec!["cwe-120".into(), "sans-top25".into()],
            effort_minutes: 15,
        },

        // ── JAVA ─────────────────────────────────────────────────────────────
        SonarRule {
            key: "java:S2068".into(),
            name: "Credentials should not be hard-coded (Java)".into(),
            description: "Hard-coded credentials in Java source code are accessible to anyone with access to the source or compiled class files.".into(),
            why: "Java .class files can be decompiled easily. Credentials in source are captured in version control history permanently.".into(),
            how_to_fix: "Use `System.getenv(\"SECRET\")`, a properties file excluded from VCS, or a secrets manager (AWS Secrets Manager, HashiCorp Vault).".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "java".into(),
            tags: vec!["cwe-798".into(), "owasp-a07".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "java:S1135".into(),
            name: "Track TODO/FIXME (Java)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates and may hide critical correctness or security issues.".into(),
            how_to_fix: "Create a Jira/GitHub issue for each TODO. Reference the issue number in the comment.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "java".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "java:S2259".into(),
            name: "Null pointer dereference (Java)".into(),
            description: "Accessing a method or field on a reference that may be null causes a NullPointerException at runtime.".into(),
            why: "NullPointerExceptions are one of the most common runtime errors in Java. Modern Java (14+) provides helpful NPE messages but prevention is always better.".into(),
            how_to_fix: "Use null checks, Optional<T>, or Objects.requireNonNull(). Enable IDE null analysis annotations (@NonNull, @Nullable).".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "java".into(),
            tags: vec!["cwe-476".into()],
            effort_minutes: 10,
        },
        SonarRule {
            key: "java:S2095".into(),
            name: "Resources should be closed (try-with-resources)".into(),
            description: "Streams, connections, and other Closeable resources that are not closed in a finally block or try-with-resources statement leak system resources.".into(),
            why: "Unclosed streams, JDBC connections, and file handles exhaust OS resource limits, causing eventual failures that are hard to diagnose.".into(),
            how_to_fix: "Use try-with-resources: `try (var reader = new FileReader(f)) { ... }`. This guarantees close() is called even if an exception is thrown.".into(),
            severity: "MAJOR".into(),
            issue_type: "BUG".into(),
            language: "java".into(),
            tags: vec!["cwe-772".into()],
            effort_minutes: 10,
        },
        SonarRule {
            key: "java:S1206".into(),
            name: "equals() and hashCode() must be overridden together".into(),
            description: "Overriding equals() without overriding hashCode() (or vice versa) breaks the equals/hashCode contract, corrupting HashMap and HashSet behavior.".into(),
            why: "The Java contract requires that objects that are equal must have the same hash code. Violating this means objects can be 'lost' in hash-based collections.".into(),
            how_to_fix: "Always override both equals() and hashCode() together. Use IDE generation or Objects.hash() and Objects.equals() helpers.".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "java".into(),
            tags: vec!["cert".into()],
            effort_minutes: 10,
        },
        SonarRule {
            key: "java:S3358".into(),
            name: "Ternary operators should not be nested (Java)".into(),
            description: "Nesting ternary operators creates expressions that are extremely difficult to read and reason about.".into(),
            why: "Nested ternaries are frequently misread — even by the author. They are a common source of subtle logic bugs.".into(),
            how_to_fix: "Replace nested ternaries with if/else statements or extract the inner condition into a named variable.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "java".into(),
            tags: vec!["brain-overload".into()],
            effort_minutes: 5,
        },

        // ── C# ──────────────────────────────────────────────────────────────
        SonarRule {
            key: "csharp:S2068".into(),
            name: "Credentials should not be hard-coded (C#)".into(),
            description: "Hard-coded credentials in C# source code are a critical security risk accessible via decompilation.".into(),
            why: ".NET assemblies can be decompiled with tools like ILSpy. Credentials in source are captured in VCS history permanently.".into(),
            how_to_fix: "Use `Environment.GetEnvironmentVariable(\"SECRET\")`, the .NET Secret Manager, Azure Key Vault, or AWS Secrets Manager.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "csharp".into(),
            tags: vec!["cwe-798".into(), "owasp-a07".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "csharp:S1135".into(),
            name: "Track TODO/FIXME (C#)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates and may hide critical correctness or security issues.".into(),
            how_to_fix: "Create an issue in your tracker for each TODO and reference it in the comment.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "csharp".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "csharp:S4457".into(),
            name: "async method not awaited".into(),
            description: "Calling an async method without `await` returns a Task that is silently discarded. The operation may not complete when expected and exceptions are swallowed.".into(),
            why: "Unawaited Tasks run independently and any exceptions they throw are swallowed by default, causing hard-to-diagnose bugs and race conditions.".into(),
            how_to_fix: "Add the `await` keyword before async method calls. For intentional fire-and-forget, use `_ = Task.Run(...)` or explicitly call `.GetAwaiter().GetResult()` if synchronous blocking is needed.".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "csharp".into(),
            tags: vec!["async".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "csharp:S3869".into(),
            name: "IDisposable should be disposed".into(),
            description: "Objects implementing IDisposable must be disposed to release unmanaged resources like file handles, database connections, and network sockets.".into(),
            why: "Failing to call Dispose() leaks unmanaged resources. The finalizer may eventually run but this is non-deterministic and can cause resource exhaustion.".into(),
            how_to_fix: "Use `using` statements or `using` declarations: `using var conn = new SqlConnection(cs);`. This guarantees Dispose() is called even if exceptions occur.".into(),
            severity: "MAJOR".into(),
            issue_type: "BUG".into(),
            language: "csharp".into(),
            tags: vec!["cwe-772".into()],
            effort_minutes: 10,
        },
        SonarRule {
            key: "csharp:S1066".into(),
            name: "Consecutive if statements can be merged (C#)".into(),
            description: "When an `if` statement has no `else` and contains only another `if`, they can be merged with `&&`.".into(),
            why: "Nested single-branch if statements add indentation without expressing additional logic.".into(),
            how_to_fix: "Merge `if (a) { if (b) { ... } }` into `if (a && b) { ... }`.".into(),
            severity: "MINOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "csharp".into(),
            tags: vec!["clumsy".into()],
            effort_minutes: 2,
        },

        // ── PHP ──────────────────────────────────────────────────────────────
        SonarRule {
            key: "php:S2068".into(),
            name: "Credentials should not be hard-coded (PHP)".into(),
            description: "Hard-coded credentials in PHP source files are a critical security risk.".into(),
            why: "PHP source is often deployed to servers where it may be readable. Credentials in source are captured in VCS history permanently.".into(),
            how_to_fix: "Use `$_ENV['SECRET']`, `getenv('SECRET')`, or a `.env` file loaded with a library like vlucas/phpdotenv that is excluded from VCS.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "php".into(),
            tags: vec!["cwe-798".into(), "owasp-a07".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "php:S1135".into(),
            name: "Track TODO/FIXME (PHP)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates and may hide critical correctness or security issues.".into(),
            how_to_fix: "Create a GitHub/Jira issue for each TODO and reference it in the comment.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "php".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "php:S2076".into(),
            name: "eval() should not be used (PHP)".into(),
            description: "`eval()` executes a string as PHP code, enabling arbitrary code execution if any part of the string is user-controlled.".into(),
            why: "eval() with user input is equivalent to remote code execution. Even without direct user input, eval() makes code hard to audit, debug, and test.".into(),
            how_to_fix: "Remove eval(). Use proper data structures, template engines (Twig, Blade), or callback-based patterns instead of dynamically generating PHP code.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "php".into(),
            tags: vec!["cwe-95".into(), "owasp-a03".into()],
            effort_minutes: 30,
        },
        SonarRule {
            key: "php:S3649".into(),
            name: "SQL injection in PHP".into(),
            description: "Constructing SQL queries via string concatenation with `mysql_query`/`mysqli_query` allows SQL injection.".into(),
            why: "An attacker can bypass authentication, dump the database, or execute arbitrary commands via SQL injection.".into(),
            how_to_fix: "Use PDO prepared statements: `$stmt = $pdo->prepare('SELECT * FROM users WHERE id = ?'); $stmt->execute([$id]);`".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "php".into(),
            tags: vec!["cwe-89".into(), "owasp-a03".into(), "sans-top25".into()],
            effort_minutes: 30,
        },
        SonarRule {
            key: "php:S1145".into(),
            name: "extract() with user data leads to variable injection".into(),
            description: "`extract()` on user-supplied arrays (e.g., `$_GET`, `$_POST`) creates PHP variables for each key, potentially overwriting existing variables.".into(),
            why: "An attacker can inject arbitrary variables including `$_SESSION`, authentication flags, and database handles, leading to privilege escalation.".into(),
            how_to_fix: "Never call `extract()` on untrusted data. Access array keys explicitly: `$name = $_POST['name'] ?? ''`.".into(),
            severity: "CRITICAL".into(),
            issue_type: "VULNERABILITY".into(),
            language: "php".into(),
            tags: vec!["cwe-95".into()],
            effort_minutes: 20,
        },

        // ── GO ───────────────────────────────────────────────────────────────
        SonarRule {
            key: "go:S2068".into(),
            name: "Credentials should not be hard-coded (Go)".into(),
            description: "Hard-coded credentials in Go source code can be extracted from compiled binaries.".into(),
            why: "Go binaries can be inspected with `strings`. Credentials in source are captured in VCS history permanently.".into(),
            how_to_fix: "Use `os.Getenv(\"SECRET\")`, the `viper` config library, or a secrets manager. Consider the `envconfig` or `godotenv` packages.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "go".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "go:S1135".into(),
            name: "Track TODO/FIXME (Go)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates. In Go codebases, TODOs often signal missing error handling or deferred goroutine safety.".into(),
            how_to_fix: "Link each TODO to a GitHub issue and address it in a scheduled sprint.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "go".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "go:S2704".into(),
            name: "Error return values must not be ignored".into(),
            description: "Assigning an error return value to `_` silently discards it. The calling code continues as if the operation succeeded.".into(),
            why: "In Go, errors are values. Ignoring them means failures go undetected and the program continues in an invalid state, often causing harder-to-diagnose failures later.".into(),
            how_to_fix: "Check the error, return it up the call chain, or explicitly log it. If the error is genuinely ignorable, document why with a comment.".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "go".into(),
            tags: vec!["error-handling".into()],
            effort_minutes: 10,
        },
        SonarRule {
            key: "go:S6288".into(),
            name: "defer in loop causes resource exhaustion".into(),
            description: "`defer` statements inside a loop accumulate until the function returns, not until the loop iteration ends.".into(),
            why: "If you open a file or acquire a lock inside a loop and defer its release, all the resources are held until the function exits — potentially thousands of open file descriptors.".into(),
            how_to_fix: "Extract the loop body into a separate function where the defer will run at the end of each call, releasing the resource each iteration.".into(),
            severity: "MAJOR".into(),
            issue_type: "BUG".into(),
            language: "go".into(),
            tags: vec!["resource-management".into()],
            effort_minutes: 15,
        },

        // ── SWIFT ────────────────────────────────────────────────────────────
        SonarRule {
            key: "swift:S2068".into(),
            name: "Credentials should not be hard-coded (Swift)".into(),
            description: "Hard-coded credentials in Swift source code can be extracted from compiled app bundles.".into(),
            why: "iOS/macOS apps can be reverse-engineered with tools like class-dump and Hopper. Credentials in source are captured in VCS history permanently.".into(),
            how_to_fix: "Use the iOS Keychain, environment variables during CI/CD, or a secrets management service. Never hard-code API keys in app source.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "swift".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "swift:S1135".into(),
            name: "Track TODO/FIXME (Swift)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked TODOs accumulate and may represent critical missing functionality.".into(),
            how_to_fix: "Create a GitHub/Jira issue and reference it in the comment. Swift compiler warnings can be triggered with `#warning(\"TODO: ...\")`.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "swift".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "swift:S6532".into(),
            name: "Force unwrap `!` should be avoided".into(),
            description: "The force unwrap operator `!` crashes at runtime with a fatal error if the optional is nil.".into(),
            why: "Force unwrapping nil causes an immediate, unrecoverable crash. This is one of the most common causes of iOS app crashes in production.".into(),
            how_to_fix: "Use optional binding (`if let x = opt`), `guard let`, or the nil-coalescing operator (`opt ?? defaultValue`). In tests, `XCTUnwrap` provides a safe force-unwrap with a test failure.".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "swift".into(),
            tags: vec!["crash".into()],
            effort_minutes: 10,
        },
        SonarRule {
            key: "swift:S1481".into(),
            name: "Unused variables (Swift)".into(),
            description: "A variable is declared but never used in the function body.".into(),
            why: "Unused variables clutter the code and may indicate forgotten logic.".into(),
            how_to_fix: "Remove the unused variable, or use it. Swift compiler warns about unused variables with `-warn-unused`.".into(),
            severity: "MINOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "swift".into(),
            tags: vec!["unused".into()],
            effort_minutes: 2,
        },

        // ── KOTLIN ───────────────────────────────────────────────────────────
        SonarRule {
            key: "kotlin:S2068".into(),
            name: "Credentials should not be hard-coded (Kotlin)".into(),
            description: "Hard-coded credentials in Kotlin source code are accessible via decompilation.".into(),
            why: "Kotlin/JVM bytecode can be decompiled easily. Credentials in source are captured in VCS history permanently.".into(),
            how_to_fix: "Use `System.getenv(\"SECRET\")`, the Android Keystore, or a secrets management library.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "kotlin".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "kotlin:S1135".into(),
            name: "Track TODO/FIXME (Kotlin)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates and may represent critical missing functionality.".into(),
            how_to_fix: "Create an issue and reference it. Kotlin provides a built-in `TODO()` function that throws NotImplementedError — prefer it over comments for truly unimplemented code.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "kotlin".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "kotlin:S6531".into(),
            name: "Non-null assertion `!!` should be avoided".into(),
            description: "The not-null assertion operator `!!` throws a NullPointerException if the value is null.".into(),
            why: "`!!` is the Kotlin equivalent of force-unwrapping a null. It defeats Kotlin's null-safety guarantees and causes runtime crashes.".into(),
            how_to_fix: "Use safe call `?.`, the Elvis operator `?:`, or `requireNotNull()` / `checkNotNull()` with a meaningful message. Use `let` for null-safe transformations.".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "kotlin".into(),
            tags: vec!["null-safety".into()],
            effort_minutes: 10,
        },
        SonarRule {
            key: "kotlin:S3776".into(),
            name: "Cognitive complexity too high (Kotlin)".into(),
            description: "Kotlin functions with deeply nested conditionals and loops are hard to test and maintain.".into(),
            why: "High complexity correlates with higher defect density. Kotlin's expressive features (when expressions, extension functions) make it easy to reduce complexity.".into(),
            how_to_fix: "Extract nested logic into extension functions. Use `when` expressions instead of chains of `if/else`. Apply functional patterns (map, filter, fold) to reduce looping.".into(),
            severity: "CRITICAL".into(),
            issue_type: "CODE_SMELL".into(),
            language: "kotlin".into(),
            tags: vec!["brain-overload".into()],
            effort_minutes: 60,
        },

        // ── RUBY ─────────────────────────────────────────────────────────────
        SonarRule {
            key: "ruby:S2068".into(),
            name: "Credentials should not be hard-coded (Ruby)".into(),
            description: "Hard-coded credentials in Ruby source code are a critical security risk.".into(),
            why: "Ruby source is interpreted and easily readable. Credentials in source are captured in VCS history permanently.".into(),
            how_to_fix: "Use `ENV['SECRET']`, the `dotenv` gem, Rails credentials (`rails credentials:edit`), or a secrets management service.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "ruby".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "ruby:S1135".into(),
            name: "Track TODO/FIXME (Ruby)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates and may represent critical missing functionality.".into(),
            how_to_fix: "Create a GitHub/Jira issue and reference it in the comment.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "ruby".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "ruby:S1481".into(),
            name: "eval() should not be used (Ruby)".into(),
            description: "`eval` executes a string as Ruby code, enabling arbitrary code execution if any part of the string is user-controlled.".into(),
            why: "eval with user input is equivalent to remote code execution. Even without direct user input, eval makes code impossible to audit and analyze statically.".into(),
            how_to_fix: "Remove eval. Use proper data structures, method_missing for DSLs, or the `public_send` method for dynamic dispatch.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "ruby".into(),
            tags: vec!["cwe-95".into()],
            effort_minutes: 30,
        },
        SonarRule {
            key: "ruby:S3649".into(),
            name: "SQL injection via string interpolation in ActiveRecord".into(),
            description: "Passing user input directly into ActiveRecord query methods via string interpolation allows SQL injection.".into(),
            why: "An attacker can manipulate SQL logic to bypass authentication, extract all data, or destroy records.".into(),
            how_to_fix: "Use ActiveRecord parameterized queries: `User.where('name = ?', name)` or `User.where(name: name)`. Never interpolate user input directly into query strings.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "ruby".into(),
            tags: vec!["cwe-89".into(), "owasp-a03".into()],
            effort_minutes: 30,
        },

        // ── SQL ──────────────────────────────────────────────────────────────
        SonarRule {
            key: "sql:S2077".into(),
            name: "SELECT * should not be used".into(),
            description: "`SELECT *` retrieves all columns, making code fragile and queries inefficient.".into(),
            why: "Schema changes that add or reorder columns silently break application code that assumes column positions. Retrieving unused columns wastes network bandwidth and database memory.".into(),
            how_to_fix: "Enumerate only the columns you need: `SELECT id, name, email FROM users`. This also serves as implicit documentation of what data the query consumes.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "sql".into(),
            tags: vec!["performance".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "sql:S1135".into(),
            name: "Track TODO/FIXME (SQL)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt in SQL files.".into(),
            why: "Untracked TODOs in SQL migrations or stored procedures accumulate and may represent missing indexes, data quality fixes, or security patches.".into(),
            how_to_fix: "Create an issue and reference it in the comment.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "sql".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "sql:S3649".into(),
            name: "DELETE/UPDATE without WHERE clause".into(),
            description: "A DELETE or UPDATE statement without a WHERE clause affects every row in the table.".into(),
            why: "Executing `DELETE FROM users` drops all user records. This is frequently caused by accidentally running test scripts on production databases.".into(),
            how_to_fix: "Always add a WHERE clause to DELETE and UPDATE statements. Use transactions and test in a staging environment first. Consider `LIMIT 1` as an additional safeguard during development.".into(),
            severity: "BLOCKER".into(),
            issue_type: "BUG".into(),
            language: "sql".into(),
            tags: vec!["data-loss".into()],
            effort_minutes: 10,
        },

        // ── SOLIDITY ─────────────────────────────────────────────────────────
        SonarRule {
            key: "solidity:S6321".into(),
            name: "tx.origin should not be used for authentication".into(),
            description: "`tx.origin` returns the original externally owned account that started the transaction, not the immediate caller.".into(),
            why: "`tx.origin` is vulnerable to phishing attacks — a malicious contract can trick a user into calling it, then use `tx.origin` to impersonate the user in your contract.".into(),
            how_to_fix: "Use `msg.sender` for authentication. `msg.sender` is always the immediate caller (EOA or contract), making it safe for access control checks.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "solidity".into(),
            tags: vec!["cwe-284".into(), "swc-115".into()],
            effort_minutes: 10,
        },
        SonarRule {
            key: "solidity:S6327".into(),
            name: "Reentrancy vulnerability".into(),
            description: "Making external calls before updating state allows the called contract to re-enter your function in an unexpected state.".into(),
            why: "The infamous DAO hack exploited reentrancy to drain 3.6M ETH. External calls can invoke fallback functions that call back into your contract before state updates complete.".into(),
            how_to_fix: "Follow the Checks-Effects-Interactions pattern: 1) Check conditions, 2) Update state, 3) Make external calls. Alternatively use a ReentrancyGuard modifier (OpenZeppelin).".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "solidity".into(),
            tags: vec!["swc-107".into(), "defi-security".into()],
            effort_minutes: 60,
        },
        SonarRule {
            key: "solidity:S6329".into(),
            name: "block.timestamp should not be used as source of truth".into(),
            description: "`block.timestamp` can be manipulated by miners within a ~15-second window, making it unreliable for time-sensitive logic.".into(),
            why: "Miners can adjust block.timestamp slightly to trigger favorable outcomes in timestamp-dependent contract logic.".into(),
            how_to_fix: "For time locks with tolerance > 15 minutes, block.timestamp is generally safe. For finer-grained timing or randomness, use a Chainlink VRF or commit-reveal scheme.".into(),
            severity: "MAJOR".into(),
            issue_type: "SECURITY_HOTSPOT".into(),
            language: "solidity".into(),
            tags: vec!["swc-116".into()],
            effort_minutes: 20,
        },
        SonarRule {
            key: "solidity:S2068".into(),
            name: "Credentials/private keys should not be hard-coded (Solidity)".into(),
            description: "Hard-coded private keys or secrets in Solidity source code are visible on-chain and in the repository.".into(),
            why: "Smart contracts and their source code are often publicly visible. Hard-coded private keys can be used to drain wallets or impersonate contract owners.".into(),
            how_to_fix: "Never embed private keys in contracts. Use constructor parameters for admin addresses, and manage private keys externally via hardware wallets or secure key management.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "solidity".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },

        // ── POWERSHELL ───────────────────────────────────────────────────────
        SonarRule {
            key: "powershell:S2068".into(),
            name: "Credentials should not be hard-coded (PowerShell)".into(),
            description: "Hard-coded credentials in PowerShell scripts are visible in plain text.".into(),
            why: "PowerShell scripts are plain text and frequently stored in version control. Hard-coded credentials are easily extracted.".into(),
            how_to_fix: "Use `$env:SECRET`, the Windows Credential Manager, or `Get-Secret` from the SecretManagement module. Use `ConvertTo-SecureString` for passwords.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "powershell".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "powershell:S1135".into(),
            name: "Track TODO/FIXME (PowerShell)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates and may represent missing security controls.".into(),
            how_to_fix: "Create an issue and reference it in the comment.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "powershell".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "powershell:S3649".into(),
            name: "Invoke-Expression with user input is dangerous".into(),
            description: "`Invoke-Expression` (iex) executes a string as PowerShell code, equivalent to eval().".into(),
            why: "If any part of the string is user-controlled, an attacker can execute arbitrary commands with the script's permissions.".into(),
            how_to_fix: "Avoid Invoke-Expression entirely. Use specific cmdlets with typed parameters. If dynamic invocation is needed, use `& $scriptBlock` with a validated script block.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "powershell".into(),
            tags: vec!["cwe-95".into()],
            effort_minutes: 30,
        },

        // ── PERL ─────────────────────────────────────────────────────────────
        SonarRule {
            key: "perl:S2068".into(),
            name: "Credentials should not be hard-coded (Perl)".into(),
            description: "Hard-coded credentials in Perl scripts are visible in plain text.".into(),
            why: "Perl scripts are plain text and easily readable. Credentials in source are captured in VCS history permanently.".into(),
            how_to_fix: "Use `$ENV{SECRET}` or read from a configuration file excluded from version control.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "perl".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "perl:S1135".into(),
            name: "Track TODO/FIXME (Perl)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates and may represent critical missing functionality.".into(),
            how_to_fix: "Create an issue and reference it in the comment.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "perl".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "perl:S2076".into(),
            name: "eval with user input (Perl)".into(),
            description: "`eval` with user-controlled input in Perl executes arbitrary Perl code.".into(),
            why: "eval() is one of the most dangerous functions in Perl when used with untrusted input — it enables arbitrary code execution.".into(),
            how_to_fix: "Avoid eval with user input. Use Safe.pm compartments if dynamic evaluation is absolutely necessary. Prefer structured data parsing over eval-based deserialization.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "perl".into(),
            tags: vec!["cwe-95".into()],
            effort_minutes: 30,
        },

        // ── LUA ──────────────────────────────────────────────────────────────
        SonarRule {
            key: "lua:S1135".into(),
            name: "Track TODO/FIXME (Lua)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates and may represent critical missing functionality.".into(),
            how_to_fix: "Create an issue and reference it in the comment.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "lua".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "lua:S2068".into(),
            name: "Credentials should not be hard-coded (Lua)".into(),
            description: "Hard-coded credentials in Lua scripts are visible in plain text.".into(),
            why: "Lua scripts are plain text and easily readable. Credentials in source are captured in VCS history permanently.".into(),
            how_to_fix: "Use `os.getenv('SECRET')` or read from a configuration file excluded from version control.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "lua".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "lua:S1481".into(),
            name: "Global variable implicitly created — use local".into(),
            description: "Lua variables are global by default unless declared with `local`. Implicit globals pollute the global namespace.".into(),
            why: "Implicit globals in Lua are shared across all modules and can cause hard-to-trace bugs when one module accidentally overwrites another's global. They also prevent garbage collection of the value.".into(),
            how_to_fix: "Prepend `local` to every variable declaration: `local x = 42`. Use module patterns to expose only intentional public API.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "lua".into(),
            tags: vec!["bad-practice".into()],
            effort_minutes: 5,
        },

        // ── SCALA ────────────────────────────────────────────────────────────
        SonarRule {
            key: "scala:S2068".into(),
            name: "Credentials should not be hard-coded (Scala)".into(),
            description: "Hard-coded credentials in Scala source code are accessible via decompilation of JVM bytecode.".into(),
            why: "Scala/JVM bytecode can be decompiled easily. Credentials in source are captured in VCS history permanently.".into(),
            how_to_fix: "Use `sys.env.getOrElse(\"SECRET\", \"\")`, the Typesafe config library, or a secrets management service.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "scala".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "scala:S1135".into(),
            name: "Track TODO/FIXME (Scala)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates and may represent critical missing functionality.".into(),
            how_to_fix: "Create a Jira/GitHub issue and reference it in the comment.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "scala".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "scala:S3776".into(),
            name: "Cognitive complexity too high (Scala)".into(),
            description: "Scala functions with deeply nested pattern matches, for-comprehensions, and conditionals are hard to test and maintain.".into(),
            why: "High complexity correlates with higher defect density.".into(),
            how_to_fix: "Extract nested logic into helper functions. Use Scala's functional features (map, flatMap, fold) to reduce explicit nesting.".into(),
            severity: "CRITICAL".into(),
            issue_type: "CODE_SMELL".into(),
            language: "scala".into(),
            tags: vec!["brain-overload".into()],
            effort_minutes: 60,
        },

        // ── DART ─────────────────────────────────────────────────────────────
        SonarRule {
            key: "dart:S2068".into(),
            name: "Credentials should not be hard-coded (Dart)".into(),
            description: "Hard-coded credentials in Dart/Flutter source code can be extracted from compiled apps.".into(),
            why: "Flutter apps can be reverse-engineered. Credentials in source are captured in VCS history permanently.".into(),
            how_to_fix: "Use `String.fromEnvironment('SECRET')` with `--dart-define`, the `flutter_secure_storage` package, or a remote config service.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "dart".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "dart:S1135".into(),
            name: "Track TODO/FIXME (Dart)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates and may represent critical missing functionality.".into(),
            how_to_fix: "Create a GitHub issue and reference it in the comment.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "dart".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "dart:S6532".into(),
            name: "Null assertion operator `!` should be avoided (Dart)".into(),
            description: "The null assertion operator `!` throws a Null check operator used on a null value exception if the value is null.".into(),
            why: "Force-asserting non-null crashes the app at runtime. Dart's null safety is designed to prevent these crashes at compile time.".into(),
            how_to_fix: "Use null-aware operators: `?.`, `??`, or `??=`. Use `if` checks or `assert` with meaningful messages. Redesign APIs to avoid nullable types where possible.".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "dart".into(),
            tags: vec!["null-safety".into(), "crash".into()],
            effort_minutes: 10,
        },

        // ── HASKELL ──────────────────────────────────────────────────────────
        SonarRule {
            key: "haskell:S1135".into(),
            name: "Track TODO/FIXME (Haskell)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates and may represent critical missing functionality.".into(),
            how_to_fix: "Create an issue and reference it. Haskell's `error` function can be used as a typed TODO: `myFunc = error \"TODO: implement\"`".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "haskell".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "haskell:S2068".into(),
            name: "Credentials should not be hard-coded (Haskell)".into(),
            description: "Hard-coded credentials in Haskell source code are accessible from the compiled binary and VCS history.".into(),
            why: "Haskell binaries embed string literals. Credentials in source are captured in VCS history permanently.".into(),
            how_to_fix: "Use `System.Environment.getEnv \"SECRET\"` or a configuration library like `configurator`.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "haskell".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "haskell:S4481".into(),
            name: "Partial functions (head, tail) should not be called on potentially empty lists".into(),
            description: "`head []` and `tail []` throw exceptions at runtime. These are partial functions — they are not defined for all inputs.".into(),
            why: "Calling head or tail on an empty list causes an unrecoverable runtime exception: `Prelude.head: empty list`. This violates Haskell's promise of totality.".into(),
            how_to_fix: "Use pattern matching (`case xs of { [] -> ...; (x:_) -> ... }`), `listToMaybe` from Data.Maybe, or safe variants from the `safe` package.".into(),
            severity: "CRITICAL".into(),
            issue_type: "BUG".into(),
            language: "haskell".into(),
            tags: vec!["partial-functions".into()],
            effort_minutes: 15,
        },

        // ── COBOL ────────────────────────────────────────────────────────────
        SonarRule {
            key: "cobol:S1135".into(),
            name: "Track TODO/FIXME (COBOL)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "In COBOL codebases, TODOs often represent critical business logic that was deferred.".into(),
            how_to_fix: "Create an issue and reference it in the comment.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "cobol".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "cobol:S2068".into(),
            name: "Credentials should not be hard-coded (COBOL)".into(),
            description: "Hard-coded credentials in COBOL source code are a critical security risk in mainframe environments.".into(),
            why: "COBOL source stored in version control or PANVALET can be read by anyone with repository access. Hard-coded credentials are a common finding in mainframe security audits.".into(),
            how_to_fix: "Use RACF-managed credentials, External Security Manager (ESM) profiles, or read from a secured PDS member at runtime.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "cobol".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "cobol:S1186".into(),
            name: "MOVE SPACES / MOVE ZEROS on large tables is a performance risk".into(),
            description: "Using MOVE SPACES or MOVE ZEROS to initialize large working storage tables in a loop is inefficient.".into(),
            why: "Initializing large tables element-by-element in a PERFORM loop is significantly slower than using INITIALIZE or VALUE clauses.".into(),
            how_to_fix: "Use the INITIALIZE statement to clear a table efficiently. Set default values via VALUE clauses in the DATA DIVISION where possible.".into(),
            severity: "MINOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "cobol".into(),
            tags: vec!["performance".into()],
            effort_minutes: 10,
        },

        // ── FORTRAN ──────────────────────────────────────────────────────────
        SonarRule {
            key: "fortran:S1135".into(),
            name: "Track TODO/FIXME (Fortran)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt in scientific Fortran code often represents numerical stability issues or missing edge case handling.".into(),
            how_to_fix: "Create an issue and reference it in the comment.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "fortran".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "fortran:S2068".into(),
            name: "Credentials should not be hard-coded (Fortran)".into(),
            description: "Hard-coded credentials in Fortran source code are accessible from binaries and VCS history.".into(),
            why: "Fortran binaries embed string literals that can be extracted. Credentials in source are captured in VCS history permanently.".into(),
            how_to_fix: "Use `GET_ENVIRONMENT_VARIABLE` to read secrets from the environment at runtime.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "fortran".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "fortran:S1481".into(),
            name: "GOTO statements should be avoided".into(),
            description: "GOTO makes control flow difficult to follow and maintain.".into(),
            why: "GOTO creates spaghetti code that is hard to reason about, test, or refactor. Modern Fortran provides structured alternatives for all GOTO use cases.".into(),
            how_to_fix: "Replace GOTO with structured constructs: DO/END DO loops, IF/END IF blocks, CYCLE for continue-like behavior, and EXIT for break-like behavior.".into(),
            severity: "MAJOR".into(),
            issue_type: "CODE_SMELL".into(),
            language: "fortran".into(),
            tags: vec!["bad-practice".into()],
            effort_minutes: 30,
        },

        // ── R ────────────────────────────────────────────────────────────────
        SonarRule {
            key: "r:S1135".into(),
            name: "Track TODO/FIXME (R)".into(),
            description: "TODO and FIXME comments mark acknowledged technical debt.".into(),
            why: "Untracked debt accumulates. In R scripts, TODOs often represent missing data validation or statistical assumption checks.".into(),
            how_to_fix: "Create an issue and reference it in the comment.".into(),
            severity: "INFO".into(),
            issue_type: "CODE_SMELL".into(),
            language: "r".into(),
            tags: vec!["convention".into()],
            effort_minutes: 0,
        },
        SonarRule {
            key: "r:S2068".into(),
            name: "Credentials should not be hard-coded (R)".into(),
            description: "Hard-coded credentials in R scripts are visible in plain text.".into(),
            why: "R scripts are plain text and easily readable. Credentials in source are captured in VCS history permanently.".into(),
            how_to_fix: "Use `Sys.getenv('SECRET')`, the `keyring` package, or `.Renviron` file excluded from VCS.".into(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            language: "r".into(),
            tags: vec!["cwe-798".into()],
            effort_minutes: 5,
        },
        SonarRule {
            key: "r:S3518".into(),
            name: "T/F should not be used instead of TRUE/FALSE".into(),
            description: "In R, `T` and `F` are variables initialized to TRUE and FALSE, but they can be overwritten. `TRUE` and `FALSE` are reserved keywords that cannot be reassigned.".into(),
            why: "If any code in your session reassigns `T <- 0` or `F <- 1`, all uses of T/F as booleans silently produce wrong results.".into(),
            how_to_fix: "Always use `TRUE` and `FALSE` instead of `T` and `F`. This is also recommended by the tidyverse style guide.".into(),
            severity: "MAJOR".into(),
            issue_type: "BUG".into(),
            language: "r".into(),
            tags: vec!["pitfall".into()],
            effort_minutes: 5,
        },
        // ── Visual Basic / VBScript ─────────────────────────────────────
        SonarRule {
            key: "vb:S2068".into(), name: "Credentials should not be hard-coded (Visual Basic)".into(),
            description: "Hard-coded credentials in VB source are accessible to anyone with access to the binary or source.".into(),
            why: "VB projects are often checked into source control or distributed as compiled binaries that can be decompiled.".into(),
            how_to_fix: "Read credentials from environment variables, app.config with encryption, or Windows Credential Manager.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "vb".into(),
            tags: vec!["cwe-798".into()], effort_minutes: 5,
        },
        SonarRule {
            key: "vb:S1135".into(), name: "Track TODO/FIXME tags (Visual Basic)".into(),
            description: "TODO and FIXME comments mark unfinished work that should be tracked in an issue system.".into(),
            why: "Untracked technical debt accumulates and is rarely addressed without a formal ticket.".into(),
            how_to_fix: "Create a ticket for each TODO/FIXME and reference it in the comment.".into(),
            severity: "INFO".into(), issue_type: "CODE_SMELL".into(), language: "vb".into(),
            tags: vec!["convention".into()], effort_minutes: 0,
        },
        SonarRule {
            key: "vb:S3403".into(), name: "Use `=` for string comparisons, not `Is`".into(),
            description: "`Is` performs reference equality in VB, not value equality for strings. `string1 Is string2` may return False even when both strings have the same text.".into(),
            why: "Reference comparison instead of value comparison is a very common VB bug that causes incorrect equality checks.".into(),
            how_to_fix: "Use `=` or `String.Equals()` for string value comparison. Reserve `Is` for Nothing checks: `If obj Is Nothing`.".into(),
            severity: "CRITICAL".into(), issue_type: "BUG".into(), language: "vb".into(),
            tags: vec!["pitfall".into()], effort_minutes: 5,
        },
        // ── Delphi / Object Pascal ──────────────────────────────────────
        SonarRule {
            key: "delphi:S2068".into(), name: "Credentials should not be hard-coded (Delphi)".into(),
            description: "Hard-coded passwords or API keys in Delphi source code or compiled executables are a security risk.".into(),
            why: "Delphi binaries can be decompiled with tools like IDA Pro or DeDe, revealing hard-coded strings.".into(),
            how_to_fix: "Store credentials in encrypted INI files, the Windows Registry with ACL protection, or a dedicated secrets manager.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "delphi".into(),
            tags: vec!["cwe-798".into()], effort_minutes: 5,
        },
        SonarRule {
            key: "delphi:S1135".into(), name: "Track TODO/FIXME tags (Delphi)".into(),
            description: "TODO and FIXME markers in Delphi code represent acknowledged technical debt.".into(),
            why: "Untracked comments are rarely actioned. Delphi codebases are often legacy and accumulate debt silently.".into(),
            how_to_fix: "Create tickets for each marker and reference the ticket ID in the comment.".into(),
            severity: "INFO".into(), issue_type: "CODE_SMELL".into(), language: "delphi".into(),
            tags: vec!["convention".into()], effort_minutes: 0,
        },
        SonarRule {
            key: "delphi:S1186".into(), name: "Assigned() should be used instead of `<> nil`".into(),
            description: "In Delphi, comparing a pointer/object to `nil` with `<>` is less readable than using the `Assigned()` function.".into(),
            why: "`Assigned(x)` clearly communicates the intent (is this pointer valid?) and is idiomatic Delphi. Raw nil comparisons can be confused with value comparisons.".into(),
            how_to_fix: "Replace `if x <> nil then` with `if Assigned(x) then`.".into(),
            severity: "MINOR".into(), issue_type: "CODE_SMELL".into(), language: "delphi".into(),
            tags: vec!["convention".into()], effort_minutes: 2,
        },
        // ── MATLAB ──────────────────────────────────────────────────────
        SonarRule {
            key: "matlab:S2068".into(), name: "Credentials should not be hard-coded (MATLAB)".into(),
            description: "Hard-coded passwords or API keys in MATLAB scripts are accessible to anyone with access to the .m files.".into(),
            why: "MATLAB scripts are often shared in academic or engineering environments without access controls.".into(),
            how_to_fix: "Use `getenv('MY_API_KEY')` to read secrets from environment variables, or use MATLAB's `keyring` functionality.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "matlab".into(),
            tags: vec!["cwe-798".into()], effort_minutes: 5,
        },
        SonarRule {
            key: "matlab:S1135".into(), name: "Track TODO/FIXME tags (MATLAB)".into(),
            description: "TODO and FIXME comments in MATLAB code are untracked technical debt.".into(),
            why: "MATLAB scripts in research environments are rarely revisited; untracked issues become permanent.".into(),
            how_to_fix: "Create GitHub/JIRA issues and reference them in comments.".into(),
            severity: "INFO".into(), issue_type: "CODE_SMELL".into(), language: "matlab".into(),
            tags: vec!["convention".into()], effort_minutes: 0,
        },
        SonarRule {
            key: "matlab:S1481".into(), name: "Suppress output with `;` to avoid cluttering the console".into(),
            description: "MATLAB statements without a trailing semicolon print their result to the console, which is usually unintended in production code.".into(),
            why: "Unintended console output slows execution, floods logs, and often indicates a debugging statement left behind.".into(),
            how_to_fix: "Add `;` at the end of each statement unless you explicitly want to display the result.".into(),
            severity: "MINOR".into(), issue_type: "CODE_SMELL".into(), language: "matlab".into(),
            tags: vec!["convention".into()], effort_minutes: 1,
        },
        // ── Ada ─────────────────────────────────────────────────────────
        SonarRule {
            key: "ada:S2068".into(), name: "Credentials should not be hard-coded (Ada)".into(),
            description: "Hard-coded secrets in Ada source code violate separation of code and configuration.".into(),
            why: "Ada is widely used in safety-critical systems (avionics, defense). Credential leaks in such systems have severe consequences.".into(),
            how_to_fix: "Read secrets from environment variables via `Ada.Environment_Variables.Value(\"KEY\")` or a secure configuration file.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "ada".into(),
            tags: vec!["cwe-798".into(), "safety-critical".into()], effort_minutes: 5,
        },
        SonarRule {
            key: "ada:S1135".into(), name: "Track TODO/FIXME tags (Ada)".into(),
            description: "TODO/FIXME comments in Ada code mark unresolved issues.".into(),
            why: "Ada is often used in long-lived safety-critical systems. Untracked issues can affect certification and safety audits.".into(),
            how_to_fix: "Track each TODO in your issue management system with the issue ID referenced in the comment.".into(),
            severity: "INFO".into(), issue_type: "CODE_SMELL".into(), language: "ada".into(),
            tags: vec!["convention".into()], effort_minutes: 0,
        },
        // ── Prolog ──────────────────────────────────────────────────────
        SonarRule {
            key: "prolog:S1135".into(), name: "Track TODO/FIXME tags (Prolog)".into(),
            description: "TODO and FIXME comments in Prolog code represent unresolved logic gaps or optimizations.".into(),
            why: "Prolog programs are often research or AI code with fragile logic; unresolved issues can cause incorrect inference.".into(),
            how_to_fix: "Reference a tracking issue ID in the comment.".into(),
            severity: "INFO".into(), issue_type: "CODE_SMELL".into(), language: "prolog".into(),
            tags: vec!["convention".into()], effort_minutes: 0,
        },
        SonarRule {
            key: "prolog:S2068".into(), name: "Credentials should not be hard-coded (Prolog)".into(),
            description: "Hard-coded API keys or passwords in Prolog source are a security risk.".into(),
            why: "Prolog source files are plain text and easily read by anyone with file access.".into(),
            how_to_fix: "Use environment variable reading predicates such as `getenv/2` (SWI-Prolog) to retrieve secrets at runtime.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "prolog".into(),
            tags: vec!["cwe-798".into()], effort_minutes: 5,
        },
        // ── SAS ─────────────────────────────────────────────────────────
        SonarRule {
            key: "sas:S2068".into(), name: "Credentials should not be hard-coded (SAS)".into(),
            description: "Hard-coded database passwords or API keys in SAS programs are a critical security risk in enterprise analytics environments.".into(),
            why: "SAS programs are often stored on shared network drives in enterprise environments with broad access.".into(),
            how_to_fix: "Use SAS macro variables loaded from a secured external file, or use PROC PWENCODE / SAS Vault for credential storage.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "sas".into(),
            tags: vec!["cwe-798".into()], effort_minutes: 5,
        },
        SonarRule {
            key: "sas:S1135".into(), name: "Track TODO/FIXME tags (SAS)".into(),
            description: "TODO and FIXME comments in SAS code mark unresolved analytical or code issues.".into(),
            why: "SAS programs are often long-lived in enterprise analytics contexts, accumulating unaddressed technical debt.".into(),
            how_to_fix: "Reference a ticket in the comment and address in a planned sprint.".into(),
            severity: "INFO".into(), issue_type: "CODE_SMELL".into(), language: "sas".into(),
            tags: vec!["convention".into()], effort_minutes: 0,
        },
        // ── Objective-C ─────────────────────────────────────────────────
        SonarRule {
            key: "objc:S2068".into(), name: "Credentials should not be hard-coded (Objective-C)".into(),
            description: "Hard-coded API keys or secrets in Objective-C iOS/macOS apps can be extracted from the compiled binary.".into(),
            why: "iOS/macOS apps are distributed as signed binaries that can be decompiled with tools like Hopper Disassembler, revealing hard-coded strings.".into(),
            how_to_fix: "Store secrets in the Keychain (`SecItemAdd`/`SecItemCopyMatching`), use environment-specific config files excluded from version control, or fetch secrets from a secure backend at runtime.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "objc".into(),
            tags: vec!["cwe-798".into(), "owasp-mobile-m9".into()], effort_minutes: 30,
        },
        SonarRule {
            key: "objc:S1135".into(), name: "Track TODO/FIXME tags (Objective-C)".into(),
            description: "TODO and FIXME markers in Objective-C code should be tracked in an issue system.".into(),
            why: "Objective-C codebases are often legacy Apple apps where untracked issues persist for years.".into(),
            how_to_fix: "Create a ticket and reference it: `// TODO(#123): fix memory leak`.".into(),
            severity: "INFO".into(), issue_type: "CODE_SMELL".into(), language: "objc".into(),
            tags: vec!["convention".into()], effort_minutes: 0,
        },
        SonarRule {
            key: "objc:S6532".into(), name: "Avoid nil dereference — check before messaging".into(),
            description: "In Objective-C, sending a message to `nil` is a no-op, which can silently swallow errors. In Swift interop or C function calls, nil dereference crashes.".into(),
            why: "Silent nil messaging can mask logic errors where you expected a valid object. In C bridge calls, nil passed as a non-nullable pointer crashes the app.".into(),
            how_to_fix: "Check for nil with `if (obj != nil)` before using objects in critical code paths. Use `NSParameterAssert` in method entry points.".into(),
            severity: "MAJOR".into(), issue_type: "BUG".into(), language: "objc".into(),
            tags: vec!["null-deref".into()], effort_minutes: 10,
        },
        // ── Lisp ────────────────────────────────────────────────────────
        SonarRule {
            key: "lisp:S1135".into(), name: "Track TODO/FIXME tags (Lisp)".into(),
            description: "TODO and FIXME comments in Lisp code mark unresolved issues.".into(),
            why: "Lisp programs tend to be research or AI code where issues can affect correctness of reasoning systems.".into(),
            how_to_fix: "Reference a tracking issue in the comment.".into(),
            severity: "INFO".into(), issue_type: "CODE_SMELL".into(), language: "lisp".into(),
            tags: vec!["convention".into()], effort_minutes: 0,
        },
        SonarRule {
            key: "lisp:S2068".into(), name: "Credentials should not be hard-coded (Lisp)".into(),
            description: "Hard-coded passwords or API keys in Lisp/Common Lisp source code expose secrets.".into(),
            why: "Lisp source files are plain text; secrets committed to source control are permanently exposed.".into(),
            how_to_fix: "Use `uiop:getenv` (Common Lisp) or equivalent to read secrets from environment variables.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "lisp".into(),
            tags: vec!["cwe-798".into()], effort_minutes: 5,
        },
        // ── Julia ────────────────────────────────────────────────────────
        SonarRule {
            key: "julia:S2068".into(), name: "Credentials should not be hard-coded (Julia)".into(),
            description: "Hard-coded API keys or database passwords in Julia scripts are a security risk.".into(),
            why: "Julia scripts are commonly shared in scientific computing environments and Jupyter notebooks, increasing exposure risk.".into(),
            how_to_fix: "Use `ENV[\"MY_SECRET\"]` to read secrets from environment variables, or use a package like `DotEnv.jl`.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "julia".into(),
            tags: vec!["cwe-798".into()], effort_minutes: 5,
        },
        SonarRule {
            key: "julia:S1135".into(), name: "Track TODO/FIXME tags (Julia)".into(),
            description: "TODO and FIXME comments in Julia code mark unresolved scientific or code issues.".into(),
            why: "Julia is widely used for research; untracked issues can affect numerical correctness or reproducibility.".into(),
            how_to_fix: "Create a GitHub issue and reference it in the comment.".into(),
            severity: "INFO".into(), issue_type: "CODE_SMELL".into(), language: "julia".into(),
            tags: vec!["convention".into()], effort_minutes: 0,
        },
        SonarRule {
            key: "julia:S3776".into(), name: "Function cognitive complexity too high (Julia)".into(),
            description: "Julia functions with deep nesting of loops, conditionals, and try/catch blocks are hard to reason about and test.".into(),
            why: "High-complexity Julia functions often mix type-unstable code paths, degrading JIT compilation performance and making bugs harder to find.".into(),
            how_to_fix: "Extract sub-computations into named helper functions. Use multiple dispatch to split behavior by type rather than nesting conditionals.".into(),
            severity: "CRITICAL".into(), issue_type: "CODE_SMELL".into(), language: "julia".into(),
            tags: vec!["brain-overload".into()], effort_minutes: 60,
        },
        // ── OCaml / ML / Caml ────────────────────────────────────────────
        SonarRule {
            key: "ocaml:S2068".into(), name: "Credentials should not be hard-coded (OCaml)".into(),
            description: "Hard-coded secrets in OCaml source code expose credentials in version control.".into(),
            why: "OCaml source is plain text; binaries can also be inspected with `strings`.".into(),
            how_to_fix: "Use `Sys.getenv \"MY_SECRET\"` to read credentials from the environment at runtime.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "ocaml".into(),
            tags: vec!["cwe-798".into()], effort_minutes: 5,
        },
        SonarRule {
            key: "ocaml:S1135".into(), name: "Track TODO/FIXME tags (OCaml)".into(),
            description: "TODO and FIXME comments mark unresolved issues in OCaml code.".into(),
            why: "OCaml is often used in compilers, formal verification, and systems code where unresolved issues have correctness implications.".into(),
            how_to_fix: "Track in an issue system and reference the issue ID.".into(),
            severity: "INFO".into(), issue_type: "CODE_SMELL".into(), language: "ocaml".into(),
            tags: vec!["convention".into()], effort_minutes: 0,
        },
        SonarRule {
            key: "ocaml:S4481".into(), name: "Partial pattern matches should be avoided".into(),
            description: "A `match` expression that does not cover all constructors of a variant type will raise `Match_failure` at runtime for unhandled cases.".into(),
            why: "OCaml's type system guarantees exhaustive matches are safe. Non-exhaustive matches are a runtime crash waiting to happen when new variant cases are added.".into(),
            how_to_fix: "Add explicit arms for all constructors, or add a catch-all `| _ ->` arm with an appropriate error/fallback. Enable `-warn-error +8` to treat non-exhaustive matches as errors.".into(),
            severity: "CRITICAL".into(), issue_type: "BUG".into(), language: "ocaml".into(),
            tags: vec!["partial-function".into()], effort_minutes: 10,
        },
        // ── Erlang ────────────────────────────────────────────────────────
        SonarRule {
            key: "erlang:S2068".into(), name: "Credentials should not be hard-coded (Erlang)".into(),
            description: "Hard-coded secrets in Erlang source code are a security risk.".into(),
            why: "Erlang is widely used in telco and financial systems where credential exposure can have severe compliance implications.".into(),
            how_to_fix: "Use `os:getenv(\"MY_SECRET\")` to read secrets from the environment, or store in a protected sys.config with restricted file permissions.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "erlang".into(),
            tags: vec!["cwe-798".into()], effort_minutes: 5,
        },
        SonarRule {
            key: "erlang:S1135".into(), name: "Track TODO/FIXME tags (Erlang)".into(),
            description: "TODO and FIXME markers in Erlang code mark unresolved concurrency or reliability issues.".into(),
            why: "Erlang systems are often long-running production services; untracked issues can affect uptime SLAs.".into(),
            how_to_fix: "Create an issue and reference it: `%% TODO(#456): handle timeout case`.".into(),
            severity: "INFO".into(), issue_type: "CODE_SMELL".into(), language: "erlang".into(),
            tags: vec!["convention".into()], effort_minutes: 0,
        },
        SonarRule {
            key: "erlang:S2704".into(), name: "Errors from spawned processes should be handled".into(),
            description: "Spawning a process with `spawn/3` instead of `spawn_link/3` or `spawn_monitor/3` means errors in the child process are silently ignored.".into(),
            why: "Unlinked processes can die silently. In a fault-tolerant Erlang system, this defeats the purpose of the supervisor tree.".into(),
            how_to_fix: "Use `spawn_link/3` if the parent should crash with the child, or `spawn_monitor/3` if you want to handle the child's death explicitly. Use OTP supervisors for production code.".into(),
            severity: "MAJOR".into(), issue_type: "BUG".into(), language: "erlang".into(),
            tags: vec!["reliability".into()], effort_minutes: 15,
        },
        // ── ABAP ─────────────────────────────────────────────────────────
        SonarRule {
            key: "abap:S2068".into(), name: "Credentials should not be hard-coded (ABAP)".into(),
            description: "Hard-coded database passwords, RFC destinations, or API keys in ABAP programs are a critical security risk in SAP environments.".into(),
            why: "SAP systems handle sensitive business data. Hard-coded credentials in ABAP code accessible to all developers violate the principle of least privilege.".into(),
            how_to_fix: "Store credentials in SM59 RFC destinations with proper authorization, or use the SAP Credential Store / SECSTORE facility.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "abap".into(),
            tags: vec!["cwe-798".into(), "sap-security".into()], effort_minutes: 30,
        },
        SonarRule {
            key: "abap:S1135".into(), name: "Track TODO/FIXME tags (ABAP)".into(),
            description: "TODO and FIXME markers in ABAP code mark unresolved issues.".into(),
            why: "ABAP systems are often critical business applications with long life cycles; untracked issues accumulate into compliance and audit risks.".into(),
            how_to_fix: "Create a SAP Solution Manager ticket or JIRA issue and reference it.".into(),
            severity: "INFO".into(), issue_type: "CODE_SMELL".into(), language: "abap".into(),
            tags: vec!["convention".into()], effort_minutes: 0,
        },
        SonarRule {
            key: "abap:S3649".into(), name: "Dynamic OPEN SQL should not use user-controlled input".into(),
            description: "Using dynamic `SELECT` with user input in ABAP (e.g., building WHERE clause strings) is vulnerable to SQL injection.".into(),
            why: "Dynamic OPEN SQL in ABAP bypasses SAP's standard SQL injection protections when user-controlled strings are directly concatenated.".into(),
            how_to_fix: "Use static OPEN SQL with typed parameters. If dynamic SQL is required, use `cl_abap_sql_statement` with proper escaping.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "abap".into(),
            tags: vec!["cwe-89".into(), "owasp-a03".into()], effort_minutes: 30,
        },
        // ── Assembly ─────────────────────────────────────────────────────
        SonarRule {
            key: "assembly:S2068".into(), name: "Credentials should not be stored as string literals (Assembly)".into(),
            description: "String literals in assembly code (`.ascii`, `.string`, `db` directives) are trivially extractable with `strings` or a hex editor.".into(),
            why: "Assembly binaries are frequently reverse-engineered. Hard-coded credentials are the first thing attackers look for using `strings` on a binary.".into(),
            how_to_fix: "Do not store credentials as string literals. Receive them via system calls from environment variables or encrypted configuration.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "assembly".into(),
            tags: vec!["cwe-798".into()], effort_minutes: 60,
        },
        SonarRule {
            key: "assembly:S1135".into(), name: "Track TODO/FIXME tags (Assembly)".into(),
            description: "TODO and FIXME comments in assembly code mark unresolved low-level issues.".into(),
            why: "Assembly-level bugs (alignment, overflow, register clobbers) are critical and untracked issues can cause subtle system corruption.".into(),
            how_to_fix: "Link each TODO to an issue tracker with a reference in the comment.".into(),
            severity: "INFO".into(), issue_type: "CODE_SMELL".into(), language: "assembly".into(),
            tags: vec!["convention".into()], effort_minutes: 0,
        },
        // ── Zig ──────────────────────────────────────────────────────────
        SonarRule {
            key: "zig:S2068".into(), name: "Credentials should not be hard-coded (Zig)".into(),
            description: "Hard-coded secrets in Zig source code are visible in the binary and source control.".into(),
            why: "Zig is used for systems/embedded programming where binary inspection is common.".into(),
            how_to_fix: "Use `std.os.getenv(\"MY_SECRET\")` or read from a configuration file with restricted permissions.".into(),
            severity: "BLOCKER".into(), issue_type: "VULNERABILITY".into(), language: "zig".into(),
            tags: vec!["cwe-798".into()], effort_minutes: 5,
        },
        SonarRule {
            key: "zig:S1135".into(), name: "Track TODO/FIXME tags (Zig)".into(),
            description: "TODO and FIXME comments in Zig code mark unresolved safety or correctness issues.".into(),
            why: "Zig is designed for safety-critical systems; unresolved issues can cause undefined behavior or security vulnerabilities.".into(),
            how_to_fix: "Zig has a built-in `@panic(\"TODO\")` — use it to make TODOs fail loudly at runtime. Track in an issue system.".into(),
            severity: "INFO".into(), issue_type: "CODE_SMELL".into(), language: "zig".into(),
            tags: vec!["convention".into()], effort_minutes: 0,
        },
    ]
}

// ── SQLite Store ───────────────────────────────────────────────────────────────

fn db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".vibecli").join("sonar_rules.db")
}

fn open_db() -> rusqlite::Result<Connection> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sonar_rules (
            key           TEXT PRIMARY KEY,
            name          TEXT NOT NULL,
            description   TEXT NOT NULL,
            why           TEXT NOT NULL,
            how_to_fix    TEXT NOT NULL,
            severity      TEXT NOT NULL,
            issue_type    TEXT NOT NULL,
            language      TEXT NOT NULL,
            tags          TEXT NOT NULL,
            effort_minutes INTEGER NOT NULL
        );",
    )?;
    Ok(conn)
}

/// Upsert all built-in rules into the local SQLite database.
/// Returns the number of rules loaded.
pub fn load_rules_to_db() -> Result<u32, String> {
    let conn = open_db().map_err(|e| e.to_string())?;
    let rules = builtin_rules();
    let count = rules.len() as u32;
    for r in &rules {
        let tags_json = serde_json::to_string(&r.tags).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO sonar_rules
             (key,name,description,why,how_to_fix,severity,issue_type,language,tags,effort_minutes)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![r.key, r.name, r.description, r.why, r.how_to_fix,
                    r.severity, r.issue_type, r.language, tags_json, r.effort_minutes],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(count)
}

/// Retrieve all rules from the local DB (falls back to builtin if DB missing).
pub fn get_rules(language: Option<&str>) -> Vec<SonarRule> {
    let rules = match open_db() {
        Ok(conn) => {
            let sql = if language.is_some() {
                "SELECT key,name,description,why,how_to_fix,severity,issue_type,language,tags,effort_minutes FROM sonar_rules WHERE language = ?1 OR language = 'general'"
            } else {
                "SELECT key,name,description,why,how_to_fix,severity,issue_type,language,tags,effort_minutes FROM sonar_rules"
            };
            let mut stmt = match conn.prepare(sql) {
                Ok(s) => s,
                Err(_) => return builtin_rules(),
            };
            let mapper = |row: &rusqlite::Row<'_>| -> rusqlite::Result<SonarRule> {
                let tags_json: String = row.get(8)?;
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                Ok(SonarRule {
                    key: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    why: row.get(3)?,
                    how_to_fix: row.get(4)?,
                    severity: row.get(5)?,
                    issue_type: row.get(6)?,
                    language: row.get(7)?,
                    tags,
                    effort_minutes: row.get(9)?,
                })
            };
            let rows = if let Some(lang) = language {
                stmt.query_map(params![lang], mapper)
            } else {
                stmt.query_map([], mapper)
            };
            match rows {
                Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
                Err(_) => builtin_rules(),
            }
        }
        Err(_) => builtin_rules(),
    };
    if rules.is_empty() { builtin_rules() } else { rules }
}

// ── Pattern-based Scanner ─────────────────────────────────────────────────────

struct RulePattern {
    rule_key: &'static str,
    /// (line_content) → Option<(col_start, matched_fragment, message)>
    matcher: fn(&str) -> Option<(u32, String, String)>,
}

fn patterns() -> Vec<RulePattern> {
    vec![
        RulePattern {
            rule_key: "typescript:S2068",
            matcher: |line| {
                let low = line.to_lowercase();
                let triggers = [
                    ("password", "Hardcoded password"),
                    ("passwd", "Hardcoded password"),
                    ("secret", "Hardcoded secret"),
                    ("api_key", "Hardcoded API key"),
                    ("apikey", "Hardcoded API key"),
                    ("access_key", "Hardcoded access key"),
                    ("auth_token", "Hardcoded auth token"),
                    ("private_key", "Private key in source"),
                    ("AKIA", "AWS access key"),
                ];
                for (kw, msg) in &triggers {
                    if low.contains(kw) && (low.contains("= \"") || low.contains("= '") || low.contains(": \"") || low.contains(": '") || low.contains("=\"") || low.contains("='")) {
                        let col = line.to_lowercase().find(kw).unwrap_or(0) as u32;
                        return Some((col, kw.to_string(), msg.to_string()));
                    }
                }
                if line.contains("AKIA") {
                    let col = line.find("AKIA").unwrap_or(0) as u32;
                    return Some((col, "AKIA".into(), "Potential AWS access key".into()));
                }
                None
            },
        },
        RulePattern {
            rule_key: "typescript:S5332",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") || trimmed.starts_with('#') { return None; }
                if let Some(pos) = line.find("http://") {
                    if !line[..pos].contains("//") {
                        return Some((pos as u32, "http://".into(), "Insecure HTTP URL — use https:// instead".into()));
                    }
                }
                None
            },
        },
        RulePattern {
            rule_key: "typescript:S3649",
            matcher: |line| {
                let low = line.to_lowercase();
                let trimmed = line.trim();
                if trimmed.starts_with("//") || trimmed.starts_with('#') { return None; }
                let sql_kws = ["select ", "insert into", "update ", "delete from"];
                for kw in &sql_kws {
                    if low.contains(kw) && (line.contains('+') || line.contains("${") || line.contains("format!") || line.contains('`') || line.contains("f\"")) {
                        let col = low.find(kw).unwrap_or(0) as u32;
                        return Some((col, kw.to_string(), format!("Potential SQL injection — '{}' query built via string concatenation/interpolation", kw.trim())));
                    }
                }
                None
            },
        },
        RulePattern {
            rule_key: "typescript:S6096",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") { return None; }
                for kw in &["innerHTML", "document.write", "dangerouslySetInnerHTML"] {
                    if let Some(pos) = line.find(kw) {
                        return Some((pos as u32, kw.to_string(), format!("Potential XSS — `{}` with user input renders unsanitized HTML", kw)));
                    }
                }
                None
            },
        },
        RulePattern {
            rule_key: "typescript:S2737",
            matcher: |line| {
                let trimmed = line.trim();
                // Detect `catch` followed immediately by `}` or empty block indicators
                if (trimmed == "} catch (e) {}" || trimmed == "} catch (_) {}" || trimmed == "catch (e) {}")
                    || (trimmed.starts_with("catch") && trimmed.ends_with("{}"))
                {
                    let col = line.find("catch").unwrap_or(0) as u32;
                    return Some((col, "catch {}".into(), "Empty catch block silently swallows errors — log or re-throw".into()));
                }
                None
            },
        },
        RulePattern {
            rule_key: "typescript:S3403",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") { return None; }
                // Find == or != but not === / !==
                let bytes = line.as_bytes();
                let len = bytes.len();
                for i in 0..len.saturating_sub(1) {
                    if (bytes[i] == b'!' || bytes[i] == b'=') && bytes[i+1] == b'=' {
                        let is_strict = i + 2 < len && bytes[i+2] == b'=';
                        let is_preceded_by_excl_or_eq = i > 0 && (bytes[i-1] == b'!' || bytes[i-1] == b'=' || bytes[i-1] == b'<' || bytes[i-1] == b'>');
                        if !is_strict && !is_preceded_by_excl_or_eq {
                            let op = if bytes[i] == b'!' { "!=" } else { "==" };
                            return Some((i as u32, op.into(), format!("Use `{}=` (strict equality) instead of `{}`", if bytes[i] == b'!' { "!" } else { "=" }, op)));
                        }
                    }
                }
                None
            },
        },
        RulePattern {
            rule_key: "typescript:S1125",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") { return None; }
                for pattern in &["=== true", "=== false", "== true", "== false", "!== true", "!== false"] {
                    if let Some(pos) = line.find(pattern) {
                        return Some((pos as u32, pattern.to_string(), format!("Redundant boolean literal `{pattern}` — simplify the expression")));
                    }
                }
                None
            },
        },
        RulePattern {
            rule_key: "typescript:S1135",
            matcher: |line| {
                let trimmed = line.trim();
                for marker in &["TODO", "FIXME", "HACK", "XXX"] {
                    if let Some(pos) = trimmed.find(marker) {
                        let abs_col = line.find(marker).unwrap_or(pos) as u32;
                        return Some((abs_col, marker.to_string(), format!("`{marker}` comment — track this in your issue tracker")));
                    }
                }
                None
            },
        },
        RulePattern {
            rule_key: "typescript:S4325",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") { return None; }
                // Detect `: any` type annotation (TypeScript)
                if let Some(pos) = line.find(": any") {
                    // Avoid false positives in comments
                    if !trimmed.starts_with("//") && !trimmed.starts_with("*") {
                        return Some((pos as u32, ": any".into(), "Avoid `any` — use a specific type or `unknown` for safer type handling".into()));
                    }
                }
                if let Some(pos) = line.find("<any>") {
                    return Some((pos as u32, "<any>".into(), "Avoid `any` cast — use a specific type or type guard".into()));
                }
                if let Some(pos) = line.find("as any") {
                    return Some((pos as u32, "as any".into(), "Avoid `as any` cast — use a specific type or `as unknown as T`".into()));
                }
                None
            },
        },
        RulePattern {
            rule_key: "typescript:S1764",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") { return None; }
                // Check for x === x or x == x patterns (simple identifiers)
                // Very basic: look for patterns like `word === word` or `word == word`
                for op in &[" === ", " == ", " !== ", " != "] {
                    if let Some(pos) = line.find(op) {
                        let before = line[..pos].trim();
                        let after = line[pos + op.len()..].trim();
                        // Extract last token before op
                        let left_token = before.split_whitespace().last().unwrap_or("");
                        let right_token = after.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.' && c != '\'' && c != '"').next().unwrap_or("");
                        if !left_token.is_empty() && left_token == right_token && left_token.len() > 1 {
                            return Some((pos as u32, op.trim().into(), format!("Identical operands `{left_token} {op_t} {right_token}` — this is always {result}", op_t = op.trim(), result = if op.contains('!') { "false" } else { "true" })));
                        }
                    }
                }
                None
            },
        },
        RulePattern {
            rule_key: "typescript:S3358",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") { return None; }
                // Detect ternaries containing ternaries: `? ... ? ... : ... : ...`
                let q_count = line.chars().filter(|&c| c == '?').count();
                let col_count = line.chars().filter(|&c| c == ':').count();
                if q_count >= 2 && col_count >= 2 {
                    let pos = line.find('?').unwrap_or(0) as u32;
                    return Some((pos, "?...?".into(), "Nested ternary operator — replace with if/else for readability".into()));
                }
                None
            },
        },
        RulePattern {
            rule_key: "typescript:S6509",
            matcher: |line| {
                let trimmed = line.trim();
                // Detect hooks inside if/for/while
                let hook_calls = ["useState(", "useEffect(", "useCallback(", "useMemo(", "useRef(", "useContext(", "useReducer("];
                let in_conditional = trimmed.starts_with("if ") || trimmed.starts_with("if(");
                let _ = in_conditional; // static analysis only — pattern match on consecutive lines requires context
                // Simpler: detect if a line has a hook call AND an if/for/while on the same line
                for hook in &hook_calls {
                    if trimmed.contains(hook) && (line.contains("if (") || line.contains("if(") || line.contains("for (") || line.contains("while (")) {
                        let pos = line.find(hook).unwrap_or(0) as u32;
                        return Some((pos, hook.to_string(), format!("`{hook}` called conditionally — React Hooks must be called at the top level")));
                    }
                }
                None
            },
        },
        RulePattern {
            rule_key: "rust:S2068",
            matcher: |line| {
                let low = line.to_lowercase();
                let trimmed = line.trim();
                if trimmed.starts_with("//") { return None; }
                let kws = [("password", "Hardcoded password"), ("secret", "Hardcoded secret"), ("api_key", "Hardcoded API key"), ("private_key", "Private key")];
                for (kw, msg) in &kws {
                    if low.contains(kw) && (low.contains("= \"") || low.contains("= b\"") || low.contains(": &str = \"")) {
                        let col = low.find(kw).unwrap_or(0) as u32;
                        return Some((col, kw.to_string(), msg.to_string()));
                    }
                }
                None
            },
        },
        RulePattern {
            rule_key: "rust:S1135",
            matcher: |line| {
                let trimmed = line.trim();
                for marker in &["TODO", "FIXME", "HACK", "XXX"] {
                    if trimmed.contains(marker) && (trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*')) {
                        let col = line.find(marker).unwrap_or(0) as u32;
                        return Some((col, marker.to_string(), format!("`{marker}` comment — track in issue tracker")));
                    }
                }
                None
            },
        },

        // ── PYTHON patterns ───────────────────────────────────────────────────
        RulePattern {
            rule_key: "python:S2068",
            matcher: |line| {
                let low = line.to_lowercase();
                let trimmed = line.trim();
                if trimmed.starts_with('#') { return None; }
                let kws = [
                    ("password", "Hardcoded password"),
                    ("secret", "Hardcoded secret"),
                    ("api_key", "Hardcoded API key"),
                    ("passwd", "Hardcoded password"),
                ];
                for (kw, msg) in &kws {
                    if low.contains(kw) && (low.contains("= \"") || low.contains("= '") || low.contains("=\"") || low.contains("='")) {
                        let col = low.find(kw).unwrap_or(0) as u32;
                        return Some((col, kw.to_string(), msg.to_string()));
                    }
                }
                None
            },
        },
        RulePattern {
            rule_key: "python:S5905",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with('#') { return None; }
                if trimmed.starts_with("print(") || trimmed.starts_with("print (") {
                    let col = line.find("print").unwrap_or(0) as u32;
                    return Some((col, "print()".into(), "`print()` in production code — use the `logging` module instead".into()));
                }
                None
            },
        },
        RulePattern {
            rule_key: "python:S2201",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with('#') { return None; }
                if trimmed.starts_with("def ") && trimmed.contains('(')
                    && (trimmed.contains("=[") || trimmed.contains("={"))
                {
                    let col = line.find("def ").unwrap_or(0) as u32;
                    return Some((col, "def".into(), "Mutable default argument — default mutable values are shared across all calls".into()));
                }
                None
            },
        },
        RulePattern {
            rule_key: "python:S1481",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed == "except:" || trimmed.starts_with("except:") {
                    let col = line.find("except").unwrap_or(0) as u32;
                    return Some((col, "except:".into(), "Bare `except:` catches ALL exceptions including SystemExit and KeyboardInterrupt — specify exception types".into()));
                }
                None
            },
        },

        // ── C/C++ patterns ────────────────────────────────────────────────────
        RulePattern {
            rule_key: "c:S3518",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') { return None; }
                for dangerous in &["gets(", "strcpy(", " sprintf(", "strcat("] {
                    if let Some(pos) = line.find(dangerous) {
                        return Some((pos as u32, dangerous.trim().into(), format!("`{}` has no bounds checking — use safe alternatives (fgets, strncpy, snprintf, strncat)", dangerous.trim())));
                    }
                }
                None
            },
        },
        RulePattern {
            rule_key: "cpp:S5445",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") { return None; }
                if let Some(pos) = line.find(" new ") {
                    if !line.contains("unique_ptr") && !line.contains("shared_ptr") && !line.contains("make_unique") && !line.contains("make_shared") {
                        return Some((pos as u32, "new".into(), "Raw `new` without smart pointer — consider std::unique_ptr or std::shared_ptr to prevent memory leaks".into()));
                    }
                }
                None
            },
        },

        // ── JAVA patterns ─────────────────────────────────────────────────────
        RulePattern {
            rule_key: "java:S2068",
            matcher: |line| {
                let low = line.to_lowercase();
                let trimmed = line.trim();
                if trimmed.starts_with("//") || trimmed.starts_with('*') { return None; }
                let kws = [
                    ("password", "Hardcoded password"),
                    ("secret", "Hardcoded secret"),
                    ("apikey", "Hardcoded API key"),
                    ("api_key", "Hardcoded API key"),
                ];
                for (kw, msg) in &kws {
                    if low.contains(kw) && (low.contains("= \"") || low.contains("= '")) {
                        let col = low.find(kw).unwrap_or(0) as u32;
                        return Some((col, kw.to_string(), msg.to_string()));
                    }
                }
                None
            },
        },

        // ── GO patterns ───────────────────────────────────────────────────────
        RulePattern {
            rule_key: "go:S2704",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") { return None; }
                if trimmed.contains(", _") && (trimmed.contains(":=") || trimmed.contains("= ")) {
                    let col = line.find(", _").unwrap_or(0) as u32;
                    return Some((col, ", _".into(), "Error return value ignored via `_` — check the error or propagate it".into()));
                }
                None
            },
        },
        RulePattern {
            rule_key: "go:S6288",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") { return None; }
                if trimmed.starts_with("defer ") && (line.contains("for ") || line.contains("range ")) {
                    let col = line.find("defer").unwrap_or(0) as u32;
                    return Some((col, "defer".into(), "`defer` inside a loop — deferred calls run at function exit, not loop iteration; extract to a helper function".into()));
                }
                None
            },
        },

        // ── PHP patterns ──────────────────────────────────────────────────────
        RulePattern {
            rule_key: "php:S2076",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") || trimmed.starts_with('#') { return None; }
                if let Some(pos) = line.find("eval(") {
                    return Some((pos as u32, "eval(".into(), "`eval()` executes arbitrary PHP — remove and use proper data structures".into()));
                }
                if let Some(pos) = line.find("eval (") {
                    return Some((pos as u32, "eval (".into(), "`eval()` executes arbitrary PHP — remove and use proper data structures".into()));
                }
                None
            },
        },
        RulePattern {
            rule_key: "php:S2068",
            matcher: |line| {
                let low = line.to_lowercase();
                let trimmed = line.trim();
                if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with('*') { return None; }
                let kws = [
                    ("password", "Hardcoded password"),
                    ("secret", "Hardcoded secret"),
                    ("api_key", "Hardcoded API key"),
                ];
                for (kw, msg) in &kws {
                    if low.contains(kw) && (low.contains("= \"") || low.contains("= '") || low.contains("= <<<")) {
                        let col = low.find(kw).unwrap_or(0) as u32;
                        return Some((col, kw.to_string(), msg.to_string()));
                    }
                }
                None
            },
        },

        // ── SWIFT patterns ────────────────────────────────────────────────────
        RulePattern {
            rule_key: "swift:S6532",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") { return None; }
                let bytes = line.as_bytes();
                for i in 0..bytes.len() {
                    if bytes[i] == b'!' {
                        let next = bytes.get(i + 1).copied().unwrap_or(0);
                        let prev = if i > 0 { bytes[i - 1] } else { 0 };
                        if next != b'=' && prev != b'!' && next != b'!' && (prev.is_ascii_alphanumeric() || prev == b')' || prev == b']') {
                            return Some((i as u32, "!".into(), "Force unwrap `!` crashes on nil — use `if let`, `guard let`, or `??` instead".into()));
                        }
                    }
                }
                None
            },
        },

        // ── KOTLIN patterns ───────────────────────────────────────────────────
        RulePattern {
            rule_key: "kotlin:S6531",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") { return None; }
                if let Some(pos) = line.find("!!") {
                    return Some((pos as u32, "!!".into(), "Non-null assertion `!!` throws NullPointerException on null — use safe call `?.`, `?:`, or explicit null check".into()));
                }
                None
            },
        },

        // ── SQL patterns ──────────────────────────────────────────────────────
        RulePattern {
            rule_key: "sql:S2077",
            matcher: |line| {
                let low = line.to_lowercase();
                let trimmed = low.trim();
                if trimmed.starts_with("--") || trimmed.starts_with("/*") { return None; }
                if trimmed.contains("select *") || trimmed.starts_with("select *") {
                    let col = low.find("select *").unwrap_or(0) as u32;
                    return Some((col, "SELECT *".into(), "`SELECT *` retrieves all columns — enumerate only needed columns to avoid schema-change breakage and improve performance".into()));
                }
                None
            },
        },
        RulePattern {
            rule_key: "sql:S3649",
            matcher: |line| {
                let low = line.to_lowercase();
                let trimmed = low.trim().to_string();
                if trimmed.starts_with("--") { return None; }
                let is_dml = trimmed.starts_with("delete ") || trimmed.starts_with("update ");
                if is_dml && !low.contains(" where ") && !low.contains("\nwhere") {
                    return Some((0u32, "DELETE/UPDATE without WHERE".into(), "DELETE or UPDATE without a WHERE clause will affect ALL rows in the table".into()));
                }
                None
            },
        },

        // ── SOLIDITY patterns ─────────────────────────────────────────────────
        RulePattern {
            rule_key: "solidity:S6321",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") { return None; }
                if let Some(pos) = line.find("tx.origin") {
                    return Some((pos as u32, "tx.origin".into(), "`tx.origin` used for authentication — use `msg.sender` instead to prevent phishing attacks".into()));
                }
                None
            },
        },
        RulePattern {
            rule_key: "solidity:S2068",
            matcher: |line| {
                let low = line.to_lowercase();
                let trimmed = line.trim();
                if trimmed.starts_with("//") { return None; }
                let kws = [
                    ("private_key", "Hardcoded private key"),
                    ("secret", "Hardcoded secret"),
                ];
                for (kw, msg) in &kws {
                    if low.contains(kw) && (low.contains("= \"") || low.contains("= '") || low.contains("= 0x")) {
                        let col = low.find(kw).unwrap_or(0) as u32;
                        return Some((col, kw.to_string(), msg.to_string()));
                    }
                }
                None
            },
        },

        // ── RUBY patterns ─────────────────────────────────────────────────────
        RulePattern {
            rule_key: "ruby:S1481",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with('#') { return None; }
                if trimmed.starts_with("eval ") || trimmed.starts_with("eval(") {
                    let col = line.find("eval").unwrap_or(0) as u32;
                    return Some((col, "eval".into(), "`eval` executes arbitrary Ruby code — this is a critical security risk".into()));
                }
                None
            },
        },

        // ── LUA patterns ──────────────────────────────────────────────────────
        RulePattern {
            rule_key: "lua:S1481",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with("--") { return None; }
                let is_assignment = trimmed.contains(" = ")
                    && !trimmed.starts_with("local ")
                    && !trimmed.starts_with("if ")
                    && !trimmed.starts_with("elseif ")
                    && !trimmed.starts_with("while ")
                    && !trimmed.starts_with("for ")
                    && !trimmed.starts_with("return ")
                    && !trimmed.starts_with("function ")
                    && !trimmed.starts_with("--");
                if is_assignment {
                    let lhs = trimmed.split(" = ").next().unwrap_or("").trim();
                    if !lhs.contains('.') && !lhs.contains('[') && !lhs.contains(':') && lhs.chars().all(|c| c.is_alphanumeric() || c == '_') && !lhs.is_empty() {
                        return Some((0u32, lhs.to_string(), format!("`{}` assigned without `local` — may pollute global namespace", lhs)));
                    }
                }
                None
            },
        },

        // ── POWERSHELL patterns ───────────────────────────────────────────────
        RulePattern {
            rule_key: "powershell:S3649",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with('#') { return None; }
                if let Some(pos) = line.find("Invoke-Expression") {
                    return Some((pos as u32, "Invoke-Expression".into(), "`Invoke-Expression` with user input is equivalent to `eval` — use typed parameters and specific cmdlets".into()));
                }
                if let Some(pos) = line.find("iex ") {
                    return Some((pos as u32, "iex".into(), "`iex` (Invoke-Expression alias) — avoid with user-controlled input".into()));
                }
                None
            },
        },
        RulePattern {
            rule_key: "powershell:S2068",
            matcher: |line| {
                let low = line.to_lowercase();
                let trimmed = line.trim();
                if trimmed.starts_with('#') { return None; }
                let kws = [
                    ("password", "Hardcoded password"),
                    ("-password ", "Hardcoded password in parameter"),
                    ("$secret", "Hardcoded secret variable"),
                ];
                for (kw, msg) in &kws {
                    if low.contains(kw) && (low.contains('"') || low.contains('\'')) {
                        let col = low.find(kw).unwrap_or(0) as u32;
                        return Some((col, kw.to_string(), msg.to_string()));
                    }
                }
                None
            },
        },

        // ── R patterns ────────────────────────────────────────────────────────
        RulePattern {
            rule_key: "r:S3518",
            matcher: |line| {
                let trimmed = line.trim();
                if trimmed.starts_with('#') { return None; }
                for token in &[" = T", " = F", "(T)", "(F)", ", T,", ", F,", " T ", " F "] {
                    if line.contains(token) {
                        let col = line.find(token).unwrap_or(0) as u32;
                        return Some((col, token.trim().to_string(), "`T`/`F` used as boolean — use `TRUE`/`FALSE`; `T` and `F` are variables that can be overwritten".into()));
                    }
                }
                None
            },
        },
        // Visual Basic — hardcoded creds
        RulePattern { rule_key: "vb:S2068", matcher: |line| {
            let low = line.to_lowercase(); let trimmed = line.trim();
            if trimmed.starts_with('\'') { return None; }
            let kws = [("password", "Hardcoded password"), ("secret", "Hardcoded secret"), ("apikey", "Hardcoded API key"), ("api_key", "Hardcoded API key")];
            for (kw, msg) in &kws {
                if low.contains(kw) && (low.contains("= \"") || low.contains("= '")) {
                    let col = low.find(kw).unwrap_or(0) as u32;
                    return Some((col, kw.to_string(), msg.to_string()));
                }
            }
            None
        }},
        // Objective-C — hardcoded creds
        RulePattern { rule_key: "objc:S2068", matcher: |line| {
            let low = line.to_lowercase(); let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') { return None; }
            let kws = [("password", "Hardcoded password"), ("secret", "Hardcoded secret"), ("apikey", "Hardcoded API key"), ("api_key", "Hardcoded API key")];
            for (kw, msg) in &kws {
                if low.contains(kw) && (low.contains("= @\"") || low.contains("= \"")) {
                    let col = low.find(kw).unwrap_or(0) as u32;
                    return Some((col, kw.to_string(), msg.to_string()));
                }
            }
            None
        }},
        // Julia — hardcoded creds
        RulePattern { rule_key: "julia:S2068", matcher: |line| {
            let low = line.to_lowercase(); let trimmed = line.trim();
            if trimmed.starts_with('#') { return None; }
            let kws = [("password", "Hardcoded password"), ("secret", "Hardcoded secret"), ("api_key", "Hardcoded API key")];
            for (kw, msg) in &kws {
                if low.contains(kw) && (low.contains("= \"") || low.contains("= '")) {
                    let col = low.find(kw).unwrap_or(0) as u32;
                    return Some((col, kw.to_string(), msg.to_string()));
                }
            }
            None
        }},
        // Erlang — hardcoded creds
        RulePattern { rule_key: "erlang:S2068", matcher: |line| {
            let low = line.to_lowercase(); let trimmed = line.trim();
            if trimmed.starts_with('%') { return None; }
            let kws = [("password", "Hardcoded password"), ("secret", "Hardcoded secret"), ("api_key", "Hardcoded API key")];
            for (kw, msg) in &kws {
                if low.contains(kw) && (low.contains("<<\"") || low.contains("= \"") || low.contains(": \"")) {
                    let col = low.find(kw).unwrap_or(0) as u32;
                    return Some((col, kw.to_string(), msg.to_string()));
                }
            }
            None
        }},
        // OCaml — hardcoded creds
        RulePattern { rule_key: "ocaml:S2068", matcher: |line| {
            let low = line.to_lowercase(); let trimmed = line.trim();
            if trimmed.starts_with("(*") || trimmed.starts_with('*') { return None; }
            let kws = [("password", "Hardcoded password"), ("secret", "Hardcoded secret"), ("api_key", "Hardcoded API key")];
            for (kw, msg) in &kws {
                if low.contains(kw) && (low.contains("= \"") || low.contains("\"")) {
                    let col = low.find(kw).unwrap_or(0) as u32;
                    return Some((col, kw.to_string(), msg.to_string()));
                }
            }
            None
        }},
        // MATLAB — hardcoded creds
        RulePattern { rule_key: "matlab:S2068", matcher: |line| {
            let low = line.to_lowercase(); let trimmed = line.trim();
            if trimmed.starts_with('%') { return None; }
            let kws = [("password", "Hardcoded password"), ("secret", "Hardcoded secret"), ("api_key", "Hardcoded API key")];
            for (kw, msg) in &kws {
                if low.contains(kw) && (low.contains("= '") || low.contains("= \"")) {
                    let col = low.find(kw).unwrap_or(0) as u32;
                    return Some((col, kw.to_string(), msg.to_string()));
                }
            }
            None
        }},
        // ABAP — dynamic SQL / hardcoded creds
        RulePattern { rule_key: "abap:S3649", matcher: |line| {
            let low = line.to_lowercase(); let trimmed = line.trim();
            if trimmed.starts_with('*') || trimmed.starts_with('"') { return None; }
            if low.contains("select") && (low.contains("&") || low.contains("(lv_") || low.contains("(v_")) {
                let col = low.find("select").unwrap_or(0) as u32;
                return Some((col, "SELECT".into(), "Dynamic OPEN SQL may be vulnerable to injection — use static SQL or parameterized queries".into()));
            }
            None
        }},
        RulePattern { rule_key: "abap:S2068", matcher: |line| {
            let low = line.to_lowercase(); let trimmed = line.trim();
            if trimmed.starts_with('*') || trimmed.starts_with('"') { return None; }
            let kws = [("password", "Hardcoded password"), ("passwd", "Hardcoded password"), ("secret", "Hardcoded secret")];
            for (kw, msg) in &kws {
                if low.contains(kw) && (low.contains("= '") || low.contains("= \"") || low.contains("value '")) {
                    let col = low.find(kw).unwrap_or(0) as u32;
                    return Some((col, kw.to_string(), msg.to_string()));
                }
            }
            None
        }},
        // Delphi — hardcoded creds
        RulePattern { rule_key: "delphi:S2068", matcher: |line| {
            let low = line.to_lowercase(); let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('{') { return None; }
            let kws = [("password", "Hardcoded password"), ("secret", "Hardcoded secret"), ("apikey", "Hardcoded API key")];
            for (kw, msg) in &kws {
                if low.contains(kw) && (low.contains(":= '") || low.contains("= '") || low.contains(":= \"")) {
                    let col = low.find(kw).unwrap_or(0) as u32;
                    return Some((col, kw.to_string(), msg.to_string()));
                }
            }
            None
        }},
        // Zig — hardcoded creds
        RulePattern { rule_key: "zig:S2068", matcher: |line| {
            let low = line.to_lowercase(); let trimmed = line.trim();
            if trimmed.starts_with("//") { return None; }
            let kws = [("password", "Hardcoded password"), ("secret", "Hardcoded secret"), ("api_key", "Hardcoded API key")];
            for (kw, msg) in &kws {
                if low.contains(kw) && (low.contains("= \"") || low.contains("= '")) {
                    let col = low.find(kw).unwrap_or(0) as u32;
                    return Some((col, kw.to_string(), msg.to_string()));
                }
            }
            None
        }},
    ]
}

// ── Public Scan API ────────────────────────────────────────────────────────────

pub fn scan_content(file_path: &str, content: &str) -> SonarScanResult {
    let rules_map: std::collections::HashMap<String, SonarRule> = builtin_rules()
        .into_iter()
        .map(|r| (r.key.clone(), r))
        .collect();

    let pats = patterns();
    let lines: Vec<&str> = content.lines().collect();
    let mut issues: Vec<SonarIssue> = Vec::new();

    // Also detect high-complexity functions (S3776) via nesting depth
    let mut max_depth = 0u32;
    let mut current_depth = 0u32;
    let mut complexity_line = 0u32;
    for (idx, line) in lines.iter().enumerate() {
        let open = line.chars().filter(|&c| c == '{').count() as u32;
        let close = line.chars().filter(|&c| c == '}').count() as u32;
        current_depth = current_depth.saturating_add(open).saturating_sub(close);
        if current_depth > max_depth {
            max_depth = current_depth;
            complexity_line = idx as u32 + 1;
        }
    }
    if max_depth >= 5 {
        let ext_for_complexity = file_path.rsplit('.').next().unwrap_or("");
        let complexity_rule_key = match ext_for_complexity {
            "rs"                         => "rust:S3776",
            "py" | "pyw"                 => "python:S3776",
            "kt" | "kts"                 => "kotlin:S3776",
            "scala" | "sc"               => "scala:S3776",
            "ts" | "tsx" | "js" | "jsx"  => "typescript:S3776",
            _                            => "typescript:S3776",
        };
        let key = complexity_rule_key;
        if let Some(rule) = rules_map.get(key) {
            let snippet = lines.get(complexity_line.saturating_sub(1) as usize).copied().unwrap_or("").to_string();
            issues.push(SonarIssue {
                rule_key: rule.key.clone(),
                rule_name: rule.name.clone(),
                file: file_path.to_string(),
                line: complexity_line,
                end_line: complexity_line,
                col_start: 0,
                message: format!("Nesting depth reaches {} — function cognitive complexity is too high (threshold: 5)", max_depth),
                severity: rule.severity.clone(),
                issue_type: rule.issue_type.clone(),
                code_snippet: snippet,
                context_before: lines.get(complexity_line.saturating_sub(2) as usize).copied().unwrap_or("").to_string(),
                context_after: lines.get(complexity_line as usize).copied().unwrap_or("").to_string(),
                why: rule.why.clone(),
                how_to_fix: rule.how_to_fix.clone(),
                effort: format!("{}min", rule.effort_minutes),
            });
        }
    }

    // Detect TODO/FIXME in general files — select language-appropriate rule key
    let todo_ext = file_path.rsplit('.').next().unwrap_or("");
    let todo_rule_key = match todo_ext {
        "rs"                                                 => "rust:S1135",
        "py" | "pyw"                                         => "python:S1135",
        "c" | "h"                                            => "c:S1135",
        "cpp" | "cc" | "cxx" | "hpp"                         => "cpp:S1135",
        "java"                                               => "java:S1135",
        "cs"                                                 => "csharp:S1135",
        "php"                                                => "php:S1135",
        "go"                                                 => "go:S1135",
        "swift"                                              => "swift:S1135",
        "kt" | "kts"                                         => "kotlin:S1135",
        "rb"                                                 => "ruby:S1135",
        "pl" | "pm"                                          => "perl:S1135",
        "lua"                                                => "lua:S1135",
        "scala" | "sc"                                       => "scala:S1135",
        "dart"                                               => "dart:S1135",
        "hs" | "lhs"                                         => "haskell:S1135",
        "cob" | "cbl"                                        => "cobol:S1135",
        "f" | "f90" | "f95" | "f03" | "f08" | "for" | "ftn" => "fortran:S1135",
        "r" | "R"                                            => "r:S1135",
        "ps1" | "psm1"                                       => "powershell:S1135",
        "vb" | "vbs" | "bas" | "cls" | "frm"                => "vb:S1135",
        "m" | "mm"                                           => "objc:S1135",
        "jl"                                                 => "julia:S1135",
        "ml" | "mli" | "sml" | "sig" | "fun"                => "ocaml:S1135",
        "erl" | "hrl"                                        => "erlang:S1135",
        "mat" | "mlx" | "mlapp"                              => "matlab:S1135",
        "abap" | "prog" | "clas" | "fugr"                    => "abap:S1135",
        "pas" | "pp" | "dpr" | "dfm"                         => "delphi:S1135",
        "zig" | "zon"                                        => "zig:S1135",
        "pls" | "plsql" | "pkb" | "pks" | "pck"             => "sql:S1135",
        "sas"                                                => "sas:S1135",
        "asm" | "s" | "nasm"                                 => "assembly:S1135",
        "adb" | "ads" | "ada"                                => "ada:S1135",
        "pro" | "prolog"                                     => "prolog:S1135",
        "lisp" | "lsp" | "cl" | "el"                         => "lisp:S1135",
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs"         => "typescript:S1135",
        _                                                    => "general:S1135",
    };

    // Pattern scan — line-by-line
    for (line_idx, &line_content) in lines.iter().enumerate() {
        let ln = line_idx as u32 + 1;
        for pat in &pats {
            // Skip TODO/FIXME patterns here — handled separately below to avoid duplicates
            if pat.rule_key.ends_with(":S1135") { continue; }

            // Determine file language from extension
            let ext = file_path.rsplit('.').next().unwrap_or("");
            let is_rust       = ext == "rs";
            let is_ts         = matches!(ext, "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs");
            let is_python     = matches!(ext, "py" | "pyw" | "pyi");
            let is_c          = matches!(ext, "c" | "h");
            let is_cpp        = matches!(ext, "cpp" | "cc" | "cxx" | "hpp" | "hxx");
            let is_java       = ext == "java";
            let is_csharp     = ext == "cs";
            let is_php        = matches!(ext, "php" | "php3" | "php4" | "php5" | "phtml");
            let is_go         = ext == "go";
            let is_swift      = ext == "swift";
            let is_kotlin     = matches!(ext, "kt" | "kts");
            let is_ruby       = matches!(ext, "rb" | "erb" | "rake");
            let is_sql        = matches!(ext, "sql" | "ddl" | "dml" | "tsql" | "mysql" | "pgsql");
            let is_solidity   = ext == "sol";
            let is_powershell = matches!(ext, "ps1" | "psm1" | "psd1");
            let is_perl       = matches!(ext, "pl" | "pm");
            let is_lua        = ext == "lua";
            let is_scala      = matches!(ext, "scala" | "sc");
            let is_dart       = ext == "dart";
            let is_haskell    = matches!(ext, "hs" | "lhs");
            let is_cobol      = matches!(ext, "cob" | "cbl" | "cpy");
            let is_fortran    = matches!(ext, "f" | "f90" | "f95" | "f03" | "f08" | "for" | "ftn");
            let is_r          = matches!(ext, "r" | "R");
            // New TIOBE languages
            let is_vb         = matches!(ext, "vb" | "vbs" | "bas" | "cls" | "frm");
            let is_objc       = matches!(ext, "m" | "mm");
            let is_julia      = ext == "jl";
            let is_ocaml      = matches!(ext, "ml" | "mli" | "sml" | "sig" | "fun");
            let is_erlang     = matches!(ext, "erl" | "hrl");
            let is_matlab     = matches!(ext, "m" | "mat" | "mlx" | "mlapp");
            let is_abap       = matches!(ext, "abap" | "prog" | "clas" | "fugr");
            let is_delphi     = matches!(ext, "pas" | "pp" | "dpr" | "dfm");
            let is_zig        = ext == "zig";
            let is_plsql      = matches!(ext, "pls" | "plsql" | "pkb" | "pks" | "pck");
            let is_sas        = ext == "sas";

            let rule_lang = pat.rule_key.split(':').next().unwrap_or("");
            let allowed = match rule_lang {
                "typescript" | "javascript" => is_ts,
                "rust"        => is_rust,
                "python"      => is_python,
                "c"           => is_c || is_cpp,
                "cpp"         => is_cpp,
                "java"        => is_java,
                "csharp"      => is_csharp,
                "php"         => is_php,
                "go"          => is_go,
                "swift"       => is_swift,
                "kotlin"      => is_kotlin,
                "ruby"        => is_ruby,
                "sql"         => is_sql || is_plsql,
                "solidity"    => is_solidity,
                "powershell"  => is_powershell,
                "perl"        => is_perl,
                "lua"         => is_lua,
                "scala"       => is_scala,
                "dart"        => is_dart,
                "haskell"     => is_haskell,
                "cobol"       => is_cobol,
                "fortran"     => is_fortran,
                "r"           => is_r,
                "vb"          => is_vb,
                "objc"        => is_objc,
                "julia"       => is_julia,
                "ocaml"       => is_ocaml,
                "erlang"      => is_erlang,
                "matlab"      => is_matlab,
                "abap"        => is_abap,
                "delphi"      => is_delphi,
                "zig"         => is_zig,
                "plsql"       => is_plsql,
                "sas"         => is_sas,
                "assembly"    => matches!(ext, "asm" | "s" | "nasm" | "S"),
                "ada"         => matches!(ext, "adb" | "ads" | "ada"),
                "prolog"      => matches!(ext, "pro" | "prolog"),
                "lisp"        => matches!(ext, "lisp" | "lsp" | "cl" | "el"),
                "general"     => true,
                _             => true,
            };
            if !allowed { continue; }

            if let Some((col, _fragment, message)) = (pat.matcher)(line_content) {
                if let Some(rule) = rules_map.get(pat.rule_key) {
                    let context_before = if line_idx > 0 { lines[line_idx - 1].to_string() } else { String::new() };
                    let context_after = lines.get(line_idx + 1).copied().unwrap_or("").to_string();
                    issues.push(SonarIssue {
                        rule_key: rule.key.clone(),
                        rule_name: rule.name.clone(),
                        file: file_path.to_string(),
                        line: ln,
                        end_line: ln,
                        col_start: col,
                        message,
                        severity: rule.severity.clone(),
                        issue_type: rule.issue_type.clone(),
                        code_snippet: line_content.to_string(),
                        context_before,
                        context_after,
                        why: rule.why.clone(),
                        how_to_fix: rule.how_to_fix.clone(),
                        effort: format!("{}min", rule.effort_minutes),
                    });
                }
            }
        }

        // TODO/FIXME scanner (language-agnostic)
        let trimmed = line_content.trim();
        for marker in &["TODO", "FIXME", "HACK", "XXX"] {
            if trimmed.contains(marker) {
                let col = line_content.find(marker).unwrap_or(0) as u32;
                if let Some(rule) = rules_map.get(todo_rule_key).or_else(|| rules_map.get("general:S1135")) {
                    let context_before = if line_idx > 0 { lines[line_idx - 1].to_string() } else { String::new() };
                    let context_after = lines.get(line_idx + 1).copied().unwrap_or("").to_string();
                    issues.push(SonarIssue {
                        rule_key: rule.key.clone(),
                        rule_name: rule.name.clone(),
                        file: file_path.to_string(),
                        line: ln,
                        end_line: ln,
                        col_start: col,
                        message: format!("`{marker}` comment — track this in your issue tracker"),
                        severity: rule.severity.clone(),
                        issue_type: rule.issue_type.clone(),
                        code_snippet: line_content.to_string(),
                        context_before,
                        context_after,
                        why: rule.why.clone(),
                        how_to_fix: rule.how_to_fix.clone(),
                        effort: format!("{}min", rule.effort_minutes),
                    });
                    break; // one issue per line for TODO
                }
            }
        }
    }

    // Also detect too-many-params (S107): look for function signatures with >7 params
    for (line_idx, &line_content) in lines.iter().enumerate() {
        let ln = line_idx as u32 + 1;
        let trimmed = line_content.trim();
        if (trimmed.contains("function ") || trimmed.contains("fn ") || trimmed.contains("=> {")) && trimmed.contains('(') {
            if let Some(start) = line_content.find('(') {
                if let Some(end) = line_content[start..].find(')') {
                    let params_str = &line_content[start + 1..start + end];
                    let param_count = if params_str.trim().is_empty() { 0 } else { params_str.split(',').count() };
                    if param_count > 7 {
                        let rule_key = if file_path.ends_with(".rs") { "rust:S107" } else { "typescript:S107" };
                        // Use general rule if specific not found
                        let rule = rules_map.get(rule_key).or_else(|| rules_map.get("typescript:S107"));
                        if let Some(rule) = rule {
                            let context_before = if line_idx > 0 { lines[line_idx - 1].to_string() } else { String::new() };
                            let context_after = lines.get(line_idx + 1).copied().unwrap_or("").to_string();
                            issues.push(SonarIssue {
                                rule_key: rule.key.clone(),
                                rule_name: rule.name.clone(),
                                file: file_path.to_string(),
                                line: ln,
                                end_line: ln,
                                col_start: start as u32,
                                message: format!("Function has {param_count} parameters (threshold: 7) — group them into an options object"),
                                severity: rule.severity.clone(),
                                issue_type: rule.issue_type.clone(),
                                code_snippet: line_content.to_string(),
                                context_before,
                                context_after,
                                why: rule.why.clone(),
                                how_to_fix: rule.how_to_fix.clone(),
                                effort: format!("{}min", rule.effort_minutes),
                            });
                        }
                    }
                }
            }
        }
    }

    let bugs = issues.iter().filter(|i| i.issue_type == "BUG").count() as u32;
    let vulns = issues.iter().filter(|i| i.issue_type == "VULNERABILITY").count() as u32;
    let smells = issues.iter().filter(|i| i.issue_type == "CODE_SMELL").count() as u32;
    let hotspots = issues.iter().filter(|i| i.issue_type == "SECURITY_HOTSPOT").count() as u32;
    let debt = issues.iter().map(|i| {
        i.effort.trim_end_matches("min").parse::<u32>().unwrap_or(0)
    }).sum();

    SonarScanResult {
        file: file_path.to_string(),
        issues,
        bugs,
        vulnerabilities: vulns,
        code_smells: smells,
        security_hotspots: hotspots,
        debt_minutes: debt,
    }
}
