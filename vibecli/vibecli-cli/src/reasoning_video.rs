//! Agent reasoning-to-video stream with thought overlay on screen capture.
//!
//! GAP-v9-009: rivals Devin Live Stream, Cursor Agent Screen Share.
//! - Maps agent thought tokens to screen-capture frames in real time
//! - Overlay rendering spec: thought bubbles, action highlights, cursor trails
//! - Frame annotation format (JSON sidecar per frame): timestamp, thought, action, confidence
//! - Live stream session lifecycle: start, pause, resume, stop, export
//! - Playback scrubbing: seek to any frame and replay associated thought sequence
//! - Export formats: annotated MP4 (spec), JSONL thoughts log, SVG overlay manifest

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Frame & Overlay ─────────────────────────────────────────────────────────

/// A single captured screen frame with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub index: u64,
    pub timestamp_ms: u64,
    pub width: u32,
    pub height: u32,
    pub format: FrameFormat,
    /// Path to raw frame file (PNG/JPEG) or base64 payload.
    pub source: FrameSource,
    pub annotations: Vec<FrameAnnotation>,
}

/// Frame encoding format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FrameFormat { Png, Jpeg, WebP }

/// Frame data source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FrameSource {
    FilePath(String),
    Base64(String),
    Placeholder, // for testing without actual screen capture
}

/// An overlay annotation on a frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameAnnotation {
    pub kind: AnnotationKind,
    pub x: f32, pub y: f32,       // 0.0 – 1.0 relative position
    pub w: f32, pub h: f32,       // relative dimensions
    pub text: String,
    pub color: String,            // CSS hex colour
    pub confidence: f32,          // 0.0 – 1.0
}

/// Types of overlay annotation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnnotationKind {
    ThoughtBubble,   // agent thought text
    ActionHighlight, // bounding box around active element
    CursorTrail,     // path the agent cursor is taking
    StatusBadge,     // phase indicator (thinking / acting / done)
    ErrorMarker,     // error location
}

// ─── Thought Token ────────────────────────────────────────────────────────────

/// A reasoning token emitted by the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtToken {
    pub id: String,
    pub timestamp_ms: u64,
    pub text: String,
    pub phase: ThoughtPhase,
    pub confidence: f32,
    pub linked_frame: Option<u64>, // frame index this thought maps to
}

/// Agent reasoning phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThoughtPhase {
    Planning,
    Executing,
    Verifying,
    Reflecting,
    Error,
}

impl ThoughtPhase {
    pub fn color(&self) -> &'static str {
        match self {
            Self::Planning   => "#4A90D9",
            Self::Executing  => "#27AE60",
            Self::Verifying  => "#F39C12",
            Self::Reflecting => "#8E44AD",
            Self::Error      => "#E74C3C",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Planning   => "Planning",
            Self::Executing  => "Executing",
            Self::Verifying  => "Verifying",
            Self::Reflecting => "Reflecting",
            Self::Error      => "Error",
        }
    }
}

// ─── Session Lifecycle ────────────────────────────────────────────────────────

/// Recording session state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionState {
    Idle,
    Recording,
    Paused,
    Stopped,
}

/// Export format for a completed session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExportFormat {
    AnnotatedMp4,    // video with burned-in overlays
    JsonlThoughts,   // line-delimited thought log
    SvgManifest,     // per-frame SVG overlay specs
    MarkdownReport,  // timestamped markdown summary
}

// ─── Reasoning Video Session ──────────────────────────────────────────────────

/// Live reasoning-to-video session.
pub struct ReasoningVideoSession {
    pub id: String,
    pub agent_id: String,
    state: SessionState,
    frames: Vec<Frame>,
    thoughts: Vec<ThoughtToken>,
    frame_counter: u64,
    thought_counter: u32,
    pub fps: u32,
    pub resolution: (u32, u32),
}

impl ReasoningVideoSession {
    pub fn new(id: &str, agent_id: &str, fps: u32, resolution: (u32, u32)) -> Self {
        Self {
            id: id.to_string(),
            agent_id: agent_id.to_string(),
            state: SessionState::Idle,
            frames: Vec::new(),
            thoughts: Vec::new(),
            frame_counter: 0,
            thought_counter: 0,
            fps,
            resolution,
        }
    }

