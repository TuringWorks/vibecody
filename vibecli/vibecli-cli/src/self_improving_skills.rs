#![allow(dead_code)]
//! Self-Improving Skills — closed-loop learning that tracks skill activations,
//! measures outcomes, and proposes (or auto-applies) refined skill files.
//!
//! Loop:
//!   1. Skill fires on a task → `record_activation`
//!   2. User accepts / rejects / corrects the response → `record_outcome`
//!   3. Engine periodically calls `compute_metrics` and `propose_evolutions`
//!   4. High-confidence evolutions are auto-written; others await human review
//!   5. Sessions with no matching skill → `extract_new_skill` drafts a new .md

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ─── Constants ────────────────────────────────────────────────────────────────

const MIN_ACTIVATIONS_FOR_METRICS: u64 = 5;
const AUTO_EVOLVE_MIN_SUCCESS_RATE: f32 = 0.80; // promote triggers at ≥80 %
const PROPOSE_EVOLVE_MAX_SUCCESS_RATE: f32 = 0.55; // flag for review at ≤55 %
const PRUNE_MAX_SUCCESS_RATE: f32 = 0.25; // prune candidate at ≤25 %
const PRUNE_MIN_ACTIVATIONS: u64 = 10;
const NEW_SKILL_MIN_PATTERN_FREQ: usize = 3;

// ─── Storage types ────────────────────────────────────────────────────────────

/// One recorded activation of a named skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillActivation {
    pub id: String,
    pub skill_name: String,
    pub task_text: String,
    pub triggered_by: String, // the trigger word that matched
    pub session_id: String,
    pub timestamp: u64,
    /// Filled in by `record_outcome`.
    pub outcome: Option<ActivationOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActivationOutcome {
    Accepted,
    Rejected,
    Corrected { correction_summary: String },
    Ignored,
}

/// Aggregated per-skill statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetrics {
    pub skill_name: String,
    pub total_activations: u64,
    pub accepted: u64,
    pub rejected: u64,
    pub corrected: u64,
    pub ignored: u64,
    pub success_rate: f32, // (accepted) / (accepted + rejected + corrected)
    pub last_activated: u64,
    pub health: SkillHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillHealth {
    Thriving,   // success_rate >= 0.80
    Healthy,    // 0.60 <= success_rate < 0.80
    Struggling, // 0.40 <= success_rate < 0.60
    Critical,   // success_rate < 0.40
    Insufficient, // not enough data yet
}

/// A proposed change to an existing skill or a new skill draft.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEvolution {
    pub id: String,
    pub kind: EvolutionKind,
    pub skill_name: String,
    pub rationale: String,
    pub proposed_content: String,
    pub confidence: f32,       // 0.0–1.0
    pub auto_applicable: bool, // true if confidence >= threshold
    pub created_at: u64,
    pub applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvolutionKind {
    RefineTriggers,   // keep content, improve trigger words
    RefineContent,    // keep triggers, improve instructions
    AddExample,       // add a usage example to the body
    NewSkill,         // brand-new skill draft from observed patterns
    Prune,            // mark as deprecated / remove
}

/// Summary returned to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImprovingStatus {
    pub total_activations: u64,
    pub skills_tracked: usize,
    pub thriving: usize,
    pub struggling: usize,
    pub critical: usize,
    pub evolutions_pending: usize,
    pub evolutions_applied: usize,
    pub new_skills_drafted: usize,
}

// ─── Storage file ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SkillStore {
    activations: Vec<SkillActivation>,
    evolutions: Vec<SkillEvolution>,
}

impl SkillStore {
    fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).unwrap_or_default();
        std::fs::write(path, json)
    }
}

// ─── Engine ───────────────────────────────────────────────────────────────────

pub struct SelfImprovingSkillsEngine {
    store_path: PathBuf,
    skills_dir: PathBuf, // where .md skill files live
}

impl SelfImprovingSkillsEngine {
    pub fn new(workspace_root: &Path) -> Self {
        let base = workspace_root.join(".vibecli");
        Self {
            store_path: base.join("skill_learning.json"),
            skills_dir: base.join("skills"),
        }
    }

    pub fn global(home: &Path) -> Self {
        let base = home.join(".vibecli");
        Self {
            store_path: base.join("skill_learning.json"),
            skills_dir: base.join("skills"),
        }
    }

    fn load(&self) -> SkillStore {
        SkillStore::load(&self.store_path)
    }

    fn save(&self, store: &SkillStore) {
        let _ = store.save(&self.store_path);
    }

    // ── Record ────────────────────────────────────────────────────────────────

