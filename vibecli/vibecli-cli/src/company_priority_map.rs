#![allow(dead_code)]
//! Priority map for company programs.
//!
//! Maps each business program (e.g. Revenue, EA, Legal) to a urgency level
//! (0=P0 critical, 1=P1 high, 2=P2 medium, 3=P3 low) and optional routing rules.
//! Stored as a single JSON blob in a SQLite table for simplicity.

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

// ── Data structs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramEntry {
    pub program: String,
    /// 0=P0 critical, 1=P1 high, 2=P2 medium, 3=P3 low
    pub urgency: u8,
    pub routing_rules: Vec<String>,
}

// ── PriorityMapStore ──────────────────────────────────────────────────────────

pub struct PriorityMapStore {
    conn: Connection,
}

impl PriorityMapStore {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.ensure_schema()?;
        Ok(store)
    }

    pub fn ensure_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS priority_map (
                id   INTEGER PRIMARY KEY,
                data TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    pub fn get(&self) -> Result<Vec<ProgramEntry>> {
        let result: rusqlite::Result<String> = self.conn.query_row(
            "SELECT data FROM priority_map WHERE id = 1",
            [],
            |row| row.get(0),
        );
        match result {
            Ok(s) => Ok(serde_json::from_str(&s).unwrap_or_else(|_| Self::defaults())),
            Err(_) => Ok(Self::defaults()),
        }
    }

    pub fn set(&self, map: &[ProgramEntry]) -> Result<()> {
        let data = serde_json::to_string(map)?;
        self.conn.execute(
            "INSERT INTO priority_map (id, data) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET data = excluded.data",
            params![data],
        )?;
        Ok(())
    }

    fn defaults() -> Vec<ProgramEntry> {
        let programs = ["Revenue", "EA", "Legal", "BizDev", "Marketing", "Product", "Personal"];
        programs
            .iter()
            .map(|&p| ProgramEntry {
                program: p.to_string(),
                urgency: 2, // P2 medium
                routing_rules: vec![],
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store() -> PriorityMapStore {
        let conn = Connection::open_in_memory().unwrap();
        let store = PriorityMapStore { conn };
        store.ensure_schema().unwrap();
        store
    }

    #[test]
    fn given_empty_store_when_get_then_returns_seven_defaults() {
        let store = make_store();
        let map = store.get().unwrap();
        assert_eq!(map.len(), 7);
        assert!(map.iter().all(|e| e.urgency == 2));
    }

    #[test]
    fn given_default_programs_when_checked_then_all_seven_present() {
        let store = make_store();
        let map = store.get().unwrap();
        let names: Vec<&str> = map.iter().map(|e| e.program.as_str()).collect();
        assert!(names.contains(&"Revenue"));
        assert!(names.contains(&"Legal"));
        assert!(names.contains(&"Personal"));
    }

    #[test]
    fn given_custom_map_when_set_then_get_returns_it() {
        let store = make_store();
        let entries = vec![
            ProgramEntry { program: "Revenue".to_string(), urgency: 0, routing_rules: vec!["ceo".to_string()] },
            ProgramEntry { program: "Legal".to_string(), urgency: 1, routing_rules: vec![] },
        ];
        store.set(&entries).unwrap();
        let got = store.get().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].urgency, 0);
        assert_eq!(got[1].program, "Legal");
    }

    #[test]
    fn given_map_when_updated_then_returns_new_data() {
        let store = make_store();
        let v1 = vec![ProgramEntry { program: "P1".to_string(), urgency: 3, routing_rules: vec![] }];
        store.set(&v1).unwrap();
        let v2 = vec![ProgramEntry { program: "P1".to_string(), urgency: 0, routing_rules: vec!["urgent".to_string()] }];
        store.set(&v2).unwrap();
        let got = store.get().unwrap();
        assert_eq!(got[0].urgency, 0);
        assert!(got[0].routing_rules.contains(&"urgent".to_string()));
    }
}
