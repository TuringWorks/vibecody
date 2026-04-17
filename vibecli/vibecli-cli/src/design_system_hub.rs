//! Design System Hub — cross-provider token registry, component catalogue, and audit tooling.
//!
//! Normalises design tokens from Figma, Penpot, Pencil, and in-house sources into a
//! unified representation. Supports token export (CSS, Tailwind, Style Dictionary),
//! component inventory, design system audits, and drift detection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::design_providers::{DesignToken, DesignTokenType, ProviderKind, tokens_to_css, tokens_to_ts};

// ─── Token namespace ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenNamespace {
    pub name: String,
    pub tokens: Vec<DesignToken>,
    pub provider: Option<ProviderKind>,
}

impl TokenNamespace {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), tokens: Vec::new(), provider: None }
    }

    pub fn add(&mut self, token: DesignToken) {
        self.tokens.push(token);
    }

    pub fn by_type(&self, t: &DesignTokenType) -> Vec<&DesignToken> {
        self.tokens.iter().filter(|tok| &tok.token_type == t).collect()
    }
}

// ─── Design system ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignSystem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub namespaces: Vec<TokenNamespace>,
    pub component_catalogue: Vec<DesignSystemComponent>,
    pub providers: Vec<ProviderKind>,
    pub updated_at_ms: u64,
}

impl DesignSystem {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            id: uuid_short(),
            name: name.to_string(),
            version: version.to_string(),
            namespaces: Vec::new(),
            component_catalogue: Vec::new(),
            providers: Vec::new(),
            updated_at_ms: epoch_ms(),
        }
    }

    pub fn add_namespace(&mut self, ns: TokenNamespace) {
        if let Some(p) = &ns.provider {
            if !self.providers.contains(p) {
                self.providers.push(p.clone());
            }
        }
        self.namespaces.push(ns);
    }

    pub fn add_component(&mut self, comp: DesignSystemComponent) {
        self.component_catalogue.push(comp);
    }

    /// Flatten all tokens across all namespaces
    pub fn all_tokens(&self) -> Vec<&DesignToken> {
        self.namespaces.iter().flat_map(|ns| ns.tokens.iter()).collect()
    }

    /// Flatten all tokens by type
    pub fn tokens_by_type(&self, t: &DesignTokenType) -> Vec<&DesignToken> {
        self.all_tokens().into_iter().filter(|tok| &tok.token_type == t).collect()
    }

    /// Export all tokens to CSS variables
    pub fn export_css(&self) -> String {
        let all: Vec<DesignToken> = self.all_tokens().into_iter().cloned().collect();
        format!("/* Design System: {} v{} */\n{}", self.name, self.version, tokens_to_css(&all))
    }

    /// Export all tokens to TypeScript
    pub fn export_ts(&self) -> String {
        let all: Vec<DesignToken> = self.all_tokens().into_iter().cloned().collect();
        format!("// Design System: {} v{}\n{}", self.name, self.version, tokens_to_ts(&all))
    }

    /// Export to Tailwind CSS config extend section
    pub fn export_tailwind(&self) -> String {
        let colors = self.tokens_by_type(&DesignTokenType::Color);
        let spacing = self.tokens_by_type(&DesignTokenType::Spacing);

        let color_entries: String = colors.iter().map(|t| {
            let key = t.name.to_lowercase().replace([' ', '/'], "-");
            format!("      \"{}\": \"{}\",\n", key, t.value)
        }).collect();

        let spacing_entries: String = spacing.iter().map(|t| {
            let key = t.name.to_lowercase().replace(' ', "-");
            format!("      \"{}\": \"{}\",\n", key, t.value)
        }).collect();

        format!(
            r#"// tailwind.config.js extend block (from Design System: {} v{})
module.exports = {{
  theme: {{
    extend: {{
      colors: {{
{}      }},
      spacing: {{
{}      }},
    }},
  }},
}};"#,
            self.name, self.version, color_entries, spacing_entries
        )
    }

    /// Export to Style Dictionary tokens.json format
    pub fn export_style_dictionary(&self) -> String {
        let mut root = serde_json::Map::new();
        for ns in &self.namespaces {
            let mut ns_obj = serde_json::Map::new();
            for token in &ns.tokens {
                let key = token.name.to_lowercase().replace([' ', '-'], "_");
                ns_obj.insert(key, serde_json::json!({
                    "value": token.value,
                    "type": format!("{:?}", token.token_type).to_lowercase(),
                    "description": token.description.as_deref().unwrap_or("")
                }));
            }
            let ns_key = ns.name.to_lowercase().replace(' ', "_");
            root.insert(ns_key, serde_json::Value::Object(ns_obj));
        }
        serde_json::to_string_pretty(&serde_json::Value::Object(root))
            .unwrap_or_else(|_| "{}".to_string())
    }
}

