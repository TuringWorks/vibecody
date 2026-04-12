#![allow(dead_code)]
//! Structured agent event lanes for observability and audit.
//!
//! Claw-code parity Wave 3: emits typed events into named lanes (tool, plan,
//! memory, user, system) enabling filtering, replay, and cost attribution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Event Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lane { Tool, Plan, Memory, User, System, Error, Cost }

impl std::fmt::Display for Lane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tool => write!(f, "tool"),     Self::Plan   => write!(f, "plan"),
            Self::Memory => write!(f, "memory"), Self::User   => write!(f, "user"),
            Self::System => write!(f, "system"), Self::Error  => write!(f, "error"),
            Self::Cost   => write!(f, "cost"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneEvent {
    pub id: u64,
    pub lane: Lane,
    pub session_id: String,
    pub ts_ms: u64,
    pub payload: EventPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPayload {
    ToolCall   { name: String, args_preview: String },
    ToolResult { name: String, ok: bool, tokens: u32 },
    PlanStep   { step: u32, description: String },
    MemoryRead { key: String },
    MemoryWrite{ key: String, bytes: usize },
    UserMessage{ content_preview: String, tokens: u32 },
    SystemNote { message: String },
    ErrorEvent { message: String, recoverable: bool },
    CostAccrued{ input_tokens: u64, output_tokens: u64, usd: f64 },
}

// ─── Event Bus ───────────────────────────────────────────────────────────────

pub struct LaneEventBus {
    events: Vec<LaneEvent>,
    next_id: u64,
    /// Per-lane subscriber callbacks (simulated via recorded lane filters).
    subscriptions: HashMap<String, Lane>,
}

impl LaneEventBus {
    pub fn new() -> Self { Self { events: Vec::new(), next_id: 0, subscriptions: HashMap::new() } }

    pub fn emit(&mut self, session_id: impl Into<String>, ts_ms: u64, lane: Lane, payload: EventPayload) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.events.push(LaneEvent { id, lane, session_id: session_id.into(), ts_ms, payload });
        id
    }

    pub fn subscribe(&mut self, subscriber: impl Into<String>, lane: Lane) {
        self.subscriptions.insert(subscriber.into(), lane);
    }

    pub fn events_for_session(&self, session_id: &str) -> Vec<&LaneEvent> {
        self.events.iter().filter(|e| e.session_id == session_id).collect()
    }

    pub fn events_in_lane(&self, lane: &Lane) -> Vec<&LaneEvent> {
        self.events.iter().filter(|e| &e.lane == lane).collect()
    }

    pub fn events_in_range(&self, from_ms: u64, to_ms: u64) -> Vec<&LaneEvent> {
        self.events.iter().filter(|e| e.ts_ms >= from_ms && e.ts_ms <= to_ms).collect()
    }

    /// Cost summary across all CostAccrued events for a session.
    pub fn cost_summary(&self, session_id: &str) -> CostSummary {
        let mut total_input = 0u64;
        let mut total_output = 0u64;
        let mut total_usd = 0.0_f64;
        for e in self.events_for_session(session_id) {
            if let EventPayload::CostAccrued { input_tokens, output_tokens, usd } = &e.payload {
                total_input += input_tokens;
                total_output += output_tokens;
                total_usd += usd;
            }
        }
        CostSummary { input_tokens: total_input, output_tokens: total_output, total_usd }
    }

    /// Count events per lane for a session.
    pub fn lane_counts(&self, session_id: &str) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for e in self.events_for_session(session_id) {
            *counts.entry(e.lane.to_string()).or_insert(0) += 1;
        }
        counts
    }

    pub fn total_events(&self) -> usize { self.events.len() }
    pub fn clear(&mut self) { self.events.clear(); }
}

impl Default for LaneEventBus {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_usd: f64,
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn bus() -> LaneEventBus { LaneEventBus::new() }
    const S: &str = "sess-1";

    #[test]
    fn test_emit_returns_incrementing_ids() {
        let mut b = bus();
        let a = b.emit(S, 0, Lane::Tool, EventPayload::SystemNote { message: "a".into() });
        let c = b.emit(S, 0, Lane::Tool, EventPayload::SystemNote { message: "b".into() });
        assert_eq!(c, a + 1);
    }

    #[test]
    fn test_events_for_session_filtered() {
        let mut b = bus();
        b.emit("s1", 0, Lane::User, EventPayload::SystemNote { message: "x".into() });
        b.emit("s2", 0, Lane::User, EventPayload::SystemNote { message: "y".into() });
        assert_eq!(b.events_for_session("s1").len(), 1);
    }