    pub fn record_activation(
        &self,
        skill_name: &str,
        task_text: &str,
        triggered_by: &str,
        session_id: &str,
    ) -> String {
        let id = new_id();
        let activation = SkillActivation {
            id: id.clone(),
            skill_name: skill_name.to_string(),
            task_text: task_text.chars().take(300).collect(),
            triggered_by: triggered_by.to_string(),
            session_id: session_id.to_string(),
            timestamp: unix_ts(),
            outcome: None,
        };
        let mut store = self.load();
        store.activations.push(activation);
        self.save(&store);
        id
    }

    pub fn record_outcome(&self, activation_id: &str, outcome: ActivationOutcome) -> bool {
        let mut store = self.load();
        if let Some(a) = store.activations.iter_mut().find(|a| a.id == activation_id) {
            a.outcome = Some(outcome);
            self.save(&store);
            return true;
        }
        false
    }

    pub fn record_session_outcome(
        &self,
        session_id: &str,
        accepted: bool,
        correction: Option<String>,
    ) {
        let outcome = if accepted {
            ActivationOutcome::Accepted
        } else if let Some(c) = correction {
            ActivationOutcome::Corrected { correction_summary: c.chars().take(200).collect() }
        } else {
            ActivationOutcome::Rejected
        };

        let mut store = self.load();
        for a in store.activations.iter_mut() {
            if a.session_id == session_id && a.outcome.is_none() {
                a.outcome = Some(outcome.clone());
            }
        }
        self.save(&store);
    }

    // ── Metrics ───────────────────────────────────────────────────────────────

    pub fn compute_metrics(&self) -> Vec<SkillMetrics> {
        let store = self.load();
        let mut map: HashMap<String, (u64, u64, u64, u64, u64, u64)> = HashMap::new();
        // (total, accepted, rejected, corrected, ignored, last_ts)

        for a in &store.activations {
            let e = map.entry(a.skill_name.clone()).or_insert((0, 0, 0, 0, 0, 0));
            e.0 += 1;
            e.5 = e.5.max(a.timestamp);
            match &a.outcome {
                Some(ActivationOutcome::Accepted) => e.1 += 1,
                Some(ActivationOutcome::Rejected) => e.2 += 1,
                Some(ActivationOutcome::Corrected { .. }) => e.3 += 1,
                Some(ActivationOutcome::Ignored) => e.4 += 1,
                None => {}
            }
        }

        map.into_iter()
            .map(|(name, (total, acc, rej, cor, ign, last_ts))| {
                let judged = acc + rej + cor;
                let success_rate = if judged == 0 { 0.5 } else { acc as f32 / judged as f32 };
                let health = if total < MIN_ACTIVATIONS_FOR_METRICS {
                    SkillHealth::Insufficient
                } else if success_rate >= 0.80 {
                    SkillHealth::Thriving
                } else if success_rate >= 0.60 {
                    SkillHealth::Healthy
                } else if success_rate >= 0.40 {
                    SkillHealth::Struggling
                } else {
                    SkillHealth::Critical
                };
                SkillMetrics {
                    skill_name: name,
                    total_activations: total,
                    accepted: acc,
                    rejected: rej,
                    corrected: cor,
                    ignored: ign,
                    success_rate,
                    last_activated: last_ts,
                    health,
                }
            })
            .collect()
    }

    // ── Propose evolutions ────────────────────────────────────────────────────

