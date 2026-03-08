//! Plugin SDK for VibeCody — types and utilities for 3rd-party plugin development.
//!
//! # Plugin Types
//! - **Connector**: Integrates external services (Jira, Linear, Notion, etc.)
//! - **Adapter**: Adds new AI providers, gateways, or container runtimes
//! - **Optimizer**: Code analysis, linting, formatting, refactoring tools
//! - **Theme**: UI themes and color schemes
//! - **SkillPack**: Bundles of skill markdown files
//! - **Workflow**: Pre-built agent workflow templates

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Plugin categories for marketplace organization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    Connector,
    Adapter,
    Optimizer,
    Theme,
    SkillPack,
    Workflow,
    Extension,  // WASM extension
}

/// Supported hook events plugins can subscribe to
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginEvent {
    PreToolUse,
    PostToolUse,
    PreCompletion,
    PostCompletion,
    OnFileOpen,
    OnFileSave,
    OnSessionStart,
    OnSessionEnd,
    OnAgentStart,
    OnAgentEnd,
    OnError,
    Custom(String),
}

/// Plugin capability declarations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    FileRead,
    FileWrite,
    NetworkAccess,
    ProcessExec,
    EnvRead,
    Notification,
    Clipboard,
    GitAccess,
    DatabaseAccess,
    HttpServer,
    WebSocket,
}

impl PluginCapability {
    pub fn is_dangerous(&self) -> bool {
        matches!(self, Self::FileWrite | Self::NetworkAccess | Self::ProcessExec | Self::DatabaseAccess)
    }

    pub fn safe_defaults() -> Vec<Self> {
        vec![Self::FileRead, Self::Notification, Self::EnvRead]
    }
}

/// Enhanced plugin manifest (plugin.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifestV2 {
    pub name: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub kind: PluginKind,
    pub capabilities: Vec<PluginCapability>,
    pub min_vibecli_version: Option<String>,
    pub max_vibecli_version: Option<String>,
    pub dependencies: Vec<PluginDependency>,
    pub hooks: Vec<PluginHookDef>,
    pub commands: Vec<PluginCommandDef>,
    pub settings: Vec<PluginSettingDef>,
    pub keywords: Vec<String>,
    pub icon: Option<String>,
    pub screenshots: Vec<String>,
    pub platforms: Vec<String>,  // "macos", "linux", "windows", "all"
}

/// Plugin dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub name: String,
    pub version: String,  // semver range: ">=1.0.0, <2.0.0"
    pub optional: bool,
}

/// Hook definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHookDef {
    pub event: PluginEvent,
    pub handler: String,  // relative path to script or WASM export
    pub filter: Option<HashMap<String, String>>,  // e.g., {"tool": "bash", "file_ext": ".rs"}
    pub priority: i32,     // lower runs first, default 100
    pub async_exec: bool,
}

/// Command definition (REPL commands the plugin adds)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommandDef {
    pub name: String,
    pub description: String,
    pub handler: String,  // relative path to script
    pub args: Vec<PluginArgDef>,
}

/// Command argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginArgDef {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

/// User-configurable setting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSettingDef {
    pub key: String,
    pub description: String,
    pub setting_type: SettingType,
    pub default: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingType {
    String,
    Number,
    Boolean,
    Secret,    // stored encrypted
    FilePath,
    Enum(Vec<std::string::String>),
}

/// Plugin scaffold templates
pub struct PluginScaffold;

impl PluginScaffold {
    /// Generate a new plugin project directory
    pub fn create(name: &str, kind: PluginKind, dir: &Path) -> anyhow::Result<PathBuf> {
        let plugin_dir = dir.join(name);
        std::fs::create_dir_all(&plugin_dir)?;
        std::fs::create_dir_all(plugin_dir.join("skills"))?;
        std::fs::create_dir_all(plugin_dir.join("hooks"))?;
        std::fs::create_dir_all(plugin_dir.join("commands"))?;

        // Write plugin.toml manifest
        let manifest = Self::default_manifest(name, &kind);
        let toml_str = toml::to_string_pretty(&manifest)
            .unwrap_or_else(|_| std::string::String::from("# plugin.toml\n"));
        std::fs::write(plugin_dir.join("plugin.toml"), toml_str)?;

        // Write README
        let readme = format!(
            "# {name}\n\nA VibeCody {kind:?} plugin.\n\n## Installation\n\n```bash\nvibecli plugin install {name}\n```\n\n## Configuration\n\nAdd to `~/.vibecli/config.toml`:\n\n```toml\n[plugins.{name}]\nenabled = true\n```\n\n## Development\n\n```bash\nvibecli plugin dev --watch\n```\n"
        );
        std::fs::write(plugin_dir.join("README.md"), readme)?;

        // Write example hook
        let hook = "#!/bin/bash\n# Example hook — receives JSON on stdin, outputs JSON on stdout\n# Exit 0 = allow, exit 2 = block\nread -r INPUT\necho '{\"action\": \"allow\"}'\nexit 0\n";
        std::fs::write(plugin_dir.join("hooks/example.sh"), hook)?;

        // Write .gitignore
        std::fs::write(plugin_dir.join(".gitignore"), "*.wasm\n.vibecli-dev/\nnode_modules/\ntarget/\n")?;

        // Kind-specific files
        match kind {
            PluginKind::Connector => {
                let skill = format!("---\ntriggers: [\"{name}\"]\ntools_allowed: [\"read_file\", \"write_file\", \"bash\"]\ncategory: connector\n---\n\n# {name} Connector\n\nWhen integrating with {name}:\n\n1. Authenticate using API key from plugin settings\n2. Use the provided commands to interact with the service\n");
                std::fs::write(plugin_dir.join("skills/connector.md"), skill)?;
            }
            PluginKind::SkillPack => {
                let skill = "---\ntriggers: [\"example\"]\ntools_allowed: [\"read_file\"]\ncategory: custom\n---\n\n# Example Skill\n\nWhen working with this topic:\n\n1. First best practice\n2. Second best practice\n";
                std::fs::write(plugin_dir.join("skills/example.md"), skill)?;
            }
            _ => {}
        }

        Ok(plugin_dir)
    }

