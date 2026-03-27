//! Warp terminal-style features: natural language commands, command correction,
//! secret redaction, next-command suggestions, output filtering, error explanation,
//! output blocks with ANSI formatting, and desktop notifications.

use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// 1. NaturalLanguageCommand
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NaturalLanguageCommand {
    pub original: String,
    pub generated: String,
    pub explanation: String,
    pub confidence: f64,
}

pub fn generate_command_prompt(nl: &str, cwd: &str, shell: &str) -> String {
    format!(
        "Translate the following natural language request into a {} shell command.\n\
         Working directory: {}\n\
         Request: {}\n\n\
         Respond in this exact format:\n\
         COMMAND: <the shell command>\n\
         EXPLANATION: <brief explanation>\n\
         CONFIDENCE: <0.0-1.0>",
        shell, cwd, nl
    )
}

pub fn parse_command_response(response: &str) -> Option<NaturalLanguageCommand> {
    let mut command: Option<&str> = None;
    let mut explanation: Option<&str> = None;
    let mut confidence: f64 = 0.0;

    for line in response.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("COMMAND:") {
            command = Some(rest.trim());
        } else if let Some(rest) = trimmed.strip_prefix("EXPLANATION:") {
            explanation = Some(rest.trim());
        } else if let Some(rest) = trimmed.strip_prefix("CONFIDENCE:") {
            if let Ok(c) = rest.trim().parse::<f64>() {
                confidence = c;
            }
        }
    }

    let cmd = command?;
    if cmd.is_empty() {
        return None;
    }

    Some(NaturalLanguageCommand {
        original: String::new(),
        generated: cmd.to_string(),
        explanation: explanation.unwrap_or("").to_string(),
        confidence,
    })
}

// ---------------------------------------------------------------------------
// 2. CommandCorrection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CommandCorrection {
    pub failed_command: String,
    pub suggested_command: String,
    pub reason: String,
}

