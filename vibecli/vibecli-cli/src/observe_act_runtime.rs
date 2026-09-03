//! Observe-Act runtime — the loop that actually drives the screen.
//!
//! [`crate::observe_act`] holds the vocabulary: the action enum, the session
//! state machine, the safety rails and the prompt shapes. This module is the
//! half that touches the world — it captures the screen, asks a vision model
//! what to do, executes what comes back, verifies the result, and does all of
//! that under the safety mode the operator chose.
//!
//! ## Why the two edges are traits
//!
//! [`ScreenDriver`] and [`VisionModel`] exist so the loop's safety semantics
//! can be tested without a display or an API key. Those semantics are the part
//! that must not regress: *a restricted session executes nothing*, *a cautious
//! session executes nothing destructive without a human*, *a forbidden region
//! is never clicked*. A loop that can only be exercised against a real desktop
//! is a loop whose safety rails are verified by hope.
//!
//! ## Coordinate spaces
//!
//! See [`ScreenGeometry`] — the model answers in the pixel space of the image
//! it was sent, which is neither the capture's pixels nor the display's
//! points. Every coordinate is mapped once, at the boundary, before validation
//! so the forbidden-region check runs in the same space the click lands in.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, oneshot, Notify};
use tracing::{debug, info, warn};
use vibe_sync_ext::{LockRecover, RwLockRecover};

use crate::desktop_agent::{self, DesktopAction, DesktopAutomation, MouseButton};
use crate::observe_act::{
    is_destructive, now_ms, validate_action_batch, LlmPromptBuilder, ObservationStep,
    ObserveActAction, ObserveActConfig, ObserveActEvent, ObserveActSession, SafetyMode,
    SafetyRails, ScreenGeometry, ScrollDirection, SessionStatus, SessionSummary,
    VerificationResult,
};

// ── Hard limits ────────────────────────────────────────────────────────────

/// Longest a single `wait` action may pause the loop.
///
/// A model that answers `{"type": "wait", "ms": 3600000}` would otherwise park
/// a session — holding the desktop hostage with no step ever recorded and
/// nothing on screen to say why. Clamped, not rejected: a long wait is a
/// reasonable thing to want, an hour of it is not.
const MAX_WAIT_MS: u64 = 30_000;

/// Longest text a single `type` action may enter.
///
/// Synthetic keystrokes are slow (`cliclick` types at human-ish speed), so a
/// model pasting a whole file would block the loop for minutes with no way to
/// interrupt between keystrokes.
const MAX_TYPE_CHARS: usize = 4_096;

/// Ceiling on `max_actions_per_step`, whatever the operator configures.
const MAX_ACTIONS_PER_STEP_CEILING: usize = 20;

/// Capacity of a session's event broadcast. A UI that falls this far behind
/// re-reads the session instead; the events are a live view, not the record.
const EVENT_BUFFER: usize = 256;

/// How long a cautious-mode confirmation waits for a human before it is
/// treated as a refusal.
///
/// Denial is the safe default for a timeout: an operator who walked away has
/// not approved anything, and the alternative (proceeding) would make the
/// confirmation gate meaningless exactly when nobody is watching.
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(300);

// ── Screen driver ──────────────────────────────────────────────────────────

/// Everything the loop needs from the machine it is driving.
#[async_trait::async_trait]
pub trait ScreenDriver: Send + Sync {
    /// The screen's size in the units the input APIs take — points on macOS,
    /// pixels elsewhere. This is the space clicks are issued in.
    async fn logical_screen(&self) -> Result<(u32, u32)>;

    /// Capture the whole screen to `path` as a PNG.
    async fn capture(&self, path: &Path) -> Result<()>;

    /// Execute one action. Coordinates are already in logical space.
    async fn perform(&self, action: &ObserveActAction) -> Result<()>;
}

/// The real driver: [`crate::desktop_agent`] shelling out to the platform's
/// automation tools.
pub struct DesktopScreenDriver {
    automation: DesktopAutomation,
}

impl DesktopScreenDriver {
    pub fn new() -> Self {
        Self {
            automation: DesktopAutomation::new(),
        }
    }
}

impl Default for DesktopScreenDriver {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the platform command that reports the screen size **in logical
/// units** — the ones `cliclick` and `xdotool` take.
///
/// Deliberately not `DesktopAutomation::build_get_screen_size_cmd`, which
/// answers a different question: on macOS that one runs `system_profiler` and
/// reports the display's *native* pixels (2560×1600 on a 1280×800 Retina
/// panel), with the backing scale inferred from whether the word "Retina"
/// appears in the output. That inference is absent on a scaled resolution and
/// wrong on a second display, and a factor-of-two error here puts every click
/// in the wrong quadrant. Finder's desktop bounds are the logical rectangle
/// directly, with nothing to infer.
fn logical_screen_cmd(platform: desktop_agent::DesktopPlatform) -> String {
    match platform {
        desktop_agent::DesktopPlatform::MacOS => {
            "osascript -e 'tell application \"Finder\" to get bounds of window of desktop'"
                .to_string()
        }
        // X11 reports one space; there is no separate backing store to correct
        // for, so the existing command is already the logical one.
        other => DesktopAutomation::for_platform(other).build_get_screen_size_cmd(),
    }
}

#[async_trait::async_trait]
impl ScreenDriver for DesktopScreenDriver {
    async fn logical_screen(&self) -> Result<(u32, u32)> {
        let cmd = logical_screen_cmd(self.automation.platform);
        let out = tokio::process::Command::new("sh")
            .args(["-c", &cmd])
            .output()
            .await
            .with_context(|| format!("running screen-size probe: {cmd}"))?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        let info = desktop_agent::parse_screen_size(&stdout, self.automation.platform).ok_or_else(
            || {
                anyhow!(
                    "could not read the screen size from `{cmd}` (got {:?})",
                    stdout.trim()
                )
            },
        )?;
        if info.width == 0 || info.height == 0 {
            return Err(anyhow!("screen-size probe reported a zero-sized screen"));
        }
        Ok((info.width, info.height))
    }

    async fn capture(&self, path: &Path) -> Result<()> {
        let action = DesktopAction::Screenshot {
            path: path.to_string_lossy().to_string(),
        };
        let result = self.automation.execute(&action).await?;
        if !result.success {
            return Err(anyhow!("screenshot failed: {}", result.output));
        }
        if !path.exists() {
            // `screencapture` exits 0 when Screen Recording permission is
            // denied and writes nothing — the one failure that looks like a
            // success, and the one a user hits first on a fresh macOS install.
            return Err(anyhow!(
                "the screenshot tool reported success but wrote no file — on macOS this is \
                 Screen Recording permission being denied to the daemon (System Settings → \
                 Privacy & Security → Screen Recording)"
            ));
        }
        Ok(())
    }

