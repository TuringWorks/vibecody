//! Text-to-speech and audio summary generation for VibeCody.
//!
//! Generates spoken summaries of code changes, PR descriptions, project status,
//! and code reviews. Closes the gap vs Jules (Google) which has audio changelogs.

use std::collections::HashMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum AudioError {
    ProviderNotConfigured(String),
    TextTooLong { length: usize, max: usize },
    InvalidVoice(String),
    GenerationFailed(String),
    FileWriteError(String),
    UnsupportedFormat(String),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioError::ProviderNotConfigured(p) => write!(f, "TTS provider not configured: {}", p),
            AudioError::TextTooLong { length, max } => {
                write!(f, "Text too long: {} chars (max {})", length, max)
            }
            AudioError::InvalidVoice(v) => write!(f, "Invalid voice ID: {}", v),
            AudioError::GenerationFailed(msg) => write!(f, "Audio generation failed: {}", msg),
            AudioError::FileWriteError(msg) => write!(f, "File write error: {}", msg),
            AudioError::UnsupportedFormat(fmt_name) => {
                write!(f, "Unsupported audio format: {}", fmt_name)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum TtsProvider {
    GoogleCloud,
    AwsPolly,
    AzureSpeech,
    PiperLocal,
    SystemTts,
}

impl TtsProvider {
    pub fn as_str(&self) -> &str {
        match self {
            TtsProvider::GoogleCloud => "google_cloud",
            TtsProvider::AwsPolly => "aws_polly",
            TtsProvider::AzureSpeech => "azure_speech",
            TtsProvider::PiperLocal => "piper_local",
            TtsProvider::SystemTts => "system_tts",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AudioFormat {
    Mp3,
    Wav,
    Ogg,
}

impl AudioFormat {
    pub fn extension(&self) -> &str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Wav => "wav",
            AudioFormat::Ogg => "ogg",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AudioRequestType {
    Changelog,
    PrSummary,
    ProjectStatus,
    CodeReview,
    CustomNarration,
}

impl AudioRequestType {
    pub fn as_str(&self) -> &str {
        match self {
            AudioRequestType::Changelog => "changelog",
            AudioRequestType::PrSummary => "pr_summary",
            AudioRequestType::ProjectStatus => "project_status",
            AudioRequestType::CodeReview => "code_review",
            AudioRequestType::CustomNarration => "custom",
        }
    }
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub tts_provider: TtsProvider,
    pub voice_id: String,
    pub speed: f32,
    pub output_format: AudioFormat,
    pub output_dir: String,
    pub language: String,
    pub max_text_length: usize,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            tts_provider: TtsProvider::SystemTts,
            voice_id: "default".to_string(),
            speed: 1.0,
            output_format: AudioFormat::Mp3,
            output_dir: ".vibecody/audio".to_string(),
            language: "en-US".to_string(),
            max_text_length: 5000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioRequest {
    pub request_type: AudioRequestType,
    pub text: String,
    pub title: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct AudioResult {
    pub file_path: String,
    pub duration_estimate_secs: f32,
    pub text_length: usize,
    pub request_type: AudioRequestType,
    pub title: String,
    pub timestamp: u64,
    pub word_count: usize,
}

#[derive(Debug, Clone)]
pub struct ChangelogEntry {
    pub commit_hash: String,
    pub author: String,
    pub message: String,
    pub files_changed: usize,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct ProjectStatusSummary {
    pub open_prs: usize,
    pub failing_tests: usize,
    pub active_agents: usize,
    pub recent_deploys: usize,
    pub coverage_percent: Option<f32>,
    pub pending_reviews: usize,
}

// ---------------------------------------------------------------------------
// AudioOutput
// ---------------------------------------------------------------------------

pub struct AudioOutput {
    config: AudioConfig,
    history: Vec<AudioResult>,
}

impl AudioOutput {
    pub fn new(config: AudioConfig) -> Self {
        Self {
            config,
            history: Vec::new(),
        }
    }

    // -- Generation methods -------------------------------------------------

    pub fn generate_changelog_audio(
        &mut self,
        entries: &[ChangelogEntry],
    ) -> Result<AudioResult, AudioError> {
        let text = Self::format_changelog_text(entries);
        self.generate_audio_internal(&text, "changelog", AudioRequestType::Changelog)
    }

    pub fn generate_pr_summary_audio(
        &mut self,
        title: &str,
        description: &str,
        files_changed: &[String],
        additions: usize,
        deletions: usize,
    ) -> Result<AudioResult, AudioError> {
        let text = Self::format_pr_text(title, description, files_changed, additions, deletions);
        let audio_title = format!("pr-{}", sanitize_filename(title));
        self.generate_audio_internal(&text, &audio_title, AudioRequestType::PrSummary)
    }

    pub fn generate_project_status_audio(
        &mut self,
        status: &ProjectStatusSummary,
    ) -> Result<AudioResult, AudioError> {
        let text = Self::format_status_text(status);
        self.generate_audio_internal(&text, "project-status", AudioRequestType::ProjectStatus)
    }

    pub fn generate_code_review_audio(
        &mut self,
        file: &str,
        issues: &[String],
        suggestions: &[String],
    ) -> Result<AudioResult, AudioError> {
        let text = Self::format_review_text(file, issues, suggestions);
        let title = format!("review-{}", sanitize_filename(file));
        self.generate_audio_internal(&text, &title, AudioRequestType::CodeReview)
    }

    pub fn generate_custom_audio(
        &mut self,
        text: &str,
        title: &str,
    ) -> Result<AudioResult, AudioError> {
        self.generate_audio_internal(text, title, AudioRequestType::CustomNarration)
    }

    // -- Formatting methods -------------------------------------------------

    pub fn format_changelog_text(entries: &[ChangelogEntry]) -> String {
        if entries.is_empty() {
            return "No changelog entries to report.".to_string();
        }

        let mut parts = Vec::new();
        parts.push(format!(
            "Changelog summary. {} commit{} to review.",
            entries.len(),
            if entries.len() == 1 { "" } else { "s" }
        ));

        for (i, entry) in entries.iter().enumerate() {
            let short_hash = if entry.commit_hash.len() >= 7 {
                &entry.commit_hash[..7]
            } else {
                &entry.commit_hash
            };
            parts.push(format!(
                "Commit {}: {} by {}, touching {} file{}. Hash {}.",
                i + 1,
                entry.message,
                entry.author,
                entry.files_changed,
                if entry.files_changed == 1 { "" } else { "s" },
                short_hash,
            ));
        }

        parts.join(" ")
    }

    pub fn format_pr_text(
        title: &str,
        description: &str,
        files: &[String],
        additions: usize,
        deletions: usize,
    ) -> String {
        let mut parts = Vec::new();
        parts.push(format!("Pull request: {}.", title));

        if !description.is_empty() {
            parts.push(format!("Description: {}", description));
        }

        parts.push(format!(
            "This PR changes {} file{}, with {} addition{} and {} deletion{}.",
            files.len(),
            if files.len() == 1 { "" } else { "s" },
            additions,
            if additions == 1 { "" } else { "s" },
            deletions,
            if deletions == 1 { "" } else { "s" },
        ));

        if !files.is_empty() {
            let display_files: Vec<&str> = files.iter().take(5).map(|s| s.as_str()).collect();
            parts.push(format!("Files modified: {}.", display_files.join(", ")));
            if files.len() > 5 {
                parts.push(format!("And {} more files.", files.len() - 5));
            }
        }

        parts.join(" ")
    }

    pub fn format_status_text(status: &ProjectStatusSummary) -> String {
        let mut parts = Vec::new();
        parts.push("Project status report.".to_string());

        parts.push(format!(
            "{} open pull request{}.",
            status.open_prs,
            if status.open_prs == 1 { "" } else { "s" }
        ));

        if status.failing_tests > 0 {
            parts.push(format!(
                "Warning: {} failing test{}.",
                status.failing_tests,
                if status.failing_tests == 1 { "" } else { "s" }
            ));
        } else {
            parts.push("All tests passing.".to_string());
        }

        parts.push(format!(
            "{} active agent{}, {} recent deployment{}.",
            status.active_agents,
            if status.active_agents == 1 { "" } else { "s" },
            status.recent_deploys,
            if status.recent_deploys == 1 { "" } else { "s" },
        ));

        if let Some(cov) = status.coverage_percent {
            parts.push(format!("Code coverage at {:.1} percent.", cov));
        }

        parts.push(format!(
            "{} pending review{}.",
            status.pending_reviews,
            if status.pending_reviews == 1 { "" } else { "s" }
        ));

        parts.join(" ")
    }

    pub fn format_review_text(file: &str, issues: &[String], suggestions: &[String]) -> String {
        let mut parts = Vec::new();
        parts.push(format!("Code review for {}.", file));

        if issues.is_empty() {
            parts.push("No issues found.".to_string());
        } else {
            parts.push(format!(
                "{} issue{} found.",
                issues.len(),
                if issues.len() == 1 { "" } else { "s" }
            ));
            for (i, issue) in issues.iter().enumerate() {
                parts.push(format!("Issue {}: {}", i + 1, issue));
            }
        }

        if !suggestions.is_empty() {
            parts.push(format!(
                "{} suggestion{}.",
                suggestions.len(),
                if suggestions.len() == 1 { "" } else { "s" }
            ));
            for (i, suggestion) in suggestions.iter().enumerate() {
                parts.push(format!("Suggestion {}: {}", i + 1, suggestion));
            }
        }

        parts.join(" ")
    }

    // -- Utility methods ----------------------------------------------------

    pub fn estimate_duration(text: &str, speed: f32) -> f32 {
        let words = Self::count_words(text);
        let words_per_minute = 150.0 * speed;
        if words_per_minute <= 0.0 {
            return 0.0;
        }
        (words as f32 / words_per_minute) * 60.0
    }

    pub fn count_words(text: &str) -> usize {
        text.split_whitespace().count()
    }

    pub fn truncate_text(text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            return text.to_string();
        }
        let truncated = &text[..max_len.min(text.len())];
        format!("{}...", truncated)
    }

    pub fn generate_tts_command(&self, text: &str, output_path: &str) -> String {
        match self.config.tts_provider {
            TtsProvider::GoogleCloud => {
                format!(
                    "gcloud text-to-speech synthesize --text=\"{}\" --voice=\"{}\" --language=\"{}\" --speed={} --output=\"{}\"",
                    escape_shell(text),
                    self.config.voice_id,
                    self.config.language,
                    self.config.speed,
                    output_path,
                )
            }
            TtsProvider::AwsPolly => {
                format!(
                    "aws polly synthesize-speech --text \"{}\" --voice-id \"{}\" --output-format {} --output \"{}\"",
                    escape_shell(text),
                    self.config.voice_id,
                    self.config.output_format.extension(),
                    output_path,
                )
            }
            TtsProvider::AzureSpeech => {
                format!(
                    "az cognitiveservices speech synthesize --text=\"{}\" --voice=\"{}\" --language=\"{}\" --output=\"{}\"",
                    escape_shell(text),
                    self.config.voice_id,
                    self.config.language,
                    output_path,
                )
            }
            TtsProvider::PiperLocal => {
                format!(
                    "echo \"{}\" | piper --model \"{}\" --output_file \"{}\"",
                    escape_shell(text),
                    self.config.voice_id,
                    output_path,
                )
            }
            TtsProvider::SystemTts => {
                format!("say -o \"{}\" \"{}\"", output_path, escape_shell(text))
            }
        }
    }

    pub fn generate_output_path(&self, request_type: &AudioRequestType, title: &str) -> String {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let safe_title = sanitize_filename(title);
        format!(
            "{}/{}-{}-{}.{}",
            self.config.output_dir,
            request_type.as_str(),
            safe_title,
            ts,
            self.config.output_format.extension(),
        )
    }

    pub fn get_history(&self) -> Vec<&AudioResult> {
        self.history.iter().collect()
    }

    // -- Internal -----------------------------------------------------------

    fn generate_audio_internal(
        &mut self,
        text: &str,
        title: &str,
        request_type: AudioRequestType,
    ) -> Result<AudioResult, AudioError> {
        if text.len() > self.config.max_text_length {
            return Err(AudioError::TextTooLong {
                length: text.len(),
                max: self.config.max_text_length,
            });
        }

        let final_text = Self::truncate_text(text, self.config.max_text_length);
        let file_path = self.generate_output_path(&request_type, title);
        let duration = Self::estimate_duration(&final_text, self.config.speed);
        let word_count = Self::count_words(&final_text);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let result = AudioResult {
            file_path,
            duration_estimate_secs: duration,
            text_length: final_text.len(),
            request_type,
            title: title.to_string(),
            timestamp,
            word_count,
        };

        self.history.push(result.clone());
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .to_lowercase()
}

fn escape_shell(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_output() -> AudioOutput {
        AudioOutput::new(AudioConfig::default())
    }

    fn sample_entry() -> ChangelogEntry {
        ChangelogEntry {
            commit_hash: "abc1234def5678".to_string(),
            author: "Alice".to_string(),
            message: "Fix login bug".to_string(),
            files_changed: 3,
            timestamp: 1700000000,
        }
    }

    // -- Config defaults ----------------------------------------------------

    #[test]
    fn test_config_defaults() {
        let cfg = AudioConfig::default();
        assert_eq!(cfg.tts_provider, TtsProvider::SystemTts);
        assert_eq!(cfg.voice_id, "default");
        assert_eq!(cfg.speed, 1.0);
        assert_eq!(cfg.output_format, AudioFormat::Mp3);
        assert_eq!(cfg.output_dir, ".vibecody/audio");
        assert_eq!(cfg.language, "en-US");
        assert_eq!(cfg.max_text_length, 5000);
    }

    // -- AudioFormat extensions ---------------------------------------------

    #[test]
    fn test_audio_format_extensions() {
        assert_eq!(AudioFormat::Mp3.extension(), "mp3");
        assert_eq!(AudioFormat::Wav.extension(), "wav");
        assert_eq!(AudioFormat::Ogg.extension(), "ogg");
    }

    // -- Changelog formatting -----------------------------------------------

    #[test]
    fn test_format_changelog_single_entry() {
        let entries = vec![sample_entry()];
        let text = AudioOutput::format_changelog_text(&entries);
        assert!(text.contains("1 commit to review"));
        assert!(text.contains("Fix login bug"));
        assert!(text.contains("Alice"));
        assert!(text.contains("abc1234"));
    }

    #[test]
    fn test_format_changelog_multiple_entries() {
        let entries = vec![
            sample_entry(),
            ChangelogEntry {
                commit_hash: "bbb2222".to_string(),
                author: "Bob".to_string(),
                message: "Add tests".to_string(),
                files_changed: 1,
                timestamp: 1700000100,
            },
        ];
        let text = AudioOutput::format_changelog_text(&entries);
        assert!(text.contains("2 commits to review"));
        assert!(text.contains("Commit 1:"));
        assert!(text.contains("Commit 2:"));
        assert!(text.contains("1 file"));
    }

    #[test]
    fn test_format_changelog_empty() {
        let text = AudioOutput::format_changelog_text(&[]);
        assert_eq!(text, "No changelog entries to report.");
    }

    // -- PR summary formatting ----------------------------------------------

    #[test]
    fn test_format_pr_text() {
        let text = AudioOutput::format_pr_text(
            "Add auth middleware",
            "Adds JWT validation",
            &["src/auth.rs".to_string(), "src/main.rs".to_string()],
            50,
            10,
        );
        assert!(text.contains("Pull request: Add auth middleware"));
        assert!(text.contains("JWT validation"));
        assert!(text.contains("2 files"));
        assert!(text.contains("50 additions"));
        assert!(text.contains("10 deletions"));
    }

    #[test]
    fn test_format_pr_text_empty_description() {
        let text = AudioOutput::format_pr_text("Title", "", &[], 0, 0);
        assert!(text.contains("Pull request: Title"));
        assert!(!text.contains("Description:"));
    }

    #[test]
    fn test_format_pr_text_many_files() {
        let files: Vec<String> = (0..8).map(|i| format!("file{}.rs", i)).collect();
        let text = AudioOutput::format_pr_text("Big PR", "Lots of changes", &files, 200, 50);
        assert!(text.contains("8 files"));
        assert!(text.contains("And 3 more files"));
    }

    // -- Status formatting --------------------------------------------------

    #[test]
    fn test_format_status_healthy() {
        let status = ProjectStatusSummary {
            open_prs: 2,
            failing_tests: 0,
            active_agents: 3,
            recent_deploys: 1,
            coverage_percent: Some(87.5),
            pending_reviews: 1,
        };
        let text = AudioOutput::format_status_text(&status);
        assert!(text.contains("2 open pull requests"));
        assert!(text.contains("All tests passing"));
        assert!(text.contains("87.5 percent"));
        assert!(text.contains("1 pending review."));
    }

    #[test]
    fn test_format_status_with_issues() {
        let status = ProjectStatusSummary {
            open_prs: 5,
            failing_tests: 3,
            active_agents: 0,
            recent_deploys: 0,
            coverage_percent: None,
            pending_reviews: 10,
        };
        let text = AudioOutput::format_status_text(&status);
        assert!(text.contains("Warning: 3 failing tests"));
        assert!(!text.contains("coverage"));
        assert!(text.contains("10 pending reviews"));
    }

    #[test]
    fn test_format_status_all_zeros() {
        let status = ProjectStatusSummary {
            open_prs: 0,
            failing_tests: 0,
            active_agents: 0,
            recent_deploys: 0,
            coverage_percent: None,
            pending_reviews: 0,
        };
        let text = AudioOutput::format_status_text(&status);
        assert!(text.contains("0 open pull requests"));
        assert!(text.contains("All tests passing"));
        assert!(text.contains("0 pending reviews"));
    }

    // -- Code review formatting ---------------------------------------------

    #[test]
    fn test_format_review_with_issues_and_suggestions() {
        let text = AudioOutput::format_review_text(
            "src/lib.rs",
            &["Unused import".to_string(), "Missing docs".to_string()],
            &["Add error handling".to_string()],
        );
        assert!(text.contains("Code review for src/lib.rs"));
        assert!(text.contains("2 issues found"));
        assert!(text.contains("Issue 1: Unused import"));
        assert!(text.contains("1 suggestion"));
        assert!(text.contains("Suggestion 1: Add error handling"));
    }

    #[test]
    fn test_format_review_no_issues() {
        let text = AudioOutput::format_review_text("main.rs", &[], &[]);
        assert!(text.contains("No issues found"));
    }

    // -- Duration estimation ------------------------------------------------

    #[test]
    fn test_estimate_duration_short() {
        // 10 words at 150 wpm = 4 seconds
        let text = "one two three four five six seven eight nine ten";
        let dur = AudioOutput::estimate_duration(text, 1.0);
        assert!((dur - 4.0).abs() < 0.1);
    }

    #[test]
    fn test_estimate_duration_long() {
        let words: Vec<&str> = std::iter::repeat("word").take(300).collect();
        let text = words.join(" ");
        // 300 words / 150 wpm = 2 min = 120 sec
        let dur = AudioOutput::estimate_duration(&text, 1.0);
        assert!((dur - 120.0).abs() < 0.5);
    }

    #[test]
    fn test_estimate_duration_speed_factor() {
        let text = "one two three four five six seven eight nine ten";
        let dur_normal = AudioOutput::estimate_duration(text, 1.0);
        let dur_fast = AudioOutput::estimate_duration(text, 2.0);
        assert!((dur_fast - dur_normal / 2.0).abs() < 0.1);
    }

    // -- Word counting ------------------------------------------------------

    #[test]
    fn test_count_words() {
        assert_eq!(AudioOutput::count_words("hello world"), 2);
        assert_eq!(AudioOutput::count_words(""), 0);
        assert_eq!(AudioOutput::count_words("  spaces  between  "), 2);
        assert_eq!(AudioOutput::count_words("single"), 1);
    }

    // -- Text truncation ----------------------------------------------------

    #[test]
    fn test_truncate_within_limit() {
        let result = AudioOutput::truncate_text("short", 100);
        assert_eq!(result, "short");
    }

    #[test]
    fn test_truncate_exceeds_limit() {
        let result = AudioOutput::truncate_text("hello world", 5);
        assert_eq!(result, "hello...");
    }

    // -- TTS command generation ---------------------------------------------

    #[test]
    fn test_tts_command_google_cloud() {
        let mut cfg = AudioConfig::default();
        cfg.tts_provider = TtsProvider::GoogleCloud;
        let ao = AudioOutput::new(cfg);
        let cmd = ao.generate_tts_command("hello", "/tmp/out.mp3");
        assert!(cmd.contains("gcloud text-to-speech"));
        assert!(cmd.contains("hello"));
        assert!(cmd.contains("/tmp/out.mp3"));
    }

    #[test]
    fn test_tts_command_aws_polly() {
        let mut cfg = AudioConfig::default();
        cfg.tts_provider = TtsProvider::AwsPolly;
        let ao = AudioOutput::new(cfg);
        let cmd = ao.generate_tts_command("test", "/tmp/out.mp3");
        assert!(cmd.contains("aws polly synthesize-speech"));
        assert!(cmd.contains("mp3"));
    }

    #[test]
    fn test_tts_command_azure_speech() {
        let mut cfg = AudioConfig::default();
        cfg.tts_provider = TtsProvider::AzureSpeech;
        let ao = AudioOutput::new(cfg);
        let cmd = ao.generate_tts_command("test", "/tmp/out.mp3");
        assert!(cmd.contains("az cognitiveservices speech"));
    }

    #[test]
    fn test_tts_command_piper_local() {
        let mut cfg = AudioConfig::default();
        cfg.tts_provider = TtsProvider::PiperLocal;
        let ao = AudioOutput::new(cfg);
        let cmd = ao.generate_tts_command("test", "/tmp/out.wav");
        assert!(cmd.contains("piper"));
        assert!(cmd.contains("--output_file"));
    }

    #[test]
    fn test_tts_command_system_tts() {
        let ao = default_output();
        let cmd = ao.generate_tts_command("hello", "/tmp/out.mp3");
        assert!(cmd.contains("say"));
        assert!(cmd.contains("hello"));
    }

    // -- Output path generation ---------------------------------------------

    #[test]
    fn test_output_path_format() {
        let ao = default_output();
        let path = ao.generate_output_path(&AudioRequestType::Changelog, "v1.0");
        assert!(path.starts_with(".vibecody/audio/changelog-v1-0-"));
        assert!(path.ends_with(".mp3"));
    }

    #[test]
    fn test_output_path_different_format() {
        let mut cfg = AudioConfig::default();
        cfg.output_format = AudioFormat::Wav;
        let ao = AudioOutput::new(cfg);
        let path = ao.generate_output_path(&AudioRequestType::PrSummary, "feat");
        assert!(path.ends_with(".wav"));
        assert!(path.contains("pr_summary"));
    }

    // -- Custom narration ---------------------------------------------------

    #[test]
    fn test_generate_custom_audio() {
        let mut ao = default_output();
        let result = ao.generate_custom_audio("Hello from VibeCody", "greeting").unwrap();
        assert_eq!(result.request_type, AudioRequestType::CustomNarration);
        assert_eq!(result.title, "greeting");
        assert!(result.word_count > 0);
    }

    // -- Request history ----------------------------------------------------

    #[test]
    fn test_history_tracking() {
        let mut ao = default_output();
        assert!(ao.get_history().is_empty());

        ao.generate_custom_audio("first", "one").unwrap();
        ao.generate_custom_audio("second", "two").unwrap();

        let hist = ao.get_history();
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0].title, "one");
        assert_eq!(hist[1].title, "two");
    }

    // -- Error cases --------------------------------------------------------

    #[test]
    fn test_text_too_long_error() {
        let mut cfg = AudioConfig::default();
        cfg.max_text_length = 10;
        let mut ao = AudioOutput::new(cfg);
        let result = ao.generate_custom_audio("this text is definitely longer than ten chars", "t");
        assert!(result.is_err());
        match result.unwrap_err() {
            AudioError::TextTooLong { length, max } => {
                assert!(length > 10);
                assert_eq!(max, 10);
            }
            other => panic!("Expected TextTooLong, got {:?}", other),
        }
    }

    #[test]
    fn test_audio_error_display() {
        let err = AudioError::ProviderNotConfigured("google".to_string());
        assert!(err.to_string().contains("google"));

        let err = AudioError::TextTooLong { length: 6000, max: 5000 };
        assert!(err.to_string().contains("6000"));

        let err = AudioError::InvalidVoice("bad_id".to_string());
        assert!(err.to_string().contains("bad_id"));
    }

    // -- AudioRequestType as_str --------------------------------------------

    #[test]
    fn test_request_type_as_str() {
        assert_eq!(AudioRequestType::Changelog.as_str(), "changelog");
        assert_eq!(AudioRequestType::PrSummary.as_str(), "pr_summary");
        assert_eq!(AudioRequestType::ProjectStatus.as_str(), "project_status");
        assert_eq!(AudioRequestType::CodeReview.as_str(), "code_review");
        assert_eq!(AudioRequestType::CustomNarration.as_str(), "custom");
    }

    // -- TtsProvider as_str -------------------------------------------------

    #[test]
    fn test_tts_provider_as_str() {
        assert_eq!(TtsProvider::GoogleCloud.as_str(), "google_cloud");
        assert_eq!(TtsProvider::AwsPolly.as_str(), "aws_polly");
        assert_eq!(TtsProvider::AzureSpeech.as_str(), "azure_speech");
        assert_eq!(TtsProvider::PiperLocal.as_str(), "piper_local");
        assert_eq!(TtsProvider::SystemTts.as_str(), "system_tts");
    }

    // -- Changelog audio generation -----------------------------------------

    #[test]
    fn test_generate_changelog_audio() {
        let mut ao = default_output();
        let entries = vec![sample_entry()];
        let result = ao.generate_changelog_audio(&entries).unwrap();
        assert_eq!(result.request_type, AudioRequestType::Changelog);
        assert!(result.duration_estimate_secs > 0.0);
        assert!(result.file_path.contains("changelog"));
    }

    // -- PR summary audio generation ----------------------------------------

    #[test]
    fn test_generate_pr_summary_audio() {
        let mut ao = default_output();
        let result = ao
            .generate_pr_summary_audio("Fix bug", "Fixes #123", &["a.rs".to_string()], 10, 2)
            .unwrap();
        assert_eq!(result.request_type, AudioRequestType::PrSummary);
        assert!(result.file_path.contains("pr_summary"));
    }

    // -- Project status audio generation ------------------------------------

    #[test]
    fn test_generate_project_status_audio() {
        let mut ao = default_output();
        let status = ProjectStatusSummary {
            open_prs: 1,
            failing_tests: 0,
            active_agents: 2,
            recent_deploys: 1,
            coverage_percent: Some(90.0),
            pending_reviews: 0,
        };
        let result = ao.generate_project_status_audio(&status).unwrap();
        assert_eq!(result.request_type, AudioRequestType::ProjectStatus);
    }

    // -- Code review audio generation ---------------------------------------

    #[test]
    fn test_generate_code_review_audio() {
        let mut ao = default_output();
        let result = ao
            .generate_code_review_audio("lib.rs", &["warning".to_string()], &[])
            .unwrap();
        assert_eq!(result.request_type, AudioRequestType::CodeReview);
        assert!(result.file_path.contains("review"));
    }

    // -- Sanitize filename --------------------------------------------------

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Hello World!"), "hello-world-");
        assert_eq!(sanitize_filename("src/main.rs"), "src-main-rs");
        assert_eq!(sanitize_filename("simple"), "simple");
    }

    // -- Shell escaping -----------------------------------------------------

    #[test]
    fn test_escape_shell() {
        assert_eq!(escape_shell("hello"), "hello");
        assert_eq!(escape_shell("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_shell("$HOME"), "\\$HOME");
    }
}
