//! Observe-Act Loop — continuous visual grounding loop for agentic computer use.
//!
//! Implements the screenshot → LLM vision → action → verify → repeat pattern
//! used by Anthropic Computer Use, OpenClaw, and similar agent frameworks.
//!
//! The loop captures screenshots, sends them to a vision-capable LLM for reasoning,
//! executes the recommended actions, optionally verifies the result, and repeats
//! until the task is complete or a safety/limit condition is reached.
//!
//! Usage:
//! - `/observe start <task>` — start an observe-act session
//! - `/observe pause` — pause the running session
//! - `/observe resume` — resume a paused session
//! - `/observe abort <reason>` — abort with reason
//! - `/observe status` — show session summary
//! - `/observe history` — show step history
//! - `/observe config` — show/edit configuration

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

// ── SafetyMode ─────────────────────────────────────────────────────────────

/// Controls how much autonomy the observe-act loop has.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyMode {
    /// Confirm before destructive actions (default).
    #[default]
    Cautious,
    /// Fully autonomous — no confirmations.
    Autonomous,
    /// Read-only observation — no actions executed.
    Restricted,
}

impl std::fmt::Display for SafetyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cautious => write!(f, "Cautious"),
            Self::Autonomous => write!(f, "Autonomous"),
            Self::Restricted => write!(f, "Restricted"),
        }
    }
}

// ── ScrollDirection ────────────────────────────────────────────────────────

/// Direction for scroll actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl std::fmt::Display for ScrollDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Up => write!(f, "Up"),
            Self::Down => write!(f, "Down"),
            Self::Left => write!(f, "Left"),
            Self::Right => write!(f, "Right"),
        }
    }
}

// ── ObserveActAction ───────────────────────────────────────────────────────

/// An action the agent can perform on the screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ObserveActAction {
    /// Single click at screen coordinates.
    Click { x: u32, y: u32 },
    /// Double click at screen coordinates.
    DoubleClick { x: u32, y: u32 },
    /// Right click at screen coordinates.
    RightClick { x: u32, y: u32 },
    /// Type text via keyboard.
    Type { text: String },
    /// Press a key combination (e.g., \["ctrl", "c"\]).
    KeyCombo { keys: Vec<String> },
    /// Scroll in a direction by an amount.
    Scroll {
        direction: ScrollDirection,
        amount: u32,
    },
    /// Wait for a specified duration.
    Wait { ms: u64 },
    /// Capture a screenshot without acting.
    Screenshot,
    /// Move the mouse without clicking.
    MoveMouse { x: u32, y: u32 },
    /// Drag from one point to another.
    Drag {
        from_x: u32,
        from_y: u32,
        to_x: u32,
        to_y: u32,
    },
    /// Signal that the task is done.
    Done { summary: String },
}

impl std::fmt::Display for ObserveActAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Click { x, y } => write!(f, "Click({}, {})", x, y),
            Self::DoubleClick { x, y } => write!(f, "DoubleClick({}, {})", x, y),
            Self::RightClick { x, y } => write!(f, "RightClick({}, {})", x, y),
            Self::Type { text } => write!(f, "Type(\"{}\")", text),
            Self::KeyCombo { keys } => write!(f, "KeyCombo({})", keys.join("+")),
            Self::Scroll { direction, amount } => {
                write!(f, "Scroll({}, {})", direction, amount)
            }
            Self::Wait { ms } => write!(f, "Wait({}ms)", ms),
            Self::Screenshot => write!(f, "Screenshot"),
            Self::MoveMouse { x, y } => write!(f, "MoveMouse({}, {})", x, y),
            Self::Drag {
                from_x,
                from_y,
                to_x,
                to_y,
            } => write!(f, "Drag({},{} → {},{})", from_x, from_y, to_x, to_y),
            Self::Done { summary } => write!(f, "Done(\"{}\")", summary),
        }
    }
}

// ── ObserveActConfig ───────────────────────────────────────────────────────

/// Configuration for the observe-act loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserveActConfig {
    /// Milliseconds between observation cycles.
    pub observation_interval_ms: u64,
    /// Maximum number of steps before the session auto-completes.
    pub max_steps: usize,
    /// Number of consecutive failures before aborting.
    pub max_consecutive_failures: usize,
    /// Width of captured screenshots in pixels.
    pub screenshot_width: u32,
    /// Height of captured screenshots in pixels.
    pub screenshot_height: u32,
    /// Which vision provider to use for reasoning.
    pub vision_provider: String,
    /// Whether to capture a verification screenshot after each action.
    pub verify_after_action: bool,
    /// Safety mode controlling autonomy level.
    pub safety_mode: SafetyMode,
}

impl Default for ObserveActConfig {
    fn default() -> Self {
        Self {
            observation_interval_ms: 2000,
            max_steps: 50,
            max_consecutive_failures: 3,
            screenshot_width: 1280,
            screenshot_height: 720,
            vision_provider: "claude".to_string(),
            verify_after_action: true,
            safety_mode: SafetyMode::Cautious,
        }
    }
}

// ── VerificationResult ─────────────────────────────────────────────────────

/// Result of verifying an action's effect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// What we expected to see after the action.
    pub expected_change: String,
    /// What the LLM actually observed.
    pub actual_observation: String,
    /// Whether the verification passed.
    pub success: bool,
    /// Confidence score from 0.0 to 1.0.
    pub confidence: f64,
}

impl VerificationResult {
    /// Create a new verification result, clamping confidence to [0.0, 1.0].
    pub fn new(
        expected_change: String,
        actual_observation: String,
        success: bool,
        confidence: f64,
    ) -> Self {
        Self {
            expected_change,
            actual_observation,
            success,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

// ── ObservationStep ────────────────────────────────────────────────────────

/// A single step in the observe-act loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationStep {
    /// Sequential step number.
    pub step_num: usize,
    /// Unix timestamp in milliseconds when this step started.
    pub timestamp_ms: u64,
    /// Path to the screenshot captured at this step.
    pub screenshot_path: Option<String>,
    /// The LLM's reasoning about what it sees and what to do.
    pub llm_reasoning: String,
    /// Actions executed during this step.
    pub actions_taken: Vec<ObserveActAction>,
    /// Optional verification of the action's effect.
    pub verification_result: Option<VerificationResult>,
    /// How long this step took in milliseconds.
    pub duration_ms: u64,
}

// ── SessionStatus ──────────────────────────────────────────────────────────

/// Current status of an observe-act session.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Session created but not yet started.
    #[default]
    Idle,
    /// Actively running the observe-act loop.
    Running,
    /// Temporarily paused by the user.
    Paused,
    /// Task completed successfully.
    Completed,
    /// Session failed after exceeding failure limits.
    Failed,
    /// Session aborted by the user.
    Aborted,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Running => write!(f, "Running"),
            Self::Paused => write!(f, "Paused"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Aborted => write!(f, "Aborted"),
        }
    }
}

