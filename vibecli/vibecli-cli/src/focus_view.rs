//! focus_view — Deep-focus session gating and distraction tracking.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationLevel { Silent, Minimal, Normal, Verbose }

#[derive(Debug, Clone)]
pub struct FocusConfig {
    pub level: NotificationLevel,
    pub auto_exit_after_secs: Option<u64>,
}

impl FocusConfig {
    pub fn default_deep() -> Self {
        Self { level: NotificationLevel::Silent, auto_exit_after_secs: None }
    }
}

impl Default for FocusConfig {
    fn default() -> Self { Self::default_deep() }
}

#[derive(Debug, Clone)]
pub struct FocusSession {
    pub config: FocusConfig,
    pub started_at: u64,
    pub distraction_count: u32,
}

#[derive(Debug, Default)]
pub struct FocusManager {
    pub active: Option<FocusSession>,
    pub sessions: Vec<FocusSession>,
}

impl FocusManager {
    pub fn new() -> Self { Self::default() }

    pub fn enter_focus(&mut self, config: FocusConfig, now: u64) {
        self.active = Some(FocusSession { config, started_at: now, distraction_count: 0 });
    }

    pub fn exit_focus(&mut self, _now: u64) {
        if let Some(session) = self.active.take() {
            self.sessions.push(session);
        }
    }

    pub fn is_in_focus(&self) -> bool { self.active.is_some() }

    pub fn session_count(&self) -> usize { self.sessions.len() }

    pub fn record_distraction(&mut self) {
        if let Some(s) = self.active.as_mut() { s.distraction_count += 1; }
    }

    pub fn should_auto_exit(&self, now: u64) -> bool {
        if let Some(s) = &self.active {
            if let Some(limit) = s.config.auto_exit_after_secs {
                return now >= s.started_at + limit;
            }
        }
        false
    }
}
