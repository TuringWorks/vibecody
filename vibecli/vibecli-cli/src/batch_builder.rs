
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum BatchStatus {
    Queued,
    Planning,
    Generating,
    Validating,
    Compiling,
    Testing,
    Completed,
    Failed(String),
    Cancelled,
    Paused,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentRole {
    Architect,
    Backend,
    Frontend,
    Database,
    Infrastructure,
    Testing,
    Documentation,
    Security,
    Performance,
    Integration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TechStack {
    ReactNode,
    ReactPython,
    VueNode,
    AngularJava,
    NextjsPrisma,
    RustActix,
    DjangoHtmx,
    FlutterFirebase,
    RailsPostgres,
    GoGin,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BatchPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GenerationPhase {
    RequirementsAnalysis,
    ArchitectureDesign,
    ModuleDecomposition,
    CodeGeneration,
    TestGeneration,
    DocumentationGeneration,
    CompileValidation,
    IntegrationTesting,
    SecurityAudit,
    FinalReview,
}

// === Core Structures ===

#[derive(Debug, Clone)]
pub struct BatchSpec {
    pub id: String,
    pub title: String,
    pub description: String,
    pub tech_stack: TechStack,
    pub requirements: Vec<String>,
    pub user_stories: Vec<UserStory>,
    pub api_endpoints: Vec<ApiEndpoint>,
    pub data_models: Vec<DataModel>,
    pub ui_components: Vec<UiComponent>,
    pub constraints: Vec<String>,
    pub target_dir: PathBuf,
    pub max_duration: Duration,
    pub priority: BatchPriority,
    pub created_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct UserStory {
    pub id: String,
    pub persona: String,
    pub action: String,
    pub benefit: String,
    pub acceptance_criteria: Vec<String>,
    pub priority: BatchPriority,
    pub estimated_complexity: u32,
}

#[derive(Debug, Clone)]
pub struct ApiEndpoint {
    pub method: String,
    pub path: String,
    pub description: String,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
    pub auth_required: bool,
}

#[derive(Debug, Clone)]
pub struct DataModel {
    pub name: String,
    pub fields: Vec<(String, String, bool)>,
    pub relationships: Vec<(String, String)>,
    pub indexes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UiComponent {
    pub name: String,
    pub component_type: String,
    pub description: String,
    pub data_source: Option<String>,
    pub interactions: Vec<String>,
}

// === Agent System ===

#[derive(Debug, Clone)]
pub struct BatchAgent {
    pub id: String,
    pub role: AgentRole,
    pub assigned_modules: Vec<String>,
    pub status: BatchStatus,
    pub lines_generated: usize,
    pub files_created: Vec<PathBuf>,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AgentPool {
    pub agents: Vec<BatchAgent>,
    pub max_concurrent: usize,
    pub active_count: usize,
    pub completed_count: usize,
    pub failed_count: usize,
}

// === Generation Output ===

#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: PathBuf,
    pub content: String,
    pub agent_id: String,
    pub agent_role: AgentRole,
    pub lines: usize,
    pub phase: GenerationPhase,
    pub validated: bool,
    pub compile_checked: bool,
}

#[derive(Debug, Clone)]
pub struct GenerationMetrics {
    pub total_files: usize,
    pub total_lines: usize,
    pub total_tokens_used: u64,
    pub files_by_role: HashMap<String, usize>,
    pub lines_by_role: HashMap<String, usize>,
    pub phases_completed: Vec<GenerationPhase>,
    pub current_phase: GenerationPhase,
    pub elapsed: Duration,
    pub estimated_remaining: Duration,
    pub compile_pass_rate: f64,
    pub test_pass_rate: f64,
    pub security_issues_found: usize,
    pub security_issues_resolved: usize,
}

// === Batch Run ===

#[derive(Debug, Clone)]
pub struct BatchRun {
    pub id: String,
    pub spec: BatchSpec,
    pub status: BatchStatus,
    pub agent_pool: AgentPool,
    pub generated_files: Vec<GeneratedFile>,
    pub metrics: GenerationMetrics,
    pub architecture_plan: Option<ArchitecturePlan>,
    pub module_graph: Vec<ModuleNode>,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub pause_reason: Option<String>,
    pub logs: Vec<BatchLog>,
}

#[derive(Debug, Clone)]
pub struct ArchitecturePlan {
    pub system_overview: String,
    pub modules: Vec<ModulePlan>,
    pub dependencies: Vec<(String, String)>,
    pub deployment_strategy: String,
    pub database_design: String,
    pub api_design: String,
    pub security_approach: String,
}

#[derive(Debug, Clone)]
pub struct ModulePlan {
    pub name: String,
    pub description: String,
    pub responsibility: String,
    pub assigned_agent: AgentRole,
    pub estimated_files: usize,
    pub estimated_lines: usize,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModuleNode {
    pub name: String,
    pub files: Vec<PathBuf>,
    pub dependencies: Vec<String>,
    pub status: BatchStatus,
    pub assigned_agent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BatchLog {
    pub timestamp: SystemTime,
    pub level: LogLevel,
    pub agent_id: Option<String>,
    pub phase: GenerationPhase,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

// === Batch Builder (Main Engine) ===

pub struct BatchBuilder {
    pub runs: Vec<BatchRun>,
    pub active_run: Option<String>,
    pub config: BatchConfig,
    pub history: Vec<BatchRunSummary>,
}

#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_concurrent_agents: usize,
    pub max_duration_hours: u64,
    pub max_lines_per_run: usize,
    pub max_files_per_run: usize,
    pub compile_check_enabled: bool,
    pub test_generation_enabled: bool,
    pub security_audit_enabled: bool,
    pub doc_generation_enabled: bool,
    pub auto_retry_on_failure: bool,
    pub max_retries: usize,
    pub checkpoint_interval_minutes: u64,
    pub output_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct BatchRunSummary {
    pub id: String,
    pub title: String,
    pub status: BatchStatus,
    pub total_files: usize,
    pub total_lines: usize,
    pub duration: Duration,
    pub agents_used: usize,
    pub completed_at: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub struct RunEstimate {
    pub estimated_files: usize,
    pub estimated_lines: usize,
    pub estimated_duration: Duration,
    pub recommended_agents: usize,
    pub complexity_score: u32,
    pub tech_stack_support: bool,
    pub warnings: Vec<String>,
}

// === Implementations ===

fn generate_id(prefix: &str) -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}-{}", prefix, now.as_nanos() % 1_000_000_000)
}

impl BatchSpec {
    pub fn new(title: &str, description: &str, tech_stack: TechStack) -> Self {
        Self {
            id: generate_id("spec"),
            title: title.to_string(),
            description: description.to_string(),
            tech_stack,
            requirements: Vec::new(),
            user_stories: Vec::new(),
            api_endpoints: Vec::new(),
            data_models: Vec::new(),
            ui_components: Vec::new(),
            constraints: Vec::new(),
            target_dir: PathBuf::from("./output"),
            max_duration: Duration::from_secs(8 * 3600),
            priority: BatchPriority::Normal,
            created_at: SystemTime::now(),
        }
    }

    pub fn add_requirement(&mut self, req: &str) {
        self.requirements.push(req.to_string());
    }

    pub fn add_user_story(&mut self, story: UserStory) {
        self.user_stories.push(story);
    }

    pub fn add_api_endpoint(&mut self, endpoint: ApiEndpoint) {
        self.api_endpoints.push(endpoint);
    }

    pub fn add_data_model(&mut self, model: DataModel) {
        self.data_models.push(model);
    }

    pub fn add_ui_component(&mut self, component: UiComponent) {
        self.ui_components.push(component);
    }

    pub fn estimated_complexity(&self) -> u32 {
        let story_score: u32 = self
            .user_stories
            .iter()
            .map(|s| s.estimated_complexity)
            .sum::<u32>()
            .min(30);
        let endpoint_score = (self.api_endpoints.len() as u32 * 2).min(25);
        let model_score = (self.data_models.len() as u32 * 3).min(20);
        let component_score = (self.ui_components.len() as u32 * 2).min(15);
        let requirement_score = (self.requirements.len() as u32).min(10);
        (story_score + endpoint_score + model_score + component_score + requirement_score).min(100)
    }

    pub fn estimated_lines(&self) -> usize {
        let complexity = self.estimated_complexity() as usize;
        // Base: 500 lines per complexity point, minimum 1000
        let base = complexity * 500;
        base.max(1000)
    }

    pub fn estimated_duration(&self) -> Duration {
        let complexity = self.estimated_complexity();
        // Roughly 5 minutes per complexity point, minimum 30 min
        let minutes = (complexity as u64 * 5).max(30);
        Duration::from_secs(minutes * 60)
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.title.is_empty() {
            errors.push("Title is required".to_string());
        }
        if self.description.is_empty() {
            errors.push("Description is required".to_string());
        }
        if self.requirements.is_empty() && self.user_stories.is_empty() {
            errors.push("At least one requirement or user story is needed".to_string());
        }
        if self.api_endpoints.is_empty() && self.data_models.is_empty() && self.ui_components.is_empty() {
            errors.push("At least one endpoint, data model, or UI component is needed".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl UserStory {
    pub fn new(persona: &str, action: &str, benefit: &str) -> Self {
        Self {
            id: generate_id("story"),
            persona: persona.to_string(),
            action: action.to_string(),
            benefit: benefit.to_string(),
            acceptance_criteria: Vec::new(),
            priority: BatchPriority::Normal,
            estimated_complexity: 3,
        }
    }

    pub fn add_criteria(&mut self, criteria: &str) {
        self.acceptance_criteria.push(criteria.to_string());
    }
}

impl ApiEndpoint {
    pub fn new(method: &str, path: &str, description: &str) -> Self {
        Self {
            method: method.to_uppercase(),
            path: path.to_string(),
            description: description.to_string(),
            request_body: None,
            response_body: None,
            auth_required: false,
        }
    }

    pub fn with_auth(mut self) -> Self {
        self.auth_required = true;
        self
    }

    pub fn with_body(mut self, request: &str, response: &str) -> Self {
        self.request_body = Some(request.to_string());
        self.response_body = Some(response.to_string());
        self
    }
}

impl DataModel {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            fields: Vec::new(),
            relationships: Vec::new(),
            indexes: Vec::new(),
        }
    }

    pub fn add_field(&mut self, name: &str, field_type: &str, required: bool) {
        self.fields.push((name.to_string(), field_type.to_string(), required));
    }

    pub fn add_relationship(&mut self, target: &str, relation: &str) {
        self.relationships.push((target.to_string(), relation.to_string()));
    }

    pub fn add_index(&mut self, field: &str) {
        self.indexes.push(field.to_string());
    }
}

impl UiComponent {
    pub fn new(name: &str, component_type: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            component_type: component_type.to_string(),
            description: description.to_string(),
            data_source: None,
            interactions: Vec::new(),
        }
    }
}

impl BatchAgent {
    pub fn new(role: AgentRole) -> Self {
        Self {
            id: generate_id(&format!("agent-{:?}", role).to_lowercase()),
            role,
            assigned_modules: Vec::new(),
            status: BatchStatus::Queued,
            lines_generated: 0,
            files_created: Vec::new(),
            started_at: None,
            completed_at: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn assign_module(&mut self, module: &str) {
        self.assigned_modules.push(module.to_string());
    }

    pub fn start(&mut self) {
        self.status = BatchStatus::Generating;
        self.started_at = Some(SystemTime::now());
    }

    pub fn complete(&mut self) {
        self.status = BatchStatus::Completed;
        self.completed_at = Some(SystemTime::now());
    }

    pub fn fail(&mut self, error: &str) {
        self.status = BatchStatus::Failed(error.to_string());
        self.errors.push(error.to_string());
        self.completed_at = Some(SystemTime::now());
    }

    pub fn add_generated_file(&mut self, path: PathBuf, lines: usize) {
        self.files_created.push(path);
        self.lines_generated += lines;
    }

    pub fn elapsed(&self) -> Duration {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => end.duration_since(start).unwrap_or_default(),
            (Some(start), None) => SystemTime::now().duration_since(start).unwrap_or_default(),
            _ => Duration::ZERO,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            BatchStatus::Generating | BatchStatus::Validating | BatchStatus::Compiling | BatchStatus::Testing
        )
    }
}

impl AgentPool {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            agents: Vec::new(),
            max_concurrent,
            active_count: 0,
            completed_count: 0,
            failed_count: 0,
        }
    }

    pub fn spawn_agent(&mut self, role: AgentRole) -> &mut BatchAgent {
        let agent = BatchAgent::new(role);
        self.agents.push(agent);
        self.agents.last_mut().expect("just pushed an agent")
    }

    pub fn get_agent(&self, id: &str) -> Option<&BatchAgent> {
        self.agents.iter().find(|a| a.id == id)
    }

    pub fn active_agents(&self) -> Vec<&BatchAgent> {
        self.agents.iter().filter(|a| a.is_active()).collect()
    }

    pub fn completed_agents(&self) -> Vec<&BatchAgent> {
        self.agents
            .iter()
            .filter(|a| a.status == BatchStatus::Completed)
            .collect()
    }

    pub fn can_spawn(&self) -> bool {
        self.active_count < self.max_concurrent
    }

    pub fn total_lines_generated(&self) -> usize {
        self.agents.iter().map(|a| a.lines_generated).sum()
    }

    pub fn total_files_created(&self) -> usize {
        self.agents.iter().map(|a| a.files_created.len()).sum()
    }

    pub fn progress_percentage(&self) -> f64 {
        if self.agents.is_empty() {
            return 0.0;
        }
        let done = self
            .agents
            .iter()
            .filter(|a| matches!(a.status, BatchStatus::Completed | BatchStatus::Failed(_)))
            .count();
        (done as f64 / self.agents.len() as f64) * 100.0
    }

    pub fn spawn_standard_team(&mut self) -> Vec<String> {
        let roles = vec![
            AgentRole::Architect,
            AgentRole::Backend,
            AgentRole::Frontend,
            AgentRole::Database,
            AgentRole::Infrastructure,
            AgentRole::Testing,
            AgentRole::Documentation,
            AgentRole::Security,
            AgentRole::Performance,
            AgentRole::Integration,
        ];
        let mut ids = Vec::new();
        for role in roles {
            let agent = self.spawn_agent(role);
            ids.push(agent.id.clone());
        }
        ids
    }
}

impl GeneratedFile {
    pub fn new(
        path: PathBuf,
        content: &str,
        agent_id: &str,
        role: AgentRole,
        phase: GenerationPhase,
    ) -> Self {
        let lines = content.lines().count();
        Self {
            path,
            content: content.to_string(),
            agent_id: agent_id.to_string(),
            agent_role: role,
            lines,
            phase,
            validated: false,
            compile_checked: false,
        }
    }

    pub fn mark_validated(&mut self) {
        self.validated = true;
    }

    pub fn mark_compile_checked(&mut self) {
        self.compile_checked = true;
    }
}

impl GenerationMetrics {
    pub fn new() -> Self {
        Self {
            total_files: 0,
            total_lines: 0,
            total_tokens_used: 0,
            files_by_role: HashMap::new(),
            lines_by_role: HashMap::new(),
            phases_completed: Vec::new(),
            current_phase: GenerationPhase::RequirementsAnalysis,
            elapsed: Duration::ZERO,
            estimated_remaining: Duration::ZERO,
            compile_pass_rate: 0.0,
            test_pass_rate: 0.0,
            security_issues_found: 0,
            security_issues_resolved: 0,
        }
    }

    pub fn update_from_files(&mut self, files: &[GeneratedFile]) {
        self.total_files = files.len();
        self.total_lines = files.iter().map(|f| f.lines).sum();
        self.files_by_role.clear();
        self.lines_by_role.clear();
        for f in files {
            let role_key = format!("{:?}", f.agent_role);
            *self.files_by_role.entry(role_key.clone()).or_insert(0) += 1;
            *self.lines_by_role.entry(role_key).or_insert(0) += f.lines;
        }
        let validated = files.iter().filter(|f| f.compile_checked).count();
        if !files.is_empty() {
            self.compile_pass_rate = validated as f64 / files.len() as f64;
        }
    }

    pub fn update_phase(&mut self, phase: GenerationPhase) {
        if self.current_phase != phase {
            self.phases_completed.push(self.current_phase.clone());
        }
        self.current_phase = phase;
    }

    pub fn lines_per_hour(&self) -> f64 {
        let hours = self.elapsed.as_secs_f64() / 3600.0;
        if hours < 0.0001 {
            return 0.0;
        }
        self.total_lines as f64 / hours
    }

    pub fn files_per_hour(&self) -> f64 {
        let hours = self.elapsed.as_secs_f64() / 3600.0;
        if hours < 0.0001 {
            return 0.0;
        }
        self.total_files as f64 / hours
    }

    pub fn estimated_total_time(&self, target_lines: usize) -> Duration {
        let lph = self.lines_per_hour();
        if lph < 0.1 {
            return Duration::from_secs(24 * 3600); // fallback: 24h
        }
        let hours_needed = target_lines as f64 / lph;
        Duration::from_secs_f64(hours_needed * 3600.0)
    }
}

impl ArchitecturePlan {
    pub fn new(overview: &str) -> Self {
        Self {
            system_overview: overview.to_string(),
            modules: Vec::new(),
            dependencies: Vec::new(),
            deployment_strategy: String::new(),
            database_design: String::new(),
            api_design: String::new(),
            security_approach: String::new(),
        }
    }

    pub fn add_module(&mut self, module: ModulePlan) {
        self.modules.push(module);
    }

    pub fn add_dependency(&mut self, from: &str, to: &str) {
        self.dependencies
            .push((from.to_string(), to.to_string()));
    }

    pub fn topological_order(&self) -> Vec<String> {
        let module_names: Vec<String> = self.modules.iter().map(|m| m.name.clone()).collect();
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();

        for name in &module_names {
            in_degree.insert(name.clone(), 0);
            adj.insert(name.clone(), Vec::new());
        }

        // (from, to) means "from depends on to", so to must come before from
        for (from, to) in &self.dependencies {
            if adj.contains_key(to) && in_degree.contains_key(from) {
                adj.get_mut(to).expect("key exists").push(from.clone());
                *in_degree.get_mut(from).expect("key exists") += 1;
            }
        }

        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();
        queue.sort(); // deterministic ordering

        let mut result = Vec::new();
        while !queue.is_empty() {
            let node = queue.remove(0); // take from front (smallest lexicographic)
            result.push(node.clone());
            if let Some(neighbors) = adj.get(&node) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor.clone());
                            queue.sort();
                        }
                    }
                }
            }
        }

        result
    }

    pub fn dependency_count(&self, module: &str) -> usize {
        self.dependencies
            .iter()
            .filter(|(from, _)| from == module)
            .count()
    }
}

impl BatchRun {
    pub fn new(spec: BatchSpec, config: &BatchConfig) -> Self {
        let id = generate_id("run");
        Self {
            id,
            spec,
            status: BatchStatus::Queued,
            agent_pool: AgentPool::new(config.max_concurrent_agents),
            generated_files: Vec::new(),
            metrics: GenerationMetrics::new(),
            architecture_plan: None,
            module_graph: Vec::new(),
            started_at: None,
            completed_at: None,
            pause_reason: None,
            logs: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        self.status = BatchStatus::Planning;
        self.started_at = Some(SystemTime::now());
    }

    pub fn pause(&mut self, reason: &str) {
        self.status = BatchStatus::Paused;
        self.pause_reason = Some(reason.to_string());
    }

    pub fn resume(&mut self) {
        self.status = BatchStatus::Generating;
        self.pause_reason = None;
    }

    pub fn cancel(&mut self) {
        self.status = BatchStatus::Cancelled;
        self.completed_at = Some(SystemTime::now());
    }

    pub fn complete(&mut self) {
        self.status = BatchStatus::Completed;
        self.completed_at = Some(SystemTime::now());
    }

    pub fn fail(&mut self, error: &str) {
        self.status = BatchStatus::Failed(error.to_string());
        self.completed_at = Some(SystemTime::now());
    }

    pub fn add_log(
        &mut self,
        level: LogLevel,
        phase: GenerationPhase,
        message: &str,
        agent_id: Option<&str>,
    ) {
        self.logs.push(BatchLog {
            timestamp: SystemTime::now(),
            level,
            agent_id: agent_id.map(|s| s.to_string()),
            phase,
            message: message.to_string(),
        });
    }

    pub fn set_architecture(&mut self, plan: ArchitecturePlan) {
        self.architecture_plan = Some(plan);
    }

    pub fn add_generated_file(&mut self, file: GeneratedFile) {
        self.generated_files.push(file);
        self.metrics.update_from_files(&self.generated_files);
    }

    pub fn elapsed(&self) -> Duration {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => end.duration_since(start).unwrap_or_default(),
            (Some(start), None) => SystemTime::now().duration_since(start).unwrap_or_default(),
            _ => Duration::ZERO,
        }
    }

    pub fn is_within_time_budget(&self) -> bool {
        self.elapsed() < self.spec.max_duration
    }

    pub fn progress_percentage(&self) -> f64 {
        self.agent_pool.progress_percentage()
    }

    pub fn to_summary(&self) -> BatchRunSummary {
        BatchRunSummary {
            id: self.id.clone(),
            title: self.spec.title.clone(),
            status: self.status.clone(),
            total_files: self.generated_files.len(),
            total_lines: self.generated_files.iter().map(|f| f.lines).sum(),
            duration: self.elapsed(),
            agents_used: self.agent_pool.agents.len(),
            completed_at: self.completed_at,
        }
    }

    pub fn files_by_phase(&self, phase: &GenerationPhase) -> Vec<&GeneratedFile> {
        self.generated_files
            .iter()
            .filter(|f| f.phase == *phase)
            .collect()
    }

    pub fn files_by_role(&self, role: &AgentRole) -> Vec<&GeneratedFile> {
        self.generated_files
            .iter()
            .filter(|f| f.agent_role == *role)
            .collect()
    }

    pub fn logs_by_level(&self, level: &LogLevel) -> Vec<&BatchLog> {
        self.logs.iter().filter(|l| l.level == *level).collect()
    }
}

impl BatchConfig {
    pub fn default_config() -> Self {
        Self {
            max_concurrent_agents: 10,
            max_duration_hours: 12,
            max_lines_per_run: 3_000_000,
            max_files_per_run: 10_000,
            compile_check_enabled: true,
            test_generation_enabled: true,
            security_audit_enabled: true,
            doc_generation_enabled: true,
            auto_retry_on_failure: true,
            max_retries: 3,
            checkpoint_interval_minutes: 30,
            output_dir: PathBuf::from("./batch_output"),
        }
    }

    pub fn high_performance() -> Self {
        Self {
            max_concurrent_agents: 20,
            max_duration_hours: 24,
            max_lines_per_run: 10_000_000,
            max_files_per_run: 50_000,
            compile_check_enabled: true,
            test_generation_enabled: true,
            security_audit_enabled: true,
            doc_generation_enabled: true,
            auto_retry_on_failure: true,
            max_retries: 5,
            checkpoint_interval_minutes: 15,
            output_dir: PathBuf::from("./batch_output"),
        }
    }

    pub fn conservative() -> Self {
        Self {
            max_concurrent_agents: 4,
            max_duration_hours: 6,
            max_lines_per_run: 500_000,
            max_files_per_run: 2_000,
            compile_check_enabled: true,
            test_generation_enabled: true,
            security_audit_enabled: true,
            doc_generation_enabled: true,
            auto_retry_on_failure: false,
            max_retries: 1,
            checkpoint_interval_minutes: 60,
            output_dir: PathBuf::from("./batch_output"),
        }
    }
}

impl BatchBuilder {
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            active_run: None,
            config: BatchConfig::default_config(),
            history: Vec::new(),
        }
    }

    pub fn with_config(config: BatchConfig) -> Self {
        Self {
            runs: Vec::new(),
            active_run: None,
            config,
            history: Vec::new(),
        }
    }

    pub fn create_run(&mut self, spec: BatchSpec) -> Result<String, String> {
        if let Err(errors) = spec.validate() {
            return Err(format!("Invalid spec: {}", errors.join("; ")));
        }
        let run = BatchRun::new(spec, &self.config);
        let id = run.id.clone();
        self.runs.push(run);
        Ok(id)
    }

    pub fn start_run(&mut self, run_id: &str) -> Result<(), String> {
        if self.active_run.is_some() {
            return Err("Another run is already active".to_string());
        }
        let run = self
            .runs
            .iter_mut()
            .find(|r| r.id == run_id)
            .ok_or_else(|| format!("Run {} not found", run_id))?;
        if run.status != BatchStatus::Queued {
            return Err(format!("Run {} is not in Queued state", run_id));
        }
        run.start();
        self.active_run = Some(run_id.to_string());
        Ok(())
    }

    pub fn pause_run(&mut self, run_id: &str, reason: &str) -> Result<(), String> {
        let run = self
            .runs
            .iter_mut()
            .find(|r| r.id == run_id)
            .ok_or_else(|| format!("Run {} not found", run_id))?;
        match &run.status {
            BatchStatus::Planning
            | BatchStatus::Generating
            | BatchStatus::Validating
            | BatchStatus::Compiling
            | BatchStatus::Testing => {
                run.pause(reason);
                Ok(())
            }
            _ => Err(format!("Run {} cannot be paused in {:?} state", run_id, run.status)),
        }
    }

    pub fn resume_run(&mut self, run_id: &str) -> Result<(), String> {
        let run = self
            .runs
            .iter_mut()
            .find(|r| r.id == run_id)
            .ok_or_else(|| format!("Run {} not found", run_id))?;
        if run.status != BatchStatus::Paused {
            return Err(format!("Run {} is not paused", run_id));
        }
        run.resume();
        Ok(())
    }

    pub fn cancel_run(&mut self, run_id: &str) -> Result<(), String> {
        let run = self
            .runs
            .iter_mut()
            .find(|r| r.id == run_id)
            .ok_or_else(|| format!("Run {} not found", run_id))?;
        if matches!(run.status, BatchStatus::Completed | BatchStatus::Cancelled) {
            return Err(format!("Run {} is already finished", run_id));
        }
        run.cancel();
        if self.active_run.as_deref() == Some(run_id) {
            self.active_run = None;
        }
        Ok(())
    }

    pub fn get_run(&self, run_id: &str) -> Option<&BatchRun> {
        self.runs.iter().find(|r| r.id == run_id)
    }

    pub fn get_run_mut(&mut self, run_id: &str) -> Option<&mut BatchRun> {
        self.runs.iter_mut().find(|r| r.id == run_id)
    }

    pub fn active_run(&self) -> Option<&BatchRun> {
        self.active_run
            .as_ref()
            .and_then(|id| self.runs.iter().find(|r| r.id == *id))
    }

    pub fn list_runs(&self) -> Vec<&BatchRunSummary> {
        self.history.iter().collect()
    }

    pub fn generate_architecture(
        &mut self,
        run_id: &str,
    ) -> Result<ArchitecturePlan, String> {
        let run = self
            .runs
            .iter()
            .find(|r| r.id == run_id)
            .ok_or_else(|| format!("Run {} not found", run_id))?;

        let spec = &run.spec;
        let mut plan = ArchitecturePlan::new(&format!(
            "System architecture for: {}. {}",
            spec.title, spec.description
        ));

        // Generate modules based on spec content
        if !spec.api_endpoints.is_empty() {
            plan.add_module(ModulePlan {
                name: "api".to_string(),
                description: "API layer with route handlers".to_string(),
                responsibility: "HTTP request/response handling".to_string(),
                assigned_agent: AgentRole::Backend,
                estimated_files: spec.api_endpoints.len() + 2,
                estimated_lines: spec.api_endpoints.len() * 150,
                dependencies: vec!["core".to_string()],
            });
        }

        if !spec.data_models.is_empty() {
            plan.add_module(ModulePlan {
                name: "database".to_string(),
                description: "Database schema and migrations".to_string(),
                responsibility: "Data persistence and queries".to_string(),
                assigned_agent: AgentRole::Database,
                estimated_files: spec.data_models.len() * 2 + 1,
                estimated_lines: spec.data_models.len() * 200,
                dependencies: Vec::new(),
            });
        }

        if !spec.ui_components.is_empty() {
            plan.add_module(ModulePlan {
                name: "frontend".to_string(),
                description: "UI components and pages".to_string(),
                responsibility: "User interface rendering".to_string(),
                assigned_agent: AgentRole::Frontend,
                estimated_files: spec.ui_components.len() * 2 + 3,
                estimated_lines: spec.ui_components.len() * 250,
                dependencies: vec!["api".to_string()],
            });
        }

        // Always add core module
        plan.add_module(ModulePlan {
            name: "core".to_string(),
            description: "Core business logic".to_string(),
            responsibility: "Domain logic and validation".to_string(),
            assigned_agent: AgentRole::Architect,
            estimated_files: 5,
            estimated_lines: 1000,
            dependencies: vec!["database".to_string()],
        });

        // Add infrastructure module
        plan.add_module(ModulePlan {
            name: "infra".to_string(),
            description: "Infrastructure and deployment".to_string(),
            responsibility: "Docker, CI/CD, environment config".to_string(),
            assigned_agent: AgentRole::Infrastructure,
            estimated_files: 5,
            estimated_lines: 300,
            dependencies: Vec::new(),
        });

        // Set up dependencies
        if !spec.api_endpoints.is_empty() {
            plan.add_dependency("api", "core");
        }
        if !spec.data_models.is_empty() {
            plan.add_dependency("core", "database");
        }
        if !spec.ui_components.is_empty() && !spec.api_endpoints.is_empty() {
            plan.add_dependency("frontend", "api");
        }

        plan.deployment_strategy = format!("Containerized deployment for {:?}", spec.tech_stack);
        plan.database_design = format!("{} data models with relationships", spec.data_models.len());
        plan.api_design = format!("{} REST endpoints", spec.api_endpoints.len());
        plan.security_approach = "JWT auth, input validation, CORS, rate limiting".to_string();

        Ok(plan)
    }

    pub fn decompose_modules(
        &self,
        plan: &ArchitecturePlan,
        _spec: &BatchSpec,
    ) -> Vec<ModuleNode> {
        plan.modules
            .iter()
            .map(|m| ModuleNode {
                name: m.name.clone(),
                files: Vec::new(),
                dependencies: m.dependencies.clone(),
                status: BatchStatus::Queued,
                assigned_agent: None,
            })
            .collect()
    }

    pub fn assign_agents(&mut self, run_id: &str) -> Result<(), String> {
        let run = self
            .runs
            .iter_mut()
            .find(|r| r.id == run_id)
            .ok_or_else(|| format!("Run {} not found", run_id))?;

        let plan = run
            .architecture_plan
            .as_ref()
            .ok_or("No architecture plan set")?
            .clone();

        let _agent_ids = run.agent_pool.spawn_standard_team();

        // Assign modules to agents based on role matching
        for module in &plan.modules {
            let matching_agent = run
                .agent_pool
                .agents
                .iter_mut()
                .find(|a| a.role == module.assigned_agent);
            if let Some(agent) = matching_agent {
                agent.assign_module(&module.name);
            }
        }

        // Update module graph with agent assignments
        for node in &mut run.module_graph {
            let agent = run
                .agent_pool
                .agents
                .iter()
                .find(|a| a.assigned_modules.contains(&node.name));
            if let Some(a) = agent {
                node.assigned_agent = Some(a.id.clone());
            }
        }

        Ok(())
    }

    pub fn estimate_run(&self, spec: &BatchSpec) -> RunEstimate {
        let complexity = spec.estimated_complexity();
        let lines = spec.estimated_lines();
        let files = lines / 80; // average ~80 lines per file
        let duration = spec.estimated_duration();
        let agents = if complexity > 60 {
            10
        } else if complexity > 30 {
            6
        } else {
            4
        };
        let tech_support = !matches!(spec.tech_stack, TechStack::Custom(_));

        let mut warnings = Vec::new();
        if complexity > 80 {
            warnings.push("Very high complexity — consider breaking into sub-projects".to_string());
        }
        if !tech_support {
            warnings.push("Custom tech stack — templates may not be available".to_string());
        }
        if lines > self.config.max_lines_per_run {
            warnings.push("Estimated lines exceed configured max_lines_per_run".to_string());
        }

        RunEstimate {
            estimated_files: files,
            estimated_lines: lines,
            estimated_duration: duration,
            recommended_agents: agents,
            complexity_score: complexity,
            tech_stack_support: tech_support,
            warnings,
        }
    }

    pub fn run_history(&self) -> &[BatchRunSummary] {
        &self.history
    }

    pub fn cleanup_completed(&mut self) {
        let mut completed_ids = Vec::new();
        for run in &self.runs {
            if matches!(
                run.status,
                BatchStatus::Completed | BatchStatus::Cancelled | BatchStatus::Failed(_)
            ) {
                self.history.push(run.to_summary());
                completed_ids.push(run.id.clone());
            }
        }
        self.runs.retain(|r| !completed_ids.contains(&r.id));
        if let Some(ref active) = self.active_run {
            if completed_ids.contains(active) {
                self.active_run = None;
            }
        }
    }

    pub fn total_lines_generated_all_time(&self) -> usize {
        let current: usize = self
            .runs
            .iter()
            .map(|r| r.generated_files.iter().map(|f| f.lines).sum::<usize>())
            .sum();
        let historical: usize = self.history.iter().map(|h| h.total_lines).sum();
        current + historical
    }
}