// ── ObserveActEvent ────────────────────────────────────────────────────────

/// Events emitted during the observe-act loop for streaming to callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ObserveActEvent {
    /// A new step has started.
    StepStarted { step_num: usize },
    /// A screenshot was captured.
    ScreenshotCaptured { path: String },
    /// The LLM produced reasoning.
    LlmReasoning { text: String },
    /// An action was executed.
    ActionExecuted {
        action: ObserveActAction,
        success: bool,
    },
    /// Verification of an action completed.
    VerificationDone { result: VerificationResult },
    /// The task is complete.
    TaskCompleted { summary: String },
    /// An error occurred.
    Error { message: String },
    /// Safety rails triggered a halt.
    SafetyHalt { reason: String },
}

// ── ScreenRegion ───────────────────────────────────────────────────────────

/// A rectangular region on the screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// Human-readable label for this region.
    pub label: String,
}

impl ScreenRegion {
    /// Check whether a point (px, py) falls within this region.
    pub fn contains_point(&self, px: u32, py: u32) -> bool {
        px >= self.x
            && px < self.x.saturating_add(self.width)
            && py >= self.y
            && py < self.y.saturating_add(self.height)
    }
}

// ── SafetyRails ────────────────────────────────────────────────────────────

/// Safety constraints for the observe-act loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRails {
    /// Screen regions where clicks are forbidden (e.g., system tray).
    pub forbidden_regions: Vec<ScreenRegion>,
    /// Maximum number of actions allowed per single step.
    pub max_actions_per_step: usize,
    /// Action patterns that require human confirmation.
    pub require_confirmation_for: Vec<String>,
    /// Key combinations that are never allowed (e.g., alt+f4).
    pub forbidden_key_combos: Vec<Vec<String>>,
    /// Minimum time in milliseconds between consecutive actions.
    pub rate_limit_ms: u64,
}

impl Default for SafetyRails {
    fn default() -> Self {
        Self {
            forbidden_regions: Vec::new(),
            max_actions_per_step: 5,
            require_confirmation_for: Vec::new(),
            forbidden_key_combos: vec![
                vec!["alt".to_string(), "f4".to_string()],
                vec!["ctrl".to_string(), "alt".to_string(), "del".to_string()],
            ],
            rate_limit_ms: 200,
        }
    }
}

// ── SessionSummary ─────────────────────────────────────────────────────────

/// Summary statistics for a completed or in-progress session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Total observation steps executed.
    pub total_steps: usize,
    /// Total individual actions executed across all steps.
    pub total_actions: usize,
    /// Fraction of verification checks that passed (0.0–1.0).
    pub success_rate: f64,
    /// Total wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Current or final session status.
    pub final_status: SessionStatus,
    /// The task description.
    pub task: String,
    /// Optional completion summary if the session finished.
    pub completion_summary: Option<String>,
}

// ── ObserveActSession ──────────────────────────────────────────────────────

/// The main session state for an observe-act loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserveActSession {
    /// Configuration for this session.
    pub config: ObserveActConfig,
    /// The high-level task description.
    pub task: String,
    /// Chronological list of observation steps.
    pub steps: Vec<ObservationStep>,
    /// Current session status.
    pub status: SessionStatus,
    /// Unix timestamp in milliseconds when the session started.
    pub started_at_ms: u64,
    /// Number of consecutive failed steps (resets on success).
    pub consecutive_failures: usize,
    /// Running total of individual actions executed.
    pub total_actions: usize,
}

impl ObserveActSession {
    /// Create a new session in the Idle state.
    pub fn new(config: ObserveActConfig, task: String) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        info!(task = %task, "Created new observe-act session");

