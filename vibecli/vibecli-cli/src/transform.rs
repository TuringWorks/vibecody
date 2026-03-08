#![allow(dead_code)]
//! Code Transformation Agent — automated language/framework upgrades.
//!
//! Supported transforms:
//! - CommonJS → ESM
//! - React class → hooks
//! - Python 2 → 3 patterns
//! - Vue 2 → 3 patterns
//! - Java version upgrades
//!
//! Workflow: detect → plan → review → execute → test → commit

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vibe_ai::provider::AIProvider;
use vibe_ai::provider::{Message, MessageRole};

/// Known transformation types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformType {
    CommonjsToEsm,
    ReactClassToHooks,
    Python2To3,
    Vue2To3,
    JavaUpgrade,
    Custom(String),
}

impl std::fmt::Display for TransformType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommonjsToEsm => write!(f, "CommonJS → ESM"),
            Self::ReactClassToHooks => write!(f, "React Class → Hooks"),
            Self::Python2To3 => write!(f, "Python 2 → 3"),
            Self::Vue2To3 => write!(f, "Vue 2 → 3"),
            Self::JavaUpgrade => write!(f, "Java Version Upgrade"),
            Self::Custom(s) => write!(f, "Custom: {}", s),
        }
    }
}

/// A planned file transformation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformPlanItem {
    pub file: String,
    pub description: String,
    pub estimated_changes: usize,
}

/// The full transform plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformPlan {
    pub transform_type: TransformType,
    pub files: Vec<TransformPlanItem>,
    pub total_files: usize,
    pub summary: String,
}

/// Result of executing a transform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformResult {
    pub files_modified: usize,
    pub files_failed: usize,
    pub summary: String,
    pub diff_stats: String,
}

/// Detect what transforms are applicable to the current workspace.
pub fn detect_transforms(workspace: &std::path::Path) -> Vec<TransformType> {
    let mut transforms = Vec::new();

    // Check for CommonJS patterns
    if has_file_with_pattern(workspace, "**/*.js", "require(") {
        transforms.push(TransformType::CommonjsToEsm);
    }

    // Check for React class components
    if has_file_with_pattern(workspace, "**/*.{jsx,tsx}", "extends React.Component")
        || has_file_with_pattern(workspace, "**/*.{jsx,tsx}", "extends Component")
    {
        transforms.push(TransformType::ReactClassToHooks);
    }

    // Check for Python 2 patterns
    if has_file_with_pattern(workspace, "**/*.py", "print ") {
        transforms.push(TransformType::Python2To3);
    }

    // Check for Vue 2 patterns
    if has_file_with_pattern(workspace, "**/*.vue", "Vue.component") {
        transforms.push(TransformType::Vue2To3);
    }

    transforms
}

fn has_file_with_pattern(workspace: &std::path::Path, _glob: &str, pattern: &str) -> bool {
    // Simple recursive search for pattern in files
    for entry in walkdir::WalkDir::new(workspace)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                if content.contains(pattern) {
                    return true;
                }
            }
        }
    }
    false
}

/// Generate a transform plan using AI analysis.
pub async fn plan_transform(
    workspace: &std::path::Path,
    transform_type: &TransformType,
    llm: Arc<dyn AIProvider>,
) -> Result<TransformPlan> {
    // Find relevant files
    let relevant_files = find_relevant_files(workspace, transform_type);

    let prompt = format!(
        "You are a code transformation expert. I need to plan a '{}' transformation.\n\n\
        The following files need to be analyzed:\n{}\n\n\
        For each file, describe what changes are needed. Return a JSON array:\n\
        [{{\"file\": \"path\", \"description\": \"what to change\", \"estimated_changes\": N}}]\n\n\
        Only return the JSON array, nothing else.",
        transform_type,
        relevant_files.iter().take(20).map(|f| format!("- {}", f)).collect::<Vec<_>>().join("\n")
    );

    let messages = vec![
        Message { role: MessageRole::System, content: "You are a code migration expert.".into() },
        Message { role: MessageRole::User, content: prompt },
    ];

    let response = llm.chat(&messages, None).await?;

    // Try to parse the plan from JSON
    let items = parse_plan_json(&response).unwrap_or_else(|_| {
        relevant_files.iter().map(|f| TransformPlanItem {
            file: f.clone(),
            description: format!("Apply {} transform", transform_type),
            estimated_changes: 5,
        }).collect()
    });

    let total = items.len();
    Ok(TransformPlan {
        transform_type: transform_type.clone(),
        files: items,
        total_files: total,
        summary: format!("{} files to transform with {}", total, transform_type),
    })
}

fn parse_plan_json(content: &str) -> Result<Vec<TransformPlanItem>> {
    // Find JSON array in the response
    let start = content.find('[').ok_or_else(|| anyhow::anyhow!("No JSON array"))?;
    let end = content.rfind(']').ok_or_else(|| anyhow::anyhow!("No JSON array end"))? + 1;
    let json_str = &content[start..end];
    Ok(serde_json::from_str(json_str)?)
}