// === TechStack Helpers ===

impl TechStack {
    pub fn display_name(&self) -> &str {
        match self {
            TechStack::ReactNode => "React + Node.js + PostgreSQL",
            TechStack::ReactPython => "React + Python/FastAPI + PostgreSQL",
            TechStack::VueNode => "Vue + Node.js + MongoDB",
            TechStack::AngularJava => "Angular + Java/Spring Boot + PostgreSQL",
            TechStack::NextjsPrisma => "Next.js + Prisma + PostgreSQL",
            TechStack::RustActix => "Rust + Actix-web + PostgreSQL",
            TechStack::DjangoHtmx => "Django + HTMX + PostgreSQL",
            TechStack::FlutterFirebase => "Flutter + Firebase",
            TechStack::RailsPostgres => "Ruby on Rails + PostgreSQL",
            TechStack::GoGin => "Go + Gin + PostgreSQL",
            TechStack::Custom(_) => "Custom Stack",
        }
    }

    pub fn frontend_framework(&self) -> Option<&str> {
        match self {
            TechStack::ReactNode | TechStack::ReactPython => Some("React"),
            TechStack::VueNode => Some("Vue"),
            TechStack::AngularJava => Some("Angular"),
            TechStack::NextjsPrisma => Some("Next.js"),
            TechStack::DjangoHtmx => Some("HTMX"),
            TechStack::FlutterFirebase => Some("Flutter"),
            TechStack::RustActix | TechStack::GoGin => None,
            TechStack::RailsPostgres => None,
            TechStack::Custom(_) => None,
        }
    }