/// Suggest a correction for a failed command based on common patterns.
pub fn suggest_correction(cmd: &str, exit_code: i32, stderr: &str) -> Option<CommandCorrection> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let first = parts[0];
    let stderr_lower = stderr.to_lowercase();

    // Typo corrections
    let typo_map: &[(&str, &str)] = &[
        ("gti", "git"),
        ("dicker", "docker"),
        ("carg", "cargo"),
        ("pytohn", "python"),
        ("nmp", "npm"),
        ("dokcer", "docker"),
        ("ndoe", "node"),
        ("pyhton", "python"),
    ];
    for &(typo, correct) in typo_map {
        if first == typo {
            let fixed = std::iter::once(correct)
                .chain(parts[1..].iter().copied())
                .collect::<Vec<&str>>()
                .join(" ");
            return Some(CommandCorrection {
                failed_command: cmd.to_string(),
                suggested_command: fixed,
                reason: format!("Did you mean '{}'?", correct),
            });
        }
    }

    // git push with no upstream
    if cmd.starts_with("git push") && stderr_lower.contains("no upstream") {
        let branch = parts.iter().last().unwrap_or(&"main");
        return Some(CommandCorrection {
            failed_command: cmd.to_string(),
            suggested_command: format!("git push --set-upstream origin {}", branch),
            reason: "No upstream branch set. Adding --set-upstream.".to_string(),
        });
    }

    // Permission denied
    if stderr_lower.contains("permission denied") {
        if exit_code == 126 || stderr_lower.contains("execute") {
            return Some(CommandCorrection {
                failed_command: cmd.to_string(),
                suggested_command: format!("chmod +x {} && {}", parts.first().unwrap_or(&""), cmd),
                reason: "File is not executable. Adding execute permission.".to_string(),
            });
        }
        return Some(CommandCorrection {
            failed_command: cmd.to_string(),
            suggested_command: format!("sudo {}", cmd),
            reason: "Permission denied. Try with sudo.".to_string(),
        });
    }

    // command not found: python -> python3
    if stderr_lower.contains("command not found") || stderr_lower.contains("not found") {
        if first == "python" {
            return Some(CommandCorrection {
                failed_command: cmd.to_string(),
                suggested_command: cmd.replacen("python", "python3", 1),
                reason: "'python' not found. Try 'python3' instead.".to_string(),
            });
        }
        if first == "pip" {
            return Some(CommandCorrection {
                failed_command: cmd.to_string(),
                suggested_command: cmd.replacen("pip", "pip3", 1),
                reason: "'pip' not found. Try 'pip3' instead.".to_string(),
            });
        }
        if first == "cargo" {
            return Some(CommandCorrection {
                failed_command: cmd.to_string(),
                suggested_command: "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh".to_string(),
                reason: "cargo not found. Install Rust via rustup.".to_string(),
            });
        }
        if first == "npm" || first == "npx" {
            return Some(CommandCorrection {
                failed_command: cmd.to_string(),
                suggested_command: "curl -fsSL https://fnm.vercel.app/install | bash".to_string(),
                reason: format!("{} not found. Install Node.js first.", first),
            });
        }
        if first == "pip" || first == "pip3" {
            return Some(CommandCorrection {
                failed_command: cmd.to_string(),
                suggested_command: "python3 -m ensurepip --upgrade".to_string(),
                reason: "pip not found. Bootstrap it with ensurepip.".to_string(),
            });
        }
    }

    // cd to non-existent directory
    if first == "cd" && (stderr_lower.contains("no such file") || stderr_lower.contains("not a directory")) {
        if let Some(target) = parts.get(1) {
            // Suggest removing trailing segments
            if let Some(pos) = target.rfind('/') {
                let parent = &target[..pos];
                if !parent.is_empty() {
                    return Some(CommandCorrection {
                        failed_command: cmd.to_string(),
                        suggested_command: format!("cd {}", parent),
                        reason: format!("Directory '{}' not found. Try parent directory.", target),
                    });
                }
            }
            return Some(CommandCorrection {
                failed_command: cmd.to_string(),
                suggested_command: format!("mkdir -p {} && cd {}", target, target),
                reason: format!("Directory '{}' does not exist. Create it first.", target),
            });
        }
    }

    // sudo needed (EACCES)
    if exit_code != 0 && (stderr_lower.contains("eacces") || stderr_lower.contains("access denied")) {
        return Some(CommandCorrection {
            failed_command: cmd.to_string(),
            suggested_command: format!("sudo {}", cmd),
            reason: "Access denied. Try running with sudo.".to_string(),
        });
    }

    None
}

// ---------------------------------------------------------------------------
// 3. SecretRedactor
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct RedactPattern {
    prefix: &'static str,
    suffix_char_fn: fn(char) -> bool,
    min_suffix_len: usize,
    replacement: &'static str,
}

#[derive(Debug)]
pub struct SecretRedactor {
    /// We store simple prefix-based patterns for manual matching.
    patterns: Vec<(&'static str, &'static str)>,
}

impl Default for SecretRedactor {
    fn default() -> Self { Self::new() }
}