fn find_relevant_files(workspace: &std::path::Path, transform_type: &TransformType) -> Vec<String> {
    let extensions: Vec<&str> = match transform_type {
        TransformType::CommonjsToEsm => vec!["js", "cjs", "mjs"],
        TransformType::ReactClassToHooks => vec!["jsx", "tsx"],
        TransformType::Python2To3 => vec!["py"],
        TransformType::Vue2To3 => vec!["vue"],
        TransformType::JavaUpgrade => vec!["java"],
        TransformType::Custom(_) => vec!["js", "ts", "py", "rs", "java", "go"],
    };

    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(workspace)
        .max_depth(6)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("");
            if extensions.contains(&ext) {
                if let Ok(rel) = entry.path().strip_prefix(workspace) {
                    let rel_str = rel.to_string_lossy().to_string();
                    // Skip node_modules, target, .git, etc.
                    if !rel_str.contains("node_modules") && !rel_str.contains("/target/") && !rel_str.starts_with(".git") {
                        files.push(rel_str);
                    }
                }
            }
        }
    }
    files.sort();
    files
}

/// Execute a single file transform using AI.
pub async fn execute_transform_file(
    workspace: &std::path::Path,
    file: &str,
    transform_type: &TransformType,
    llm: Arc<dyn AIProvider>,
) -> Result<bool> {
    let file_path = workspace.join(file);
    let content = std::fs::read_to_string(&file_path)?;

    let prompt = format!(
        "Apply the '{}' transformation to this file. Return ONLY the complete transformed file content, nothing else.\n\n\
        File: {}\n```\n{}\n```",
        transform_type, file, content
    );

    let messages = vec![
        Message { role: MessageRole::System, content: "You are a code migration tool. Return only the transformed code.".into() },
        Message { role: MessageRole::User, content: prompt },
    ];

    let response = llm.chat(&messages, None).await?;

    // Extract code from response (strip markdown fences if present)
    let transformed = strip_code_fences(&response);

    if transformed.trim().is_empty() {
        anyhow::bail!("Empty transformed output");
    }

    std::fs::write(&file_path, transformed)?;
    Ok(true)
}

