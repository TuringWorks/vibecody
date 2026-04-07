#![allow(dead_code)]
//! Workspace-level configuration for company orchestration.
//!
//! Stores named configuration values (owner name, business name, timezone, etc.)
//! in a key-value SQLite table. Supports template substitution for prompts and
//! messages that reference `{{owner_name}}`, `{{business_name}}`, etc.

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

// ── Data structs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceConfig {
    pub owner_name: String,
    pub assistant_name: String,
    pub business_name: String,
    pub timezone: String,
    pub target_market: String,
    pub primary_update_channel: String,
    pub assistant_email: String,
    pub work_email: String,
}

// ── WorkspaceConfigStore ──────────────────────────────────────────────────────

pub struct WorkspaceConfigStore {
    conn: Connection,
}

impl WorkspaceConfigStore {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.ensure_schema()?;
        Ok(store)
    }

    pub fn ensure_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS workspace_config (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    fn get_key(&self, key: &str) -> Result<String> {
        let result: rusqlite::Result<String> = self.conn.query_row(
            "SELECT value FROM workspace_config WHERE key = ?1",
            params![key],
            |row| row.get(0),
        );
        Ok(result.unwrap_or_default())
    }

    fn set_key(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO workspace_config (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get(&self) -> Result<WorkspaceConfig> {
        Ok(WorkspaceConfig {
            owner_name: self.get_key("owner_name")?,
            assistant_name: self.get_key("assistant_name")?,
            business_name: self.get_key("business_name")?,
            timezone: self.get_key("timezone")?,
            target_market: self.get_key("target_market")?,
            primary_update_channel: self.get_key("primary_update_channel")?,
            assistant_email: self.get_key("assistant_email")?,
            work_email: self.get_key("work_email")?,
        })
    }

    pub fn set(&self, cfg: &WorkspaceConfig) -> Result<()> {
        self.set_key("owner_name", &cfg.owner_name)?;
        self.set_key("assistant_name", &cfg.assistant_name)?;
        self.set_key("business_name", &cfg.business_name)?;
        self.set_key("timezone", &cfg.timezone)?;
        self.set_key("target_market", &cfg.target_market)?;
        self.set_key("primary_update_channel", &cfg.primary_update_channel)?;
        self.set_key("assistant_email", &cfg.assistant_email)?;
        self.set_key("work_email", &cfg.work_email)?;
        Ok(())
    }

    /// Replace `{{key}}` placeholders in `template` with values from the config.
    pub fn apply_substitutions(&self, template: &str) -> Result<String> {
        let cfg = self.get()?;
        let result = template
            .replace("{{owner_name}}", &cfg.owner_name)
            .replace("{{assistant_name}}", &cfg.assistant_name)
            .replace("{{business_name}}", &cfg.business_name)
            .replace("{{timezone}}", &cfg.timezone)
            .replace("{{target_market}}", &cfg.target_market)
            .replace("{{primary_update_channel}}", &cfg.primary_update_channel)
            .replace("{{assistant_email}}", &cfg.assistant_email)
            .replace("{{work_email}}", &cfg.work_email);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store() -> WorkspaceConfigStore {
        let conn = Connection::open_in_memory().unwrap();
        let store = WorkspaceConfigStore { conn };
        store.ensure_schema().unwrap();
        store
    }

    #[test]
    fn given_empty_store_when_get_then_returns_defaults() {
        let store = make_store();
        let cfg = store.get().unwrap();
        assert_eq!(cfg.owner_name, "");
        assert_eq!(cfg.business_name, "");
    }

    #[test]
    fn given_config_when_set_then_get_returns_same_values() {
        let store = make_store();
        let cfg = WorkspaceConfig {
            owner_name: "Alice".to_string(),
            assistant_name: "Vibe".to_string(),
            business_name: "Acme Corp".to_string(),
            timezone: "America/New_York".to_string(),
            target_market: "SMB".to_string(),
            primary_update_channel: "email".to_string(),
            assistant_email: "vibe@acme.com".to_string(),
            work_email: "alice@acme.com".to_string(),
        };
        store.set(&cfg).unwrap();
        let got = store.get().unwrap();
        assert_eq!(got.owner_name, "Alice");
        assert_eq!(got.business_name, "Acme Corp");
        assert_eq!(got.timezone, "America/New_York");
    }

    #[test]
    fn given_template_when_apply_substitutions_then_placeholders_replaced() {
        let store = make_store();
        let cfg = WorkspaceConfig {
            owner_name: "Bob".to_string(),
            business_name: "TechCo".to_string(),
            ..Default::default()
        };
        store.set(&cfg).unwrap();
        let result = store.apply_substitutions("Hello {{owner_name}} from {{business_name}}!").unwrap();
        assert_eq!(result, "Hello Bob from TechCo!");
    }

    #[test]
    fn given_set_when_updated_then_returns_new_value() {
        let store = make_store();
        let cfg1 = WorkspaceConfig { owner_name: "Old".to_string(), ..Default::default() };
        store.set(&cfg1).unwrap();
        let cfg2 = WorkspaceConfig { owner_name: "New".to_string(), ..Default::default() };
        store.set(&cfg2).unwrap();
        let got = store.get().unwrap();
        assert_eq!(got.owner_name, "New");
    }
}
