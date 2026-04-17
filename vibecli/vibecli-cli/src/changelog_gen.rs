//! Automated changelog generator — parses conventional commits and produces
//! structured changelogs. Matches Copilot Workspace v2's changelog generation.
//!
//! Follows Conventional Commits 1.0 spec:
//! `<type>[(scope)][!]: <description>`
//! Supported types: feat, fix, docs, style, refactor, perf, test, chore, ci, build

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A parsed conventional commit.
#[derive(Debug, Clone)]
pub struct ConventionalCommit {
    pub hash: String,
    pub commit_type: CommitType,
    pub scope: Option<String>,
    pub breaking: bool,
    pub description: String,
    pub body: Option<String>,
    pub footer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CommitType {
    Feat,
    Fix,
    Docs,
    Style,
    Refactor,
    Perf,
    Test,
    Chore,
    Ci,
    Build,
    Unknown(String),
}

impl CommitType {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "feat" | "feature" => CommitType::Feat,
            "fix" | "bugfix" => CommitType::Fix,
            "docs" | "doc" => CommitType::Docs,
            "style" => CommitType::Style,
            "refactor" | "refact" => CommitType::Refactor,
            "perf" | "performance" => CommitType::Perf,
            "test" | "tests" => CommitType::Test,
            "chore" => CommitType::Chore,
            "ci" => CommitType::Ci,
            "build" => CommitType::Build,
            other => CommitType::Unknown(other.to_string()),
        }
    }

    pub fn section_title(&self) -> &str {
        match self {
            CommitType::Feat => "Features",
            CommitType::Fix => "Bug Fixes",
            CommitType::Docs => "Documentation",
            CommitType::Style => "Code Style",
            CommitType::Refactor => "Refactoring",
            CommitType::Perf => "Performance",
            CommitType::Test => "Tests",
            CommitType::Chore => "Chores",
            CommitType::Ci => "CI",
            CommitType::Build => "Build",
            CommitType::Unknown(_) => "Other",
        }
    }

    /// Whether this type should appear in the user-facing changelog.
    pub fn is_notable(&self) -> bool {
        matches!(self, CommitType::Feat | CommitType::Fix | CommitType::Perf | CommitType::Refactor)
    }
}

impl std::fmt::Display for CommitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitType::Unknown(s) => write!(f, "{}", s),
            _ => write!(f, "{}", self.section_title()),
        }
    }
}

/// A changelog release section.
#[derive(Debug)]
pub struct Release {
    pub version: String,
    pub date: String,
    pub breaking_changes: Vec<ConventionalCommit>,
    pub sections: HashMap<CommitType, Vec<ConventionalCommit>>,
}