    pub fn backend_framework(&self) -> Option<&str> {
        match self {
            TechStack::ReactNode | TechStack::VueNode => Some("Node.js/Express"),
            TechStack::ReactPython => Some("FastAPI"),
            TechStack::AngularJava => Some("Spring Boot"),
            TechStack::NextjsPrisma => Some("Next.js API Routes"),
            TechStack::RustActix => Some("Actix-web"),
            TechStack::DjangoHtmx => Some("Django"),
            TechStack::FlutterFirebase => Some("Firebase"),
            TechStack::RailsPostgres => Some("Ruby on Rails"),
            TechStack::GoGin => Some("Gin"),
            TechStack::Custom(_) => None,
        }
    }

    pub fn database(&self) -> Option<&str> {
        match self {
            TechStack::ReactNode
            | TechStack::ReactPython
            | TechStack::AngularJava
            | TechStack::NextjsPrisma
            | TechStack::RustActix
            | TechStack::DjangoHtmx
            | TechStack::RailsPostgres
            | TechStack::GoGin => Some("PostgreSQL"),
            TechStack::VueNode => Some("MongoDB"),
            TechStack::FlutterFirebase => Some("Firestore"),
            TechStack::Custom(_) => None,
        }
    }

    pub fn language(&self) -> &str {
        match self {
            TechStack::ReactNode | TechStack::VueNode | TechStack::NextjsPrisma => "TypeScript",
            TechStack::ReactPython | TechStack::DjangoHtmx => "Python",
            TechStack::AngularJava => "Java",
            TechStack::RustActix => "Rust",
            TechStack::FlutterFirebase => "Dart",
            TechStack::RailsPostgres => "Ruby",
            TechStack::GoGin => "Go",
            TechStack::Custom(_) => "Unknown",
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            TechStack::ReactNode => 3000,
            TechStack::ReactPython => 8000,
            TechStack::VueNode => 3000,
            TechStack::AngularJava => 8080,
            TechStack::NextjsPrisma => 3000,
            TechStack::RustActix => 8080,
            TechStack::DjangoHtmx => 8000,
            TechStack::FlutterFirebase => 5000,
            TechStack::RailsPostgres => 3000,
            TechStack::GoGin => 8080,
            TechStack::Custom(_) => 8080,
        }
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to build a valid spec for tests
    fn make_valid_spec() -> BatchSpec {
        let mut spec = BatchSpec::new("Test App", "A test application", TechStack::ReactNode);
        spec.add_requirement("User authentication");
        spec.add_user_story(UserStory::new("user", "log in", "access account"));
        spec.add_api_endpoint(ApiEndpoint::new("GET", "/api/users", "List users"));
        spec.add_data_model(DataModel::new("User"));
        spec
    }

    // --- BatchSpec tests ---

    #[test]
    fn test_batch_spec_new() {
        let spec = BatchSpec::new("My App", "Description", TechStack::ReactNode);
        assert_eq!(spec.title, "My App");
        assert_eq!(spec.description, "Description");
        assert_eq!(spec.tech_stack, TechStack::ReactNode);
        assert!(spec.requirements.is_empty());
        assert!(spec.id.starts_with("spec-"));
    }

    #[test]
    fn test_batch_spec_add_requirement() {
        let mut spec = BatchSpec::new("App", "Desc", TechStack::GoGin);
        spec.add_requirement("Must support REST API");
        spec.add_requirement("Must have auth");
        assert_eq!(spec.requirements.len(), 2);
        assert_eq!(spec.requirements[0], "Must support REST API");
    }

    #[test]
    fn test_batch_spec_add_user_story() {
        let mut spec = BatchSpec::new("App", "Desc", TechStack::GoGin);
        let story = UserStory::new("admin", "manage users", "control access");
        spec.add_user_story(story);
        assert_eq!(spec.user_stories.len(), 1);
        assert_eq!(spec.user_stories[0].persona, "admin");
    }

    #[test]
    fn test_batch_spec_add_api_endpoint() {
        let mut spec = BatchSpec::new("App", "Desc", TechStack::GoGin);
        spec.add_api_endpoint(ApiEndpoint::new("POST", "/api/login", "Login"));
        assert_eq!(spec.api_endpoints.len(), 1);
        assert_eq!(spec.api_endpoints[0].method, "POST");
    }

    #[test]
    fn test_batch_spec_add_data_model() {
        let mut spec = BatchSpec::new("App", "Desc", TechStack::GoGin);
        let mut model = DataModel::new("User");
        model.add_field("email", "String", true);
        spec.add_data_model(model);
        assert_eq!(spec.data_models.len(), 1);
    }

    #[test]
    fn test_batch_spec_add_ui_component() {
        let mut spec = BatchSpec::new("App", "Desc", TechStack::ReactNode);
        spec.add_ui_component(UiComponent::new("LoginForm", "form", "Login form"));
        assert_eq!(spec.ui_components.len(), 1);
    }

    #[test]
    fn test_batch_spec_validate_valid() {
        let spec = make_valid_spec();
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_batch_spec_validate_empty_title() {
        let mut spec = make_valid_spec();
        spec.title = String::new();
        let err = spec.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("Title")));
    }