    #[test]
    fn test_events_in_lane() {
        let mut b = bus();
        b.emit(S, 0, Lane::Tool, EventPayload::SystemNote { message: "t".into() });
        b.emit(S, 0, Lane::Plan, EventPayload::PlanStep { step: 1, description: "do".into() });
        assert_eq!(b.events_in_lane(&Lane::Tool).len(), 1);
    }

    #[test]
    fn test_events_in_range() {
        let mut b = bus();
        b.emit(S, 100, Lane::System, EventPayload::SystemNote { message: "a".into() });
        b.emit(S, 200, Lane::System, EventPayload::SystemNote { message: "b".into() });
        b.emit(S, 300, Lane::System, EventPayload::SystemNote { message: "c".into() });
        assert_eq!(b.events_in_range(150, 250).len(), 1);
    }

    #[test]
    fn test_cost_summary_accumulates() {
        let mut b = bus();
        b.emit(S, 0, Lane::Cost, EventPayload::CostAccrued { input_tokens: 1000, output_tokens: 500, usd: 0.01 });
        b.emit(S, 1, Lane::Cost, EventPayload::CostAccrued { input_tokens: 2000, output_tokens: 100, usd: 0.02 });
        let summary = b.cost_summary(S);
        assert_eq!(summary.input_tokens, 3000);
        assert_eq!(summary.output_tokens, 600);
        assert!((summary.total_usd - 0.03).abs() < 1e-9);
    }

    #[test]
    fn test_cost_summary_empty_session() {
        let b = bus();
        let summary = b.cost_summary("no-session");
        assert_eq!(summary.total_usd, 0.0);
    }

    #[test]
    fn test_lane_counts() {
        let mut b = bus();
        b.emit(S, 0, Lane::Tool, EventPayload::SystemNote { message: "t".into() });
        b.emit(S, 0, Lane::Tool, EventPayload::SystemNote { message: "t".into() });
        b.emit(S, 0, Lane::Error, EventPayload::ErrorEvent { message: "e".into(), recoverable: true });
        let counts = b.lane_counts(S);
        assert_eq!(counts["tool"], 2);
        assert_eq!(counts["error"], 1);
    }

    #[test]
    fn test_total_events() {
        let mut b = bus();
        b.emit(S, 0, Lane::User, EventPayload::SystemNote { message: "a".into() });
        b.emit(S, 0, Lane::User, EventPayload::SystemNote { message: "b".into() });
        assert_eq!(b.total_events(), 2);
    }

    #[test]
    fn test_clear() {
        let mut b = bus();
        b.emit(S, 0, Lane::User, EventPayload::SystemNote { message: "x".into() });
        b.clear();
        assert_eq!(b.total_events(), 0);
    }

    #[test]
    fn test_subscribe_records() {
        let mut b = bus();
        b.subscribe("observer-1", Lane::Error);
        assert_eq!(b.subscriptions["observer-1"], Lane::Error);
    }

    #[test]
    fn test_lane_display() {
        assert_eq!(Lane::Tool.to_string(), "tool");
        assert_eq!(Lane::Cost.to_string(), "cost");
    }


    #[test]
    fn test_tool_call_payload() {
        let mut b = bus();
        let id = b.emit(S, 0, Lane::Tool, EventPayload::ToolCall { name: "Read".into(), args_preview: "x.rs".into() });
        let ev = b.events_for_session(S).into_iter().find(|e| e.id == id).unwrap();
        assert!(matches!(&ev.payload, EventPayload::ToolCall { name, .. } if name == "Read"));
    }