fn strip_code_fences(s: &str) -> String {
    let s = s.trim();
    if s.starts_with("```") {
        let start = s.find('\n').map(|i| i + 1).unwrap_or(3);
        let end = s.rfind("```").unwrap_or(s.len());
        s[start..end].trim().to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_type_display() {
        assert_eq!(TransformType::CommonjsToEsm.to_string(), "CommonJS → ESM");
        assert_eq!(TransformType::ReactClassToHooks.to_string(), "React Class → Hooks");
    }

    #[test]
    fn strip_code_fences_works() {
        let input = "```javascript\nconst x = 1;\n```";
        assert_eq!(strip_code_fences(input), "const x = 1;");
    }

    #[test]
    fn strip_code_fences_no_fences() {
        let input = "const x = 1;";
        assert_eq!(strip_code_fences(input), "const x = 1;");
    }

    #[test]
    fn parse_plan_json_works() {
        let input = r#"Here is the plan: [{"file":"a.js","description":"convert require","estimated_changes":3}]"#;
        let items = parse_plan_json(input).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].file, "a.js");
    }

    #[test]
    fn transform_plan_serde() {
        let plan = TransformPlan {
            transform_type: TransformType::CommonjsToEsm,
            files: vec![TransformPlanItem {
                file: "index.js".into(),
                description: "Convert require to import".into(),
                estimated_changes: 5,
            }],
            total_files: 1,
            summary: "1 file".into(),
        };
        let json = serde_json::to_string(&plan).unwrap();
        let parsed: TransformPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_files, 1);
    }

    // ── TransformType serde roundtrip ──

    #[test]
    fn transform_type_serde_roundtrip() {
        let types = vec![
            TransformType::CommonjsToEsm,
            TransformType::ReactClassToHooks,
            TransformType::Python2To3,
            TransformType::Vue2To3,
            TransformType::JavaUpgrade,
            TransformType::Custom("custom-migration".to_string()),
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let back: TransformType = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{}", back), format!("{}", t));
        }
    }

    // ── TransformType Display for Custom ──

    #[test]
    fn transform_type_display_custom() {
        let t = TransformType::Custom("Angular v15 to v16".to_string());
        assert_eq!(t.to_string(), "Custom: Angular v15 to v16");
    }

    // ── strip_code_fences edge cases ──

    #[test]
    fn strip_code_fences_empty_string() {
        assert_eq!(strip_code_fences(""), "");
    }

    #[test]
    fn strip_code_fences_only_fences() {
        let input = "```\n```";
        let result = strip_code_fences(input);
        assert_eq!(result, "");
    }

    #[test]
    fn strip_code_fences_with_language_and_trailing_whitespace() {
        let input = "```python  \nprint('hello')\n```";
        assert_eq!(strip_code_fences(input), "print('hello')");
    }

    // ── parse_plan_json edge cases ──

    #[test]
    fn parse_plan_json_no_array_returns_error() {
        let input = "No JSON here, just text.";
        assert!(parse_plan_json(input).is_err());
    }

    #[test]
    fn parse_plan_json_empty_array() {
        let input = "Here is the plan: []";
        let items = parse_plan_json(input).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn parse_plan_json_multiple_items() {
        let input = r#"[
            {"file":"a.js","description":"convert","estimated_changes":3},
            {"file":"b.ts","description":"migrate","estimated_changes":10}
        ]"#;
        let items = parse_plan_json(input).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].file, "a.js");
        assert_eq!(items[1].estimated_changes, 10);
    }

    // ── TransformResult construction ──

    #[test]
    fn transform_result_construction() {
        let result = TransformResult {
            files_modified: 5,
            files_failed: 1,
            summary: "5 of 6 files transformed successfully".to_string(),
            diff_stats: "+42 -18".to_string(),
        };
        assert_eq!(result.files_modified, 5);
        assert_eq!(result.files_failed, 1);
        assert!(result.summary.contains("successfully"));
    }

    // ── TransformPlanItem serde ──

    #[test]
    fn transform_plan_item_serde_roundtrip() {
        let item = TransformPlanItem {
            file: "src/utils.js".into(),
            description: "Convert require() to import".into(),
            estimated_changes: 7,
        };
        let json = serde_json::to_string(&item).unwrap();
        let back: TransformPlanItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.file, "src/utils.js");
        assert_eq!(back.estimated_changes, 7);
    }

    // ── TransformType display all variants ──

    #[test]
    fn transform_type_display_all_variants() {
        assert_eq!(TransformType::Python2To3.to_string(), "Python 2 \u{2192} 3");
        assert_eq!(TransformType::Vue2To3.to_string(), "Vue 2 \u{2192} 3");
        assert_eq!(TransformType::JavaUpgrade.to_string(), "Java Version Upgrade");
    }

    // ── TransformResult serde roundtrip ──

    #[test]
    fn transform_result_serde_roundtrip() {
        let result = TransformResult {
            files_modified: 10,
            files_failed: 2,
            summary: "done".to_string(),
            diff_stats: "+100 -50".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: TransformResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.files_modified, 10);
        assert_eq!(back.files_failed, 2);
        assert_eq!(back.diff_stats, "+100 -50");
    }

    // ── parse_plan_json with surrounding text ──

    #[test]
    fn parse_plan_json_with_surrounding_prose() {
        let input = r#"Sure, here is the plan:
[{"file":"src/index.js","description":"Convert require to import","estimated_changes":4}]
Let me know if you want changes."#;
        let items = parse_plan_json(input).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].file, "src/index.js");
        assert_eq!(items[0].estimated_changes, 4);
    }

    // ── parse_plan_json with invalid JSON content ──

    #[test]
    fn parse_plan_json_invalid_json_in_brackets() {
        let input = "[not valid json}";
        assert!(parse_plan_json(input).is_err());
    }

    // ── strip_code_fences with multiple fence blocks (takes first) ──

    #[test]
    fn strip_code_fences_nested_backticks() {
        let input = "```rust\nfn main() {\n    println!(\"hello\");\n}\n```";
        let result = strip_code_fences(input);
        assert!(result.contains("fn main()"));
        assert!(!result.contains("```"));
    }

    // ── TransformPlan summary format ──

    #[test]
    fn transform_plan_summary_format() {
        let plan = TransformPlan {
            transform_type: TransformType::Python2To3,
            files: vec![
                TransformPlanItem { file: "a.py".into(), description: "fix print".into(), estimated_changes: 2 },
                TransformPlanItem { file: "b.py".into(), description: "fix dict".into(), estimated_changes: 3 },
            ],
            total_files: 2,
            summary: "2 files to transform with Python 2 \u{2192} 3".into(),
        };
        assert_eq!(plan.total_files, plan.files.len());
        assert!(plan.summary.contains("2 files"));
    }

    // ── TransformType JSON values use snake_case ──

    #[test]
    fn transform_type_json_snake_case() {
        let json = serde_json::to_string(&TransformType::CommonjsToEsm).unwrap();
        assert_eq!(json, r#""commonjs_to_esm""#);
        let json = serde_json::to_string(&TransformType::ReactClassToHooks).unwrap();
        assert_eq!(json, r#""react_class_to_hooks""#);
        let json = serde_json::to_string(&TransformType::Python2To3).unwrap();
        assert_eq!(json, r#""python2_to3""#);
        let json = serde_json::to_string(&TransformType::JavaUpgrade).unwrap();
        assert_eq!(json, r#""java_upgrade""#);
    }

    // ── Custom TransformType serde ──

    #[test]
    fn transform_type_custom_serde_roundtrip() {
        let t = TransformType::Custom("webpack-to-vite".to_string());
        let json = serde_json::to_string(&t).unwrap();
        let back: TransformType = serde_json::from_str(&json).unwrap();
        match back {
            TransformType::Custom(s) => assert_eq!(s, "webpack-to-vite"),
            _ => panic!("Expected Custom variant"),
        }
    }
}
