#![allow(dead_code)]
//! Client-engagement lifecycle: the spine that ties VibeCody's tooling to a
//! four-phase delivery contract.
//!
//! VibeCody already ships the individual capabilities — architecture specs,
//! threat models, CI gates, SLO dashboards, runbooks, cost estimation. What it
//! lacked was an object that says *which* of them a given client engagement has
//! actually produced, and *whether* the phase they belong to may be exited.
//! This module is that object.
//!
//! Four phases, in order:
//!
//! 1. **Discover & Assess** — map the current state, separate the problems
//!    worth solving from the noise.
//! 2. **Prove** — a narrow pilot on the client's data and infrastructure,
//!    judged against criteria agreed *before* the pilot starts.
//! 3. **Build & Harden** — production delivery with IaC, CI/CD, observability
//!    and a security review that happens *during* the build.
//! 4. **Operate & Transfer** — managed operation for as long as it is useful,
//!    then a deliberate handover.
//!
//! ## Honesty rules this module enforces
//!
//! Borrowed wholesale from the evaluation harness, because an engagement report
//! is a measurement and the same failure modes apply:
//!
//! * **Five verdicts, kept strictly apart.** A gate is `Pass`, `Fail`,
//!   `Waived`, `Pending` (scheduled, not yet judged) or `NotMeasured` (nobody
//!   has looked). `NotMeasured` is never silently folded into `Fail`, and never
//!   into `Pass`.
//! * **A phase with no deliverables has no completion percentage.** It reports
//!   `None`, rendered `n/a` — never `0%`, which reads as "measured, and bad".
//! * **Unmeasured blocks.** A gate nobody judged does not let a phase exit; it
//!   appears in `blockers` with its own reason, distinct from a gate that was
//!   judged and failed.
//! * **Absent stays absent.** `Phase::Discover` carries no published cadence in
//!   the engagement model, so `cadence()` returns `None` for it rather than a
//!   plausible-looking "2–4 weeks" that nobody agreed to.

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Milliseconds since the epoch, as `i64` because that is what SQLite stores.
/// The structs expose `u64`; the cast happens once, at the row boundary.
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ── Phase ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Discover,
    Prove,
    Build,
    Operate,
}

impl Phase {
    pub const ALL: [Phase; 4] = [Phase::Discover, Phase::Prove, Phase::Build, Phase::Operate];

    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Discover => "discover",
            Phase::Prove => "prove",
            Phase::Build => "build",
            Phase::Operate => "operate",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Phase> {
        match s {
            "discover" => Some(Phase::Discover),
            "prove" => Some(Phase::Prove),
            "build" => Some(Phase::Build),
            "operate" => Some(Phase::Operate),
            _ => None,
        }
    }

    /// Display title as it appears in the engagement model.
    pub fn title(&self) -> &'static str {
        match self {
            Phase::Discover => "Discover & Assess",
            Phase::Prove => "Prove",
            Phase::Build => "Build & Harden",
            Phase::Operate => "Operate & Transfer",
        }
    }

    /// Published cadence, where one exists.
    ///
    /// `Discover` deliberately returns `None`: the engagement model publishes a
    /// duration for the other three phases and not for this one. Inventing
    /// "2–4 weeks" here would put a commitment in front of a client that nobody
    /// made.
    pub fn cadence(&self) -> Option<&'static str> {
        match self {
            Phase::Discover => None,
            Phase::Prove => Some("4–8 weeks"),
            Phase::Build => Some("Scope-dependent"),
            Phase::Operate => Some("Ongoing or fixed"),
        }
    }

    pub fn purpose(&self) -> &'static str {
        match self {
            Phase::Discover => {
                "Map the current state, interview the people who operate it, and separate the \
                 problems worth solving from the noise. No engagement starts with a solution \
                 already chosen."
            }
            Phase::Prove => {
                "A narrow pilot on your data, on your infrastructure, judged against success \
                 criteria agreed up front. If the approach is wrong, this is where we find out \
                 cheaply."
            }
            Phase::Build => {
                "Production delivery alongside your engineers — infrastructure as code, CI/CD, \
                 test coverage, observability, and a security review that happens during the \
                 build rather than after it."
            }
            Phase::Operate => {
                "Managed operation for as long as it is useful, and a deliberate handover after \
                 that. The goal is for your team to own the system, not to keep us on the invoice."
            }
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Phase::Discover => 0,
            Phase::Prove => 1,
            Phase::Build => 2,
            Phase::Operate => 3,
        }
    }

    pub fn next(&self) -> Option<Phase> {
        match self {
            Phase::Discover => Some(Phase::Prove),
            Phase::Prove => Some(Phase::Build),
            Phase::Build => Some(Phase::Operate),
            Phase::Operate => None,
        }
    }
}

// ── Status enums ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngagementStatus {
    Draft,
    Active,
    Paused,
    Closed,
}

impl EngagementStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Closed => "closed",
        }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "paused" => Self::Paused,
            "closed" => Self::Closed,
            _ => Self::Draft,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliverableStatus {
    NotStarted,
    InProgress,
    /// Produced and self-reviewed, but the client has not signed it off.
    Ready,
    /// The client accepted it. Only this state closes a deliverable.
    Accepted,
    /// Agreed as out of scope for this engagement. Excluded from the
    /// completion denominator and reported separately, never as "done".
    Waived,
}

impl DeliverableStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::Ready => "ready",
            Self::Accepted => "accepted",
            Self::Waived => "waived",
        }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "in_progress" => Self::InProgress,
            "ready" => Self::Ready,
            "accepted" => Self::Accepted,
            "waived" => Self::Waived,
            _ => Self::NotStarted,
        }
    }
}

/// Gate verdicts. Deliberately five, deliberately disjoint.
///
/// The two that are easy to conflate carry the whole value of the type:
/// `Pending` means a judgement is scheduled and has not happened yet;
/// `NotMeasured` means nobody has arranged to judge it at all. Collapsing
/// either into `Fail` produces a report that says the work is broken when the
/// truth is that nobody looked, and collapsing either into `Pass` ships an
/// unmeasured claim to a client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateVerdict {
    NotMeasured,
    Pending,
    Pass,
    Fail,
    Waived,
}

impl GateVerdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotMeasured => "not_measured",
            Self::Pending => "pending",
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Waived => "waived",
        }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "pass" => Self::Pass,
            "fail" => Self::Fail,
            "waived" => Self::Waived,
            _ => Self::NotMeasured,
        }
    }

    /// Whether this verdict permits a phase to close. Only two do.
    pub fn satisfies_gate(&self) -> bool {
        matches!(self, Self::Pass | Self::Waived)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// A file in the workspace (path relative to the engagement root).
    File,
    /// An external link — dashboard, ticket, PR, deployed environment.
    Url,
    /// A recorded VibeCody run: eval run id, job id, workflow run.
    Run,
    /// A measured number with its unit, captured at a point in time.
    Metric,
    /// A human note. Carries no automatic weight.
    Note,
}

impl EvidenceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Url => "url",
            Self::Run => "run",
            Self::Metric => "metric",
            Self::Note => "note",
        }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "file" => Self::File,
            "url" => Self::Url,
            "run" => Self::Run,
            "metric" => Self::Metric,
            _ => Self::Note,
        }
    }
}

