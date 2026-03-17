
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Status of a cloud IDE instance lifecycle.
#[derive(Debug, Clone, PartialEq)]
pub enum CloudIdeStatus {
    Provisioning,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed(String),
    Hibernating,
}

/// Resource tier for cloud IDE instances.
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceTier {
    Micro,
    Small,
    Medium,
    Large,
    XLarge,
    Custom {
        cpu_cores: u32,
        memory_gb: u32,
        disk_gb: u32,
    },
}

impl ResourceTier {
    pub fn cpu_cores(&self) -> u32 {
        match self {
            ResourceTier::Micro => 1,
            ResourceTier::Small => 2,
            ResourceTier::Medium => 4,
            ResourceTier::Large => 8,
            ResourceTier::XLarge => 16,
            ResourceTier::Custom { cpu_cores, .. } => *cpu_cores,
        }
    }

    pub fn memory_gb(&self) -> u32 {
        match self {
            ResourceTier::Micro => 1,
            ResourceTier::Small => 2,
            ResourceTier::Medium => 8,
            ResourceTier::Large => 16,
            ResourceTier::XLarge => 32,
            ResourceTier::Custom { memory_gb, .. } => *memory_gb,
        }
    }

    pub fn disk_gb(&self) -> u32 {
        match self {
            ResourceTier::Micro => 10,
            ResourceTier::Small => 20,
            ResourceTier::Medium => 50,
            ResourceTier::Large => 100,
            ResourceTier::XLarge => 200,
            ResourceTier::Custom { disk_gb, .. } => *disk_gb,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            ResourceTier::Micro => "Micro (1 CPU, 1 GB RAM, 10 GB disk)",
            ResourceTier::Small => "Small (2 CPU, 2 GB RAM, 20 GB disk)",
            ResourceTier::Medium => "Medium (4 CPU, 8 GB RAM, 50 GB disk)",
            ResourceTier::Large => "Large (8 CPU, 16 GB RAM, 100 GB disk)",
            ResourceTier::XLarge => "XLarge (16 CPU, 32 GB RAM, 200 GB disk)",
            ResourceTier::Custom { .. } => "Custom",
        }
    }

    pub fn cost_per_hour(&self) -> f64 {
        match self {
            ResourceTier::Micro => 0.02,
            ResourceTier::Small => 0.05,
            ResourceTier::Medium => 0.15,
            ResourceTier::Large => 0.40,
            ResourceTier::XLarge => 0.80,
            ResourceTier::Custom {
                cpu_cores,
                memory_gb,
                ..
            } => {
                (*cpu_cores as f64) * 0.03 + (*memory_gb as f64) * 0.01
            }
        }
    }
}

/// Port mapping for exposing services from the cloud IDE.
#[derive(Debug, Clone, PartialEq)]
pub struct PortMapping {
    pub internal: u16,
    pub external: Option<u16>,
    pub protocol: String,
    pub label: String,
    pub public: bool,
}

/// Cloud provider backend for IDE instances.
#[derive(Debug, Clone, PartialEq)]
pub enum CloudProvider {
    VibeCodyCloud,
    AWS,
    GCP,
    Azure,
    DigitalOcean,
    Fly,
    Railway,
    Custom(String),
}

/// A running cloud IDE instance with terminal, editor, and browser preview.
#[derive(Debug, Clone)]
pub struct CloudIdeInstance {
    pub id: String,
    pub name: String,
    pub status: CloudIdeStatus,
    pub resource_tier: ResourceTier,
    pub url: Option<String>,
    pub terminal_url: Option<String>,
    pub browser_url: Option<String>,
    pub workspace_path: String,
    pub git_repo: Option<String>,
    pub git_branch: Option<String>,
    pub environment: HashMap<String, String>,
    pub ports: Vec<PortMapping>,
    pub created_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub stopped_at: Option<SystemTime>,
    pub cost_per_hour: f64,
    pub total_cost: f64,
    pub auto_stop_minutes: u64,
    pub extensions: Vec<String>,
}

