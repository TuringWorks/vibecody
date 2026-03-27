//! Enterprise agent analytics — user, team, and project metrics with ROI calculation.
//!
//! Gap 17 — Tracks task completions, suggestion acceptance, time saved, costs,
//! and generates trend analysis and exportable reports.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-user usage metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserMetrics {
    pub user_id: String,
    pub tasks_completed: u64,
    pub suggestions_accepted: u64,
    pub suggestions_rejected: u64,
    pub time_saved_mins: f64,
    pub cost: f64,
}

impl UserMetrics {
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            tasks_completed: 0,
            suggestions_accepted: 0,
            suggestions_rejected: 0,
            time_saved_mins: 0.0,
            cost: 0.0,
        }
    }

    pub fn acceptance_rate(&self) -> f64 {
        let total = self.suggestions_accepted + self.suggestions_rejected;
        if total == 0 { 0.0 } else { self.suggestions_accepted as f64 / total as f64 }
    }
}

/// Aggregated team metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamMetrics {
    pub team_id: String,
    pub members: Vec<String>,
    pub total_tasks: u64,
    pub total_accepted: u64,
    pub total_rejected: u64,
    pub total_time_saved_mins: f64,
    pub total_cost: f64,
}

impl TeamMetrics {
    pub fn new(team_id: &str) -> Self {
        Self {
            team_id: team_id.to_string(),
            members: Vec::new(),
            total_tasks: 0,
            total_accepted: 0,
            total_rejected: 0,
            total_time_saved_mins: 0.0,
            total_cost: 0.0,
        }
    }

    pub fn add_member(&mut self, user_id: &str) {
        if !self.members.contains(&user_id.to_string()) {
            self.members.push(user_id.to_string());
        }
    }

    pub fn aggregate_from(&mut self, user: &UserMetrics) {
        self.total_tasks += user.tasks_completed;
        self.total_accepted += user.suggestions_accepted;
        self.total_rejected += user.suggestions_rejected;
        self.total_time_saved_mins += user.time_saved_mins;
        self.total_cost += user.cost;
    }
}

/// Per-project metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectMetrics {
    pub project_id: String,
    pub tasks_completed: u64,
    pub lines_generated: u64,
    pub bugs_found: u64,
    pub time_saved_mins: f64,
    pub cost: f64,
}

impl ProjectMetrics {
    pub fn new(project_id: &str) -> Self {
        Self {
            project_id: project_id.to_string(),
            tasks_completed: 0,
            lines_generated: 0,
            bugs_found: 0,
            time_saved_mins: 0.0,
            cost: 0.0,
        }
    }
}

/// Report output format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReportFormat {
    Csv,
    Json,
}

/// Generated analytics report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsReport {
    pub format: ReportFormat,
    pub data: String,
    pub generated_at: u64,
}

/// A single data point in a trend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrendPoint {
    pub timestamp: u64,
    pub value: f64,
}

/// Direction of a trend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrendDirection {
    Up,
    Down,
    Flat,
}

/// Trend analysis over a series of data points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub data_points: Vec<TrendPoint>,
    pub trend_direction: TrendDirection,
    pub percent_change: f64,
}

impl TrendAnalysis {
    pub fn from_points(points: Vec<TrendPoint>) -> Self {
        let (direction, pct) = if points.len() < 2 {
            (TrendDirection::Flat, 0.0)
        } else {
            let first = points.first().expect("has points").value;
            let last = points.last().expect("has points").value;
            let pct = if first.abs() < f64::EPSILON {
                if last > 0.0 { 100.0 } else { 0.0 }
            } else {
                ((last - first) / first) * 100.0
            };
            let dir = if pct > 1.0 {
                TrendDirection::Up
            } else if pct < -1.0 {
                TrendDirection::Down
            } else {
                TrendDirection::Flat
            };
            (dir, pct)
        };
        Self {
            data_points: points,
            trend_direction: direction,
            percent_change: pct,
        }
    }
}