impl SecretRedactor {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                ("sk-", "sk-****"),
                ("AKIA", "AKIA****"),
                ("ghp_", "ghp_****"),
                ("gho_", "gho_****"),
                ("ghs_", "ghs_****"),
                ("ghr_", "ghr_****"),
            ],
        }
    }

    pub fn redact(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Redact Bearer tokens: "Bearer <token>" → "Bearer ****"
        result = Self::redact_bearer(&result);

        // Redact password=xxx
        result = Self::redact_password(&result);

        // Redact private keys
        result = Self::redact_private_keys(&result);

        // Redact prefix-based tokens
        for &(prefix, replacement) in &self.patterns {
            result = Self::redact_prefix_token(&result, prefix, replacement);
        }

        result
    }

    fn redact_prefix_token(text: &str, prefix: &str, replacement: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut remaining = text;

        while let Some(pos) = remaining.find(prefix) {
            result.push_str(&remaining[..pos]);
            let after_prefix = &remaining[pos + prefix.len()..];
            // Count how many alphanumeric chars follow
            let token_len: usize = after_prefix
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .count();
            if token_len >= 4 {
                result.push_str(replacement);
                remaining = &after_prefix[token_len..];
            } else {
                result.push_str(prefix);
                remaining = after_prefix;
            }
        }
        result.push_str(remaining);
        result
    }

    fn redact_bearer(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut remaining = text;
        let needle = "Bearer ";

        while let Some(pos) = remaining.find(needle) {
            result.push_str(&remaining[..pos]);
            let after = &remaining[pos + needle.len()..];
            let token_len: usize = after
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
                .count();
            if token_len > 0 {
                result.push_str("Bearer ****");
                remaining = &after[token_len..];
            } else {
                result.push_str(needle);
                remaining = after;
            }
        }
        result.push_str(remaining);
        result
    }

    fn redact_password(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut remaining = text;
        let needle = "password=";

        while let Some(pos) = remaining.find(needle) {
            result.push_str(&remaining[..pos]);
            let after = &remaining[pos + needle.len()..];
            let val_len: usize = after
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '&' && *c != ';')
                .count();
            result.push_str("password=****");
            remaining = &after[val_len..];
        }
        result.push_str(remaining);
        result
    }

    fn redact_private_keys(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut remaining = text;
        let begin = "-----BEGIN";

        while let Some(pos) = remaining.find(begin) {
            result.push_str(&remaining[..pos]);
            // Check if this line contains PRIVATE KEY
            let line_end = remaining[pos..].find('\n').unwrap_or(remaining.len() - pos);
            let line = &remaining[pos..pos + line_end];
            if line.contains("PRIVATE KEY") {
                result.push_str("[PRIVATE KEY REDACTED]");
                // Skip until -----END ... PRIVATE KEY-----
                let end_marker = "-----END";
                if let Some(end_pos) = remaining[pos..].find(end_marker) {
                    let after_end = &remaining[pos + end_pos..];
                    let end_line_len = after_end.find('\n').unwrap_or(after_end.len());
                    remaining = &remaining[pos + end_pos + end_line_len..];
                } else {
                    remaining = &remaining[pos + line_end..];
                }
            } else {
                result.push_str(line);
                remaining = &remaining[pos + line_end..];
            }
        }
        result.push_str(remaining);
        result
    }
}

// ---------------------------------------------------------------------------
// 4. CommandSuggestion + suggest_next_commands
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CommandSuggestion {
    pub command: String,
    pub description: String,
    pub confidence: f64,
}