    #[test]
    fn test_batch_spec_validate_no_requirements_or_stories() {
        let mut spec = BatchSpec::new("App", "Desc", TechStack::GoGin);
        spec.add_api_endpoint(ApiEndpoint::new("GET", "/", "root"));
        let err = spec.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("requirement or user story")));
    }

    #[test]
    fn test_batch_spec_validate_no_artifacts() {
        let mut spec = BatchSpec::new("App", "Desc", TechStack::GoGin);
        spec.add_requirement("Something");
        let err = spec.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("endpoint, data model, or UI component")));
    }

    #[test]
    fn test_batch_spec_complexity_empty() {
        let spec = BatchSpec::new("App", "Desc", TechStack::GoGin);
        assert_eq!(spec.estimated_complexity(), 0);
    }

    #[test]
    fn test_batch_spec_complexity_with_content() {
        let spec = make_valid_spec();
        let c = spec.estimated_complexity();
        assert!(c > 0 && c <= 100);
    }

    #[test]
    fn test_batch_spec_estimated_lines() {
        let spec = make_valid_spec();
        assert!(spec.estimated_lines() >= 1000);
    }

    #[test]
    fn test_batch_spec_estimated_duration() {
        let spec = make_valid_spec();
        assert!(spec.estimated_duration() >= Duration::from_secs(30 * 60));
    }

    // --- UserStory tests ---

    #[test]
    fn test_user_story_new() {
        let story = UserStory::new("developer", "deploy code", "ship features");
        assert_eq!(story.persona, "developer");
        assert_eq!(story.action, "deploy code");
        assert_eq!(story.benefit, "ship features");
        assert_eq!(story.estimated_complexity, 3);
        assert!(story.id.starts_with("story-"));
    }

    #[test]
    fn test_user_story_add_criteria() {
        let mut story = UserStory::new("user", "act", "benefit");
        story.add_criteria("Must respond within 2s");
        story.add_criteria("Must show confirmation");
        assert_eq!(story.acceptance_criteria.len(), 2);
    }

    // --- ApiEndpoint tests ---

    #[test]
    fn test_api_endpoint_new() {
        let ep = ApiEndpoint::new("get", "/api/items", "List items");
        assert_eq!(ep.method, "GET"); // uppercased
        assert_eq!(ep.path, "/api/items");
        assert!(!ep.auth_required);
    }

    #[test]
    fn test_api_endpoint_with_auth() {
        let ep = ApiEndpoint::new("POST", "/api/admin", "Admin").with_auth();
        assert!(ep.auth_required);
    }

    #[test]
    fn test_api_endpoint_with_body() {
        let ep = ApiEndpoint::new("POST", "/api/users", "Create user")
            .with_body("{\"name\":\"\"}", "{\"id\":1}");
        assert_eq!(ep.request_body.unwrap(), "{\"name\":\"\"}");
        assert_eq!(ep.response_body.unwrap(), "{\"id\":1}");
    }

    // --- DataModel tests ---

    #[test]
    fn test_data_model_new() {
        let model = DataModel::new("Product");
        assert_eq!(model.name, "Product");
        assert!(model.fields.is_empty());
    }

    #[test]
    fn test_data_model_add_field() {
        let mut model = DataModel::new("Product");
        model.add_field("name", "String", true);
        model.add_field("price", "Float", false);
        assert_eq!(model.fields.len(), 2);
        assert_eq!(model.fields[0], ("name".to_string(), "String".to_string(), true));
    }

    #[test]
    fn test_data_model_add_relationship() {
        let mut model = DataModel::new("Order");
        model.add_relationship("User", "belongs_to");
        assert_eq!(model.relationships.len(), 1);
    }

    #[test]
    fn test_data_model_add_index() {
        let mut model = DataModel::new("User");
        model.add_index("email");
        assert_eq!(model.indexes, vec!["email".to_string()]);
    }

    // --- UiComponent tests ---

    #[test]
    fn test_ui_component_new() {
        let comp = UiComponent::new("Dashboard", "page", "Main dashboard");
        assert_eq!(comp.name, "Dashboard");
        assert_eq!(comp.component_type, "page");
        assert!(comp.data_source.is_none());
    }

    // --- BatchAgent tests ---

    #[test]
    fn test_batch_agent_new() {
        let agent = BatchAgent::new(AgentRole::Backend);
        assert_eq!(agent.role, AgentRole::Backend);
        assert_eq!(agent.status, BatchStatus::Queued);
        assert!(!agent.is_active());
    }

    #[test]
    fn test_batch_agent_lifecycle() {
        let mut agent = BatchAgent::new(AgentRole::Frontend);
        assert!(!agent.is_active());
        agent.start();
        assert!(agent.is_active());
        assert!(agent.started_at.is_some());
        agent.complete();
        assert!(!agent.is_active());
        assert!(agent.completed_at.is_some());
        assert_eq!(agent.status, BatchStatus::Completed);
    }

    #[test]
    fn test_batch_agent_fail() {
        let mut agent = BatchAgent::new(AgentRole::Testing);
        agent.start();
        agent.fail("compilation error");
        assert_eq!(agent.status, BatchStatus::Failed("compilation error".to_string()));
        assert_eq!(agent.errors.len(), 1);
    }

    #[test]
    fn test_batch_agent_assign_module() {
        let mut agent = BatchAgent::new(AgentRole::Database);
        agent.assign_module("schema");
        agent.assign_module("migrations");
        assert_eq!(agent.assigned_modules.len(), 2);
    }

    #[test]
    fn test_batch_agent_add_generated_file() {
        let mut agent = BatchAgent::new(AgentRole::Backend);
        agent.add_generated_file(PathBuf::from("src/main.rs"), 100);
        agent.add_generated_file(PathBuf::from("src/lib.rs"), 50);
        assert_eq!(agent.files_created.len(), 2);
        assert_eq!(agent.lines_generated, 150);
    }

    #[test]
    fn test_batch_agent_elapsed_not_started() {
        let agent = BatchAgent::new(AgentRole::Architect);
        assert_eq!(agent.elapsed(), Duration::ZERO);
    }

    // --- AgentPool tests ---

    #[test]
    fn test_agent_pool_new() {
        let pool = AgentPool::new(5);
        assert_eq!(pool.max_concurrent, 5);
        assert!(pool.agents.is_empty());
    }

    #[test]
    fn test_agent_pool_spawn_agent() {
        let mut pool = AgentPool::new(5);
        let agent = pool.spawn_agent(AgentRole::Backend);
        assert_eq!(agent.role, AgentRole::Backend);
        assert_eq!(pool.agents.len(), 1);
    }

    #[test]
    fn test_agent_pool_spawn_standard_team() {
        let mut pool = AgentPool::new(10);
        let ids = pool.spawn_standard_team();
        assert_eq!(ids.len(), 10);
        assert_eq!(pool.agents.len(), 10);
    }

    #[test]
    fn test_agent_pool_get_agent() {
        let mut pool = AgentPool::new(5);
        let agent = pool.spawn_agent(AgentRole::Security);
        let id = agent.id.clone();
        assert!(pool.get_agent(&id).is_some());
        assert!(pool.get_agent("nonexistent").is_none());
    }

    #[test]
    fn test_agent_pool_active_agents() {
        let mut pool = AgentPool::new(5);
        pool.spawn_agent(AgentRole::Backend);
        pool.spawn_agent(AgentRole::Frontend);
        pool.agents[0].start();
        assert_eq!(pool.active_agents().len(), 1);
    }

    #[test]
    fn test_agent_pool_completed_agents() {
        let mut pool = AgentPool::new(5);
        pool.spawn_agent(AgentRole::Backend);
        pool.agents[0].start();
        pool.agents[0].complete();
        assert_eq!(pool.completed_agents().len(), 1);
    }

    #[test]
    fn test_agent_pool_can_spawn() {
        let mut pool = AgentPool::new(1);
        assert!(pool.can_spawn());
        pool.active_count = 1;
        assert!(!pool.can_spawn());
    }

    #[test]
    fn test_agent_pool_total_lines_and_files() {
        let mut pool = AgentPool::new(5);
        pool.spawn_agent(AgentRole::Backend);
        pool.spawn_agent(AgentRole::Frontend);
        pool.agents[0].add_generated_file(PathBuf::from("a.rs"), 100);
        pool.agents[1].add_generated_file(PathBuf::from("b.tsx"), 200);
        assert_eq!(pool.total_lines_generated(), 300);
        assert_eq!(pool.total_files_created(), 2);
    }

    #[test]
    fn test_agent_pool_progress_empty() {
        let pool = AgentPool::new(5);
        assert_eq!(pool.progress_percentage(), 0.0);
    }

    #[test]
    fn test_agent_pool_progress_partial() {
        let mut pool = AgentPool::new(5);
        pool.spawn_agent(AgentRole::Backend);
        pool.spawn_agent(AgentRole::Frontend);
        pool.agents[0].start();
        pool.agents[0].complete();
        assert!((pool.progress_percentage() - 50.0).abs() < 0.01);
    }

    // --- GeneratedFile tests ---

    #[test]
    fn test_generated_file_new() {
        let f = GeneratedFile::new(
            PathBuf::from("src/main.rs"),
            "fn main() {\n    println!(\"hello\");\n}\n",
            "agent-1",
            AgentRole::Backend,
            GenerationPhase::CodeGeneration,
        );
        assert_eq!(f.lines, 3);
        assert!(!f.validated);
        assert!(!f.compile_checked);
    }

    #[test]
    fn test_generated_file_mark_validated() {
        let mut f = GeneratedFile::new(
            PathBuf::from("a.rs"),
            "code",
            "a1",
            AgentRole::Backend,
            GenerationPhase::CodeGeneration,
        );
        f.mark_validated();
        assert!(f.validated);
    }

    #[test]
    fn test_generated_file_mark_compile_checked() {
        let mut f = GeneratedFile::new(
            PathBuf::from("a.rs"),
            "code",
            "a1",
            AgentRole::Backend,
            GenerationPhase::CodeGeneration,
        );
        f.mark_compile_checked();
        assert!(f.compile_checked);
    }

    // --- GenerationMetrics tests ---

    #[test]
    fn test_generation_metrics_new() {
        let m = GenerationMetrics::new();
        assert_eq!(m.total_files, 0);
        assert_eq!(m.total_lines, 0);
    }

    #[test]
    fn test_generation_metrics_update_from_files() {
        let mut m = GenerationMetrics::new();
        let files = vec![
            GeneratedFile::new(PathBuf::from("a.rs"), "line1\nline2", "a1", AgentRole::Backend, GenerationPhase::CodeGeneration),
            GeneratedFile::new(PathBuf::from("b.rs"), "line1", "a2", AgentRole::Frontend, GenerationPhase::CodeGeneration),
        ];
        m.update_from_files(&files);
        assert_eq!(m.total_files, 2);
        assert_eq!(m.total_lines, 3);
        assert_eq!(*m.files_by_role.get("Backend").unwrap(), 1);
        assert_eq!(*m.files_by_role.get("Frontend").unwrap(), 1);
    }

    #[test]
    fn test_generation_metrics_update_phase() {
        let mut m = GenerationMetrics::new();
        m.update_phase(GenerationPhase::CodeGeneration);
        assert_eq!(m.current_phase, GenerationPhase::CodeGeneration);
        assert_eq!(m.phases_completed.len(), 1);
    }

    #[test]
    fn test_generation_metrics_lines_per_hour_zero_elapsed() {
        let m = GenerationMetrics::new();
        assert_eq!(m.lines_per_hour(), 0.0);
    }

    #[test]
    fn test_generation_metrics_lines_per_hour() {
        let mut m = GenerationMetrics::new();
        m.total_lines = 10000;
        m.elapsed = Duration::from_secs(3600); // 1 hour
        assert!((m.lines_per_hour() - 10000.0).abs() < 0.1);
    }

    #[test]
    fn test_generation_metrics_files_per_hour() {
        let mut m = GenerationMetrics::new();
        m.total_files = 60;
        m.elapsed = Duration::from_secs(3600);
        assert!((m.files_per_hour() - 60.0).abs() < 0.1);
    }

    #[test]
    fn test_generation_metrics_estimated_total_time() {
        let mut m = GenerationMetrics::new();
        m.total_lines = 5000;
        m.elapsed = Duration::from_secs(3600);
        let est = m.estimated_total_time(10000);
        // should be roughly 2 hours
        assert!(est.as_secs() > 7000 && est.as_secs() < 7400);
    }

    // --- ArchitecturePlan tests ---

    #[test]
    fn test_architecture_plan_new() {
        let plan = ArchitecturePlan::new("Overview");
        assert_eq!(plan.system_overview, "Overview");
        assert!(plan.modules.is_empty());
    }

    #[test]
    fn test_architecture_plan_add_module() {
        let mut plan = ArchitecturePlan::new("Overview");
        plan.add_module(ModulePlan {
            name: "api".to_string(),
            description: "API module".to_string(),
            responsibility: "Handle requests".to_string(),
            assigned_agent: AgentRole::Backend,
            estimated_files: 10,
            estimated_lines: 1000,
            dependencies: Vec::new(),
        });
        assert_eq!(plan.modules.len(), 1);
    }

    #[test]
    fn test_architecture_plan_dependency_count() {
        let mut plan = ArchitecturePlan::new("Overview");
        plan.add_dependency("api", "core");
        plan.add_dependency("api", "auth");
        plan.add_dependency("frontend", "api");
        assert_eq!(plan.dependency_count("api"), 2);
        assert_eq!(plan.dependency_count("frontend"), 1);
        assert_eq!(plan.dependency_count("core"), 0);
    }

    #[test]
    fn test_architecture_plan_topological_order_simple() {
        let mut plan = ArchitecturePlan::new("Overview");
        plan.add_module(ModulePlan {
            name: "database".to_string(),
            description: "".to_string(),
            responsibility: "".to_string(),
            assigned_agent: AgentRole::Database,
            estimated_files: 1,
            estimated_lines: 100,
            dependencies: Vec::new(),
        });
        plan.add_module(ModulePlan {
            name: "core".to_string(),
            description: "".to_string(),
            responsibility: "".to_string(),
            assigned_agent: AgentRole::Backend,
            estimated_files: 1,
            estimated_lines: 100,
            dependencies: vec!["database".to_string()],
        });
        plan.add_module(ModulePlan {
            name: "api".to_string(),
            description: "".to_string(),
            responsibility: "".to_string(),
            assigned_agent: AgentRole::Backend,
            estimated_files: 1,
            estimated_lines: 100,
            dependencies: vec!["core".to_string()],
        });
        plan.add_dependency("core", "database");
        plan.add_dependency("api", "core");

        let order = plan.topological_order();
        assert_eq!(order.len(), 3);
        // database must come before core, core before api
        let db_pos = order.iter().position(|n| n == "database").unwrap();
        let core_pos = order.iter().position(|n| n == "core").unwrap();
        let api_pos = order.iter().position(|n| n == "api").unwrap();
        assert!(db_pos < core_pos);
        assert!(core_pos < api_pos);
    }

    #[test]
    fn test_architecture_plan_topological_order_no_deps() {
        let mut plan = ArchitecturePlan::new("Overview");
        for name in &["alpha", "bravo", "charlie"] {
            plan.add_module(ModulePlan {
                name: name.to_string(),
                description: "".to_string(),
                responsibility: "".to_string(),
                assigned_agent: AgentRole::Backend,
                estimated_files: 1,
                estimated_lines: 100,
                dependencies: Vec::new(),
            });
        }
        let order = plan.topological_order();
        assert_eq!(order.len(), 3);
    }

    // --- BatchRun tests ---

    #[test]
    fn test_batch_run_new() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let run = BatchRun::new(spec, &config);
        assert_eq!(run.status, BatchStatus::Queued);
        assert!(run.started_at.is_none());
    }

    #[test]
    fn test_batch_run_start() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let mut run = BatchRun::new(spec, &config);
        run.start();
        assert_eq!(run.status, BatchStatus::Planning);
        assert!(run.started_at.is_some());
    }

    #[test]
    fn test_batch_run_pause_resume() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let mut run = BatchRun::new(spec, &config);
        run.start();
        run.pause("need review");
        assert_eq!(run.status, BatchStatus::Paused);
        assert_eq!(run.pause_reason.as_deref(), Some("need review"));
        run.resume();
        assert_eq!(run.status, BatchStatus::Generating);
        assert!(run.pause_reason.is_none());
    }

    #[test]
    fn test_batch_run_cancel() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let mut run = BatchRun::new(spec, &config);
        run.start();
        run.cancel();
        assert_eq!(run.status, BatchStatus::Cancelled);
        assert!(run.completed_at.is_some());
    }

    #[test]
    fn test_batch_run_complete() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let mut run = BatchRun::new(spec, &config);
        run.start();
        run.complete();
        assert_eq!(run.status, BatchStatus::Completed);
    }

    #[test]
    fn test_batch_run_fail() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let mut run = BatchRun::new(spec, &config);
        run.start();
        run.fail("out of memory");
        assert_eq!(run.status, BatchStatus::Failed("out of memory".to_string()));
    }

    #[test]
    fn test_batch_run_add_log() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let mut run = BatchRun::new(spec, &config);
        run.add_log(LogLevel::Info, GenerationPhase::CodeGeneration, "started gen", Some("a1"));
        run.add_log(LogLevel::Error, GenerationPhase::CompileValidation, "compile fail", None);
        assert_eq!(run.logs.len(), 2);
    }

    #[test]
    fn test_batch_run_logs_by_level() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let mut run = BatchRun::new(spec, &config);
        run.add_log(LogLevel::Info, GenerationPhase::CodeGeneration, "info1", None);
        run.add_log(LogLevel::Error, GenerationPhase::CodeGeneration, "err1", None);
        run.add_log(LogLevel::Info, GenerationPhase::CodeGeneration, "info2", None);
        assert_eq!(run.logs_by_level(&LogLevel::Info).len(), 2);
        assert_eq!(run.logs_by_level(&LogLevel::Error).len(), 1);
        assert_eq!(run.logs_by_level(&LogLevel::Warning).len(), 0);
    }

    #[test]
    fn test_batch_run_add_generated_file() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let mut run = BatchRun::new(spec, &config);
        let file = GeneratedFile::new(
            PathBuf::from("src/main.rs"),
            "fn main() {}\n",
            "a1",
            AgentRole::Backend,
            GenerationPhase::CodeGeneration,
        );
        run.add_generated_file(file);
        assert_eq!(run.generated_files.len(), 1);
        assert_eq!(run.metrics.total_files, 1);
    }

    #[test]
    fn test_batch_run_files_by_phase() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let mut run = BatchRun::new(spec, &config);
        run.add_generated_file(GeneratedFile::new(
            PathBuf::from("a.rs"), "code", "a1", AgentRole::Backend, GenerationPhase::CodeGeneration,
        ));
        run.add_generated_file(GeneratedFile::new(
            PathBuf::from("t.rs"), "test", "a1", AgentRole::Testing, GenerationPhase::TestGeneration,
        ));
        assert_eq!(run.files_by_phase(&GenerationPhase::CodeGeneration).len(), 1);
        assert_eq!(run.files_by_phase(&GenerationPhase::TestGeneration).len(), 1);
    }

    #[test]
    fn test_batch_run_files_by_role() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let mut run = BatchRun::new(spec, &config);
        run.add_generated_file(GeneratedFile::new(
            PathBuf::from("a.rs"), "code", "a1", AgentRole::Backend, GenerationPhase::CodeGeneration,
        ));
        run.add_generated_file(GeneratedFile::new(
            PathBuf::from("b.rs"), "more", "a2", AgentRole::Backend, GenerationPhase::CodeGeneration,
        ));
        assert_eq!(run.files_by_role(&AgentRole::Backend).len(), 2);
        assert_eq!(run.files_by_role(&AgentRole::Frontend).len(), 0);
    }

    #[test]
    fn test_batch_run_is_within_time_budget() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let run = BatchRun::new(spec, &config);
        // Not started yet, elapsed is zero
        assert!(run.is_within_time_budget());
    }

    #[test]
    fn test_batch_run_to_summary() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let mut run = BatchRun::new(spec, &config);
        run.start();
        let summary = run.to_summary();
        assert_eq!(summary.title, "Test App");
        assert_eq!(summary.status, BatchStatus::Planning);
    }

    #[test]
    fn test_batch_run_elapsed_not_started() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let run = BatchRun::new(spec, &config);
        assert_eq!(run.elapsed(), Duration::ZERO);
    }

    // --- BatchConfig tests ---

    #[test]
    fn test_batch_config_default() {
        let c = BatchConfig::default_config();
        assert_eq!(c.max_concurrent_agents, 10);
        assert_eq!(c.max_duration_hours, 12);
        assert!(c.compile_check_enabled);
        assert_eq!(c.max_retries, 3);
    }

    #[test]
    fn test_batch_config_high_performance() {
        let c = BatchConfig::high_performance();
        assert_eq!(c.max_concurrent_agents, 20);
        assert_eq!(c.max_duration_hours, 24);
        assert_eq!(c.max_retries, 5);
    }

    #[test]
    fn test_batch_config_conservative() {
        let c = BatchConfig::conservative();
        assert_eq!(c.max_concurrent_agents, 4);
        assert!(!c.auto_retry_on_failure);
        assert_eq!(c.max_retries, 1);
    }

    // --- BatchBuilder tests ---

    #[test]
    fn test_batch_builder_new() {
        let bb = BatchBuilder::new();
        assert!(bb.runs.is_empty());
        assert!(bb.active_run.is_none());
    }

    #[test]
    fn test_batch_builder_with_config() {
        let config = BatchConfig::high_performance();
        let bb = BatchBuilder::with_config(config);
        assert_eq!(bb.config.max_concurrent_agents, 20);
    }

    #[test]
    fn test_batch_builder_create_run() {
        let mut bb = BatchBuilder::new();
        let spec = make_valid_spec();
        let result = bb.create_run(spec);
        assert!(result.is_ok());
        assert_eq!(bb.runs.len(), 1);
    }

    #[test]
    fn test_batch_builder_create_run_invalid_spec() {
        let mut bb = BatchBuilder::new();
        let spec = BatchSpec::new("", "Desc", TechStack::GoGin);
        let result = bb.create_run(spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_builder_start_run() {
        let mut bb = BatchBuilder::new();
        let spec = make_valid_spec();
        let id = bb.create_run(spec).unwrap();
        assert!(bb.start_run(&id).is_ok());
        assert_eq!(bb.active_run, Some(id.clone()));
        assert_eq!(bb.runs[0].status, BatchStatus::Planning);
    }

    #[test]
    fn test_batch_builder_start_run_already_active() {
        let mut bb = BatchBuilder::new();
        let id1 = bb.create_run(make_valid_spec()).unwrap();
        let id2 = bb.create_run(make_valid_spec()).unwrap();
        bb.start_run(&id1).unwrap();
        let result = bb.start_run(&id2);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already active"));
    }

    #[test]
    fn test_batch_builder_start_run_not_found() {
        let mut bb = BatchBuilder::new();
        assert!(bb.start_run("nonexistent").is_err());
    }

    #[test]
    fn test_batch_builder_pause_run() {
        let mut bb = BatchBuilder::new();
        let id = bb.create_run(make_valid_spec()).unwrap();
        bb.start_run(&id).unwrap();
        assert!(bb.pause_run(&id, "review needed").is_ok());
        assert_eq!(bb.runs[0].status, BatchStatus::Paused);
    }

    #[test]
    fn test_batch_builder_pause_run_wrong_state() {
        let mut bb = BatchBuilder::new();
        let id = bb.create_run(make_valid_spec()).unwrap();
        // Still queued, can't pause
        let result = bb.pause_run(&id, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_builder_resume_run() {
        let mut bb = BatchBuilder::new();
        let id = bb.create_run(make_valid_spec()).unwrap();
        bb.start_run(&id).unwrap();
        bb.pause_run(&id, "test").unwrap();
        assert!(bb.resume_run(&id).is_ok());
        assert_eq!(bb.runs[0].status, BatchStatus::Generating);
    }

    #[test]
    fn test_batch_builder_resume_run_not_paused() {
        let mut bb = BatchBuilder::new();
        let id = bb.create_run(make_valid_spec()).unwrap();
        bb.start_run(&id).unwrap();
        let result = bb.resume_run(&id);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_builder_cancel_run() {
        let mut bb = BatchBuilder::new();
        let id = bb.create_run(make_valid_spec()).unwrap();
        bb.start_run(&id).unwrap();
        assert!(bb.cancel_run(&id).is_ok());
        assert_eq!(bb.runs[0].status, BatchStatus::Cancelled);
        assert!(bb.active_run.is_none());
    }

    #[test]
    fn test_batch_builder_cancel_already_finished() {
        let mut bb = BatchBuilder::new();
        let id = bb.create_run(make_valid_spec()).unwrap();
        bb.start_run(&id).unwrap();
        bb.cancel_run(&id).unwrap();
        let result = bb.cancel_run(&id);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_builder_get_run() {
        let mut bb = BatchBuilder::new();
        let id = bb.create_run(make_valid_spec()).unwrap();
        assert!(bb.get_run(&id).is_some());
        assert!(bb.get_run("nope").is_none());
    }

    #[test]
    fn test_batch_builder_get_run_mut() {
        let mut bb = BatchBuilder::new();
        let id = bb.create_run(make_valid_spec()).unwrap();
        let run = bb.get_run_mut(&id).unwrap();
        run.start();
        assert_eq!(bb.runs[0].status, BatchStatus::Planning);
    }

    #[test]
    fn test_batch_builder_active_run() {
        let mut bb = BatchBuilder::new();
        assert!(bb.active_run().is_none());
        let id = bb.create_run(make_valid_spec()).unwrap();
        bb.start_run(&id).unwrap();
        assert!(bb.active_run().is_some());
    }

    #[test]
    fn test_batch_builder_estimate_run() {
        let bb = BatchBuilder::new();
        let spec = make_valid_spec();
        let est = bb.estimate_run(&spec);
        assert!(est.estimated_files > 0);
        assert!(est.estimated_lines > 0);
        assert!(est.complexity_score > 0);
        assert!(est.tech_stack_support);
    }

    #[test]
    fn test_batch_builder_estimate_custom_stack_warning() {
        let bb = BatchBuilder::new();
        let mut spec = make_valid_spec();
        spec.tech_stack = TechStack::Custom("Elixir+Phoenix".to_string());
        let est = bb.estimate_run(&spec);
        assert!(!est.tech_stack_support);
        assert!(est.warnings.iter().any(|w| w.contains("Custom tech stack")));
    }

    #[test]
    fn test_batch_builder_generate_architecture() {
        let mut bb = BatchBuilder::new();
        let id = bb.create_run(make_valid_spec()).unwrap();
        let plan = bb.generate_architecture(&id).unwrap();
        assert!(!plan.system_overview.is_empty());
        assert!(!plan.modules.is_empty());
    }

    #[test]
    fn test_batch_builder_decompose_modules() {
        let bb = BatchBuilder::new();
        let mut plan = ArchitecturePlan::new("test");
        plan.add_module(ModulePlan {
            name: "core".to_string(),
            description: "".to_string(),
            responsibility: "".to_string(),
            assigned_agent: AgentRole::Backend,
            estimated_files: 5,
            estimated_lines: 500,
            dependencies: Vec::new(),
        });
        let spec = make_valid_spec();
        let nodes = bb.decompose_modules(&plan, &spec);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].name, "core");
        assert_eq!(nodes[0].status, BatchStatus::Queued);
    }

    #[test]
    fn test_batch_builder_cleanup_completed() {
        let mut bb = BatchBuilder::new();
        let id = bb.create_run(make_valid_spec()).unwrap();
        bb.start_run(&id).unwrap();
        bb.runs[0].complete();
        bb.active_run = None;
        bb.cleanup_completed();
        assert!(bb.runs.is_empty());
        assert_eq!(bb.history.len(), 1);
    }

    #[test]
    fn test_batch_builder_total_lines_all_time() {
        let mut bb = BatchBuilder::new();
        bb.history.push(BatchRunSummary {
            id: "old".to_string(),
            title: "Old Run".to_string(),
            status: BatchStatus::Completed,
            total_files: 10,
            total_lines: 5000,
            duration: Duration::from_secs(3600),
            agents_used: 5,
            completed_at: None,
        });
        let _id = bb.create_run(make_valid_spec()).unwrap();
        bb.runs[0].add_generated_file(GeneratedFile::new(
            PathBuf::from("a.rs"),
            "line1\nline2\nline3",
            "a1",
            AgentRole::Backend,
            GenerationPhase::CodeGeneration,
        ));
        assert_eq!(bb.total_lines_generated_all_time(), 5003);
    }

    #[test]
    fn test_batch_builder_run_history() {
        let mut bb = BatchBuilder::new();
        assert!(bb.run_history().is_empty());
        bb.history.push(BatchRunSummary {
            id: "h1".to_string(),
            title: "First".to_string(),
            status: BatchStatus::Completed,
            total_files: 1,
            total_lines: 100,
            duration: Duration::from_secs(60),
            agents_used: 1,
            completed_at: None,
        });
        assert_eq!(bb.run_history().len(), 1);
    }

    // --- TechStack tests ---

    #[test]
    fn test_tech_stack_display_name() {
        assert_eq!(TechStack::ReactNode.display_name(), "React + Node.js + PostgreSQL");
        assert_eq!(TechStack::GoGin.display_name(), "Go + Gin + PostgreSQL");
        assert_eq!(TechStack::Custom("X".to_string()).display_name(), "Custom Stack");
    }

    #[test]
    fn test_tech_stack_frontend_framework() {
        assert_eq!(TechStack::ReactNode.frontend_framework(), Some("React"));
        assert_eq!(TechStack::VueNode.frontend_framework(), Some("Vue"));
        assert_eq!(TechStack::AngularJava.frontend_framework(), Some("Angular"));
        assert_eq!(TechStack::RustActix.frontend_framework(), None);
    }

    #[test]
    fn test_tech_stack_backend_framework() {
        assert_eq!(TechStack::RustActix.backend_framework(), Some("Actix-web"));
        assert_eq!(TechStack::GoGin.backend_framework(), Some("Gin"));
        assert_eq!(TechStack::DjangoHtmx.backend_framework(), Some("Django"));
        assert_eq!(TechStack::Custom("X".to_string()).backend_framework(), None);
    }

    #[test]
    fn test_tech_stack_database() {
        assert_eq!(TechStack::ReactNode.database(), Some("PostgreSQL"));
        assert_eq!(TechStack::VueNode.database(), Some("MongoDB"));
        assert_eq!(TechStack::FlutterFirebase.database(), Some("Firestore"));
        assert_eq!(TechStack::Custom("X".to_string()).database(), None);
    }

    #[test]
    fn test_tech_stack_language() {
        assert_eq!(TechStack::ReactNode.language(), "TypeScript");
        assert_eq!(TechStack::RustActix.language(), "Rust");
        assert_eq!(TechStack::GoGin.language(), "Go");
        assert_eq!(TechStack::RailsPostgres.language(), "Ruby");
        assert_eq!(TechStack::FlutterFirebase.language(), "Dart");
        assert_eq!(TechStack::ReactPython.language(), "Python");
        assert_eq!(TechStack::AngularJava.language(), "Java");
    }

    #[test]
    fn test_tech_stack_default_port() {
        assert_eq!(TechStack::ReactNode.default_port(), 3000);
        assert_eq!(TechStack::RustActix.default_port(), 8080);
        assert_eq!(TechStack::ReactPython.default_port(), 8000);
        assert_eq!(TechStack::FlutterFirebase.default_port(), 5000);
    }

    // --- Edge case tests ---

    #[test]
    fn test_batch_spec_complexity_capped_at_100() {
        let mut spec = BatchSpec::new("Big", "Huge app", TechStack::ReactNode);
        for i in 0..50 {
            spec.add_requirement(&format!("req {}", i));
            spec.add_api_endpoint(ApiEndpoint::new("GET", &format!("/api/{}", i), "ep"));
            spec.add_data_model(DataModel::new(&format!("Model{}", i)));
            spec.add_ui_component(UiComponent::new(&format!("Comp{}", i), "page", "desc"));
            let mut story = UserStory::new("user", "act", "benefit");
            story.estimated_complexity = 10;
            spec.add_user_story(story);
        }
        assert!(spec.estimated_complexity() <= 100);
    }

    #[test]
    fn test_generated_file_empty_content() {
        let f = GeneratedFile::new(
            PathBuf::from("empty.rs"),
            "",
            "a1",
            AgentRole::Backend,
            GenerationPhase::CodeGeneration,
        );
        assert_eq!(f.lines, 0);
    }

    #[test]
    fn test_batch_run_set_architecture() {
        let spec = make_valid_spec();
        let config = BatchConfig::default_config();
        let mut run = BatchRun::new(spec, &config);
        assert!(run.architecture_plan.is_none());
        run.set_architecture(ArchitecturePlan::new("test arch"));
        assert!(run.architecture_plan.is_some());
    }

    #[test]
    fn test_batch_builder_assign_agents() {
        let mut bb = BatchBuilder::new();
        let id = bb.create_run(make_valid_spec()).unwrap();
        bb.start_run(&id).unwrap();
        let plan = bb.generate_architecture(&id).unwrap();
        let modules = bb.decompose_modules(&plan, &bb.runs[0].spec.clone());
        bb.runs[0].set_architecture(plan);
        bb.runs[0].module_graph = modules;
        assert!(bb.assign_agents(&id).is_ok());
        assert!(!bb.runs[0].agent_pool.agents.is_empty());
    }

    #[test]
    fn test_batch_builder_assign_agents_no_plan() {
        let mut bb = BatchBuilder::new();
        let id = bb.create_run(make_valid_spec()).unwrap();
        let result = bb.assign_agents(&id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No architecture plan"));
    }

    #[test]
    fn test_batch_status_equality() {
        assert_eq!(BatchStatus::Queued, BatchStatus::Queued);
        assert_ne!(BatchStatus::Queued, BatchStatus::Planning);
        assert_eq!(
            BatchStatus::Failed("x".to_string()),
            BatchStatus::Failed("x".to_string())
        );
        assert_ne!(
            BatchStatus::Failed("x".to_string()),
            BatchStatus::Failed("y".to_string())
        );
    }

    #[test]
    fn test_log_level_equality() {
        assert_eq!(LogLevel::Debug, LogLevel::Debug);
        assert_ne!(LogLevel::Debug, LogLevel::Info);
    }

    #[test]
    fn test_generation_phase_equality() {
        assert_eq!(GenerationPhase::CodeGeneration, GenerationPhase::CodeGeneration);
        assert_ne!(GenerationPhase::CodeGeneration, GenerationPhase::TestGeneration);
    }
}
