//! `vibecli-plugin.toml` — inner manifest carried inside an MCPB bundle.
//!
//! B2.1 of the May-2026 plugin-bundle work. The outer container is the
//! open MCPB format already shipped in A2 (`mcpb_bundle.rs`); this is
//! the VibeCody-specific manifest that lives at the root of an
//! extracted bundle and tells VibeCLI which components to register.
//!
//! Patent-distance posture (fit-gap §18):
//!   - principle #3: artifact format is open MCPB; lineage to `.vsix`
//!     + MetaPK keeps prior art clear. This module only defines the
//!     manifest *inside* that container — no proprietary wrapping.
//!   - principle #4: publisher trust roots are per-publisher P-256
//!     ECDSA keys, embedded here as a `PublicKeyJwk` reusing the same
//!     JWK shape as the A2A signed agent card (`signed_agent_card.rs`).
//!     This module records the key; B2.2 verifies the signature.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::signed_agent_card::PublicKeyJwk;

/// Maximum length of the free-form description, in bytes. Keeps panel
/// rendering predictable and discourages bundling release notes.
pub const MAX_DESCRIPTION_BYTES: usize = 500;

/// Hook events a plugin may register against. Matches the existing
/// hook system (`.claude/settings.json` / `hook_abort.rs`).
pub const ALLOWED_HOOK_EVENTS: &[&str] = &[
    "PreToolUse",
    "PostToolUse",
    "UserPromptSubmit",
    "Stop",
    "SubagentStop",
    "Notification",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginManifest {
    /// Stable plugin id, kebab-case lowercase. The install registry
    /// is keyed on this — it must be globally unique within a
    /// workspace.
    pub name: String,

    /// SemVer-shaped version string (`major.minor.patch` with optional
    /// `-prerelease` / `+build` suffix). We validate the shape but
    /// don't pull in a full `semver` crate just for parsing — the
    /// registry handles structural ordering elsewhere.
    pub version: String,

    /// Publisher identity + signing key.
    pub publisher: Publisher,

    /// Free-form description, ≤ `MAX_DESCRIPTION_BYTES`.
    #[serde(default)]
    pub description: String,

    /// Components carried by this bundle. Every field is `default`
    /// so a manifest can ship just one kind of component.
    #[serde(default)]
    pub components: Components,

    /// Optional minimum VibeCLI version required at install time.
    /// Format matches `version` above.
    #[serde(default)]
    pub min_vibecli_version: Option<String>,

    /// Initial policy when the plugin is first installed in a
    /// workspace. Admin can override at install time. See B2.3 for
    /// the runtime semantics.
    #[serde(default)]
    pub default_policy: DefaultPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Publisher {
    /// Display name. Required.
    pub name: String,

    /// Optional homepage / repo URL.
    #[serde(default)]
    pub url: Option<String>,

    /// P-256 ECDSA public key as JWK (RFC 7517 + RFC 7518), reusing
    /// the same shape as the A2A signed agent card. B2.2 verifies the
    /// detached signature; we just record the key.
    pub key: PublicKeyJwk,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Components {
    #[serde(default)]
    pub mcp_servers: Vec<McpServerComponent>,
    #[serde(default)]
    pub skills: Vec<SkillComponent>,
    #[serde(default)]
    pub subagents: Vec<SubagentComponent>,
    #[serde(default)]
    pub rules: Vec<RuleComponent>,
    #[serde(default)]
    pub hooks: Vec<HookComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpServerComponent {
    pub name: String,
    /// Relative path inside the extracted bundle.
    pub path: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillComponent {
    pub name: String,
    /// Relative path to the `.md` skill file inside the bundle.
    pub path: String,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubagentComponent {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuleComponent {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookComponent {
    pub name: String,
    /// One of `ALLOWED_HOOK_EVENTS`.
    pub event: String,
    pub path: String,
}

/// Default install policy. Admin can override at `vibecli plugin
/// install` time. See B2.3 for the runtime semantics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DefaultPolicy {
    #[default]
    Off,
    On,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    EmptyField(&'static str),
    InvalidName(String),
    InvalidVersion(String),
    DescriptionTooLong { max: usize, got: usize },
    UnknownHookEvent(String),
    DuplicateComponent { kind: &'static str, name: String },
    Toml(String),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField(k) => write!(f, "field `{k}` must not be empty"),
            Self::InvalidName(n) => {
                write!(
                    f,
                    "plugin name `{n}` must be kebab-case lowercase (a-z, 0-9, -)"
                )
            }
            Self::InvalidVersion(v) => {
                write!(
                    f,
                    "version `{v}` is not semver-shaped (expected major.minor.patch)"
                )
            }
            Self::DescriptionTooLong { max, got } => {
                write!(f, "description is {got} bytes, max {max}")
            }
            Self::UnknownHookEvent(e) => write!(f, "unknown hook event `{e}`"),
            Self::DuplicateComponent { kind, name } => {
                write!(f, "duplicate {kind} component `{name}`")
            }
            Self::Toml(msg) => write!(f, "toml parse: {msg}"),
        }
    }
}

impl std::error::Error for ManifestError {}

impl PluginManifest {
    /// Parse a TOML string and validate it in one step. Most callers
    /// want this; the raw `serde` path is fine for round-trip tests.
    pub fn parse(toml_str: &str) -> Result<Self, ManifestError> {
        let m: Self = toml::from_str(toml_str).map_err(|e| ManifestError::Toml(e.to_string()))?;
        m.validate()?;
        Ok(m)
    }

    /// Structural validation. Run after deserialization to catch the
    /// constraints serde can't express.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.name.is_empty() {
            return Err(ManifestError::EmptyField("name"));
        }
        if !is_kebab_case(&self.name) {
            return Err(ManifestError::InvalidName(self.name.clone()));
        }
        if self.version.is_empty() {
            return Err(ManifestError::EmptyField("version"));
        }
        if !is_semverish(&self.version) {
            return Err(ManifestError::InvalidVersion(self.version.clone()));
        }
        if self.publisher.name.is_empty() {
            return Err(ManifestError::EmptyField("publisher.name"));
        }
        // The JWK fields are surface-checked here; full P-256 point
        // validation happens in B2.2 when we actually load the key.
        if self.publisher.key.kty.is_empty() {
            return Err(ManifestError::EmptyField("publisher.key.kty"));
        }
        if self.publisher.key.crv.is_empty() {
            return Err(ManifestError::EmptyField("publisher.key.crv"));
        }
        if self.publisher.key.x.is_empty() || self.publisher.key.y.is_empty() {
            return Err(ManifestError::EmptyField("publisher.key.x|y"));
        }
        if self.description.len() > MAX_DESCRIPTION_BYTES {
            return Err(ManifestError::DescriptionTooLong {
                max: MAX_DESCRIPTION_BYTES,
                got: self.description.len(),
            });
        }
        if let Some(min) = &self.min_vibecli_version {
            if !is_semverish(min) {
                return Err(ManifestError::InvalidVersion(min.clone()));
            }
        }
        check_dup(
            "mcp_server",
            self.components.mcp_servers.iter().map(|c| &c.name),
        )?;
        check_dup("skill", self.components.skills.iter().map(|c| &c.name))?;
        check_dup(
            "subagent",
            self.components.subagents.iter().map(|c| &c.name),
        )?;
        check_dup("rule", self.components.rules.iter().map(|c| &c.name))?;
        check_dup("hook", self.components.hooks.iter().map(|c| &c.name))?;
        for h in &self.components.hooks {
            if !ALLOWED_HOOK_EVENTS.contains(&h.event.as_str()) {
                return Err(ManifestError::UnknownHookEvent(h.event.clone()));
            }
        }
        Ok(())
    }
}

fn is_kebab_case(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
}

fn is_semverish(s: &str) -> bool {
    let core = s.split(['-', '+']).next().unwrap_or("");
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn check_dup<'a>(
    kind: &'static str,
    names: impl Iterator<Item = &'a String>,
) -> Result<(), ManifestError> {
    let mut seen = HashSet::new();
    for n in names {
        if !seen.insert(n.as_str()) {
            return Err(ManifestError::DuplicateComponent {
                kind,
                name: n.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_jwk() -> PublicKeyJwk {
        PublicKeyJwk {
            kty: "EC".into(),
            crv: "P-256".into(),
            x: "f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU".into(),
            y: "x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0".into(),
        }
    }

    fn minimal_toml() -> String {
        format!(
            r#"
name = "example-plugin"
version = "1.0.0"
description = "A test bundle"

[publisher]
name = "Test Publisher"
url = "https://example.test"

[publisher.key]
kty = "{kty}"
crv = "{crv}"
x = "{x}"
y = "{y}"
"#,
            kty = sample_jwk().kty,
            crv = sample_jwk().crv,
            x = sample_jwk().x,
            y = sample_jwk().y,
        )
    }

    #[test]
    fn parse_minimal_manifest_succeeds() {
        let m = PluginManifest::parse(&minimal_toml()).expect("parse");
        assert_eq!(m.name, "example-plugin");
        assert_eq!(m.version, "1.0.0");
        assert_eq!(m.publisher.name, "Test Publisher");
        assert_eq!(m.default_policy, DefaultPolicy::Off);
    }

    #[test]
    fn parse_full_manifest_round_trips() {
        // Build a manifest with at least one of every component kind
        // and round-trip through toml::to_string → parse.
        let original = PluginManifest {
            name: "full-plugin".into(),
            version: "0.2.3".into(),
            publisher: Publisher {
                name: "Pub".into(),
                url: None,
                key: sample_jwk(),
            },
            description: "ok".into(),
            components: Components {
                mcp_servers: vec![McpServerComponent {
                    name: "srv".into(),
                    path: "bin/srv".into(),
                    args: vec!["--port".into(), "9".into()],
                }],
                skills: vec![SkillComponent {
                    name: "s1".into(),
                    path: "skills/s1.md".into(),
                    category: Some("tools".into()),
                }],
                subagents: vec![SubagentComponent {
                    name: "sa".into(),
                    path: "agents/sa.toml".into(),
                }],
                rules: vec![RuleComponent {
                    name: "r1".into(),
                    path: "rules/r1.md".into(),
                }],
                hooks: vec![HookComponent {
                    name: "h1".into(),
                    event: "PreToolUse".into(),
                    path: "hooks/h1.sh".into(),
                }],
            },
            min_vibecli_version: Some("0.5.7".into()),
            default_policy: DefaultPolicy::On,
        };
        let serialized = toml::to_string(&original).expect("serialize");
        let parsed = PluginManifest::parse(&serialized).expect("parse round-trip");
        assert_eq!(parsed, original);
    }

    #[test]
    fn reject_empty_name() {
        let mut s = minimal_toml();
        s = s.replace(r#"name = "example-plugin""#, r#"name = """#);
        let err = PluginManifest::parse(&s).unwrap_err();
        assert!(
            matches!(err, ManifestError::EmptyField("name")),
            "got {err}"
        );
    }

    #[test]
    fn reject_uppercase_name() {
        let mut s = minimal_toml();
        s = s.replace(r#"name = "example-plugin""#, r#"name = "Example-Plugin""#);
        let err = PluginManifest::parse(&s).unwrap_err();
        assert!(matches!(err, ManifestError::InvalidName(_)), "got {err}");
    }

    #[test]
    fn reject_two_part_version() {
        let mut s = minimal_toml();
        s = s.replace(r#"version = "1.0.0""#, r#"version = "1.0""#);
        let err = PluginManifest::parse(&s).unwrap_err();
        assert!(matches!(err, ManifestError::InvalidVersion(_)), "got {err}");
    }

    #[test]
    fn accept_prerelease_version() {
        let mut s = minimal_toml();
        s = s.replace(r#"version = "1.0.0""#, r#"version = "1.0.0-rc.1""#);
        let m = PluginManifest::parse(&s).expect("parse");
        assert_eq!(m.version, "1.0.0-rc.1");
    }

    #[test]
    fn reject_description_too_long() {
        let mut s = minimal_toml();
        let big = "x".repeat(MAX_DESCRIPTION_BYTES + 1);
        s = s.replace(
            r#"description = "A test bundle""#,
            &format!(r#"description = "{big}""#),
        );
        let err = PluginManifest::parse(&s).unwrap_err();
        assert!(
            matches!(err, ManifestError::DescriptionTooLong { .. }),
            "got {err}"
        );
    }

    #[test]
    fn reject_unknown_hook_event() {
        let s = minimal_toml()
            + r#"
[[components.hooks]]
name = "evil"
event = "OnEverything"
path = "hooks/evil.sh"
"#;
        let err = PluginManifest::parse(&s).unwrap_err();
        assert!(
            matches!(err, ManifestError::UnknownHookEvent(_)),
            "got {err}"
        );
    }

    #[test]
    fn reject_duplicate_skill_component() {
        let s = minimal_toml()
            + r#"
[[components.skills]]
name = "same"
path = "a.md"
[[components.skills]]
name = "same"
path = "b.md"
"#;
        let err = PluginManifest::parse(&s).unwrap_err();
        assert!(
            matches!(err, ManifestError::DuplicateComponent { kind: "skill", .. }),
            "got {err}"
        );
    }

    #[test]
    fn default_policy_is_off() {
        assert_eq!(DefaultPolicy::default(), DefaultPolicy::Off);
    }

    #[test]
    fn reject_empty_publisher_key_coordinate() {
        let mut s = minimal_toml();
        s = s.replace(&format!(r#"x = "{}""#, sample_jwk().x), r#"x = """#);
        let err = PluginManifest::parse(&s).unwrap_err();
        assert!(matches!(err, ManifestError::EmptyField(_)), "got {err}");
    }
}