/// ROI calculator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoiCalculator {
    pub hourly_rate: f64,
    pub agent_cost: f64,
}

impl RoiCalculator {
    pub fn new(hourly_rate: f64, agent_cost: f64) -> Self {
        Self { hourly_rate, agent_cost }
    }

    /// Compute ROI given time saved (minutes) and agent cost.
    pub fn compute_roi(&self, time_saved_mins: f64) -> f64 {
        let value = (time_saved_mins / 60.0) * self.hourly_rate;
        if self.agent_cost == 0.0 {
            return value;
        }
        ((value - self.agent_cost) / self.agent_cost) * 100.0
    }

    pub fn net_savings(&self, time_saved_mins: f64) -> f64 {
        let value = (time_saved_mins / 60.0) * self.hourly_rate;
        value - self.agent_cost
    }
}

/// Configuration for the analytics engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    pub retention_days: u32,
    pub default_hourly_rate: f64,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            retention_days: 90,
            default_hourly_rate: 75.0,
        }
    }
}

/// Core analytics engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEngine {
    pub users: HashMap<String, UserMetrics>,
    pub teams: HashMap<String, TeamMetrics>,
    pub projects: HashMap<String, ProjectMetrics>,
    pub config: AnalyticsConfig,
    pub date_range: (u64, u64),
    task_log: Vec<TaskRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TaskRecord {
    user_id: String,
    project_id: Option<String>,
    time_saved_mins: f64,
    cost: f64,
    timestamp: u64,
}

impl AnalyticsEngine {
    pub fn new(config: AnalyticsConfig) -> Self {
        Self {
            users: HashMap::new(),
            teams: HashMap::new(),
            projects: HashMap::new(),
            config,
            date_range: (0, u64::MAX),
            task_log: Vec::new(),
        }
    }

    pub fn set_date_range(&mut self, start: u64, end: u64) {
        self.date_range = (start, end);
    }

    /// Record a completed task for a user.
    pub fn record_task(
        &mut self,
        user_id: &str,
        project_id: Option<&str>,
        time_saved_mins: f64,
        cost: f64,
        timestamp: u64,
    ) {
        let user = self.users.entry(user_id.to_string())
            .or_insert_with(|| UserMetrics::new(user_id));
        user.tasks_completed += 1;
        user.time_saved_mins += time_saved_mins;
        user.cost += cost;

        if let Some(pid) = project_id {
            let proj = self.projects.entry(pid.to_string())
                .or_insert_with(|| ProjectMetrics::new(pid));
            proj.tasks_completed += 1;
            proj.time_saved_mins += time_saved_mins;
            proj.cost += cost;
        }

        self.task_log.push(TaskRecord {
            user_id: user_id.to_string(),
            project_id: project_id.map(|s| s.to_string()),
            time_saved_mins,
            cost,
            timestamp,
        });
    }

    /// Record a suggestion acceptance or rejection.
    pub fn record_suggestion(&mut self, user_id: &str, accepted: bool) {
        let user = self.users.entry(user_id.to_string())
            .or_insert_with(|| UserMetrics::new(user_id));
        if accepted {
            user.suggestions_accepted += 1;
        } else {
            user.suggestions_rejected += 1;
        }
    }

    /// Calculate ROI for a user.
    pub fn calculate_roi(&self, user_id: &str) -> Result<f64, String> {
        let user = self.users.get(user_id)
            .ok_or_else(|| format!("User {} not found", user_id))?;
        let calc = RoiCalculator::new(self.config.default_hourly_rate, user.cost);
        Ok(calc.compute_roi(user.time_saved_mins))
    }