    #[test]
    fn test_memory_write_payload() {
        let mut b = bus();
        b.emit(S, 0, Lane::Memory, EventPayload::MemoryWrite { key: "ctx".into(), bytes: 512 });
        let mem_events = b.events_in_lane(&Lane::Memory);
        assert_eq!(mem_events.len(), 1);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Workflow Lane Event System — 18 structured event types
// ══════════════════════════════════════════════════════════════════════════════
//
// Lanes represent concurrent agent workflows.  Events are emitted as agents
// progress through commits, branches, quality gates, and failure states.
//
// # Event categories
// - **Lifecycle** (3):  LaneStarted, LaneStopped, LaneResumed
// - **Commits**   (2):  CommitCreated, CommitSuperseded
// - **Branches**  (2):  BranchLocked, BranchUnlocked
// - **Quality**   (2):  QualityGreen, QualityRed
// - **Failures**  (9):  Build, Test, Lint, Merge, Timeout, Permission,
//                       Provider, Compaction, Unknown

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

static EVENT_COUNTER: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

fn next_event_id() -> String {
    let n = EVENT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("evt-{:08x}-{}", now_millis(), n)
}

// ── LaneEventType ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneEventType {
    // Lifecycle
    LaneStarted,
    LaneStopped,
    LaneResumed,
    // Commits
    CommitCreated,
    CommitSuperseded,
    // Branches
    BranchLocked,
    BranchUnlocked,
    // Quality
    QualityGreen,
    QualityRed,
    // Failures (9)
    FailureBuild,
    FailureTest,
    FailureLint,
    FailureMerge,
    FailureTimeout,
    FailurePermission,
    FailureProvider,
    FailureCompaction,
    FailureUnknown,
}

impl LaneEventType {
    pub fn all_variants() -> &'static [LaneEventType] {
        &[
            LaneEventType::LaneStarted,
            LaneEventType::LaneStopped,
            LaneEventType::LaneResumed,
            LaneEventType::CommitCreated,
            LaneEventType::CommitSuperseded,
            LaneEventType::BranchLocked,
            LaneEventType::BranchUnlocked,
            LaneEventType::QualityGreen,
            LaneEventType::QualityRed,
            LaneEventType::FailureBuild,
            LaneEventType::FailureTest,
            LaneEventType::FailureLint,
            LaneEventType::FailureMerge,
            LaneEventType::FailureTimeout,
            LaneEventType::FailurePermission,
            LaneEventType::FailureProvider,
            LaneEventType::FailureCompaction,
            LaneEventType::FailureUnknown,
        ]
    }

    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::FailureBuild
                | Self::FailureTest
                | Self::FailureLint
                | Self::FailureMerge
                | Self::FailureTimeout
                | Self::FailurePermission
                | Self::FailureProvider
                | Self::FailureCompaction
                | Self::FailureUnknown
        )
    }
}

impl std::fmt::Display for LaneEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::LaneStarted       => "lane_started",
            Self::LaneStopped       => "lane_stopped",
            Self::LaneResumed       => "lane_resumed",
            Self::CommitCreated     => "commit_created",
            Self::CommitSuperseded  => "commit_superseded",
            Self::BranchLocked      => "branch_locked",
            Self::BranchUnlocked    => "branch_unlocked",
            Self::QualityGreen      => "quality_green",
            Self::QualityRed        => "quality_red",
            Self::FailureBuild      => "failure_build",
            Self::FailureTest       => "failure_test",
            Self::FailureLint       => "failure_lint",
            Self::FailureMerge      => "failure_merge",
            Self::FailureTimeout    => "failure_timeout",
            Self::FailurePermission => "failure_permission",
            Self::FailureProvider   => "failure_provider",
            Self::FailureCompaction => "failure_compaction",
            Self::FailureUnknown    => "failure_unknown",
        };
        write!(f, "{s}")
    }
}

// ── WorkflowLaneEvent ─────────────────────────────────────────────────────────

/// A typed event emitted within a workflow lane.
///
/// Use [`LaneEventBuilder`] to construct instances — direct struct init is
/// intentionally omitted to guarantee a valid `id` and `timestamp`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLaneEvent {
    pub id: String,
    pub event_type: LaneEventType,
    pub lane_id: String,
    pub timestamp: u64,
    pub metadata: HashMap<String, String>,
}

// ── CommitProvenance ──────────────────────────────────────────────────────────

/// Tracks the origin and supersession state of a commit within a lane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitProvenance {
    pub commit_sha: String,
    pub canonical_ref: String,
    pub lane_id: String,
    pub superseded_by: Option<String>,
}

impl CommitProvenance {
    pub fn is_superseded(&self) -> bool {
        self.superseded_by.is_some()
    }
}

// ── LaneEventBuilder ──────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct LaneEventBuilder {
    event_type: Option<LaneEventType>,
    lane_id: Option<String>,
    metadata: HashMap<String, String>,
}

