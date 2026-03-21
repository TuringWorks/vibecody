#![allow(dead_code)]
//! Cross-surface task routing for VibeCody.
//!
//! Routes agent tasks between CLI, IDE, cloud, and mobile surfaces based on
//! capability matching, surface availability, and priority scoring.
//!
//! REPL commands: `/surface register|list|route|status|complete|stats`

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceType {
    Cli,
    DesktopIde,
    CloudVm,
    Mobile,
    Browser,
    WebSocket,
}

impl std::fmt::Display for SurfaceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cli => write!(f, "cli"),
            Self::DesktopIde => write!(f, "desktop_ide"),
            Self::CloudVm => write!(f, "cloud_vm"),
            Self::Mobile => write!(f, "mobile"),
            Self::Browser => write!(f, "browser"),
            Self::WebSocket => write!(f, "websocket"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceCapability {
    FileEdit,
    TerminalExec,
    BrowserAutomation,
    VoiceInput,
    VoiceOutput,
    ScreenCapture,
    GitOperations,
    BuildRun,
    Deploy,
    Preview,
}

impl std::fmt::Display for SurfaceCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileEdit => write!(f, "file_edit"),
            Self::TerminalExec => write!(f, "terminal_exec"),
            Self::BrowserAutomation => write!(f, "browser_automation"),
            Self::VoiceInput => write!(f, "voice_input"),
            Self::VoiceOutput => write!(f, "voice_output"),
            Self::ScreenCapture => write!(f, "screen_capture"),
            Self::GitOperations => write!(f, "git_operations"),
            Self::BuildRun => write!(f, "build_run"),
            Self::Deploy => write!(f, "deploy"),
            Self::Preview => write!(f, "preview"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceStatus {
    Online,
    Offline,
    Busy,
}

impl std::fmt::Display for SurfaceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Online => write!(f, "online"),
            Self::Offline => write!(f, "offline"),
            Self::Busy => write!(f, "busy"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RouteStatus {
    Pending,
    InTransit,
    Delivered,
    Working,
    Completed,
    Failed,
}

impl std::fmt::Display for RouteStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InTransit => write!(f, "in_transit"),
            Self::Delivered => write!(f, "delivered"),
            Self::Working => write!(f, "working"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoutePriority {
    Low,
    Normal,
    High,
    Urgent,
}

impl RoutePriority {
    fn weight(&self) -> f32 {
        match self {
            Self::Low => 0.25,
            Self::Normal => 0.5,
            Self::High => 0.75,
            Self::Urgent => 1.0,
        }
    }
}

impl std::fmt::Display for RoutePriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Urgent => write!(f, "urgent"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoutingError {
    NoSuitableSurface,
    SurfaceOffline(String),
    RouteFull,
    RouteNotFound(String),
    SurfaceNotFound(String),
    CapabilityMismatch(String),
    DuplicateSurface(String),
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSuitableSurface => write!(f, "no suitable surface found"),
            Self::SurfaceOffline(id) => write!(f, "surface offline: {id}"),
            Self::RouteFull => write!(f, "max active routes reached"),
            Self::RouteNotFound(id) => write!(f, "route not found: {id}"),
            Self::SurfaceNotFound(id) => write!(f, "surface not found: {id}"),
            Self::CapabilityMismatch(msg) => write!(f, "capability mismatch: {msg}"),
            Self::DuplicateSurface(id) => write!(f, "duplicate surface: {id}"),
        }
    }
}

// === Config ===

#[derive(Debug, Clone)]
pub struct RoutingConfig {
    pub max_active_routes: usize,
    pub sync_interval_secs: u64,
    pub auto_route: bool,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            max_active_routes: 20,
            sync_interval_secs: 10,
            auto_route: true,
        }
    }
}

// === Data Structures ===

#[derive(Debug, Clone)]
pub struct Surface {
    pub id: String,
    pub surface_type: SurfaceType,
    pub capabilities: Vec<SurfaceCapability>,
    pub status: SurfaceStatus,
    pub endpoint: String,
    pub last_seen: u64,
}

#[derive(Debug, Clone)]
pub struct TaskRoute {
    pub id: String,
    pub task_description: String,
    pub source_surface: String,
    pub target_surface: String,
    pub status: RouteStatus,
    pub created_at: u64,
    pub delivered_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub result: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HandoffRequest {
    pub task: String,
    pub required_capabilities: Vec<SurfaceCapability>,
    pub preferred_surface: Option<SurfaceType>,
    pub priority: RoutePriority,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub target_surface_id: String,
    pub reason: String,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct SyncState {
    pub route_id: String,
    pub progress_percent: f32,
    pub last_update: String,
    pub files_changed: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RoutingStats {
    pub total_routes: usize,
    pub active_routes: usize,
    pub completed_routes: usize,
    pub failed_routes: usize,
    pub avg_duration_secs: f64,
}

// === Main Struct ===

pub struct SurfaceRouter {
    config: RoutingConfig,
    surfaces: HashMap<String, Surface>,
    routes: Vec<TaskRoute>,
    route_counter: usize,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl SurfaceRouter {
    pub fn new(config: RoutingConfig) -> Self {
        Self {
            config,
            surfaces: HashMap::new(),
            routes: Vec::new(),
            route_counter: 0,
        }
    }

    /// Register a new surface for routing.
    pub fn register_surface(&mut self, surface: Surface) -> Result<(), RoutingError> {
        if self.surfaces.contains_key(&surface.id) {
            return Err(RoutingError::DuplicateSurface(surface.id.clone()));
        }
        self.surfaces.insert(surface.id.clone(), surface);
        Ok(())
    }

    /// Unregister a surface by ID.
    pub fn unregister_surface(&mut self, id: &str) -> Result<(), RoutingError> {
        self.surfaces
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| RoutingError::SurfaceNotFound(id.to_string()))
    }

    /// Get a surface by ID.
    pub fn get_surface(&self, id: &str) -> Option<&Surface> {
        self.surfaces.get(id)
    }

    /// List all registered surfaces.
    pub fn list_surfaces(&self) -> Vec<&Surface> {
        self.surfaces.values().collect()
    }

    /// List only online surfaces.
    pub fn list_online_surfaces(&self) -> Vec<&Surface> {
        self.surfaces
            .values()
            .filter(|s| s.status == SurfaceStatus::Online)
            .collect()
    }

    /// Route a task to the best available surface. Returns the route ID.
    pub fn route_task(&mut self, request: HandoffRequest) -> Result<String, RoutingError> {
        let active = self
            .routes
            .iter()
            .filter(|r| {
                matches!(
                    r.status,
                    RouteStatus::Pending | RouteStatus::InTransit | RouteStatus::Working
                )
            })
            .count();

        if active >= self.config.max_active_routes {
            return Err(RoutingError::RouteFull);
        }

        let decision = self.find_best_surface(&request)?;
        self.route_counter += 1;
        let route_id = format!("route-{}", self.route_counter);
        let now = now_secs();

        let route = TaskRoute {
            id: route_id.clone(),
            task_description: request.task,
            source_surface: "local".to_string(),
            target_surface: decision.target_surface_id,
            status: RouteStatus::Pending,
            created_at: now,
            delivered_at: None,
            completed_at: None,
            result: None,
        };

        self.routes.push(route);
        Ok(route_id)
    }

    /// Find the best surface for a handoff request using capability matching and scoring.
    pub fn find_best_surface(
        &self,
        request: &HandoffRequest,
    ) -> Result<RoutingDecision, RoutingError> {
        let mut best: Option<(&Surface, f32)> = None;

        for surface in self.surfaces.values() {
            if surface.status == SurfaceStatus::Offline {
                continue;
            }

            // Check all required capabilities are present.
            let has_all = request
                .required_capabilities
                .iter()
                .all(|cap| surface.capabilities.contains(cap));
            if !has_all {
                continue;
            }

            let score = self.score_surface(surface, request);
            if best.is_none() || score > best.unwrap().1 {
                best = Some((surface, score));
            }
        }

        match best {
            Some((surface, score)) => {
                let reason = format!(
                    "Selected {} ({}) with score {:.2}",
                    surface.id, surface.surface_type, score
                );
                Ok(RoutingDecision {
                    target_surface_id: surface.id.clone(),
                    reason,
                    score,
                })
            }
            None => Err(RoutingError::NoSuitableSurface),
        }
    }

    /// Score a surface for a given request (0.0 - 1.0).
    pub fn score_surface(&self, surface: &Surface, request: &HandoffRequest) -> f32 {
        let mut score: f32 = 0.0;

        // Capability match ratio (40% weight).
        if !request.required_capabilities.is_empty() {
            let matched = request
                .required_capabilities
                .iter()
                .filter(|c| surface.capabilities.contains(c))
                .count();
            score += 0.4 * (matched as f32 / request.required_capabilities.len() as f32);
        } else {
            score += 0.4;
        }

        // Status bonus (30% weight): online > busy.
        score += match surface.status {
            SurfaceStatus::Online => 0.3,
            SurfaceStatus::Busy => 0.1,
            SurfaceStatus::Offline => 0.0,
        };

        // Preferred surface type match (20% weight).
        if let Some(preferred) = &request.preferred_surface {
            if &surface.surface_type == preferred {
                score += 0.2;
            }
        } else {
            score += 0.1;
        }

        // Extra capabilities bonus (10% weight).
        let extra = surface.capabilities.len() as f32 / 10.0;
        score += 0.1 * extra.min(1.0);

        score.min(1.0)
    }

    /// Get a route by ID.
    pub fn get_route(&self, id: &str) -> Option<&TaskRoute> {
        self.routes.iter().find(|r| r.id == id)
    }

    /// List all routes.
    pub fn list_routes(&self) -> Vec<&TaskRoute> {
        self.routes.iter().collect()
    }

    /// Mark a route as completed with a result.
    pub fn complete_route(&mut self, id: &str, result: &str) -> Result<(), RoutingError> {
        let route = self
            .routes
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| RoutingError::RouteNotFound(id.to_string()))?;

        route.status = RouteStatus::Completed;
        route.completed_at = Some(now_secs());
        route.result = Some(result.to_string());
        Ok(())
    }

    /// Mark a route as failed with an error message.
    pub fn fail_route(&mut self, id: &str, error: &str) -> Result<(), RoutingError> {
        let route = self
            .routes
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| RoutingError::RouteNotFound(id.to_string()))?;

        route.status = RouteStatus::Failed;
        route.completed_at = Some(now_secs());
        route.result = Some(format!("ERROR: {error}"));
        Ok(())
    }

    /// Update a surface's status.
    pub fn update_surface_status(
        &mut self,
        id: &str,
        status: SurfaceStatus,
    ) -> Result<(), RoutingError> {
        let surface = self
            .surfaces
            .get_mut(id)
            .ok_or_else(|| RoutingError::SurfaceNotFound(id.to_string()))?;

        surface.status = status;
        surface.last_seen = now_secs();
        Ok(())
    }

    /// Get the sync state for a route (simulated progress).
    pub fn get_sync_state(&self, route_id: &str) -> Option<SyncState> {
        let route = self.routes.iter().find(|r| r.id == route_id)?;

        let progress = match &route.status {
            RouteStatus::Pending => 0.0,
            RouteStatus::InTransit => 25.0,
            RouteStatus::Delivered => 50.0,
            RouteStatus::Working => 75.0,
            RouteStatus::Completed => 100.0,
            RouteStatus::Failed => 0.0,
        };

        Some(SyncState {
            route_id: route_id.to_string(),
            progress_percent: progress,
            last_update: format!("{}s ago", now_secs().saturating_sub(route.created_at)),
            files_changed: Vec::new(),
        })
    }

    /// Get aggregate routing statistics.
    pub fn get_routing_stats(&self) -> RoutingStats {
        let total = self.routes.len();
        let active = self
            .routes
            .iter()
            .filter(|r| {
                matches!(
                    r.status,
                    RouteStatus::Pending | RouteStatus::InTransit | RouteStatus::Working
                )
            })
            .count();
        let completed = self
            .routes
            .iter()
            .filter(|r| r.status == RouteStatus::Completed)
            .count();
        let failed = self
            .routes
            .iter()
            .filter(|r| r.status == RouteStatus::Failed)
            .count();

        let durations: Vec<u64> = self
            .routes
            .iter()
            .filter_map(|r| {
                r.completed_at
                    .map(|end| end.saturating_sub(r.created_at))
            })
            .collect();

        let avg_duration = if durations.is_empty() {
            0.0
        } else {
            durations.iter().sum::<u64>() as f64 / durations.len() as f64
        };

        RoutingStats {
            total_routes: total,
            active_routes: active,
            completed_routes: completed,
            failed_routes: failed,
            avg_duration_secs: avg_duration,
        }
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    fn default_router() -> SurfaceRouter {
        SurfaceRouter::new(RoutingConfig::default())
    }

    fn cli_surface() -> Surface {
        Surface {
            id: "cli-1".to_string(),
            surface_type: SurfaceType::Cli,
            capabilities: vec![
                SurfaceCapability::FileEdit,
                SurfaceCapability::TerminalExec,
                SurfaceCapability::GitOperations,
            ],
            status: SurfaceStatus::Online,
            endpoint: "localhost:7878".to_string(),
            last_seen: now_secs(),
        }
    }

    fn ide_surface() -> Surface {
        Surface {
            id: "ide-1".to_string(),
            surface_type: SurfaceType::DesktopIde,
            capabilities: vec![
                SurfaceCapability::FileEdit,
                SurfaceCapability::TerminalExec,
                SurfaceCapability::GitOperations,
                SurfaceCapability::BuildRun,
                SurfaceCapability::Preview,
            ],
            status: SurfaceStatus::Online,
            endpoint: "localhost:3000".to_string(),
            last_seen: now_secs(),
        }
    }

    fn cloud_surface() -> Surface {
        Surface {
            id: "cloud-1".to_string(),
            surface_type: SurfaceType::CloudVm,
            capabilities: vec![
                SurfaceCapability::FileEdit,
                SurfaceCapability::TerminalExec,
                SurfaceCapability::BuildRun,
                SurfaceCapability::Deploy,
            ],
            status: SurfaceStatus::Online,
            endpoint: "cloud.vibecody.dev".to_string(),
            last_seen: now_secs(),
        }
    }

    #[test]
    fn test_default_config() {
        let cfg = RoutingConfig::default();
        assert_eq!(cfg.max_active_routes, 20);
        assert_eq!(cfg.sync_interval_secs, 10);
        assert!(cfg.auto_route);
    }

    #[test]
    fn test_register_surface() {
        let mut router = default_router();
        assert!(router.register_surface(cli_surface()).is_ok());
        assert_eq!(router.list_surfaces().len(), 1);
    }

    #[test]
    fn test_register_duplicate_surface() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        let result = router.register_surface(cli_surface());
        assert_eq!(result, Err(RoutingError::DuplicateSurface("cli-1".to_string())));
    }

    #[test]
    fn test_unregister_surface() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        assert!(router.unregister_surface("cli-1").is_ok());
        assert_eq!(router.list_surfaces().len(), 0);
    }

    #[test]
    fn test_unregister_nonexistent_surface() {
        let mut router = default_router();
        let result = router.unregister_surface("nope");
        assert_eq!(result, Err(RoutingError::SurfaceNotFound("nope".to_string())));
    }

    #[test]
    fn test_get_surface() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        let s = router.get_surface("cli-1");
        assert!(s.is_some());
        assert_eq!(s.unwrap().surface_type, SurfaceType::Cli);
    }

    #[test]
    fn test_list_online_surfaces() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        let mut offline = ide_surface();
        offline.status = SurfaceStatus::Offline;
        router.register_surface(offline).unwrap();
        assert_eq!(router.list_online_surfaces().len(), 1);
    }

    #[test]
    fn test_route_task_basic() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        let request = HandoffRequest {
            task: "Edit file".to_string(),
            required_capabilities: vec![SurfaceCapability::FileEdit],
            preferred_surface: None,
            priority: RoutePriority::Normal,
            timeout_secs: None,
        };
        let id = router.route_task(request).unwrap();
        assert!(id.starts_with("route-"));
    }

    #[test]
    fn test_route_task_no_suitable_surface() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        let request = HandoffRequest {
            task: "Deploy app".to_string(),
            required_capabilities: vec![SurfaceCapability::Deploy],
            preferred_surface: None,
            priority: RoutePriority::High,
            timeout_secs: None,
        };
        let result = router.route_task(request);
        assert_eq!(result, Err(RoutingError::NoSuitableSurface));
    }

    #[test]
    fn test_route_task_max_routes() {
        let mut router = SurfaceRouter::new(RoutingConfig {
            max_active_routes: 1,
            ..RoutingConfig::default()
        });
        router.register_surface(cli_surface()).unwrap();
        let req = || HandoffRequest {
            task: "task".to_string(),
            required_capabilities: vec![SurfaceCapability::FileEdit],
            preferred_surface: None,
            priority: RoutePriority::Normal,
            timeout_secs: None,
        };
        router.route_task(req()).unwrap();
        let result = router.route_task(req());
        assert_eq!(result, Err(RoutingError::RouteFull));
    }

    #[test]
    fn test_find_best_surface_prefers_more_capabilities() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        router.register_surface(ide_surface()).unwrap();
        let request = HandoffRequest {
            task: "Edit and build".to_string(),
            required_capabilities: vec![SurfaceCapability::FileEdit],
            preferred_surface: None,
            priority: RoutePriority::Normal,
            timeout_secs: None,
        };
        let decision = router.find_best_surface(&request).unwrap();
        // IDE has more capabilities, so higher score.
        assert_eq!(decision.target_surface_id, "ide-1");
    }

    #[test]
    fn test_find_best_surface_preferred_type() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        router.register_surface(cloud_surface()).unwrap();
        let request = HandoffRequest {
            task: "Deploy".to_string(),
            required_capabilities: vec![SurfaceCapability::FileEdit],
            preferred_surface: Some(SurfaceType::Cli),
            priority: RoutePriority::Normal,
            timeout_secs: None,
        };
        let decision = router.find_best_surface(&request).unwrap();
        assert_eq!(decision.target_surface_id, "cli-1");
    }

    #[test]
    fn test_score_surface_online_vs_busy() {
        let router = default_router();
        let mut busy = cli_surface();
        busy.status = SurfaceStatus::Busy;
        let request = HandoffRequest {
            task: "task".to_string(),
            required_capabilities: vec![SurfaceCapability::FileEdit],
            preferred_surface: None,
            priority: RoutePriority::Normal,
            timeout_secs: None,
        };
        let online_score = router.score_surface(&cli_surface(), &request);
        let busy_score = router.score_surface(&busy, &request);
        assert!(online_score > busy_score);
    }

    #[test]
    fn test_score_surface_offline_zero_status() {
        let router = default_router();
        let mut offline = cli_surface();
        offline.status = SurfaceStatus::Offline;
        let request = HandoffRequest {
            task: "task".to_string(),
            required_capabilities: vec![SurfaceCapability::FileEdit],
            preferred_surface: None,
            priority: RoutePriority::Normal,
            timeout_secs: None,
        };
        let score = router.score_surface(&offline, &request);
        // Offline gets 0.0 for status weight.
        assert!(score < 0.7);
    }

    #[test]
    fn test_get_route() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        let request = HandoffRequest {
            task: "Edit".to_string(),
            required_capabilities: vec![SurfaceCapability::FileEdit],
            preferred_surface: None,
            priority: RoutePriority::Normal,
            timeout_secs: None,
        };
        let id = router.route_task(request).unwrap();
        let route = router.get_route(&id);
        assert!(route.is_some());
        assert_eq!(route.unwrap().status, RouteStatus::Pending);
    }

    #[test]
    fn test_complete_route() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        let request = HandoffRequest {
            task: "Edit".to_string(),
            required_capabilities: vec![SurfaceCapability::FileEdit],
            preferred_surface: None,
            priority: RoutePriority::Normal,
            timeout_secs: None,
        };
        let id = router.route_task(request).unwrap();
        router.complete_route(&id, "done").unwrap();
        let route = router.get_route(&id).unwrap();
        assert_eq!(route.status, RouteStatus::Completed);
        assert_eq!(route.result, Some("done".to_string()));
    }

    #[test]
    fn test_fail_route() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        let request = HandoffRequest {
            task: "Edit".to_string(),
            required_capabilities: vec![SurfaceCapability::FileEdit],
            preferred_surface: None,
            priority: RoutePriority::Normal,
            timeout_secs: None,
        };
        let id = router.route_task(request).unwrap();
        router.fail_route(&id, "timeout").unwrap();
        let route = router.get_route(&id).unwrap();
        assert_eq!(route.status, RouteStatus::Failed);
        assert!(route.result.as_ref().unwrap().contains("ERROR"));
    }

    #[test]
    fn test_complete_route_not_found() {
        let mut router = default_router();
        let result = router.complete_route("nope", "done");
        assert_eq!(result, Err(RoutingError::RouteNotFound("nope".to_string())));
    }

    #[test]
    fn test_fail_route_not_found() {
        let mut router = default_router();
        let result = router.fail_route("nope", "err");
        assert_eq!(result, Err(RoutingError::RouteNotFound("nope".to_string())));
    }

    #[test]
    fn test_update_surface_status() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        router
            .update_surface_status("cli-1", SurfaceStatus::Busy)
            .unwrap();
        let s = router.get_surface("cli-1").unwrap();
        assert_eq!(s.status, SurfaceStatus::Busy);
    }

    #[test]
    fn test_update_surface_status_not_found() {
        let mut router = default_router();
        let result = router.update_surface_status("nope", SurfaceStatus::Online);
        assert_eq!(result, Err(RoutingError::SurfaceNotFound("nope".to_string())));
    }

    #[test]
    fn test_get_sync_state_pending() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        let request = HandoffRequest {
            task: "Edit".to_string(),
            required_capabilities: vec![SurfaceCapability::FileEdit],
            preferred_surface: None,
            priority: RoutePriority::Normal,
            timeout_secs: None,
        };
        let id = router.route_task(request).unwrap();
        let state = router.get_sync_state(&id).unwrap();
        assert_eq!(state.progress_percent, 0.0);
    }

    #[test]
    fn test_get_sync_state_completed() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        let request = HandoffRequest {
            task: "Edit".to_string(),
            required_capabilities: vec![SurfaceCapability::FileEdit],
            preferred_surface: None,
            priority: RoutePriority::Normal,
            timeout_secs: None,
        };
        let id = router.route_task(request).unwrap();
        router.complete_route(&id, "ok").unwrap();
        let state = router.get_sync_state(&id).unwrap();
        assert_eq!(state.progress_percent, 100.0);
    }

    #[test]
    fn test_get_sync_state_nonexistent() {
        let router = default_router();
        assert!(router.get_sync_state("nope").is_none());
    }

    #[test]
    fn test_routing_stats_empty() {
        let router = default_router();
        let stats = router.get_routing_stats();
        assert_eq!(stats.total_routes, 0);
        assert_eq!(stats.avg_duration_secs, 0.0);
    }

    #[test]
    fn test_routing_stats_with_routes() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        let req = || HandoffRequest {
            task: "task".to_string(),
            required_capabilities: vec![SurfaceCapability::FileEdit],
            preferred_surface: None,
            priority: RoutePriority::Normal,
            timeout_secs: None,
        };
        let id1 = router.route_task(req()).unwrap();
        let id2 = router.route_task(req()).unwrap();
        router.complete_route(&id1, "ok").unwrap();
        router.fail_route(&id2, "err").unwrap();
        let stats = router.get_routing_stats();
        assert_eq!(stats.total_routes, 2);
        assert_eq!(stats.completed_routes, 1);
        assert_eq!(stats.failed_routes, 1);
        assert_eq!(stats.active_routes, 0);
    }

    #[test]
    fn test_surface_type_display() {
        assert_eq!(format!("{}", SurfaceType::Cli), "cli");
        assert_eq!(format!("{}", SurfaceType::CloudVm), "cloud_vm");
    }

    #[test]
    fn test_surface_capability_display() {
        assert_eq!(format!("{}", SurfaceCapability::FileEdit), "file_edit");
        assert_eq!(format!("{}", SurfaceCapability::Deploy), "deploy");
    }

    #[test]
    fn test_route_status_display() {
        assert_eq!(format!("{}", RouteStatus::Pending), "pending");
        assert_eq!(format!("{}", RouteStatus::InTransit), "in_transit");
    }

    #[test]
    fn test_routing_error_display() {
        assert_eq!(format!("{}", RoutingError::RouteFull), "max active routes reached");
        assert_eq!(
            format!("{}", RoutingError::SurfaceOffline("x".to_string())),
            "surface offline: x"
        );
    }

    #[test]
    fn test_route_priority_weight() {
        assert!(RoutePriority::Urgent.weight() > RoutePriority::Low.weight());
        assert_eq!(RoutePriority::Normal.weight(), 0.5);
    }

    #[test]
    fn test_list_routes() {
        let mut router = default_router();
        router.register_surface(cli_surface()).unwrap();
        let req = HandoffRequest {
            task: "task".to_string(),
            required_capabilities: vec![SurfaceCapability::FileEdit],
            preferred_surface: None,
            priority: RoutePriority::Normal,
            timeout_secs: None,
        };
        router.route_task(req).unwrap();
        assert_eq!(router.list_routes().len(), 1);
    }
}