    /// Generate a report in the given format.
    pub fn generate_report(&self, format: ReportFormat) -> AnalyticsReport {
        let data = match format {
            ReportFormat::Json => {
                let mut entries = Vec::new();
                for (id, u) in &self.users {
                    entries.push(format!(
                        r#"{{"user":"{}","tasks":{},"accepted":{},"rejected":{},"time_saved":{:.1},"cost":{:.2}}}"#,
                        id, u.tasks_completed, u.suggestions_accepted, u.suggestions_rejected,
                        u.time_saved_mins, u.cost
                    ));
                }
                format!("[{}]", entries.join(","))
            }
            ReportFormat::Csv => {
                let mut lines = vec!["user,tasks,accepted,rejected,time_saved,cost".to_string()];
                for (id, u) in &self.users {
                    lines.push(format!(
                        "{},{},{},{},{:.1},{:.2}",
                        id, u.tasks_completed, u.suggestions_accepted, u.suggestions_rejected,
                        u.time_saved_mins, u.cost
                    ));
                }
                lines.join("\n")
            }
        };
        AnalyticsReport {
            format,
            data,
            generated_at: 0,
        }
    }

    /// Get trends from the task log.
    pub fn get_trends(&self, user_id: Option<&str>) -> TrendAnalysis {
        let filtered: Vec<&TaskRecord> = self.task_log.iter()
            .filter(|r| {
                r.timestamp >= self.date_range.0
                    && r.timestamp <= self.date_range.1
                    && user_id.map_or(true, |u| r.user_id == u)
            })
            .collect();

        let points: Vec<TrendPoint> = filtered.iter()
            .map(|r| TrendPoint {
                timestamp: r.timestamp,
                value: r.time_saved_mins,
            })
            .collect();

        TrendAnalysis::from_points(points)
    }

    /// Export all data as JSON.
    pub fn export(&self) -> String {
        serde_json::to_string_pretty(&self.users).unwrap_or_default()
    }

    /// Create or get a team and assign a user.
    pub fn assign_user_to_team(&mut self, user_id: &str, team_id: &str) {
        let team = self.teams.entry(team_id.to_string())
            .or_insert_with(|| TeamMetrics::new(team_id));
        team.add_member(user_id);
    }