    fn default_manifest(name: &str, kind: &PluginKind) -> PluginManifestV2 {
        PluginManifestV2 {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            display_name: name.replace('-', " ").to_string(),
            description: format!("A VibeCody {kind:?} plugin"),
            author: "".to_string(),
            license: "MIT".to_string(),
            repository: None,
            homepage: None,
            kind: kind.clone(),
            capabilities: PluginCapability::safe_defaults(),
            min_vibecli_version: Some("0.1.0".to_string()),
            max_vibecli_version: None,
            dependencies: vec![],
            hooks: vec![],
            commands: vec![],
            settings: vec![],
            keywords: vec![],
            icon: None,
            screenshots: vec![],
            platforms: vec!["all".to_string()],
        }
    }
}

/// Validate a plugin manifest
pub fn validate_manifest(manifest: &PluginManifestV2) -> Vec<String> {
    let mut errors = vec![];

    if manifest.name.is_empty() {
        errors.push("Plugin name is required".to_string());
    }
    if !manifest.name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        errors.push("Plugin name must contain only alphanumeric, hyphens, underscores".to_string());
    }
    if manifest.version.is_empty() {
        errors.push("Version is required".to_string());
    }
    // Validate semver
    let parts: Vec<&str> = manifest.version.split('.').collect();
    if parts.len() != 3 || !parts.iter().all(|p| p.parse::<u32>().is_ok()) {
        errors.push("Version must be valid semver (major.minor.patch)".to_string());
    }
    if manifest.description.is_empty() {
        errors.push("Description is required".to_string());
    }
    if manifest.author.is_empty() {
        errors.push("Author is required".to_string());
    }

    // Check for dangerous capabilities without justification
    let dangerous: Vec<_> = manifest.capabilities.iter()
        .filter(|c| c.is_dangerous())
        .collect();
    if !dangerous.is_empty() && manifest.description.len() < 20 {
        errors.push("Plugins requesting dangerous capabilities need detailed descriptions".to_string());
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_kind_serialization() {
        let kind = PluginKind::Connector;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"connector\"");
        let back: PluginKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, PluginKind::Connector);
    }

    #[test]
    fn test_capability_dangerous() {
        assert!(PluginCapability::FileWrite.is_dangerous());
        assert!(PluginCapability::NetworkAccess.is_dangerous());
        assert!(PluginCapability::ProcessExec.is_dangerous());
        assert!(!PluginCapability::FileRead.is_dangerous());
        assert!(!PluginCapability::Notification.is_dangerous());
    }

    #[test]
    fn test_safe_defaults() {
        let defaults = PluginCapability::safe_defaults();
        assert_eq!(defaults.len(), 3);
        assert!(!defaults.iter().any(|c| c.is_dangerous()));
    }

    #[test]
    fn test_validate_manifest_empty_name() {
        let mut m = PluginManifestV2 {
            name: "".to_string(),
            version: "1.0.0".to_string(),
            display_name: "Test".to_string(),
            description: "A test plugin".to_string(),
            author: "Test Author".to_string(),
            license: "MIT".to_string(),
            repository: None,
            homepage: None,
            kind: PluginKind::Connector,
            capabilities: vec![],
            min_vibecli_version: None,
            max_vibecli_version: None,
            dependencies: vec![],
            hooks: vec![],
            commands: vec![],
            settings: vec![],
            keywords: vec![],
            icon: None,
            screenshots: vec![],
            platforms: vec![],
        };
        let errors = validate_manifest(&m);
        assert!(errors.iter().any(|e| e.contains("name is required")));

        m.name = "valid-name".to_string();
        m.version = "bad".to_string();
        let errors = validate_manifest(&m);
        assert!(errors.iter().any(|e| e.contains("semver")));
    }

    #[test]
    fn test_validate_manifest_valid() {
        let m = PluginManifestV2 {
            name: "my-plugin".to_string(),
            version: "1.0.0".to_string(),
            display_name: "My Plugin".to_string(),
            description: "A test plugin for VibeCody".to_string(),
            author: "Test Author".to_string(),
            license: "MIT".to_string(),
            repository: None,
            homepage: None,
            kind: PluginKind::SkillPack,
            capabilities: PluginCapability::safe_defaults(),
            min_vibecli_version: None,
            max_vibecli_version: None,
            dependencies: vec![],
            hooks: vec![],
            commands: vec![],
            settings: vec![],
            keywords: vec!["test".to_string()],
            icon: None,
            screenshots: vec![],
            platforms: vec!["all".to_string()],
        };
        let errors = validate_manifest(&m);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_plugin_event_custom() {
        let event = PluginEvent::Custom("my_event".to_string());
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("my_event"));
    }

    #[test]
    fn test_setting_type_enum() {
        let t = SettingType::Enum(vec!["a".into(), "b".into()]);
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("enum"));
    }

    #[test]
    fn test_plugin_dependency() {
        let dep = PluginDependency {
            name: "some-plugin".to_string(),
            version: ">=1.0.0".to_string(),
            optional: false,
        };
        let json = serde_json::to_string(&dep).unwrap();
        assert!(json.contains("some-plugin"));
        assert!(json.contains(">=1.0.0"));
    }
}