impl CloudIdeInstance {
    pub fn new(name: &str, tier: ResourceTier) -> Self {
        let cost = tier.cost_per_hour();
        let id = format!("ide-{}-{}", name.replace(' ', "-").to_lowercase(), SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() % 100_000);
        Self {
            id,
            name: name.to_string(),
            status: CloudIdeStatus::Provisioning,
            resource_tier: tier,
            url: None,
            terminal_url: None,
            browser_url: None,
            workspace_path: format!("/workspace/{}", name.replace(' ', "-").to_lowercase()),
            git_repo: None,
            git_branch: None,
            environment: HashMap::new(),
            ports: Vec::new(),
            created_at: SystemTime::now(),
            started_at: None,
            stopped_at: None,
            cost_per_hour: cost,
            total_cost: 0.0,
            auto_stop_minutes: 60,
            extensions: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        self.status = CloudIdeStatus::Running;
        self.started_at = Some(SystemTime::now());
        self.stopped_at = None;
        let base = format!("https://{}.cloud.vibecody.dev", self.id);
        self.url = Some(base.clone());
        self.terminal_url = Some(format!("{}/terminal", base));
        self.browser_url = Some(format!("{}/preview", base));
    }

    pub fn stop(&mut self) {
        self.update_cost();
        self.status = CloudIdeStatus::Stopped;
        self.stopped_at = Some(SystemTime::now());
        self.url = None;
        self.terminal_url = None;
        self.browser_url = None;
    }

    pub fn hibernate(&mut self) {
        self.update_cost();
        self.status = CloudIdeStatus::Hibernating;
        self.stopped_at = Some(SystemTime::now());
        self.url = None;
        self.terminal_url = None;
        self.browser_url = None;
    }

    pub fn resume(&mut self) {
        self.status = CloudIdeStatus::Running;
        self.started_at = Some(SystemTime::now());
        self.stopped_at = None;
        let base = format!("https://{}.cloud.vibecody.dev", self.id);
        self.url = Some(base.clone());
        self.terminal_url = Some(format!("{}/terminal", base));
        self.browser_url = Some(format!("{}/preview", base));
    }

    pub fn fail(&mut self, error: &str) {
        self.update_cost();
        self.status = CloudIdeStatus::Failed(error.to_string());
        self.stopped_at = Some(SystemTime::now());
        self.url = None;
        self.terminal_url = None;
        self.browser_url = None;
    }

    pub fn add_port(&mut self, mapping: PortMapping) {
        self.ports.push(mapping);
    }

    pub fn add_env(&mut self, key: &str, value: &str) {
        self.environment.insert(key.to_string(), value.to_string());
    }

    pub fn set_git(&mut self, repo: &str, branch: &str) {
        self.git_repo = Some(repo.to_string());
        self.git_branch = Some(branch.to_string());
    }

    pub fn elapsed_running(&self) -> Duration {
        match (&self.status, self.started_at) {
            (CloudIdeStatus::Running, Some(started)) => {
                SystemTime::now().duration_since(started).unwrap_or(Duration::ZERO)
            }
            (_, Some(started)) => {
                if let Some(stopped) = self.stopped_at {
                    stopped.duration_since(started).unwrap_or(Duration::ZERO)
                } else {
                    Duration::ZERO
                }
            }
            _ => Duration::ZERO,
        }
    }

    pub fn update_cost(&mut self) {
        let elapsed = self.elapsed_running();
        let hours = elapsed.as_secs_f64() / 3600.0;
        self.total_cost = hours * self.cost_per_hour;
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, CloudIdeStatus::Running)
    }

    pub fn full_url(&self) -> Option<String> {
        self.url.as_ref().map(|u| {
            if let Some(ref repo) = self.git_repo {
                format!("{}?repo={}", u, repo)
            } else {
                u.clone()
            }
        })
    }
}

