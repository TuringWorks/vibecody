//! Deeper VS Code extension API compatibility layer.
//!
//! Provides manifest parsing, compatibility analysis, API shim generation,
//! and scoring for VS Code extensions running under VibeCody.

#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub engines: EngineRequirement,
    pub contributes: ExtensionContributions,
    #[serde(default, alias = "activationEvents")]
    pub activation_events: Vec<String>,
    pub main: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineRequirement {
    pub vscode: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtensionContributions {
    pub commands: Option<Vec<CommandContrib>>,
    pub keybindings: Option<Vec<KeybindingContrib>>,
    pub configuration: Option<Vec<ConfigContrib>>,
    pub languages: Option<Vec<LanguageContrib>>,
    pub grammars: Option<Vec<GrammarContrib>>,
    pub themes: Option<Vec<ThemeContrib>>,
    pub snippets: Option<Vec<SnippetContrib>>,
    pub views: Option<HashMap<String, Vec<ViewContrib>>>,
    pub debuggers: Option<Vec<DebuggerContrib>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContrib {
    pub command: String,
    pub title: String,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingContrib {
    pub command: String,
    pub key: String,
    pub mac: Option<String>,
    pub when: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigContrib {
    pub title: String,
    pub properties: HashMap<String, ConfigProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigProperty {
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(rename = "default")]
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageContrib {
    pub id: String,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarContrib {
    pub language: String,
    #[serde(alias = "scopeName")]
    pub scope_name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeContrib {
    pub label: String,
    #[serde(alias = "uiTheme")]
    pub ui_theme: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetContrib {
    pub language: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewContrib {
    pub id: String,
    pub name: String,
    pub when: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebuggerContrib {
    #[serde(rename = "type")]
    pub type_field: String,
    pub label: String,
    #[serde(default)]
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SupportedApi {
    WorkspaceGetConfig,
    WorkspaceFindFiles,
    WorkspaceOpenTextDocument,
    WorkspaceApplyEdit,
    WorkspaceGetWorkspaceFolders,
    WorkspaceOnDidChangeConfig,
    WorkspaceCreateFileSystemWatcher,
    WindowShowMessage,
    WindowShowQuickPick,
    WindowShowInputBox,
    WindowCreateTerminal,
    WindowCreateOutputChannel,
    WindowShowTextDocument,
    WindowActiveTextEditor,
    WindowOnDidChangeActiveEditor,
    CommandsRegister,
    CommandsExecute,
    CommandsGetCommands,
    LanguagesRegisterCompletion,
    LanguagesRegisterHover,
    LanguagesRegisterDefinition,
    LanguagesRegisterCodeActions,
    LanguagesRegisterFormatting,
    LanguagesGetDiagnostics,
    LanguagesCreateDiagnosticCollection,
    ExtensionsGetExtension,
    ExtensionsAll,
    EnvAppName,
    EnvLanguage,
    EnvMachineId,
    EnvUriScheme,
    DebugStartDebugging,
    DebugRegisterDebugAdapterProvider,
    TasksRegisterTaskProvider,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompatibilityReport {
    pub extension_name: String,
    pub supported_features: Vec<String>,
    pub unsupported_features: Vec<String>,
    pub compatibility_score: f64,
    pub can_install: bool,
    pub migration_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PartialFeature {
    pub name: String,
    pub supported_part: String,
    pub unsupported_part: String,
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

/// Parse a VS Code extension manifest (package.json) from JSON.
pub fn parse_manifest(json: &str) -> Result<ExtensionManifest> {
    serde_json::from_str(json).context("Failed to parse extension manifest")
}

/// Compute a compatibility score for a contribution category.
pub fn compatibility_score_for_category(category: &str) -> f64 {
    match category.to_lowercase().as_str() {
        "themes" => 1.0,
        "snippets" => 1.0,
        "languages" => 0.9,
        "commands" => 0.9,
        "keybindings" => 0.9,
        "grammars" => 0.85,
        "configuration" => 0.85,
        "debuggers" => 0.5,
        "views" => 0.3,
        _ => 0.0,
    }
}

/// Analyze an extension manifest and produce a compatibility report.
pub fn check_compatibility(manifest: &ExtensionManifest) -> CompatibilityReport {
    let mut supported = Vec::new();
    let mut unsupported = Vec::new();
    let mut notes = Vec::new();
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;

    let c = &manifest.contributes;

    // Helper: evaluate a contribution bucket.
    macro_rules! eval {
        ($field:expr, $name:expr) => {
            if let Some(items) = &$field {
                if !items.is_empty() {
                    let score = compatibility_score_for_category($name);
                    let count = items.len() as f64;
                    weighted_sum += score * count;
                    weight_total += count;
                    if score >= 0.8 {
                        supported.push(format!("{} ({} items)", $name, items.len()));
                    } else if score >= 0.5 {
                        supported.push(format!("{} (partial, {} items)", $name, items.len()));
                        notes.push(format!("{}: partial support — some features may not work", $name));
                    } else {
                        unsupported.push(format!("{} ({} items)", $name, items.len()));
                        notes.push(format!("{}: limited support in VibeCody", $name));
                    }
                }
            }
        };
    }

    eval!(c.commands, "commands");
    eval!(c.keybindings, "keybindings");
    eval!(c.configuration, "configuration");
    eval!(c.languages, "languages");
    eval!(c.grammars, "grammars");
    eval!(c.themes, "themes");
    eval!(c.snippets, "snippets");
    eval!(c.debuggers, "debuggers");

    // Views use HashMap, handle separately.
    if let Some(views_map) = &c.views {
        let total_views: usize = views_map.values().map(|v| v.len()).sum();
        if total_views > 0 {
            let score = compatibility_score_for_category("views");
            weighted_sum += score * total_views as f64;
            weight_total += total_views as f64;
            unsupported.push(format!("views ({} items)", total_views));
            notes.push("views: limited support — custom webviews are not fully compatible".into());
        }
    }

    // If the extension has a main entry point it relies on the Node extension host.
    if manifest.main.is_some() {
        notes.push(
            "Extension has a main entry point; programmatic VS Code API usage may be limited."
                .into(),
        );
    }

    let score = if weight_total > 0.0 {
        (weighted_sum / weight_total).clamp(0.0, 1.0)
    } else {
        // No contributions — purely declarative / empty.
        1.0
    };

    let can_install = score >= 0.3;

    CompatibilityReport {
        extension_name: format!("{}.{}", manifest.publisher, manifest.name),
        supported_features: supported,
        unsupported_features: unsupported,
        compatibility_score: score,
        can_install,
        migration_notes: notes,
    }
}

/// Return all supported APIs with human-readable descriptions.
pub fn supported_apis() -> Vec<(SupportedApi, &'static str)> {
    vec![
        (SupportedApi::WorkspaceGetConfig, "Read workspace configuration values"),
        (SupportedApi::WorkspaceFindFiles, "Glob-search files in the workspace"),
        (SupportedApi::WorkspaceOpenTextDocument, "Open a text document by URI"),
        (SupportedApi::WorkspaceApplyEdit, "Apply a workspace edit (multi-file)"),
        (SupportedApi::WorkspaceGetWorkspaceFolders, "List workspace root folders"),
        (SupportedApi::WorkspaceOnDidChangeConfig, "Listen for configuration changes"),
        (SupportedApi::WorkspaceCreateFileSystemWatcher, "Watch filesystem for changes"),
        (SupportedApi::WindowShowMessage, "Show an information/warning/error message"),
        (SupportedApi::WindowShowQuickPick, "Show a quick-pick selection list"),
        (SupportedApi::WindowShowInputBox, "Show a text input box"),
        (SupportedApi::WindowCreateTerminal, "Create an integrated terminal instance"),
        (SupportedApi::WindowCreateOutputChannel, "Create an output channel for logging"),
        (SupportedApi::WindowShowTextDocument, "Show a text document in the editor"),
        (SupportedApi::WindowActiveTextEditor, "Get the currently active text editor"),
        (SupportedApi::WindowOnDidChangeActiveEditor, "Listen for active editor changes"),
        (SupportedApi::CommandsRegister, "Register a command handler"),
        (SupportedApi::CommandsExecute, "Execute a registered command"),
        (SupportedApi::CommandsGetCommands, "List all registered commands"),
        (SupportedApi::LanguagesRegisterCompletion, "Register a completion provider"),
        (SupportedApi::LanguagesRegisterHover, "Register a hover provider"),
        (SupportedApi::LanguagesRegisterDefinition, "Register a go-to-definition provider"),
        (SupportedApi::LanguagesRegisterCodeActions, "Register a code action provider"),
        (SupportedApi::LanguagesRegisterFormatting, "Register a document formatting provider"),
        (SupportedApi::LanguagesGetDiagnostics, "Get diagnostics for a document"),
        (SupportedApi::LanguagesCreateDiagnosticCollection, "Create a diagnostic collection"),
        (SupportedApi::ExtensionsGetExtension, "Get an extension by its identifier"),
        (SupportedApi::ExtensionsAll, "List all installed extensions"),
        (SupportedApi::EnvAppName, "Get the application name"),
        (SupportedApi::EnvLanguage, "Get the display language (locale)"),
        (SupportedApi::EnvMachineId, "Get a unique machine identifier"),
        (SupportedApi::EnvUriScheme, "Get the URI scheme for the editor"),
        (SupportedApi::DebugStartDebugging, "Start a debug session"),
        (SupportedApi::DebugRegisterDebugAdapterProvider, "Register a debug adapter provider"),
        (SupportedApi::TasksRegisterTaskProvider, "Register a task provider"),
    ]
}

/// Generate a JavaScript shim for a given API entry point.
pub fn generate_shim(api: &SupportedApi) -> String {
    match api {
        SupportedApi::WorkspaceGetConfig => {
            "const vscode = require('vscode');\n\
             module.exports.getConfiguration = function(section) {\n  \
               return vscode.workspace.getConfiguration(section);\n\
             };\n"
                .into()
        }
        SupportedApi::WorkspaceFindFiles => {
            "const vscode = require('vscode');\n\
             module.exports.findFiles = async function(include, exclude, maxResults) {\n  \
               return await vscode.workspace.findFiles(include, exclude, maxResults);\n\
             };\n"
                .into()
        }
        SupportedApi::WorkspaceOpenTextDocument => {
            "const vscode = require('vscode');\n\
             module.exports.openTextDocument = async function(uri) {\n  \
               return await vscode.workspace.openTextDocument(uri);\n\
             };\n"
                .into()
        }
        SupportedApi::WorkspaceApplyEdit => {
            "const vscode = require('vscode');\n\
             module.exports.applyEdit = async function(edit) {\n  \
               return await vscode.workspace.applyEdit(edit);\n\
             };\n"
                .into()
        }
        SupportedApi::WorkspaceGetWorkspaceFolders => {
            "const vscode = require('vscode');\n\
             module.exports.getWorkspaceFolders = function() {\n  \
               return vscode.workspace.workspaceFolders || [];\n\
             };\n"
                .into()
        }
        SupportedApi::WorkspaceOnDidChangeConfig => {
            "const vscode = require('vscode');\n\
             module.exports.onDidChangeConfiguration = function(listener) {\n  \
               return vscode.workspace.onDidChangeConfiguration(listener);\n\
             };\n"
                .into()
        }
        SupportedApi::WorkspaceCreateFileSystemWatcher => {
            "const vscode = require('vscode');\n\
             module.exports.createFileSystemWatcher = function(globPattern) {\n  \
               return vscode.workspace.createFileSystemWatcher(globPattern);\n\
             };\n"
                .into()
        }
        SupportedApi::WindowShowMessage => {
            "const vscode = require('vscode');\n\
             module.exports.showInformationMessage = function(msg, ...items) {\n  \
               return vscode.window.showInformationMessage(msg, ...items);\n\
             };\n"
                .into()
        }
        SupportedApi::WindowShowQuickPick => {
            "const vscode = require('vscode');\n\
             module.exports.showQuickPick = function(items, options) {\n  \
               return vscode.window.showQuickPick(items, options);\n\
             };\n"
                .into()
        }
        SupportedApi::WindowShowInputBox => {
            "const vscode = require('vscode');\n\
             module.exports.showInputBox = function(options) {\n  \
               return vscode.window.showInputBox(options);\n\
             };\n"
                .into()
        }
        SupportedApi::WindowCreateTerminal => {
            "const vscode = require('vscode');\n\
             module.exports.createTerminal = function(name) {\n  \
               return vscode.window.createTerminal(name);\n\
             };\n"
                .into()
        }
        SupportedApi::WindowCreateOutputChannel => {
            "const vscode = require('vscode');\n\
             module.exports.createOutputChannel = function(name) {\n  \
               return vscode.window.createOutputChannel(name);\n\
             };\n"
                .into()
        }
        SupportedApi::WindowShowTextDocument => {
            "const vscode = require('vscode');\n\
             module.exports.showTextDocument = async function(doc, options) {\n  \
               return await vscode.window.showTextDocument(doc, options);\n\
             };\n"
                .into()
        }
        SupportedApi::WindowActiveTextEditor => {
            "const vscode = require('vscode');\n\
             module.exports.activeTextEditor = function() {\n  \
               return vscode.window.activeTextEditor;\n\
             };\n"
                .into()
        }
        SupportedApi::WindowOnDidChangeActiveEditor => {
            "const vscode = require('vscode');\n\
             module.exports.onDidChangeActiveTextEditor = function(listener) {\n  \
               return vscode.window.onDidChangeActiveTextEditor(listener);\n\
             };\n"
                .into()
        }
        SupportedApi::CommandsRegister => {
            "const vscode = require('vscode');\n\
             module.exports.registerCommand = function(command, callback) {\n  \
               return vscode.commands.registerCommand(command, callback);\n\
             };\n"
                .into()
        }
        SupportedApi::CommandsExecute => {
            "const vscode = require('vscode');\n\
             module.exports.executeCommand = async function(command, ...args) {\n  \
               return await vscode.commands.executeCommand(command, ...args);\n\
             };\n"
                .into()
        }
        SupportedApi::CommandsGetCommands => {
            "const vscode = require('vscode');\n\
             module.exports.getCommands = async function(filterInternal) {\n  \
               return await vscode.commands.getCommands(filterInternal);\n\
             };\n"
                .into()
        }
        SupportedApi::LanguagesRegisterCompletion => {
            "const vscode = require('vscode');\n\
             module.exports.registerCompletionItemProvider = function(selector, provider, ...triggers) {\n  \
               return vscode.languages.registerCompletionItemProvider(selector, provider, ...triggers);\n\
             };\n"
                .into()
        }
        SupportedApi::LanguagesRegisterHover => {
            "const vscode = require('vscode');\n\
             module.exports.registerHoverProvider = function(selector, provider) {\n  \
               return vscode.languages.registerHoverProvider(selector, provider);\n\
             };\n"
                .into()
        }
        SupportedApi::LanguagesRegisterDefinition => {
            "const vscode = require('vscode');\n\
             module.exports.registerDefinitionProvider = function(selector, provider) {\n  \
               return vscode.languages.registerDefinitionProvider(selector, provider);\n\
             };\n"
                .into()
        }
        SupportedApi::LanguagesRegisterCodeActions => {
            "const vscode = require('vscode');\n\
             module.exports.registerCodeActionsProvider = function(selector, provider) {\n  \
               return vscode.languages.registerCodeActionsProvider(selector, provider);\n\
             };\n"
                .into()
        }
        SupportedApi::LanguagesRegisterFormatting => {
            "const vscode = require('vscode');\n\
             module.exports.registerDocumentFormattingEditProvider = function(selector, provider) {\n  \
               return vscode.languages.registerDocumentFormattingEditProvider(selector, provider);\n\
             };\n"
                .into()
        }
        SupportedApi::LanguagesGetDiagnostics => {
            "const vscode = require('vscode');\n\
             module.exports.getDiagnostics = function(resource) {\n  \
               return vscode.languages.getDiagnostics(resource);\n\
             };\n"
                .into()
        }
        SupportedApi::LanguagesCreateDiagnosticCollection => {
            "const vscode = require('vscode');\n\
             module.exports.createDiagnosticCollection = function(name) {\n  \
               return vscode.languages.createDiagnosticCollection(name);\n\
             };\n"
                .into()
        }
        SupportedApi::ExtensionsGetExtension => {
            "const vscode = require('vscode');\n\
             module.exports.getExtension = function(extensionId) {\n  \
               return vscode.extensions.getExtension(extensionId);\n\
             };\n"
                .into()
        }
        SupportedApi::ExtensionsAll => {
            "const vscode = require('vscode');\n\
             module.exports.allExtensions = function() {\n  \
               return vscode.extensions.all;\n\
             };\n"
                .into()
        }
        SupportedApi::EnvAppName => {
            "const vscode = require('vscode');\n\
             module.exports.appName = vscode.env.appName;\n"
                .into()
        }
        SupportedApi::EnvLanguage => {
            "const vscode = require('vscode');\n\
             module.exports.language = vscode.env.language;\n"
                .into()
        }
        SupportedApi::EnvMachineId => {
            "const vscode = require('vscode');\n\
             module.exports.machineId = vscode.env.machineId;\n"
                .into()
        }
        SupportedApi::EnvUriScheme => {
            "const vscode = require('vscode');\n\
             module.exports.uriScheme = vscode.env.uriScheme;\n"
                .into()
        }
        SupportedApi::DebugStartDebugging => {
            "const vscode = require('vscode');\n\
             module.exports.startDebugging = async function(folder, config) {\n  \
               return await vscode.debug.startDebugging(folder, config);\n\
             };\n"
                .into()
        }
        SupportedApi::DebugRegisterDebugAdapterProvider => {
            "const vscode = require('vscode');\n\
             module.exports.registerDebugAdapterDescriptorFactory = function(type, factory) {\n  \
               return vscode.debug.registerDebugAdapterDescriptorFactory(type, factory);\n\
             };\n"
                .into()
        }
        SupportedApi::TasksRegisterTaskProvider => {
            "const vscode = require('vscode');\n\
             module.exports.registerTaskProvider = function(type, provider) {\n  \
               return vscode.tasks.registerTaskProvider(type, provider);\n\
             };\n"
                .into()
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn full_manifest_json() -> &'static str {
        r#"{
            "name": "my-ext",
            "version": "1.2.3",
            "publisher": "testpub",
            "engines": { "vscode": "^1.70.0" },
            "activationEvents": ["onLanguage:rust"],
            "main": "./out/extension.js",
            "categories": ["Programming Languages"],
            "contributes": {
                "commands": [
                    { "command": "myext.hello", "title": "Hello", "category": "MyExt" }
                ],
                "keybindings": [
                    { "command": "myext.hello", "key": "ctrl+shift+h", "mac": "cmd+shift+h", "when": "editorTextFocus" }
                ],
                "configuration": [
                    {
                        "title": "My Extension",
                        "properties": {
                            "myext.enabled": {
                                "type": "boolean",
                                "default": true,
                                "description": "Enable extension"
                            }
                        }
                    }
                ],
                "languages": [
                    { "id": "mylang", "extensions": [".ml"], "aliases": ["MyLang"] }
                ],
                "grammars": [
                    { "language": "mylang", "scopeName": "source.mylang", "path": "./syntaxes/mylang.tmLanguage.json" }
                ],
                "themes": [
                    { "label": "My Theme", "uiTheme": "vs-dark", "path": "./themes/my-theme.json" }
                ],
                "snippets": [
                    { "language": "mylang", "path": "./snippets/mylang.json" }
                ],
                "views": {
                    "explorer": [
                        { "id": "myView", "name": "My View", "when": "workspaceFolderCount > 0" }
                    ]
                },
                "debuggers": [
                    { "type": "mydbg", "label": "My Debugger", "languages": ["mylang"] }
                ]
            }
        }"#
    }

    fn minimal_manifest_json() -> &'static str {
        r#"{
            "name": "mini",
            "version": "0.0.1",
            "publisher": "nobody",
            "engines": { "vscode": "^1.60.0" },
            "contributes": {}
        }"#
    }

    fn theme_only_manifest_json() -> &'static str {
        r#"{
            "name": "pretty-theme",
            "version": "2.0.0",
            "publisher": "themer",
            "engines": { "vscode": "^1.50.0" },
            "categories": ["Themes"],
            "contributes": {
                "themes": [
                    { "label": "Pretty Dark", "uiTheme": "vs-dark", "path": "./themes/dark.json" },
                    { "label": "Pretty Light", "uiTheme": "vs", "path": "./themes/light.json" }
                ]
            }
        }"#
    }

    fn webview_heavy_manifest_json() -> &'static str {
        r#"{
            "name": "webview-ext",
            "version": "1.0.0",
            "publisher": "viewmaker",
            "engines": { "vscode": "^1.80.0" },
            "main": "./dist/extension.js",
            "categories": ["Other"],
            "contributes": {
                "views": {
                    "explorer": [
                        { "id": "v1", "name": "View 1" },
                        { "id": "v2", "name": "View 2" },
                        { "id": "v3", "name": "View 3" }
                    ],
                    "debug": [
                        { "id": "v4", "name": "View 4" }
                    ]
                }
            }
        }"#
    }

    // --- Manifest parsing tests ---

    #[test]
    fn test_parse_full_manifest() {
        let m = parse_manifest(full_manifest_json()).unwrap();
        assert_eq!(m.name, "my-ext");
        assert_eq!(m.version, "1.2.3");
        assert_eq!(m.publisher, "testpub");
        assert_eq!(m.engines.vscode, "^1.70.0");
        assert_eq!(m.activation_events, vec!["onLanguage:rust"]);
        assert_eq!(m.main.as_deref(), Some("./out/extension.js"));
        assert_eq!(m.categories, vec!["Programming Languages"]);
    }

    #[test]
    fn test_parse_minimal_manifest() {
        let m = parse_manifest(minimal_manifest_json()).unwrap();
        assert_eq!(m.name, "mini");
        assert!(m.activation_events.is_empty());
        assert!(m.main.is_none());
        assert!(m.categories.is_empty());
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse_manifest("not json").is_err());
    }

    #[test]
    fn test_parse_missing_required_field() {
        let json = r#"{ "name": "x" }"#;
        assert!(parse_manifest(json).is_err());
    }

    #[test]
    fn test_parse_empty_object() {
        assert!(parse_manifest("{}").is_err());
    }

    #[test]
    fn test_parse_contributions_commands() {
        let m = parse_manifest(full_manifest_json()).unwrap();
        let cmds = m.contributes.commands.as_ref().unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].command, "myext.hello");
        assert_eq!(cmds[0].title, "Hello");
        assert_eq!(cmds[0].category.as_deref(), Some("MyExt"));
    }

    #[test]
    fn test_parse_contributions_keybindings() {
        let m = parse_manifest(full_manifest_json()).unwrap();
        let kb = m.contributes.keybindings.as_ref().unwrap();
        assert_eq!(kb.len(), 1);
        assert_eq!(kb[0].key, "ctrl+shift+h");
        assert_eq!(kb[0].mac.as_deref(), Some("cmd+shift+h"));
        assert_eq!(kb[0].when.as_deref(), Some("editorTextFocus"));
    }

    #[test]
    fn test_parse_contributions_configuration() {
        let m = parse_manifest(full_manifest_json()).unwrap();
        let cfg = m.contributes.configuration.as_ref().unwrap();
        assert_eq!(cfg.len(), 1);
        let prop = cfg[0].properties.get("myext.enabled").unwrap();
        assert_eq!(prop.type_field, "boolean");
        assert_eq!(prop.default_value, Some(serde_json::Value::Bool(true)));
    }

    #[test]
    fn test_parse_contributions_languages() {
        let m = parse_manifest(full_manifest_json()).unwrap();
        let langs = m.contributes.languages.as_ref().unwrap();
        assert_eq!(langs[0].id, "mylang");
        assert_eq!(langs[0].extensions, vec![".ml"]);
    }

    #[test]
    fn test_parse_contributions_grammars() {
        let m = parse_manifest(full_manifest_json()).unwrap();
        let g = m.contributes.grammars.as_ref().unwrap();
        assert_eq!(g[0].scope_name, "source.mylang");
    }

    #[test]
    fn test_parse_contributions_themes() {
        let m = parse_manifest(theme_only_manifest_json()).unwrap();
        let themes = m.contributes.themes.as_ref().unwrap();
        assert_eq!(themes.len(), 2);
        assert_eq!(themes[0].label, "Pretty Dark");
    }

    #[test]
    fn test_parse_contributions_snippets() {
        let m = parse_manifest(full_manifest_json()).unwrap();
        let snips = m.contributes.snippets.as_ref().unwrap();
        assert_eq!(snips[0].language, "mylang");
    }

    #[test]
    fn test_parse_contributions_views() {
        let m = parse_manifest(full_manifest_json()).unwrap();
        let views = m.contributes.views.as_ref().unwrap();
        assert!(views.contains_key("explorer"));
        assert_eq!(views["explorer"][0].id, "myView");
    }

    #[test]
    fn test_parse_contributions_debuggers() {
        let m = parse_manifest(full_manifest_json()).unwrap();
        let dbg = m.contributes.debuggers.as_ref().unwrap();
        assert_eq!(dbg[0].type_field, "mydbg");
        assert_eq!(dbg[0].languages, vec!["mylang"]);
    }

    // --- Compatibility check tests ---

    #[test]
    fn test_compat_theme_extension_high_score() {
        let m = parse_manifest(theme_only_manifest_json()).unwrap();
        let report = check_compatibility(&m);
        assert_eq!(report.extension_name, "themer.pretty-theme");
        assert!(report.compatibility_score >= 0.95, "Theme score should be ~1.0, got {}", report.compatibility_score);
        assert!(report.can_install);
        assert!(!report.supported_features.is_empty());
        assert!(report.unsupported_features.is_empty());
    }

    #[test]
    fn test_compat_webview_extension_low_score() {
        let m = parse_manifest(webview_heavy_manifest_json()).unwrap();
        let report = check_compatibility(&m);
        assert!(report.compatibility_score <= 0.4, "Webview-heavy score should be low, got {}", report.compatibility_score);
        assert!(!report.unsupported_features.is_empty());
    }

    #[test]
    fn test_compat_minimal_extension() {
        let m = parse_manifest(minimal_manifest_json()).unwrap();
        let report = check_compatibility(&m);
        // No contributions => score 1.0 (nothing to be incompatible with).
        assert!((report.compatibility_score - 1.0).abs() < f64::EPSILON);
        assert!(report.can_install);
    }

    #[test]
    fn test_compat_full_extension() {
        let m = parse_manifest(full_manifest_json()).unwrap();
        let report = check_compatibility(&m);
        // Has views and debuggers pulling score down, but many high-compat items.
        assert!(report.compatibility_score > 0.5);
        assert!(report.compatibility_score < 1.0);
        assert!(report.can_install);
    }

    #[test]
    fn test_compat_report_has_migration_notes_for_views() {
        let m = parse_manifest(webview_heavy_manifest_json()).unwrap();
        let report = check_compatibility(&m);
        assert!(report.migration_notes.iter().any(|n| n.contains("views")));
    }

    #[test]
    fn test_compat_report_has_main_note() {
        let m = parse_manifest(full_manifest_json()).unwrap();
        let report = check_compatibility(&m);
        assert!(report.migration_notes.iter().any(|n| n.contains("main entry point")));
    }

    // --- Score for category tests ---

    #[test]
    fn test_category_score_themes() {
        assert!((compatibility_score_for_category("themes") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_category_score_snippets() {
        assert!((compatibility_score_for_category("snippets") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_category_score_commands() {
        assert!((compatibility_score_for_category("commands") - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_category_score_debuggers() {
        assert!((compatibility_score_for_category("debuggers") - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_category_score_views() {
        assert!((compatibility_score_for_category("views") - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn test_category_score_unknown() {
        assert!((compatibility_score_for_category("unknown")).abs() < f64::EPSILON);
    }

    #[test]
    fn test_category_score_case_insensitive() {
        assert!((compatibility_score_for_category("Themes") - 1.0).abs() < f64::EPSILON);
        assert!((compatibility_score_for_category("COMMANDS") - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_score_bounds() {
        for cat in &["themes", "snippets", "languages", "commands", "keybindings", "grammars", "configuration", "debuggers", "views", "other"] {
            let s = compatibility_score_for_category(cat);
            assert!(s >= 0.0 && s <= 1.0, "Score for {} out of bounds: {}", cat, s);
        }
    }

    // --- Supported APIs tests ---

    #[test]
    fn test_supported_apis_count() {
        let apis = supported_apis();
        assert!(apis.len() >= 30, "Expected at least 30 APIs, got {}", apis.len());
    }

    #[test]
    fn test_supported_apis_no_empty_descriptions() {
        for (api, desc) in supported_apis() {
            assert!(!desc.is_empty(), "Empty description for {:?}", api);
        }
    }

    #[test]
    fn test_supported_apis_unique() {
        let apis = supported_apis();
        let mut seen = std::collections::HashSet::new();
        for (api, _) in &apis {
            assert!(seen.insert(api), "Duplicate API: {:?}", api);
        }
    }

    // --- Shim generation tests ---

    #[test]
    fn test_shim_window_show_message() {
        let shim = generate_shim(&SupportedApi::WindowShowMessage);
        assert!(shim.contains("showInformationMessage"));
        assert!(shim.contains("require('vscode')"));
    }

    #[test]
    fn test_shim_commands_register() {
        let shim = generate_shim(&SupportedApi::CommandsRegister);
        assert!(shim.contains("registerCommand"));
    }

    #[test]
    fn test_shim_workspace_find_files() {
        let shim = generate_shim(&SupportedApi::WorkspaceFindFiles);
        assert!(shim.contains("findFiles"));
        assert!(shim.contains("async"));
    }

    #[test]
    fn test_shim_env_app_name() {
        let shim = generate_shim(&SupportedApi::EnvAppName);
        assert!(shim.contains("appName"));
    }

    #[test]
    fn test_shim_all_non_empty() {
        for (api, _) in supported_apis() {
            let shim = generate_shim(&api);
            assert!(!shim.is_empty(), "Empty shim for {:?}", api);
            assert!(shim.contains("vscode"), "Shim for {:?} missing vscode reference", api);
        }
    }

    #[test]
    fn test_shim_languages_register_completion() {
        let shim = generate_shim(&SupportedApi::LanguagesRegisterCompletion);
        assert!(shim.contains("registerCompletionItemProvider"));
    }

    #[test]
    fn test_shim_debug_start() {
        let shim = generate_shim(&SupportedApi::DebugStartDebugging);
        assert!(shim.contains("startDebugging"));
    }

    // --- Serialization round-trip tests ---

    #[test]
    fn test_manifest_serialize_roundtrip() {
        let m = parse_manifest(full_manifest_json()).unwrap();
        let json = serde_json::to_string(&m).unwrap();
        let m2: ExtensionManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m.name, m2.name);
        assert_eq!(m.version, m2.version);
    }

    #[test]
    fn test_compatibility_report_serializes() {
        let m = parse_manifest(full_manifest_json()).unwrap();
        let report = check_compatibility(&m);
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("compatibility_score"));
        assert!(json.contains("can_install"));
    }

    #[test]
    fn test_partial_feature_serializes() {
        let pf = PartialFeature {
            name: "webviews".into(),
            supported_part: "basic panels".into(),
            unsupported_part: "custom editors".into(),
        };
        let json = serde_json::to_string(&pf).unwrap();
        assert!(json.contains("webviews"));
        assert!(json.contains("basic panels"));
    }

    #[test]
    fn test_supported_api_serializes() {
        let json = serde_json::to_string(&SupportedApi::WindowShowMessage).unwrap();
        assert!(json.contains("WindowShowMessage"));
    }
}
