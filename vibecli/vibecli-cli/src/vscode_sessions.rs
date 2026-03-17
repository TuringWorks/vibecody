
use std::time::{Duration, SystemTime};

/// VS Code session browser — list all VibeCLI sessions in VS Code sidebar;
/// open as full editors.

#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Active,
    Paused,
    Completed,
    Abandoned,
    Archived,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionType {
    Chat,
    Agent,
    Batch,
    Migration,
    Review,
}

#[derive(Debug, Clone)]
pub struct VsCodeSession {
    pub id: String,
    pub title: String,
    pub session_type: SessionType,
    pub state: SessionState,
    pub workspace_path: String,
    pub provider: String,
    pub model: String,
    pub messages_count: usize,
    pub tokens_used: u64,
    pub cost: f64,
    pub files_modified: Vec<String>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub tags: Vec<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionFilter {
    pub state: Option<SessionState>,
    pub session_type: Option<SessionType>,
    pub workspace: Option<String>,
    pub search: Option<String>,
    pub tags: Vec<String>,
    pub since: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub struct SessionBrowser {
    pub sessions: Vec<VsCodeSession>,
    pub config: SessionBrowserConfig,
}

#[derive(Debug, Clone)]
pub struct SessionBrowserConfig {
    pub sessions_dir: String,
    pub max_display: usize,
    pub auto_archive_days: u64,
    pub show_cost: bool,
    pub group_by_workspace: bool,
}

#[derive(Debug, Clone)]
pub struct SessionGroup {
    pub workspace: String,
    pub sessions: Vec<String>,
    pub total_cost: f64,
    pub total_tokens: u64,
}

impl Default for SessionBrowserConfig {
    fn default() -> Self {
        Self {
            sessions_dir: "~/.vibecli/sessions/".to_string(),
            max_display: 50,
            auto_archive_days: 30,
            show_cost: true,
            group_by_workspace: true,
        }
    }
}

impl VsCodeSession {
    pub fn new(
        title: &str,
        session_type: SessionType,
        workspace: &str,
        provider: &str,
        model: &str,
    ) -> Self {
        let now = SystemTime::now();
        let id = format!(
            "session-{}",
            now.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                % 1_000_000_000
        );
        Self {
            id,
            title: title.to_string(),
            session_type,
            state: SessionState::Active,
            workspace_path: workspace.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            messages_count: 0,
            tokens_used: 0,
            cost: 0.0,
            files_modified: Vec::new(),
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
            summary: None,
        }
    }

    pub fn add_message(&mut self) {
        self.messages_count += 1;
        self.updated_at = SystemTime::now();
    }

    pub fn add_file(&mut self, path: &str) {
        if !self.files_modified.contains(&path.to_string()) {
            self.files_modified.push(path.to_string());
            self.updated_at = SystemTime::now();
        }
    }

    pub fn add_tag(&mut self, tag: &str) {
        if !self.tags.contains(&tag.to_string()) {
            self.tags.push(tag.to_string());
        }
    }

    pub fn set_summary(&mut self, summary: &str) {
        self.summary = Some(summary.to_string());
    }

    pub fn pause(&mut self) {
        if self.state == SessionState::Active {
            self.state = SessionState::Paused;
            self.updated_at = SystemTime::now();
        }
    }

    pub fn resume(&mut self) {
        if self.state == SessionState::Paused {
            self.state = SessionState::Active;
            self.updated_at = SystemTime::now();
        }
    }

    pub fn complete(&mut self) {
        self.state = SessionState::Completed;
        self.updated_at = SystemTime::now();
    }

    pub fn abandon(&mut self) {
        self.state = SessionState::Abandoned;
        self.updated_at = SystemTime::now();
    }

    pub fn archive(&mut self) {
        self.state = SessionState::Archived;
        self.updated_at = SystemTime::now();
    }

    pub fn elapsed(&self) -> Duration {
        self.updated_at
            .duration_since(self.created_at)
            .unwrap_or_default()
    }

    pub fn to_tree_item(&self) -> String {
        let state_icon = match self.state {
            SessionState::Active => "[*]",
            SessionState::Paused => "[||]",
            SessionState::Completed => "[v]",
            SessionState::Abandoned => "[x]",
            SessionState::Archived => "[~]",
        };
        let type_label = match self.session_type {
            SessionType::Chat => "Chat",
            SessionType::Agent => "Agent",
            SessionType::Batch => "Batch",
            SessionType::Migration => "Migration",
            SessionType::Review => "Review",
        };
        format!(
            "{} {} - {} ({}, {} msgs, {})",
            state_icon,
            self.title,
            type_label,
            self.model,
            self.messages_count,
            format_cost(self.cost),
        )
    }

    pub fn matches_filter(&self, filter: &SessionFilter) -> bool {
        if let Some(ref state) = filter.state {
            if self.state != *state {
                return false;
            }
        }
        if let Some(ref session_type) = filter.session_type {
            if self.session_type != *session_type {
                return false;
            }
        }
        if let Some(ref workspace) = filter.workspace {
            if !self.workspace_path.contains(workspace.as_str()) {
                return false;
            }
        }
        if let Some(ref search) = filter.search {
            let lower = search.to_lowercase();
            let title_match = self.title.to_lowercase().contains(&lower);
            let summary_match = self
                .summary
                .as_ref()
                .map(|s| s.to_lowercase().contains(&lower))
                .unwrap_or(false);
            let tag_match = self.tags.iter().any(|t| t.to_lowercase().contains(&lower));
            if !title_match && !summary_match && !tag_match {
                return false;
            }
        }
        if !filter.tags.is_empty() {
            let has_all_tags = filter.tags.iter().all(|ft| self.tags.contains(ft));
            if !has_all_tags {
                return false;
            }
        }
        if let Some(since) = filter.since {
            if self.created_at < since {
                return false;
            }
        }
        true
    }
}

fn format_cost(cost: f64) -> String {
    if cost < 0.01 {
        "<$0.01".to_string()
    } else {
        format!("${:.2}", cost)
    }
}

impl SessionFilter {
    pub fn new() -> Self {
        Self {
            state: None,
            session_type: None,
            workspace: None,
            search: None,
            tags: Vec::new(),
            since: None,
        }
    }

    pub fn with_state(mut self, state: SessionState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn with_type(mut self, session_type: SessionType) -> Self {
        self.session_type = Some(session_type);
        self
    }

    pub fn with_search(mut self, query: &str) -> Self {
        self.search = Some(query.to_string());
        self
    }

    pub fn with_workspace(mut self, workspace: &str) -> Self {
        self.workspace = Some(workspace.to_string());
        self
    }

    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    pub fn with_since(mut self, since: SystemTime) -> Self {
        self.since = Some(since);
        self
    }
}

impl SessionBrowser {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            config: SessionBrowserConfig::default(),
        }
    }

    pub fn with_config(config: SessionBrowserConfig) -> Self {
        Self {
            sessions: Vec::new(),
            config,
        }
    }

    pub fn add_session(&mut self, session: VsCodeSession) {
        self.sessions.push(session);
    }

    pub fn list(&self, filter: &SessionFilter) -> Vec<&VsCodeSession> {
        self.sessions
            .iter()
            .filter(|s| s.matches_filter(filter))
            .take(self.config.max_display)
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<&VsCodeSession> {
        self.sessions.iter().find(|s| s.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut VsCodeSession> {
        self.sessions.iter_mut().find(|s| s.id == id)
    }

    pub fn group_by_workspace(&self) -> Vec<SessionGroup> {
        let mut groups: Vec<SessionGroup> = Vec::new();
        for session in &self.sessions {
            if let Some(group) = groups
                .iter_mut()
                .find(|g| g.workspace == session.workspace_path)
            {
                group.sessions.push(session.id.clone());
                group.total_cost += session.cost;
                group.total_tokens += session.tokens_used;
            } else {
                groups.push(SessionGroup {
                    workspace: session.workspace_path.clone(),
                    sessions: vec![session.id.clone()],
                    total_cost: session.cost,
                    total_tokens: session.tokens_used,
                });
            }
        }
        groups
    }

    pub fn search(&self, query: &str) -> Vec<&VsCodeSession> {
        let filter = SessionFilter::new().with_search(query);
        self.list(&filter)
    }

    pub fn recent(&self, n: usize) -> Vec<&VsCodeSession> {
        let mut sorted: Vec<&VsCodeSession> = self.sessions.iter().collect();
        sorted.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        sorted.truncate(n);
        sorted
    }

    pub fn archive_old(&mut self, days: u64) -> usize {
        let cutoff = SystemTime::now()
            .checked_sub(Duration::from_secs(days * 86400))
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let mut count = 0;
        for session in &mut self.sessions {
            if session.state != SessionState::Archived
                && session.state != SessionState::Active
                && session.updated_at < cutoff
            {
                session.state = SessionState::Archived;
                count += 1;
            }
        }
        count
    }

    pub fn total_cost(&self) -> f64 {
        self.sessions.iter().map(|s| s.cost).sum()
    }

    pub fn total_tokens(&self) -> u64 {
        self.sessions.iter().map(|s| s.tokens_used).sum()
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        let active = self
            .sessions
            .iter()
            .filter(|s| s.state == SessionState::Active)
            .count();
        let completed = self
            .sessions
            .iter()
            .filter(|s| s.state == SessionState::Completed)
            .count();
        let archived = self
            .sessions
            .iter()
            .filter(|s| s.state == SessionState::Archived)
            .count();
        (active, completed, archived)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session(title: &str, workspace: &str) -> VsCodeSession {
        VsCodeSession::new(title, SessionType::Chat, workspace, "anthropic", "claude-opus-4-6")
    }

    #[test]
    fn test_session_new() {
        let s = make_session("Test Session", "/home/user/project");
        assert_eq!(s.title, "Test Session");
        assert_eq!(s.state, SessionState::Active);
        assert_eq!(s.messages_count, 0);
        assert!(s.id.starts_with("session-"));
    }

    #[test]
    fn test_session_add_message() {
        let mut s = make_session("s", "/w");
        s.add_message();
        s.add_message();
        assert_eq!(s.messages_count, 2);
    }

    #[test]
    fn test_session_add_file_dedup() {
        let mut s = make_session("s", "/w");
        s.add_file("src/main.rs");
        s.add_file("src/lib.rs");
        s.add_file("src/main.rs");
        assert_eq!(s.files_modified.len(), 2);
    }

    #[test]
    fn test_session_add_tag_dedup() {
        let mut s = make_session("s", "/w");
        s.add_tag("rust");
        s.add_tag("ai");
        s.add_tag("rust");
        assert_eq!(s.tags.len(), 2);
    }

    #[test]
    fn test_session_set_summary() {
        let mut s = make_session("s", "/w");
        assert!(s.summary.is_none());
        s.set_summary("Implemented feature X");
        assert_eq!(s.summary, Some("Implemented feature X".to_string()));
    }

    #[test]
    fn test_session_pause_resume() {
        let mut s = make_session("s", "/w");
        s.pause();
        assert_eq!(s.state, SessionState::Paused);
        s.resume();
        assert_eq!(s.state, SessionState::Active);
    }

    #[test]
    fn test_session_pause_only_from_active() {
        let mut s = make_session("s", "/w");
        s.complete();
        s.pause();
        assert_eq!(s.state, SessionState::Completed);
    }

    #[test]
    fn test_session_resume_only_from_paused() {
        let mut s = make_session("s", "/w");
        s.complete();
        s.resume();
        assert_eq!(s.state, SessionState::Completed);
    }

    #[test]
    fn test_session_complete() {
        let mut s = make_session("s", "/w");
        s.complete();
        assert_eq!(s.state, SessionState::Completed);
    }

    #[test]
    fn test_session_abandon() {
        let mut s = make_session("s", "/w");
        s.abandon();
        assert_eq!(s.state, SessionState::Abandoned);
    }

    #[test]
    fn test_session_archive() {
        let mut s = make_session("s", "/w");
        s.archive();
        assert_eq!(s.state, SessionState::Archived);
    }

    #[test]
    fn test_session_elapsed() {
        let s = make_session("s", "/w");
        let elapsed = s.elapsed();
        assert!(elapsed.as_secs() < 1);
    }

    #[test]
    fn test_session_to_tree_item() {
        let mut s = make_session("Feature X", "/w");
        s.messages_count = 10;
        s.cost = 0.15;
        let item = s.to_tree_item();
        assert!(item.contains("Feature X"));
        assert!(item.contains("Chat"));
        assert!(item.contains("10 msgs"));
        assert!(item.contains("$0.15"));
    }

    #[test]
    fn test_session_to_tree_item_low_cost() {
        let s = make_session("s", "/w");
        let item = s.to_tree_item();
        assert!(item.contains("<$0.01"));
    }

    #[test]
    fn test_session_matches_filter_empty() {
        let s = make_session("s", "/w");
        let filter = SessionFilter::new();
        assert!(s.matches_filter(&filter));
    }

    #[test]
    fn test_session_matches_filter_state() {
        let s = make_session("s", "/w");
        let filter = SessionFilter::new().with_state(SessionState::Active);
        assert!(s.matches_filter(&filter));
        let filter2 = SessionFilter::new().with_state(SessionState::Completed);
        assert!(!s.matches_filter(&filter2));
    }

    #[test]
    fn test_session_matches_filter_type() {
        let s = make_session("s", "/w");
        let filter = SessionFilter::new().with_type(SessionType::Chat);
        assert!(s.matches_filter(&filter));
        let filter2 = SessionFilter::new().with_type(SessionType::Agent);
        assert!(!s.matches_filter(&filter2));
    }

    #[test]
    fn test_session_matches_filter_search_title() {
        let s = make_session("Refactor auth module", "/w");
        let filter = SessionFilter::new().with_search("auth");
        assert!(s.matches_filter(&filter));
        let filter2 = SessionFilter::new().with_search("database");
        assert!(!s.matches_filter(&filter2));
    }

    #[test]
    fn test_session_matches_filter_search_summary() {
        let mut s = make_session("s", "/w");
        s.set_summary("Fixed authentication bug");
        let filter = SessionFilter::new().with_search("authentication");
        assert!(s.matches_filter(&filter));
    }

    #[test]
    fn test_session_matches_filter_tags() {
        let mut s = make_session("s", "/w");
        s.add_tag("rust");
        s.add_tag("ai");
        let filter = SessionFilter::new().with_tag("rust");
        assert!(s.matches_filter(&filter));
        let filter2 = SessionFilter::new().with_tag("python");
        assert!(!s.matches_filter(&filter2));
    }

    #[test]
    fn test_session_matches_filter_workspace() {
        let s = make_session("s", "/home/user/project");
        let filter = SessionFilter::new().with_workspace("user/project");
        assert!(s.matches_filter(&filter));
        let filter2 = SessionFilter::new().with_workspace("other");
        assert!(!s.matches_filter(&filter2));
    }

    #[test]
    fn test_browser_new() {
        let browser = SessionBrowser::new();
        assert!(browser.sessions.is_empty());
        assert_eq!(browser.config.max_display, 50);
    }

    #[test]
    fn test_browser_add_and_get() {
        let mut browser = SessionBrowser::new();
        let s = make_session("s1", "/w");
        let id = s.id.clone();
        browser.add_session(s);
        assert!(browser.get(&id).is_some());
        assert!(browser.get("nonexistent").is_none());
    }

    #[test]
    fn test_browser_list_with_filter() {
        let mut browser = SessionBrowser::new();
        browser.add_session(make_session("s1", "/w"));
        let mut s2 = make_session("s2", "/w");
        s2.complete();
        browser.add_session(s2);
        let active = browser.list(&SessionFilter::new().with_state(SessionState::Active));
        assert_eq!(active.len(), 1);
        let completed = browser.list(&SessionFilter::new().with_state(SessionState::Completed));
        assert_eq!(completed.len(), 1);
    }

    #[test]
    fn test_browser_group_by_workspace() {
        let mut browser = SessionBrowser::new();
        let mut s1 = make_session("s1", "/project-a");
        s1.cost = 0.10;
        s1.tokens_used = 100;
        browser.add_session(s1);
        let mut s2 = make_session("s2", "/project-a");
        s2.cost = 0.20;
        s2.tokens_used = 200;
        browser.add_session(s2);
        browser.add_session(make_session("s3", "/project-b"));
        let groups = browser.group_by_workspace();
        assert_eq!(groups.len(), 2);
        let group_a = groups.iter().find(|g| g.workspace == "/project-a").unwrap();
        assert_eq!(group_a.sessions.len(), 2);
        assert!((group_a.total_cost - 0.30).abs() < f64::EPSILON);
        assert_eq!(group_a.total_tokens, 300);
    }

    #[test]
    fn test_browser_search() {
        let mut browser = SessionBrowser::new();
        browser.add_session(make_session("Auth refactor", "/w"));
        browser.add_session(make_session("Database migration", "/w"));
        let results = browser.search("auth");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Auth refactor");
    }

    #[test]
    fn test_browser_recent() {
        let mut browser = SessionBrowser::new();
        browser.add_session(make_session("s1", "/w"));
        browser.add_session(make_session("s2", "/w"));
        browser.add_session(make_session("s3", "/w"));
        let recent = browser.recent(2);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn test_browser_archive_old() {
        let mut browser = SessionBrowser::new();
        let mut s = make_session("old", "/w");
        s.complete();
        s.updated_at = SystemTime::UNIX_EPOCH;
        browser.add_session(s);
        browser.add_session(make_session("new", "/w"));
        let archived = browser.archive_old(1);
        assert_eq!(archived, 1);
        assert_eq!(browser.sessions[0].state, SessionState::Archived);
        assert_eq!(browser.sessions[1].state, SessionState::Active);
    }

    #[test]
    fn test_browser_archive_old_skips_active() {
        let mut browser = SessionBrowser::new();
        let mut s = make_session("active old", "/w");
        s.updated_at = SystemTime::UNIX_EPOCH;
        browser.add_session(s);
        let archived = browser.archive_old(1);
        assert_eq!(archived, 0);
    }

    #[test]
    fn test_browser_total_cost() {
        let mut browser = SessionBrowser::new();
        let mut s1 = make_session("s1", "/w");
        s1.cost = 1.50;
        let mut s2 = make_session("s2", "/w");
        s2.cost = 2.25;
        browser.add_session(s1);
        browser.add_session(s2);
        assert!((browser.total_cost() - 3.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_browser_total_tokens() {
        let mut browser = SessionBrowser::new();
        let mut s1 = make_session("s1", "/w");
        s1.tokens_used = 1000;
        let mut s2 = make_session("s2", "/w");
        s2.tokens_used = 2500;
        browser.add_session(s1);
        browser.add_session(s2);
        assert_eq!(browser.total_tokens(), 3500);
    }

    #[test]
    fn test_browser_stats() {
        let mut browser = SessionBrowser::new();
        browser.add_session(make_session("a1", "/w"));
        browser.add_session(make_session("a2", "/w"));
        let mut s3 = make_session("c1", "/w");
        s3.complete();
        browser.add_session(s3);
        let mut s4 = make_session("ar1", "/w");
        s4.archive();
        browser.add_session(s4);
        let (active, completed, archived) = browser.stats();
        assert_eq!(active, 2);
        assert_eq!(completed, 1);
        assert_eq!(archived, 1);
    }

    #[test]
    fn test_filter_combined() {
        let mut s = make_session("Auth fix", "/project");
        s.add_tag("bugfix");
        let filter = SessionFilter::new()
            .with_state(SessionState::Active)
            .with_type(SessionType::Chat)
            .with_search("auth")
            .with_tag("bugfix")
            .with_workspace("project");
        assert!(s.matches_filter(&filter));
    }

    #[test]
    fn test_config_default() {
        let config = SessionBrowserConfig::default();
        assert_eq!(config.sessions_dir, "~/.vibecli/sessions/");
        assert_eq!(config.max_display, 50);
        assert_eq!(config.auto_archive_days, 30);
        assert!(config.show_cost);
        assert!(config.group_by_workspace);
    }
}