    pub fn start(&mut self) -> bool {
        if self.state == SessionState::Idle || self.state == SessionState::Paused {
            self.state = SessionState::Recording;
            true
        } else { false }
    }

    pub fn pause(&mut self) -> bool {
        if self.state == SessionState::Recording {
            self.state = SessionState::Paused;
            true
        } else { false }
    }

    pub fn resume(&mut self) -> bool {
        if self.state == SessionState::Paused {
            self.state = SessionState::Recording;
            true
        } else { false }
    }

    pub fn stop(&mut self) -> bool {
        if self.state == SessionState::Recording || self.state == SessionState::Paused {
            self.state = SessionState::Stopped;
            true
        } else { false }
    }

    pub fn state(&self) -> &SessionState { &self.state }

    /// Capture a frame (placeholder in test mode).
    pub fn capture_frame(&mut self, timestamp_ms: u64) -> Option<u64> {
        if self.state != SessionState::Recording { return None; }
        let idx = self.frame_counter;
        self.frame_counter += 1;
        self.frames.push(Frame {
            index: idx,
            timestamp_ms,
            width: self.resolution.0,
            height: self.resolution.1,
            format: FrameFormat::Png,
            source: FrameSource::Placeholder,
            annotations: Vec::new(),
        });
        Some(idx)
    }

    /// Ingest a thought token and link it to the nearest frame.
    pub fn ingest_thought(&mut self, text: &str, phase: ThoughtPhase, confidence: f32, timestamp_ms: u64) -> String {
        self.thought_counter += 1;
        let id = format!("tht-{:05}", self.thought_counter);
        // Link to the most recent frame within 2 frame intervals
        let frame_interval = if self.fps > 0 { 1000 / self.fps as u64 } else { 33 };
        let linked = self.frames.iter().rev()
            .find(|f| timestamp_ms.saturating_sub(f.timestamp_ms) <= frame_interval * 2)
            .map(|f| f.index);
        self.thoughts.push(ThoughtToken {
            id: id.clone(),
            timestamp_ms,
            text: text.to_string(),
            phase,
            confidence,
            linked_frame: linked,
        });
        id
    }

    /// Generate overlay annotations for a specific frame from linked thoughts.
    pub fn overlays_for_frame(&self, frame_idx: u64) -> Vec<FrameAnnotation> {
        self.thoughts.iter()
            .filter(|t| t.linked_frame == Some(frame_idx))
            .map(|t| FrameAnnotation {
                kind: AnnotationKind::ThoughtBubble,
                x: 0.02,
                y: 0.02,
                w: 0.5,
                h: 0.08,
                text: t.text.clone(),
                color: t.phase.color().to_string(),
                confidence: t.confidence,
            })
            .collect()
    }

    /// Seek to a timestamp and return all thoughts up to that point.
    pub fn seek(&self, timestamp_ms: u64) -> Vec<&ThoughtToken> {
        self.thoughts.iter().filter(|t| t.timestamp_ms <= timestamp_ms).collect()
    }

    /// Export as a JSONL thoughts log (serialized lines).
    pub fn export_thoughts_jsonl(&self) -> Vec<String> {
        self.thoughts.iter()
            .map(|t| serde_json::to_string(t).unwrap_or_default())
            .collect()
    }

    /// Build a markdown report of the session.
    pub fn export_markdown(&self) -> String {
        let mut md = format!("# Reasoning Session: {}\n\nAgent: `{}`\n\nFrames: {}\nThoughts: {}\n\n",
            self.id, self.agent_id, self.frames.len(), self.thoughts.len());
        for t in &self.thoughts {
            md.push_str(&format!("- **[{}ms] {}**: {}\n", t.timestamp_ms, t.phase.label(), t.text));
        }
        md
    }

    /// Total duration in milliseconds (last frame timestamp).
    pub fn duration_ms(&self) -> u64 {
        self.frames.last().map(|f| f.timestamp_ms).unwrap_or(0)
    }

    pub fn frame_count(&self) -> usize { self.frames.len() }
    pub fn thought_count(&self) -> usize { self.thoughts.len() }
    pub fn frames(&self) -> &[Frame] { &self.frames }
    pub fn thoughts(&self) -> &[ThoughtToken] { &self.thoughts }