pub fn suggest_next_commands(cmd: &str, exit_code: i32, cwd: &str) -> Vec<CommandSuggestion> {
    if exit_code != 0 {
        return vec![];
    }

    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return vec![];
    }

    let mut suggestions = Vec::new();

    // git clone <url> → cd <dir>
    if parts.len() >= 3 && parts[0] == "git" && parts[1] == "clone" {
        let url = parts[2];
        let dir_name = url
            .rsplit('/')
            .next()
            .unwrap_or("repo")
            .trim_end_matches(".git");
        suggestions.push(CommandSuggestion {
            command: format!("cd {}", dir_name),
            description: "Change into the cloned directory".to_string(),
            confidence: 0.95,
        });
    }

    // git add → git commit
    if parts.len() >= 2 && parts[0] == "git" && parts[1] == "add" {
        suggestions.push(CommandSuggestion {
            command: "git commit -m \"\"".to_string(),
            description: "Commit staged changes".to_string(),
            confidence: 0.9,
        });
    }

    // git commit → git push
    if parts.len() >= 2 && parts[0] == "git" && parts[1] == "commit" {
        suggestions.push(CommandSuggestion {
            command: "git push".to_string(),
            description: "Push committed changes to remote".to_string(),
            confidence: 0.85,
        });
    }

    // cargo new <name> → cd <name>
    if parts.len() >= 3 && parts[0] == "cargo" && parts[1] == "new" {
        suggestions.push(CommandSuggestion {
            command: format!("cd {}", parts[2]),
            description: format!("Change into the new project '{}'", parts[2]),
            confidence: 0.9,
        });
    }

    // cargo build → cargo test / cargo run
    if parts.len() >= 2 && parts[0] == "cargo" && parts[1] == "build" {
        suggestions.push(CommandSuggestion {
            command: "cargo test".to_string(),
            description: "Run tests".to_string(),
            confidence: 0.8,
        });
        suggestions.push(CommandSuggestion {
            command: "cargo run".to_string(),
            description: "Run the project".to_string(),
            confidence: 0.75,
        });
    }

    // npm init → npm install
    if parts.len() >= 2 && parts[0] == "npm" && parts[1] == "init" {
        suggestions.push(CommandSuggestion {
            command: "npm install".to_string(),
            description: "Install dependencies".to_string(),
            confidence: 0.85,
        });
    }

    // mkdir <dir> → cd <dir>
    if parts.len() >= 2 && parts[0] == "mkdir" {
        let dir = parts.last().unwrap_or(&"");
        if !dir.starts_with('-') {
            suggestions.push(CommandSuggestion {
                command: format!("cd {}", dir),
                description: format!("Change into '{}'", dir),
                confidence: 0.85,
            });
        }
    }

    // docker build → docker run
    if parts.len() >= 2 && parts[0] == "docker" && parts[1] == "build" {
        // Try to find -t tag
        let tag = parts
            .windows(2)
            .find(|w| w[0] == "-t")
            .map(|w| w[1])
            .unwrap_or("image");
        suggestions.push(CommandSuggestion {
            command: format!("docker run {}", tag),
            description: "Run the built Docker image".to_string(),
            confidence: 0.8,
        });
    }

    let _ = cwd; // reserved for future cwd-aware suggestions
    suggestions
}

// ---------------------------------------------------------------------------
// 5. OutputFilter
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct OutputFilter {
    pub last_output: String,
}

impl OutputFilter {
    pub fn set_output(&mut self, s: &str) {
        self.last_output = s.to_string();
    }

    pub fn filter(&self, pattern: &str, invert: bool, case_sensitive: bool) -> String {
        self.last_output
            .lines()
            .filter(|line| {
                let matches = if case_sensitive {
                    line.contains(pattern)
                } else {
                    line.to_lowercase().contains(&pattern.to_lowercase())
                };
                if invert { !matches } else { matches }
            })
            .collect::<Vec<&str>>()
            .join("\n")
    }

    pub fn filter_count(&self, pattern: &str) -> usize {
        self.last_output
            .lines()
            .filter(|line| line.contains(pattern))
            .count()
    }
}

// ---------------------------------------------------------------------------
// 6. generate_error_explanation_prompt
// ---------------------------------------------------------------------------

pub fn generate_error_explanation_prompt(
    cmd: &str,
    exit_code: i32,
    stderr: &str,
    stdout: &str,
) -> String {
    format!(
        "A command failed. Please explain the error in plain language and suggest a fix.\n\n\
         Command: {}\n\
         Exit code: {}\n\
         Stderr:\n{}\n\
         Stdout:\n{}\n\n\
         Provide:\n\
         1. What went wrong (plain English)\n\
         2. How to fix it\n\
         3. Any relevant documentation links",
        cmd, exit_code, stderr, stdout
    )
}

// ---------------------------------------------------------------------------
// 7. OutputBlock
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct OutputBlock {
    pub command: String,
    pub output: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub cwd: String,
    pub timestamp: u64,
}