// ── Data structs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Engagement {
    pub id: String,
    pub name: String,
    pub client: String,
    /// Absolute path to the workspace this engagement is delivered from, when
    /// one is bound. `None` for an engagement scoped to work not yet in a repo.
    pub workspace_path: Option<String>,
    pub status: EngagementStatus,
    pub current_phase: Phase,
    pub summary: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deliverable {
    pub id: String,
    pub engagement_id: String,
    pub phase: Phase,
    /// Stable slug — the join key to the template and to tooling.
    pub key: String,
    pub title: String,
    pub description: String,
    pub status: DeliverableStatus,
    pub owner: Option<String>,
    /// Which VibeCody surface produces this. See [`TOOLING`].
    pub tool_hint: Option<String>,
    pub notes: String,
    pub evidence_count: usize,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub deliverable_id: String,
    pub kind: EvidenceKind,
    pub label: String,
    /// Path, URL, run id, or the metric's value — interpreted per `kind`.
    pub reference: String,
    pub captured_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate {
    pub id: String,
    pub engagement_id: String,
    pub phase: Phase,
    pub title: String,
    /// What must be true. Agreed before the phase starts.
    pub criterion: String,
    /// How it will be judged — the measurement procedure, named up front so the
    /// verdict cannot be argued into existence afterwards.
    pub measurement: String,
    /// The observed result. `None` until someone measures; never defaulted.
    pub observed: Option<String>,
    pub verdict: GateVerdict,
    pub rationale: String,
    pub decided_by: Option<String>,
    pub decided_at: Option<u64>,
    pub created_at: u64,
    pub updated_at: u64,
}

// ── Readiness reporting ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverableTally {
    pub total: usize,
    pub not_started: usize,
    pub in_progress: usize,
    pub ready: usize,
    pub accepted: usize,
    pub waived: usize,
    /// Deliverables in `Ready` or `Accepted` with zero attached evidence.
    /// A deliverable claimed done with nothing behind it is the single most
    /// common way an engagement report lies.
    pub claimed_without_evidence: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateTally {
    pub total: usize,
    pub not_measured: usize,
    pub pending: usize,
    pub pass: usize,
    pub fail: usize,
    pub waived: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerKind {
    DeliverableOutstanding,
    DeliverableWithoutEvidence,
    GateFailed,
    GatePending,
    GateNotMeasured,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blocker {
    pub kind: BlockerKind,
    /// Deliverable key or gate id.
    pub subject: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseReadiness {
    pub phase: Phase,
    pub title: String,
    pub cadence: Option<String>,
    pub deliverables: DeliverableTally,
    pub gates: GateTally,
    /// Fraction of in-scope (non-waived) deliverables accepted, 0.0–1.0.
    ///
    /// `None` when there are no in-scope deliverables — "n/a", not "0%".
    pub completion: Option<f64>,
    pub blockers: Vec<Blocker>,
    /// True only when every in-scope deliverable is accepted with evidence and
    /// every gate is `Pass` or `Waived`.
    pub can_exit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementReport {
    pub engagement: Engagement,
    pub phases: Vec<PhaseReadiness>,
    pub generated_at: u64,
}

// ── Templates ─────────────────────────────────────────────────────────────────

/// One row of the deliverable template: phase, stable key, title, description,
/// and the VibeCody surface that produces it.
pub struct DeliverableTemplate {
    pub phase: Phase,
    pub key: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// The panel, CLI command, or daemon route that produces this artifact.
    /// This is what makes the engagement navigable: every promised deliverable
    /// points at the tool that makes it, so no phase requires the operator to
    /// remember which of 300 panels applies.
    pub tool_hint: &'static str,
}

macro_rules! d {
    ($phase:expr, $key:literal, $title:literal, $desc:literal, $tool:literal) => {
        DeliverableTemplate {
            phase: $phase,
            key: $key,
            title: $title,
            description: $desc,
            tool_hint: $tool,
        }
    };
}

/// The deliverables promised by the engagement model, one row per commitment.
///
/// Every bullet the client was shown appears here. Adding a promise to the
/// sales model without adding it here is how an engagement quietly ships less
/// than it sold; `template_covers_every_phase` fails if a phase empties out.
pub static TEMPLATE: &[DeliverableTemplate] = &[
    // ── 01 Discover & Assess ──────────────────────────────────────────────
    d!(
        Phase::Discover,
        "current-state-architecture-map",
        "Current-state architecture map",
        "A diagram and written model of the system as it is today, not as it was designed. \
         Derived from the running code and confirmed with the people who operate it.",
        "ArchitectureSpecPanel"
    ),
    d!(
        Phase::Discover,
        "system-inventory",
        "System inventory",
        "Every deployed service, datastore, scheduled job, and third-party dependency, with \
         owner and criticality. The list nobody has in one place.",
        "DepsPanel"
    ),
    d!(
        Phase::Discover,
        "stakeholder-interviews",
        "Operator interviews",
        "Notes from the people who run the system day to day. The problems worth solving are \
         usually named here and nowhere in the codebase.",
        "DocumentIngestPanel"
    ),
    d!(
        Phase::Discover,
        "functional-requirements",
        "Functional requirements",
        "What the system must do, stated so that a disagreement about scope can be settled by \
         reading it.",
        "SpecPanel"
    ),
    d!(
        Phase::Discover,
        "non-functional-requirements",
        "Non-functional requirements",
        "Latency, availability, throughput, retention, residency, and cost ceilings — each with \
         a number and the source of that number.",
        "SpecPanel"
    ),
    d!(
        Phase::Discover,
        "risk-register",
        "Risk register",
        "Each risk with likelihood, impact, owner, and the trigger that would tell us it is \
         happening. A risk with no owner is a wish.",
        "SecurityPosturePanel"
    ),
    d!(
        Phase::Discover,
        "dependency-register",
        "Dependency register",
        "Upstream and downstream dependencies including the human ones — teams, vendors, and \
         approvals that gate delivery.",
        "SmartDepsPanel"
    ),
    d!(
        Phase::Discover,
        "tech-debt-register",
        "Technical-debt register",
        "Debt itemised with the interest it charges: what it costs per change, not just that it \
         exists.",
        "CodeMetricsPanel"
    ),
    d!(
        Phase::Discover,
        "prioritized-roadmap",
        "Prioritized roadmap with effort and sequencing",
        "Ordered work with effort estimates and the dependencies that force the order. The \
         output the engagement is actually judged on.",
        "PlanDocumentPanel"
    ),
    // ── 02 Prove ──────────────────────────────────────────────────────────
    d!(
        Phase::Prove,
        "success-criteria",
        "Agreed success criteria",
        "The criteria the pilot will be judged against, recorded as gates before the pilot \
         starts. Criteria written afterwards measure nothing.",
        "EngagementPanel"
    ),
    d!(
        Phase::Prove,
        "pilot-deployment",
        "Working pilot deployed in your environment",
        "Running on the client's infrastructure, against the client's data. A pilot on our \
         laptop proves our laptop works.",
        "DeployPanel"
    ),
    d!(
        Phase::Prove,
        "measured-results",
        "Measured results against agreed criteria",
        "Every criterion judged, including the ones that failed and the ones nobody could \
         measure. Unmeasured is reported as unmeasured.",
        "ArenaPanel"
    ),
    d!(
        Phase::Prove,
        "cost-model",
        "Cost model at target production volume",
        "Extrapolated to production volume with the assumptions stated, so the client can \
         challenge the assumptions rather than the total.",
        "CostPanel"
    ),
    d!(
        Phase::Prove,
        "go-no-go",
        "Go / no-go recommendation with alternatives",
        "A recommendation that is allowed to be 'no', plus the alternatives considered and why \
         they lost.",
        "CounselPanel"
    ),
    // ── 03 Build & Harden ─────────────────────────────────────────────────
    d!(
        Phase::Build,
        "production-system",
        "Production system",
        "The delivered system itself, built alongside the client's engineers rather than handed \
         over at the end.",
        "BuildPanel"
    ),
    d!(
        Phase::Build,
        "infrastructure-as-code",
        "Infrastructure as code",
        "The environment reproducible from a repository. If it cannot be rebuilt from source, \
         it is not delivered.",
        "K8sPanel"
    ),
    d!(
        Phase::Build,
        "ci-cd-pipelines",
        "Automated CI/CD pipelines",
        "Build, test, and deploy automated end to end, with the gates that stop a bad change.",
        "CicdPanel"
    ),
    d!(
        Phase::Build,
        "test-coverage",
        "Test coverage",
        "Coverage with the caveat attached: which paths are covered, which are not, and which \
         are covered only nominally.",
        "CoveragePanel"
    ),
    d!(
        Phase::Build,
        "observability",
        "Observability",
        "Logs, metrics, and traces sufficient to diagnose an incident without shell access to \
         production.",
        "TraceDashboard"
    ),
    d!(
        Phase::Build,
        "alerting-and-slos",
        "Alerting and SLO definitions",
        "SLOs with error budgets, and alerts that fire on symptoms the user feels rather than \
         on causes engineers find interesting.",
        "HealthMonitorPanel"
    ),
    d!(
        Phase::Build,
        "threat-model",
        "Threat model",
        "Produced during the build, not after it, so its findings can still change the design.",
        "SecurityReviewPanel"
    ),
    d!(
        Phase::Build,
        "compliance-evidence",
        "Compliance evidence",
        "The artifacts an auditor will ask for, collected as the build produces them.",
        "CompliancePanel"
    ),
    d!(
        Phase::Build,
        "architecture-decision-records",
        "Architecture decision records",
        "Each significant decision with its context, the options rejected, and the consequences \
         accepted.",
        "ArchitectureSpecPanel"
    ),
    // ── 04 Operate & Transfer ─────────────────────────────────────────────
    d!(
        Phase::Operate,
        "runbooks",
        "Runbooks",
        "One per alert, each ending in a resolved state. An alert without a runbook is a page \
         to somebody who does not know what to do.",
        "PlanDocumentPanel"
    ),
    d!(
        Phase::Operate,
        "escalation-paths",
        "Escalation paths",
        "Who is called, in what order, with what authority to act — tested, not just written.",
        "TeamGovernancePanel"
    ),
    d!(
        Phase::Operate,
        "oncall-handbook",
        "On-call handbook",
        "What on-call means here: rotation, response expectations, and what a responder is \
         permitted to do at 3am without waking anyone else.",
        "PlanDocumentPanel"
    ),
    d!(
        Phase::Operate,
        "sla-managed-operation",
        "SLA-backed managed operation",
        "The operating agreement, where the client wants one — scope, response times, and what \
         is explicitly not covered.",
        "CompanyDashboardPanel"
    ),
    d!(
        Phase::Operate,
        "enablement-sessions",
        "Enablement sessions",
        "Sessions delivered to the client's engineers, with attendance and the material handed \
         over.",
        "TeamOnboardingPanel"
    ),
    d!(
        Phase::Operate,
        "pairing-log",
        "Pairing with your engineers",
        "A record of paired work: which client engineers touched which parts of the system, so \
         ownership can be evidenced rather than asserted.",
        "CollabPanel"
    ),
    d!(
        Phase::Operate,
        "exit-plan",
        "Documented exit plan",
        "How this engagement ends: access revocation, knowledge transfer checkpoints, and the \
         date after which the client's team owns the system outright.",
        "CompanyPortabilityPanel"
    ),
];

/// One row of the gate template.
pub struct GateTemplate {
    pub phase: Phase,
    pub title: &'static str,
    pub criterion: &'static str,
    pub measurement: &'static str,
}

macro_rules! g {
    ($phase:expr, $title:literal, $criterion:literal, $measurement:literal) => {
        GateTemplate {
            phase: $phase,
            title: $title,
            criterion: $criterion,
            measurement: $measurement,
        }
    };
}

/// Default phase gates, seeded as `NotMeasured`.
///
/// They start unmeasured on purpose. A freshly created engagement that reported
/// its gates as passing would be asserting a fact about the world nobody has
/// checked, which is exactly the failure this module exists to prevent.
pub static GATE_TEMPLATE: &[GateTemplate] = &[
    g!(
        Phase::Discover,
        "Inventory is complete",
        "Every deployed system, datastore, and scheduled job appears in the inventory with a \
         named owner.",
        "Reconcile the inventory against the cloud account, the CI project list, and the \
         on-call rotation. Any system in one and not the other is a miss."
    ),
    g!(
        Phase::Discover,
        "Requirements are testable",
        "Every non-functional requirement carries a number and the source of that number.",
        "Read each NFR and attempt to write the assertion that would falsify it. An NFR that \
         cannot be falsified is not a requirement."
    ),
    g!(
        Phase::Discover,
        "Roadmap is sequenced by dependency",
        "Every roadmap item's position is explained by a dependency, a risk, or a stated \
         business date — not by preference.",
        "Walk the roadmap and name the constraint fixing each item's position. Unexplained \
         ordering fails."
    ),
    g!(
        Phase::Prove,
        "Criteria agreed before the pilot ran",
        "The success criteria were recorded and accepted before the pilot began.",
        "Compare each gate's creation timestamp against the pilot's start. A criterion created \
         after the start is evidence of the outcome, not a test of it."
    ),
    g!(
        Phase::Prove,
        "Pilot ran on client infrastructure and client data",
        "The pilot executed in the client's environment against the client's data, not a \
         synthetic sample.",
        "Deployment target and dataset provenance recorded as evidence on the pilot \
         deliverable."
    ),
    g!(
        Phase::Prove,
        "Every criterion has a verdict",
        "Each agreed criterion is judged pass, fail, or explicitly not-measured with a reason.",
        "Count gates in this phase with verdict `not_measured`. A criterion silently dropped is \
         a failed measurement, not a passed one."
    ),
    g!(
        Phase::Prove,
        "Cost model is stated at production volume with assumptions",
        "The cost model extrapolates to the agreed production volume and lists every assumption \
         it depends on.",
        "Check the model names its volume assumption, its unit cost source, and the date those \
         unit costs were observed."
    ),
    g!(
        Phase::Build,
        "Environment rebuildable from source",
        "The whole environment can be provisioned from the IaC repository with no manual step.",
        "Provision a clean environment from a fresh checkout and record what, if anything, had \
         to be done by hand."
    ),
    g!(
        Phase::Build,
        "Pipeline blocks a bad change",
        "CI/CD refuses to deploy a change that fails tests, linting, or the security scan.",
        "Open a pull request that deliberately violates each gate and record that the pipeline \
         stopped it."
    ),
    g!(
        Phase::Build,
        "SLOs defined with alerting that fires",
        "Every SLO has an error budget and an alert proven to fire.",
        "Inject a synthetic breach per SLO and confirm the alert reached the on-call \
         destination."
    ),
    g!(
        Phase::Build,
        "Security review happened during the build",
        "The threat model was produced early enough that its findings changed the design.",
        "Compare the threat model's date against the production-system delivery date, and list \
         the design changes it caused."
    ),
    g!(
        Phase::Build,
        "Significant decisions have ADRs",
        "Every architecturally significant decision has a record with its rejected \
         alternatives.",
        "Walk the commit history for decisions that changed a boundary, a datastore, or a \
         protocol; each should map to an ADR."
    ),
    g!(
        Phase::Operate,
        "Every alert has a runbook",
        "No alert can fire without a runbook that ends in a resolved state.",
        "Enumerate configured alerts and join against runbooks. Any unmatched alert fails."
    ),
    g!(
        Phase::Operate,
        "Escalation path tested end to end",
        "The escalation path has been exercised, not just documented.",
        "Run a drill: page the primary, escalate to secondary, and record the elapsed time at \
         each hop."
    ),
    g!(
        Phase::Operate,
        "Client engineers have operated the system unaided",
        "Client engineers have handled a real change and a real incident without us in the \
         room.",
        "Record the change and the incident, with the client engineer named and no vendor \
         participation."
    ),
    g!(
        Phase::Operate,
        "Exit plan agreed and dated",
        "The exit plan is documented, accepted by the client, and carries a date after which \
         the client owns the system.",
        "Signed exit plan attached as evidence, with the handover date and the access-\
         revocation checklist."
    ),
];

/// Deliverable templates for one phase.
pub fn template_for_phase(phase: Phase) -> Vec<&'static DeliverableTemplate> {
    TEMPLATE.iter().filter(|t| t.phase == phase).collect()
}

/// The VibeCody surface that produces each deliverable key.
pub fn tooling_map() -> BTreeMap<&'static str, &'static str> {
    TEMPLATE.iter().map(|t| (t.key, t.tool_hint)).collect()
}

// ── Store ─────────────────────────────────────────────────────────────────────

pub struct EngagementStore {
    conn: Connection,
}

impl EngagementStore {
    /// Open (or create) the engagement database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs for {parent:?}"))?;
        }
        let conn = Connection::open(path).with_context(|| format!("open SQLite at {path:?}"))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let store = Self { conn };
        store.create_schema()?;
        Ok(store)
    }

    /// Open from the default path: `~/.vibecli/engagements.db`.
    pub fn open_default() -> Result<Self> {
        Self::open(default_db_path())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    fn create_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS engagements (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                client          TEXT NOT NULL DEFAULT '',
                workspace_path  TEXT,
                status          TEXT NOT NULL DEFAULT 'draft',
                current_phase   TEXT NOT NULL DEFAULT 'discover',
                summary         TEXT NOT NULL DEFAULT '',
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS engagement_deliverables (
                id              TEXT PRIMARY KEY,
                engagement_id   TEXT NOT NULL REFERENCES engagements(id) ON DELETE CASCADE,
                phase           TEXT NOT NULL,
                key             TEXT NOT NULL,
                title           TEXT NOT NULL,
                description     TEXT NOT NULL DEFAULT '',
                status          TEXT NOT NULL DEFAULT 'not_started',
                owner           TEXT,
                tool_hint       TEXT,
                notes           TEXT NOT NULL DEFAULT '',
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL,
                UNIQUE(engagement_id, key)
            );
            CREATE INDEX IF NOT EXISTS idx_deliverables_engagement
                ON engagement_deliverables(engagement_id, phase);

            CREATE TABLE IF NOT EXISTS engagement_evidence (
                id              TEXT PRIMARY KEY,
                deliverable_id  TEXT NOT NULL
                                REFERENCES engagement_deliverables(id) ON DELETE CASCADE,
                kind            TEXT NOT NULL DEFAULT 'note',
                label           TEXT NOT NULL DEFAULT '',
                reference       TEXT NOT NULL DEFAULT '',
                captured_at     INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_evidence_deliverable
                ON engagement_evidence(deliverable_id);

            CREATE TABLE IF NOT EXISTS engagement_gates (
                id              TEXT PRIMARY KEY,
                engagement_id   TEXT NOT NULL REFERENCES engagements(id) ON DELETE CASCADE,
                phase           TEXT NOT NULL,
                title           TEXT NOT NULL,
                criterion       TEXT NOT NULL DEFAULT '',
                measurement     TEXT NOT NULL DEFAULT '',
                observed        TEXT,
                verdict         TEXT NOT NULL DEFAULT 'not_measured',
                rationale       TEXT NOT NULL DEFAULT '',
                decided_by      TEXT,
                decided_at      INTEGER,
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_gates_engagement
                ON engagement_gates(engagement_id, phase);
        "#,
        )?;
        Ok(())
    }

    // ── Engagements ───────────────────────────────────────────────────────

    /// Create an engagement and seed it with the full template — every
    /// promised deliverable and every default gate, so the phase board shows
    /// the whole commitment from day one rather than only what someone
    /// remembered to add.
    pub fn create(
        &self,
        name: &str,
        client: &str,
        workspace_path: Option<&str>,
        summary: &str,
    ) -> Result<Engagement> {
        let now = now_ms();
        let id = new_id();
        self.conn.execute(
            "INSERT INTO engagements
                (id, name, client, workspace_path, status, current_phase, summary,
                 created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'draft', 'discover', ?5, ?6, ?6)",
            params![id, name, client, workspace_path, summary, now],
        )?;
        self.seed_template(&id)?;
        self.get(&id)?
            .ok_or_else(|| anyhow::anyhow!("engagement {id} vanished after insert"))
    }

    /// Insert any template rows the engagement is missing.
    ///
    /// Idempotent, and safe to re-run after the template gains a row: existing
    /// deliverables keep their status, evidence, and owner. That matters
    /// because the alternative — recreating rows — would silently discard the
    /// client's acceptance record.
    pub fn seed_template(&self, engagement_id: &str) -> Result<usize> {
        let now = now_ms();
        let mut inserted = 0usize;
        for t in TEMPLATE {
            let changed = self.conn.execute(
                "INSERT INTO engagement_deliverables
                    (id, engagement_id, phase, key, title, description, status, tool_hint,
                     created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'not_started', ?7, ?8, ?8)
                 ON CONFLICT(engagement_id, key) DO NOTHING",
                params![
                    new_id(),
                    engagement_id,
                    t.phase.as_str(),
                    t.key,
                    t.title,
                    t.description,
                    t.tool_hint,
                    now
                ],
            )?;
            inserted += changed;
        }
        // Gates have no natural key — dedupe on (engagement, phase, title).
        for t in GATE_TEMPLATE {
            let exists: Option<String> = self
                .conn
                .query_row(
                    "SELECT id FROM engagement_gates
                     WHERE engagement_id = ?1 AND phase = ?2 AND title = ?3",
                    params![engagement_id, t.phase.as_str(), t.title],
                    |r| r.get(0),
                )
                .optional()?;
            if exists.is_none() {
                self.conn.execute(
                    "INSERT INTO engagement_gates
                        (id, engagement_id, phase, title, criterion, measurement, verdict,
                         created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'not_measured', ?7, ?7)",
                    params![
                        new_id(),
                        engagement_id,
                        t.phase.as_str(),
                        t.title,
                        t.criterion,
                        t.measurement,
                        now
                    ],
                )?;
                inserted += 1;
            }
        }
        Ok(inserted)
    }

    pub fn get(&self, id: &str) -> Result<Option<Engagement>> {
        let e = self
            .conn
            .query_row(
                "SELECT id, name, client, workspace_path, status, current_phase, summary,
                        created_at, updated_at
                 FROM engagements WHERE id = ?1",
                params![id],
                row_to_engagement,
            )
            .optional()?;
        Ok(e)
    }

    pub fn list(&self) -> Result<Vec<Engagement>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, client, workspace_path, status, current_phase, summary,
                    created_at, updated_at
             FROM engagements ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], row_to_engagement)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn set_status(&self, id: &str, status: EngagementStatus) -> Result<()> {
        self.conn.execute(
            "UPDATE engagements SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, status.as_str(), now_ms()],
        )?;
        Ok(())
    }

    pub fn set_phase(&self, id: &str, phase: Phase) -> Result<()> {
        self.conn.execute(
            "UPDATE engagements SET current_phase = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, phase.as_str(), now_ms()],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<bool> {
        Ok(self
            .conn
            .execute("DELETE FROM engagements WHERE id = ?1", params![id])?
            > 0)
    }

    // ── Deliverables ──────────────────────────────────────────────────────

    pub fn deliverables(
        &self,
        engagement_id: &str,
        phase: Option<Phase>,
    ) -> Result<Vec<Deliverable>> {
        let mut stmt = self.conn.prepare(
            "SELECT d.id, d.engagement_id, d.phase, d.key, d.title, d.description, d.status,
                    d.owner, d.tool_hint, d.notes, d.created_at, d.updated_at,
                    (SELECT COUNT(*) FROM engagement_evidence e WHERE e.deliverable_id = d.id)
             FROM engagement_deliverables d
             WHERE d.engagement_id = ?1 AND (?2 IS NULL OR d.phase = ?2)
             ORDER BY d.created_at ASC",
        )?;
        let rows = stmt.query_map(
            params![engagement_id, phase.map(|p| p.as_str())],
            row_to_deliverable,
        )?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn deliverable_by_key(
        &self,
        engagement_id: &str,
        key: &str,
    ) -> Result<Option<Deliverable>> {
        let d = self
            .conn
            .query_row(
                "SELECT d.id, d.engagement_id, d.phase, d.key, d.title, d.description, d.status,
                        d.owner, d.tool_hint, d.notes, d.created_at, d.updated_at,
                        (SELECT COUNT(*) FROM engagement_evidence e WHERE e.deliverable_id = d.id)
                 FROM engagement_deliverables d
                 WHERE d.engagement_id = ?1 AND d.key = ?2",
                params![engagement_id, key],
                row_to_deliverable,
            )
            .optional()?;
        Ok(d)
    }

    /// Patch a deliverable. Every field is optional; `None` leaves the column
    /// untouched rather than blanking it — the reason this is a `DO UPDATE SET`
    /// of named columns and never an `INSERT OR REPLACE`.
    pub fn update_deliverable(
        &self,
        id: &str,
        status: Option<DeliverableStatus>,
        owner: Option<&str>,
        notes: Option<&str>,
    ) -> Result<()> {
        let now = now_ms();
        if let Some(s) = status {
            self.conn.execute(
                "UPDATE engagement_deliverables SET status = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, s.as_str(), now],
            )?;
        }
        if let Some(o) = owner {
            self.conn.execute(
                "UPDATE engagement_deliverables SET owner = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, o, now],
            )?;
        }
        if let Some(n) = notes {
            self.conn.execute(
                "UPDATE engagement_deliverables SET notes = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, n, now],
            )?;
        }
        Ok(())
    }

    /// Add a custom deliverable outside the template — engagements acquire
    /// commitments the model did not anticipate.
    pub fn add_deliverable(
        &self,
        engagement_id: &str,
        phase: Phase,
        key: &str,
        title: &str,
        description: &str,
        tool_hint: Option<&str>,
    ) -> Result<Deliverable> {
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO engagement_deliverables
                (id, engagement_id, phase, key, title, description, status, tool_hint,
                 created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'not_started', ?7, ?8, ?8)
             ON CONFLICT(engagement_id, key) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                tool_hint = excluded.tool_hint,
                updated_at = excluded.updated_at",
            params![
                new_id(),
                engagement_id,
                phase.as_str(),
                key,
                title,
                description,
                tool_hint,
                now
            ],
        )?;
        self.deliverable_by_key(engagement_id, key)?
            .ok_or_else(|| anyhow::anyhow!("deliverable {key} missing after upsert"))
    }

    // ── Evidence ──────────────────────────────────────────────────────────

    pub fn add_evidence(
        &self,
        deliverable_id: &str,
        kind: EvidenceKind,
        label: &str,
        reference: &str,
    ) -> Result<Evidence> {
        let id = new_id();
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO engagement_evidence
                (id, deliverable_id, kind, label, reference, captured_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, deliverable_id, kind.as_str(), label, reference, now],
        )?;
        Ok(Evidence {
            id,
            deliverable_id: deliverable_id.to_string(),
            kind,
            label: label.to_string(),
            reference: reference.to_string(),
            captured_at: now as u64,
        })
    }

    pub fn evidence(&self, deliverable_id: &str) -> Result<Vec<Evidence>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, deliverable_id, kind, label, reference, captured_at
             FROM engagement_evidence WHERE deliverable_id = ?1 ORDER BY captured_at ASC",
        )?;
        let rows = stmt.query_map(params![deliverable_id], |r| {
            Ok(Evidence {
                id: r.get(0)?,
                deliverable_id: r.get(1)?,
                kind: EvidenceKind::from_str(&r.get::<_, String>(2)?),
                label: r.get(3)?,
                reference: r.get(4)?,
                captured_at: r.get::<_, i64>(5)? as u64,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn delete_evidence(&self, id: &str) -> Result<bool> {
        Ok(self
            .conn
            .execute("DELETE FROM engagement_evidence WHERE id = ?1", params![id])?
            > 0)
    }

    // ── Gates ─────────────────────────────────────────────────────────────

    pub fn gates(&self, engagement_id: &str, phase: Option<Phase>) -> Result<Vec<Gate>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, engagement_id, phase, title, criterion, measurement, observed, verdict,
                    rationale, decided_by, decided_at, created_at, updated_at
             FROM engagement_gates
             WHERE engagement_id = ?1 AND (?2 IS NULL OR phase = ?2)
             ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(
            params![engagement_id, phase.map(|p| p.as_str())],
            row_to_gate,
        )?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn add_gate(
        &self,
        engagement_id: &str,
        phase: Phase,
        title: &str,
        criterion: &str,
        measurement: &str,
    ) -> Result<Gate> {
        let id = new_id();
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO engagement_gates
                (id, engagement_id, phase, title, criterion, measurement, verdict,
                 created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'not_measured', ?7, ?7)",
            params![
                id,
                engagement_id,
                phase.as_str(),
                title,
                criterion,
                measurement,
                now
            ],
        )?;
        self.gate(&id)?
            .ok_or_else(|| anyhow::anyhow!("gate {id} missing after insert"))
    }

    pub fn gate(&self, id: &str) -> Result<Option<Gate>> {
        let g = self
            .conn
            .query_row(
                "SELECT id, engagement_id, phase, title, criterion, measurement, observed,
                        verdict, rationale, decided_by, decided_at, created_at, updated_at
                 FROM engagement_gates WHERE id = ?1",
                params![id],
                row_to_gate,
            )
            .optional()?;
        Ok(g)
    }

    /// Record a judgement against a gate.
    ///
    /// `observed` is what was actually seen. It stays `NULL` unless a caller
    /// supplies it — a verdict with no observation is visible as exactly that,
    /// which is the point.
    pub fn judge_gate(
        &self,
        id: &str,
        verdict: GateVerdict,
        observed: Option<&str>,
        rationale: &str,
        decided_by: Option<&str>,
    ) -> Result<()> {
        let now = now_ms();
        // A gate returned to `not_measured` clears its decision metadata:
        // leaving a decider and a timestamp on an unmeasured gate would assert
        // that somebody judged it.
        let decided_at: Option<i64> = if verdict == GateVerdict::NotMeasured {
            None
        } else {
            Some(now)
        };
        let decider = if verdict == GateVerdict::NotMeasured {
            None
        } else {
            decided_by
        };
        self.conn.execute(
            "UPDATE engagement_gates
             SET verdict = ?2, observed = ?3, rationale = ?4, decided_by = ?5, decided_at = ?6,
                 updated_at = ?7
             WHERE id = ?1",
            params![id, verdict.as_str(), observed, rationale, decider, decided_at, now],
        )?;
        Ok(())
    }

    pub fn delete_gate(&self, id: &str) -> Result<bool> {
        Ok(self
            .conn
            .execute("DELETE FROM engagement_gates WHERE id = ?1", params![id])?
            > 0)
    }

    // ── Readiness ─────────────────────────────────────────────────────────

    /// Readiness for one phase: the tallies, the blockers, and whether the
    /// phase may be exited.
    pub fn phase_readiness(&self, engagement_id: &str, phase: Phase) -> Result<PhaseReadiness> {
        let deliverables = self.deliverables(engagement_id, Some(phase))?;
        let gates = self.gates(engagement_id, Some(phase))?;

        let mut dt = DeliverableTally {
            total: deliverables.len(),
            not_started: 0,
            in_progress: 0,
            ready: 0,
            accepted: 0,
            waived: 0,
            claimed_without_evidence: 0,
        };
        let mut blockers: Vec<Blocker> = Vec::new();

        for d in &deliverables {
            match d.status {
                DeliverableStatus::NotStarted => dt.not_started += 1,
                DeliverableStatus::InProgress => dt.in_progress += 1,
                DeliverableStatus::Ready => dt.ready += 1,
                DeliverableStatus::Accepted => dt.accepted += 1,
                DeliverableStatus::Waived => dt.waived += 1,
            }
            if matches!(
                d.status,
                DeliverableStatus::Ready | DeliverableStatus::Accepted
            ) && d.evidence_count == 0
            {
                dt.claimed_without_evidence += 1;
                blockers.push(Blocker {
                    kind: BlockerKind::DeliverableWithoutEvidence,
                    subject: d.key.clone(),
                    detail: format!(
                        "'{}' is marked {} with no evidence attached.",
                        d.title,
                        d.status.as_str()
                    ),
                });
            }
            if matches!(
                d.status,
                DeliverableStatus::NotStarted
                    | DeliverableStatus::InProgress
                    | DeliverableStatus::Ready
            ) {
                blockers.push(Blocker {
                    kind: BlockerKind::DeliverableOutstanding,
                    subject: d.key.clone(),
                    detail: format!("'{}' is {}, not accepted.", d.title, d.status.as_str()),
                });
            }
        }

        let mut gt = GateTally {
            total: gates.len(),
            not_measured: 0,
            pending: 0,
            pass: 0,
            fail: 0,
            waived: 0,
        };
        for g in &gates {
            match g.verdict {
                GateVerdict::NotMeasured => {
                    gt.not_measured += 1;
                    blockers.push(Blocker {
                        kind: BlockerKind::GateNotMeasured,
                        subject: g.id.clone(),
                        detail: format!("'{}' has not been measured.", g.title),
                    });
                }
                GateVerdict::Pending => {
                    gt.pending += 1;
                    blockers.push(Blocker {
                        kind: BlockerKind::GatePending,
                        subject: g.id.clone(),
                        detail: format!("'{}' is scheduled but not yet judged.", g.title),
                    });
                }
                GateVerdict::Pass => gt.pass += 1,
                GateVerdict::Fail => {
                    gt.fail += 1;
                    blockers.push(Blocker {
                        kind: BlockerKind::GateFailed,
                        subject: g.id.clone(),
                        detail: format!(
                            "'{}' failed{}",
                            g.title,
                            if g.rationale.is_empty() {
                                ".".to_string()
                            } else {
                                format!(": {}", g.rationale)
                            }
                        ),
                    });
                }
                GateVerdict::Waived => gt.waived += 1,
            }
        }

        // In-scope = everything not waived. A phase whose deliverables are all
        // waived has no denominator, so it reports `None` rather than 0% or a
        // triumphant 100%.
        let in_scope = dt.total - dt.waived;
        let completion = if in_scope == 0 {
            None
        } else {
            Some(dt.accepted as f64 / in_scope as f64)
        };

        Ok(PhaseReadiness {
            phase,
            title: phase.title().to_string(),
            cadence: phase.cadence().map(str::to_string),
            completion,
            can_exit: blockers.is_empty(),
            deliverables: dt,
            gates: gt,
            blockers,
        })
    }

    pub fn report(&self, engagement_id: &str) -> Result<EngagementReport> {
        let engagement = self
            .get(engagement_id)?
            .ok_or_else(|| anyhow::anyhow!("no engagement {engagement_id}"))?;
        let phases = Phase::ALL
            .iter()
            .map(|p| self.phase_readiness(engagement_id, *p))
            .collect::<Result<Vec<_>>>()?;
        Ok(EngagementReport {
            engagement,
            phases,
            generated_at: now_ms() as u64,
        })
    }

    /// Advance to the next phase if the current one can be exited.
    ///
    /// Returns the blockers instead of advancing when it cannot. `force`
    /// advances anyway — sometimes the client decides to proceed with a gate
    /// failing — but the blockers are returned either way so the decision is
    /// recorded rather than hidden.
    pub fn advance_phase(&self, engagement_id: &str, force: bool) -> Result<AdvanceOutcome> {
        let engagement = self
            .get(engagement_id)?
            .ok_or_else(|| anyhow::anyhow!("no engagement {engagement_id}"))?;
        let current = engagement.current_phase;
        let readiness = self.phase_readiness(engagement_id, current)?;
        let Some(next) = current.next() else {
            return Ok(AdvanceOutcome {
                advanced: false,
                from: current,
                to: None,
                forced: false,
                blockers: readiness.blockers,
                reason: "Operate & Transfer is the final phase.".to_string(),
            });
        };
        if !readiness.can_exit && !force {
            let n = readiness.blockers.len();
            return Ok(AdvanceOutcome {
                advanced: false,
                from: current,
                to: Some(next),
                forced: false,
                blockers: readiness.blockers,
                reason: format!("{n} blocker(s) outstanding in {}.", current.title()),
            });
        }
        self.set_phase(engagement_id, next)?;
        let forced = !readiness.can_exit;
        Ok(AdvanceOutcome {
            advanced: true,
            from: current,
            to: Some(next),
            forced,
            reason: if forced {
                format!(
                    "Advanced with {} blocker(s) overridden.",
                    readiness.blockers.len()
                )
            } else {
                format!("{} closed cleanly.", current.title())
            },
            blockers: readiness.blockers,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvanceOutcome {
    pub advanced: bool,
    pub from: Phase,
    pub to: Option<Phase>,
    /// True when the phase was exited with blockers outstanding.
    pub forced: bool,
    pub blockers: Vec<Blocker>,
    pub reason: String,
}

// ── Row mappers ───────────────────────────────────────────────────────────────

fn row_to_engagement(r: &rusqlite::Row<'_>) -> rusqlite::Result<Engagement> {
    Ok(Engagement {
        id: r.get(0)?,
        name: r.get(1)?,
        client: r.get(2)?,
        workspace_path: r.get(3)?,
        status: EngagementStatus::from_str(&r.get::<_, String>(4)?),
        current_phase: Phase::from_str(&r.get::<_, String>(5)?).unwrap_or(Phase::Discover),
        summary: r.get(6)?,
        created_at: r.get::<_, i64>(7)? as u64,
        updated_at: r.get::<_, i64>(8)? as u64,
    })
}

fn row_to_deliverable(r: &rusqlite::Row<'_>) -> rusqlite::Result<Deliverable> {
    Ok(Deliverable {
        id: r.get(0)?,
        engagement_id: r.get(1)?,
        phase: Phase::from_str(&r.get::<_, String>(2)?).unwrap_or(Phase::Discover),
        key: r.get(3)?,
        title: r.get(4)?,
        description: r.get(5)?,
        status: DeliverableStatus::from_str(&r.get::<_, String>(6)?),
        owner: r.get(7)?,
        tool_hint: r.get(8)?,
        notes: r.get(9)?,
        created_at: r.get::<_, i64>(10)? as u64,
        updated_at: r.get::<_, i64>(11)? as u64,
        evidence_count: r.get::<_, i64>(12)? as usize,
    })
}

fn row_to_gate(r: &rusqlite::Row<'_>) -> rusqlite::Result<Gate> {
    Ok(Gate {
        id: r.get(0)?,
        engagement_id: r.get(1)?,
        phase: Phase::from_str(&r.get::<_, String>(2)?).unwrap_or(Phase::Discover),
        title: r.get(3)?,
        criterion: r.get(4)?,
        measurement: r.get(5)?,
        observed: r.get(6)?,
        verdict: GateVerdict::from_str(&r.get::<_, String>(7)?),
        rationale: r.get(8)?,
        decided_by: r.get(9)?,
        decided_at: r.get::<_, Option<i64>>(10)?.map(|v| v as u64),
        created_at: r.get::<_, i64>(11)? as u64,
        updated_at: r.get::<_, i64>(12)? as u64,
    })
}

// ── Default path ──────────────────────────────────────────────────────────────

pub fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vibecli")
        .join("engagements.db")
}

// ── Markdown rendering ────────────────────────────────────────────────────────

fn pct(v: Option<f64>) -> String {
    match v {
        // "n/a" and "0%" are different claims. Nothing here may blur them.
        None => "n/a".to_string(),
        Some(f) => format!("{:.0}%", f * 100.0),
    }
}

/// Render the engagement status report — the artifact a client sees.
pub fn render_report_markdown(store: &EngagementStore, engagement_id: &str) -> Result<String> {
    let report = store.report(engagement_id)?;
    let e = &report.engagement;
    let mut out = String::new();

    out.push_str(&format!("# {} — engagement status\n\n", e.name));
    if !e.client.is_empty() {
        out.push_str(&format!("**Client:** {}  \n", e.client));
    }
    out.push_str(&format!(
        "**Status:** {}  \n**Current phase:** {} ({} of 4)\n\n",
        e.status.as_str(),
        e.current_phase.title(),
        e.current_phase.index() + 1
    ));
    if !e.summary.is_empty() {
        out.push_str(&format!("{}\n\n", e.summary));
    }

    out.push_str("## Phase summary\n\n");
    out.push_str("| Phase | Cadence | Accepted | In scope | Complete | Gates pass | Gates open | Exit |\n");
    out.push_str("|---|---|---:|---:|---:|---:|---:|---|\n");
    for p in &report.phases {
        let in_scope = p.deliverables.total - p.deliverables.waived;
        let gates_open = p.gates.not_measured + p.gates.pending + p.gates.fail;
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {}/{} | {} | {} |\n",
            p.title,
            p.cadence.as_deref().unwrap_or("—"),
            p.deliverables.accepted,
            in_scope,
            pct(p.completion),
            p.gates.pass,
            p.gates.total,
            gates_open,
            if p.can_exit { "ready" } else { "blocked" },
        ));
    }
    out.push('\n');

    for p in &report.phases {
        out.push_str(&format!("## {} — {}\n\n", p.phase.index() + 1, p.title));
        if let Some(c) = &p.cadence {
            out.push_str(&format!("*{c}*\n\n"));
        }
        out.push_str(&format!("{}\n\n", p.phase.purpose()));

        out.push_str("### Deliverables\n\n");
        let ds = store.deliverables(engagement_id, Some(p.phase))?;
        if ds.is_empty() {
            out.push_str("_No deliverables defined for this phase._\n\n");
        } else {
            out.push_str("| Deliverable | Status | Evidence | Owner | Produced with |\n");
            out.push_str("|---|---|---:|---|---|\n");
            for d in &ds {
                out.push_str(&format!(
                    "| {} | {} | {} | {} | {} |\n",
                    d.title,
                    d.status.as_str(),
                    d.evidence_count,
                    d.owner.as_deref().unwrap_or("—"),
                    d.tool_hint.as_deref().unwrap_or("—"),
                ));
            }
            out.push('\n');
        }

        out.push_str("### Gates\n\n");
        let gs = store.gates(engagement_id, Some(p.phase))?;
        if gs.is_empty() {
            out.push_str("_No gates defined for this phase._\n\n");
        } else {
            for g in &gs {
                out.push_str(&format!("**{}** — `{}`\n\n", g.title, g.verdict.as_str()));
                out.push_str(&format!("- Criterion: {}\n", g.criterion));
                out.push_str(&format!("- Measured by: {}\n", g.measurement));
                match &g.observed {
                    // Absent stays absent. No "0", no "pending", no guess.
                    None => out.push_str("- Observed: _not recorded_\n"),
                    Some(o) => out.push_str(&format!("- Observed: {o}\n")),
                }
                if !g.rationale.is_empty() {
                    out.push_str(&format!("- Rationale: {}\n", g.rationale));
                }
                out.push('\n');
            }
        }

        if p.blockers.is_empty() {
            out.push_str("_No blockers. This phase can be closed._\n\n");
        } else {
            out.push_str(&format!("### Blockers ({})\n\n", p.blockers.len()));
            for b in &p.blockers {
                out.push_str(&format!("- {}\n", b.detail));
            }
            out.push('\n');
        }
    }

    Ok(out)
}

/// Render the handover pack — what the client's team needs in order to own the
/// system, plus an honest account of what is not ready.
pub fn render_handover_markdown(store: &EngagementStore, engagement_id: &str) -> Result<String> {
    let report = store.report(engagement_id)?;
    let e = &report.engagement;
    let mut out = String::new();

    out.push_str(&format!("# {} — handover pack\n\n", e.name));
    out.push_str(
        "The goal of this document is for your team to own the system. It lists what was \
         delivered, where the evidence is, and — deliberately — what is not finished.\n\n",
    );

    let operate = store.deliverables(engagement_id, Some(Phase::Operate))?;
    out.push_str("## Operating the system\n\n");
    for d in &operate {
        out.push_str(&format!("### {}\n\n{}\n\n", d.title, d.description));
        let ev = store.evidence(&d.id)?;
        if ev.is_empty() {
            out.push_str("**No artifact attached.** This is not ready for handover.\n\n");
        } else {
            for x in &ev {
                out.push_str(&format!(
                    "- [{}] {} — `{}`\n",
                    x.kind.as_str(),
                    x.label,
                    x.reference
                ));
            }
            out.push('\n');
        }
    }

    out.push_str("## Everything delivered\n\n");
    for p in Phase::ALL {
        let ds = store.deliverables(engagement_id, Some(p))?;
        let accepted: Vec<_> = ds
            .iter()
            .filter(|d| d.status == DeliverableStatus::Accepted)
            .collect();
        if accepted.is_empty() {
            continue;
        }
        out.push_str(&format!("### {}\n\n", p.title()));
        for d in accepted {
            out.push_str(&format!("- **{}** — {} evidence item(s)\n", d.title, d.evidence_count));
        }
        out.push('\n');
    }

    // The section that makes the pack worth reading.
    out.push_str("## Not delivered\n\n");
    let mut any_outstanding = false;
    for p in &report.phases {
        let outstanding: Vec<_> = p
            .blockers
            .iter()
            .filter(|b| b.kind != BlockerKind::DeliverableWithoutEvidence)
            .collect();
        if outstanding.is_empty() {
            continue;
        }
        any_outstanding = true;
        out.push_str(&format!("### {}\n\n", p.title));
        for b in outstanding {
            out.push_str(&format!("- {}\n", b.detail));
        }
        out.push('\n');
    }
    if !any_outstanding {
        out.push_str("Every deliverable is accepted and every gate is pass or waived.\n\n");
    }

    let waived: Vec<_> = Phase::ALL
        .iter()
        .filter_map(|p| store.deliverables(engagement_id, Some(*p)).ok())
        .flatten()
        .filter(|d| d.status == DeliverableStatus::Waived)
        .collect();
    if !waived.is_empty() {
        out.push_str("## Waived by agreement\n\n");
        out.push_str(
            "These were agreed out of scope. They are listed because 'waived' and 'done' are \
             different things.\n\n",
        );
        for d in &waived {
            out.push_str(&format!(
                "- **{}** — {}\n",
                d.title,
                if d.notes.is_empty() {
                    "no reason recorded"
                } else {
                    &d.notes
                }
            ));
        }
        out.push('\n');
    }

    Ok(out)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store() -> (TempDir, EngagementStore) {
        let dir = TempDir::new().expect("tempdir");
        let s = EngagementStore::open(dir.path().join("engagements.db")).expect("open");
        (dir, s)
    }

    #[test]
    fn template_covers_every_phase() {
        for p in Phase::ALL {
            assert!(
                !template_for_phase(p).is_empty(),
                "{} has no deliverable template — a phase that promises nothing \
                 cannot be delivered against",
                p.title()
            );
            assert!(
                GATE_TEMPLATE.iter().any(|g| g.phase == p),
                "{} has no gates",
                p.title()
            );
        }
    }

    #[test]
    fn template_keys_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for t in TEMPLATE {
            assert!(seen.insert(t.key), "duplicate template key {}", t.key);
        }
    }

    #[test]
    fn every_gate_template_names_a_measurement() {
        // A criterion with no measurement procedure is decided by whoever
        // argues hardest at the review. Refuse to ship one.
        for g in GATE_TEMPLATE {
            assert!(
                !g.measurement.trim().is_empty(),
                "gate '{}' has no measurement procedure",
                g.title
            );
            assert!(
                !g.criterion.trim().is_empty(),
                "gate '{}' has no criterion",
                g.title
            );
        }
    }

    #[test]
    fn discover_publishes_no_cadence() {
        // The engagement model states a duration for three phases. Inventing
        // one for the fourth would put an unmade commitment in front of a
        // client.
        assert_eq!(Phase::Discover.cadence(), None);
        assert_eq!(Phase::Prove.cadence(), Some("4–8 weeks"));
    }

    #[test]
    fn create_seeds_the_whole_template() {
        let (_d, s) = store();
        let e = s.create("Acme platform", "Acme", None, "").expect("create");
        let all = s.deliverables(&e.id, None).expect("deliverables");
        assert_eq!(all.len(), TEMPLATE.len());
        let gates = s.gates(&e.id, None).expect("gates");
        assert_eq!(gates.len(), GATE_TEMPLATE.len());
        assert!(gates.iter().all(|g| g.verdict == GateVerdict::NotMeasured));
        assert!(gates.iter().all(|g| g.observed.is_none()));
    }

    #[test]
    fn reseeding_preserves_client_acceptance() {
        let (_d, s) = store();
        let e = s.create("Acme", "Acme", None, "").expect("create");
        let d = s
            .deliverable_by_key(&e.id, "risk-register")
            .expect("query")
            .expect("present");
        s.update_deliverable(&d.id, Some(DeliverableStatus::Accepted), Some("rb"), None)
            .expect("update");

        let added = s.seed_template(&e.id).expect("reseed");
        assert_eq!(added, 0, "reseed should insert nothing when complete");

        let after = s
            .deliverable_by_key(&e.id, "risk-register")
            .expect("query")
            .expect("present");
        assert_eq!(after.status, DeliverableStatus::Accepted);
        assert_eq!(after.owner.as_deref(), Some("rb"));
    }

    #[test]
    fn empty_phase_reports_na_not_zero() {
        let (_d, s) = store();
        let e = s.create("Acme", "Acme", None, "").expect("create");
        // Waive every Discover deliverable → no in-scope denominator.
        for d in s.deliverables(&e.id, Some(Phase::Discover)).expect("list") {
            s.update_deliverable(&d.id, Some(DeliverableStatus::Waived), None, None)
                .expect("waive");
        }
        let r = s
            .phase_readiness(&e.id, Phase::Discover)
            .expect("readiness");
        assert_eq!(r.completion, None, "no in-scope work must report n/a, not 0%");
        assert_eq!(pct(r.completion), "n/a");
    }

    #[test]
    fn unmeasured_gate_blocks_and_is_not_a_failure() {
        let (_d, s) = store();
        let e = s.create("Acme", "Acme", None, "").expect("create");
        let r = s.phase_readiness(&e.id, Phase::Prove).expect("readiness");
        assert!(!r.can_exit);
        assert_eq!(r.gates.fail, 0, "unmeasured must not be counted as failed");
        assert!(r.gates.not_measured > 0);
        assert!(r
            .blockers
            .iter()
            .any(|b| b.kind == BlockerKind::GateNotMeasured));
    }

    #[test]
    fn accepted_without_evidence_is_flagged() {
        let (_d, s) = store();
        let e = s.create("Acme", "Acme", None, "").expect("create");
        let d = s
            .deliverable_by_key(&e.id, "cost-model")
            .expect("query")
            .expect("present");
        s.update_deliverable(&d.id, Some(DeliverableStatus::Accepted), None, None)
            .expect("accept");
        let r = s.phase_readiness(&e.id, Phase::Prove).expect("readiness");
        assert_eq!(r.deliverables.claimed_without_evidence, 1);
        assert!(r
            .blockers
            .iter()
            .any(|b| b.kind == BlockerKind::DeliverableWithoutEvidence));

        s.add_evidence(&d.id, EvidenceKind::File, "model", "docs/cost.md")
            .expect("evidence");
        let r2 = s.phase_readiness(&e.id, Phase::Prove).expect("readiness");
        assert_eq!(r2.deliverables.claimed_without_evidence, 0);
    }

    #[test]
    fn phase_exits_only_when_clean() {
        let (_d, s) = store();
        let e = s.create("Acme", "Acme", None, "").expect("create");

        let blocked = s.advance_phase(&e.id, false).expect("advance");
        assert!(!blocked.advanced);
        assert!(!blocked.blockers.is_empty());
        assert_eq!(
            s.get(&e.id).expect("get").expect("present").current_phase,
            Phase::Discover
        );

        // Satisfy Discover completely.
        for d in s.deliverables(&e.id, Some(Phase::Discover)).expect("list") {
            s.update_deliverable(&d.id, Some(DeliverableStatus::Accepted), None, None)
                .expect("accept");
            s.add_evidence(&d.id, EvidenceKind::Note, "done", "seen")
                .expect("evidence");
        }
        for g in s.gates(&e.id, Some(Phase::Discover)).expect("gates") {
            s.judge_gate(&g.id, GateVerdict::Pass, Some("verified"), "", Some("rb"))
                .expect("judge");
        }

        let ok = s.advance_phase(&e.id, false).expect("advance");
        assert!(ok.advanced);
        assert!(!ok.forced);
        assert_eq!(ok.to, Some(Phase::Prove));
        assert_eq!(
            s.get(&e.id).expect("get").expect("present").current_phase,
            Phase::Prove
        );
    }

    #[test]
    fn forced_advance_records_the_override() {
        let (_d, s) = store();
        let e = s.create("Acme", "Acme", None, "").expect("create");
        let out = s.advance_phase(&e.id, true).expect("advance");
        assert!(out.advanced);
        assert!(out.forced, "an override must be visible as an override");
        assert!(!out.blockers.is_empty(), "blockers survive the override");
    }

    #[test]
    fn returning_a_gate_to_unmeasured_clears_its_decision() {
        let (_d, s) = store();
        let e = s.create("Acme", "Acme", None, "").expect("create");
        let g = s
            .gates(&e.id, Some(Phase::Build))
            .expect("gates")
            .into_iter()
            .next()
            .expect("at least one build gate");
        s.judge_gate(&g.id, GateVerdict::Pass, Some("yes"), "ok", Some("rb"))
            .expect("judge");
        let judged = s.gate(&g.id).expect("get").expect("present");
        assert!(judged.decided_at.is_some());

        s.judge_gate(&g.id, GateVerdict::NotMeasured, None, "", None)
            .expect("unjudge");
        let reset = s.gate(&g.id).expect("get").expect("present");
        assert_eq!(reset.decided_at, None);
        assert_eq!(reset.decided_by, None);
        assert_eq!(reset.observed, None);
    }

    #[test]
    fn waived_gate_satisfies_but_pending_does_not() {
        assert!(GateVerdict::Waived.satisfies_gate());
        assert!(GateVerdict::Pass.satisfies_gate());
        assert!(!GateVerdict::Pending.satisfies_gate());
        assert!(!GateVerdict::NotMeasured.satisfies_gate());
        assert!(!GateVerdict::Fail.satisfies_gate());
    }

    #[test]
    fn report_markdown_names_unmeasured_gates_honestly() {
        let (_d, s) = store();
        let e = s.create("Acme platform", "Acme Corp", None, "Pilot then build")
            .expect("create");
        let md = render_report_markdown(&s, &e.id).expect("render");
        assert!(md.contains("Acme platform"));
        assert!(md.contains("not_measured"));
        assert!(md.contains("_not recorded_"));
        // Discover has no published cadence — the table must show a dash, not
        // an invented duration.
        assert!(md.contains("| Discover & Assess | — |"));
    }

    #[test]
    fn handover_lists_what_is_not_delivered() {
        let (_d, s) = store();
        let e = s.create("Acme", "Acme", None, "").expect("create");
        let md = render_handover_markdown(&s, &e.id).expect("render");
        assert!(md.contains("## Not delivered"));
        assert!(md.contains("**No artifact attached.**"));
    }

    #[test]
    fn every_deliverable_points_at_a_tool() {
        // The whole navigational claim of the engagement panel rests on this:
        // a promised deliverable with no surface behind it means the operator
        // has to guess which of 300 panels applies.
        let map = tooling_map();
        assert_eq!(map.len(), TEMPLATE.len());
        for t in TEMPLATE {
            assert!(
                !t.tool_hint.trim().is_empty(),
                "deliverable '{}' names no producing surface",
                t.key
            );
        }
    }

    #[test]
    fn phase_ordering_is_total_and_terminates() {
        let mut p = Phase::Discover;
        let mut seen = vec![p];
        while let Some(n) = p.next() {
            assert!(n.index() == p.index() + 1);
            p = n;
            seen.push(p);
        }
        assert_eq!(seen, Phase::ALL.to_vec());
        assert_eq!(Phase::Operate.next(), None);
    }

    #[test]
    fn roundtrips_through_string_forms() {
        for p in Phase::ALL {
            assert_eq!(Phase::from_str(p.as_str()), Some(p));
        }
        for v in [
            GateVerdict::NotMeasured,
            GateVerdict::Pending,
            GateVerdict::Pass,
            GateVerdict::Fail,
            GateVerdict::Waived,
        ] {
            assert_eq!(GateVerdict::from_str(v.as_str()), v);
        }
        for d in [
            DeliverableStatus::NotStarted,
            DeliverableStatus::InProgress,
            DeliverableStatus::Ready,
            DeliverableStatus::Accepted,
            DeliverableStatus::Waived,
        ] {
            assert_eq!(DeliverableStatus::from_str(d.as_str()), d);
        }
    }
}