// ─── Component catalogue entry ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignSystemComponent {
    pub id: String,
    pub name: String,
    pub category: ComponentCategory,
    pub status: ComponentStatus,
    pub providers: Vec<ProviderKind>,
    pub variants: Vec<ComponentVariant>,
    pub tokens_used: Vec<String>,
    pub figma_url: Option<String>,
    pub penpot_id: Option<String>,
    pub documentation_url: Option<String>,
    pub source_file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentCategory {
    Foundation,
    Layout,
    Navigation,
    Forms,
    Feedback,
    DataDisplay,
    Overlay,
    Chart,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStatus {
    Stable,
    Beta,
    Experimental,
    Deprecated,
    Planned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentVariant {
    pub name: String,
    pub props: HashMap<String, String>,
    pub preview_html: Option<String>,
}

// ─── Design system audit ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditIssue {
    pub severity: AuditSeverity,
    pub code: String,
    pub message: String,
    pub affected: Vec<String>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub system_name: String,
    pub system_version: String,
    pub issues: Vec<AuditIssue>,
    pub score: u8,
    pub summary: String,
}

/// Run a design system audit and produce a report.
pub fn audit_design_system(ds: &DesignSystem) -> AuditReport {
    let mut issues = Vec::new();

    // Check: no colors defined
    if ds.tokens_by_type(&DesignTokenType::Color).is_empty() {
        issues.push(AuditIssue {
            severity: AuditSeverity::Warning,
            code: "NO_COLORS".to_string(),
            message: "No color tokens defined".to_string(),
            affected: vec![],
            suggestion: Some("Add a color namespace with primary, secondary, and semantic colors".to_string()),
        });
    }

    // Check: no typography
    if ds.tokens_by_type(&DesignTokenType::Typography).is_empty() {
        issues.push(AuditIssue {
            severity: AuditSeverity::Warning,
            code: "NO_TYPOGRAPHY".to_string(),
            message: "No typography tokens defined".to_string(),
            affected: vec![],
            suggestion: Some("Add font-family, font-size, and line-height tokens".to_string()),
        });
    }

    // Check: no spacing
    if ds.tokens_by_type(&DesignTokenType::Spacing).is_empty() {
        issues.push(AuditIssue {
            severity: AuditSeverity::Info,
            code: "NO_SPACING".to_string(),
            message: "No spacing tokens defined".to_string(),
            affected: vec![],
            suggestion: Some("Define a spacing scale (4px, 8px, 16px, 24px, 32px, ...)".to_string()),
        });
    }

    // Check: duplicate token names
    let all_tokens = ds.all_tokens();
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    for t in &all_tokens {
        *name_counts.entry(t.name.clone()).or_insert(0) += 1;
    }
    let duplicates: Vec<String> = name_counts.into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(name, _)| name)
        .collect();
    if !duplicates.is_empty() {
        issues.push(AuditIssue {
            severity: AuditSeverity::Error,
            code: "DUPLICATE_TOKENS".to_string(),
            message: format!("{} duplicate token name(s) found", duplicates.len()),
            affected: duplicates,
            suggestion: Some("Ensure each token has a unique name within the system".to_string()),
        });
    }

    // Check: no components
    if ds.component_catalogue.is_empty() {
        issues.push(AuditIssue {
            severity: AuditSeverity::Info,
            code: "NO_COMPONENTS".to_string(),
            message: "No components registered in the catalogue".to_string(),
            affected: vec![],
            suggestion: Some("Add components to the catalogue via import or manual registration".to_string()),
        });
    }

    // Score: start at 100, deduct for issues
    let error_count = issues.iter().filter(|i| i.severity == AuditSeverity::Error).count();
    let warning_count = issues.iter().filter(|i| i.severity == AuditSeverity::Warning).count();
    let score = (100u8).saturating_sub((error_count * 20 + warning_count * 10) as u8);

    let summary = if issues.is_empty() {
        format!("Design system '{}' passes all checks. Score: {}/100", ds.name, score)
    } else {
        format!(
            "Design system '{}' has {} error(s), {} warning(s). Score: {}/100",
            ds.name, error_count, warning_count, score
        )
    };

    AuditReport { system_name: ds.name.clone(), system_version: ds.version.clone(), issues, score, summary }
}