impl OutputBlock {
    pub fn new(command: &str, output: &str, exit_code: i32, duration_ms: u64, cwd: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            command: command.to_string(),
            output: output.to_string(),
            exit_code,
            duration_ms,
            cwd: cwd.to_string(),
            timestamp,
        }
    }

    pub fn format(&self) -> String {
        let color = if self.exit_code == 0 {
            "\x1b[32m" // green
        } else {
            "\x1b[31m" // red
        };
        let reset = "\x1b[0m";
        let bold = "\x1b[1m";
        let dim = "\x1b[2m";

        let bar = format!("{}│{}", color, reset);
        let duration_str = if self.duration_ms >= 1000 {
            format!("{:.1}s", self.duration_ms as f64 / 1000.0)
        } else {
            format!("{}ms", self.duration_ms)
        };

        let mut lines = Vec::new();
        lines.push(format!(
            "{} {}{}$ {}{}  {}[{}]{}",
            bar, bold, color, self.command, reset, dim, duration_str, reset
        ));
        for output_line in self.output.lines() {
            lines.push(format!("{} {}", bar, output_line));
        }
        if self.exit_code != 0 {
            lines.push(format!(
                "{} {}exit code: {}{}",
                bar, color, self.exit_code, reset
            ));
        }
        lines.join("\n")
    }
}

// ---------------------------------------------------------------------------
// 8. Notifications
// ---------------------------------------------------------------------------

pub fn send_notification(title: &str, body: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            body.replace('\"', "\\\""),
            title.replace('\"', "\\\"")
        );
        let status = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status()
            .map_err(|e| format!("Failed to run osascript: {}", e))?;
        if status.success() {
            return Ok(());
        }
        Err("osascript returned non-zero".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        let status = std::process::Command::new("notify-send")
            .arg(title)
            .arg(body)
            .status()
            .map_err(|e| format!("Failed to run notify-send: {}", e))?;
        if status.success() {
            return Ok(());
        }
        return Err("notify-send returned non-zero".to_string());
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        // Bell fallback
        print!("\x07");
        let _ = (title, body);
        Ok(())
    }
}

pub fn should_notify(duration_ms: u64, threshold_ms: u64) -> bool {
    duration_ms >= threshold_ms
}

