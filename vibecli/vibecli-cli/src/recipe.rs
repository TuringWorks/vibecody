//! Recipe system — parameterized, shareable multi-step automation workflows.
//!
//! Inspired by Goose's recipe system but extended with:
//! - Multi-step sequential prompts
//! - Handlebars-style `{{ param }}` injection
//! - Per-recipe provider/model overrides
//! - AI-generated recipe suggestions from completed sessions
//!
//! # Recipe YAML format
//! ```yaml
//! name: create-feature-branch
//! description: Scaffold a feature with tests
//! version: "1.0"
//! author: your-name
//! parameters:
//!   feature_name:
//!     type: string
//!     required: true
//!     description: Name of the feature
//!   language:
//!     type: string
//!     default: rust
//! provider: claude          # optional — overrides CLI --provider
//! model: claude-sonnet-4-6  # optional — overrides CLI --model
//! steps:
//!   - prompt: "Create a git branch named feature/{{ feature_name }}"
//!   - prompt: "Write a {{ language }} module for {{ feature_name }}"
//!   - prompt: "Write unit tests for the {{ feature_name }} module"
//! ```

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeParam {
    #[serde(rename = "type", default = "default_type")]
    pub param_type: String,
    #[serde(default)]
    pub required: bool,
    pub default: Option<String>,
    pub description: Option<String>,
}

fn default_type() -> String { "string".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStep {
    pub prompt: String,
    /// Optional provider override for this step only.
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_version")]
    pub version: String,
    pub author: Option<String>,
    #[serde(default)]
    pub parameters: HashMap<String, RecipeParam>,
    pub provider: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub steps: Vec<RecipeStep>,
    /// Tags for categorization and search.
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_version() -> String { "1.0".into() }

impl Recipe {
    /// Load a recipe from a YAML file.
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read recipe '{}': {}", path, e))?;
        let recipe: Self = serde_yaml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Invalid recipe YAML in '{}': {}", path, e))?;
        Ok(recipe)
    }

    /// Validate that all required parameters are provided.
    pub fn validate_params(&self, params: &HashMap<String, String>) -> Result<()> {
        for (name, spec) in &self.parameters {
            if spec.required && !params.contains_key(name) && spec.default.is_none() {
                bail!("Recipe '{}' requires parameter --param {}=<value>", self.name, name);
            }
        }
        Ok(())
    }

    /// Build the full parameter map (provided values + defaults for missing).
    pub fn resolve_params(&self, provided: &HashMap<String, String>) -> HashMap<String, String> {
        let mut resolved = HashMap::new();
        for (name, spec) in &self.parameters {
            if let Some(v) = provided.get(name) {
                resolved.insert(name.clone(), v.clone());
            } else if let Some(d) = &spec.default {
                resolved.insert(name.clone(), d.clone());
            }
        }
        // Also pass through any extra params the user provided
        for (k, v) in provided {
            resolved.entry(k.clone()).or_insert_with(|| v.clone());
        }
        resolved
    }
}

// ── Template rendering ────────────────────────────────────────────────────────

/// Render `{{ param }}` placeholders in a template string.
/// Unresolved placeholders are left as-is with a warning.
pub fn render_template(template: &str, params: &HashMap<String, String>) -> String {
    let re = regex::Regex::new(r"\{\{\s*(\w+)\s*\}\}").unwrap();
    re.replace_all(template, |caps: &regex::Captures| {
        let key = &caps[1];
        params.get(key).cloned().unwrap_or_else(|| {
            eprintln!("  \x1b[33mWarning:\x1b[0m unresolved template variable: {{{{ {} }}}}", key);
            caps[0].to_string()
        })
    }).to_string()
}

// ── Runner ────────────────────────────────────────────────────────────────────

/// Run a recipe file with the given parameters.
pub async fn run_recipe(
    recipe_file: &str,
    params: &HashMap<String, String>,
    provider: &str,
    model: &Option<String>,
    _sandbox: bool,
) -> Result<()> {
    let recipe = Recipe::load(recipe_file)?;
    recipe.validate_params(params)?;
    let resolved = recipe.resolve_params(params);

    println!("\x1b[1;36m▶ Recipe: {}\x1b[0m", recipe.name);
    if let Some(desc) = &recipe.description {
        println!("  {}", desc);
    }
    println!("  Steps: {}", recipe.steps.len());
    println!("  Parameters: {:?}", resolved);
    println!();

    let effective_provider = recipe.provider.as_deref().unwrap_or(provider);
    let effective_model = recipe.model.as_ref().or(model.as_ref());

    for (i, step) in recipe.steps.iter().enumerate() {
        let prompt = render_template(&step.prompt, &resolved);
        let step_provider = step.provider.as_deref().unwrap_or(effective_provider);

        println!("\x1b[1;34m── Step {}/{}: {}\x1b[0m", i + 1, recipe.steps.len(), &prompt[..prompt.len().min(80)]);

        // Build an LLM provider and run the step
        let llm = crate::create_provider(step_provider, effective_model.map(|s| s.to_string()))?;
        run_recipe_step(llm.as_ref(), &prompt).await?;
        println!();
    }

    println!("\x1b[1;32m✔ Recipe '{}' completed ({} steps)\x1b[0m", recipe.name, recipe.steps.len());
    Ok(())
}

async fn run_recipe_step(
    llm: &dyn vibe_ai::provider::AIProvider,
    prompt: &str,
) -> Result<()> {
    use vibe_ai::provider::{Message, MessageRole};
    let msgs = vec![Message { role: MessageRole::User, content: prompt.to_string() }];
    let mut stream = llm.stream_chat(&msgs).await?;
    use futures::StreamExt;
    while let Some(chunk) = stream.next().await {
        print!("{}", chunk?);
    }
    println!();
    Ok(())
}

// ── List recipes ──────────────────────────────────────────────────────────────

/// List all recipes in the recipes directory and the current directory.
pub fn list_recipes() -> Vec<(String, Recipe)> {
    let mut recipes = vec![];
    let search_dirs = vec![
        std::path::PathBuf::from("recipes"),
        dirs::home_dir().map(|h| h.join(".vibecli").join("recipes")).unwrap_or_default(),
    ];
    for dir in search_dirs {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                    || path.extension().and_then(|e| e.to_str()) == Some("yml")
                {
                    if let Ok(r) = Recipe::load(path.to_str().unwrap_or("")) {
                        recipes.push((path.display().to_string(), r));
                    }
                }
            }
        }
    }
    recipes
}