// ─── Token drift detection ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDrift {
    pub token_name: String,
    pub baseline_value: String,
    pub current_value: String,
    pub provider: ProviderKind,
}

/// Compare two versions of a design system and report token value changes
pub fn detect_token_drift(baseline: &DesignSystem, current: &DesignSystem) -> Vec<TokenDrift> {
    let mut baseline_map: HashMap<String, String> = HashMap::new();
    for t in baseline.all_tokens() {
        baseline_map.insert(t.name.clone(), t.value.clone());
    }

    let mut drifts = Vec::new();
    for t in current.all_tokens() {
        if let Some(base_val) = baseline_map.get(&t.name) {
            if base_val != &t.value {
                drifts.push(TokenDrift {
                    token_name: t.name.clone(),
                    baseline_value: base_val.clone(),
                    current_value: t.value.clone(),
                    provider: t.provider.clone(),
                });
            }
        }
    }
    drifts
}

// ─── Provider merge ───────────────────────────────────────────────────────────

/// Merge tokens from multiple providers into a single design system namespace.
/// When the same token name appears in multiple providers, the first occurrence wins
/// unless `prefer_provider` is specified.
pub fn merge_provider_tokens(
    providers_tokens: &[(ProviderKind, Vec<DesignToken>)],
    prefer_provider: Option<&ProviderKind>,
) -> Vec<DesignToken> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut merged: Vec<DesignToken> = Vec::new();

    // First pass: add tokens from the preferred provider
    if let Some(preferred) = prefer_provider {
        for (provider, tokens) in providers_tokens {
            if provider == preferred {
                for t in tokens {
                    seen.insert(t.name.clone(), merged.len());
                    merged.push(t.clone());
                }
            }
        }
    }

    // Second pass: add remaining tokens (skip duplicates)
    for (provider, tokens) in providers_tokens {
        if prefer_provider.map(|p| p != provider).unwrap_or(true) {
            for t in tokens {
                if !seen.contains_key(&t.name) {
                    seen.insert(t.name.clone(), merged.len());
                    merged.push(t.clone());
                }
            }
        }
    }

    merged
}

// ─── Standard VibeCody design system ─────────────────────────────────────────

/// Returns the default VibeCody design token system
pub fn vibecody_default_design_system() -> DesignSystem {
    let mut ds = DesignSystem::new("VibeCody", "1.0.0");
    let mut colors = TokenNamespace::new("colors");
    colors.provider = Some(ProviderKind::Inhouse);

    for (name, value) in [
        ("accent-blue", "#3b82f6"),
        ("accent-green", "#10b981"),
        ("accent-orange", "#f59e0b"),
        ("accent-red", "#ef4444"),
        ("accent-purple", "#8b5cf6"),
        ("bg-primary", "#0d1117"),
        ("bg-secondary", "#161b22"),
        ("bg-tertiary", "#21262d"),
        ("bg-elevated", "#30363d"),
        ("text-primary", "#e6edf3"),
        ("text-secondary", "#8b949e"),
        ("text-success", "#3fb950"),
        ("border-color", "#30363d"),
        ("warning-color", "#d29922"),
        ("error-color", "#f85149"),
    ] {
        colors.add(DesignToken {
            name: name.to_string(),
            token_type: DesignTokenType::Color,
            value: value.to_string(),
            description: None,
            provider: ProviderKind::Inhouse,
        });
    }
    ds.add_namespace(colors);

    let mut spacing = TokenNamespace::new("spacing");
    spacing.provider = Some(ProviderKind::Inhouse);
    for (name, value) in [
        ("space-1", "4px"), ("space-2", "8px"), ("space-3", "12px"),
        ("space-4", "16px"), ("space-6", "24px"), ("space-8", "32px"),
        ("space-12", "48px"), ("space-16", "64px"),
    ] {
        spacing.add(DesignToken {
            name: name.to_string(),
            token_type: DesignTokenType::Spacing,
            value: value.to_string(),
            description: None,
            provider: ProviderKind::Inhouse,
        });
    }
    ds.add_namespace(spacing);

    let mut typography = TokenNamespace::new("typography");
    typography.provider = Some(ProviderKind::Inhouse);
    for (name, value) in [
        ("font-mono", "'SF Mono', 'Fira Code', monospace"),
        ("font-sans", "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif"),
        ("font-size-xs", "11px"), ("font-size-sm", "12px"), ("font-size-base", "14px"),
        ("font-size-lg", "16px"), ("font-size-xl", "20px"), ("font-size-2xl", "24px"),
    ] {
        typography.add(DesignToken {
            name: name.to_string(),
            token_type: DesignTokenType::Typography,
            value: value.to_string(),
            description: None,
            provider: ProviderKind::Inhouse,
        });
    }
    ds.add_namespace(typography);
    ds
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{:x}{:04x}", t.as_secs(), t.subsec_micros() & 0xffff)
}