    pub fn propose_evolutions(&self) -> Vec<SkillEvolution> {
        let metrics = self.compute_metrics();
        let mut store = self.load();
        let existing_names: std::collections::HashSet<String> =
            store.evolutions.iter().map(|e| e.skill_name.clone()).collect();
        let mut new_evolutions: Vec<SkillEvolution> = Vec::new();

        for m in &metrics {
            if m.total_activations < MIN_ACTIVATIONS_FOR_METRICS {
                continue;
            }
            if existing_names.contains(&m.skill_name) {
                continue; // already has a pending evolution
            }

            if m.success_rate <= PROPOSE_EVOLVE_MAX_SUCCESS_RATE {
                // Skill is struggling — propose trigger refinement
                let ev = SkillEvolution {
                    id: new_id(),
                    kind: EvolutionKind::RefineTriggers,
                    skill_name: m.skill_name.clone(),
                    rationale: format!(
                        "Success rate is {:.0}% over {} activations ({} rejected, {} corrected). \
                         Review and refine trigger keywords to better target relevant tasks.",
                        m.success_rate * 100.0,
                        m.total_activations,
                        m.rejected,
                        m.corrected,
                    ),
                    proposed_content: self
                        .draft_trigger_refinement(&m.skill_name, &store.activations),
                    confidence: 1.0 - m.success_rate,
                    auto_applicable: false,
                    created_at: unix_ts(),
                    applied: false,
                };
                new_evolutions.push(ev);
            } else if m.success_rate >= AUTO_EVOLVE_MIN_SUCCESS_RATE && m.corrected > 0 {
                // Skill works but has corrections — add examples from corrections
                let ev = SkillEvolution {
                    id: new_id(),
                    kind: EvolutionKind::AddExample,
                    skill_name: m.skill_name.clone(),
                    rationale: format!(
                        "Skill succeeds {:.0}% of the time but has {} corrections. \
                         Incorporating correction patterns as examples improves future accuracy.",
                        m.success_rate * 100.0,
                        m.corrected,
                    ),
                    proposed_content: self
                        .draft_example_addition(&m.skill_name, &store.activations),
                    confidence: m.success_rate * 0.9,
                    auto_applicable: m.success_rate >= 0.90,
                    created_at: unix_ts(),
                    applied: false,
                };
                new_evolutions.push(ev);
            }

            if m.success_rate <= PRUNE_MAX_SUCCESS_RATE
                && m.total_activations >= PRUNE_MIN_ACTIVATIONS
            {
                let ev = SkillEvolution {
                    id: new_id(),
                    kind: EvolutionKind::Prune,
                    skill_name: m.skill_name.clone(),
                    rationale: format!(
                        "Only {:.0}% success over {} activations. Consider retiring or rewriting \
                         this skill entirely.",
                        m.success_rate * 100.0,
                        m.total_activations,
                    ),
                    proposed_content: String::new(),
                    confidence: 1.0 - m.success_rate,
                    auto_applicable: false,
                    created_at: unix_ts(),
                    applied: false,
                };
                new_evolutions.push(ev);
            }
        }

        store.evolutions.extend(new_evolutions);
        self.save(&store);
        store.evolutions.iter().filter(|e| !e.applied).cloned().collect()
    }

    // ── Apply an evolution ────────────────────────────────────────────────────

    pub fn apply_evolution(&self, evolution_id: &str) -> Result<String, String> {
        let mut store = self.load();

        // Scope the mutable borrow so it is released before store is read via save()
        let (kind, skill_name, proposed_content) = {
            let ev = store
                .evolutions
                .iter_mut()
                .find(|e| e.id == evolution_id)
                .ok_or_else(|| format!("Evolution {evolution_id} not found"))?;

            if ev.applied {
                return Err("Already applied".to_string());
            }

            let data = (ev.kind.clone(), ev.skill_name.clone(), ev.proposed_content.clone());
            ev.applied = true;
            data
            // ev drops here, releasing the mutable borrow on store
        };

        match kind {
            EvolutionKind::Prune => {
                let path = self.skill_path(&skill_name);
                if path.exists() {
                    let deprecated = path.with_extension("md.deprecated");
                    std::fs::rename(&path, &deprecated).map_err(|e| e.to_string())?;
                }
                self.save(&store);
                Ok(format!("Skill '{skill_name}' deprecated."))
            }
            _ => {
                if proposed_content.is_empty() {
                    return Err("No content to apply".to_string());
                }
                let path = self.skill_path(&skill_name);
                std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
                std::fs::write(&path, &proposed_content).map_err(|e| e.to_string())?;
                self.save(&store);
                Ok(format!("Skill '{skill_name}' updated at {path:?}"))
            }
        }
    }

    // ── Auto-create from a successful session ─────────────────────────────────

    pub fn extract_new_skill(
        &self,
        task_text: &str,
        response_text: &str,
        session_id: &str,
    ) -> Option<SkillEvolution> {
        let triggers = extract_skill_triggers(task_text, response_text);
        if triggers.len() < 2 {
            return None;
        }

        let name_slug = triggers[0]
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric(), "-");
        let trigger_list = triggers
            .iter()
            .map(|t| format!("\"{}\"", t))
            .collect::<Vec<_>>()
            .join(", ");

        let summary: String = response_text.lines().take(5).collect::<Vec<_>>().join("\n");
        let content = format!(
            "---\ntriggers: [{trigger_list}]\ncategory: learned\nversion: \"1.0.0\"\n---\n\n\
             # {name}\n\n> Auto-generated from session `{session_id}`.\n\n\
             ## Pattern\n\n{summary}\n\n## Notes\n\n- Review and refine this skill before relying on it in production.\n",
            name = triggers[0],
        );