impl Release {
    pub fn new(version: impl Into<String>, date: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            date: date.into(),
            breaking_changes: Vec::new(),
            sections: HashMap::new(),
        }
    }

    pub fn add_commit(&mut self, commit: ConventionalCommit) {
        if commit.breaking {
            self.breaking_changes.push(commit.clone());
        }
        self.sections.entry(commit.commit_type.clone()).or_default().push(commit);
    }

    pub fn total_commits(&self) -> usize {
        self.sections.values().map(|v| v.len()).sum()
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

pub struct CommitParser;

impl CommitParser {
    /// Parse a commit message string into a `ConventionalCommit`.
    pub fn parse(hash: &str, message: &str) -> ConventionalCommit {
        let (subject, rest) = message.split_once('\n').unwrap_or((message, ""));
        let subject = subject.trim();

        // Match `type[(scope)][!]: description`
        let (commit_type, scope, breaking, description) = Self::parse_subject(subject);

        let (body, footer) = if rest.trim().is_empty() {
            (None, None)
        } else {
            let parts: Vec<&str> = rest.trim().splitn(2, "\n\n").collect();
            let body = parts.first().filter(|s| !s.is_empty()).map(|s| s.to_string());
            let footer = parts.get(1).filter(|s| !s.is_empty()).map(|s| s.to_string());
            (body, footer)
        };

        // Also detect breaking change from footer
        let is_breaking = breaking || rest.contains("BREAKING CHANGE:");

        ConventionalCommit {
            hash: hash.to_string(),
            commit_type,
            scope,
            breaking: is_breaking,
            description,
            body,
            footer,
        }
    }

    fn parse_subject(subject: &str) -> (CommitType, Option<String>, bool, String) {
        // Try to match `type[(scope)][!]: description`
        let colon_pos = match subject.find(':') {
            Some(p) => p,
            None => return (CommitType::Unknown(String::new()), None, false, subject.to_string()),
        };

        let prefix = &subject[..colon_pos];
        let description = subject[colon_pos + 1..].trim().to_string();

        // Check for breaking `!`
        let breaking = prefix.ends_with('!');
        let prefix = prefix.trim_end_matches('!');

        // Check for scope in parens
        let (type_str, scope) = if let Some(open) = prefix.find('(') {
            if let Some(close) = prefix.find(')') {
                let t = &prefix[..open];
                let s = &prefix[open + 1..close];
                (t.to_string(), Some(s.to_string()))
            } else {
                (prefix.to_string(), None)
            }
        } else {
            (prefix.to_string(), None)
        };

        (CommitType::from_str(&type_str), scope, breaking, description)
    }
}

// ---------------------------------------------------------------------------
// Generator
// ---------------------------------------------------------------------------

/// Generates a Markdown changelog from a list of commits.
pub struct ChangelogGenerator {
    pub include_all_types: bool,
    pub include_hashes: bool,
}

impl Default for ChangelogGenerator {
    fn default() -> Self { Self { include_all_types: false, include_hashes: true } }
}

impl ChangelogGenerator {
    pub fn new() -> Self { Self::default() }

    /// Build a `Release` from a list of raw commit messages.
    pub fn build_release(
        &self,
        version: impl Into<String>,
        date: impl Into<String>,
        commits: &[(String, String)], // (hash, message)
    ) -> Release {
        let mut release = Release::new(version, date);
        for (hash, message) in commits {
            let commit = CommitParser::parse(hash, message);
            if self.include_all_types || commit.commit_type.is_notable() || commit.breaking {
                release.add_commit(commit);
            }
        }
        release
    }

    /// Render a release to Markdown.
    pub fn render_markdown(&self, release: &Release) -> String {
        let mut out = String::new();
        out.push_str(&format!("## [{}] — {}\n\n", release.version, release.date));

        // Breaking changes first
        if !release.breaking_changes.is_empty() {
            out.push_str("### ⚠ BREAKING CHANGES\n\n");
            for commit in &release.breaking_changes {
                out.push_str(&self.format_entry(commit));
            }
            out.push('\n');
        }

        // Notable sections in order
        let order = [
            CommitType::Feat,
            CommitType::Fix,
            CommitType::Perf,
            CommitType::Refactor,
            CommitType::Docs,
            CommitType::Test,
            CommitType::Chore,
            CommitType::Ci,
            CommitType::Build,
        ];

        for commit_type in &order {
            if let Some(commits) = release.sections.get(commit_type) {
                if commits.is_empty() { continue; }
                out.push_str(&format!("### {}\n\n", commit_type.section_title()));
                for commit in commits {
                    out.push_str(&self.format_entry(commit));
                }
                out.push('\n');
            }
        }

        out
    }

    fn format_entry(&self, commit: &ConventionalCommit) -> String {
        let scope_part = commit.scope.as_ref()
            .map(|s| format!("**{}**: ", s))
            .unwrap_or_default();
        let hash_part = if self.include_hashes {
            format!(" ({})", &commit.hash[..commit.hash.len().min(8)])
        } else {
            String::new()
        };
        format!("- {}{}{}\n", scope_part, commit.description, hash_part)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(hash: &str, msg: &str) -> (String, String) {
        (hash.to_string(), msg.to_string())
    }

    #[test]
    fn test_parse_feat() {
        let c = CommitParser::parse("abc1234", "feat: add dark mode");
        assert_eq!(c.commit_type, CommitType::Feat);
        assert_eq!(c.description, "add dark mode");
        assert!(!c.breaking);
    }

    #[test]
    fn test_parse_fix_with_scope() {
        let c = CommitParser::parse("def5678", "fix(auth): handle expired tokens");
        assert_eq!(c.commit_type, CommitType::Fix);
        assert_eq!(c.scope.as_deref(), Some("auth"));
        assert_eq!(c.description, "handle expired tokens");
    }

    #[test]
    fn test_parse_breaking_change() {
        let c = CommitParser::parse("xxx", "feat!: remove deprecated API");
        assert!(c.breaking);
        assert_eq!(c.commit_type, CommitType::Feat);
    }

    #[test]
    fn test_parse_breaking_footer() {
        let c = CommitParser::parse("yyy", "feat: new config format\n\nBREAKING CHANGE: old config removed");
        assert!(c.breaking);
    }

    #[test]
    fn test_parse_no_colon_unknown() {
        let c = CommitParser::parse("zzz", "just a random commit message");
        assert!(matches!(c.commit_type, CommitType::Unknown(_)));
    }

    #[test]
    fn test_build_release() {
        let gen = ChangelogGenerator::new();
        let commits = vec![
            commit("abc1234", "feat: new feature"),
            commit("def5678", "fix: resolve crash"),
            commit("ghi9012", "chore: update deps"), // not notable by default
        ];
        let release = gen.build_release("1.2.0", "2026-04-12", &commits);
        assert!(release.sections.contains_key(&CommitType::Feat));
        assert!(release.sections.contains_key(&CommitType::Fix));
        // chore is not notable, should not appear
        assert!(!release.sections.contains_key(&CommitType::Chore));
    }

    #[test]
    fn test_build_release_include_all() {
        let gen = ChangelogGenerator { include_all_types: true, include_hashes: true };
        let commits = vec![commit("abc", "chore: update deps")];
        let release = gen.build_release("1.0.0", "2026-04-12", &commits);
        assert!(release.sections.contains_key(&CommitType::Chore));
    }

    #[test]
    fn test_render_markdown_structure() {
        let gen = ChangelogGenerator::new();
        let commits = vec![
            commit("abc1234", "feat(ui): add dark mode"),
            commit("def5678", "fix: resolve memory leak"),
        ];
        let release = gen.build_release("2.0.0", "2026-04-12", &commits);
        let md = gen.render_markdown(&release);
        assert!(md.contains("## [2.0.0]"));
        assert!(md.contains("### Features"));
        assert!(md.contains("### Bug Fixes"));
        assert!(md.contains("**ui**"));
    }

    #[test]
    fn test_breaking_change_section() {
        let gen = ChangelogGenerator::new();
        let commits = vec![commit("abc1234", "feat!: drop old API")];
        let release = gen.build_release("3.0.0", "2026-04-12", &commits);
        let md = gen.render_markdown(&release);
        assert!(md.contains("⚠ BREAKING CHANGES"));
    }

    #[test]
    fn test_total_commits() {
        let gen = ChangelogGenerator::new();
        let commits = vec![
            commit("a", "feat: a"),
            commit("b", "feat: b"),
            commit("c", "fix: c"),
        ];
        let release = gen.build_release("1.0.0", "today", &commits);
        assert_eq!(release.total_commits(), 3);
    }

    #[test]
    fn test_commit_type_is_notable() {
        assert!(CommitType::Feat.is_notable());
        assert!(CommitType::Fix.is_notable());
        assert!(!CommitType::Chore.is_notable());
        assert!(!CommitType::Ci.is_notable());
    }

    #[test]
    fn test_hash_truncated_in_output() {
        let gen = ChangelogGenerator { include_hashes: true, ..Default::default() };
        let commits = vec![commit("abcdef1234567890", "feat: something")];
        let release = gen.build_release("1.0.0", "today", &commits);
        let md = gen.render_markdown(&release);
        assert!(md.contains("abcdef12")); // first 8 chars
    }

    #[test]
    fn test_no_hash_option() {
        let gen = ChangelogGenerator { include_hashes: false, ..Default::default() };
        let commits = vec![commit("abcdef1234567890", "feat: something")];
        let release = gen.build_release("1.0.0", "today", &commits);
        let md = gen.render_markdown(&release);
        assert!(!md.contains("abcdef12"));
    }
}
