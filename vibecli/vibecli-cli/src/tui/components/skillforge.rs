//! TUI SkillForge component (gap item G2).
//!
//! Read-only browse screen for the SkillForge skill library — the daemon's
//! own client, mirroring the `GoalsComponent` shape. Pulls the catalogue
//! straight from `skillforge_index::list_skills_value()` (no HTTP
//! round-trip — same process) and the `/health`-style status block from
//! `status_value()`. The train-status pane reads `list_jobs_value()`.
//!
//! Score / train / promote stay REPL commands (`/skillforge score <name>`,
//! `/skillforge train <name>`, `/skillforge promote <name> <job>` — see
//! G1); this screen is browse-only, like the Goals screen.

use crate::skillforge_index;

#[derive(Debug, Clone)]
pub struct SkillRow {
    pub name: String,
    pub category: String,
    pub summary: String,
    pub source: String,
    /// Deterministic 0.0–1.0 trigger coverage (no LLM). `None` when the
    /// skill has never been measured.
    pub trigger_coverage: Option<f32>,
    /// LLM-judge target evolvability (0.0–1.0). `None` until SkillLens
    /// scores the skill.
    pub target_evolvability: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct TrainJobRow {
    pub id: String,
    pub skill: String,
    pub state: String,
    pub llm: String,
}

pub struct SkillforgeComponent {
    pub items: Vec<SkillRow>,
    pub selected_index: usize,
    pub jobs: Vec<TrainJobRow>,
    /// `{status, skills, cached_reports, toolchain}` footer summary.
    pub status: serde_json::Value,
    pub last_error: Option<String>,
}

impl SkillforgeComponent {
    pub fn new() -> Self {
        let mut c = Self {
            items: Vec::new(),
            selected_index: 0,
            jobs: Vec::new(),
            status: serde_json::json!({}),
            last_error: None,
        };
        c.refresh();
        c
    }

    /// Reload the catalogue + status block from the in-process
    /// `skillforge_index`. If the index hasn't been initialised yet
    /// (`Loading` / `Disabled`), kick `init_skillforge(None)` first —
    /// idempotent via its internal `OnceLock`.
    pub fn refresh(&mut self) {
        let status = skillforge_index::current_status();
        if matches!(
            status,
            skillforge_index::SkillForgeStatus::Loading
                | skillforge_index::SkillForgeStatus::Disabled
        ) {
            let _ = skillforge_index::init_skillforge(None);
        }

        self.status = skillforge_index::status_value();

        let rows_val = skillforge_index::list_skills_value();
        let arr = rows_val.as_array().cloned().unwrap_or_default();
        self.items = arr
            .into_iter()
            .map(|v| SkillRow {
                name: v["name"].as_str().unwrap_or("").to_string(),
                category: v["category"].as_str().unwrap_or("").to_string(),
                summary: v["summary"].as_str().unwrap_or("").to_string(),
                source: v["source"].as_str().unwrap_or("").to_string(),
                trigger_coverage: v["trigger_coverage"].as_f64().map(|f| f as f32),
                target_evolvability: v["target_evolvability"].as_f64().map(|f| f as f32),
            })
            .collect();
        if self.selected_index >= self.items.len() {
            self.selected_index = self.items.len().saturating_sub(1);
        }
        self.last_error = None;
    }

    /// Refresh the train-jobs pane only. Async because the job map is
    /// behind a `Mutex` (not `RwLock`); the caller drives it from the
    /// async event loop, mirroring how `/skillforge status` polls.
    pub async fn refresh_jobs(&mut self) {
        let val = skillforge_index::list_jobs_value().await;
        let arr = val.as_array().cloned().unwrap_or_default();
        self.jobs = arr
            .into_iter()
            .map(|v| TrainJobRow {
                id: v["id"].as_str().unwrap_or("").to_string(),
                skill: v["skill"].as_str().unwrap_or("").to_string(),
                state: v["state"].as_str().unwrap_or("").to_string(),
                llm: v["llm"].as_str().unwrap_or("").to_string(),
            })
            .collect();
    }

    pub fn next(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.items.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.items.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.items.len() - 1;
            }
        }
    }

    pub fn selected(&self) -> Option<&SkillRow> {
        self.items.get(self.selected_index)
    }
}

impl Default for SkillforgeComponent {
    fn default() -> Self {
        Self::new()
    }
}
