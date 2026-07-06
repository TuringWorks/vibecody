//! Optional SQLite persistence for trajectories (feature = "sqlite").
//!
//! A standalone, unencrypted store — the vibecli bridge is responsible for
//! placing the DB inside the workspace and never in a production secrets DB.
//! Tests use [`TrajectoryStore::open_in_memory`] so they never touch disk.

use std::path::Path;

use rusqlite::Connection;

use crate::model::trajectory::Trajectory;

/// A SQLite-backed trajectory store.
pub struct TrajectoryStore {
    conn: Connection,
}

impl TrajectoryStore {
    /// Open (creating if needed) a store at `path`.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        Self::init(Connection::open(path)?)
    }

    /// An ephemeral in-memory store (tests, dry runs).
    pub fn open_in_memory() -> anyhow::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> anyhow::Result<Self> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS trajectories (
                 id      TEXT PRIMARY KEY,
                 task_id TEXT NOT NULL,
                 json    TEXT NOT NULL
             );",
        )?;
        Ok(Self { conn })
    }

    /// Insert or replace a trajectory.
    pub fn put(&self, t: &Trajectory) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO trajectories (id, task_id, json) VALUES (?1, ?2, ?3)",
            rusqlite::params![t.id, t.task_id, serde_json::to_string(t)?],
        )?;
        Ok(())
    }

    /// Bulk insert/replace.
    pub fn put_all(&self, ts: &[Trajectory]) -> anyhow::Result<()> {
        for t in ts {
            self.put(t)?;
        }
        Ok(())
    }

    /// Fetch one trajectory by id.
    pub fn get(&self, id: &str) -> anyhow::Result<Option<Trajectory>> {
        let mut stmt = self
            .conn
            .prepare("SELECT json FROM trajectories WHERE id = ?1")?;
        let mut rows = stmt.query([id])?;
        match rows.next()? {
            Some(row) => {
                let json: String = row.get(0)?;
                Ok(Some(serde_json::from_str(&json)?))
            }
            None => Ok(None),
        }
    }

    /// All trajectories, ordered by id.
    pub fn all(&self) -> anyhow::Result<Vec<Trajectory>> {
        let mut stmt = self
            .conn
            .prepare("SELECT json FROM trajectories ORDER BY id")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(serde_json::from_str(&r?)?);
        }
        Ok(out)
    }

    /// Row count.
    pub fn len(&self) -> anyhow::Result<usize> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM trajectories", [], |r| {
                r.get::<_, i64>(0)
            })? as usize)
    }

    pub fn is_empty(&self) -> anyhow::Result<bool> {
        Ok(self.len()? == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::trajectory::Outcome;

    fn traj(id: &str) -> Trajectory {
        Trajectory {
            id: id.to_string(),
            task_id: "t".to_string(),
            steps: vec![],
            outcome: Outcome::Success,
            score: None,
            meta: serde_json::Value::Null,
        }
    }

    #[test]
    fn put_get_roundtrip() {
        let store = TrajectoryStore::open_in_memory().unwrap();
        assert!(store.is_empty().unwrap());
        store.put_all(&[traj("a"), traj("b")]).unwrap();
        assert_eq!(store.len().unwrap(), 2);
        let got = store.get("a").unwrap().unwrap();
        assert_eq!(got.task_id, "t");
        assert!(got.is_success());
        assert_eq!(store.all().unwrap().len(), 2);
        assert!(store.get("missing").unwrap().is_none());
    }
}