        Self {
            config,
            task,
            steps: Vec::new(),
            status: SessionStatus::Idle,
            started_at_ms: now_ms,
            consecutive_failures: 0,
            total_actions: 0,
        }
    }

    /// Returns true if the session has reached a terminal state.
    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            SessionStatus::Completed | SessionStatus::Failed | SessionStatus::Aborted
        )
    }

    /// Returns true if the session can execute another step.
    pub fn can_continue(&self) -> bool {
        if self.is_complete() {
            return false;
        }
        if self.status == SessionStatus::Paused {
            return false;
        }
        if self.steps.len() >= self.config.max_steps {
            debug!(
                steps = self.steps.len(),
                max = self.config.max_steps,
                "Max steps reached"
            );
            return false;
        }
        if self.consecutive_failures >= self.config.max_consecutive_failures {
            debug!(
                failures = self.consecutive_failures,
                max = self.config.max_consecutive_failures,
                "Max consecutive failures reached"
            );
            return false;
        }
        true
    }

    /// Record a completed observation step. Updates failure tracking and action count.
    pub fn record_step(&mut self, step: ObservationStep) {
        let step_num = step.step_num;
        let action_count = step.actions_taken.len();

        // Track consecutive failures based on verification
        let step_succeeded = step
            .verification_result
            .as_ref()
            .map(|v| v.success)
            .unwrap_or(true); // No verification = assume success

        if step_succeeded {
            self.consecutive_failures = 0;
        } else {
            self.consecutive_failures += 1;
            warn!(
                step = step_num,
                consecutive_failures = self.consecutive_failures,
                "Step verification failed"
            );
        }

        self.total_actions += action_count;
        self.steps.push(step);

        debug!(
            step = step_num,
            actions = action_count,
            total_actions = self.total_actions,
            "Recorded observation step"
        );

        // Auto-transition to failed if we hit the limit
        if self.consecutive_failures >= self.config.max_consecutive_failures {
            self.status = SessionStatus::Failed;
            warn!("Session failed: max consecutive failures exceeded");
        }

        // Auto-transition to completed if we hit max steps
        if self.steps.len() >= self.config.max_steps && self.status == SessionStatus::Running {
            self.status = SessionStatus::Completed;
            info!("Session completed: max steps reached");
        }
    }

    /// Generate a summary of the current session state.
    pub fn get_summary(&self) -> SessionSummary {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let verified_steps: Vec<&VerificationResult> = self
            .steps
            .iter()
            .filter_map(|s| s.verification_result.as_ref())
            .collect();

        let success_rate = if verified_steps.is_empty() {
            1.0
        } else {
            let successes = verified_steps.iter().filter(|v| v.success).count();
            successes as f64 / verified_steps.len() as f64
        };

        let completion_summary = self
            .steps
            .iter()
            .rev()
            .flat_map(|s| s.actions_taken.iter())
            .find_map(|a| {
                if let ObserveActAction::Done { summary } = a {
                    Some(summary.clone())
                } else {
                    None
                }
            });

        SessionSummary {
            total_steps: self.steps.len(),
            total_actions: self.total_actions,
            success_rate,
            duration_ms: now_ms.saturating_sub(self.started_at_ms),
            final_status: self.status.clone(),
            task: self.task.clone(),
            completion_summary,
        }
    }

    /// Abort the session with a reason.
    pub fn abort(&mut self, reason: &str) {
        warn!(reason = reason, "Aborting observe-act session");
        self.status = SessionStatus::Aborted;
    }

    /// Pause the session (only if running).
    pub fn pause(&mut self) {
        if self.status == SessionStatus::Running {
            info!("Pausing observe-act session");
            self.status = SessionStatus::Paused;
        } else {
            warn!(
                status = %self.status,
                "Cannot pause session in current state"
            );
        }
    }

    /// Resume a paused session.
    pub fn resume(&mut self) {
        if self.status == SessionStatus::Paused {
            info!("Resuming observe-act session");
            self.status = SessionStatus::Running;
        } else {
            warn!(
                status = %self.status,
                "Cannot resume session in current state"
            );
        }
    }

    /// Transition to Running state (used when starting the loop).
    pub fn start(&mut self) {
        if self.status == SessionStatus::Idle {
            info!(task = %self.task, "Starting observe-act session");
            self.status = SessionStatus::Running;
        }
    }

    /// Mark the session as completed.
    pub fn complete(&mut self) {
        if self.status == SessionStatus::Running {
            info!("Observe-act session completed");
            self.status = SessionStatus::Completed;
        }
    }
}

// ── Action Validation ──────────────────────────────────────────────────────

/// Check whether an action is classified as destructive.
pub fn is_destructive(action: &ObserveActAction) -> bool {
    match action {
        // Key combos that could close, delete, or alter system state
        ObserveActAction::KeyCombo { keys } => {
            let lower: Vec<String> = keys.iter().map(|k| k.to_lowercase()).collect();
            // Delete, backspace, enter can be destructive
            lower.contains(&"delete".to_string())
                || lower.contains(&"backspace".to_string())
                // Ctrl+W (close tab), Ctrl+Q (quit), Ctrl+Z (undo), Ctrl+X (cut)
                || (lower.contains(&"ctrl".to_string())
                    && (lower.contains(&"w".to_string())
                        || lower.contains(&"q".to_string())
                        || lower.contains(&"x".to_string())))
                || (lower.contains(&"alt".to_string()) && lower.contains(&"f4".to_string()))
        }
        // Typing could be destructive in certain contexts
        ObserveActAction::Type { text } => {
            // Commands that modify file system or have side effects
            let lower = text.to_lowercase();
            lower.contains("rm ")
                || lower.contains("del ")
                || lower.contains("format ")
                || lower.contains("sudo ")
                || lower.contains("shutdown")
                || lower.contains("reboot")
        }
        // Drag can move/rearrange things
        ObserveActAction::Drag { .. } => true,
        // Click-based actions are generally not destructive by themselves
        _ => false,
    }
}

/// Validate an action against safety rails. Returns Ok(()) if allowed.
pub fn validate_action(action: &ObserveActAction, safety: &SafetyRails) -> Result<()> {
    // Check forbidden regions for coordinate-based actions
    let check_point = |x: u32, y: u32| -> Result<()> {
        for region in &safety.forbidden_regions {
            if region.contains_point(x, y) {
                return Err(anyhow!(
                    "Action targets forbidden region '{}' at ({}, {})",
                    region.label,
                    x,
                    y
                ));
            }
        }
        Ok(())
    };

    match action {
        ObserveActAction::Click { x, y }
        | ObserveActAction::DoubleClick { x, y }
        | ObserveActAction::RightClick { x, y }
        | ObserveActAction::MoveMouse { x, y } => {
            check_point(*x, *y)?;
        }
        ObserveActAction::Drag {
            from_x,
            from_y,
            to_x,
            to_y,
        } => {
            check_point(*from_x, *from_y)?;
            check_point(*to_x, *to_y)?;
        }
        ObserveActAction::KeyCombo { keys } => {
            let lower: Vec<String> = keys.iter().map(|k| k.to_lowercase()).collect();
            for forbidden in &safety.forbidden_key_combos {
                let forbidden_lower: Vec<String> =
                    forbidden.iter().map(|k| k.to_lowercase()).collect();
                if forbidden_lower.len() == lower.len()
                    && forbidden_lower.iter().all(|k| lower.contains(k))
                {
                    return Err(anyhow!(
                        "Key combo [{}] is forbidden",
                        keys.join("+")
                    ));
                }
            }
        }
        _ => {}
    }

    Ok(())
}

/// Validate a batch of actions against safety rails including the per-step limit.
pub fn validate_action_batch(actions: &[ObserveActAction], safety: &SafetyRails) -> Result<()> {
    if actions.len() > safety.max_actions_per_step {
        return Err(anyhow!(
            "Too many actions in step: {} exceeds limit of {}",
            actions.len(),
            safety.max_actions_per_step
        ));
    }
    for action in actions {
        validate_action(action, safety)?;
    }
    Ok(())
}

// ── LlmPromptBuilder ──────────────────────────────────────────────────────

/// Builds prompts for LLM vision interactions.
pub struct LlmPromptBuilder;

