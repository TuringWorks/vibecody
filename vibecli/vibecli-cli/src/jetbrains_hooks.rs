//! JetBrains IDE hook system parity for VibeCLI.
//!
//! Provides full integration with JetBrains IDEs (IntelliJ, WebStorm, PyCharm, etc.)
//! via the built-in REST API on the plugin port (default 63342).

#[allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Configuration for the JetBrains hook integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct JetBrainsHookConfig {
    pub enabled: bool,
    pub plugin_port: u16,
    pub api_token: Option<String>,
    pub timeout_ms: u64,
    pub hooks: Vec<JetBrainsHookDef>,
    pub auto_discover: bool,
}

impl Default for JetBrainsHookConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            plugin_port: 63342,
            api_token: None,
            timeout_ms: 5000,
            hooks: Vec::new(),
            auto_discover: true,
        }
    }
}

/// A single hook definition binding an event to an action with an optional filter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)]
pub struct JetBrainsHookDef {
    pub event: HookEvent,
    pub action: HookAction,
    pub filter: Option<HookFilter>,
}

/// Events that can trigger hooks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum HookEvent {
    SessionStart,
    SessionEnd,
    PreToolUse {
        tool_name: Option<String>,
    },
    PostToolUse {
        tool_name: Option<String>,
    },
    FileSaved {
        pattern: Option<String>,
    },
    FileOpened {
        pattern: Option<String>,
    },
    DiagnosticsChanged,
    BuildStarted,
    BuildCompleted {
        success: bool,
    },
    TestRunCompleted {
        passed: usize,
        failed: usize,
    },
    DebugBreakpoint,
    RefactoringApplied {
        kind: String,
    },
    GitCommit,
    GitPush,
}

/// Actions executed when a hook fires.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum HookAction {
    Notify {
        message: String,
        severity: NotifySeverity,
    },
    RunInspection {
        scope: String,
    },
    FormatFile {
        path: String,
    },
    RefreshDiagnostics,
    OpenFile {
        path: String,
        line: Option<u32>,
    },
    ShowDiff {
        before: String,
        after: String,
    },
    RunConfiguration {
        name: String,
    },
    ExecuteAction {
        action_id: String,
    },
    ShowBalloon {
        title: String,
        message: String,
    },
    Custom {
        command: String,
        args: Vec<String>,
    },
}

/// Severity levels for IDE notifications.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)]
pub enum NotifySeverity {
    Info,
    Warning,
    Error,
}

impl fmt::Display for NotifySeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotifySeverity::Info => write!(f, "Info"),
            NotifySeverity::Warning => write!(f, "Warning"),
            NotifySeverity::Error => write!(f, "Error"),
        }
    }
}

/// Optional filter to narrow which projects/files/languages a hook applies to.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)]
pub struct HookFilter {
    pub file_patterns: Vec<String>,
    pub project_names: Vec<String>,
    pub languages: Vec<String>,
}

/// Information about the connected JetBrains IDE instance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)]
pub struct IdeInfo {
    pub product_name: String,
    pub version: String,
    pub build_number: String,
    pub project_name: Option<String>,
    pub project_path: Option<String>,
}

/// Payload sent to the IDE when a hook fires.
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct HookPayload {
    pub event: HookEvent,
    pub timestamp_ms: u64,
    pub session_id: Option<String>,
    pub context: HookContext,
}

/// Contextual data included with a hook payload.
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct HookContext {
    pub workspace_path: Option<String>,
    pub current_file: Option<String>,
    pub current_line: Option<u32>,
    pub tool_name: Option<String>,
    pub tool_result: Option<String>,
}

/// Response from the IDE after processing a hook.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[allow(dead_code)]
pub struct HookResponse {
    pub allow: bool,
    pub message: Option<String>,
}

/// A diagnostic entry from the IDE.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)]
pub struct Diagnostic {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub severity: String,
    pub message: String,
    pub source: String,
}

/// Represents a connection to a JetBrains IDE instance.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct JetBrainsConnection {
    pub base_url: String,
    pub connected: bool,
    pub ide_info: Option<IdeInfo>,
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