    /// Phase distribution of thoughts.
    pub fn phase_distribution(&self) -> HashMap<String, usize> {
        let mut map: HashMap<String, usize> = HashMap::new();
        for t in &self.thoughts {
            *map.entry(t.phase.label().to_string()).or_insert(0) += 1;
        }
        map
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session() -> ReasoningVideoSession {
        ReasoningVideoSession::new("s1", "agent-claude", 30, (1920, 1080))
    }

    // ── ThoughtPhase ──────────────────────────────────────────────────────

    #[test]
    fn test_phase_colors_are_hex() {
        for p in [ThoughtPhase::Planning, ThoughtPhase::Executing, ThoughtPhase::Verifying,
                  ThoughtPhase::Reflecting, ThoughtPhase::Error] {
            assert!(p.color().starts_with('#'));
            assert_eq!(p.color().len(), 7);
        }
    }

    #[test]
    fn test_phase_labels_non_empty() {
        for p in [ThoughtPhase::Planning, ThoughtPhase::Executing, ThoughtPhase::Error] {
            assert!(!p.label().is_empty());
        }
    }

    // ── Session lifecycle ─────────────────────────────────────────────────

    #[test]
    fn test_session_initial_state_idle() {
        let s = make_session();
        assert_eq!(s.state(), &SessionState::Idle);
    }

    #[test]
    fn test_session_start() {
        let mut s = make_session();
        assert!(s.start());
        assert_eq!(s.state(), &SessionState::Recording);
    }

    #[test]
    fn test_session_cannot_start_when_stopped() {
        let mut s = make_session();
        s.start(); s.stop();
        assert!(!s.start());
    }

    #[test]
    fn test_session_pause_and_resume() {
        let mut s = make_session();
        s.start();
        assert!(s.pause());
        assert_eq!(s.state(), &SessionState::Paused);
        assert!(s.resume());
        assert_eq!(s.state(), &SessionState::Recording);
    }

    #[test]
    fn test_session_pause_when_not_recording_fails() {
        let mut s = make_session();
        assert!(!s.pause()); // Idle → cannot pause
    }

    #[test]
    fn test_session_stop() {
        let mut s = make_session();
        s.start();
        assert!(s.stop());
        assert_eq!(s.state(), &SessionState::Stopped);
    }

    #[test]
    fn test_session_stop_when_idle_fails() {
        let mut s = make_session();
        assert!(!s.stop());
    }

    // ── capture_frame ─────────────────────────────────────────────────────

    #[test]
    fn test_capture_frame_when_recording() {
        let mut s = make_session();
        s.start();
        let idx = s.capture_frame(1000);
        assert!(idx.is_some());
        assert_eq!(s.frame_count(), 1);
    }

    #[test]
    fn test_capture_frame_when_not_recording_returns_none() {
        let mut s = make_session();
        let idx = s.capture_frame(1000);
        assert!(idx.is_none());
        assert_eq!(s.frame_count(), 0);
    }

    #[test]
    fn test_capture_frame_increments_index() {
        let mut s = make_session();
        s.start();
        let i1 = s.capture_frame(0).unwrap();
        let i2 = s.capture_frame(33).unwrap();
        assert_eq!(i2, i1 + 1);
    }

    #[test]
    fn test_captured_frame_has_correct_resolution() {
        let mut s = make_session();
        s.start();
        s.capture_frame(0);
        assert_eq!(s.frames()[0].width, 1920);
        assert_eq!(s.frames()[0].height, 1080);
    }

    // ── ingest_thought ────────────────────────────────────────────────────

    #[test]
    fn test_ingest_thought_returns_id() {
        let mut s = make_session();
        s.start();
        let id = s.ingest_thought("analyzing code", ThoughtPhase::Planning, 0.85, 100);
        assert!(id.starts_with("tht-"));
    }

    #[test]
    fn test_ingest_thought_increments_count() {
        let mut s = make_session();
        s.start();
        s.ingest_thought("t1", ThoughtPhase::Planning, 0.9, 0);
        s.ingest_thought("t2", ThoughtPhase::Executing, 0.8, 50);
        assert_eq!(s.thought_count(), 2);
    }

    #[test]
    fn test_ingest_thought_links_to_nearby_frame() {
        let mut s = make_session();
        s.start();
        s.capture_frame(0);  // frame 0 at t=0ms
        s.ingest_thought("plan", ThoughtPhase::Planning, 0.9, 20); // within 2 frame intervals at 30fps (~66ms)
        let t = &s.thoughts()[0];
        assert_eq!(t.linked_frame, Some(0));
    }

    #[test]
    fn test_ingest_thought_no_link_if_far_from_frame() {
        let mut s = make_session();
        s.start();
        s.capture_frame(0);  // frame 0 at t=0ms
        s.ingest_thought("late thought", ThoughtPhase::Reflecting, 0.5, 5000); // 5 seconds later
        let t = &s.thoughts()[0];
        assert!(t.linked_frame.is_none());
    }

    // ── overlays_for_frame ────────────────────────────────────────────────

    #[test]
    fn test_overlays_for_frame_linked() {
        let mut s = make_session();
        s.start();
        s.capture_frame(0);
        s.ingest_thought("I'm thinking", ThoughtPhase::Planning, 0.9, 10);
        let overlays = s.overlays_for_frame(0);
        assert_eq!(overlays.len(), 1);
        assert_eq!(overlays[0].kind, AnnotationKind::ThoughtBubble);
        assert!(overlays[0].text.contains("thinking"));
    }

    #[test]
    fn test_overlays_for_frame_unlinked_returns_empty() {
        let mut s = make_session();
        s.start();
        s.capture_frame(0);
        s.ingest_thought("far thought", ThoughtPhase::Error, 0.3, 9999);
        let overlays = s.overlays_for_frame(0);
        assert!(overlays.is_empty());
    }

    // ── seek ──────────────────────────────────────────────────────────────

    #[test]
    fn test_seek_returns_thoughts_up_to_timestamp() {
        let mut s = make_session();
        s.start();
        s.ingest_thought("early", ThoughtPhase::Planning, 0.9, 100);
        s.ingest_thought("late", ThoughtPhase::Executing, 0.8, 5000);
        let result = s.seek(500);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "early");
    }