    async fn perform(&self, action: &ObserveActAction) -> Result<()> {
        for desktop_action in to_desktop_actions(action) {
            let result = self.automation.execute(&desktop_action).await?;
            if !result.success {
                return Err(anyhow!("{desktop_action} failed: {}", result.output));
            }
        }
        Ok(())
    }
}

/// Translate one observe-act action into the desktop primitives that carry it
/// out. A scroll becomes N key presses; everything else is one-to-one.
///
/// `Screenshot` and `Done` produce nothing: the loop captures on its own
/// cadence and `Done` is a signal, not a keystroke.
pub fn to_desktop_actions(action: &ObserveActAction) -> Vec<DesktopAction> {
    match action {
        ObserveActAction::Click { x, y } => vec![DesktopAction::Click {
            button: MouseButton::Left,
            x: *x,
            y: *y,
        }],
        ObserveActAction::DoubleClick { x, y } => vec![DesktopAction::DoubleClick { x: *x, y: *y }],
        ObserveActAction::RightClick { x, y } => vec![DesktopAction::Click {
            button: MouseButton::Right,
            x: *x,
            y: *y,
        }],
        ObserveActAction::MoveMouse { x, y } => vec![DesktopAction::MoveMouse { x: *x, y: *y }],
        ObserveActAction::Drag {
            from_x,
            from_y,
            to_x,
            to_y,
        } => vec![DesktopAction::Drag {
            from_x: *from_x,
            from_y: *from_y,
            to_x: *to_x,
            to_y: *to_y,
        }],
        ObserveActAction::Type { text } => {
            let text: String = text.chars().take(MAX_TYPE_CHARS).collect();
            vec![DesktopAction::TypeText { text }]
        }
        ObserveActAction::KeyCombo { keys } => match keys.split_last() {
            // The last key is the one pressed; everything before it is a
            // modifier held down. `["ctrl", "shift", "s"]` → ctrl+shift held,
            // s tapped.
            Some((key, modifiers)) if !modifiers.is_empty() => vec![DesktopAction::KeyCombo {
                modifiers: modifiers.to_vec(),
                key: key.clone(),
            }],
            Some((key, _)) => vec![DesktopAction::PressKey { key: key.clone() }],
            None => Vec::new(),
        },
        ObserveActAction::Scroll { direction, amount } => {
            let key = match direction {
                ScrollDirection::Up => "Up",
                ScrollDirection::Down => "Down",
                ScrollDirection::Left => "Left",
                ScrollDirection::Right => "Right",
            };
            // Bounded so a model asking to scroll 10,000 lines does not hold
            // the loop for the rest of the session.
            (0..(*amount).min(50))
                .map(|_| DesktopAction::PressKey {
                    key: key.to_string(),
                })
                .collect()
        }
        ObserveActAction::Wait { ms } => vec![DesktopAction::Delay {
            ms: (*ms).min(MAX_WAIT_MS),
        }],
        ObserveActAction::Screenshot | ObserveActAction::Done { .. } => Vec::new(),
    }
}

// ── Vision model ───────────────────────────────────────────────────────────

/// A vision-capable model, narrowed to the one call the loop makes.
#[async_trait::async_trait]
pub trait VisionModel: Send + Sync {
    /// Ask about a single image. Returns the model's raw text.
    async fn ask(&self, prompt: &str, image: EncodedImage) -> Result<String>;

    /// Provider/model label, for the session record.
    fn label(&self) -> String;
}

/// The real model: any `AIProvider` that accepts image attachments.
pub struct ProviderVisionModel {
    provider: Arc<dyn vibe_ai::provider::AIProvider>,
    label: String,
}

impl ProviderVisionModel {
    pub fn new(provider: Arc<dyn vibe_ai::provider::AIProvider>, label: String) -> Self {
        Self { provider, label }
    }

    /// Whether the provider claims vision support.
    ///
    /// Reported, never enforced: `supports_vision` defaults to `false` on the
    /// trait and several providers that do implement `chat_with_images` never
    /// override it. Blocking on it would reject working configurations, so the
    /// preflight surfaces it as a caution and the run proceeds.
    pub fn claims_vision(&self) -> bool {
        self.provider.supports_vision()
    }
}

#[async_trait::async_trait]
impl VisionModel for ProviderVisionModel {
    async fn ask(&self, prompt: &str, image: EncodedImage) -> Result<String> {
        let messages = vec![vibe_ai::provider::Message {
            role: vibe_ai::provider::MessageRole::User,
            content: prompt.to_string(),
        }];
        let attachment = vibe_ai::provider::ImageAttachment {
            base64: image.base64,
            media_type: image.media_type,
        };
        self.provider
            .chat_with_images(&messages, std::slice::from_ref(&attachment), None)
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    fn label(&self) -> String {
        self.label.clone()
    }
}

// ── Screenshot encoding ────────────────────────────────────────────────────

/// A screenshot resized and re-encoded for a vision API.
#[derive(Debug, Clone)]
pub struct EncodedImage {
    pub base64: String,
    pub media_type: String,
    /// Width of the encoded image — the space the model answers in.
    pub width: u32,
    /// Height of the encoded image.
    pub height: u32,
}

/// JPEG quality for the images sent to the model.
///
/// 80 keeps UI text legible while cutting a full-screen capture to a few
/// hundred kilobytes. Higher buys nothing a model can read; lower starts
/// smearing small labels, which is exactly what it is being asked to find.
const JPEG_QUALITY: u8 = 80;

/// Load a captured screenshot, fit it inside `max_w × max_h`, and encode it as
/// a base64 JPEG.
///
/// Blocking work (decode + resize + encode of a multi-megapixel image), so
/// call it from `spawn_blocking` — [`capture_for_model`] does.
pub fn encode_for_model(path: &Path, max_w: u32, max_h: u32) -> Result<EncodedImage> {
    use base64::Engine as _;

    let img = image::open(path).with_context(|| format!("reading screenshot {path:?}"))?;
    // `resize` preserves aspect ratio and only ever fits *inside* the box, so a
    // screenshot already smaller than the cap is left alone rather than being
    // upscaled into blur.
    let (w, h) = (img.width(), img.height());
    let img = if w > max_w || h > max_h {
        img.resize(
            max_w.max(1),
            max_h.max(1),
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };
    let (width, height) = (img.width(), img.height());

    let mut buf = Vec::with_capacity(256 * 1024);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
        std::io::Cursor::new(&mut buf),
        JPEG_QUALITY,
    );
    encoder
        .encode_image(&img.to_rgb8())
        .context("encoding screenshot as JPEG")?;

    Ok(EncodedImage {
        base64: base64::engine::general_purpose::STANDARD.encode(&buf),
        media_type: "image/jpeg".to_string(),
        width,
        height,
    })
}

/// Capture the screen to `path` and encode it for the model.
async fn capture_for_model(
    screen: &dyn ScreenDriver,
    path: &Path,
    max_w: u32,
    max_h: u32,
) -> Result<EncodedImage> {
    screen.capture(path).await?;
    let owned = path.to_path_buf();
    tokio::task::spawn_blocking(move || encode_for_model(&owned, max_w, max_h))
        .await
        .map_err(|e| anyhow!("screenshot encoder panicked: {e}"))?
}

// ── Session control ────────────────────────────────────────────────────────

/// What the operator has asked the loop to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Run,
    Paused,
    Aborted,
}

impl Control {
    fn code(self) -> u8 {
        match self {
            Self::Run => 0,
            Self::Paused => 1,
            Self::Aborted => 2,
        }
    }
    fn from_code(v: u8) -> Self {
        match v {
            1 => Self::Paused,
            2 => Self::Aborted,
            _ => Self::Run,
        }
    }
}

/// A destructive action waiting on a human in cautious mode.
#[derive(Debug, Clone, Serialize)]
pub struct PendingApproval {
    pub id: String,
    pub step_num: usize,
    pub action: ObserveActAction,
    /// Rendered form of the action, so a client need not know the enum.
    pub description: String,
    pub requested_at_ms: u64,
}

struct PendingSlot {
    info: PendingApproval,
    responder: oneshot::Sender<bool>,
}

// ── Session handle ─────────────────────────────────────────────────────────

/// A live (or finished) observe-act session.
pub struct SessionHandle {
    pub id: String,
    /// Provider/model the vision calls go to, as chosen by the caller.
    pub model_label: String,
    /// Where this session's screenshots and record live.
    pub dir: PathBuf,
    state: Mutex<ObserveActSession>,
    safety: SafetyRails,
    control: AtomicU8,
    /// Woken when `control` changes or an approval is answered.
    wake: Notify,
    pending: Mutex<Option<PendingSlot>>,
    events: broadcast::Sender<ObserveActEvent>,
    latest_screenshot: Mutex<Option<PathBuf>>,
}

impl SessionHandle {
    fn new(
        id: String,
        dir: PathBuf,
        session: ObserveActSession,
        safety: SafetyRails,
        model_label: String,
    ) -> Self {
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        Self {
            id,
            model_label,
            dir,
            state: Mutex::new(session),
            safety,
            control: AtomicU8::new(Control::Run.code()),
            wake: Notify::new(),
            pending: Mutex::new(None),
            events,
            latest_screenshot: Mutex::new(None),
        }
    }

    /// Subscribe to this session's events.
    pub fn subscribe(&self) -> broadcast::Receiver<ObserveActEvent> {
        self.events.subscribe()
    }

    /// A snapshot of the session state.
    pub fn snapshot(&self) -> ObserveActSession {
        self.state.lock_recover().clone()
    }

    pub fn summary(&self) -> SessionSummary {
        self.state.lock_recover().get_summary()
    }

    pub fn status(&self) -> SessionStatus {
        self.state.lock_recover().status.clone()
    }

    /// The most recent screenshot on disk, if any step has captured one.
    pub fn latest_screenshot(&self) -> Option<PathBuf> {
        self.latest_screenshot.lock_recover().clone()
    }

