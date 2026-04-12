#![allow(dead_code)]
//! Auto-deploy — pipeline, plan, stage lifecycle, health gates, and rollback.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── DeployTarget ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeployTarget {
    DockerCompose { compose_file: String },
    Kubernetes { namespace: String, manifest_path: String },
    Serverless { provider: String, function_name: String },
    StaticHosting { bucket: String, cdn_prefix: String },
}

// ─── DeployStageKind ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeployStageKind {
    Build,
    Test,
    Package,
    Provision,
    Deploy,
    HealthCheck,
    Promote,
    Rollback,
}

// ─── StageStatus ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StageStatus {
    Pending,
    Running,
    Passed,
    Failed(String),
    Skipped,
}

// ─── DeployStage ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployStage {
    pub stage_id: String,
    pub kind: DeployStageKind,
    pub target: DeployTarget,
    pub pre_conditions: Vec<String>,
    pub post_conditions: Vec<String>,
    pub status: StageStatus,
    pub started_ms: Option<u64>,
    pub completed_ms: Option<u64>,
}

// ─── HealthCheckType ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthCheckType {
    HttpEndpoint { url: String, expected_status: u16 },
    SmokeTest { command: String },
    MetricsThreshold { query: String, threshold: f64 },
}

// ─── HealthGate ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthGate {
    pub gate_id: String,
    pub check_type: HealthCheckType,
    pub config_json: String,
    pub passed: Option<bool>,
    pub checked_at_ms: Option<u64>,
}

// ─── DeployPlan ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployPlan {
    pub plan_id: String,
    pub description: String,
    pub stages: Vec<DeployStage>,
    pub gates: Vec<HealthGate>,
    pub dry_run: bool,
    pub created_at_ms: u64,
}

impl DeployPlan {
    pub fn new(description: &str, dry_run: bool) -> Self {
        Self {
            plan_id: simple_id("plan"),
            description: description.to_string(),
            stages: Vec::new(),
            gates: Vec::new(),
            dry_run,
            created_at_ms: now_ms(),
        }
    }

    pub fn add_stage(&mut self, stage: DeployStage) {
        self.stages.push(stage);
    }

    pub fn add_gate(&mut self, gate: HealthGate) {
        self.gates.push(gate);
    }

    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Returns true when all stages are Passed or Skipped (and there is at least one stage).
    pub fn is_complete(&self) -> bool {
        !self.stages.is_empty()
            && self.stages.iter().all(|s| {
                s.status == StageStatus::Passed || s.status == StageStatus::Skipped
            })
    }

    /// Returns true when any stage has failed.
    pub fn has_failures(&self) -> bool {
        self.stages
            .iter()
            .any(|s| matches!(s.status, StageStatus::Failed(_)))
    }

    /// Returns the next stage whose status is Pending.
    pub fn next_pending_stage(&self) -> Option<&DeployStage> {
        self.stages
            .iter()
            .find(|s| s.status == StageStatus::Pending)
    }
}

// ─── DeployPipeline ──────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct DeployPipeline {
    plans: HashMap<String, DeployPlan>,
}

impl DeployPipeline {
    pub fn new() -> Self {
        Self {
            plans: HashMap::new(),
        }
    }

    /// Creates a new deploy plan (one stage per target) and returns the `plan_id`.
    pub fn create_plan(
        &mut self,
        description: &str,
        targets: Vec<DeployTarget>,
        dry_run: bool,
    ) -> String {
        let mut plan = DeployPlan::new(description, dry_run);
        for (i, target) in targets.into_iter().enumerate() {
            plan.add_stage(DeployStage {
                stage_id: format!("{}-stage-{}", plan.plan_id, i),
                kind: DeployStageKind::Deploy,
                target,
                pre_conditions: vec![],
                post_conditions: vec![],
                status: StageStatus::Pending,
                started_ms: None,
                completed_ms: None,
            });
        }
        let id = plan.plan_id.clone();
        self.plans.insert(id.clone(), plan);
        id
    }

    pub fn get_plan(&self, plan_id: &str) -> Option<&DeployPlan> {
        self.plans.get(plan_id)
    }