impl LlmPromptBuilder {
    /// Build an observation prompt that includes the task, step history, and current screenshot.
    pub fn build_observation_prompt(
        task: &str,
        step_history: &[ObservationStep],
        current_screenshot: &str,
    ) -> String {
        let mut prompt = String::with_capacity(2048);

        prompt.push_str("You are a computer-use agent executing a task by observing screenshots and performing actions.\n\n");
        prompt.push_str(&format!("## Task\n{}\n\n", task));

        if !step_history.is_empty() {
            prompt.push_str("## Previous Steps\n");
            for step in step_history {
                prompt.push_str(&format!(
                    "Step {}: {} — Actions: [{}]\n",
                    step.step_num,
                    step.llm_reasoning,
                    step.actions_taken
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                if let Some(ref v) = step.verification_result {
                    prompt.push_str(&format!(
                        "  Verification: {} (confidence: {:.0}%)\n",
                        if v.success { "PASS" } else { "FAIL" },
                        v.confidence * 100.0
                    ));
                }
            }
            prompt.push('\n');
        }

        prompt.push_str("## Current Screenshot\n");
        prompt.push_str(&format!("[Image: {}]\n\n", current_screenshot));

        prompt.push_str("## Instructions\n");
        prompt.push_str("Analyze the screenshot and determine the next actions to make progress on the task.\n");
        prompt.push_str("Respond with a JSON array of actions. Each action has a \"type\" field.\n\n");
        prompt.push_str("Available action types:\n");
        prompt.push_str("- {\"type\": \"click\", \"x\": <num>, \"y\": <num>}\n");
        prompt.push_str("- {\"type\": \"double_click\", \"x\": <num>, \"y\": <num>}\n");
        prompt.push_str("- {\"type\": \"right_click\", \"x\": <num>, \"y\": <num>}\n");
        prompt.push_str("- {\"type\": \"type\", \"text\": \"<string>\"}\n");
        prompt.push_str("- {\"type\": \"key_combo\", \"keys\": [\"ctrl\", \"s\"]}\n");
        prompt.push_str(
            "- {\"type\": \"scroll\", \"direction\": \"up\"|\"down\"|\"left\"|\"right\", \"amount\": <num>}\n",
        );
        prompt.push_str("- {\"type\": \"wait\", \"ms\": <num>}\n");
        prompt.push_str("- {\"type\": \"screenshot\"}\n");
        prompt.push_str("- {\"type\": \"move_mouse\", \"x\": <num>, \"y\": <num>}\n");
        prompt.push_str("- {\"type\": \"drag\", \"from_x\": <num>, \"from_y\": <num>, \"to_x\": <num>, \"to_y\": <num>}\n");
        prompt.push_str("- {\"type\": \"done\", \"summary\": \"<string>\"}\n\n");
        prompt.push_str("Respond ONLY with the JSON array, no other text.\n");

        prompt
    }

    /// Parse a JSON array of actions from an LLM response.
    pub fn parse_actions(llm_response: &str) -> Vec<ObserveActAction> {
        // Try to find a JSON array in the response
        let trimmed = llm_response.trim();

        // Try direct parse first
        if let Ok(actions) = serde_json::from_str::<Vec<ObserveActAction>>(trimmed) {
            return actions;
        }

        // Try to extract a JSON array from surrounding text
        if let Some(start) = trimmed.find('[') {
            if let Some(end) = trimmed.rfind(']') {
                let json_str = &trimmed[start..=end];
                if let Ok(actions) = serde_json::from_str::<Vec<ObserveActAction>>(json_str) {
                    return actions;
                }
            }
        }

        warn!("Failed to parse actions from LLM response");
        Vec::new()
    }

    /// Build a prompt to verify the effect of an action.
    pub fn build_verification_prompt(expected: &str, screenshot: &str) -> String {
        let mut prompt = String::with_capacity(512);
        prompt.push_str(
            "You are verifying whether an action had the expected effect on the screen.\n\n",
        );
        prompt.push_str(&format!("## Expected Change\n{}\n\n", expected));
        prompt.push_str(&format!("## Current Screenshot\n[Image: {}]\n\n", screenshot));
        prompt.push_str("Respond with a JSON object:\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"actual_observation\": \"<what you see>\",\n");
        prompt.push_str("  \"success\": true|false,\n");
        prompt.push_str("  \"confidence\": <0.0 to 1.0>\n");
        prompt.push_str("}\n");
        prompt
    }
}

// ── Utility ────────────────────────────────────────────────────────────────

/// Get current time as Unix milliseconds.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Config Tests ───────────────────────────────────────────────────

    #[test]
    fn test_config_defaults() {
        let config = ObserveActConfig::default();
        assert_eq!(config.observation_interval_ms, 2000);
        assert_eq!(config.max_steps, 50);
        assert_eq!(config.max_consecutive_failures, 3);
        assert_eq!(config.screenshot_width, 1280);
        assert_eq!(config.screenshot_height, 720);
        assert_eq!(config.vision_provider, "claude");
        assert!(config.verify_after_action);
        assert_eq!(config.safety_mode, SafetyMode::Cautious);
    }

    #[test]
    fn test_config_custom_values() {
        let config = ObserveActConfig {
            observation_interval_ms: 500,
            max_steps: 100,
            max_consecutive_failures: 5,
            screenshot_width: 1920,
            screenshot_height: 1080,
            vision_provider: "openai".to_string(),
            verify_after_action: false,
            safety_mode: SafetyMode::Autonomous,
        };
        assert_eq!(config.observation_interval_ms, 500);
        assert_eq!(config.max_steps, 100);
        assert_eq!(config.vision_provider, "openai");
        assert!(!config.verify_after_action);
        assert_eq!(config.safety_mode, SafetyMode::Autonomous);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = ObserveActConfig::default();
        let json = serde_json::to_string(&config).expect("serialize");
        let deserialized: ObserveActConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.max_steps, config.max_steps);
        assert_eq!(deserialized.safety_mode, config.safety_mode);
    }

    // ── SafetyMode Tests ───────────────────────────────────────────────

    #[test]
    fn test_safety_mode_serialization() {
        let modes = vec![SafetyMode::Cautious, SafetyMode::Autonomous, SafetyMode::Restricted];
        for mode in modes {
            let json = serde_json::to_string(&mode).expect("serialize");
            let deserialized: SafetyMode = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(deserialized, mode);
        }
    }

    #[test]
    fn test_safety_mode_display() {
        assert_eq!(SafetyMode::Cautious.to_string(), "Cautious");
        assert_eq!(SafetyMode::Autonomous.to_string(), "Autonomous");
        assert_eq!(SafetyMode::Restricted.to_string(), "Restricted");
    }