    /// The approval this session is currently blocked on, if any.
    pub fn pending_approval(&self) -> Option<PendingApproval> {
        self.pending.lock_recover().as_ref().map(|p| p.info.clone())
    }

    fn control(&self) -> Control {
        Control::from_code(self.control.load(Ordering::SeqCst))
    }

    fn set_control(&self, c: Control) {
        self.control.store(c.code(), Ordering::SeqCst);
        self.wake.notify_waiters();
    }

    /// Pause after the current step finishes.
    pub fn pause(&self) {
        if self.control() == Control::Run {
            self.set_control(Control::Paused);
            self.state.lock_recover().pause();
        }
    }

    /// Resume a paused session.
    pub fn resume(&self) {
        if self.control() == Control::Paused {
            self.set_control(Control::Run);
            self.state.lock_recover().resume();
        }
    }

    /// Abort the session. Takes effect at the next action boundary — an
    /// in-flight keystroke is not something we can recall.
    pub fn abort(&self, reason: &str) {
        self.set_control(Control::Aborted);
        self.state.lock_recover().abort(reason);
        // A session waiting on a human must not keep waiting after the human
        // aborted it; resolving the approval as a refusal unblocks the loop
        // into its abort check.
        self.resolve_approval(None, false);
        self.emit(ObserveActEvent::SafetyHalt {
            reason: reason.to_string(),
        });
    }

    /// Answer a pending approval. `id` of `None` answers whatever is pending —
    /// used by abort, which does not care which action was queued.
    ///
    /// Returns true when an approval was actually resolved.
    pub fn resolve_approval(&self, id: Option<&str>, approve: bool) -> bool {
        let slot = {
            let mut guard = self.pending.lock_recover();
            match guard.as_ref() {
                Some(p) if id.is_none_or(|want| want == p.info.id) => guard.take(),
                _ => None,
            }
        };
        match slot {
            Some(slot) => {
                // Receiver dropped means the loop already gave up waiting
                // (timeout, abort) — the decision arrived too late to matter,
                // and saying it resolved would be a lie to the caller.
                let delivered = slot.responder.send(approve).is_ok();
                self.wake.notify_waiters();
                delivered
            }
            None => false,
        }
    }

    fn emit(&self, event: ObserveActEvent) {
        // No receivers is the normal case for a session nobody is watching.
        let _ = self.events.send(event);
    }

    /// Persist the session record so history survives a daemon restart.
    fn persist(&self) {
        let snapshot = self.snapshot();
        let path = self.dir.join("session.json");
        let record = PersistedSession {
            id: self.id.clone(),
            model_label: self.model_label.clone(),
            session: snapshot,
        };
        match serde_json::to_vec_pretty(&record) {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(&path, bytes) {
                    warn!(session = %self.id, error = %e, "could not persist observe-act session");
                }
            }
            Err(e) => {
                warn!(session = %self.id, error = %e, "could not serialise observe-act session")
            }
        }
    }
}

/// The on-disk form of a session.
#[derive(Debug, Serialize, Deserialize)]
struct PersistedSession {
    id: String,
    #[serde(default)]
    model_label: String,
    session: ObserveActSession,
}

// ── Registry ───────────────────────────────────────────────────────────────

/// What a caller supplies to start a session.
pub struct StartSpec {
    pub task: String,
    pub config: ObserveActConfig,
    pub safety: SafetyRails,
    pub screen: Arc<dyn ScreenDriver>,
    pub vision: Arc<dyn VisionModel>,
}

/// All observe-act sessions this daemon knows about.
///
/// Constructed with an explicit root so tests never touch the developer's real
/// `~/.vibecli` (AGENTS.md → Test Isolation).
pub struct ObserveActRegistry {
    sessions: RwLock<HashMap<String, Arc<SessionHandle>>>,
    /// Newest-first order of session ids, so listing is stable.
    order: Mutex<Vec<String>>,
    root: PathBuf,
}

/// How many finished sessions are kept resident. Their screenshots stay on
/// disk either way; this only bounds memory.
const MAX_RESIDENT_SESSIONS: usize = 50;

impl ObserveActRegistry {
    /// Create a registry rooted at `root`, loading any sessions a previous
    /// daemon left behind.
    pub fn new(root: PathBuf) -> Self {
        let registry = Self {
            sessions: RwLock::new(HashMap::new()),
            order: Mutex::new(Vec::new()),
            root,
        };
        registry.load_existing();
        registry
    }

    /// The standard location: `~/.vibecli/observe_act`.
    pub fn default_root() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vibecli")
            .join("observe_act")
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Read back sessions written by an earlier daemon process.
    ///
    /// Any session recorded as `Running` or `Paused` is re-read as `Aborted`:
    /// the process that was driving it is gone, so it is not running, and
    /// showing it as running would leave a permanent phantom in the history
    /// that no stop button can clear.
    fn load_existing(&self) {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return;
        };
        let mut loaded: Vec<(u64, Arc<SessionHandle>)> = Vec::new();
        for entry in entries.flatten() {
            let dir = entry.path();
            let record = match std::fs::read(dir.join("session.json")) {
                Ok(bytes) => match serde_json::from_slice::<PersistedSession>(&bytes) {
                    Ok(r) => r,
                    Err(e) => {
                        warn!(path = ?dir, error = %e, "skipping unreadable observe-act session");
                        continue;
                    }
                },
                Err(_) => continue,
            };
            let mut session = record.session;
            if matches!(
                session.status,
                SessionStatus::Running | SessionStatus::Paused
            ) {
                session.status = SessionStatus::Aborted;
            }
            let started = session.started_at_ms;
            let last_shot = session
                .steps
                .iter()
                .rev()
                .find_map(|s| s.screenshot_path.clone())
                .map(PathBuf::from);
            let handle = Arc::new(SessionHandle::new(
                record.id.clone(),
                dir,
                session,
                SafetyRails::default(),
                record.model_label,
            ));
            handle.set_control(Control::Aborted);
            *handle.latest_screenshot.lock_recover() = last_shot;
            loaded.push((started, handle));
        }
        loaded.sort_by_key(|(started, _)| std::cmp::Reverse(*started));
        let mut sessions = self.sessions.write_recover();
        let mut order = self.order.lock_recover();
        for (_, handle) in loaded.into_iter().take(MAX_RESIDENT_SESSIONS) {
            order.push(handle.id.clone());
            sessions.insert(handle.id.clone(), handle);
        }
    }

    /// Sessions, newest first.
    pub fn list(&self) -> Vec<Arc<SessionHandle>> {
        let sessions = self.sessions.read_recover();
        self.order
            .lock_recover()
            .iter()
            .filter_map(|id| sessions.get(id).cloned())
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<Arc<SessionHandle>> {
        self.sessions.read_recover().get(id).cloned()
    }

    /// Is a session currently driving the screen?
    ///
    /// There is one desktop, so there is one session at a time — two loops
    /// moving the same mouse would each see the other's half-finished work and
    /// verify against it.
    pub fn active(&self) -> Option<Arc<SessionHandle>> {
        self.list().into_iter().find(|h| {
            matches!(
                h.status(),
                SessionStatus::Running | SessionStatus::Paused | SessionStatus::Idle
            )
        })
    }

    /// Start a session and spawn its loop.
    ///
    /// Fails if another session is still active, or if the screen geometry
    /// cannot be established — a loop that does not know the coordinate space
    /// would click at scaled-wrong positions, which on a desktop means
    /// clicking whatever happens to be there.
    pub async fn start(&self, spec: StartSpec) -> Result<Arc<SessionHandle>> {
        if let Some(existing) = self.active() {
            return Err(anyhow!(
                "session {} is still {} — stop it before starting another",
                existing.id,
                existing.status()
            ));
        }
        let task = spec.task.trim();
        if task.is_empty() {
            return Err(anyhow!("task description is empty"));
        }

        let (logical_w, logical_h) = spec.screen.logical_screen().await.context(
            "could not determine the screen size, so screen coordinates cannot be mapped",
        )?;

        let id = uuid::Uuid::new_v4().to_string();
        let dir = self.root.join(&id);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("creating session directory {dir:?}"))?;

        let safety = clamp_safety(spec.safety);
        let session = ObserveActSession::new(spec.config, task.to_string());
        let handle = Arc::new(SessionHandle::new(
            id.clone(),
            dir,
            session,
            safety,
            spec.vision.label(),
        ));

        {
            let mut sessions = self.sessions.write_recover();
            let mut order = self.order.lock_recover();
            order.insert(0, id.clone());
            sessions.insert(id.clone(), Arc::clone(&handle));
            // Evict the oldest resident sessions; their records stay on disk.
            while order.len() > MAX_RESIDENT_SESSIONS {
                if let Some(old) = order.pop() {
                    sessions.remove(&old);
                }
            }
        }

        let loop_handle = Arc::clone(&handle);
        let screen = Arc::clone(&spec.screen);
        let vision = Arc::clone(&spec.vision);
        tokio::spawn(async move {
            run_session(loop_handle, screen, vision, logical_w, logical_h).await;
        });

        Ok(handle)
    }

    /// Run a session to completion on the current task rather than spawning
    /// it, so a test can assert on a finished loop instead of polling one.
    ///
    /// `cfg(test)` because it deliberately skips the single-active-session
    /// check that [`Self::start`] enforces — shipping it would be shipping a
    /// way around the one rule that keeps two loops off the same mouse.
    #[cfg(test)]
    pub async fn start_blocking(&self, spec: StartSpec) -> Result<Arc<SessionHandle>> {
        let (logical_w, logical_h) = spec.screen.logical_screen().await?;
        let id = uuid::Uuid::new_v4().to_string();
        let dir = self.root.join(&id);
        std::fs::create_dir_all(&dir)?;
        let session = ObserveActSession::new(spec.config, spec.task.clone());
        let handle = Arc::new(SessionHandle::new(
            id.clone(),
            dir,
            session,
            clamp_safety(spec.safety),
            spec.vision.label(),
        ));
        {
            let mut sessions = self.sessions.write_recover();
            self.order.lock_recover().insert(0, id.clone());
            sessions.insert(id.clone(), Arc::clone(&handle));
        }
        run_session(
            Arc::clone(&handle),
            spec.screen,
            spec.vision,
            logical_w,
            logical_h,
        )
        .await;
        Ok(handle)
    }
}

