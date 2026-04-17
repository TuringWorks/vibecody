//! Penpot integration — REST API client + component/token extraction.
//!
//! Penpot is an open-source design tool (figma-alternative).
//! Supports self-hosted instances (https://penpot.app or custom URL).
//! API: https://{host}/api/rpc/command/{method}
//!
//! Also handles the TuringWorks/penpot fork's extended capabilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::design_providers::{DesignComponent, DesignError, DesignFile, DesignFrame, DesignToken, DesignTokenType, ProviderKind};

// ─── Penpot API types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenpotConfig {
    /// Base URL of the Penpot instance, e.g. "https://design.penpot.app"
    pub host: String,
    /// Personal access token (Settings → Access Tokens in Penpot UI)
    pub token: String,
}

impl PenpotConfig {
    pub fn new(host: &str, token: &str) -> Self {
        Self { host: host.trim_end_matches('/').to_string(), token: token.to_string() }
    }

    pub fn default_cloud() -> Self {
        Self::new("https://design.penpot.app", "")
    }

    pub fn api_url(&self, command: &str) -> String {
        format!("{}/api/rpc/command/{}", self.host, command)
    }
}

// ─── Penpot project / file types ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenpotProject {
    pub id: String,
    pub name: String,
    pub team_id: String,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenpotFile {
    pub id: String,
    pub name: String,
    pub project_id: String,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub revn: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenpotPage {
    pub id: String,
    pub name: String,
    pub objects: HashMap<String, PenpotObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenpotObject {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub obj_type: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub fill_color: Option<String>,
    pub stroke_color: Option<String>,
    pub opacity: Option<f64>,
    pub children: Option<Vec<String>>,
    pub component_id: Option<String>,
    pub component_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenpotComponent {
    pub id: String,
    pub name: String,
    pub path: String,
    pub objects: HashMap<String, PenpotObject>,
    pub main_instance_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenpotColor {
    pub id: String,
    pub name: String,
    pub color: String,
    pub opacity: Option<f64>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenpotTypography {
    pub id: String,
    pub name: String,
    pub font_family: String,
    pub font_size: Option<String>,
    pub font_weight: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenpotFileData {
    pub file: PenpotFile,
    pub pages: Vec<PenpotPage>,
    pub components: HashMap<String, PenpotComponent>,
    pub colors: Vec<PenpotColor>,
    pub typographies: Vec<PenpotTypography>,
}

// ─── HTTP request builders ────────────────────────────────────────────────────

/// Builds curl-compatible HTTP request descriptors for Penpot API calls.
/// Actual HTTP execution is handled by the agent via reqwest or the Tauri command layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenpotRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
}

impl PenpotRequest {
    fn new(method: &str, url: &str, token: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), format!("Token {}", token));
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Accept".to_string(), "application/json".to_string());
        Self { method: method.to_string(), url: url.to_string(), headers, body: None }
    }

    pub fn get_profile(cfg: &PenpotConfig) -> Self {
        Self::new("GET", &cfg.api_url("get-profile"), &cfg.token)
    }

    pub fn get_projects(cfg: &PenpotConfig) -> Self {
        Self::new("GET", &cfg.api_url("get-all-projects"), &cfg.token)
    }

    pub fn get_project_files(cfg: &PenpotConfig, project_id: &str) -> Self {
        let url = format!("{}?project-id={}", cfg.api_url("get-files"), project_id);
        Self::new("GET", &url, &cfg.token)
    }

    pub fn get_file(cfg: &PenpotConfig, file_id: &str) -> Self {
        let url = format!("{}?id={}", cfg.api_url("get-file"), file_id);
        Self::new("GET", &url, &cfg.token)
    }

    pub fn get_file_data_for_thumbnail(cfg: &PenpotConfig, file_id: &str) -> Self {
        let url = format!("{}?file-id={}", cfg.api_url("get-file-data-for-thumbnail"), file_id);
        Self::new("GET", &url, &cfg.token)
    }

    pub fn get_team_shared_files(cfg: &PenpotConfig, team_id: &str) -> Self {
        let url = format!("{}?team-id={}", cfg.api_url("get-team-shared-files"), team_id);
        Self::new("GET", &url, &cfg.token)
    }

    pub fn export_file_svg(cfg: &PenpotConfig, file_id: &str, page_id: &str, object_ids: &[&str]) -> Self {
        let ids: Vec<serde_json::Value> = object_ids.iter().map(|id| serde_json::json!(id)).collect();
        let mut r = Self::new("POST", &cfg.api_url("export-binfile"), &cfg.token);
        r.body = Some(serde_json::json!({
            "file-id": file_id,
            "page-id": page_id,
            "object-ids": ids,
            "format": "svg"
        }));
        r
    }

    pub fn duplicate_file(cfg: &PenpotConfig, file_id: &str, new_name: &str) -> Self {
        let mut r = Self::new("POST", &cfg.api_url("duplicate-file"), &cfg.token);
        r.body = Some(serde_json::json!({ "file-id": file_id, "name": new_name }));
        r
    }

    pub fn to_curl(&self) -> String {
        let method_flag = match self.method.as_str() {
            "POST" => "-X POST".to_string(),
            "DELETE" => "-X DELETE".to_string(),
            _ => String::new(),
        };
        let headers: String = self.headers.iter()
            .map(|(k, v)| format!("-H \"{}: {}\"", k, v))
            .collect::<Vec<_>>().join(" \\\n  ");
        let body_flag = self.body.as_ref()
            .map(|b| format!("-d '{}'", serde_json::to_string(b).unwrap_or_default()))
            .unwrap_or_default();
        format!("curl {} \\\n  {} \\\n  {} \\\n  \"{}\"", method_flag, headers, body_flag, self.url)
    }
}

// ─── Response parsing ─────────────────────────────────────────────────────────

/// Parse a Penpot get-file JSON response into a DesignFile
pub fn parse_penpot_file_response(json_str: &str) -> Result<DesignFile, DesignError> {
    let v: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| DesignError::new("PARSE_ERROR", &e.to_string()))?;

    let file_id = v["id"].as_str().unwrap_or("").to_string();
    let name = v["name"].as_str().unwrap_or("Untitled").to_string();
    let modified = v["modifiedAt"].as_str().map(|s| s.to_string());

    let mut frames = Vec::new();
    let mut components = Vec::new();
    let mut tokens = Vec::new();

    // Extract pages / frames
    if let Some(pages) = v["data"]["pagesIndex"].as_object() {
        for (_pid, page) in pages {
            let pname = page["name"].as_str().unwrap_or("Page").to_string();
            let pid = page["id"].as_str().unwrap_or("").to_string();
            // Find frame objects
            if let Some(objects) = page["objects"].as_object() {
                for (_oid, obj) in objects {
                    if obj["type"].as_str() == Some("frame") {
                        frames.push(DesignFrame {
                            id: obj["id"].as_str().unwrap_or("").to_string(),
                            name: format!("{} / {}", pname, obj["name"].as_str().unwrap_or("Frame")),
                            width: obj["width"].as_f64().unwrap_or(0.0) as u32,
                            height: obj["height"].as_f64().unwrap_or(0.0) as u32,
                            thumbnail_url: None,
                        });
                    }
                }
            }
            let _ = pid;
        }
    }

    // Extract shared components
    if let Some(comp_map) = v["data"]["components"].as_object() {
        for (cid, comp) in comp_map {
            components.push(DesignComponent {
                id: cid.clone(),
                name: comp["name"].as_str().unwrap_or("Component").to_string(),
                description: comp["path"].as_str().unwrap_or("").to_string(),
                category: "penpot-component".to_string(),
                props: HashMap::new(),
            });
        }
    }

    // Extract colors as tokens
    if let Some(color_map) = v["data"]["colors"].as_object() {
        for (_cid, color) in color_map {
            let hex = color["color"].as_str().unwrap_or("").to_string();
            let cname = color["name"].as_str().unwrap_or("color").to_string();
            if !hex.is_empty() {
                tokens.push(DesignToken {
                    name: cname,
                    token_type: DesignTokenType::Color,
                    value: hex,
                    description: color["path"].as_str().map(|s| s.to_string()),
                    provider: ProviderKind::Penpot,
                });
            }
        }
    }

    // Extract typographies
    if let Some(typo_map) = v["data"]["typographies"].as_object() {
        for (_tid, typo) in typo_map {
            let fname = typo["fontFamily"].as_str().unwrap_or("sans-serif").to_string();
            let tname = typo["name"].as_str().unwrap_or("typography").to_string();
            tokens.push(DesignToken {
                name: tname,
                token_type: DesignTokenType::Typography,
                value: fname,
                description: None,
                provider: ProviderKind::Penpot,
            });
        }
    }

    Ok(DesignFile {
        id: file_id,
        name,
        provider: ProviderKind::Penpot,
        last_modified: modified,
        frames,
        components,
        tokens,
    })
}

// ─── Component to React code generation ──────────────────────────────────────

/// Generate a React component scaffold from a Penpot component definition
pub fn penpot_component_to_react(comp: &PenpotComponent, framework: &str) -> String {
    let comp_name = to_pascal_case(&comp.name);
    let objects_summary: Vec<String> = comp.objects.values()
        .filter(|o| !matches!(o.obj_type.as_str(), "frame" | "group"))
        .map(|o| format!("  // {} ({})", o.name, o.obj_type))
        .take(10)
        .collect();

    match framework {
        "vue" => format!(
            r#"<!-- Penpot Component: {} -->
<template>
  <div class="{}-wrapper">
    <!-- Generated from Penpot component {} -->
{}
  </div>
</template>

<script setup lang="ts">
defineProps<{{ label?: string }}>()
</script>

<style scoped>
.{}-wrapper {{
  position: relative;
}}
</style>"#,
            comp.name,
            comp.name.to_lowercase().replace(' ', "-"),
            comp.id,
            objects_summary.join("\n"),
            comp.name.to_lowercase().replace(' ', "-"),
        ),
        "svelte" => format!(
            r#"<!-- Penpot Component: {} -->
<script lang="ts">
  export let label: string = '{}';
</script>

<div class="{}-wrapper">
  <!-- Generated from Penpot component {} -->
{}
</div>

<style>
  .{}-wrapper {{
    position: relative;
  }}
</style>"#,
            comp.name,
            comp.name,
            comp.name.to_lowercase().replace(' ', "-"),
            comp.id,
            objects_summary.join("\n"),
            comp.name.to_lowercase().replace(' ', "-"),
        ),
        _ => format!(
            r#"// Penpot Component: {}
// Source component ID: {}
import React from 'react';

interface {}Props {{
  label?: string;
  className?: string;
}}

export function {}({{ label = '{}', className = '' }}: {}Props) {{
  return (
    <div className={{`{}-wrapper ${{className}}`}}>
      {{/* Generated from Penpot component: {} */}}
{}
      <span>{{label}}</span>
    </div>
  );
}}
"#,
            comp.name, comp.id,
            comp_name, comp_name, comp.name, comp_name,
            comp.name.to_lowercase().replace(' ', "-"),
            comp.id,
            objects_summary.join("\n"),
        ),
    }
}

// ─── Design token export ──────────────────────────────────────────────────────

/// Export Penpot colors to CSS custom properties
pub fn penpot_colors_to_css(colors: &[PenpotColor]) -> String {
    let mut css = String::from(":root {\n  /* Penpot Design Tokens */\n");
    for c in colors {
        let var_name = c.name.to_lowercase().replace(' ', "-").replace('/', "--");
        css.push_str(&format!("  --{}: {};\n", var_name, c.color));
    }
    css.push('}');
    css
}

/// Export Penpot typographies to CSS variables
pub fn penpot_typography_to_css(typographies: &[PenpotTypography]) -> String {
    let mut css = String::from(":root {\n  /* Penpot Typography Tokens */\n");
    for t in typographies {
        let var_name = t.name.to_lowercase().replace(' ', "-");
        css.push_str(&format!("  --font-{}: {};\n", var_name, t.font_family));
        if let Some(size) = &t.font_size {
            css.push_str(&format!("  --font-size-{}: {};\n", var_name, size));
        }
        if let Some(weight) = &t.font_weight {
            css.push_str(&format!("  --font-weight-{}: {};\n", var_name, weight));
        }
    }
    css.push('}');
    css
}

// ─── Validate config ──────────────────────────────────────────────────────────

pub fn validate_penpot_config(cfg: &PenpotConfig) -> Result<(), DesignError> {
    if cfg.host.is_empty() {
        return Err(DesignError::new("INVALID_CONFIG", "Penpot host URL is required"));
    }
    if !cfg.host.starts_with("http://") && !cfg.host.starts_with("https://") {
        return Err(DesignError::new("INVALID_CONFIG", "Penpot host must start with http:// or https://"));
    }
    if cfg.token.is_empty() {
        return Err(DesignError::new("INVALID_CONFIG", "Penpot access token is required"));
    }
    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn to_pascal_case(s: &str) -> String {
    s.split([' ', '-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_api_url() {
        let cfg = PenpotConfig::new("https://design.penpot.app", "tok123");
        assert_eq!(cfg.api_url("get-profile"), "https://design.penpot.app/api/rpc/command/get-profile");
    }

    #[test]
    fn config_trims_trailing_slash() {
        let cfg = PenpotConfig::new("https://design.penpot.app/", "tok");
        assert!(!cfg.host.ends_with('/'));
    }

    #[test]
    fn validate_config_empty_host_fails() {
        let cfg = PenpotConfig::new("", "tok");
        assert!(validate_penpot_config(&cfg).is_err());
    }

    #[test]
    fn validate_config_no_schema_fails() {
        let cfg = PenpotConfig::new("design.penpot.app", "tok");
        assert!(validate_penpot_config(&cfg).is_err());
    }

    #[test]
    fn validate_config_empty_token_fails() {
        let cfg = PenpotConfig::new("https://design.penpot.app", "");
        assert!(validate_penpot_config(&cfg).is_err());
    }

    #[test]
    fn validate_config_valid_passes() {
        let cfg = PenpotConfig::new("https://design.penpot.app", "abc123");
        assert!(validate_penpot_config(&cfg).is_ok());
    }

    #[test]
    fn request_to_curl_includes_token() {
        let cfg = PenpotConfig::new("https://example.penpot.app", "mytoken");
        let req = PenpotRequest::get_profile(&cfg);
        let curl = req.to_curl();
        assert!(curl.contains("mytoken"));
        assert!(curl.contains("get-profile"));
    }

    #[test]
    fn penpot_colors_to_css_output() {
        let colors = vec![PenpotColor {
            id: "c1".into(), name: "Primary Blue".into(), color: "#3b82f6".into(),
            opacity: Some(1.0), path: None,
        }];
        let css = penpot_colors_to_css(&colors);
        assert!(css.contains("--primary-blue: #3b82f6;"));
    }

    #[test]
    fn penpot_component_to_react_generates_function() {
        let comp = PenpotComponent {
            id: "comp-1".into(), name: "Card".into(), path: "ui/cards".into(),
            objects: HashMap::new(), main_instance_id: None,
        };
        let code = penpot_component_to_react(&comp, "react");
        assert!(code.contains("function Card"));
        assert!(code.contains("CardProps"));
    }

    #[test]
    fn penpot_component_to_vue() {
        let comp = PenpotComponent {
            id: "c2".into(), name: "Button".into(), path: "".into(),
            objects: HashMap::new(), main_instance_id: None,
        };
        let code = penpot_component_to_react(&comp, "vue");
        assert!(code.contains("<template>"));
        assert!(code.contains("defineProps"));
    }

    #[test]
    fn parse_penpot_file_response_empty_json_fails() {
        assert!(parse_penpot_file_response("not-json").is_err());
    }

    #[test]
    fn parse_penpot_file_response_minimal_json() {
        let json = r#"{"id": "file-1", "name": "My Design", "data": {}}"#;
        let df = parse_penpot_file_response(json).unwrap();
        assert_eq!(df.id, "file-1");
        assert_eq!(df.name, "My Design");
        assert_eq!(df.provider, ProviderKind::Penpot);
    }

    #[test]
    fn to_pascal_case_converts() {
        assert_eq!(to_pascal_case("primary button"), "PrimaryButton");
    }
}