    #[test]
    fn test_safety_mode_default() {
        assert_eq!(SafetyMode::default(), SafetyMode::Cautious);
    }

    // ── Action Tests ───────────────────────────────────────────────────

    #[test]
    fn test_action_serialization_click() {
        let action = ObserveActAction::Click { x: 100, y: 200 };
        let json = serde_json::to_string(&action).expect("serialize");
        assert!(json.contains("\"type\":\"click\""));
        let deserialized: ObserveActAction = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, action);
    }

    #[test]
    fn test_action_serialization_all_variants() {
        let actions = vec![
            ObserveActAction::Click { x: 10, y: 20 },
            ObserveActAction::DoubleClick { x: 30, y: 40 },
            ObserveActAction::RightClick { x: 50, y: 60 },
            ObserveActAction::Type { text: "hello".into() },
            ObserveActAction::KeyCombo { keys: vec!["ctrl".into(), "c".into()] },
            ObserveActAction::Scroll { direction: ScrollDirection::Down, amount: 3 },
            ObserveActAction::Wait { ms: 1000 },
            ObserveActAction::Screenshot,
            ObserveActAction::MoveMouse { x: 70, y: 80 },
            ObserveActAction::Drag { from_x: 0, from_y: 0, to_x: 100, to_y: 100 },
            ObserveActAction::Done { summary: "Finished".into() },
        ];
        for action in &actions {
            let json = serde_json::to_string(action).expect("serialize");
            let deserialized: ObserveActAction = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(&deserialized, action);
        }
    }

    #[test]
    fn test_action_display() {
        assert_eq!(
            ObserveActAction::Click { x: 10, y: 20 }.to_string(),
            "Click(10, 20)"
        );
        assert_eq!(
            ObserveActAction::Type { text: "hi".into() }.to_string(),
            "Type(\"hi\")"
        );
        assert_eq!(
            ObserveActAction::KeyCombo { keys: vec!["ctrl".into(), "s".into()] }.to_string(),
            "KeyCombo(ctrl+s)"
        );
        assert_eq!(ObserveActAction::Screenshot.to_string(), "Screenshot");
        assert_eq!(
            ObserveActAction::Drag { from_x: 1, from_y: 2, to_x: 3, to_y: 4 }.to_string(),
            "Drag(1,2 → 3,4)"
        );
        assert_eq!(
            ObserveActAction::Done { summary: "done".into() }.to_string(),
            "Done(\"done\")"
        );
    }

    #[test]
    fn test_scroll_direction_display() {
        assert_eq!(ScrollDirection::Up.to_string(), "Up");
        assert_eq!(ScrollDirection::Down.to_string(), "Down");
        assert_eq!(ScrollDirection::Left.to_string(), "Left");
        assert_eq!(ScrollDirection::Right.to_string(), "Right");
    }

    // ── ScreenRegion Tests ─────────────────────────────────────────────

    #[test]
    fn test_screen_region_contains_point_inside() {
        let region = ScreenRegion {
            x: 100, y: 100, width: 200, height: 150, label: "test".into(),
        };
        assert!(region.contains_point(100, 100)); // top-left corner
        assert!(region.contains_point(200, 175)); // center
        assert!(region.contains_point(299, 249)); // just inside bottom-right
    }

    #[test]
    fn test_screen_region_contains_point_outside() {
        let region = ScreenRegion {
            x: 100, y: 100, width: 200, height: 150, label: "test".into(),
        };
        assert!(!region.contains_point(99, 100));  // just left
        assert!(!region.contains_point(100, 99));  // just above
        assert!(!region.contains_point(300, 100)); // just right
        assert!(!region.contains_point(100, 250)); // just below
        assert!(!region.contains_point(0, 0));     // far away
    }

    #[test]
    fn test_screen_region_contains_point_zero_size() {
        let region = ScreenRegion {
            x: 50, y: 50, width: 0, height: 0, label: "zero".into(),
        };
        assert!(!region.contains_point(50, 50));
    }

    #[test]
    fn test_screen_region_contains_point_overflow_safe() {
        let region = ScreenRegion {
            x: u32::MAX - 10, y: u32::MAX - 10, width: 100, height: 100,
            label: "edge".into(),
        };
        // Should not panic from overflow; saturating_add clamps to MAX
        // so contains_point checks px < MAX, which is true for MAX-5
        assert!(region.contains_point(u32::MAX - 5, u32::MAX - 5));
        // u32::MAX is NOT < u32::MAX (saturated), so this is outside
        assert!(!region.contains_point(u32::MAX, u32::MAX));
        // But anything within the non-saturated range works
        assert!(region.contains_point(u32::MAX - 1, u32::MAX - 1));
    }

    // ── Session Lifecycle Tests ────────────────────────────────────────

    #[test]
    fn test_session_new() {
        let session = ObserveActSession::new(
            ObserveActConfig::default(),
            "Open the browser".into(),
        );
        assert_eq!(session.status, SessionStatus::Idle);
        assert_eq!(session.task, "Open the browser");
        assert!(session.steps.is_empty());
        assert_eq!(session.consecutive_failures, 0);
        assert_eq!(session.total_actions, 0);
    }

    #[test]
    fn test_session_start() {
        let mut session = ObserveActSession::new(ObserveActConfig::default(), "task".into());
        session.start();
        assert_eq!(session.status, SessionStatus::Running);
    }

    #[test]
    fn test_session_lifecycle_to_completed() {
        let mut session = ObserveActSession::new(
            ObserveActConfig { max_steps: 2, ..Default::default() },
            "task".into(),
        );
        session.start();
        assert_eq!(session.status, SessionStatus::Running);

        session.record_step(make_step(1, true));
        assert_eq!(session.status, SessionStatus::Running);

        session.record_step(make_step(2, true));
        assert_eq!(session.status, SessionStatus::Completed);
    }

    #[test]
    fn test_session_lifecycle_to_failed() {
        let mut session = ObserveActSession::new(
            ObserveActConfig {
                max_consecutive_failures: 3,
                ..Default::default()
            },
            "task".into(),
        );
        session.start();

        session.record_step(make_step(1, false));
        assert_eq!(session.status, SessionStatus::Running);
        assert_eq!(session.consecutive_failures, 1);

        session.record_step(make_step(2, false));
        assert_eq!(session.status, SessionStatus::Running);
        assert_eq!(session.consecutive_failures, 2);

        session.record_step(make_step(3, false));
        assert_eq!(session.status, SessionStatus::Failed);
        assert_eq!(session.consecutive_failures, 3);
    }

    #[test]
    fn test_session_consecutive_failure_reset_on_success() {
        let mut session = ObserveActSession::new(
            ObserveActConfig {
                max_consecutive_failures: 3,
                ..Default::default()
            },
            "task".into(),
        );
        session.start();

        session.record_step(make_step(1, false));
        session.record_step(make_step(2, false));
        assert_eq!(session.consecutive_failures, 2);

        // A success resets the counter
        session.record_step(make_step(3, true));
        assert_eq!(session.consecutive_failures, 0);
    }

    #[test]
    fn test_session_step_recording_tracks_actions() {
        let mut session = ObserveActSession::new(ObserveActConfig::default(), "task".into());
        session.start();

        let step = ObservationStep {
            step_num: 1,
            timestamp_ms: now_ms(),
            screenshot_path: Some("/tmp/screen1.png".into()),
            llm_reasoning: "I see a button".into(),
            actions_taken: vec![
                ObserveActAction::Click { x: 50, y: 50 },
                ObserveActAction::Type { text: "hello".into() },
            ],
            verification_result: None,
            duration_ms: 500,
        };

        session.record_step(step);
        assert_eq!(session.steps.len(), 1);
        assert_eq!(session.total_actions, 2);
    }

    #[test]
    fn test_session_is_complete() {
        let mut session = ObserveActSession::new(ObserveActConfig::default(), "task".into());
        assert!(!session.is_complete());

        session.status = SessionStatus::Running;
        assert!(!session.is_complete());

        session.status = SessionStatus::Paused;
        assert!(!session.is_complete());

        session.status = SessionStatus::Completed;
        assert!(session.is_complete());

        session.status = SessionStatus::Failed;
        assert!(session.is_complete());

        session.status = SessionStatus::Aborted;
        assert!(session.is_complete());
    }

    #[test]
    fn test_session_can_continue() {
        let mut session = ObserveActSession::new(
            ObserveActConfig { max_steps: 5, max_consecutive_failures: 2, ..Default::default() },
            "task".into(),
        );

        // Idle can continue (will be started)
        assert!(session.can_continue());

        session.start();
        assert!(session.can_continue());

        session.pause();
        assert!(!session.can_continue());

        session.resume();
        assert!(session.can_continue());

        session.status = SessionStatus::Completed;
        assert!(!session.can_continue());
    }

    #[test]
    fn test_session_can_continue_max_steps() {
        let mut session = ObserveActSession::new(
            ObserveActConfig { max_steps: 1, ..Default::default() },
            "task".into(),
        );
        session.start();
        assert!(session.can_continue());

        session.record_step(make_step(1, true));
        // After recording max steps, can_continue should return false
        assert!(!session.can_continue());
    }

    #[test]
    fn test_session_can_continue_max_failures() {
        let mut session = ObserveActSession::new(
            ObserveActConfig { max_consecutive_failures: 1, ..Default::default() },
            "task".into(),
        );
        session.start();

        session.record_step(make_step(1, false));
        // Status should be Failed, can_continue returns false
        assert!(!session.can_continue());
    }

    // ── Pause / Resume / Abort ─────────────────────────────────────────

    #[test]
    fn test_session_pause_resume() {
        let mut session = ObserveActSession::new(ObserveActConfig::default(), "task".into());
        session.start();
        assert_eq!(session.status, SessionStatus::Running);

        session.pause();
        assert_eq!(session.status, SessionStatus::Paused);

        session.resume();
        assert_eq!(session.status, SessionStatus::Running);
    }

    #[test]
    fn test_session_pause_only_from_running() {
        let mut session = ObserveActSession::new(ObserveActConfig::default(), "task".into());
        // Idle state — pause should not change status
        session.pause();
        assert_eq!(session.status, SessionStatus::Idle);
    }

    #[test]
    fn test_session_resume_only_from_paused() {
        let mut session = ObserveActSession::new(ObserveActConfig::default(), "task".into());
        session.start();
        // Running state — resume should not change status
        session.resume();
        assert_eq!(session.status, SessionStatus::Running);
    }

    #[test]
    fn test_session_abort() {
        let mut session = ObserveActSession::new(ObserveActConfig::default(), "task".into());
        session.start();
        session.abort("User requested stop");
        assert_eq!(session.status, SessionStatus::Aborted);
        assert!(session.is_complete());
    }

    // ── Summary ────────────────────────────────────────────────────────

    #[test]
    fn test_session_summary_empty() {
        let session = ObserveActSession::new(ObserveActConfig::default(), "my task".into());
        let summary = session.get_summary();
        assert_eq!(summary.total_steps, 0);
        assert_eq!(summary.total_actions, 0);
        assert_eq!(summary.success_rate, 1.0); // no verified steps = 100%
        assert_eq!(summary.task, "my task");
        assert_eq!(summary.final_status, SessionStatus::Idle);
        assert!(summary.completion_summary.is_none());
    }

    #[test]
    fn test_session_summary_with_steps() {
        let mut session = ObserveActSession::new(ObserveActConfig::default(), "task".into());
        session.start();

        session.record_step(make_step(1, true));
        session.record_step(make_step(2, false));

        let summary = session.get_summary();
        assert_eq!(summary.total_steps, 2);
        assert_eq!(summary.total_actions, 2); // 1 action per step
        assert!((summary.success_rate - 0.5).abs() < f64::EPSILON);
        assert_eq!(summary.final_status, SessionStatus::Running);
    }

    #[test]
    fn test_session_summary_with_done_action() {
        let mut session = ObserveActSession::new(ObserveActConfig::default(), "task".into());
        session.start();

        let step = ObservationStep {
            step_num: 1,
            timestamp_ms: now_ms(),
            screenshot_path: None,
            llm_reasoning: "Task is finished".into(),
            actions_taken: vec![ObserveActAction::Done {
                summary: "Opened browser and navigated".into(),
            }],
            verification_result: None,
            duration_ms: 100,
        };
        session.record_step(step);

        let summary = session.get_summary();
        assert_eq!(
            summary.completion_summary,
            Some("Opened browser and navigated".into())
        );
    }

    // ── SessionStatus Display ──────────────────────────────────────────

    #[test]
    fn test_session_status_display() {
        assert_eq!(SessionStatus::Idle.to_string(), "Idle");
        assert_eq!(SessionStatus::Running.to_string(), "Running");
        assert_eq!(SessionStatus::Paused.to_string(), "Paused");
        assert_eq!(SessionStatus::Completed.to_string(), "Completed");
        assert_eq!(SessionStatus::Failed.to_string(), "Failed");
        assert_eq!(SessionStatus::Aborted.to_string(), "Aborted");
    }

    #[test]
    fn test_session_status_serialization() {
        let statuses = vec![
            SessionStatus::Idle,
            SessionStatus::Running,
            SessionStatus::Paused,
            SessionStatus::Completed,
            SessionStatus::Failed,
            SessionStatus::Aborted,
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).expect("serialize");
            let deserialized: SessionStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(deserialized, status);
        }
    }

    // ── LLM Prompt Builder Tests ───────────────────────────────────────

    #[test]
    fn test_observation_prompt_no_history() {
        let prompt = LlmPromptBuilder::build_observation_prompt(
            "Click the Save button",
            &[],
            "/tmp/screenshot.png",
        );
        assert!(prompt.contains("Click the Save button"));
        assert!(prompt.contains("/tmp/screenshot.png"));
        assert!(!prompt.contains("Previous Steps"));
    }

    #[test]
    fn test_observation_prompt_with_history() {
        let steps = vec![make_step(1, true)];
        let prompt = LlmPromptBuilder::build_observation_prompt(
            "Click the Save button",
            &steps,
            "/tmp/screenshot.png",
        );
        assert!(prompt.contains("Previous Steps"));
        assert!(prompt.contains("Step 1"));
        assert!(prompt.contains("PASS"));
    }

    #[test]
    fn test_observation_prompt_contains_action_types() {
        let prompt = LlmPromptBuilder::build_observation_prompt("task", &[], "/tmp/s.png");
        assert!(prompt.contains("\"type\": \"click\""));
        assert!(prompt.contains("\"type\": \"type\""));
        assert!(prompt.contains("\"type\": \"key_combo\""));
        assert!(prompt.contains("\"type\": \"scroll\""));
        assert!(prompt.contains("\"type\": \"done\""));
        assert!(prompt.contains("\"type\": \"drag\""));
    }

    #[test]
    fn test_parse_actions_valid_json() {
        let input = r#"[{"type": "click", "x": 100, "y": 200}, {"type": "type", "text": "hello"}]"#;
        let actions = LlmPromptBuilder::parse_actions(input);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], ObserveActAction::Click { x: 100, y: 200 });
        assert_eq!(
            actions[1],
            ObserveActAction::Type { text: "hello".into() }
        );
    }

    #[test]
    fn test_parse_actions_with_surrounding_text() {
        let input = r#"Here are the actions:
[{"type": "click", "x": 50, "y": 60}]
That should work."#;
        let actions = LlmPromptBuilder::parse_actions(input);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], ObserveActAction::Click { x: 50, y: 60 });
    }

    #[test]
    fn test_parse_actions_malformed_input() {
        let actions = LlmPromptBuilder::parse_actions("this is not JSON at all");
        assert!(actions.is_empty());
    }

    #[test]
    fn test_parse_actions_empty_array() {
        let actions = LlmPromptBuilder::parse_actions("[]");
        assert!(actions.is_empty());
    }

    #[test]
    fn test_parse_actions_partial_json() {
        let actions = LlmPromptBuilder::parse_actions("[{\"type\": \"click\", \"x\": 10");
        assert!(actions.is_empty());
    }

    #[test]
    fn test_verification_prompt() {
        let prompt = LlmPromptBuilder::build_verification_prompt(
            "A dialog should appear",
            "/tmp/after.png",
        );
        assert!(prompt.contains("A dialog should appear"));
        assert!(prompt.contains("/tmp/after.png"));
        assert!(prompt.contains("actual_observation"));
        assert!(prompt.contains("confidence"));
    }

    // ── Verification Result Tests ──────────────────────────────────────

    #[test]
    fn test_verification_result_confidence_clamped() {
        let v = VerificationResult::new("x".into(), "y".into(), true, 1.5);
        assert!((v.confidence - 1.0).abs() < f64::EPSILON);

        let v = VerificationResult::new("x".into(), "y".into(), false, -0.5);
        assert!(v.confidence.abs() < f64::EPSILON);
    }

    #[test]
    fn test_verification_result_normal_confidence() {
        let v = VerificationResult::new("x".into(), "y".into(), true, 0.85);
        assert!((v.confidence - 0.85).abs() < f64::EPSILON);
    }

    // ── Safety Validation Tests ────────────────────────────────────────

    #[test]
    fn test_validate_action_forbidden_region() {
        let safety = SafetyRails {
            forbidden_regions: vec![ScreenRegion {
                x: 0, y: 0, width: 100, height: 50,
                label: "System tray".into(),
            }],
            ..Default::default()
        };

        let result = validate_action(
            &ObserveActAction::Click { x: 50, y: 25 },
            &safety,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("System tray"));
    }

    #[test]
    fn test_validate_action_outside_forbidden_region() {
        let safety = SafetyRails {
            forbidden_regions: vec![ScreenRegion {
                x: 0, y: 0, width: 100, height: 50,
                label: "System tray".into(),
            }],
            ..Default::default()
        };

        let result = validate_action(
            &ObserveActAction::Click { x: 200, y: 200 },
            &safety,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_action_forbidden_key_combo() {
        let safety = SafetyRails::default(); // includes alt+f4 and ctrl+alt+del

        let result = validate_action(
            &ObserveActAction::KeyCombo {
                keys: vec!["Alt".into(), "F4".into()],
            },
            &safety,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("forbidden"));
    }

    #[test]
    fn test_validate_action_allowed_key_combo() {
        let safety = SafetyRails::default();

        let result = validate_action(
            &ObserveActAction::KeyCombo {
                keys: vec!["ctrl".into(), "s".into()],
            },
            &safety,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_action_forbidden_ctrl_alt_del() {
        let safety = SafetyRails::default();

        let result = validate_action(
            &ObserveActAction::KeyCombo {
                keys: vec!["Ctrl".into(), "Alt".into(), "Del".into()],
            },
            &safety,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_action_drag_forbidden_region() {
        let safety = SafetyRails {
            forbidden_regions: vec![ScreenRegion {
                x: 500, y: 500, width: 100, height: 100,
                label: "Danger zone".into(),
            }],
            ..Default::default()
        };

        // from point in forbidden region
        let result = validate_action(
            &ObserveActAction::Drag { from_x: 550, from_y: 550, to_x: 100, to_y: 100 },
            &safety,
        );
        assert!(result.is_err());

        // to point in forbidden region
        let result = validate_action(
            &ObserveActAction::Drag { from_x: 100, from_y: 100, to_x: 550, to_y: 550 },
            &safety,
        );
        assert!(result.is_err());

        // both points outside
        let result = validate_action(
            &ObserveActAction::Drag { from_x: 100, from_y: 100, to_x: 200, to_y: 200 },
            &safety,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_action_batch_too_many() {
        let safety = SafetyRails {
            max_actions_per_step: 2,
            ..Default::default()
        };

        let actions = vec![
            ObserveActAction::Click { x: 1, y: 1 },
            ObserveActAction::Click { x: 2, y: 2 },
            ObserveActAction::Click { x: 3, y: 3 },
        ];

        let result = validate_action_batch(&actions, &safety);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Too many actions"));
    }

    #[test]
    fn test_validate_action_batch_within_limit() {
        let safety = SafetyRails {
            max_actions_per_step: 5,
            ..Default::default()
        };

        let actions = vec![
            ObserveActAction::Click { x: 1, y: 1 },
            ObserveActAction::Type { text: "hi".into() },
        ];

        assert!(validate_action_batch(&actions, &safety).is_ok());
    }

    // ── Destructive Action Classification ──────────────────────────────

    #[test]
    fn test_is_destructive_key_combos() {
        assert!(is_destructive(&ObserveActAction::KeyCombo {
            keys: vec!["ctrl".into(), "w".into()],
        }));
        assert!(is_destructive(&ObserveActAction::KeyCombo {
            keys: vec!["ctrl".into(), "q".into()],
        }));
        assert!(is_destructive(&ObserveActAction::KeyCombo {
            keys: vec!["alt".into(), "f4".into()],
        }));
        assert!(is_destructive(&ObserveActAction::KeyCombo {
            keys: vec!["Delete".into()],
        }));
        assert!(is_destructive(&ObserveActAction::KeyCombo {
            keys: vec!["Backspace".into()],
        }));
    }

    #[test]
    fn test_is_destructive_safe_key_combos() {
        assert!(!is_destructive(&ObserveActAction::KeyCombo {
            keys: vec!["ctrl".into(), "s".into()],
        }));
        assert!(!is_destructive(&ObserveActAction::KeyCombo {
            keys: vec!["ctrl".into(), "c".into()],
        }));
    }

    #[test]
    fn test_is_destructive_type_commands() {
        assert!(is_destructive(&ObserveActAction::Type {
            text: "rm -rf /".into(),
        }));
        assert!(is_destructive(&ObserveActAction::Type {
            text: "sudo reboot".into(),
        }));
        assert!(!is_destructive(&ObserveActAction::Type {
            text: "echo hello".into(),
        }));
    }

    #[test]
    fn test_is_destructive_drag() {
        assert!(is_destructive(&ObserveActAction::Drag {
            from_x: 0, from_y: 0, to_x: 100, to_y: 100,
        }));
    }

    #[test]
    fn test_is_destructive_non_destructive_actions() {
        assert!(!is_destructive(&ObserveActAction::Click { x: 10, y: 20 }));
        assert!(!is_destructive(&ObserveActAction::Screenshot));
        assert!(!is_destructive(&ObserveActAction::Wait { ms: 500 }));
        assert!(!is_destructive(&ObserveActAction::MoveMouse { x: 10, y: 20 }));
        assert!(!is_destructive(&ObserveActAction::Scroll {
            direction: ScrollDirection::Down,
            amount: 3,
        }));
        assert!(!is_destructive(&ObserveActAction::Done {
            summary: "done".into(),
        }));
    }

    // ── Event Serialization ────────────────────────────────────────────

    #[test]
    fn test_event_serialization() {
        let events = vec![
            ObserveActEvent::StepStarted { step_num: 1 },
            ObserveActEvent::ScreenshotCaptured { path: "/tmp/s.png".into() },
            ObserveActEvent::LlmReasoning { text: "I see a button".into() },
            ObserveActEvent::ActionExecuted {
                action: ObserveActAction::Click { x: 10, y: 20 },
                success: true,
            },
            ObserveActEvent::VerificationDone {
                result: VerificationResult::new("x".into(), "y".into(), true, 0.9),
            },
            ObserveActEvent::TaskCompleted { summary: "done".into() },
            ObserveActEvent::Error { message: "oops".into() },
            ObserveActEvent::SafetyHalt { reason: "forbidden region".into() },
        ];
        for event in &events {
            let json = serde_json::to_string(event).expect("serialize");
            let deserialized: ObserveActEvent = serde_json::from_str(&json).expect("deserialize");
            // Just verify round-trip doesn't panic
            let _ = serde_json::to_string(&deserialized).expect("re-serialize");
        }
    }

    // ── Safety Rails Defaults ──────────────────────────────────────────

    #[test]
    fn test_safety_rails_defaults() {
        let rails = SafetyRails::default();
        assert!(rails.forbidden_regions.is_empty());
        assert_eq!(rails.max_actions_per_step, 5);
        assert!(rails.require_confirmation_for.is_empty());
        assert_eq!(rails.forbidden_key_combos.len(), 2);
        assert_eq!(rails.rate_limit_ms, 200);
    }

    // ── Helper ─────────────────────────────────────────────────────────

    /// Create a test step with one click action and a verification result.
    fn make_step(num: usize, success: bool) -> ObservationStep {
        ObservationStep {
            step_num: num,
            timestamp_ms: now_ms(),
            screenshot_path: Some(format!("/tmp/step_{}.png", num)),
            llm_reasoning: format!("Step {} reasoning", num),
            actions_taken: vec![ObserveActAction::Click { x: 10, y: 20 }],
            verification_result: Some(VerificationResult::new(
                "expected".into(),
                "actual".into(),
                success,
                if success { 0.95 } else { 0.3 },
            )),
            duration_ms: 500,
        }
    }
}