    pub fn start_stage(&mut self, plan_id: &str, stage_id: &str) -> Result<(), String> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| format!("plan '{}' not found", plan_id))?;
        let stage = plan
            .stages
            .iter_mut()
            .find(|s| s.stage_id == stage_id)
            .ok_or_else(|| format!("stage '{}' not found", stage_id))?;
        stage.status = StageStatus::Running;
        stage.started_ms = Some(now_ms());
        Ok(())
    }

    pub fn pass_stage(&mut self, plan_id: &str, stage_id: &str) -> Result<(), String> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| format!("plan '{}' not found", plan_id))?;
        let stage = plan
            .stages
            .iter_mut()
            .find(|s| s.stage_id == stage_id)
            .ok_or_else(|| format!("stage '{}' not found", stage_id))?;
        stage.status = StageStatus::Passed;
        stage.completed_ms = Some(now_ms());
        Ok(())
    }

    pub fn fail_stage(
        &mut self,
        plan_id: &str,
        stage_id: &str,
        error: &str,
    ) -> Result<(), String> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| format!("plan '{}' not found", plan_id))?;
        let stage = plan
            .stages
            .iter_mut()
            .find(|s| s.stage_id == stage_id)
            .ok_or_else(|| format!("stage '{}' not found", stage_id))?;
        stage.status = StageStatus::Failed(error.to_string());
        stage.completed_ms = Some(now_ms());
        Ok(())
    }

    /// Adds a Rollback stage for each failed stage and returns the count rolled back.
    pub fn trigger_rollback(&mut self, plan_id: &str) -> Result<usize, String> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| format!("plan '{}' not found", plan_id))?;

        let failed_stages: Vec<(String, DeployTarget)> = plan
            .stages
            .iter()
            .filter(|s| matches!(s.status, StageStatus::Failed(_)))
            .map(|s| (s.stage_id.clone(), s.target.clone()))
            .collect();

        let count = failed_stages.len();
        for (id, target) in failed_stages {
            plan.stages.push(DeployStage {
                stage_id: format!("{}-rollback", id),
                kind: DeployStageKind::Rollback,
                target,
                pre_conditions: vec![],
                post_conditions: vec![],
                status: StageStatus::Pending,
                started_ms: None,
                completed_ms: None,
            });
        }
        Ok(count)
    }

    pub fn plan_count(&self) -> usize {
        self.plans.len()
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn simple_id(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{}-{:x}", prefix, t)
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn docker_target() -> DeployTarget {
        DeployTarget::DockerCompose {
            compose_file: "docker-compose.yml".into(),
        }
    }

    fn k8s_target() -> DeployTarget {
        DeployTarget::Kubernetes {
            namespace: "default".into(),
            manifest_path: "k8s/deployment.yaml".into(),
        }
    }

    fn make_stage(id: &str, kind: DeployStageKind, status: StageStatus) -> DeployStage {
        DeployStage {
            stage_id: id.to_string(),
            kind,
            target: docker_target(),
            pre_conditions: vec![],
            post_conditions: vec![],
            status,
            started_ms: None,
            completed_ms: None,
        }
    }

    // ── DeployPlan ────────────────────────────────────────────────────────

    #[test]
    fn test_plan_new_empty() {
        let plan = DeployPlan::new("deploy v1.2", false);
        assert_eq!(plan.stage_count(), 0);
        assert!(!plan.is_complete());
        assert!(!plan.has_failures());
        assert!(plan.next_pending_stage().is_none());
    }

    #[test]
    fn test_plan_description() {
        let plan = DeployPlan::new("deploy to prod", false);
        assert_eq!(plan.description, "deploy to prod");
    }

    #[test]
    fn test_plan_dry_run_flag() {
        let plan = DeployPlan::new("test", true);
        assert!(plan.dry_run);
    }

    #[test]
    fn test_plan_id_not_empty() {
        let plan = DeployPlan::new("x", false);
        assert!(!plan.plan_id.is_empty());
    }

    #[test]
    fn test_plan_add_stage() {
        let mut plan = DeployPlan::new("deploy", false);
        plan.add_stage(make_stage("s1", DeployStageKind::Build, StageStatus::Pending));
        assert_eq!(plan.stage_count(), 1);
    }

    #[test]
    fn test_plan_add_gate() {
        let mut plan = DeployPlan::new("deploy", false);
        plan.add_gate(HealthGate {
            gate_id: "g1".into(),
            check_type: HealthCheckType::SmokeTest { command: "curl".into() },
            config_json: "{}".into(),
            passed: None,
            checked_at_ms: None,
        });
        assert_eq!(plan.gates.len(), 1);
    }

    #[test]
    fn test_plan_is_complete_all_passed() {
        let mut plan = DeployPlan::new("deploy", false);
        plan.add_stage(make_stage("s1", DeployStageKind::Build, StageStatus::Passed));
        plan.add_stage(make_stage("s2", DeployStageKind::Deploy, StageStatus::Passed));
        assert!(plan.is_complete());
    }

    #[test]
    fn test_plan_is_complete_with_skipped() {
        let mut plan = DeployPlan::new("deploy", false);
        plan.add_stage(make_stage("s1", DeployStageKind::Build, StageStatus::Passed));
        plan.add_stage(make_stage("s2", DeployStageKind::Test, StageStatus::Skipped));
        assert!(plan.is_complete());
    }

    #[test]
    fn test_plan_not_complete_with_pending() {
        let mut plan = DeployPlan::new("deploy", false);
        plan.add_stage(make_stage("s1", DeployStageKind::Build, StageStatus::Passed));
        plan.add_stage(make_stage("s2", DeployStageKind::Deploy, StageStatus::Pending));
        assert!(!plan.is_complete());
    }

    #[test]
    fn test_plan_not_complete_with_running() {
        let mut plan = DeployPlan::new("deploy", false);
        plan.add_stage(make_stage("s1", DeployStageKind::Deploy, StageStatus::Running));
        assert!(!plan.is_complete());
    }

    #[test]
    fn test_plan_has_failures_true() {
        let mut plan = DeployPlan::new("deploy", false);
        plan.add_stage(make_stage(
            "s1",
            DeployStageKind::Build,
            StageStatus::Failed("compilation error".into()),
        ));
        assert!(plan.has_failures());
    }

    #[test]
    fn test_plan_has_failures_false() {
        let mut plan = DeployPlan::new("deploy", false);
        plan.add_stage(make_stage("s1", DeployStageKind::Build, StageStatus::Passed));
        assert!(!plan.has_failures());
    }

    #[test]
    fn test_plan_next_pending_stage() {
        let mut plan = DeployPlan::new("deploy", false);
        plan.add_stage(make_stage("s1", DeployStageKind::Build, StageStatus::Passed));
        plan.add_stage(make_stage("s2", DeployStageKind::Deploy, StageStatus::Pending));
        let next = plan.next_pending_stage();
        assert!(next.is_some());
        assert_eq!(next.unwrap().stage_id, "s2");
    }

    #[test]
    fn test_plan_next_pending_none_when_all_done() {
        let mut plan = DeployPlan::new("deploy", false);
        plan.add_stage(make_stage("s1", DeployStageKind::Build, StageStatus::Passed));
        assert!(plan.next_pending_stage().is_none());
    }

    // ── DeployPipeline ────────────────────────────────────────────────────

    #[test]
    fn test_pipeline_new_empty() {
        let p = DeployPipeline::new();
        assert_eq!(p.plan_count(), 0);
    }

    #[test]
    fn test_pipeline_create_plan_returns_id() {
        let mut p = DeployPipeline::new();
        let id = p.create_plan("deploy", vec![docker_target()], false);
        assert!(!id.is_empty());
    }

    #[test]
    fn test_pipeline_create_plan_stored() {
        let mut p = DeployPipeline::new();
        let id = p.create_plan("deploy", vec![docker_target()], false);
        assert!(p.get_plan(&id).is_some());
    }

    #[test]
    fn test_pipeline_plan_count_increases() {
        let mut p = DeployPipeline::new();
        p.create_plan("d1", vec![docker_target()], false);
        p.create_plan("d2", vec![k8s_target()], false);
        assert_eq!(p.plan_count(), 2);
    }

    #[test]
    fn test_pipeline_get_plan_not_found() {
        let p = DeployPipeline::new();
        assert!(p.get_plan("nonexistent").is_none());
    }

    #[test]
    fn test_pipeline_start_stage() {
        let mut p = DeployPipeline::new();
        let plan_id = p.create_plan("deploy", vec![docker_target()], false);
        let stage_id = p.get_plan(&plan_id).unwrap().stages[0].stage_id.clone();
        let result = p.start_stage(&plan_id, &stage_id);
        assert!(result.is_ok());
        let stage = &p.get_plan(&plan_id).unwrap().stages[0];
        assert_eq!(stage.status, StageStatus::Running);
    }

    #[test]
    fn test_pipeline_start_stage_invalid_plan() {
        let mut p = DeployPipeline::new();
        let result = p.start_stage("bad-plan", "s1");
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_start_stage_invalid_stage() {
        let mut p = DeployPipeline::new();
        let plan_id = p.create_plan("deploy", vec![docker_target()], false);
        let result = p.start_stage(&plan_id, "bad-stage");
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_pass_stage() {
        let mut p = DeployPipeline::new();
        let plan_id = p.create_plan("deploy", vec![docker_target()], false);
        let stage_id = p.get_plan(&plan_id).unwrap().stages[0].stage_id.clone();
        p.start_stage(&plan_id, &stage_id).unwrap();
        p.pass_stage(&plan_id, &stage_id).unwrap();
        let stage = &p.get_plan(&plan_id).unwrap().stages[0];
        assert_eq!(stage.status, StageStatus::Passed);
    }

    #[test]
    fn test_pipeline_fail_stage() {
        let mut p = DeployPipeline::new();
        let plan_id = p.create_plan("deploy", vec![docker_target()], false);
        let stage_id = p.get_plan(&plan_id).unwrap().stages[0].stage_id.clone();
        p.start_stage(&plan_id, &stage_id).unwrap();
        p.fail_stage(&plan_id, &stage_id, "timeout").unwrap();
        let stage = &p.get_plan(&plan_id).unwrap().stages[0];
        assert!(matches!(&stage.status, StageStatus::Failed(e) if e == "timeout"));
    }

    #[test]
    fn test_pipeline_trigger_rollback_count() {
        let mut p = DeployPipeline::new();
        let plan_id = p.create_plan("deploy", vec![docker_target(), k8s_target()], false);
        let stage_ids: Vec<String> = p
            .get_plan(&plan_id)
            .unwrap()
            .stages
            .iter()
            .map(|s| s.stage_id.clone())
            .collect();
        for sid in &stage_ids {
            p.start_stage(&plan_id, sid).unwrap();
            p.fail_stage(&plan_id, sid, "error").unwrap();
        }
        let rolled_back = p.trigger_rollback(&plan_id).unwrap();
        assert_eq!(rolled_back, 2);
    }

    #[test]
    fn test_pipeline_rollback_adds_stages() {
        let mut p = DeployPipeline::new();
        let plan_id = p.create_plan("deploy", vec![docker_target()], false);
        let stage_id = p.get_plan(&plan_id).unwrap().stages[0].stage_id.clone();
        p.start_stage(&plan_id, &stage_id).unwrap();
        p.fail_stage(&plan_id, &stage_id, "error").unwrap();
        let count_before = p.get_plan(&plan_id).unwrap().stage_count();
        p.trigger_rollback(&plan_id).unwrap();
        let count_after = p.get_plan(&plan_id).unwrap().stage_count();
        assert!(count_after > count_before);
    }

    #[test]
    fn test_pipeline_rollback_no_failures() {
        let mut p = DeployPipeline::new();
        let plan_id = p.create_plan("deploy", vec![docker_target()], false);
        let rolled_back = p.trigger_rollback(&plan_id).unwrap();
        assert_eq!(rolled_back, 0);
    }

    #[test]
    fn test_pipeline_pass_stage_invalid_plan() {
        let mut p = DeployPipeline::new();
        let result = p.pass_stage("bad-plan", "s1");
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_fail_stage_invalid_plan() {
        let mut p = DeployPipeline::new();
        let result = p.fail_stage("bad-plan", "s1", "err");
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_rollback_invalid_plan() {
        let mut p = DeployPipeline::new();
        let result = p.trigger_rollback("bad-plan");
        assert!(result.is_err());
    }

    #[test]
    fn test_plan_started_ms_set_on_start() {
        let mut p = DeployPipeline::new();
        let plan_id = p.create_plan("deploy", vec![docker_target()], false);
        let stage_id = p.get_plan(&plan_id).unwrap().stages[0].stage_id.clone();
        p.start_stage(&plan_id, &stage_id).unwrap();
        assert!(p.get_plan(&plan_id).unwrap().stages[0].started_ms.is_some());
    }

    #[test]
    fn test_plan_completed_ms_set_on_pass() {
        let mut p = DeployPipeline::new();
        let plan_id = p.create_plan("deploy", vec![docker_target()], false);
        let stage_id = p.get_plan(&plan_id).unwrap().stages[0].stage_id.clone();
        p.start_stage(&plan_id, &stage_id).unwrap();
        p.pass_stage(&plan_id, &stage_id).unwrap();
        assert!(p.get_plan(&plan_id).unwrap().stages[0].completed_ms.is_some());
    }

    #[test]
    fn test_is_complete_empty_plan_false() {
        let plan = DeployPlan::new("empty", false);
        assert!(!plan.is_complete());
    }

    #[test]
    fn test_serverless_target() {
        let t = DeployTarget::Serverless {
            provider: "aws".into(),
            function_name: "my-fn".into(),
        };
        assert!(matches!(t, DeployTarget::Serverless { .. }));
    }

    #[test]
    fn test_static_hosting_target() {
        let t = DeployTarget::StaticHosting {
            bucket: "my-bucket".into(),
            cdn_prefix: "https://cdn.example.com".into(),
        };
        assert!(matches!(t, DeployTarget::StaticHosting { .. }));
    }
}