    #[test]
    fn test_seek_all_thoughts_at_max_timestamp() {
        let mut s = make_session();
        s.start();
        s.ingest_thought("t1", ThoughtPhase::Planning, 0.9, 100);
        s.ingest_thought("t2", ThoughtPhase::Executing, 0.8, 200);
        let result = s.seek(u64::MAX);
        assert_eq!(result.len(), 2);
    }

    // ── export ────────────────────────────────────────────────────────────

    #[test]
    fn test_export_thoughts_jsonl_non_empty() {
        let mut s = make_session();
        s.start();
        s.ingest_thought("doing stuff", ThoughtPhase::Executing, 0.7, 100);
        let lines = s.export_thoughts_jsonl();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("Executing"));
    }

    #[test]
    fn test_export_markdown_contains_session_id() {
        let mut s = make_session();
        s.start();
        s.ingest_thought("plan", ThoughtPhase::Planning, 0.9, 0);
        let md = s.export_markdown();
        assert!(md.contains("s1"));
        assert!(md.contains("Planning"));
    }

    // ── duration & distribution ───────────────────────────────────────────

    #[test]
    fn test_duration_is_last_frame_timestamp() {
        let mut s = make_session();
        s.start();
        s.capture_frame(0);
        s.capture_frame(500);
        s.capture_frame(1000);
        assert_eq!(s.duration_ms(), 1000);
    }

    #[test]
    fn test_duration_zero_with_no_frames() {
        let s = make_session();
        assert_eq!(s.duration_ms(), 0);
    }

    #[test]
    fn test_phase_distribution_counts() {
        let mut s = make_session();
        s.start();
        s.ingest_thought("p1", ThoughtPhase::Planning, 0.9, 0);
        s.ingest_thought("p2", ThoughtPhase::Planning, 0.8, 50);
        s.ingest_thought("e1", ThoughtPhase::Executing, 0.7, 100);
        let dist = s.phase_distribution();
        assert_eq!(dist["Planning"], 2);
        assert_eq!(dist["Executing"], 1);
    }

    #[test]
    fn test_session_metadata() {
        let s = ReasoningVideoSession::new("vid-42", "my-agent", 60, (2560, 1440));
        assert_eq!(s.fps, 60);
        assert_eq!(s.resolution, (2560, 1440));
        assert_eq!(s.agent_id, "my-agent");
    }
}