/// Configuration for the cloud IDE manager.
#[derive(Debug, Clone)]
pub struct CloudIdeConfig {
    pub provider: CloudProvider,
    pub default_tier: ResourceTier,
    pub auto_stop_minutes: u64,
    pub auto_hibernate_minutes: u64,
    pub max_instances: usize,
    pub default_extensions: Vec<String>,
    pub snapshot_on_stop: bool,
    pub region: String,
}

impl CloudIdeConfig {
    pub fn default_config() -> Self {
        Self {
            provider: CloudProvider::VibeCodyCloud,
            default_tier: ResourceTier::Small,
            auto_stop_minutes: 60,
            auto_hibernate_minutes: 240,
            max_instances: 5,
            default_extensions: vec![
                "rust-analyzer".to_string(),
                "eslint".to_string(),
                "prettier".to_string(),
            ],
            snapshot_on_stop: true,
            region: "us-east-1".to_string(),
        }
    }

    pub fn development() -> Self {
        Self {
            provider: CloudProvider::VibeCodyCloud,
            default_tier: ResourceTier::Small,
            auto_stop_minutes: 480,
            auto_hibernate_minutes: 720,
            max_instances: 3,
            default_extensions: vec![
                "rust-analyzer".to_string(),
                "eslint".to_string(),
                "prettier".to_string(),
                "github-copilot".to_string(),
            ],
            snapshot_on_stop: true,
            region: "us-east-1".to_string(),
        }
    }

    pub fn ci() -> Self {
        Self {
            provider: CloudProvider::VibeCodyCloud,
            default_tier: ResourceTier::Medium,
            auto_stop_minutes: 15,
            auto_hibernate_minutes: 30,
            max_instances: 10,
            default_extensions: Vec::new(),
            snapshot_on_stop: false,
            region: "us-east-1".to_string(),
        }
    }
}

/// A snapshot of a cloud IDE instance state.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub id: String,
    pub instance_id: String,
    pub created_at: SystemTime,
    pub size_mb: u64,
    pub description: String,
}

/// Manages multiple cloud IDE instances.
#[derive(Debug)]
pub struct CloudIdeManager {
    pub instances: Vec<CloudIdeInstance>,
    pub config: CloudIdeConfig,
    pub snapshots: Vec<Snapshot>,
}