    /// Aggregate user metrics into team.
    pub fn refresh_team(&mut self, team_id: &str) -> Result<(), String> {
        let team = self.teams.get(team_id)
            .ok_or_else(|| format!("Team {} not found", team_id))?;
        let member_ids = team.members.clone();
        let mut agg = TeamMetrics::new(team_id);
        agg.members = member_ids.clone();
        for uid in &member_ids {
            if let Some(u) = self.users.get(uid) {
                agg.aggregate_from(u);
            }
        }
        self.teams.insert(team_id.to_string(), agg);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> AnalyticsEngine {
        AnalyticsEngine::new(AnalyticsConfig::default())
    }

    #[test]
    fn test_user_metrics_new() {
        let u = UserMetrics::new("alice");
        assert_eq!(u.user_id, "alice");
        assert_eq!(u.tasks_completed, 0);
    }

    #[test]
    fn test_user_acceptance_rate() {
        let mut u = UserMetrics::new("a");
        u.suggestions_accepted = 3;
        u.suggestions_rejected = 1;
        assert!((u.acceptance_rate() - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_user_acceptance_rate_zero() {
        let u = UserMetrics::new("a");
        assert_eq!(u.acceptance_rate(), 0.0);
    }

    #[test]
    fn test_team_metrics_new() {
        let t = TeamMetrics::new("eng");
        assert!(t.members.is_empty());
        assert_eq!(t.total_tasks, 0);
    }

    #[test]
    fn test_team_add_member() {
        let mut t = TeamMetrics::new("eng");
        t.add_member("alice");
        t.add_member("alice"); // duplicate
        assert_eq!(t.members.len(), 1);
    }

    #[test]
    fn test_team_aggregate_from() {
        let mut t = TeamMetrics::new("eng");
        let mut u = UserMetrics::new("a");
        u.tasks_completed = 5;
        u.time_saved_mins = 30.0;
        u.cost = 10.0;
        t.aggregate_from(&u);
        assert_eq!(t.total_tasks, 5);
        assert_eq!(t.total_time_saved_mins, 30.0);
    }

    #[test]
    fn test_project_metrics_new() {
        let p = ProjectMetrics::new("proj1");
        assert_eq!(p.project_id, "proj1");
        assert_eq!(p.tasks_completed, 0);
    }

    #[test]
    fn test_roi_calculator() {
        let calc = RoiCalculator::new(100.0, 50.0);
        // 60 min saved = $100 value; cost $50; ROI = ((100-50)/50)*100 = 100%
        let roi = calc.compute_roi(60.0);
        assert!((roi - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_roi_calculator_zero_cost() {
        let calc = RoiCalculator::new(100.0, 0.0);
        let roi = calc.compute_roi(60.0);
        assert!((roi - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_roi_net_savings() {
        let calc = RoiCalculator::new(120.0, 30.0);
        // 30 min = 0.5hr * 120 = 60; net = 60 - 30 = 30
        assert!((calc.net_savings(30.0) - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_engine_new() {
        let e = engine();
        assert!(e.users.is_empty());
        assert!(e.teams.is_empty());
    }

    #[test]
    fn test_record_task() {
        let mut e = engine();
        e.record_task("alice", Some("proj1"), 10.0, 1.0, 100);
        assert_eq!(e.users["alice"].tasks_completed, 1);
        assert_eq!(e.projects["proj1"].tasks_completed, 1);
    }

    #[test]
    fn test_record_task_no_project() {
        let mut e = engine();
        e.record_task("alice", None, 5.0, 0.5, 200);
        assert_eq!(e.users["alice"].tasks_completed, 1);
        assert!(e.projects.is_empty());
    }

    #[test]
    fn test_record_task_accumulates() {
        let mut e = engine();
        e.record_task("alice", None, 10.0, 1.0, 100);
        e.record_task("alice", None, 20.0, 2.0, 200);
        assert_eq!(e.users["alice"].tasks_completed, 2);
        assert!((e.users["alice"].time_saved_mins - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_record_suggestion_accepted() {
        let mut e = engine();
        e.record_suggestion("alice", true);
        assert_eq!(e.users["alice"].suggestions_accepted, 1);
    }

    #[test]
    fn test_record_suggestion_rejected() {
        let mut e = engine();
        e.record_suggestion("alice", false);
        assert_eq!(e.users["alice"].suggestions_rejected, 1);
    }

    #[test]
    fn test_calculate_roi() {
        let mut e = engine();
        e.record_task("alice", None, 60.0, 25.0, 100);
        let roi = e.calculate_roi("alice").unwrap();
        // value = 75 (60min at $75/hr), cost = 25, roi = (75-25)/25 * 100 = 200%
        assert!((roi - 200.0).abs() < 0.1);
    }

    #[test]
    fn test_calculate_roi_not_found() {
        let e = engine();
        assert!(e.calculate_roi("nobody").is_err());
    }

    #[test]
    fn test_generate_report_json() {
        let mut e = engine();
        e.record_task("alice", None, 10.0, 1.0, 100);
        let report = e.generate_report(ReportFormat::Json);
        assert_eq!(report.format, ReportFormat::Json);
        assert!(report.data.contains("alice"));
    }

    #[test]
    fn test_generate_report_csv() {
        let mut e = engine();
        e.record_task("alice", None, 10.0, 1.0, 100);
        let report = e.generate_report(ReportFormat::Csv);
        assert!(report.data.contains("user,tasks"));
        assert!(report.data.contains("alice"));
    }

    #[test]
    fn test_get_trends_empty() {
        let e = engine();
        let trend = e.get_trends(None);
        assert_eq!(trend.trend_direction, TrendDirection::Flat);
        assert!(trend.data_points.is_empty());
    }

    #[test]
    fn test_get_trends_up() {
        let mut e = engine();
        e.record_task("a", None, 10.0, 1.0, 100);
        e.record_task("a", None, 50.0, 1.0, 200);
        let trend = e.get_trends(Some("a"));
        assert_eq!(trend.trend_direction, TrendDirection::Up);
        assert!(trend.percent_change > 0.0);
    }

    #[test]
    fn test_get_trends_down() {
        let mut e = engine();
        e.record_task("a", None, 50.0, 1.0, 100);
        e.record_task("a", None, 10.0, 1.0, 200);
        let trend = e.get_trends(Some("a"));
        assert_eq!(trend.trend_direction, TrendDirection::Down);
    }

    #[test]
    fn test_get_trends_date_filter() {
        let mut e = engine();
        e.record_task("a", None, 10.0, 1.0, 100);
        e.record_task("a", None, 20.0, 1.0, 300);
        e.set_date_range(200, 400);
        let trend = e.get_trends(Some("a"));
        assert_eq!(trend.data_points.len(), 1);
    }

    #[test]
    fn test_export() {
        let mut e = engine();
        e.record_task("alice", None, 10.0, 1.0, 100);
        let json = e.export();
        assert!(json.contains("alice"));
    }

    #[test]
    fn test_assign_user_to_team() {
        let mut e = engine();
        e.assign_user_to_team("alice", "eng");
        assert_eq!(e.teams["eng"].members, vec!["alice"]);
    }

    #[test]
    fn test_refresh_team() {
        let mut e = engine();
        e.assign_user_to_team("alice", "eng");
        e.record_task("alice", None, 30.0, 5.0, 100);
        e.refresh_team("eng").unwrap();
        assert_eq!(e.teams["eng"].total_tasks, 1);
        assert!((e.teams["eng"].total_time_saved_mins - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_refresh_team_not_found() {
        let mut e = engine();
        assert!(e.refresh_team("nope").is_err());
    }

    #[test]
    fn test_trend_analysis_single_point() {
        let trend = TrendAnalysis::from_points(vec![TrendPoint { timestamp: 1, value: 10.0 }]);
        assert_eq!(trend.trend_direction, TrendDirection::Flat);
        assert_eq!(trend.percent_change, 0.0);
    }

    #[test]
    fn test_trend_analysis_flat() {
        let trend = TrendAnalysis::from_points(vec![
            TrendPoint { timestamp: 1, value: 10.0 },
            TrendPoint { timestamp: 2, value: 10.05 },
        ]);
        assert_eq!(trend.trend_direction, TrendDirection::Flat);
    }

    #[test]
    fn test_analytics_config_default() {
        let cfg = AnalyticsConfig::default();
        assert_eq!(cfg.retention_days, 90);
        assert_eq!(cfg.default_hourly_rate, 75.0);
    }

    #[test]
    fn test_report_format_serde() {
        let f = ReportFormat::Csv;
        let json = serde_json::to_string(&f).unwrap();
        let de: ReportFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(f, de);
    }

    #[test]
    fn test_multiple_users_report() {
        let mut e = engine();
        e.record_task("alice", None, 10.0, 1.0, 100);
        e.record_task("bob", None, 20.0, 2.0, 200);
        let report = e.generate_report(ReportFormat::Csv);
        assert!(report.data.contains("alice"));
        assert!(report.data.contains("bob"));
    }

    #[test]
    fn test_project_accumulates() {
        let mut e = engine();
        e.record_task("a", Some("p"), 10.0, 1.0, 100);
        e.record_task("b", Some("p"), 20.0, 2.0, 200);
        assert_eq!(e.projects["p"].tasks_completed, 2);
        assert!((e.projects["p"].cost - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_user_metrics_serde() {
        let u = UserMetrics::new("alice");
        let json = serde_json::to_string(&u).unwrap();
        let de: UserMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(u, de);
    }

    #[test]
    fn test_get_trends_filtered_by_user() {
        let mut e = engine();
        e.record_task("a", None, 10.0, 1.0, 100);
        e.record_task("b", None, 20.0, 1.0, 200);
        let trend = e.get_trends(Some("a"));
        assert_eq!(trend.data_points.len(), 1);
    }
}