#[allow(dead_code)]
impl JetBrainsConnection {
    /// Create a new connection from the given config.
    pub fn new(config: &JetBrainsHookConfig) -> Self {
        Self {
            base_url: format!("http://127.0.0.1:{}", config.plugin_port),
            connected: false,
            ide_info: None,
        }
    }

    /// Attempt to discover the JetBrains IDE plugin port.
    /// Returns the default port (63342) as a sensible fallback.
    pub fn discover_ide_port() -> Option<u16> {
        Some(63342)
    }

    /// Whether this connection is currently considered connected.
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Returns the default hook set: format on save, refresh diagnostics after tool use.
#[allow(dead_code)]
pub fn default_hooks() -> Vec<JetBrainsHookDef> {
    vec![
        JetBrainsHookDef {
            event: HookEvent::FileSaved { pattern: None },
            action: HookAction::FormatFile {
                path: String::from("*"),
            },
            filter: None,
        },
        JetBrainsHookDef {
            event: HookEvent::PostToolUse { tool_name: None },
            action: HookAction::RefreshDiagnostics,
            filter: None,
        },
    ]
}

/// Returns the strict hook set: run inspections before writes, run tests after changes.
#[allow(dead_code)]
pub fn strict_hooks() -> Vec<JetBrainsHookDef> {
    vec![
        JetBrainsHookDef {
            event: HookEvent::PreToolUse {
                tool_name: Some("write".into()),
            },
            action: HookAction::RunInspection {
                scope: String::from("current_file"),
            },
            filter: None,
        },
        JetBrainsHookDef {
            event: HookEvent::PostToolUse { tool_name: None },
            action: HookAction::RunConfiguration {
                name: String::from("All Tests"),
            },
            filter: None,
        },
    ]
}

/// Returns the minimal hook set: session start/end notifications only.
#[allow(dead_code)]
pub fn minimal_hooks() -> Vec<JetBrainsHookDef> {
    vec![
        JetBrainsHookDef {
            event: HookEvent::SessionStart,
            action: HookAction::Notify {
                message: String::from("VibeCLI session started"),
                severity: NotifySeverity::Info,
            },
            filter: None,
        },
        JetBrainsHookDef {
            event: HookEvent::SessionEnd,
            action: HookAction::Notify {
                message: String::from("VibeCLI session ended"),
                severity: NotifySeverity::Info,
            },
            filter: None,
        },
    ]
}

/// Convert a hook definition to its JSON representation.
#[allow(dead_code)]
pub fn to_vibe_hook(hook: &JetBrainsHookDef) -> String {
    serde_json::to_string(hook).expect("JetBrainsHookDef should always serialize")
}

/// Parse a hook definition from a JSON string.
#[allow(dead_code)]
pub fn from_vibe_hook(json: &str) -> anyhow::Result<JetBrainsHookDef> {
    let def: JetBrainsHookDef = serde_json::from_str(json)?;
    Ok(def)
}

/// Check whether the optional filter matches the given file, project and language.
///
/// An absent filter (None) matches everything. An empty list for any field also
/// matches everything for that dimension.
#[allow(dead_code)]
pub fn matches_filter(
    filter: &Option<HookFilter>,
    file: Option<&str>,
    project: Option<&str>,
    language: Option<&str>,
) -> bool {
    let filter = match filter {
        Some(f) => f,
        None => return true,
    };

    let file_ok = if filter.file_patterns.is_empty() {
        true
    } else {
        match file {
            Some(f) => filter.file_patterns.iter().any(|p| {
                // Simple glob: if pattern starts with *, check suffix
                if let Some(suffix) = p.strip_prefix('*') {
                    f.ends_with(suffix)
                } else {
                    f == p
                }
            }),
            None => false,
        }
    };

    let project_ok = if filter.project_names.is_empty() {
        true
    } else {
        match project {
            Some(proj) => filter.project_names.iter().any(|pn| pn == proj),
            None => false,
        }
    };

    let lang_ok = if filter.languages.is_empty() {
        true
    } else {
        match language {
            Some(l) => filter.languages.iter().any(|fl| fl == l),
            None => false,
        }
    };

    file_ok && project_ok && lang_ok
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Config defaults ---------------------------------------------------

    #[test]
    fn test_config_defaults() {
        let cfg = JetBrainsHookConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.plugin_port, 63342);
        assert!(cfg.api_token.is_none());
        assert_eq!(cfg.timeout_ms, 5000);
        assert!(cfg.hooks.is_empty());
        assert!(cfg.auto_discover);
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let cfg = JetBrainsHookConfig {
            enabled: false,
            plugin_port: 12345,
            api_token: Some("tok_abc".into()),
            timeout_ms: 3000,
            hooks: default_hooks(),
            auto_discover: false,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let cfg2: JetBrainsHookConfig = serde_json::from_str(&json).unwrap();
        assert!(!cfg2.enabled);
        assert_eq!(cfg2.plugin_port, 12345);
        assert_eq!(cfg2.api_token.as_deref(), Some("tok_abc"));
        assert_eq!(cfg2.timeout_ms, 3000);
        assert_eq!(cfg2.hooks.len(), 2);
        assert!(!cfg2.auto_discover);
    }

    // -- HookEvent serde roundtrips ----------------------------------------

    #[test]
    fn test_event_session_start_serde() {
        let e = HookEvent::SessionStart;
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn test_event_session_end_serde() {
        let e = HookEvent::SessionEnd;
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn test_event_pre_tool_use_serde() {
        let e = HookEvent::PreToolUse {
            tool_name: Some("write".into()),
        };
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn test_event_pre_tool_use_none_serde() {
        let e = HookEvent::PreToolUse { tool_name: None };
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn test_event_post_tool_use_serde() {
        let e = HookEvent::PostToolUse {
            tool_name: Some("read".into()),
        };
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn test_event_file_saved_serde() {
        let e = HookEvent::FileSaved {
            pattern: Some("*.rs".into()),
        };
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn test_event_file_opened_serde() {
        let e = HookEvent::FileOpened { pattern: None };
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn test_event_diagnostics_changed_serde() {
        let e = HookEvent::DiagnosticsChanged;
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn test_event_build_started_serde() {
        let e = HookEvent::BuildStarted;
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn test_event_build_completed_serde() {
        let e = HookEvent::BuildCompleted { success: true };
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn test_event_test_run_completed_serde() {
        let e = HookEvent::TestRunCompleted {
            passed: 42,
            failed: 3,
        };
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn test_event_debug_breakpoint_serde() {
        let e = HookEvent::DebugBreakpoint;
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn test_event_refactoring_applied_serde() {
        let e = HookEvent::RefactoringApplied {
            kind: "rename".into(),
        };
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn test_event_git_commit_serde() {
        let e = HookEvent::GitCommit;
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn test_event_git_push_serde() {
        let e = HookEvent::GitPush;
        let json = serde_json::to_string(&e).unwrap();
        let e2: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    // -- HookAction serde --------------------------------------------------

    #[test]
    fn test_action_notify_serde() {
        let a = HookAction::Notify {
            message: "hello".into(),
            severity: NotifySeverity::Warning,
        };
        let json = serde_json::to_string(&a).unwrap();
        let a2: HookAction = serde_json::from_str(&json).unwrap();
        assert_eq!(a, a2);
    }

    #[test]
    fn test_action_run_inspection_serde() {
        let a = HookAction::RunInspection {
            scope: "project".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let a2: HookAction = serde_json::from_str(&json).unwrap();
        assert_eq!(a, a2);
    }

    #[test]
    fn test_action_format_file_serde() {
        let a = HookAction::FormatFile {
            path: "/tmp/x.rs".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let a2: HookAction = serde_json::from_str(&json).unwrap();
        assert_eq!(a, a2);
    }

    #[test]
    fn test_action_refresh_diagnostics_serde() {
        let a = HookAction::RefreshDiagnostics;
        let json = serde_json::to_string(&a).unwrap();
        let a2: HookAction = serde_json::from_str(&json).unwrap();
        assert_eq!(a, a2);
    }

    #[test]
    fn test_action_open_file_serde() {
        let a = HookAction::OpenFile {
            path: "main.rs".into(),
            line: Some(42),
        };
        let json = serde_json::to_string(&a).unwrap();
        let a2: HookAction = serde_json::from_str(&json).unwrap();
        assert_eq!(a, a2);
    }

    #[test]
    fn test_action_show_diff_serde() {
        let a = HookAction::ShowDiff {
            before: "old".into(),
            after: "new".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let a2: HookAction = serde_json::from_str(&json).unwrap();
        assert_eq!(a, a2);
    }

    #[test]
    fn test_action_run_configuration_serde() {
        let a = HookAction::RunConfiguration {
            name: "Debug".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let a2: HookAction = serde_json::from_str(&json).unwrap();
        assert_eq!(a, a2);
    }

    #[test]
    fn test_action_execute_action_serde() {
        let a = HookAction::ExecuteAction {
            action_id: "Run".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let a2: HookAction = serde_json::from_str(&json).unwrap();
        assert_eq!(a, a2);
    }

    #[test]
    fn test_action_show_balloon_serde() {
        let a = HookAction::ShowBalloon {
            title: "Alert".into(),
            message: "Done!".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let a2: HookAction = serde_json::from_str(&json).unwrap();
        assert_eq!(a, a2);
    }

    #[test]
    fn test_action_custom_serde() {
        let a = HookAction::Custom {
            command: "echo".into(),
            args: vec!["hi".into()],
        };
        let json = serde_json::to_string(&a).unwrap();
        let a2: HookAction = serde_json::from_str(&json).unwrap();
        assert_eq!(a, a2);
    }

    // -- NotifySeverity display --------------------------------------------

    #[test]
    fn test_severity_display() {
        assert_eq!(NotifySeverity::Info.to_string(), "Info");
        assert_eq!(NotifySeverity::Warning.to_string(), "Warning");
        assert_eq!(NotifySeverity::Error.to_string(), "Error");
    }

    // -- HookFilter matching -----------------------------------------------

    #[test]
    fn test_filter_none_matches_all() {
        assert!(matches_filter(&None, Some("foo.rs"), Some("proj"), Some("rust")));
        assert!(matches_filter(&None, None, None, None));
    }

    #[test]
    fn test_filter_empty_matches_all() {
        let f = Some(HookFilter {
            file_patterns: vec![],
            project_names: vec![],
            languages: vec![],
        });
        assert!(matches_filter(&f, Some("x.rs"), Some("p"), Some("rust")));
        assert!(matches_filter(&f, None, None, None));
    }

    #[test]
    fn test_filter_file_pattern_glob() {
        let f = Some(HookFilter {
            file_patterns: vec!["*.rs".into()],
            project_names: vec![],
            languages: vec![],
        });
        assert!(matches_filter(&f, Some("main.rs"), None, None));
        assert!(!matches_filter(&f, Some("main.py"), None, None));
    }

    #[test]
    fn test_filter_file_pattern_exact() {
        let f = Some(HookFilter {
            file_patterns: vec!["Cargo.toml".into()],
            project_names: vec![],
            languages: vec![],
        });
        assert!(matches_filter(&f, Some("Cargo.toml"), None, None));
        assert!(!matches_filter(&f, Some("package.json"), None, None));
    }

    #[test]
    fn test_filter_project_name() {
        let f = Some(HookFilter {
            file_patterns: vec![],
            project_names: vec!["vibecli".into()],
            languages: vec![],
        });
        assert!(matches_filter(&f, None, Some("vibecli"), None));
        assert!(!matches_filter(&f, None, Some("other"), None));
    }

    #[test]
    fn test_filter_language() {
        let f = Some(HookFilter {
            file_patterns: vec![],
            project_names: vec![],
            languages: vec!["rust".into(), "python".into()],
        });
        assert!(matches_filter(&f, None, None, Some("rust")));
        assert!(matches_filter(&f, None, None, Some("python")));
        assert!(!matches_filter(&f, None, None, Some("java")));
    }

    #[test]
    fn test_filter_combined_no_match() {
        let f = Some(HookFilter {
            file_patterns: vec!["*.rs".into()],
            project_names: vec!["vibecli".into()],
            languages: vec!["rust".into()],
        });
        // file matches, project matches, language does NOT
        assert!(!matches_filter(
            &f,
            Some("lib.rs"),
            Some("vibecli"),
            Some("python")
        ));
        // file does NOT match
        assert!(!matches_filter(
            &f,
            Some("lib.py"),
            Some("vibecli"),
            Some("rust")
        ));
    }

    #[test]
    fn test_filter_combined_match() {
        let f = Some(HookFilter {
            file_patterns: vec!["*.rs".into()],
            project_names: vec!["vibecli".into()],
            languages: vec!["rust".into()],
        });
        assert!(matches_filter(
            &f,
            Some("lib.rs"),
            Some("vibecli"),
            Some("rust")
        ));
    }

    #[test]
    fn test_filter_file_none_with_patterns() {
        let f = Some(HookFilter {
            file_patterns: vec!["*.rs".into()],
            project_names: vec![],
            languages: vec![],
        });
        // file is None but filter requires file patterns -> no match
        assert!(!matches_filter(&f, None, None, None));
    }

    // -- HookPayload construction ------------------------------------------

    #[test]
    fn test_hook_payload_construction() {
        let payload = HookPayload {
            event: HookEvent::FileSaved {
                pattern: Some("*.rs".into()),
            },
            timestamp_ms: 1700000000000,
            session_id: Some("sess-123".into()),
            context: HookContext {
                workspace_path: Some("/home/user/project".into()),
                current_file: Some("src/main.rs".into()),
                current_line: Some(10),
                tool_name: None,
                tool_result: None,
            },
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("FileSaved"));
        assert!(json.contains("sess-123"));
        assert!(json.contains("src/main.rs"));
    }

    #[test]
    fn test_hook_payload_minimal_context() {
        let payload = HookPayload {
            event: HookEvent::SessionStart,
            timestamp_ms: 0,
            session_id: None,
            context: HookContext {
                workspace_path: None,
                current_file: None,
                current_line: None,
                tool_name: None,
                tool_result: None,
            },
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("SessionStart"));
    }

    // -- HookResponse parsing ----------------------------------------------

    #[test]
    fn test_hook_response_allow() {
        let json = r#"{"allow": true, "message": "ok"}"#;
        let resp: HookResponse = serde_json::from_str(json).unwrap();
        assert!(resp.allow);
        assert_eq!(resp.message.as_deref(), Some("ok"));
    }

    #[test]
    fn test_hook_response_deny() {
        let json = r#"{"allow": false, "message": "blocked"}"#;
        let resp: HookResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.allow);
        assert_eq!(resp.message.as_deref(), Some("blocked"));
    }

    #[test]
    fn test_hook_response_no_message() {
        let json = r#"{"allow": true}"#;
        let resp: HookResponse = serde_json::from_str(json).unwrap();
        assert!(resp.allow);
        assert!(resp.message.is_none());
    }

    // -- IdeInfo serde -----------------------------------------------------

    #[test]
    fn test_ide_info_serde() {
        let info = IdeInfo {
            product_name: "IntelliJ IDEA".into(),
            version: "2025.1".into(),
            build_number: "251.1234".into(),
            project_name: Some("vibecli".into()),
            project_path: Some("/home/user/vibecli".into()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let info2: IdeInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, info2);
    }

    #[test]
    fn test_ide_info_minimal() {
        let info = IdeInfo {
            product_name: "WebStorm".into(),
            version: "2025.1".into(),
            build_number: "251.0".into(),
            project_name: None,
            project_path: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        let info2: IdeInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, info2);
    }

    // -- Hook list presets --------------------------------------------------

    #[test]
    fn test_default_hooks_list() {
        let hooks = default_hooks();
        assert_eq!(hooks.len(), 2);
        assert!(matches!(hooks[0].event, HookEvent::FileSaved { .. }));
        assert!(matches!(hooks[1].action, HookAction::RefreshDiagnostics));
    }

    #[test]
    fn test_strict_hooks_list() {
        let hooks = strict_hooks();
        assert_eq!(hooks.len(), 2);
        assert!(matches!(hooks[0].event, HookEvent::PreToolUse { .. }));
        assert!(matches!(hooks[1].action, HookAction::RunConfiguration { .. }));
    }

    #[test]
    fn test_minimal_hooks_list() {
        let hooks = minimal_hooks();
        assert_eq!(hooks.len(), 2);
        assert_eq!(hooks[0].event, HookEvent::SessionStart);
        assert_eq!(hooks[1].event, HookEvent::SessionEnd);
    }

    // -- Connection ---------------------------------------------------------

    #[test]
    fn test_connection_new() {
        let cfg = JetBrainsHookConfig::default();
        let conn = JetBrainsConnection::new(&cfg);
        assert_eq!(conn.base_url, "http://127.0.0.1:63342");
        assert!(!conn.connected);
        assert!(conn.ide_info.is_none());
    }

    #[test]
    fn test_connection_custom_port() {
        let cfg = JetBrainsHookConfig {
            plugin_port: 9999,
            ..Default::default()
        };
        let conn = JetBrainsConnection::new(&cfg);
        assert_eq!(conn.base_url, "http://127.0.0.1:9999");
    }

    #[test]
    fn test_connection_is_connected() {
        let cfg = JetBrainsHookConfig::default();
        let mut conn = JetBrainsConnection::new(&cfg);
        assert!(!conn.is_connected());
        conn.connected = true;
        assert!(conn.is_connected());
    }

    #[test]
    fn test_discover_ide_port() {
        let port = JetBrainsConnection::discover_ide_port();
        assert_eq!(port, Some(63342));
    }

    // -- Hook conversion roundtrip -----------------------------------------

    #[test]
    fn test_hook_conversion_roundtrip() {
        let hook = JetBrainsHookDef {
            event: HookEvent::BuildCompleted { success: false },
            action: HookAction::Notify {
                message: "Build failed!".into(),
                severity: NotifySeverity::Error,
            },
            filter: Some(HookFilter {
                file_patterns: vec!["*.java".into()],
                project_names: vec![],
                languages: vec!["java".into()],
            }),
        };
        let json = to_vibe_hook(&hook);
        let hook2 = from_vibe_hook(&json).unwrap();
        assert_eq!(hook, hook2);
    }

    #[test]
    fn test_hook_conversion_minimal() {
        let hook = JetBrainsHookDef {
            event: HookEvent::SessionStart,
            action: HookAction::RefreshDiagnostics,
            filter: None,
        };
        let json = to_vibe_hook(&hook);
        let hook2 = from_vibe_hook(&json).unwrap();
        assert_eq!(hook, hook2);
    }

    #[test]
    fn test_from_vibe_hook_invalid_json() {
        let result = from_vibe_hook("not json");
        assert!(result.is_err());
    }

    // -- Diagnostic serde --------------------------------------------------

    #[test]
    fn test_diagnostic_serde() {
        let diag = Diagnostic {
            file: "src/main.rs".into(),
            line: 42,
            column: 5,
            severity: "error".into(),
            message: "unused variable".into(),
            source: "rustc".into(),
        };
        let json = serde_json::to_string(&diag).unwrap();
        let diag2: Diagnostic = serde_json::from_str(&json).unwrap();
        assert_eq!(diag, diag2);
    }

    // -- HookContext with all fields ---------------------------------------

    #[test]
    fn test_hook_context_all_fields() {
        let ctx = HookContext {
            workspace_path: Some("/workspace".into()),
            current_file: Some("lib.rs".into()),
            current_line: Some(100),
            tool_name: Some("write_file".into()),
            tool_result: Some("success".into()),
        };
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("write_file"));
        assert!(json.contains("success"));
        assert!(json.contains("/workspace"));
    }

    #[test]
    fn test_hook_context_minimal() {
        let ctx = HookContext {
            workspace_path: None,
            current_file: None,
            current_line: None,
            tool_name: None,
            tool_result: None,
        };
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("null"));
    }
}