        let ev = SkillEvolution {
            id: new_id(),
            kind: EvolutionKind::NewSkill,
            skill_name: name_slug.clone(),
            rationale: format!(
                "Session '{session_id}' produced an accepted response that matched no existing skill. \
                 Extracted triggers: {trigger_list}."
            ),
            proposed_content: content,
            confidence: 0.6,
            auto_applicable: false,
            created_at: unix_ts(),
            applied: false,
        };

        let mut store = self.load();
        store.evolutions.push(ev.clone());
        self.save(&store);
        Some(ev)
    }

    // ── Prune candidates ──────────────────────────────────────────────────────

    pub fn prune_candidates(&self) -> Vec<SkillMetrics> {
        self.compute_metrics()
            .into_iter()
            .filter(|m| {
                m.total_activations >= PRUNE_MIN_ACTIVATIONS
                    && m.success_rate <= PRUNE_MAX_SUCCESS_RATE
            })
            .collect()
    }

    // ── Status summary ────────────────────────────────────────────────────────

    pub fn status(&self) -> SelfImprovingStatus {
        let store = self.load();
        let metrics = self.compute_metrics();
        SelfImprovingStatus {
            total_activations: store.activations.len() as u64,
            skills_tracked: metrics.len(),
            thriving: metrics.iter().filter(|m| m.health == SkillHealth::Thriving).count(),
            struggling: metrics.iter().filter(|m| m.health == SkillHealth::Struggling).count(),
            critical: metrics.iter().filter(|m| m.health == SkillHealth::Critical).count(),
            evolutions_pending: store.evolutions.iter().filter(|e| !e.applied).count(),
            evolutions_applied: store.evolutions.iter().filter(|e| e.applied).count(),
            new_skills_drafted: store
                .evolutions
                .iter()
                .filter(|e| e.kind == EvolutionKind::NewSkill && !e.applied)
                .count(),
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn skill_path(&self, name: &str) -> PathBuf {
        let filename = format!("{}.md", name.replace(' ', "-").to_lowercase());
        self.skills_dir.join(filename)
    }

    fn draft_trigger_refinement(
        &self,
        skill_name: &str,
        activations: &[SkillActivation],
    ) -> String {
        // Collect the task texts for this skill's rejected/corrected activations
        let bad_tasks: Vec<&str> = activations
            .iter()
            .filter(|a| a.skill_name == skill_name)
            .filter(|a| matches!(a.outcome, Some(ActivationOutcome::Rejected) | Some(ActivationOutcome::Corrected { .. })))
            .map(|a| a.task_text.as_str())
            .take(5)
            .collect();

        let note = if bad_tasks.is_empty() {
            String::new()
        } else {
            format!(
                "\n\n<!-- Poorly-matched tasks (review and adjust triggers):\n{}\n-->",
                bad_tasks.join("\n---\n")
            )
        };

        // Return the current skill file if it exists, appended with the review note
        let path = self.skill_path(skill_name);
        let current = std::fs::read_to_string(&path).unwrap_or_else(|_| {
            format!(
                "---\ntriggers: [\"TODO — refine these\"]\ncategory: learned\n---\n\n# {skill_name}\n\nTODO: update content.\n"
            )
        });
        format!("{current}{note}")
    }

    fn draft_example_addition(&self, skill_name: &str, activations: &[SkillActivation]) -> String {
        let examples: Vec<String> = activations
            .iter()
            .filter(|a| {
                a.skill_name == skill_name
                    && matches!(a.outcome, Some(ActivationOutcome::Corrected { .. }))
            })
            .take(3)
            .map(|a| {
                let correction = match &a.outcome {
                    Some(ActivationOutcome::Corrected { correction_summary }) => {
                        correction_summary.as_str()
                    }
                    _ => "",
                };
                format!("- Task: `{}`\n  Correction: {correction}", &a.task_text)
            })
            .collect();

        let path = self.skill_path(skill_name);
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        if examples.is_empty() {
            return current;
        }
        format!(
            "{current}\n## Learned Examples\n\n{}\n",
            examples.join("\n")
        )
    }
}

// ─── Free functions ───────────────────────────────────────────────────────────

/// Extract candidate trigger keywords from task + response text.
fn extract_skill_triggers(task: &str, response: &str) -> Vec<String> {
    let combined = format!("{task} {response}");
    let mut freq: HashMap<String, usize> = HashMap::new();

    // Simple tokeniser: split on non-alphanumeric, keep tokens >= 4 chars
    let stopwords = ["this", "that", "with", "from", "have", "will", "your",
                     "the", "and", "for", "are", "not", "was", "but", "use",
                     "you", "can", "all", "has", "been", "they", "when", "also",
                     "into", "more", "then", "than", "just", "like", "would"];
    for word in combined.split(|c: char| !c.is_alphanumeric()) {
        let w = word.to_lowercase();
        if w.len() >= 4 && !stopwords.contains(&w.as_str()) {
            *freq.entry(w).or_insert(0) += 1;
        }
    }

    let mut sorted: Vec<(String, usize)> = freq.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted
        .into_iter()
        .filter(|(_, c)| *c >= NEW_SKILL_MIN_PATTERN_FREQ)
        .take(6)
        .map(|(w, _)| w)
        .collect()
}

fn unix_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn new_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let tid = format!("{:?}", std::thread::current().id())
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(6)
        .collect::<String>();
    format!("sis-{ts:08x}-{tid}")
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_engine() -> (SelfImprovingSkillsEngine, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let engine = SelfImprovingSkillsEngine::new(dir.path());
        (engine, dir)
    }

    #[test]
    fn test_record_and_outcome() {
        let (engine, _dir) = tmp_engine();
        let id = engine.record_activation("rust-safety", "fix unsafe block", "unsafe", "sess1");
        assert!(!id.is_empty());
        let ok = engine.record_outcome(&id, ActivationOutcome::Accepted);
        assert!(ok);
        let metrics = engine.compute_metrics();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].accepted, 1);
    }

    #[test]
    fn test_session_outcome() {
        let (engine, _dir) = tmp_engine();
        engine.record_activation("rust-safety", "task text", "rust", "sess42");
        engine.record_session_outcome("sess42", true, None);
        let metrics = engine.compute_metrics();
        assert_eq!(metrics[0].accepted, 1);
    }

    #[test]
    fn test_health_insufficient() {
        let (engine, _dir) = tmp_engine();
        engine.record_activation("x", "t", "x", "s1");
        let metrics = engine.compute_metrics();
        assert_eq!(metrics[0].health, SkillHealth::Insufficient);
    }

    #[test]
    fn test_prune_candidate() {
        let (engine, _dir) = tmp_engine();
        for i in 0..12u32 {
            let id = engine.record_activation("bad-skill", "task", "trigger", &format!("s{i}"));
            engine.record_outcome(&id, ActivationOutcome::Rejected);
        }
        let prune = engine.prune_candidates();
        assert_eq!(prune.len(), 1);
        assert_eq!(prune[0].skill_name, "bad-skill");
    }

    #[test]
    fn test_extract_new_skill() {
        let (engine, _dir) = tmp_engine();
        let task = "implement async tokio runtime executor pattern";
        let resp = "use tokio runtime executor async pattern for task";
        let ev = engine.extract_new_skill(task, resp, "sess99");
        assert!(ev.is_some());
        let ev = ev.unwrap();
        assert_eq!(ev.kind, EvolutionKind::NewSkill);
        assert!(!ev.proposed_content.is_empty());
    }

    #[test]
    fn test_status() {
        let (engine, _dir) = tmp_engine();
        let s = engine.status();
        assert_eq!(s.total_activations, 0);
        assert_eq!(s.evolutions_pending, 0);
    }

    #[test]
    fn test_propose_evolutions_struggling() {
        let (engine, _dir) = tmp_engine();
        for i in 0..8u32 {
            let id = engine.record_activation("flaky", "some task", "flaky", &format!("s{i}"));
            let outcome = if i < 2 { ActivationOutcome::Accepted } else { ActivationOutcome::Rejected };
            engine.record_outcome(&id, outcome);
        }
        let evs = engine.propose_evolutions();
        assert!(!evs.is_empty());
        assert!(evs.iter().any(|e| e.kind == EvolutionKind::RefineTriggers || e.kind == EvolutionKind::Prune));
    }

    #[test]
    fn test_apply_prune_evolution() {
        let (engine, dir) = tmp_engine();
        // Create a dummy skill file
        let skills_dir = dir.path().join(".vibecli").join("skills");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(skills_dir.join("dead-skill.md"), "# dead skill").unwrap();

        for i in 0..12u32 {
            let id = engine.record_activation("dead-skill", "t", "dead", &format!("s{i}"));
            engine.record_outcome(&id, ActivationOutcome::Rejected);
        }
        let evs = engine.propose_evolutions();
        let prune_ev = evs.iter().find(|e| e.kind == EvolutionKind::Prune).unwrap();
        let result = engine.apply_evolution(&prune_ev.id);
        assert!(result.is_ok());
        assert!(!skills_dir.join("dead-skill.md").exists());
    }
}