impl CloudIdeManager {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
            config: CloudIdeConfig::default_config(),
            snapshots: Vec::new(),
        }
    }

    pub fn create_instance(&mut self, name: &str, tier: ResourceTier) -> &CloudIdeInstance {
        let instance = CloudIdeInstance::new(name, tier);
        self.instances.push(instance);
        self.instances.last().expect("just pushed an instance")
    }

    pub fn start_instance(&mut self, id: &str) -> Result<(), String> {
        let instance = self
            .instances
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or_else(|| format!("Instance not found: {}", id))?;
        match &instance.status {
            CloudIdeStatus::Running => return Err("Instance is already running".to_string()),
            CloudIdeStatus::Failed(e) => {
                return Err(format!("Instance is in failed state: {}", e))
            }
            _ => {}
        }
        instance.start();
        Ok(())
    }

    pub fn stop_instance(&mut self, id: &str) -> Result<(), String> {
        let instance = self
            .instances
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or_else(|| format!("Instance not found: {}", id))?;
        if !instance.is_running() {
            return Err("Instance is not running".to_string());
        }
        if self.config.snapshot_on_stop {
            let snap = Snapshot {
                id: format!("snap-{}", self.snapshots.len() + 1),
                instance_id: instance.id.clone(),
                created_at: SystemTime::now(),
                size_mb: instance.resource_tier.disk_gb() as u64 * 10,
                description: format!("Auto-snapshot before stopping {}", instance.name),
            };
            self.snapshots.push(snap);
        }
        instance.stop();
        Ok(())
    }

    pub fn get_instance(&self, id: &str) -> Option<&CloudIdeInstance> {
        self.instances.iter().find(|i| i.id == id)
    }

    pub fn running_instances(&self) -> Vec<&CloudIdeInstance> {
        self.instances.iter().filter(|i| i.is_running()).collect()
    }

    pub fn total_cost(&self) -> f64 {
        self.instances.iter().map(|i| i.total_cost).sum()
    }

    pub fn create_snapshot(&mut self, instance_id: &str, desc: &str) -> Result<Snapshot, String> {
        let instance = self
            .instances
            .iter()
            .find(|i| i.id == instance_id)
            .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
        let snap = Snapshot {
            id: format!("snap-{}", self.snapshots.len() + 1),
            instance_id: instance.id.clone(),
            created_at: SystemTime::now(),
            size_mb: instance.resource_tier.disk_gb() as u64 * 10,
            description: desc.to_string(),
        };
        self.snapshots.push(snap.clone());
        Ok(snap)
    }

    pub fn cleanup_stopped(&mut self) {
        self.instances
            .retain(|i| !matches!(i.status, CloudIdeStatus::Stopped));
    }

    pub fn can_create(&self) -> bool {
        self.instances.len() < self.config.max_instances
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_tier_micro() {
        let tier = ResourceTier::Micro;
        assert_eq!(tier.cpu_cores(), 1);
        assert_eq!(tier.memory_gb(), 1);
        assert_eq!(tier.disk_gb(), 10);
    }

    #[test]
    fn test_resource_tier_small() {
        let tier = ResourceTier::Small;
        assert_eq!(tier.cpu_cores(), 2);
        assert_eq!(tier.memory_gb(), 2);
        assert_eq!(tier.disk_gb(), 20);
    }

    #[test]
    fn test_resource_tier_medium() {
        let tier = ResourceTier::Medium;
        assert_eq!(tier.cpu_cores(), 4);
        assert_eq!(tier.memory_gb(), 8);
        assert_eq!(tier.disk_gb(), 50);
    }

    #[test]
    fn test_resource_tier_large() {
        let tier = ResourceTier::Large;
        assert_eq!(tier.cpu_cores(), 8);
        assert_eq!(tier.memory_gb(), 16);
        assert_eq!(tier.disk_gb(), 100);
    }

    #[test]
    fn test_resource_tier_xlarge() {
        let tier = ResourceTier::XLarge;
        assert_eq!(tier.cpu_cores(), 16);
        assert_eq!(tier.memory_gb(), 32);
        assert_eq!(tier.disk_gb(), 200);
    }

    #[test]
    fn test_resource_tier_custom() {
        let tier = ResourceTier::Custom {
            cpu_cores: 6,
            memory_gb: 12,
            disk_gb: 80,
        };
        assert_eq!(tier.cpu_cores(), 6);
        assert_eq!(tier.memory_gb(), 12);
        assert_eq!(tier.disk_gb(), 80);
    }

    #[test]
    fn test_resource_tier_display_name() {
        assert!(ResourceTier::Micro.display_name().contains("Micro"));
        assert!(ResourceTier::XLarge.display_name().contains("XLarge"));
        assert_eq!(
            ResourceTier::Custom {
                cpu_cores: 1,
                memory_gb: 1,
                disk_gb: 1
            }
            .display_name(),
            "Custom"
        );
    }

    #[test]
    fn test_resource_tier_cost() {
        assert!((ResourceTier::Micro.cost_per_hour() - 0.02).abs() < f64::EPSILON);
        assert!((ResourceTier::Small.cost_per_hour() - 0.05).abs() < f64::EPSILON);
        assert!((ResourceTier::Medium.cost_per_hour() - 0.15).abs() < f64::EPSILON);
        assert!((ResourceTier::Large.cost_per_hour() - 0.40).abs() < f64::EPSILON);
        assert!((ResourceTier::XLarge.cost_per_hour() - 0.80).abs() < f64::EPSILON);
    }

    #[test]
    fn test_custom_tier_cost() {
        let tier = ResourceTier::Custom {
            cpu_cores: 4,
            memory_gb: 8,
            disk_gb: 50,
        };
        let expected = 4.0 * 0.03 + 8.0 * 0.01;
        assert!((tier.cost_per_hour() - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_instance_new() {
        let inst = CloudIdeInstance::new("my-project", ResourceTier::Small);
        assert!(inst.id.starts_with("ide-my-project-"));
        assert_eq!(inst.name, "my-project");
        assert_eq!(inst.status, CloudIdeStatus::Provisioning);
        assert_eq!(inst.resource_tier, ResourceTier::Small);
        assert!(inst.url.is_none());
        assert_eq!(inst.auto_stop_minutes, 60);
        assert_eq!(inst.workspace_path, "/workspace/my-project");
    }

    #[test]
    fn test_instance_start() {
        let mut inst = CloudIdeInstance::new("test", ResourceTier::Micro);
        inst.start();
        assert!(inst.is_running());
        assert!(inst.url.is_some());
        assert!(inst.terminal_url.is_some());
        assert!(inst.browser_url.is_some());
        assert!(inst.started_at.is_some());
    }

    #[test]
    fn test_instance_stop() {
        let mut inst = CloudIdeInstance::new("test", ResourceTier::Micro);
        inst.start();
        inst.stop();
        assert_eq!(inst.status, CloudIdeStatus::Stopped);
        assert!(inst.url.is_none());
        assert!(inst.terminal_url.is_none());
        assert!(inst.stopped_at.is_some());
    }

    #[test]
    fn test_instance_hibernate() {
        let mut inst = CloudIdeInstance::new("test", ResourceTier::Micro);
        inst.start();
        inst.hibernate();
        assert_eq!(inst.status, CloudIdeStatus::Hibernating);
        assert!(inst.url.is_none());
    }

    #[test]
    fn test_instance_resume() {
        let mut inst = CloudIdeInstance::new("test", ResourceTier::Micro);
        inst.start();
        inst.hibernate();
        inst.resume();
        assert!(inst.is_running());
        assert!(inst.url.is_some());
    }

    #[test]
    fn test_instance_fail() {
        let mut inst = CloudIdeInstance::new("test", ResourceTier::Micro);
        inst.start();
        inst.fail("OOM killed");
        assert_eq!(inst.status, CloudIdeStatus::Failed("OOM killed".to_string()));
        assert!(inst.url.is_none());
    }

    #[test]
    fn test_instance_add_port() {
        let mut inst = CloudIdeInstance::new("test", ResourceTier::Micro);
        inst.add_port(PortMapping {
            internal: 3000,
            external: Some(8080),
            protocol: "http".to_string(),
            label: "Frontend".to_string(),
            public: true,
        });
        assert_eq!(inst.ports.len(), 1);
        assert_eq!(inst.ports[0].internal, 3000);
    }

    #[test]
    fn test_instance_add_env() {
        let mut inst = CloudIdeInstance::new("test", ResourceTier::Micro);
        inst.add_env("NODE_ENV", "development");
        assert_eq!(inst.environment.get("NODE_ENV").unwrap(), "development");
    }

    #[test]
    fn test_instance_set_git() {
        let mut inst = CloudIdeInstance::new("test", ResourceTier::Micro);
        inst.set_git("https://github.com/user/repo.git", "main");
        assert_eq!(inst.git_repo.as_deref(), Some("https://github.com/user/repo.git"));
        assert_eq!(inst.git_branch.as_deref(), Some("main"));
    }

    #[test]
    fn test_instance_is_running() {
        let mut inst = CloudIdeInstance::new("test", ResourceTier::Micro);
        assert!(!inst.is_running());
        inst.start();
        assert!(inst.is_running());
        inst.stop();
        assert!(!inst.is_running());
    }

    #[test]
    fn test_instance_full_url_none_when_stopped() {
        let inst = CloudIdeInstance::new("test", ResourceTier::Micro);
        assert!(inst.full_url().is_none());
    }

    #[test]
    fn test_instance_full_url_with_repo() {
        let mut inst = CloudIdeInstance::new("test", ResourceTier::Micro);
        inst.set_git("https://github.com/user/repo.git", "main");
        inst.start();
        let url = inst.full_url().unwrap();
        assert!(url.contains("repo="));
    }

    #[test]
    fn test_instance_full_url_without_repo() {
        let mut inst = CloudIdeInstance::new("test", ResourceTier::Micro);
        inst.start();
        let url = inst.full_url().unwrap();
        assert!(!url.contains("repo="));
    }

    #[test]
    fn test_instance_elapsed_zero_when_not_started() {
        let inst = CloudIdeInstance::new("test", ResourceTier::Micro);
        assert_eq!(inst.elapsed_running(), Duration::ZERO);
    }

    #[test]
    fn test_config_default() {
        let cfg = CloudIdeConfig::default_config();
        assert_eq!(cfg.provider, CloudProvider::VibeCodyCloud);
        assert_eq!(cfg.default_tier, ResourceTier::Small);
        assert_eq!(cfg.auto_stop_minutes, 60);
        assert_eq!(cfg.max_instances, 5);
        assert!(cfg.snapshot_on_stop);
        assert_eq!(cfg.region, "us-east-1");
    }

    #[test]
    fn test_config_development() {
        let cfg = CloudIdeConfig::development();
        assert_eq!(cfg.default_tier, ResourceTier::Small);
        assert_eq!(cfg.auto_stop_minutes, 480);
        assert!(cfg.default_extensions.len() >= 3);
    }

    #[test]
    fn test_config_ci() {
        let cfg = CloudIdeConfig::ci();
        assert_eq!(cfg.default_tier, ResourceTier::Medium);
        assert_eq!(cfg.auto_stop_minutes, 15);
        assert_eq!(cfg.max_instances, 10);
        assert!(!cfg.snapshot_on_stop);
    }

    #[test]
    fn test_manager_new() {
        let mgr = CloudIdeManager::new();
        assert!(mgr.instances.is_empty());
        assert!(mgr.snapshots.is_empty());
    }

    #[test]
    fn test_manager_create_instance() {
        let mut mgr = CloudIdeManager::new();
        let inst = mgr.create_instance("test-proj", ResourceTier::Medium);
        assert_eq!(inst.name, "test-proj");
        assert_eq!(inst.resource_tier, ResourceTier::Medium);
        assert_eq!(mgr.instances.len(), 1);
    }

    #[test]
    fn test_manager_start_instance() {
        let mut mgr = CloudIdeManager::new();
        mgr.create_instance("test", ResourceTier::Micro);
        let id = mgr.instances[0].id.clone();
        assert!(mgr.start_instance(&id).is_ok());
        assert!(mgr.instances[0].is_running());
    }

    #[test]
    fn test_manager_start_already_running() {
        let mut mgr = CloudIdeManager::new();
        mgr.create_instance("test", ResourceTier::Micro);
        let id = mgr.instances[0].id.clone();
        mgr.start_instance(&id).unwrap();
        assert!(mgr.start_instance(&id).is_err());
    }

    #[test]
    fn test_manager_start_nonexistent() {
        let mut mgr = CloudIdeManager::new();
        assert!(mgr.start_instance("nonexistent").is_err());
    }

    #[test]
    fn test_manager_stop_instance() {
        let mut mgr = CloudIdeManager::new();
        mgr.create_instance("test", ResourceTier::Micro);
        let id = mgr.instances[0].id.clone();
        mgr.start_instance(&id).unwrap();
        assert!(mgr.stop_instance(&id).is_ok());
        assert_eq!(mgr.instances[0].status, CloudIdeStatus::Stopped);
    }

    #[test]
    fn test_manager_stop_creates_snapshot() {
        let mut mgr = CloudIdeManager::new();
        mgr.create_instance("test", ResourceTier::Micro);
        let id = mgr.instances[0].id.clone();
        mgr.start_instance(&id).unwrap();
        mgr.stop_instance(&id).unwrap();
        assert_eq!(mgr.snapshots.len(), 1);
        assert!(mgr.snapshots[0].description.contains("test"));
    }

    #[test]
    fn test_manager_stop_not_running() {
        let mut mgr = CloudIdeManager::new();
        mgr.create_instance("test", ResourceTier::Micro);
        let id = mgr.instances[0].id.clone();
        assert!(mgr.stop_instance(&id).is_err());
    }

    #[test]
    fn test_manager_get_instance() {
        let mut mgr = CloudIdeManager::new();
        mgr.create_instance("test", ResourceTier::Micro);
        let id = mgr.instances[0].id.clone();
        assert!(mgr.get_instance(&id).is_some());
        assert!(mgr.get_instance("nope").is_none());
    }

    #[test]
    fn test_manager_running_instances() {
        let mut mgr = CloudIdeManager::new();
        mgr.create_instance("a", ResourceTier::Micro);
        mgr.create_instance("b", ResourceTier::Micro);
        let id_a = mgr.instances[0].id.clone();
        mgr.start_instance(&id_a).unwrap();
        assert_eq!(mgr.running_instances().len(), 1);
    }

    #[test]
    fn test_manager_total_cost() {
        let mut mgr = CloudIdeManager::new();
        mgr.create_instance("a", ResourceTier::Micro);
        mgr.instances[0].total_cost = 1.50;
        mgr.create_instance("b", ResourceTier::Small);
        mgr.instances[1].total_cost = 2.25;
        assert!((mgr.total_cost() - 3.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_manager_create_snapshot() {
        let mut mgr = CloudIdeManager::new();
        mgr.create_instance("test", ResourceTier::Micro);
        let id = mgr.instances[0].id.clone();
        let snap = mgr.create_snapshot(&id, "before deploy").unwrap();
        assert_eq!(snap.description, "before deploy");
        assert_eq!(snap.instance_id, id);
    }

    #[test]
    fn test_manager_create_snapshot_nonexistent() {
        let mut mgr = CloudIdeManager::new();
        assert!(mgr.create_snapshot("nope", "test").is_err());
    }

    #[test]
    fn test_manager_cleanup_stopped() {
        let mut mgr = CloudIdeManager::new();
        mgr.config.snapshot_on_stop = false;
        mgr.create_instance("a", ResourceTier::Micro);
        mgr.create_instance("b", ResourceTier::Micro);
        let id_a = mgr.instances[0].id.clone();
        let id_b = mgr.instances[1].id.clone();
        mgr.start_instance(&id_a).unwrap();
        mgr.start_instance(&id_b).unwrap();
        mgr.stop_instance(&id_a).unwrap();
        mgr.cleanup_stopped();
        assert_eq!(mgr.instances.len(), 1);
        assert_eq!(mgr.instances[0].id, id_b);
    }

    #[test]
    fn test_manager_can_create() {
        let mut mgr = CloudIdeManager::new();
        mgr.config.max_instances = 2;
        assert!(mgr.can_create());
        mgr.create_instance("a", ResourceTier::Micro);
        assert!(mgr.can_create());
        mgr.create_instance("b", ResourceTier::Micro);
        assert!(!mgr.can_create());
    }

    #[test]
    fn test_cloud_provider_variants() {
        let p = CloudProvider::Custom("hetzner".to_string());
        assert_eq!(p, CloudProvider::Custom("hetzner".to_string()));
        assert_ne!(p, CloudProvider::AWS);
    }

    #[test]
    fn test_port_mapping() {
        let pm = PortMapping {
            internal: 5432,
            external: Some(15432),
            protocol: "tcp".to_string(),
            label: "Database".to_string(),
            public: false,
        };
        assert_eq!(pm.internal, 5432);
        assert!(!pm.public);
    }

    #[test]
    fn test_instance_workspace_path_normalization() {
        let inst = CloudIdeInstance::new("My Cool Project", ResourceTier::Micro);
        assert_eq!(inst.workspace_path, "/workspace/my-cool-project");
    }

    #[test]
    fn test_instance_extensions() {
        let mut inst = CloudIdeInstance::new("test", ResourceTier::Micro);
        inst.extensions.push("rust-analyzer".to_string());
        inst.extensions.push("eslint".to_string());
        assert_eq!(inst.extensions.len(), 2);
    }
}