impl LaneEventBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn event_type(mut self, t: LaneEventType) -> Self {
        self.event_type = Some(t);
        self
    }

    pub fn lane_id(mut self, id: &str) -> Self {
        self.lane_id = Some(id.to_string());
        self
    }

    pub fn meta(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn build(self) -> Result<WorkflowLaneEvent, String> {
        let event_type = self.event_type.ok_or("event_type is required")?;
        let lane_id = self.lane_id.ok_or("lane_id is required")?;
        Ok(WorkflowLaneEvent {
            id: next_event_id(),
            event_type,
            lane_id,
            timestamp: now_millis(),
            metadata: self.metadata,
        })
    }
}

// ── Deduplication ─────────────────────────────────────────────────────────────

/// Remove `CommitCreated` events whose SHA has been superseded by a later
/// `CommitSuperseded` event.  Both event types must carry a `"sha"` metadata
/// key for the match to fire.
pub fn deduplicate_superseded(events: &[WorkflowLaneEvent]) -> Vec<&WorkflowLaneEvent> {
    let superseded_shas: std::collections::HashSet<&str> = events
        .iter()
        .filter(|e| e.event_type == LaneEventType::CommitSuperseded)
        .filter_map(|e| e.metadata.get("sha").map(|s| s.as_str()))
        .collect();

    events
        .iter()
        .filter(|e| {
            if e.event_type == LaneEventType::CommitCreated {
                !e.metadata
                    .get("sha")
                    .map(|s| superseded_shas.contains(s.as_str()))
                    .unwrap_or(false)
            } else {
                true
            }
        })
        .collect()
}

// ── Workflow Lane Event Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod workflow_tests {
    use super::*;

    #[test]
    fn event_type_count_is_18() {
        // 3 lifecycle + 2 commit + 2 branch + 2 quality + 9 failure = 18
        assert_eq!(LaneEventType::all_variants().len(), 18);
    }

    #[test]
    fn builder_produces_valid_event() {
        let evt = LaneEventBuilder::new()
            .event_type(LaneEventType::LaneStarted)
            .lane_id("lane-1")
            .build()
            .unwrap();
        assert!(!evt.id.is_empty());
        assert_eq!(evt.event_type, LaneEventType::LaneStarted);
        assert_eq!(evt.lane_id, "lane-1");
    }

    #[test]
    fn builder_fails_without_event_type() {
        let result = LaneEventBuilder::new().lane_id("lane-1").build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_fails_without_lane_id() {
        let result = LaneEventBuilder::new()
            .event_type(LaneEventType::LaneStarted)
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn event_ids_are_unique() {
        let e1 = LaneEventBuilder::new()
            .event_type(LaneEventType::LaneStarted)
            .lane_id("l")
            .build()
            .unwrap();
        let e2 = LaneEventBuilder::new()
            .event_type(LaneEventType::LaneStopped)
            .lane_id("l")
            .build()
            .unwrap();
        assert_ne!(e1.id, e2.id);
    }

    #[test]
    fn deduplicate_removes_superseded_commits() {
        let create = LaneEventBuilder::new()
            .event_type(LaneEventType::CommitCreated)
            .lane_id("l")
            .meta("sha", "abc123")
            .build()
            .unwrap();
        let supersede = LaneEventBuilder::new()
            .event_type(LaneEventType::CommitSuperseded)
            .lane_id("l")
            .meta("sha", "abc123")
            .build()
            .unwrap();
        let keep = LaneEventBuilder::new()
            .event_type(LaneEventType::CommitCreated)
            .lane_id("l")
            .meta("sha", "def456")
            .build()
            .unwrap();
        let events = vec![create, supersede, keep];
        let deduped = deduplicate_superseded(&events);
        assert!(!deduped.iter().any(|e| {
            e.event_type == LaneEventType::CommitCreated
                && e.metadata.get("sha").map(|s| s == "abc123").unwrap_or(false)
        }));
        assert!(deduped
            .iter()
            .any(|e| e.metadata.get("sha").map(|s| s == "def456").unwrap_or(false)));
    }

    #[test]
    fn commit_provenance_tracks_supersession() {
        let p = CommitProvenance {
            commit_sha: "abc".into(),
            canonical_ref: "refs/heads/main".into(),
            lane_id: "l1".into(),
            superseded_by: Some("def".into()),
        };
        assert!(p.is_superseded());
    }

    #[test]
    fn metadata_preserved_through_builder() {
        let evt = LaneEventBuilder::new()
            .event_type(LaneEventType::LaneStarted)
            .lane_id("l")
            .meta("key", "value")
            .build()
            .unwrap();
        assert_eq!(evt.metadata.get("key").unwrap(), "value");
    }
}