/// Bound operator-supplied safety values.
///
/// The panel is not the only caller — the route takes JSON — and
/// `max_actions_per_step: 100000` would turn one model turn into an
/// unstoppable burst. The ceiling is a floor on safety, not a preference.
fn clamp_safety(mut safety: SafetyRails) -> SafetyRails {
    safety.max_actions_per_step = safety
        .max_actions_per_step
        .clamp(1, MAX_ACTIONS_PER_STEP_CEILING);
    safety.rate_limit_ms = safety.rate_limit_ms.min(10_000);
    safety
}

// ── The loop ───────────────────────────────────────────────────────────────

/// Outcome of executing one step's actions.
struct StepExecution {
    executed: Vec<ObserveActAction>,
    proposed: Vec<ObserveActAction>,
    done_summary: Option<String>,
    halted: Option<String>,
}

async fn run_session(
    handle: Arc<SessionHandle>,
    screen: Arc<dyn ScreenDriver>,
    vision: Arc<dyn VisionModel>,
    logical_w: u32,
    logical_h: u32,
) {
    handle.state.lock_recover().start();
    handle.persist();

    let (task, config) = {
        let s = handle.state.lock_recover();
        (s.task.clone(), s.config.clone())
    };
    info!(session = %handle.id, task = %task, mode = %config.safety_mode, "observe-act session started");

    loop {
        // Pause is checked before the step, not inside it: stopping between
        // steps leaves the screen in a state the next step can observe, while
        // stopping mid-action would leave a half-typed string nobody recorded.
        if !wait_until_runnable(&handle).await {
            break;
        }
        if !handle.state.lock_recover().can_continue() {
            break;
        }

        let step_num = handle.state.lock_recover().steps.len() + 1;
        let step_start = now_ms();
        handle.emit(ObserveActEvent::StepStarted { step_num });

        match run_step(
            &handle,
            screen.as_ref(),
            vision.as_ref(),
            &task,
            &config,
            step_num,
            logical_w,
            logical_h,
        )
        .await
        {
            Ok(step) => {
                let done = step.actions_taken.iter().find_map(|a| match a {
                    ObserveActAction::Done { summary } => Some(summary.clone()),
                    _ => None,
                });
                {
                    let mut s = handle.state.lock_recover();
                    s.record_step(step);
                    if done.is_some() {
                        s.complete();
                    }
                }
                handle.persist();
                if let Some(summary) = done {
                    handle.emit(ObserveActEvent::TaskCompleted { summary });
                    break;
                }
            }
            Err(e) => {
                // A step that could not be taken is a failure, and it must
                // count against the streak — otherwise a broken screenshot
                // tool spins the loop for the whole `max_steps` budget.
                warn!(session = %handle.id, step = step_num, error = %e, "observe-act step failed");
                handle.emit(ObserveActEvent::Error {
                    message: e.to_string(),
                });
                let step = ObservationStep {
                    step_num,
                    timestamp_ms: step_start,
                    screenshot_path: None,
                    llm_reasoning: format!("Step failed: {e}"),
                    actions_taken: Vec::new(),
                    proposed_actions: Vec::new(),
                    verification_result: Some(VerificationResult::new(
                        String::new(),
                        e.to_string(),
                        false,
                        1.0,
                    )),
                    duration_ms: now_ms().saturating_sub(step_start),
                };
                handle.state.lock_recover().record_step(step);
                handle.persist();
            }
        }

        if handle.control() == Control::Aborted {
            break;
        }
        tokio::time::sleep(Duration::from_millis(config.observation_interval_ms)).await;
    }

    // A loop that fell out of `can_continue` without a terminal status ran out
    // of budget rather than finishing — say so instead of leaving it Running.
    {
        let mut s = handle.state.lock_recover();
        if s.status == SessionStatus::Running {
            s.status = SessionStatus::Completed;
        }
    }
    handle.persist();
    info!(session = %handle.id, status = %handle.status(), "observe-act session ended");
}