fn epoch_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_color_token(name: &str, value: &str) -> DesignToken {
        DesignToken {
            name: name.to_string(),
            token_type: DesignTokenType::Color,
            value: value.to_string(),
            description: None,
            provider: ProviderKind::Inhouse,
        }
    }

    #[test]
    fn design_system_export_css() {
        let mut ds = DesignSystem::new("Test", "1.0");
        let mut ns = TokenNamespace::new("colors");
        ns.add(make_color_token("primary", "#3b82f6"));
        ds.add_namespace(ns);
        let css = ds.export_css();
        assert!(css.contains(":root"));
        assert!(css.contains("--primary: #3b82f6"));
    }

    #[test]
    fn design_system_export_tailwind() {
        let mut ds = DesignSystem::new("Test", "1.0");
        let mut ns = TokenNamespace::new("colors");
        ns.add(make_color_token("blue-500", "#3b82f6"));
        ds.add_namespace(ns);
        let tw = ds.export_tailwind();
        assert!(tw.contains("colors"));
        assert!(tw.contains("blue-500"));
    }

    #[test]
    fn design_system_export_style_dictionary() {
        let mut ds = DesignSystem::new("Test", "1.0");
        let mut ns = TokenNamespace::new("brand");
        ns.add(make_color_token("primary", "#000"));
        ds.add_namespace(ns);
        let sd = ds.export_style_dictionary();
        let v: serde_json::Value = serde_json::from_str(&sd).unwrap();
        assert!(v["brand"]["primary"]["value"].as_str().is_some());
    }

    #[test]
    fn audit_empty_system_has_warnings() {
        let ds = DesignSystem::new("Empty", "0.1");
        let report = audit_design_system(&ds);
        assert!(!report.issues.is_empty());
        assert!(report.score < 100);
    }

    #[test]
    fn audit_duplicate_tokens_flagged() {
        let mut ds = DesignSystem::new("Test", "1.0");
        let mut ns1 = TokenNamespace::new("a");
        ns1.add(make_color_token("primary", "#000"));
        let mut ns2 = TokenNamespace::new("b");
        ns2.add(make_color_token("primary", "#fff"));
        ds.add_namespace(ns1);
        ds.add_namespace(ns2);
        let report = audit_design_system(&ds);
        assert!(report.issues.iter().any(|i| i.code == "DUPLICATE_TOKENS"));
    }

    #[test]
    fn detect_token_drift_finds_changes() {
        let mut baseline = DesignSystem::new("DS", "1.0");
        let mut ns = TokenNamespace::new("c");
        ns.add(make_color_token("primary", "#000"));
        baseline.add_namespace(ns);

        let mut current = DesignSystem::new("DS", "1.1");
        let mut ns2 = TokenNamespace::new("c");
        ns2.add(make_color_token("primary", "#3b82f6")); // changed
        current.add_namespace(ns2);

        let drifts = detect_token_drift(&baseline, &current);
        assert_eq!(drifts.len(), 1);
        assert_eq!(drifts[0].token_name, "primary");
    }

    #[test]
    fn merge_provider_tokens_deduplicates() {
        let tokens_a = vec![make_color_token("primary", "#000")];
        let tokens_b = vec![make_color_token("primary", "#fff"), make_color_token("secondary", "#888")];
        let merged = merge_provider_tokens(
            &[(ProviderKind::Figma, tokens_a), (ProviderKind::Penpot, tokens_b)],
            Some(&ProviderKind::Figma),
        );
        assert_eq!(merged.len(), 2); // primary from Figma + secondary from Penpot
        assert_eq!(merged[0].value, "#000");
    }

    #[test]
    fn vibecody_default_ds_has_colors() {
        let ds = vibecody_default_design_system();
        let colors = ds.tokens_by_type(&DesignTokenType::Color);
        assert!(!colors.is_empty());
        assert!(colors.iter().any(|t| t.name == "accent-blue"));
    }
}
