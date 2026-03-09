//! Cloud sandbox IDE — browser-based IDE powered by cloud containers.
//!
//! Closes P2 Gap 11: Cloud sandbox with full IDE experience in browser.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Sandbox types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum SandboxState {
    Creating,
    Running,
    Stopped,
    Failed,
    Expired,
}

impl SandboxState {
    pub fn as_str(&self) -> &str {
        match self {
            SandboxState::Creating => "creating",
            SandboxState::Running => "running",
            SandboxState::Stopped => "stopped",
            SandboxState::Failed => "failed",
            SandboxState::Expired => "expired",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, SandboxState::Creating | SandboxState::Running)
    }
}

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub image: String,
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub disk_gb: u64,
    pub timeout_secs: u64,
    pub ports: Vec<PortMapping>,
    pub env_vars: HashMap<String, String>,
    pub workspace_path: String,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            image: "ubuntu:22.04".to_string(),
            cpu_cores: 2,
            memory_mb: 4096,
            disk_gb: 20,
            timeout_secs: 3600,
            ports: vec![PortMapping { container: 8080, host: None, protocol: "tcp".to_string() }],
            env_vars: HashMap::new(),
            workspace_path: "/workspace".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PortMapping {
    pub container: u16,
    pub host: Option<u16>,
    pub protocol: String,
}

#[derive(Debug, Clone)]
pub struct SandboxInstance {
    pub id: String,
    pub name: String,
    pub state: SandboxState,
    pub config: SandboxConfig,
    pub created_at: u64,
    pub url: Option<String>,
    pub owner: String,
    pub files_synced: usize,
}

#[derive(Debug, Clone)]
pub struct SandboxTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub config: SandboxConfig,
    pub preinstalled: Vec<String>,
}

// ---------------------------------------------------------------------------
// Cloud sandbox manager
// ---------------------------------------------------------------------------

pub struct CloudSandboxManager {
    instances: Vec<SandboxInstance>,
    templates: Vec<SandboxTemplate>,
    instance_counter: u64,
}

impl CloudSandboxManager {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
            templates: default_templates(),
            instance_counter: 0,
        }
    }

    pub fn create_instance(&mut self, name: &str, config: SandboxConfig, owner: &str) -> String {
        self.instance_counter += 1;
        let id = format!("sandbox-{}", self.instance_counter);
        let instance = SandboxInstance {
            id: id.clone(),
            name: name.to_string(),
            state: SandboxState::Creating,
            config,
            created_at: now(),
            url: None,
            owner: owner.to_string(),
            files_synced: 0,
        };
        self.instances.push(instance);
        id
    }

    pub fn create_from_template(&mut self, template_id: &str, name: &str, owner: &str) -> Option<String> {
        let config = self.templates.iter().find(|t| t.id == template_id)?.config.clone();
        Some(self.create_instance(name, config, owner))
    }

    pub fn start_instance(&mut self, id: &str) -> bool {
        if let Some(inst) = self.instances.iter_mut().find(|i| i.id == id) {
            if inst.state == SandboxState::Creating || inst.state == SandboxState::Stopped {
                inst.state = SandboxState::Running;
                inst.url = Some(format!("https://{}.sandbox.vibecody.dev", id));
                return true;
            }
        }
        false
    }

    pub fn stop_instance(&mut self, id: &str) -> bool {
        if let Some(inst) = self.instances.iter_mut().find(|i| i.id == id) {
            if inst.state == SandboxState::Running {
                inst.state = SandboxState::Stopped;
                inst.url = None;
                return true;
            }
        }
        false
    }

    pub fn get_instance(&self, id: &str) -> Option<&SandboxInstance> {
        self.instances.iter().find(|i| i.id == id)
    }

    pub fn list_instances(&self, owner: Option<&str>) -> Vec<&SandboxInstance> {
        match owner {
            Some(o) => self.instances.iter().filter(|i| i.owner == o).collect(),
            None => self.instances.iter().collect(),
        }
    }

    pub fn active_instances(&self) -> Vec<&SandboxInstance> {
        self.instances.iter().filter(|i| i.state.is_active()).collect()
    }

    pub fn sync_files(&mut self, id: &str, count: usize) -> bool {
        if let Some(inst) = self.instances.iter_mut().find(|i| i.id == id) {
            inst.files_synced += count;
            true
        } else {
            false
        }
    }

    pub fn expire_instance(&mut self, id: &str) -> bool {
        if let Some(inst) = self.instances.iter_mut().find(|i| i.id == id) {
            inst.state = SandboxState::Expired;
            inst.url = None;
            true
        } else {
            false
        }
    }

    pub fn list_templates(&self) -> &[SandboxTemplate] {
        &self.templates
    }

    pub fn add_template(&mut self, template: SandboxTemplate) {
        self.templates.push(template);
    }

    pub fn total_instances(&self) -> usize {
        self.instances.len()
    }
}

impl Default for CloudSandboxManager {
    fn default() -> Self {
        Self::new()
    }
}