/// Block while the session is paused. Returns false if it was aborted.
async fn wait_until_runnable(handle: &SessionHandle) -> bool {
    loop {
        match handle.control() {
            Control::Run => return true,
            Control::Aborted => return false,
            Control::Paused => {
                // `notified()` is created before re-checking the flag, so a
                // resume racing this loop cannot be missed.
                let notified = handle.wake.notified();
                if handle.control() != Control::Paused {
                    continue;
                }
                notified.await;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_step(
    handle: &SessionHandle,
    screen: &dyn ScreenDriver,
    vision: &dyn VisionModel,
    task: &str,
    config: &ObserveActConfig,
    step_num: usize,
    logical_w: u32,
    logical_h: u32,
) -> Result<ObservationStep> {
    let step_start = now_ms();
    let shot_path = handle.dir.join(format!("step-{step_num:04}.png"));

    let image = capture_for_model(
        screen,
        &shot_path,
        config.screenshot_width,
        config.screenshot_height,
    )
    .await?;
    *handle.latest_screenshot.lock_recover() = Some(shot_path.clone());
    handle.emit(ObserveActEvent::ScreenshotCaptured {
        path: shot_path.to_string_lossy().to_string(),
    });

    let geometry = ScreenGeometry {
        image_width: image.width,
        image_height: image.height,
        logical_width: logical_w,
        logical_height: logical_h,
    };

    let history = handle.state.lock_recover().steps.clone();
    let prompt = LlmPromptBuilder::build_observation_prompt(task, &history, &geometry);
    let response = vision.ask(&prompt, image).await?;
    let decision = LlmPromptBuilder::parse_decision(&response);

    let reasoning = if decision.reasoning.is_empty() {
        // Keep the raw answer rather than an empty string: when a model
        // ignores the schema, the answer itself is the only diagnostic the
        // operator has for why nothing happened.
        response.chars().take(500).collect::<String>()
    } else {
        decision.reasoning.clone()
    };
    handle.emit(ObserveActEvent::LlmReasoning {
        text: reasoning.clone(),
    });

    // Map into logical space *before* validating, so the forbidden-region
    // check runs against the coordinates the click will actually use.
    let proposed: Vec<ObserveActAction> = decision
        .actions
        .iter()
        .map(|a| geometry.map_action(a))
        .collect();

    if let Err(e) = validate_action_batch(&proposed, &handle.safety) {
        handle.emit(ObserveActEvent::SafetyHalt {
            reason: e.to_string(),
        });
        return Ok(ObservationStep {
            step_num,
            timestamp_ms: step_start,
            screenshot_path: Some(shot_path.to_string_lossy().to_string()),
            llm_reasoning: reasoning,
            actions_taken: Vec::new(),
            proposed_actions: proposed,
            verification_result: Some(VerificationResult::new(
                decision.expected_change.clone(),
                format!("Blocked by safety rails: {e}"),
                false,
                1.0,
            )),
            duration_ms: now_ms().saturating_sub(step_start),
        });
    }

    let execution = execute_actions(handle, screen, config, step_num, &proposed).await;

    let verification = verify_step(
        handle,
        screen,
        vision,
        config,
        step_num,
        &decision.expected_change,
        &execution,
    )
    .await;

    Ok(ObservationStep {
        step_num,
        timestamp_ms: step_start,
        screenshot_path: Some(shot_path.to_string_lossy().to_string()),
        llm_reasoning: reasoning,
        actions_taken: execution.executed,
        proposed_actions: execution.proposed,
        verification_result: verification,
        duration_ms: now_ms().saturating_sub(step_start),
    })
}

async fn execute_actions(
    handle: &SessionHandle,
    screen: &dyn ScreenDriver,
    config: &ObserveActConfig,
    step_num: usize,
    proposed: &[ObserveActAction],
) -> StepExecution {
    let mut execution = StepExecution {
        executed: Vec::new(),
        proposed: proposed.to_vec(),
        done_summary: None,
        halted: None,
    };

    for action in proposed {
        if handle.control() == Control::Aborted {
            execution.halted = Some("aborted".to_string());
            break;
        }

        // `Done` is a signal, not something to perform. Record it so the
        // summary can be read back out of the history.
        if let ObserveActAction::Done { summary } = action {
            execution.done_summary = Some(summary.clone());
            execution.executed.push(action.clone());
            break;
        }

        // Read-only: observe and propose, never touch the screen. The proposal
        // is still recorded, which is what makes this mode worth running
        // before an autonomous one.
        if config.safety_mode == SafetyMode::Restricted {
            continue;
        }

        // Short-circuits left to right, so approval is asked for only when the
        // mode gates and the action is destructive.
        if config.safety_mode == SafetyMode::Cautious
            && is_destructive(action)
            && !request_approval(handle, step_num, action).await
        {
            handle.emit(ObserveActEvent::SafetyHalt {
                reason: format!("operator declined {action}"),
            });
            execution.halted = Some(format!("declined {action}"));
            break;
        }

        match screen.perform(action).await {
            Ok(()) => {
                handle.emit(ObserveActEvent::ActionExecuted {
                    action: action.clone(),
                    success: true,
                });
                execution.executed.push(action.clone());
            }
            Err(e) => {
                warn!(session = %handle.id, action = %action, error = %e, "action failed");
                handle.emit(ObserveActEvent::ActionExecuted {
                    action: action.clone(),
                    success: false,
                });
                handle.emit(ObserveActEvent::Error {
                    message: format!("{action}: {e}"),
                });
                execution.halted = Some(format!("{action} failed: {e}"));
                break;
            }
        }

        if handle.safety.rate_limit_ms > 0 {
            tokio::time::sleep(Duration::from_millis(handle.safety.rate_limit_ms)).await;
        }
    }

    execution
}

/// Ask the operator about a destructive action. Returns whether to proceed.
async fn request_approval(
    handle: &SessionHandle,
    step_num: usize,
    action: &ObserveActAction,
) -> bool {
    let (tx, rx) = oneshot::channel();
    let info = PendingApproval {
        id: uuid::Uuid::new_v4().to_string(),
        step_num,
        action: action.clone(),
        description: action.to_string(),
        requested_at_ms: now_ms(),
    };
    handle.emit(ObserveActEvent::ApprovalRequired {
        approval_id: info.id.clone(),
        step_num,
        action: action.clone(),
        description: info.description.clone(),
    });
    let approval_id = info.id.clone();
    *handle.pending.lock_recover() = Some(PendingSlot {
        info,
        responder: tx,
    });

    let approved = match tokio::time::timeout(APPROVAL_TIMEOUT, rx).await {
        Ok(Ok(decision)) => decision,
        // Sender dropped, or nobody answered in time. Both mean no human said
        // yes, and no is what "confirm destructive actions" has to mean when
        // the confirmation never comes.
        Ok(Err(_)) | Err(_) => false,
    };
    *handle.pending.lock_recover() = None;
    handle.emit(ObserveActEvent::ApprovalResolved {
        approval_id,
        approved,
    });
    debug!(session = %handle.id, action = %action, approved, "approval resolved");
    approved
}

async fn verify_step(
    handle: &SessionHandle,
    screen: &dyn ScreenDriver,
    vision: &dyn VisionModel,
    config: &ObserveActConfig,
    step_num: usize,
    expected: &str,
    execution: &StepExecution,
) -> Option<VerificationResult> {
    // Nothing ran, or the model never said what to expect: there is no claim
    // to check. Absent, not passed — `record_step` leaves the failure streak
    // alone for an unverified step precisely so this cannot be read as success.
    if !config.verify_after_action
        || expected.trim().is_empty()
        || execution.executed.is_empty()
        || execution.done_summary.is_some()
    {
        return None;
    }

    let path = handle.dir.join(format!("step-{step_num:04}-verify.png"));
    let image = match capture_for_model(
        screen,
        &path,
        config.screenshot_width,
        config.screenshot_height,
    )
    .await
    {
        Ok(i) => i,
        Err(e) => {
            warn!(session = %handle.id, error = %e, "verification screenshot failed");
            return None;
        }
    };
    *handle.latest_screenshot.lock_recover() = Some(path);

    let prompt = LlmPromptBuilder::build_verification_prompt(expected);
    let response = match vision.ask(&prompt, image).await {
        Ok(r) => r,
        Err(e) => {
            warn!(session = %handle.id, error = %e, "verification call failed");
            return None;
        }
    };

    let result = LlmPromptBuilder::parse_verification(expected, &response);
    if let Some(ref v) = result {
        handle.emit(ObserveActEvent::VerificationDone { result: v.clone() });
    }
    result
}

// ── Preflight ──────────────────────────────────────────────────────────────

/// What the machine can and cannot do, reported before a session starts.
#[derive(Debug, Clone, Serialize)]
pub struct Preflight {
    pub platform: String,
    /// Automation tools not found on `$PATH`.
    pub missing_tools: Vec<String>,
    /// Screen size in logical units, or `None` when the probe failed.
    pub logical_screen: Option<(u32, u32)>,
    /// Why the screen probe failed, when it did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_error: Option<String>,
    /// True when every prerequisite for running a session is satisfied.
    pub ready: bool,
}

/// Check whether this machine can run an observe-act session, and say exactly
/// what is missing when it cannot.
///
/// Every element has a distinct remedy — install `cliclick`, grant Screen
/// Recording, pick a vision model — so they are reported separately rather
/// than collapsed into one "not supported".
pub async fn preflight(screen: &dyn ScreenDriver) -> Preflight {
    let missing_tools = desktop_agent::check_prerequisites().await;
    let (logical_screen, screen_error) = match screen.logical_screen().await {
        Ok(size) => (Some(size), None),
        Err(e) => (None, Some(e.to_string())),
    };
    Preflight {
        platform: desktop_agent::detect_platform().to_string(),
        ready: missing_tools.is_empty() && logical_screen.is_some(),
        missing_tools,
        logical_screen,
        screen_error,
    }
}

// ── Config persistence ─────────────────────────────────────────────────────

/// The operator's saved configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoredConfig {
    #[serde(flatten)]
    pub config: ObserveActConfig,
    #[serde(default = "SafetyRails::default")]
    pub safety: SafetyRails,
}

/// Read the saved configuration, falling back to defaults.
///
/// A config file that cannot be parsed returns the defaults *and logs*: the
/// alternative — refusing to serve the panel — would leave the operator with
/// no way to fix the file through the UI that wrote it.
pub fn load_config(root: &Path) -> StoredConfig {
    let path = root.join("config.json");
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            warn!(path = ?path, error = %e, "observe-act config unreadable — using defaults");
            StoredConfig::default()
        }),
        Err(_) => StoredConfig::default(),
    }
}

