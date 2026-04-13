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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enter_focus_sets_active() {
        let mut mgr = FocusManager::new();
        assert!(!mgr.is_in_focus());
        mgr.enter_focus(FocusConfig::default_deep(), 0);
        assert!(mgr.is_in_focus());
    }

    #[test]
    fn test_exit_focus_archives_session() {
        let mut mgr = FocusManager::new();
        mgr.enter_focus(FocusConfig::default_deep(), 0);
        mgr.exit_focus(100);
        assert!(!mgr.is_in_focus());
        assert_eq!(mgr.session_count(), 1);
    }

    #[test]
    fn test_record_distraction_increments_count() {
        let mut mgr = FocusManager::new();
        mgr.enter_focus(FocusConfig::default_deep(), 0);
        mgr.record_distraction();
        mgr.record_distraction();
        assert_eq!(mgr.active.as_ref().unwrap().distraction_count, 2);
    }

    #[test]
    fn test_record_distraction_no_session_is_noop() {
        let mut mgr = FocusManager::new();
        mgr.record_distraction(); // should not panic
        assert!(mgr.active.is_none());
    }

    #[test]
    fn test_auto_exit_triggers_after_limit() {
        let mut mgr = FocusManager::new();
        let cfg = FocusConfig { auto_exit_after_secs: Some(60), ..Default::default() };
        mgr.enter_focus(cfg, 0);
        assert!(mgr.should_auto_exit(60));
        assert!(!mgr.should_auto_exit(59));
    }

    #[test]
    fn test_no_auto_exit_when_limit_not_set() {
        let mut mgr = FocusManager::new();
        mgr.enter_focus(FocusConfig::default_deep(), 0);
        assert!(!mgr.should_auto_exit(u64::MAX));
    }

    #[test]
    fn test_notification_level_ordering() {
        assert!(NotificationLevel::Verbose > NotificationLevel::Normal);
        assert!(NotificationLevel::Normal > NotificationLevel::Minimal);
        assert!(NotificationLevel::Minimal > NotificationLevel::Silent);
    }

    #[test]
    fn test_multiple_sessions_accumulated() {
        let mut mgr = FocusManager::new();
        for i in 0..3u64 {
            mgr.enter_focus(FocusConfig::default_deep(), i * 100);
            mgr.exit_focus(i * 100 + 50);
        }
        assert_eq!(mgr.session_count(), 3);
        assert!(!mgr.is_in_focus());
    }
}