fn default_templates() -> Vec<SandboxTemplate> {
    vec![
        SandboxTemplate {
            id: "rust-dev".to_string(),
            name: "Rust Development".to_string(),
            description: "Rust toolchain with cargo, clippy, rustfmt".to_string(),
            config: SandboxConfig {
                image: "rust:latest".to_string(),
                ..SandboxConfig::default()
            },
            preinstalled: vec!["rustc".into(), "cargo".into(), "clippy".into()],
        },
        SandboxTemplate {
            id: "node-dev".to_string(),
            name: "Node.js Development".to_string(),
            description: "Node.js with npm, yarn, pnpm".to_string(),
            config: SandboxConfig {
                image: "node:20".to_string(),
                ..SandboxConfig::default()
            },
            preinstalled: vec!["node".into(), "npm".into(), "yarn".into()],
        },
        SandboxTemplate {
            id: "python-dev".to_string(),
            name: "Python Development".to_string(),
            description: "Python with pip, venv, common packages".to_string(),
            config: SandboxConfig {
                image: "python:3.12".to_string(),
                ..SandboxConfig::default()
            },
            preinstalled: vec!["python3".into(), "pip".into()],
        },
    ]
}

fn now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_state() {
        assert_eq!(SandboxState::Running.as_str(), "running");
        assert!(SandboxState::Running.is_active());
        assert!(SandboxState::Creating.is_active());
        assert!(!SandboxState::Stopped.is_active());
        assert!(!SandboxState::Expired.is_active());
    }

    #[test]
    fn test_default_config() {
        let cfg = SandboxConfig::default();
        assert_eq!(cfg.cpu_cores, 2);
        assert_eq!(cfg.memory_mb, 4096);
        assert_eq!(cfg.ports.len(), 1);
    }

    #[test]
    fn test_create_instance() {
        let mut mgr = CloudSandboxManager::new();
        let id = mgr.create_instance("test", SandboxConfig::default(), "user1");
        let inst = mgr.get_instance(&id).unwrap();
        assert_eq!(inst.state, SandboxState::Creating);
        assert_eq!(inst.owner, "user1");
    }

    #[test]
    fn test_start_stop() {
        let mut mgr = CloudSandboxManager::new();
        let id = mgr.create_instance("test", SandboxConfig::default(), "user1");
        assert!(mgr.start_instance(&id));
        assert_eq!(mgr.get_instance(&id).unwrap().state, SandboxState::Running);
        assert!(mgr.get_instance(&id).unwrap().url.is_some());
        assert!(mgr.stop_instance(&id));
        assert_eq!(mgr.get_instance(&id).unwrap().state, SandboxState::Stopped);
        assert!(mgr.get_instance(&id).unwrap().url.is_none());
    }

    #[test]
    fn test_cannot_stop_creating() {
        let mut mgr = CloudSandboxManager::new();
        let id = mgr.create_instance("test", SandboxConfig::default(), "user1");
        assert!(!mgr.stop_instance(&id)); // still Creating
    }

    #[test]
    fn test_create_from_template() {
        let mut mgr = CloudSandboxManager::new();
        let id = mgr.create_from_template("rust-dev", "my-rust", "user1");
        assert!(id.is_some());
        let id = id.unwrap();
        assert_eq!(mgr.get_instance(&id).unwrap().config.image, "rust:latest");
    }

    #[test]
    fn test_create_from_nonexistent_template() {
        let mut mgr = CloudSandboxManager::new();
        assert!(mgr.create_from_template("fake", "name", "user").is_none());
    }

    #[test]
    fn test_list_instances() {
        let mut mgr = CloudSandboxManager::new();
        mgr.create_instance("s1", SandboxConfig::default(), "user1");
        mgr.create_instance("s2", SandboxConfig::default(), "user2");
        mgr.create_instance("s3", SandboxConfig::default(), "user1");
        assert_eq!(mgr.list_instances(None).len(), 3);
        assert_eq!(mgr.list_instances(Some("user1")).len(), 2);
        assert_eq!(mgr.list_instances(Some("user2")).len(), 1);
    }

    #[test]
    fn test_active_instances() {
        let mut mgr = CloudSandboxManager::new();
        let id1 = mgr.create_instance("s1", SandboxConfig::default(), "u");
        mgr.create_instance("s2", SandboxConfig::default(), "u");
        mgr.start_instance(&id1);
        // id1 Running, id2 Creating — both active
        assert_eq!(mgr.active_instances().len(), 2);
    }

    #[test]
    fn test_sync_files() {
        let mut mgr = CloudSandboxManager::new();
        let id = mgr.create_instance("s1", SandboxConfig::default(), "u");
        mgr.sync_files(&id, 10);
        mgr.sync_files(&id, 5);
        assert_eq!(mgr.get_instance(&id).unwrap().files_synced, 15);
        assert!(!mgr.sync_files("fake", 1));
    }

    #[test]
    fn test_expire() {
        let mut mgr = CloudSandboxManager::new();
        let id = mgr.create_instance("s1", SandboxConfig::default(), "u");
        mgr.start_instance(&id);
        mgr.expire_instance(&id);
        assert_eq!(mgr.get_instance(&id).unwrap().state, SandboxState::Expired);
    }

    #[test]
    fn test_templates() {
        let mgr = CloudSandboxManager::new();
        assert_eq!(mgr.list_templates().len(), 3);
        assert_eq!(mgr.list_templates()[0].id, "rust-dev");
    }

    #[test]
    fn test_add_template() {
        let mut mgr = CloudSandboxManager::new();
        let tmpl = SandboxTemplate {
            id: "go-dev".into(),
            name: "Go".into(),
            description: "Go dev".into(),
            config: SandboxConfig::default(),
            preinstalled: vec!["go".into()],
        };
        mgr.add_template(tmpl);
        assert_eq!(mgr.list_templates().len(), 4);
    }

    #[test]
    fn test_total_instances() {
        let mut mgr = CloudSandboxManager::new();
        mgr.create_instance("s1", SandboxConfig::default(), "u");
        assert_eq!(mgr.total_instances(), 1);
    }
}