/// Persist the configuration.
pub fn save_config(root: &Path, config: &StoredConfig) -> Result<()> {
    std::fs::create_dir_all(root).with_context(|| format!("creating {root:?}"))?;
    let bytes = serde_json::to_vec_pretty(config).context("serialising observe-act config")?;
    std::fs::write(root.join("config.json"), bytes)
        .with_context(|| format!("writing observe-act config to {root:?}"))?;
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observe_act::ScreenRegion;
    use std::sync::atomic::AtomicUsize;

    /// A driver that records what it was asked to do and never touches a
    /// screen. The loop's safety rules are about which actions reach a driver
    /// at all, so a recording one is the whole instrument.
    struct FakeScreen {
        performed: Mutex<Vec<ObserveActAction>>,
        captures: AtomicUsize,
        logical: (u32, u32),
        /// Pixel size of the PNGs it writes — the "capture space".
        capture_size: (u32, u32),
    }

    impl FakeScreen {
        fn new() -> Self {
            Self {
                performed: Mutex::new(Vec::new()),
                captures: AtomicUsize::new(0),
                logical: (1000, 1000),
                capture_size: (1000, 1000),
            }
        }

        fn retina() -> Self {
            Self {
                performed: Mutex::new(Vec::new()),
                captures: AtomicUsize::new(0),
                logical: (1000, 1000),
                capture_size: (2000, 2000),
            }
        }

        fn performed(&self) -> Vec<ObserveActAction> {
            self.performed.lock_recover().clone()
        }
    }

    #[async_trait::async_trait]
    impl ScreenDriver for FakeScreen {
        async fn logical_screen(&self) -> Result<(u32, u32)> {
            Ok(self.logical)
        }

        async fn capture(&self, path: &Path) -> Result<()> {
            self.captures.fetch_add(1, Ordering::SeqCst);
            let (w, h) = self.capture_size;
            image::RgbImage::new(w, h).save(path)?;
            Ok(())
        }

        async fn perform(&self, action: &ObserveActAction) -> Result<()> {
            self.performed.lock_recover().push(action.clone());
            Ok(())
        }
    }

    /// Replies from a list, then keeps repeating the last one.
    struct ScriptedVision {
        replies: Mutex<Vec<String>>,
        asked: Mutex<Vec<String>>,
    }

    impl ScriptedVision {
        fn new(replies: &[&str]) -> Self {
            Self {
                replies: Mutex::new(replies.iter().rev().map(|s| s.to_string()).collect()),
                asked: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl VisionModel for ScriptedVision {
        async fn ask(&self, prompt: &str, _image: EncodedImage) -> Result<String> {
            self.asked.lock_recover().push(prompt.to_string());
            let mut replies = self.replies.lock_recover();
            if replies.len() > 1 {
                Ok(replies.pop().unwrap_or_default())
            } else {
                Ok(replies.last().cloned().unwrap_or_else(|| "[]".into()))
            }
        }

        fn label(&self) -> String {
            "scripted/test".to_string()
        }
    }

    /// A config whose screenshot cap matches [`FakeScreen`]'s capture size, so
    /// image space and logical space coincide and a test about safety is not
    /// also a test about scaling. The scaling tests set their own cap.
    fn config(mode: SafetyMode, max_steps: usize) -> ObserveActConfig {
        ObserveActConfig {
            observation_interval_ms: 0,
            max_steps,
            screenshot_width: 1000,
            screenshot_height: 1000,
            verify_after_action: false,
            safety_mode: mode,
            ..ObserveActConfig::default()
        }
    }

    fn spec(
        task: &str,
        cfg: ObserveActConfig,
        safety: SafetyRails,
        screen: Arc<FakeScreen>,
        vision: Arc<ScriptedVision>,
    ) -> StartSpec {
        StartSpec {
            task: task.to_string(),
            config: cfg,
            safety,
            screen,
            vision,
        }
    }

    const CLICK_THEN_DONE: &[&str] = &[
        r#"{"reasoning":"click it","expected_change":"","actions":[{"type":"click","x":100,"y":200}]}"#,
        r#"{"reasoning":"finished","expected_change":"","actions":[{"type":"done","summary":"all set"}]}"#,
    ];

    // ── Safety mode ────────────────────────────────────────────────────

    /// Restricted mode is the one an operator reaches for to watch an agent
    /// *before* letting it act. If a single action leaked through, the mode
    /// would be worse than useless — it would be a lie the operator relied on.
    #[tokio::test]
    async fn restricted_mode_executes_nothing_but_records_the_proposal() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let registry = ObserveActRegistry::new(tmp.path().to_path_buf());
        let screen = Arc::new(FakeScreen::new());
        let vision = Arc::new(ScriptedVision::new(CLICK_THEN_DONE));

        let handle = registry
            .start_blocking(spec(
                "open the thing",
                config(SafetyMode::Restricted, 4),
                SafetyRails::default(),
                Arc::clone(&screen),
                vision,
            ))
            .await
            .expect("session starts");

        assert!(
            screen.performed().is_empty(),
            "restricted mode performed {:?}",
            screen.performed()
        );
        let session = handle.snapshot();
        let proposals: Vec<_> = session
            .steps
            .iter()
            .flat_map(|s| s.proposed_actions.clone())
            .collect();
        assert!(
            proposals.contains(&ObserveActAction::Click { x: 100, y: 200 }),
            "the proposal should still be recorded: {proposals:?}"
        );
    }

    #[tokio::test]
    async fn autonomous_mode_executes_without_asking() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let registry = ObserveActRegistry::new(tmp.path().to_path_buf());
        let screen = Arc::new(FakeScreen::new());
        let vision = Arc::new(ScriptedVision::new(CLICK_THEN_DONE));

        let handle = registry
            .start_blocking(spec(
                "open the thing",
                config(SafetyMode::Autonomous, 4),
                SafetyRails::default(),
                Arc::clone(&screen),
                vision,
            ))
            .await
            .expect("session starts");

        assert_eq!(
            screen.performed(),
            vec![ObserveActAction::Click { x: 100, y: 200 }]
        );
        assert_eq!(handle.status(), SessionStatus::Completed);
        assert_eq!(
            handle.summary().completion_summary.as_deref(),
            Some("all set")
        );
    }

    /// Cautious mode's contract is the confirmation gate. A destructive action
    /// must not reach the driver while the answer is outstanding, and a
    /// refusal must keep it from reaching the driver at all.
    #[tokio::test]
    async fn cautious_mode_blocks_a_destructive_action_until_the_operator_answers() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let registry = Arc::new(ObserveActRegistry::new(tmp.path().to_path_buf()));
        let screen = Arc::new(FakeScreen::new());
        let vision = Arc::new(ScriptedVision::new(&[
            r#"{"reasoning":"quit it","expected_change":"","actions":[{"type":"key_combo","keys":["ctrl","q"]}]}"#,
            r#"{"reasoning":"done","expected_change":"","actions":[{"type":"done","summary":"stopped"}]}"#,
        ]));

        let handle = registry
            .start(spec(
                "quit the app",
                config(SafetyMode::Cautious, 4),
                SafetyRails::default(),
                Arc::clone(&screen),
                vision,
            ))
            .await
            .expect("session starts");

        let pending = wait_for_pending(&handle).await.expect("approval requested");
        assert!(
            screen.performed().is_empty(),
            "nothing may run while the operator has not answered"
        );

        assert!(handle.resolve_approval(Some(&pending.id), false));
        wait_for_terminal(&handle).await;

        assert!(
            screen.performed().is_empty(),
            "a declined action must never reach the driver: {:?}",
            screen.performed()
        );
    }

    #[tokio::test]
    async fn cautious_mode_runs_the_action_once_approved() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let registry = Arc::new(ObserveActRegistry::new(tmp.path().to_path_buf()));
        let screen = Arc::new(FakeScreen::new());
        let vision = Arc::new(ScriptedVision::new(&[
            r#"{"reasoning":"quit","expected_change":"","actions":[{"type":"key_combo","keys":["ctrl","q"]}]}"#,
            r#"{"reasoning":"done","expected_change":"","actions":[{"type":"done","summary":"stopped"}]}"#,
        ]));

        let handle = registry
            .start(spec(
                "quit the app",
                config(SafetyMode::Cautious, 4),
                SafetyRails::default(),
                Arc::clone(&screen),
                vision,
            ))
            .await
            .expect("session starts");

        let pending = wait_for_pending(&handle).await.expect("approval requested");
        assert!(handle.resolve_approval(Some(&pending.id), true));
        wait_for_terminal(&handle).await;

        assert_eq!(
            screen.performed(),
            vec![ObserveActAction::KeyCombo {
                keys: vec!["ctrl".into(), "q".into()],
            }]
        );
    }

    /// A non-destructive action in cautious mode must not stop to ask —
    /// otherwise the mode is unusable and operators switch to autonomous,
    /// which is the opposite of what the gate is for.
    #[tokio::test]
    async fn cautious_mode_does_not_gate_an_ordinary_click() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let registry = ObserveActRegistry::new(tmp.path().to_path_buf());
        let screen = Arc::new(FakeScreen::new());
        let vision = Arc::new(ScriptedVision::new(CLICK_THEN_DONE));

        registry
            .start_blocking(spec(
                "click",
                config(SafetyMode::Cautious, 4),
                SafetyRails::default(),
                Arc::clone(&screen),
                vision,
            ))
            .await
            .expect("session starts");

        assert_eq!(
            screen.performed(),
            vec![ObserveActAction::Click { x: 100, y: 200 }]
        );
    }

    // ── Safety rails ───────────────────────────────────────────────────

    #[tokio::test]
    async fn a_click_in_a_forbidden_region_never_reaches_the_driver() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let registry = ObserveActRegistry::new(tmp.path().to_path_buf());
        let screen = Arc::new(FakeScreen::new());
        let vision = Arc::new(ScriptedVision::new(&[
            r#"{"reasoning":"menu bar","expected_change":"","actions":[{"type":"click","x":10,"y":5}]}"#,
        ]));

        let safety = SafetyRails {
            forbidden_regions: vec![ScreenRegion {
                x: 0,
                y: 0,
                width: 1000,
                height: 30,
                label: "menu bar".into(),
            }],
            ..SafetyRails::default()
        };

        let handle = registry
            .start_blocking(spec(
                "click the menu bar",
                config(SafetyMode::Autonomous, 2),
                safety,
                Arc::clone(&screen),
                vision,
            ))
            .await
            .expect("session starts");

        assert!(screen.performed().is_empty());
        let session = handle.snapshot();
        assert!(
            session
                .steps
                .iter()
                .any(|s| s.llm_reasoning.contains("menu bar")
                    || s.verification_result
                        .as_ref()
                        .is_some_and(|v| v.actual_observation.contains("menu bar"))),
            "the block should be recorded against the step"
        );
    }

    /// The safety rails run on the coordinates that will actually be clicked,
    /// not the ones the model gave — otherwise a downscaled screenshot lets a
    /// forbidden region be reached by naming a coordinate outside it.
    ///
    /// The capture is 2000px of a 1000pt screen and the image is capped at
    /// 500, so model coordinates double. A click at y=120 is outside the
    /// forbidden 200..300 band *as the model stated it* and inside it once
    /// mapped — so validating before mapping would let this through.
    #[tokio::test]
    async fn forbidden_regions_are_checked_in_logical_space() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let registry = ObserveActRegistry::new(tmp.path().to_path_buf());
        let screen = Arc::new(FakeScreen::retina());
        let vision = Arc::new(ScriptedVision::new(&[
            r#"{"reasoning":"aim below the band","expected_change":"","actions":[{"type":"click","x":100,"y":120}]}"#,
        ]));
        let safety = SafetyRails {
            forbidden_regions: vec![ScreenRegion {
                x: 0,
                y: 200,
                width: 1000,
                height: 100,
                label: "toolbar".into(),
            }],
            ..SafetyRails::default()
        };
        let cfg = ObserveActConfig {
            screenshot_width: 500,
            screenshot_height: 500,
            ..config(SafetyMode::Autonomous, 2)
        };

        registry
            .start_blocking(spec(
                "click below the toolbar",
                cfg,
                safety,
                Arc::clone(&screen),
                vision,
            ))
            .await
            .expect("session starts");

        assert!(
            screen.performed().is_empty(),
            "a coordinate that maps into a forbidden region must be blocked: {:?}",
            screen.performed()
        );
    }

    #[tokio::test]
    async fn too_many_actions_in_one_step_are_rejected_as_a_batch() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let registry = ObserveActRegistry::new(tmp.path().to_path_buf());
        let screen = Arc::new(FakeScreen::new());
        let vision = Arc::new(ScriptedVision::new(&[
            r#"{"reasoning":"burst","expected_change":"","actions":[
                {"type":"click","x":1,"y":1},{"type":"click","x":2,"y":2},
                {"type":"click","x":3,"y":3},{"type":"click","x":4,"y":4}]}"#,
        ]));
        let safety = SafetyRails {
            max_actions_per_step: 2,
            ..SafetyRails::default()
        };

        registry
            .start_blocking(spec(
                "burst",
                config(SafetyMode::Autonomous, 2),
                safety,
                Arc::clone(&screen),
                vision,
            ))
            .await
            .expect("session starts");

        assert!(
            screen.performed().is_empty(),
            "an over-long batch is refused whole, not truncated"
        );
    }

    #[test]
    fn safety_rails_are_clamped_to_the_ceiling() {
        let clamped = clamp_safety(SafetyRails {
            max_actions_per_step: 100_000,
            rate_limit_ms: 10_000_000,
            ..SafetyRails::default()
        });
        assert_eq!(clamped.max_actions_per_step, MAX_ACTIONS_PER_STEP_CEILING);
        assert_eq!(clamped.rate_limit_ms, 10_000);
        assert_eq!(
            clamp_safety(SafetyRails {
                max_actions_per_step: 0,
                ..SafetyRails::default()
            })
            .max_actions_per_step,
            1
        );
    }

    // ── Coordinate mapping ─────────────────────────────────────────────

    /// End-to-end proof that the model's image-space answer arrives at the
    /// driver in logical space. The capture is 2× the logical screen and the
    /// image sent to the model is capped at 500, so a 250,250 answer must land
    /// at 500,500 — halved by the downscale, not by the backing store.
    #[tokio::test]
    async fn model_coordinates_reach_the_driver_in_logical_space() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let registry = ObserveActRegistry::new(tmp.path().to_path_buf());
        let screen = Arc::new(FakeScreen::retina());
        let vision = Arc::new(ScriptedVision::new(&[
            r#"{"reasoning":"middle","expected_change":"","actions":[{"type":"click","x":250,"y":250}]}"#,
        ]));

        let cfg = ObserveActConfig {
            screenshot_width: 500,
            screenshot_height: 500,
            ..config(SafetyMode::Autonomous, 1)
        };

        registry
            .start_blocking(spec(
                "click the middle",
                cfg,
                SafetyRails::default(),
                Arc::clone(&screen),
                vision,
            ))
            .await
            .expect("session starts");

        assert_eq!(
            screen.performed(),
            vec![ObserveActAction::Click { x: 500, y: 500 }]
        );
    }

    // ── Session lifecycle ──────────────────────────────────────────────

    #[tokio::test]
    async fn a_second_session_is_refused_while_one_is_active() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let registry = Arc::new(ObserveActRegistry::new(tmp.path().to_path_buf()));
        let screen = Arc::new(FakeScreen::new());
        // Never emits `done`, so the session stays running.
        let vision = Arc::new(ScriptedVision::new(&[
            r#"{"reasoning":"wait","expected_change":"","actions":[{"type":"wait","ms":50}]}"#,
        ]));

        let first = registry
            .start(spec(
                "first",
                ObserveActConfig {
                    observation_interval_ms: 50,
                    ..config(SafetyMode::Autonomous, 100)
                },
                SafetyRails::default(),
                Arc::clone(&screen),
                Arc::clone(&vision),
            ))
            .await
            .expect("first starts");

        let second = registry
            .start(spec(
                "second",
                config(SafetyMode::Autonomous, 1),
                SafetyRails::default(),
                Arc::clone(&screen),
                vision,
            ))
            .await;
        assert!(second.is_err(), "two loops must not drive one desktop");

        first.abort("test teardown");
        wait_for_terminal(&first).await;
    }

    #[tokio::test]
    async fn abort_stops_the_loop_and_records_why() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let registry = Arc::new(ObserveActRegistry::new(tmp.path().to_path_buf()));
        let screen = Arc::new(FakeScreen::new());
        let vision = Arc::new(ScriptedVision::new(&[
            r#"{"reasoning":"wait","expected_change":"","actions":[{"type":"wait","ms":10}]}"#,
        ]));

        let handle = registry
            .start(spec(
                "forever",
                ObserveActConfig {
                    observation_interval_ms: 20,
                    ..config(SafetyMode::Autonomous, 1000)
                },
                SafetyRails::default(),
                Arc::clone(&screen),
                vision,
            ))
            .await
            .expect("starts");

        handle.abort("operator pressed stop");
        wait_for_terminal(&handle).await;
        assert_eq!(handle.status(), SessionStatus::Aborted);
    }

    #[tokio::test]
    async fn max_steps_ends_the_session() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let registry = ObserveActRegistry::new(tmp.path().to_path_buf());
        let screen = Arc::new(FakeScreen::new());
        let vision = Arc::new(ScriptedVision::new(&[
            r#"{"reasoning":"again","expected_change":"","actions":[{"type":"click","x":1,"y":1}]}"#,
        ]));

        let handle = registry
            .start_blocking(spec(
                "loop forever",
                config(SafetyMode::Autonomous, 3),
                SafetyRails::default(),
                Arc::clone(&screen),
                vision,
            ))
            .await
            .expect("starts");

        assert_eq!(handle.snapshot().steps.len(), 3);
        assert!(handle.snapshot().is_complete());
    }

    /// A model whose answer cannot be parsed produces a step with no actions,
    /// and the raw answer is kept as the reasoning — it is the only thing an
    /// operator has to explain why nothing moved.
    #[tokio::test]
    async fn an_unparseable_answer_records_itself_and_does_not_act() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let registry = ObserveActRegistry::new(tmp.path().to_path_buf());
        let screen = Arc::new(FakeScreen::new());
        let vision = Arc::new(ScriptedVision::new(&["I can't see the screen well."]));

        let handle = registry
            .start_blocking(spec(
                "do something",
                config(SafetyMode::Autonomous, 1),
                SafetyRails::default(),
                Arc::clone(&screen),
                vision,
            ))
            .await
            .expect("starts");

        assert!(screen.performed().is_empty());
        let steps = handle.snapshot().steps;
        assert_eq!(steps.len(), 1);
        assert!(steps[0].llm_reasoning.contains("can't see the screen"));
    }

    // ── Persistence ────────────────────────────────────────────────────

    /// A session the daemon was running when it died is not running now.
    /// Reading it back as `Running` would leave a phantom in the history that
    /// no stop button can ever clear.
    #[tokio::test]
    async fn a_session_interrupted_by_a_restart_reloads_as_aborted() {
        let tmp = tempfile::tempdir().expect("tempdir");
        {
            let registry = ObserveActRegistry::new(tmp.path().to_path_buf());
            let screen = Arc::new(FakeScreen::new());
            let vision = Arc::new(ScriptedVision::new(CLICK_THEN_DONE));
            let handle = registry
                .start_blocking(spec(
                    "task",
                    config(SafetyMode::Autonomous, 4),
                    SafetyRails::default(),
                    screen,
                    vision,
                ))
                .await
                .expect("starts");
            // Forge the state a crash would leave behind.
            handle.state.lock_recover().status = SessionStatus::Running;
            handle.persist();
        }

        let reloaded = ObserveActRegistry::new(tmp.path().to_path_buf());
        let sessions = reloaded.list();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].status(), SessionStatus::Aborted);
        assert_eq!(sessions[0].snapshot().task, "task");
        assert!(
            reloaded.active().is_none(),
            "a reloaded session must not block a new one"
        );
    }

    #[test]
    fn config_round_trips_through_disk() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let stored = StoredConfig {
            config: ObserveActConfig {
                max_steps: 7,
                safety_mode: SafetyMode::Restricted,
                ..ObserveActConfig::default()
            },
            safety: SafetyRails {
                rate_limit_ms: 350,
                ..SafetyRails::default()
            },
        };
        save_config(tmp.path(), &stored).expect("saves");
        let back = load_config(tmp.path());
        assert_eq!(back.config.max_steps, 7);
        assert_eq!(back.config.safety_mode, SafetyMode::Restricted);
        assert_eq!(back.safety.rate_limit_ms, 350);
    }

    /// A corrupt config must not take the panel down with it — the operator
    /// needs the UI that wrote the file in order to fix it.
    #[test]
    fn an_unreadable_config_falls_back_to_defaults() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::write(tmp.path().join("config.json"), b"{not json").expect("write");
        let back = load_config(tmp.path());
        assert_eq!(back.config.max_steps, ObserveActConfig::default().max_steps);
    }

    // ── Action translation ─────────────────────────────────────────────

    #[test]
    fn a_key_combo_splits_into_modifiers_and_a_key() {
        let actions = to_desktop_actions(&ObserveActAction::KeyCombo {
            keys: vec!["ctrl".into(), "shift".into(), "s".into()],
        });
        assert_eq!(
            actions,
            vec![DesktopAction::KeyCombo {
                modifiers: vec!["ctrl".into(), "shift".into()],
                key: "s".into(),
            }]
        );
    }

    #[test]
    fn a_lone_key_becomes_a_key_press() {
        let actions = to_desktop_actions(&ObserveActAction::KeyCombo {
            keys: vec!["Return".into()],
        });
        assert_eq!(
            actions,
            vec![DesktopAction::PressKey {
                key: "Return".into()
            }]
        );
    }

    /// An unbounded wait would park the session with nothing on screen to say
    /// why, so it is clamped rather than honoured.
    #[test]
    fn an_absurd_wait_is_clamped() {
        let actions = to_desktop_actions(&ObserveActAction::Wait { ms: 3_600_000 });
        assert_eq!(actions, vec![DesktopAction::Delay { ms: MAX_WAIT_MS }]);
    }

    #[test]
    fn an_absurd_scroll_is_bounded() {
        let actions = to_desktop_actions(&ObserveActAction::Scroll {
            direction: ScrollDirection::Down,
            amount: 100_000,
        });
        assert_eq!(actions.len(), 50);
    }

    #[test]
    fn typed_text_is_bounded() {
        let long = "a".repeat(MAX_TYPE_CHARS + 100);
        let actions = to_desktop_actions(&ObserveActAction::Type { text: long });
        match actions.as_slice() {
            [DesktopAction::TypeText { text }] => assert_eq!(text.chars().count(), MAX_TYPE_CHARS),
            other => panic!("expected one TypeText, got {other:?}"),
        }
    }

    #[test]
    fn done_and_screenshot_drive_nothing() {
        assert!(to_desktop_actions(&ObserveActAction::Screenshot).is_empty());
        assert!(to_desktop_actions(&ObserveActAction::Done {
            summary: "x".into()
        })
        .is_empty());
    }

    /// The logical-screen probe must not be the native-pixel one: on macOS
    /// those differ by the backing scale, and using the wrong one puts every
    /// click in the wrong quadrant.
    #[test]
    fn the_macos_screen_probe_asks_for_logical_bounds() {
        let cmd = logical_screen_cmd(desktop_agent::DesktopPlatform::MacOS);
        assert!(cmd.contains("bounds of window of desktop"), "{cmd}");
        assert!(!cmd.contains("system_profiler"), "{cmd}");
    }

    // ── Image encoding ─────────────────────────────────────────────────

    #[test]
    fn a_large_capture_is_downscaled_to_the_cap() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("shot.png");
        image::RgbImage::new(2560, 1600).save(&path).expect("write");

        let encoded = encode_for_model(&path, 1280, 720).expect("encodes");
        assert!(encoded.width <= 1280 && encoded.height <= 720);
        assert_eq!(encoded.media_type, "image/jpeg");
        // Aspect ratio preserved: 2560×1600 is 1.6, so the height binds first.
        assert_eq!(encoded.height, 720);
        assert_eq!(encoded.width, 1152);
    }

    /// A capture already under the cap is left alone — upscaling it would add
    /// blur and bytes and no detail.
    #[test]
    fn a_small_capture_is_not_upscaled() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("small.png");
        image::RgbImage::new(400, 300).save(&path).expect("write");

        let encoded = encode_for_model(&path, 1280, 720).expect("encodes");
        assert_eq!((encoded.width, encoded.height), (400, 300));
    }

    // ── Helpers ────────────────────────────────────────────────────────

    /// Poll until the session is waiting on a human, or give up.
    async fn wait_for_pending(handle: &SessionHandle) -> Option<PendingApproval> {
        for _ in 0..200 {
            if let Some(p) = handle.pending_approval() {
                return Some(p);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        None
    }

    /// Poll until the session reaches a terminal state.
    async fn wait_for_terminal(handle: &SessionHandle) {
        for _ in 0..400 {
            if handle.snapshot().is_complete() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}