// ── REPL handler ─────────────────────────────────────────────────────────────

pub fn handle_recipe_command(args: &str) -> String {
    let parts: Vec<&str> = args.splitn(3, ' ').collect();
    let subcmd = parts.first().copied().unwrap_or("list");
    match subcmd {
        "list" => {
            let recipes = list_recipes();
            if recipes.is_empty() {
                return "No recipes found. Place .yaml files in ./recipes/ or ~/.vibecli/recipes/\n".to_string();
            }
            let mut out = format!("📜 Recipes ({}):\n", recipes.len());
            for (path, r) in &recipes {
                out.push_str(&format!(
                    "  {} — {}\n    Path: {}\n",
                    r.name,
                    r.description.as_deref().unwrap_or("(no description)"),
                    path,
                ));
            }
            out
        }
        "show" | "info" => {
            let file = parts.get(1).copied().unwrap_or("");
            if file.is_empty() { return "Usage: /recipe show <file.yaml>\n".to_string(); }
            match Recipe::load(file) {
                Ok(r) => {
                    let mut out = format!("📜 {} (v{})\n", r.name, r.version);
                    if let Some(d) = &r.description { out.push_str(&format!("  {}\n", d)); }
                    if !r.parameters.is_empty() {
                        out.push_str("  Parameters:\n");
                        for (k, p) in &r.parameters {
                            let req = if p.required { " (required)" } else { "" };
                            let def = p.default.as_deref().map(|d| format!(" [default: {}]", d)).unwrap_or_default();
                            out.push_str(&format!("    --param {}=<{}>{}{}\n", k, p.param_type, req, def));
                        }
                    }
                    out.push_str(&format!("  Steps: {}\n", r.steps.len()));
                    for (i, s) in r.steps.iter().enumerate() {
                        out.push_str(&format!("    {}. {}\n", i + 1, &s.prompt[..s.prompt.len().min(60)]));
                    }
                    out
                }
                Err(e) => format!("❌ {}\n", e),
            }
        }
        _ => {
            "📜 Recipe Commands:\n\
              /recipe list           — List available recipes\n\
              /recipe show <file>    — Show recipe details\n\n\
            Run from CLI: vibecli --recipe <file.yaml> --param key=value\n".to_string()
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_template_substitutes() {
        let mut params = HashMap::new();
        params.insert("feature_name".into(), "auth".into());
        params.insert("language".into(), "rust".into());
        let result = render_template("Create {{ feature_name }} in {{ language }}", &params);
        assert_eq!(result, "Create auth in rust");
    }

    #[test]
    fn test_render_template_leaves_unknown() {
        let params = HashMap::new();
        let result = render_template("Hello {{ unknown }}", &params);
        assert!(result.contains("{{ unknown }}"));
    }

    #[test]
    fn test_recipe_resolve_params_defaults() {
        let mut recipe = Recipe {
            name: "test".into(), description: None, version: "1.0".into(),
            author: None, provider: None, model: None, steps: vec![], tags: vec![],
            parameters: HashMap::new(),
        };
        recipe.parameters.insert("lang".into(), RecipeParam {
            param_type: "string".into(), required: false,
            default: Some("rust".into()), description: None,
        });
        let resolved = recipe.resolve_params(&HashMap::new());
        assert_eq!(resolved.get("lang").map(String::as_str), Some("rust"));
    }

    #[test]
    fn test_recipe_validate_params_missing_required() {
        let mut recipe = Recipe {
            name: "test".into(), description: None, version: "1.0".into(),
            author: None, provider: None, model: None, steps: vec![], tags: vec![],
            parameters: HashMap::new(),
        };
        recipe.parameters.insert("name".into(), RecipeParam {
            param_type: "string".into(), required: true, default: None, description: None,
        });
        let result = recipe.validate_params(&HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_recipe_validate_params_ok() {
        let mut recipe = Recipe {
            name: "test".into(), description: None, version: "1.0".into(),
            author: None, provider: None, model: None, steps: vec![], tags: vec![],
            parameters: HashMap::new(),
        };
        recipe.parameters.insert("name".into(), RecipeParam {
            param_type: "string".into(), required: true, default: None, description: None,
        });
        let mut params = HashMap::new();
        params.insert("name".into(), "auth".into());
        assert!(recipe.validate_params(&params).is_ok());
    }
}
