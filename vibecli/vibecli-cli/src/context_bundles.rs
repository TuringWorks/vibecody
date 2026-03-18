//! Context Bundles / Spaces for VibeCody.
//!
//! A context bundle is a named, shareable set of context that includes pinned files,
//! instructions, excluded paths, and model preferences. Bundles can be activated,
//! deactivated, prioritized, and merged into the agent system prompt. They are
//! serializable to `.vibebundle.toml` for sharing and portability.

use std::collections::HashMap;
use std::path::PathBuf;

/// A named, shareable set of context.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextBundle {
    pub id: String,
    pub name: String,
    pub description: String,
    pub pinned_files: Vec<String>,
    pub instructions: Vec<String>,
    pub excluded_paths: Vec<String>,
    pub model_preference: Option<String>,
    pub priority: u32,
    pub tags: Vec<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl ContextBundle {
    fn new(id: String, name: String, description: String) -> Self {
        let now = current_timestamp();
        Self {
            id,
            name,
            description,
            pinned_files: Vec::new(),
            instructions: Vec::new(),
            excluded_paths: Vec::new(),
            model_preference: None,
            priority: 100,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Configuration for the bundle manager.
#[derive(Debug, Clone, PartialEq)]
pub struct BundleConfig {
    pub max_active_bundles: usize,
    pub auto_activate_tags: Vec<String>,
    pub default_priority: u32,
}

impl Default for BundleConfig {
    fn default() -> Self {
        Self {
            max_active_bundles: 10,
            auto_activate_tags: Vec::new(),
            default_priority: 100,
        }
    }
}

/// Summary status of a bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct BundleStatus {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub priority: u32,
    pub pinned_count: usize,
    pub instruction_count: usize,
}

/// Serializable export wrapper for a bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct BundleExport {
    pub version: String,
    pub bundle: ContextBundle,
    pub exported_at: u64,
}

/// Manages a collection of context bundles with activation and priority ordering.
#[derive(Debug)]
pub struct BundleManager {
    pub bundles: HashMap<String, ContextBundle>,
    pub active_bundles: Vec<String>,
    pub storage_dir: PathBuf,
    config: BundleConfig,
    next_id: u64,
}

fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn generate_id(counter: &mut u64) -> String {
    *counter += 1;
    format!("bundle_{}", counter)
}

impl BundleManager {
    /// Create a new bundle manager rooted at the given storage directory.
    pub fn new(storage_dir: PathBuf, config: BundleConfig) -> Self {
        Self {
            bundles: HashMap::new(),
            active_bundles: Vec::new(),
            storage_dir,
            config,
            next_id: 0,
        }
    }

    /// Create a bundle and return its generated id.
    pub fn create_bundle(&mut self, name: &str, description: &str) -> String {
        let id = generate_id(&mut self.next_id);
        let mut bundle = ContextBundle::new(id.clone(), name.to_string(), description.to_string());
        bundle.priority = self.config.default_priority;
        // Auto-activate if bundle has matching tags (applied later when tags are added)
        self.bundles.insert(id.clone(), bundle);
        id
    }

    pub fn get_bundle(&self, id: &str) -> Option<&ContextBundle> {
        self.bundles.get(id)
    }

    pub fn get_bundle_mut(&mut self, id: &str) -> Option<&mut ContextBundle> {
        self.bundles.get_mut(id)
    }

    pub fn delete_bundle(&mut self, id: &str) -> bool {
        if self.bundles.remove(id).is_some() {
            self.active_bundles.retain(|a| a != id);
            true
        } else {
            false
        }
    }

    /// Activate a bundle. Returns an error if the bundle does not exist or the
    /// maximum number of active bundles has been reached.
    pub fn activate(&mut self, id: &str) -> Result<(), String> {
        if !self.bundles.contains_key(id) {
            return Err(format!("Bundle '{}' not found", id));
        }
        if self.active_bundles.contains(&id.to_string()) {
            return Ok(()); // already active
        }
        if self.active_bundles.len() >= self.config.max_active_bundles {
            return Err(format!(
                "Maximum active bundles ({}) reached",
                self.config.max_active_bundles
            ));
        }
        self.active_bundles.push(id.to_string());
        self.sort_active_by_priority();
        Ok(())
    }

    /// Deactivate a bundle.
    pub fn deactivate(&mut self, id: &str) -> Result<(), String> {
        if !self.bundles.contains_key(id) {
            return Err(format!("Bundle '{}' not found", id));
        }
        let before = self.active_bundles.len();
        self.active_bundles.retain(|a| a != id);
        if self.active_bundles.len() == before {
            return Err(format!("Bundle '{}' is not active", id));
        }
        Ok(())
    }

    /// Return active bundles sorted by priority (lowest number = highest priority).
    pub fn list_active(&self) -> Vec<&ContextBundle> {
        self.active_bundles
            .iter()
            .filter_map(|id| self.bundles.get(id))
            .collect()
    }

    /// Return status summaries for all bundles.
    pub fn list_all(&self) -> Vec<BundleStatus> {
        let mut statuses: Vec<BundleStatus> = self
            .bundles
            .values()
            .map(|b| BundleStatus {
                id: b.id.clone(),
                name: b.name.clone(),
                active: self.active_bundles.contains(&b.id),
                priority: b.priority,
                pinned_count: b.pinned_files.len(),
                instruction_count: b.instructions.len(),
            })
            .collect();
        statuses.sort_by(|a, b| a.id.cmp(&b.id));
        statuses
    }

    pub fn add_pinned_file(&mut self, bundle_id: &str, path: &str) -> Result<(), String> {
        let bundle = self
            .bundles
            .get_mut(bundle_id)
            .ok_or_else(|| format!("Bundle '{}' not found", bundle_id))?;
        if !bundle.pinned_files.contains(&path.to_string()) {
            bundle.pinned_files.push(path.to_string());
            bundle.updated_at = current_timestamp();
        }
        Ok(())
    }

    pub fn remove_pinned_file(&mut self, bundle_id: &str, path: &str) -> Result<(), String> {
        let bundle = self
            .bundles
            .get_mut(bundle_id)
            .ok_or_else(|| format!("Bundle '{}' not found", bundle_id))?;
        let before = bundle.pinned_files.len();
        bundle.pinned_files.retain(|p| p != path);
        if bundle.pinned_files.len() == before {
            return Err(format!("File '{}' not found in bundle", path));
        }
        bundle.updated_at = current_timestamp();
        Ok(())
    }

    pub fn add_instruction(&mut self, bundle_id: &str, instruction: &str) -> Result<(), String> {
        let bundle = self
            .bundles
            .get_mut(bundle_id)
            .ok_or_else(|| format!("Bundle '{}' not found", bundle_id))?;
        bundle.instructions.push(instruction.to_string());
        bundle.updated_at = current_timestamp();
        Ok(())
    }

    pub fn add_exclusion(&mut self, bundle_id: &str, pattern: &str) -> Result<(), String> {
        let bundle = self
            .bundles
            .get_mut(bundle_id)
            .ok_or_else(|| format!("Bundle '{}' not found", bundle_id))?;
        if !bundle.excluded_paths.contains(&pattern.to_string()) {
            bundle.excluded_paths.push(pattern.to_string());
            bundle.updated_at = current_timestamp();
        }
        Ok(())
    }

    pub fn set_priority(&mut self, bundle_id: &str, priority: u32) -> Result<(), String> {
        let bundle = self
            .bundles
            .get_mut(bundle_id)
            .ok_or_else(|| format!("Bundle '{}' not found", bundle_id))?;
        bundle.priority = priority;
        bundle.updated_at = current_timestamp();
        self.sort_active_by_priority();
        Ok(())
    }

    pub fn set_model_preference(
        &mut self,
        bundle_id: &str,
        model: &str,
    ) -> Result<(), String> {
        let bundle = self
            .bundles
            .get_mut(bundle_id)
            .ok_or_else(|| format!("Bundle '{}' not found", bundle_id))?;
        bundle.model_preference = Some(model.to_string());
        bundle.updated_at = current_timestamp();
        Ok(())
    }

    /// Build a merged context prompt from all active bundles, ordered by priority.
    pub fn build_context_prompt(&self) -> String {
        let active = self.list_active();
        if active.is_empty() {
            return String::new();
        }

        let mut sections: Vec<String> = Vec::new();
        sections.push("## Active Context Bundles".to_string());

        for bundle in &active {
            sections.push(format!("### {} (priority {})", bundle.name, bundle.priority));

            if !bundle.description.is_empty() {
                sections.push(bundle.description.clone());
            }

            if !bundle.instructions.is_empty() {
                sections.push("**Instructions:**".to_string());
                for instr in &bundle.instructions {
                    sections.push(format!("- {}", instr));
                }
            }

            if !bundle.pinned_files.is_empty() {
                sections.push("**Pinned files:**".to_string());
                for f in &bundle.pinned_files {
                    sections.push(format!("- {}", f));
                }
            }

            if let Some(ref model) = bundle.model_preference {
                sections.push(format!("**Preferred model:** {}", model));
            }
        }

        // Merged exclusions
        let all_exclusions: Vec<&str> = active
            .iter()
            .flat_map(|b| b.excluded_paths.iter().map(|s| s.as_str()))
            .collect();
        if !all_exclusions.is_empty() {
            sections.push("### Excluded paths".to_string());
            let mut unique: Vec<&str> = all_exclusions;
            unique.sort();
            unique.dedup();
            for ex in unique {
                sections.push(format!("- {}", ex));
            }
        }

        sections.join("\n")
    }

    /// Export a bundle as a JSON string.
    pub fn export_bundle(&self, id: &str) -> Result<String, String> {
        let bundle = self
            .bundles
            .get(id)
            .ok_or_else(|| format!("Bundle '{}' not found", id))?;

        let export = BundleExport {
            version: "1.0".to_string(),
            bundle: bundle.clone(),
            exported_at: current_timestamp(),
        };

        Ok(serialize_export_json(&export))
    }

    /// Import a bundle from a JSON string. Returns the new bundle id.
    pub fn import_bundle(&mut self, json: &str) -> Result<String, String> {
        let export = deserialize_export_json(json)?;
        let new_id = generate_id(&mut self.next_id);
        let mut bundle = export.bundle;
        bundle.id = new_id.clone();
        bundle.updated_at = current_timestamp();
        self.bundles.insert(new_id.clone(), bundle);
        Ok(new_id)
    }

    /// Generate `.vibebundle.toml` content for a bundle.
    pub fn to_toml(bundle: &ContextBundle) -> String {
        let mut lines = Vec::new();
        lines.push("[bundle]".to_string());
        lines.push(format!("id = \"{}\"", bundle.id));
        lines.push(format!("name = \"{}\"", escape_toml_string(&bundle.name)));
        lines.push(format!(
            "description = \"{}\"",
            escape_toml_string(&bundle.description)
        ));
        lines.push(format!("priority = {}", bundle.priority));
        lines.push(format!("created_at = {}", bundle.created_at));
        lines.push(format!("updated_at = {}", bundle.updated_at));

        if let Some(ref model) = bundle.model_preference {
            lines.push(format!("model_preference = \"{}\"", escape_toml_string(model)));
        }

        lines.push(format!(
            "pinned_files = [{}]",
            bundle
                .pinned_files
                .iter()
                .map(|f| format!("\"{}\"", escape_toml_string(f)))
                .collect::<Vec<_>>()
                .join(", ")
        ));

        lines.push(format!(
            "instructions = [{}]",
            bundle
                .instructions
                .iter()
                .map(|i| format!("\"{}\"", escape_toml_string(i)))
                .collect::<Vec<_>>()
                .join(", ")
        ));

        lines.push(format!(
            "excluded_paths = [{}]",
            bundle
                .excluded_paths
                .iter()
                .map(|e| format!("\"{}\"", escape_toml_string(e)))
                .collect::<Vec<_>>()
                .join(", ")
        ));

        lines.push(format!(
            "tags = [{}]",
            bundle
                .tags
                .iter()
                .map(|t| format!("\"{}\"", escape_toml_string(t)))
                .collect::<Vec<_>>()
                .join(", ")
        ));

        lines.join("\n")
    }

    /// Parse a bundle from `.vibebundle.toml` content.
    pub fn from_toml(content: &str) -> Result<ContextBundle, String> {
        let get_str = |key: &str| -> Result<String, String> {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with(&format!("{} = ", key))
                    || trimmed.starts_with(&format!("{}=", key))
                {
                    // Extract value between quotes
                    if let Some(start) = trimmed.find('"') {
                        if let Some(end) = trimmed.rfind('"') {
                            if end > start {
                                return Ok(
                                    unescape_toml_string(&trimmed[start + 1..end])
                                );
                            }
                        }
                    }
                }
            }
            Err(format!("Missing key '{}'", key))
        };

        let get_u64 = |key: &str| -> Result<u64, String> {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with(&format!("{} = ", key))
                    || trimmed.starts_with(&format!("{}=", key))
                {
                    let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        return parts[1]
                            .trim()
                            .parse::<u64>()
                            .map_err(|e| format!("Invalid u64 for '{}': {}", key, e));
                    }
                }
            }
            Err(format!("Missing key '{}'", key))
        };

        let get_u32 = |key: &str| -> Result<u32, String> {
            get_u64(key).map(|v| v as u32)
        };

        let get_array = |key: &str| -> Vec<String> {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with(&format!("{} = ", key))
                    || trimmed.starts_with(&format!("{}=", key))
                {
                    if let Some(bracket_start) = trimmed.find('[') {
                        if let Some(bracket_end) = trimmed.rfind(']') {
                            let inner = &trimmed[bracket_start + 1..bracket_end];
                            if inner.trim().is_empty() {
                                return Vec::new();
                            }
                            return parse_toml_string_array(inner);
                        }
                    }
                }
            }
            Vec::new()
        };

        let id = get_str("id")?;
        let name = get_str("name")?;
        let description = get_str("description")?;
        let priority = get_u32("priority").unwrap_or(100);
        let created_at = get_u64("created_at").unwrap_or(0);
        let updated_at = get_u64("updated_at").unwrap_or(0);
        let model_preference = get_str("model_preference").ok();

        Ok(ContextBundle {
            id,
            name,
            description,
            pinned_files: get_array("pinned_files"),
            instructions: get_array("instructions"),
            excluded_paths: get_array("excluded_paths"),
            model_preference,
            priority,
            tags: get_array("tags"),
            created_at,
            updated_at,
        })
    }

    /// Search bundles by name, description, or tag.
    pub fn search_bundles(&self, query: &str) -> Vec<&ContextBundle> {
        let q = query.to_lowercase();
        self.bundles
            .values()
            .filter(|b| {
                b.name.to_lowercase().contains(&q)
                    || b.description.to_lowercase().contains(&q)
                    || b.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Check if a file path is excluded by any active bundle.
    pub fn is_file_excluded(&self, path: &str) -> bool {
        for id in &self.active_bundles {
            if let Some(bundle) = self.bundles.get(id) {
                for pattern in &bundle.excluded_paths {
                    if path_matches_pattern(path, pattern) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn sort_active_by_priority(&mut self) {
        let bundles = &self.bundles;
        self.active_bundles.sort_by(|a, b| {
            let pa = bundles.get(a).map(|b| b.priority).unwrap_or(u32::MAX);
            let pb = bundles.get(b).map(|b| b.priority).unwrap_or(u32::MAX);
            pa.cmp(&pb)
        });
    }
}

/// Simple glob-like pattern matching: supports * as wildcard segment and exact prefix.
fn path_matches_pattern(path: &str, pattern: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix('*') {
        path.starts_with(prefix)
    } else if let Some(suffix) = pattern.strip_prefix('*') {
        path.ends_with(suffix)
    } else {
        path == pattern || path.contains(pattern)
    }
}

fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn unescape_toml_string(s: &str) -> String {
    s.replace("\\\"", "\"").replace("\\\\", "\\")
}

fn parse_toml_string_array(inner: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut in_quote = false;
    let mut current = String::new();
    let mut escape = false;

    for ch in inner.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }
        match ch {
            '\\' if in_quote => {
                escape = true;
            }
            '"' => {
                if in_quote {
                    results.push(current.clone());
                    current.clear();
                }
                in_quote = !in_quote;
            }
            _ if in_quote => {
                current.push(ch);
            }
            _ => {} // skip commas, spaces outside quotes
        }
    }
    results
}

fn serialize_export_json(export: &BundleExport) -> String {
    let b = &export.bundle;
    let pinned = json_string_array(&b.pinned_files);
    let instructions = json_string_array(&b.instructions);
    let excluded = json_string_array(&b.excluded_paths);
    let tags = json_string_array(&b.tags);
    let model = match &b.model_preference {
        Some(m) => format!("\"{}\"", json_escape(m)),
        None => "null".to_string(),
    };

    format!(
        r#"{{"version":"{}","exported_at":{},"bundle":{{"id":"{}","name":"{}","description":"{}","pinned_files":{},"instructions":{},"excluded_paths":{},"model_preference":{},"priority":{},"tags":{},"created_at":{},"updated_at":{}}}}}"#,
        json_escape(&export.version),
        export.exported_at,
        json_escape(&b.id),
        json_escape(&b.name),
        json_escape(&b.description),
        pinned,
        instructions,
        excluded,
        model,
        b.priority,
        tags,
        b.created_at,
        b.updated_at
    )
}

fn deserialize_export_json(json: &str) -> Result<BundleExport, String> {
    let get_json_str = |key: &str| -> Result<String, String> {
        let search = format!("\"{}\":\"", key);
        if let Some(start) = json.find(&search) {
            let val_start = start + search.len();
            let rest = &json[val_start..];
            let mut end = 0;
            let mut escape = false;
            for ch in rest.chars() {
                if escape {
                    escape = false;
                    end += ch.len_utf8();
                    continue;
                }
                if ch == '\\' {
                    escape = true;
                    end += 1;
                    continue;
                }
                if ch == '"' {
                    break;
                }
                end += ch.len_utf8();
            }
            Ok(json_unescape(&rest[..end]))
        } else {
            Err(format!("Missing key '{}'", key))
        }
    };

    let get_json_num = |key: &str| -> Result<u64, String> {
        let search = format!("\"{}\":", key);
        if let Some(start) = json.find(&search) {
            let val_start = start + search.len();
            let rest = &json[val_start..];
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            num_str
                .parse::<u64>()
                .map_err(|e| format!("Invalid number for '{}': {}", key, e))
        } else {
            Err(format!("Missing key '{}'", key))
        }
    };

    let get_json_array = |key: &str| -> Vec<String> {
        let search = format!("\"{}\":[", key);
        if let Some(start) = json.find(&search) {
            let arr_start = start + search.len();
            let rest = &json[arr_start..];
            if let Some(end) = rest.find(']') {
                let inner = &rest[..end];
                if inner.trim().is_empty() {
                    return Vec::new();
                }
                return parse_json_string_array(inner);
            }
        }
        Vec::new()
    };

    let get_json_optional_str = |key: &str| -> Option<String> {
        let search_null = format!("\"{}\":null", key);
        if json.contains(&search_null) {
            return None;
        }
        get_json_str(key).ok()
    };

    let version = get_json_str("version")?;
    let exported_at = get_json_num("exported_at")?;
    let id = get_json_str("id")?;
    let name = get_json_str("name")?;
    let description = get_json_str("description")?;
    let priority = get_json_num("priority")? as u32;
    let created_at = get_json_num("created_at")?;
    let updated_at = get_json_num("updated_at")?;
    let model_preference = get_json_optional_str("model_preference");
    let pinned_files = get_json_array("pinned_files");
    let instructions = get_json_array("instructions");
    let excluded_paths = get_json_array("excluded_paths");
    let tags = get_json_array("tags");

    Ok(BundleExport {
        version,
        exported_at,
        bundle: ContextBundle {
            id,
            name,
            description,
            pinned_files,
            instructions,
            excluded_paths,
            model_preference,
            priority,
            tags,
            created_at,
            updated_at,
        },
    })
}

fn parse_json_string_array(inner: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut in_quote = false;
    let mut current = String::new();
    let mut escape = false;

    for ch in inner.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }
        match ch {
            '\\' if in_quote => {
                escape = true;
            }
            '"' => {
                if in_quote {
                    results.push(current.clone());
                    current.clear();
                }
                in_quote = !in_quote;
            }
            _ if in_quote => {
                current.push(ch);
            }
            _ => {}
        }
    }
    results
}

fn json_string_array(items: &[String]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let inner: Vec<String> = items.iter().map(|s| format!("\"{}\"", json_escape(s))).collect();
    format!("[{}]", inner.join(","))
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn json_unescape(s: &str) -> String {
    s.replace("\\\"", "\"").replace("\\\\", "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager() -> BundleManager {
        BundleManager::new(PathBuf::from("/tmp/bundles"), BundleConfig::default())
    }

    #[test]
    fn test_create_bundle() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("test", "A test bundle");
        assert!(mgr.get_bundle(&id).is_some());
        let b = mgr.get_bundle(&id).unwrap();
        assert_eq!(b.name, "test");
        assert_eq!(b.description, "A test bundle");
        assert_eq!(b.priority, 100);
    }

    #[test]
    fn test_create_multiple_bundles_unique_ids() {
        let mut mgr = make_manager();
        let id1 = mgr.create_bundle("a", "");
        let id2 = mgr.create_bundle("b", "");
        let id3 = mgr.create_bundle("c", "");
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_eq!(mgr.bundles.len(), 3);
    }

    #[test]
    fn test_delete_bundle() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("del", "");
        assert!(mgr.delete_bundle(&id));
        assert!(mgr.get_bundle(&id).is_none());
    }

    #[test]
    fn test_delete_nonexistent() {
        let mut mgr = make_manager();
        assert!(!mgr.delete_bundle("nope"));
    }

    #[test]
    fn test_activate_bundle() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("act", "");
        assert!(mgr.activate(&id).is_ok());
        assert_eq!(mgr.list_active().len(), 1);
    }

    #[test]
    fn test_activate_nonexistent() {
        let mut mgr = make_manager();
        assert!(mgr.activate("nope").is_err());
    }

    #[test]
    fn test_activate_idempotent() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("x", "");
        mgr.activate(&id).unwrap();
        mgr.activate(&id).unwrap(); // no error
        assert_eq!(mgr.list_active().len(), 1);
    }

    #[test]
    fn test_deactivate_bundle() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("da", "");
        mgr.activate(&id).unwrap();
        assert!(mgr.deactivate(&id).is_ok());
        assert!(mgr.list_active().is_empty());
    }

    #[test]
    fn test_deactivate_not_active() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("na", "");
        assert!(mgr.deactivate(&id).is_err());
    }

    #[test]
    fn test_deactivate_nonexistent() {
        let mut mgr = make_manager();
        assert!(mgr.deactivate("nope").is_err());
    }

    #[test]
    fn test_priority_ordering() {
        let mut mgr = make_manager();
        let id1 = mgr.create_bundle("low", "");
        let id2 = mgr.create_bundle("high", "");
        mgr.set_priority(&id1, 50).unwrap();
        mgr.set_priority(&id2, 10).unwrap();
        mgr.activate(&id1).unwrap();
        mgr.activate(&id2).unwrap();
        let active = mgr.list_active();
        assert_eq!(active[0].name, "high");
        assert_eq!(active[1].name, "low");
    }

    #[test]
    fn test_set_priority_reorders_active() {
        let mut mgr = make_manager();
        let id1 = mgr.create_bundle("a", "");
        let id2 = mgr.create_bundle("b", "");
        mgr.activate(&id1).unwrap();
        mgr.activate(&id2).unwrap();
        mgr.set_priority(&id1, 200).unwrap();
        mgr.set_priority(&id2, 1).unwrap();
        let active = mgr.list_active();
        assert_eq!(active[0].name, "b");
        assert_eq!(active[1].name, "a");
    }

    #[test]
    fn test_max_active_bundles() {
        let config = BundleConfig {
            max_active_bundles: 2,
            ..Default::default()
        };
        let mut mgr = BundleManager::new(PathBuf::from("/tmp"), config);
        let id1 = mgr.create_bundle("a", "");
        let id2 = mgr.create_bundle("b", "");
        let id3 = mgr.create_bundle("c", "");
        mgr.activate(&id1).unwrap();
        mgr.activate(&id2).unwrap();
        let err = mgr.activate(&id3).unwrap_err();
        assert!(err.contains("Maximum"));
    }

    #[test]
    fn test_add_pinned_file() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("pin", "");
        mgr.add_pinned_file(&id, "src/main.rs").unwrap();
        let b = mgr.get_bundle(&id).unwrap();
        assert_eq!(b.pinned_files, vec!["src/main.rs"]);
    }

    #[test]
    fn test_add_pinned_file_no_duplicates() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("pin", "");
        mgr.add_pinned_file(&id, "a.rs").unwrap();
        mgr.add_pinned_file(&id, "a.rs").unwrap();
        assert_eq!(mgr.get_bundle(&id).unwrap().pinned_files.len(), 1);
    }

    #[test]
    fn test_remove_pinned_file() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("pin", "");
        mgr.add_pinned_file(&id, "a.rs").unwrap();
        mgr.remove_pinned_file(&id, "a.rs").unwrap();
        assert!(mgr.get_bundle(&id).unwrap().pinned_files.is_empty());
    }

    #[test]
    fn test_remove_pinned_file_not_found() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("pin", "");
        assert!(mgr.remove_pinned_file(&id, "nope.rs").is_err());
    }

    #[test]
    fn test_add_instruction() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("inst", "");
        mgr.add_instruction(&id, "Use Rust").unwrap();
        mgr.add_instruction(&id, "Follow conventions").unwrap();
        let b = mgr.get_bundle(&id).unwrap();
        assert_eq!(b.instructions.len(), 2);
    }

    #[test]
    fn test_add_exclusion() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("excl", "");
        mgr.add_exclusion(&id, "target/*").unwrap();
        assert_eq!(mgr.get_bundle(&id).unwrap().excluded_paths, vec!["target/*"]);
    }

    #[test]
    fn test_add_exclusion_no_duplicates() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("excl", "");
        mgr.add_exclusion(&id, "*.log").unwrap();
        mgr.add_exclusion(&id, "*.log").unwrap();
        assert_eq!(mgr.get_bundle(&id).unwrap().excluded_paths.len(), 1);
    }

    #[test]
    fn test_set_model_preference() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("model", "");
        mgr.set_model_preference(&id, "claude-opus-4-20250514").unwrap();
        assert_eq!(
            mgr.get_bundle(&id).unwrap().model_preference,
            Some("claude-opus-4-20250514".to_string())
        );
    }

    #[test]
    fn test_build_context_prompt_empty() {
        let mgr = make_manager();
        assert_eq!(mgr.build_context_prompt(), "");
    }

    #[test]
    fn test_build_context_prompt_single() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("rust-project", "A Rust project");
        mgr.add_instruction(&id, "Use idiomatic Rust").unwrap();
        mgr.add_pinned_file(&id, "Cargo.toml").unwrap();
        mgr.activate(&id).unwrap();
        let prompt = mgr.build_context_prompt();
        assert!(prompt.contains("rust-project"));
        assert!(prompt.contains("Use idiomatic Rust"));
        assert!(prompt.contains("Cargo.toml"));
    }

    #[test]
    fn test_build_context_prompt_multiple_bundles_merged() {
        let mut mgr = make_manager();
        let id1 = mgr.create_bundle("backend", "Backend context");
        let id2 = mgr.create_bundle("frontend", "Frontend context");
        mgr.set_priority(&id1, 1).unwrap();
        mgr.set_priority(&id2, 2).unwrap();
        mgr.add_instruction(&id1, "Use async/await").unwrap();
        mgr.add_instruction(&id2, "Use React hooks").unwrap();
        mgr.add_exclusion(&id1, "node_modules/*").unwrap();
        mgr.add_exclusion(&id2, "target/*").unwrap();
        mgr.activate(&id1).unwrap();
        mgr.activate(&id2).unwrap();
        let prompt = mgr.build_context_prompt();
        assert!(prompt.contains("backend"));
        assert!(prompt.contains("frontend"));
        assert!(prompt.contains("Use async/await"));
        assert!(prompt.contains("Use React hooks"));
        assert!(prompt.contains("node_modules/*"));
        assert!(prompt.contains("target/*"));
        // backend should come first (priority 1 < 2)
        let backend_pos = prompt.find("backend").unwrap();
        let frontend_pos = prompt.find("frontend").unwrap();
        assert!(backend_pos < frontend_pos);
    }

    #[test]
    fn test_build_context_prompt_with_model() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("m", "");
        mgr.set_model_preference(&id, "gpt-4").unwrap();
        mgr.activate(&id).unwrap();
        let prompt = mgr.build_context_prompt();
        assert!(prompt.contains("gpt-4"));
    }

    #[test]
    fn test_toml_roundtrip() {
        let bundle = ContextBundle {
            id: "test_1".to_string(),
            name: "My Bundle".to_string(),
            description: "A test".to_string(),
            pinned_files: vec!["src/lib.rs".to_string(), "Cargo.toml".to_string()],
            instructions: vec!["Be concise".to_string()],
            excluded_paths: vec!["target/*".to_string()],
            model_preference: Some("claude-opus-4-20250514".to_string()),
            priority: 42,
            tags: vec!["rust".to_string(), "backend".to_string()],
            created_at: 1000,
            updated_at: 2000,
        };
        let toml = BundleManager::to_toml(&bundle);
        let parsed = BundleManager::from_toml(&toml).unwrap();
        assert_eq!(parsed.id, bundle.id);
        assert_eq!(parsed.name, bundle.name);
        assert_eq!(parsed.description, bundle.description);
        assert_eq!(parsed.pinned_files, bundle.pinned_files);
        assert_eq!(parsed.instructions, bundle.instructions);
        assert_eq!(parsed.excluded_paths, bundle.excluded_paths);
        assert_eq!(parsed.model_preference, bundle.model_preference);
        assert_eq!(parsed.priority, bundle.priority);
        assert_eq!(parsed.tags, bundle.tags);
        assert_eq!(parsed.created_at, bundle.created_at);
        assert_eq!(parsed.updated_at, bundle.updated_at);
    }

    #[test]
    fn test_toml_no_model_preference() {
        let bundle = ContextBundle::new("x".into(), "N".into(), "D".into());
        let toml = BundleManager::to_toml(&bundle);
        assert!(!toml.contains("model_preference"));
        let parsed = BundleManager::from_toml(&toml).unwrap();
        assert_eq!(parsed.model_preference, None);
    }

    #[test]
    fn test_export_import_json() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("exportable", "For export");
        mgr.add_pinned_file(&id, "main.rs").unwrap();
        mgr.add_instruction(&id, "Write tests").unwrap();
        mgr.add_exclusion(&id, "*.tmp").unwrap();
        mgr.set_model_preference(&id, "gemini-pro").unwrap();

        let json = mgr.export_bundle(&id).unwrap();
        assert!(json.contains("exportable"));

        let new_id = mgr.import_bundle(&json).unwrap();
        assert_ne!(new_id, id);
        let imported = mgr.get_bundle(&new_id).unwrap();
        assert_eq!(imported.name, "exportable");
        assert_eq!(imported.pinned_files, vec!["main.rs"]);
        assert_eq!(imported.instructions, vec!["Write tests"]);
        assert_eq!(imported.excluded_paths, vec!["*.tmp"]);
        assert_eq!(imported.model_preference, Some("gemini-pro".to_string()));
    }

    #[test]
    fn test_export_nonexistent() {
        let mgr = make_manager();
        assert!(mgr.export_bundle("nope").is_err());
    }

    #[test]
    fn test_import_invalid_json() {
        let mut mgr = make_manager();
        assert!(mgr.import_bundle("not json").is_err());
    }

    #[test]
    fn test_search_by_name() {
        let mut mgr = make_manager();
        mgr.create_bundle("rust-backend", "");
        mgr.create_bundle("python-ml", "");
        let results = mgr.search_bundles("rust");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "rust-backend");
    }

    #[test]
    fn test_search_by_description() {
        let mut mgr = make_manager();
        mgr.create_bundle("proj", "Machine learning project");
        let results = mgr.search_bundles("machine");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_by_tag() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("proj", "");
        mgr.get_bundle_mut(&id).unwrap().tags.push("kubernetes".to_string());
        let results = mgr.search_bundles("kubernetes");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_case_insensitive() {
        let mut mgr = make_manager();
        mgr.create_bundle("MyProject", "");
        let results = mgr.search_bundles("myproject");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_no_results() {
        let mut mgr = make_manager();
        mgr.create_bundle("test", "");
        let results = mgr.search_bundles("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_is_file_excluded_prefix() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("ex", "");
        mgr.add_exclusion(&id, "target/*").unwrap();
        mgr.activate(&id).unwrap();
        assert!(mgr.is_file_excluded("target/debug/main"));
        assert!(!mgr.is_file_excluded("src/main.rs"));
    }

    #[test]
    fn test_is_file_excluded_suffix() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("ex", "");
        mgr.add_exclusion(&id, "*.log").unwrap();
        mgr.activate(&id).unwrap();
        assert!(mgr.is_file_excluded("app.log"));
        assert!(!mgr.is_file_excluded("app.rs"));
    }

    #[test]
    fn test_is_file_excluded_across_bundles() {
        let mut mgr = make_manager();
        let id1 = mgr.create_bundle("a", "");
        let id2 = mgr.create_bundle("b", "");
        mgr.add_exclusion(&id1, "*.tmp").unwrap();
        mgr.add_exclusion(&id2, "build/*").unwrap();
        mgr.activate(&id1).unwrap();
        mgr.activate(&id2).unwrap();
        assert!(mgr.is_file_excluded("foo.tmp"));
        assert!(mgr.is_file_excluded("build/out.o"));
        assert!(!mgr.is_file_excluded("src/lib.rs"));
    }

    #[test]
    fn test_is_file_excluded_inactive_bundle() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("inactive", "");
        mgr.add_exclusion(&id, "*.log").unwrap();
        // not activated
        assert!(!mgr.is_file_excluded("app.log"));
    }

    #[test]
    fn test_list_all() {
        let mut mgr = make_manager();
        let id1 = mgr.create_bundle("alpha", "");
        let id2 = mgr.create_bundle("beta", "");
        mgr.add_pinned_file(&id1, "a.rs").unwrap();
        mgr.add_instruction(&id2, "do stuff").unwrap();
        mgr.activate(&id1).unwrap();
        let statuses = mgr.list_all();
        assert_eq!(statuses.len(), 2);
        let s1 = statuses.iter().find(|s| s.name == "alpha").unwrap();
        assert!(s1.active);
        assert_eq!(s1.pinned_count, 1);
        let s2 = statuses.iter().find(|s| s.name == "beta").unwrap();
        assert!(!s2.active);
        assert_eq!(s2.instruction_count, 1);
    }

    #[test]
    fn test_delete_active_bundle_removes_from_active() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("del", "");
        mgr.activate(&id).unwrap();
        mgr.delete_bundle(&id);
        assert!(mgr.list_active().is_empty());
        assert!(mgr.active_bundles.is_empty());
    }

    #[test]
    fn test_empty_bundle_context_prompt() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("empty", "");
        mgr.activate(&id).unwrap();
        let prompt = mgr.build_context_prompt();
        assert!(prompt.contains("empty"));
        // Should not crash with empty fields
    }

    #[test]
    fn test_get_bundle_mut() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("mut", "");
        mgr.get_bundle_mut(&id).unwrap().tags.push("test".to_string());
        assert_eq!(mgr.get_bundle(&id).unwrap().tags, vec!["test"]);
    }

    #[test]
    fn test_add_pinned_file_invalid_bundle() {
        let mut mgr = make_manager();
        assert!(mgr.add_pinned_file("nope", "a.rs").is_err());
    }

    #[test]
    fn test_add_instruction_invalid_bundle() {
        let mut mgr = make_manager();
        assert!(mgr.add_instruction("nope", "x").is_err());
    }

    #[test]
    fn test_set_priority_invalid_bundle() {
        let mut mgr = make_manager();
        assert!(mgr.set_priority("nope", 1).is_err());
    }

    #[test]
    fn test_toml_special_characters() {
        let bundle = ContextBundle {
            id: "sp".to_string(),
            name: "Bundle with \"quotes\"".to_string(),
            description: "Path: C:\\Users\\test".to_string(),
            pinned_files: vec!["file with spaces.rs".to_string()],
            instructions: vec![],
            excluded_paths: vec![],
            model_preference: None,
            priority: 100,
            tags: vec![],
            created_at: 0,
            updated_at: 0,
        };
        let toml = BundleManager::to_toml(&bundle);
        let parsed = BundleManager::from_toml(&toml).unwrap();
        assert_eq!(parsed.name, bundle.name);
        assert_eq!(parsed.description, bundle.description);
        assert_eq!(parsed.pinned_files, bundle.pinned_files);
    }

    #[test]
    fn test_bundle_config_defaults() {
        let config = BundleConfig::default();
        assert_eq!(config.max_active_bundles, 10);
        assert!(config.auto_activate_tags.is_empty());
        assert_eq!(config.default_priority, 100);
    }

    #[test]
    fn test_path_matches_pattern_exact() {
        assert!(path_matches_pattern("src/main.rs", "src/main.rs"));
        assert!(!path_matches_pattern("src/lib.rs", "src/main.rs"));
    }

    #[test]
    fn test_path_matches_pattern_contains() {
        assert!(path_matches_pattern("src/utils/helpers.rs", "utils"));
    }

    #[test]
    fn test_export_bundle_no_model() {
        let mut mgr = make_manager();
        let id = mgr.create_bundle("nomodel", "");
        let json = mgr.export_bundle(&id).unwrap();
        assert!(json.contains("null"));
        let new_id = mgr.import_bundle(&json).unwrap();
        assert_eq!(mgr.get_bundle(&new_id).unwrap().model_preference, None);
    }

    #[test]
    fn test_context_prompt_exclusions_deduped() {
        let mut mgr = make_manager();
        let id1 = mgr.create_bundle("a", "");
        let id2 = mgr.create_bundle("b", "");
        mgr.add_exclusion(&id1, "*.log").unwrap();
        mgr.add_exclusion(&id2, "*.log").unwrap();
        mgr.activate(&id1).unwrap();
        mgr.activate(&id2).unwrap();
        let prompt = mgr.build_context_prompt();
        // Count occurrences of "*.log" in the excluded paths section
        let exclusion_section = prompt.split("Excluded paths").nth(1).unwrap_or("");
        let count = exclusion_section.matches("*.log").count();
        assert_eq!(count, 1, "Duplicate exclusions should be deduped");
    }
}