/// Current epoch time in seconds.
pub fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- NaturalLanguageCommand tests --

    #[test]
    fn test_generate_command_prompt_contains_fields() {
        let prompt = generate_command_prompt("list files", "/home/user", "bash");
        assert!(prompt.contains("bash"));
        assert!(prompt.contains("/home/user"));
        assert!(prompt.contains("list files"));
    }

    #[test]
    fn test_parse_command_response_valid() {
        let resp = "COMMAND: ls -la\nEXPLANATION: Lists all files\nCONFIDENCE: 0.95";
        let cmd = parse_command_response(resp).unwrap();
        assert_eq!(cmd.generated, "ls -la");
        assert_eq!(cmd.explanation, "Lists all files");
        assert!((cmd.confidence - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_parse_command_response_missing_command() {
        let resp = "EXPLANATION: something\nCONFIDENCE: 0.5";
        assert!(parse_command_response(resp).is_none());
    }

    #[test]
    fn test_parse_command_response_empty_command() {
        let resp = "COMMAND: \nEXPLANATION: x";
        assert!(parse_command_response(resp).is_none());
    }

    #[test]
    fn test_parse_command_response_no_confidence() {
        let resp = "COMMAND: echo hello\nEXPLANATION: prints hello";
        let cmd = parse_command_response(resp).unwrap();
        assert_eq!(cmd.generated, "echo hello");
        assert!((cmd.confidence - 0.0).abs() < 0.001);
    }

    // -- CommandCorrection tests --

    #[test]
    fn test_typo_gti() {
        let c = suggest_correction("gti status", 127, "command not found").unwrap();
        assert_eq!(c.suggested_command, "git status");
    }

    #[test]
    fn test_typo_dicker() {
        let c = suggest_correction("dicker ps", 127, "command not found").unwrap();
        assert_eq!(c.suggested_command, "docker ps");
    }

    #[test]
    fn test_typo_carg() {
        let c = suggest_correction("carg build", 127, "command not found").unwrap();
        assert_eq!(c.suggested_command, "cargo build");
    }

    #[test]
    fn test_typo_pytohn() {
        let c = suggest_correction("pytohn script.py", 127, "command not found").unwrap();
        assert_eq!(c.suggested_command, "python script.py");
    }

    #[test]
    fn test_typo_nmp() {
        let c = suggest_correction("nmp install", 127, "command not found").unwrap();
        assert_eq!(c.suggested_command, "npm install");
    }

    #[test]
    fn test_git_push_no_upstream() {
        let c = suggest_correction("git push", 1, "fatal: no upstream branch").unwrap();
        assert!(c.suggested_command.contains("--set-upstream"));
    }

    #[test]
    fn test_permission_denied_execute() {
        let c = suggest_correction("./script.sh", 126, "Permission denied: execute").unwrap();
        assert!(c.suggested_command.contains("chmod +x"));
    }

    #[test]
    fn test_permission_denied_sudo() {
        let c = suggest_correction("apt install vim", 1, "Permission denied").unwrap();
        assert!(c.suggested_command.starts_with("sudo "));
    }

    #[test]
    fn test_python_not_found() {
        let c = suggest_correction("python main.py", 127, "command not found").unwrap();
        assert!(c.suggested_command.contains("python3"));
    }

    #[test]
    fn test_cd_no_such_dir() {
        let c = suggest_correction("cd /foo/bar/baz", 1, "No such file or directory").unwrap();
        assert_eq!(c.suggested_command, "cd /foo/bar");
    }

    #[test]
    fn test_cd_no_parent() {
        let c = suggest_correction("cd mydir", 1, "No such file or directory").unwrap();
        assert!(c.suggested_command.contains("mkdir -p"));
    }

    #[test]
    fn test_cargo_not_found() {
        let c = suggest_correction("cargo build", 127, "cargo: command not found").unwrap();
        assert!(c.suggested_command.contains("rustup"));
    }

    #[test]
    fn test_npm_not_found() {
        let c = suggest_correction("npm install", 127, "npm: command not found").unwrap();
        assert!(c.suggested_command.contains("fnm") || c.suggested_command.contains("node"));
    }

    #[test]
    fn test_eacces() {
        let c = suggest_correction("rm /etc/hosts", 1, "EACCES: permission denied").unwrap();
        assert!(c.suggested_command.starts_with("sudo "));
    }

    #[test]
    fn test_no_correction_needed() {
        assert!(suggest_correction("ls", 0, "").is_none());
    }

    // -- SecretRedactor tests --

    #[test]
    fn test_redact_openai_key() {
        let r = SecretRedactor::new();
        let input = "key is sk-abcdefghijklmnopqrstuvwx done";
        let output = r.redact(input);
        assert!(output.contains("sk-****"));
        assert!(!output.contains("abcdefgh"));
    }

    #[test]
    fn test_redact_aws_key() {
        let r = SecretRedactor::new();
        let input = "aws AKIAIOSFODNN7EXAMPLE here";
        let output = r.redact(input);
        assert!(output.contains("AKIA****"));
        assert!(!output.contains("IOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_redact_github_tokens() {
        let r = SecretRedactor::new();
        for prefix in &["ghp_", "gho_", "ghs_", "ghr_"] {
            let token = format!("{}abcdefghijklmnopqrst end", prefix);
            let output = r.redact(&token);
            assert!(output.contains(&format!("{}****", &prefix[..4])), "Failed for {}", prefix);
        }
    }

    #[test]
    fn test_redact_bearer() {
        let r = SecretRedactor::new();
        let input = "Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9 done";
        let output = r.redact(input);
        assert!(output.contains("Bearer ****"));
        assert!(!output.contains("eyJhbGci"));
    }

    #[test]
    fn test_redact_password() {
        let r = SecretRedactor::new();
        let input = "url=http://host?password=s3cret&other=val";
        let output = r.redact(input);
        assert!(output.contains("password=****"));
        assert!(!output.contains("s3cret"));
    }

    #[test]
    fn test_redact_private_key() {
        let r = SecretRedactor::new();
        let input = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAK...\n-----END RSA PRIVATE KEY-----\nafter";
        let output = r.redact(input);
        assert!(output.contains("[PRIVATE KEY REDACTED]"));
        assert!(!output.contains("MIIEowIBAAK"));
    }

    #[test]
    fn test_redact_no_secrets() {
        let r = SecretRedactor::new();
        let input = "Hello world, nothing secret here";
        assert_eq!(r.redact(input), input);
    }

    #[test]
    fn test_redact_short_prefix_not_matched() {
        let r = SecretRedactor::new();
        // Too short to be a real token
        let input = "sk-ab end";
        let output = r.redact(input);
        assert_eq!(output, input);
    }

    // -- suggest_next_commands tests --

    #[test]
    fn test_suggest_after_git_clone() {
        let suggestions = suggest_next_commands("git clone https://github.com/user/repo.git", 0, "/home");
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].command, "cd repo");
    }

    #[test]
    fn test_suggest_after_git_add() {
        let suggestions = suggest_next_commands("git add .", 0, "/home");
        assert!(suggestions.iter().any(|s| s.command.contains("git commit")));
    }

    #[test]
    fn test_suggest_after_git_commit() {
        let suggestions = suggest_next_commands("git commit -m \"fix\"", 0, "/home");
        assert!(suggestions.iter().any(|s| s.command == "git push"));
    }

    #[test]
    fn test_suggest_after_cargo_new() {
        let suggestions = suggest_next_commands("cargo new myapp", 0, "/home");
        assert!(suggestions.iter().any(|s| s.command == "cd myapp"));
    }

    #[test]
    fn test_suggest_after_cargo_build() {
        let suggestions = suggest_next_commands("cargo build", 0, "/home");
        assert!(suggestions.iter().any(|s| s.command == "cargo test"));
        assert!(suggestions.iter().any(|s| s.command == "cargo run"));
    }

    #[test]
    fn test_suggest_after_npm_init() {
        let suggestions = suggest_next_commands("npm init", 0, "/home");
        assert!(suggestions.iter().any(|s| s.command == "npm install"));
    }

    #[test]
    fn test_suggest_after_mkdir() {
        let suggestions = suggest_next_commands("mkdir -p src/lib", 0, "/home");
        assert!(suggestions.iter().any(|s| s.command == "cd src/lib"));
    }

    #[test]
    fn test_suggest_after_docker_build() {
        let suggestions = suggest_next_commands("docker build -t myimg .", 0, "/home");
        assert!(suggestions.iter().any(|s| s.command.contains("docker run")));
        assert!(suggestions.iter().any(|s| s.command.contains("myimg")));
    }

    #[test]
    fn test_suggest_nothing_on_failure() {
        let suggestions = suggest_next_commands("cargo build", 1, "/home");
        assert!(suggestions.is_empty());
    }

    // -- OutputFilter tests --

    #[test]
    fn test_filter_basic() {
        let mut f = OutputFilter::default();
        f.set_output("line one\nerror: bad\nline three\nerror: worse");
        let result = f.filter("error", false, true);
        assert_eq!(result, "error: bad\nerror: worse");
    }

    #[test]
    fn test_filter_invert() {
        let mut f = OutputFilter::default();
        f.set_output("keep\nremove error\nkeep");
        let result = f.filter("error", true, true);
        assert_eq!(result, "keep\nkeep");
    }

    #[test]
    fn test_filter_case_insensitive() {
        let mut f = OutputFilter::default();
        f.set_output("ERROR here\nerror there\nnothing");
        let result = f.filter("error", false, false);
        assert_eq!(result, "ERROR here\nerror there");
    }

    #[test]
    fn test_filter_count() {
        let mut f = OutputFilter::default();
        f.set_output("a\nb\na\nc\na");
        assert_eq!(f.filter_count("a"), 3);
    }

    #[test]
    fn test_filter_empty_output() {
        let f = OutputFilter::default();
        assert_eq!(f.filter("x", false, true), "");
        assert_eq!(f.filter_count("x"), 0);
    }

    // -- generate_error_explanation_prompt tests --

    #[test]
    fn test_error_prompt_format() {
        let prompt = generate_error_explanation_prompt("cargo build", 101, "error[E0308]", "");
        assert!(prompt.contains("cargo build"));
        assert!(prompt.contains("101"));
        assert!(prompt.contains("E0308"));
        assert!(prompt.contains("How to fix"));
    }

    // -- OutputBlock tests --

    #[test]
    fn test_output_block_format_success() {
        let block = OutputBlock {
            command: "ls".to_string(),
            output: "file1\nfile2".to_string(),
            exit_code: 0,
            duration_ms: 50,
            cwd: "/home".to_string(),
            timestamp: 0,
        };
        let formatted = block.format();
        // Green color code
        assert!(formatted.contains("\x1b[32m"));
        assert!(formatted.contains("ls"));
        assert!(formatted.contains("file1"));
        assert!(formatted.contains("50ms"));
    }

    #[test]
    fn test_output_block_format_failure() {
        let block = OutputBlock {
            command: "false".to_string(),
            output: "".to_string(),
            exit_code: 1,
            duration_ms: 2500,
            cwd: "/tmp".to_string(),
            timestamp: 0,
        };
        let formatted = block.format();
        // Red color code
        assert!(formatted.contains("\x1b[31m"));
        assert!(formatted.contains("exit code: 1"));
        assert!(formatted.contains("2.5s"));
    }

    #[test]
    fn test_output_block_new_timestamp() {
        let block = OutputBlock::new("pwd", "/home", 0, 10, "/home");
        assert!(block.timestamp > 0);
    }

    // -- Notification tests --

    #[test]
    fn test_should_notify_above_threshold() {
        assert!(should_notify(5000, 3000));
    }

    #[test]
    fn test_should_notify_below_threshold() {
        assert!(!should_notify(1000, 3000));
    }

    #[test]
    fn test_should_notify_exact_threshold() {
        assert!(should_notify(3000, 3000));
    }

    // -- Additional edge case tests --

    #[test]
    fn test_suggest_git_clone_no_git_suffix() {
        let suggestions = suggest_next_commands("git clone https://github.com/user/myrepo", 0, "/");
        assert_eq!(suggestions[0].command, "cd myrepo");
    }

    #[test]
    fn test_redact_multiple_secrets() {
        let r = SecretRedactor::new();
        let input = "key1=sk-aaaabbbbccccddddeeee key2=ghp_xxxxyyyyzzzzwwwwqqqq";
        let output = r.redact(input);
        assert!(output.contains("sk-****"));
        assert!(output.contains("ghp_****"));
    }

    #[test]
    fn test_parse_response_with_extra_whitespace() {
        let resp = "  COMMAND:   git log --oneline  \n  EXPLANATION:  shows log  \n  CONFIDENCE:  0.8  ";
        let cmd = parse_command_response(resp).unwrap();
        assert_eq!(cmd.generated, "git log --oneline");
        assert!((cmd.confidence - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_generate_prompt_zsh() {
        let prompt = generate_command_prompt("find large files", "/var", "zsh");
        assert!(prompt.contains("zsh"));
    }

    #[test]
    fn test_correction_pip_not_found() {
        let c = suggest_correction("pip install requests", 127, "pip: command not found").unwrap();
        assert!(c.suggested_command.contains("pip3"));
    }

    #[test]
    fn test_output_block_empty_output() {
        let block = OutputBlock::new("true", "", 0, 1, "/");
        let formatted = block.format();
        assert!(formatted.contains("true"));
    }
}
