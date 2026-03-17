use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceTier {
    Gold,
    Silver,
    Bronze,
    Experimental,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Active,
    Deprecated,
    InDevelopment,
    Maintenance,
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScorecardMetric {
    Reliability,
    Security,
    Documentation,
    TestCoverage,
    DeployFrequency,
    LeadTime,
    Mttr,
    ChangeFailureRate,
    CodeQuality,
    Ownership,
    Observability,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InfraTemplate {
    Database,
    Cache,
    MessageQueue,
    ObjectStorage,
    Cdn,
    LoadBalancer,
    Dns,
    Monitoring,
    Logging,
    SecretStore,
    ServiceMesh,
    ApiGateway,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OnboardingStep {
    RepoSetup,
    CiPipeline,
    Environments,
    AccessControl,
    Documentation,
    Monitoring,
    Alerting,
    ServiceCatalog,
    GoldenPath,
    SecurityBaseline,
}

impl OnboardingStep {
    fn all() -> Vec<OnboardingStep> {
        vec![
            OnboardingStep::RepoSetup,
            OnboardingStep::CiPipeline,
            OnboardingStep::Environments,
            OnboardingStep::AccessControl,
            OnboardingStep::Documentation,
            OnboardingStep::Monitoring,
            OnboardingStep::Alerting,
            OnboardingStep::ServiceCatalog,
            OnboardingStep::GoldenPath,
            OnboardingStep::SecurityBaseline,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InfraStatus {
    Requested,
    Provisioning,
    Active,
    Failed,
    Decommissioned,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IdpPlatform {
    Backstage,
    Cycloid,
    Humanitec,
    Port,
    Qovery,
    MiaPlatform,
    OpsLevel,
    Roadie,
    Cortex,
    MorpheusData,
    CloudBolt,
    Harness,
    Custom,
}

impl IdpPlatform {
    fn all() -> Vec<IdpPlatform> {
        vec![
            IdpPlatform::Backstage,
            IdpPlatform::Cycloid,
            IdpPlatform::Humanitec,
            IdpPlatform::Port,
            IdpPlatform::Qovery,
            IdpPlatform::MiaPlatform,
            IdpPlatform::OpsLevel,
            IdpPlatform::Roadie,
            IdpPlatform::Cortex,
            IdpPlatform::MorpheusData,
            IdpPlatform::CloudBolt,
            IdpPlatform::Harness,
            IdpPlatform::Custom,
        ]
    }

    fn name(&self) -> &str {
        match self {
            IdpPlatform::Backstage => "Backstage",
            IdpPlatform::Cycloid => "Cycloid",
            IdpPlatform::Humanitec => "Humanitec",
            IdpPlatform::Port => "Port",
            IdpPlatform::Qovery => "Qovery",
            IdpPlatform::MiaPlatform => "Mia Platform",
            IdpPlatform::OpsLevel => "OpsLevel",
            IdpPlatform::Roadie => "Roadie",
            IdpPlatform::Cortex => "Cortex",
            IdpPlatform::MorpheusData => "Morpheus Data",
            IdpPlatform::CloudBolt => "CloudBolt",
            IdpPlatform::Harness => "Harness",
            IdpPlatform::Custom => "Custom",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PluginCategory {
    ServiceCatalog,
    CiCd,
    Monitoring,
    Security,
    CostManagement,
    Documentation,
    Scaffolding,
    Search,
    Analytics,
    Governance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServiceEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub owner_team: String,
    pub tier: ServiceTier,
    pub status: ServiceStatus,
    pub repository: String,
    pub docs_url: Option<String>,
    pub api_spec_url: Option<String>,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub language: String,
    pub framework: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GoldenPath {
    pub id: String,
    pub name: String,
    pub language: String,
    pub framework: String,
    pub template_repo: String,
    pub includes: Vec<String>,
    pub description: String,
    pub recommended_for: Vec<String>,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScorecardResult {
    pub service_id: String,
    pub scores: HashMap<ScorecardMetric, f64>,
    pub overall_score: f64,
    pub evaluated_at: String,
    pub recommendations: Vec<String>,
    pub grade: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InfraRequest {
    pub id: String,
    pub template: InfraTemplate,
    pub service_id: String,
    pub config: HashMap<String, String>,
    pub requested_by: String,
    pub status: InfraStatus,
    pub provisioned_at: Option<String>,
    pub cloud_provider: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TeamProfile {
    pub id: String,
    pub name: String,
    pub members: Vec<String>,
    pub owned_services: Vec<String>,
    pub slack_channel: Option<String>,
    pub on_call_schedule: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnboardingChecklist {
    pub team_id: String,
    pub steps: Vec<(OnboardingStep, bool)>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub mentor: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoraMetrics {
    pub deploy_frequency_per_week: f64,
    pub lead_time_hours: f64,
    pub mttr_hours: f64,
    pub change_failure_rate_pct: f64,
    pub rating: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdpPlatformConfig {
    pub platform: IdpPlatform,
    pub base_url: Option<String>,
    pub api_key_env: Option<String>,
    pub features: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackstageComponent {
    pub kind: String,
    pub name: String,
    pub namespace: String,
    pub owner: String,
    pub system: String,
    pub lifecycle: String,
    pub spec_type: String,
    pub annotations: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackstageTemplate {
    pub name: String,
    pub title: String,
    pub description: String,
    pub owner: String,
    pub steps: Vec<BackstageTemplateStep>,
    pub parameters: Vec<BackstageParameter>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackstageTemplateStep {
    pub id: String,
    pub name: String,
    pub action: String,
    pub input: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackstageParameter {
    pub name: String,
    pub title: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdpPlugin {
    pub id: String,
    pub name: String,
    pub category: PluginCategory,
    pub platform: IdpPlatform,
    pub version: String,
    pub description: String,
    pub installed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfServiceAction {
    pub id: String,
    pub name: String,
    pub description: String,
    pub template: InfraTemplate,
    pub required_params: Vec<String>,
    pub approval_required: bool,
    pub estimated_time_secs: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdpManager {
    pub catalog: Vec<ServiceEntry>,
    pub golden_paths: Vec<GoldenPath>,
    pub scorecards: Vec<ScorecardResult>,
    pub infra_requests: Vec<InfraRequest>,
    pub teams: Vec<TeamProfile>,
    pub onboarding_checklists: Vec<OnboardingChecklist>,
    pub platform_configs: Vec<IdpPlatformConfig>,
    pub plugins: Vec<IdpPlugin>,
    pub backstage_components: Vec<BackstageComponent>,
    pub backstage_templates: Vec<BackstageTemplate>,
    pub self_service_actions: Vec<SelfServiceAction>,
}

impl IdpManager {
    pub fn new() -> Self {
        let platform_configs: Vec<IdpPlatformConfig> = IdpPlatform::all()
            .into_iter()
            .map(|p| {
                let features = match &p {
                    IdpPlatform::Backstage => vec![
                        "service-catalog".to_string(),
                        "software-templates".to_string(),
                        "techdocs".to_string(),
                        "search".to_string(),
                    ],
                    IdpPlatform::Cycloid => vec![
                        "stacks".to_string(),
                        "infracost".to_string(),
                        "environments".to_string(),
                    ],
                    IdpPlatform::Humanitec => vec![
                        "score".to_string(),
                        "resources".to_string(),
                        "deployments".to_string(),
                    ],
                    IdpPlatform::Port => vec![
                        "blueprints".to_string(),
                        "self-service".to_string(),
                        "scorecards".to_string(),
                    ],
                    IdpPlatform::Qovery => vec![
                        "environments".to_string(),
                        "preview-envs".to_string(),
                        "deployments".to_string(),
                    ],
                    IdpPlatform::MiaPlatform => vec![
                        "marketplace".to_string(),
                        "console".to_string(),
                        "fast-data".to_string(),
                    ],
                    IdpPlatform::OpsLevel => vec![
                        "service-maturity".to_string(),
                        "checks".to_string(),
                        "rubrics".to_string(),
                    ],
                    IdpPlatform::Roadie => vec![
                        "backstage-hosted".to_string(),
                        "plugins".to_string(),
                        "catalog".to_string(),
                    ],
                    IdpPlatform::Cortex => vec![
                        "scorecards".to_string(),
                        "catalog".to_string(),
                        "initiatives".to_string(),
                    ],
                    IdpPlatform::MorpheusData => vec![
                        "provisioning".to_string(),
                        "governance".to_string(),
                        "analytics".to_string(),
                    ],
                    IdpPlatform::CloudBolt => vec![
                        "cloud-management".to_string(),
                        "cost-optimization".to_string(),
                        "self-service".to_string(),
                    ],
                    IdpPlatform::Harness => vec![
                        "ci".to_string(),
                        "cd".to_string(),
                        "feature-flags".to_string(),
                        "idp".to_string(),
                    ],
                    IdpPlatform::Custom => vec!["custom".to_string()],
                };
                IdpPlatformConfig {
                    platform: p,
                    base_url: None,
                    api_key_env: None,
                    features,
                    enabled: false,
                }
            })
            .collect();

        let golden_paths = vec![
            GoldenPath {
                id: "gp-rust-api".to_string(),
                name: "Rust API Service".to_string(),
                language: "rust".to_string(),
                framework: "actix-web".to_string(),
                template_repo: "org/golden-rust-api".to_string(),
                includes: vec![
                    "Dockerfile".to_string(),
                    "CI pipeline".to_string(),
                    "Observability".to_string(),
                    "Health checks".to_string(),
                ],
                description: "Production-ready Rust API with observability and CI/CD".to_string(),
                recommended_for: vec!["microservices".to_string(), "api".to_string()],
                version: "1.0.0".to_string(),
            },
            GoldenPath {
                id: "gp-ts-react".to_string(),
                name: "TypeScript React App".to_string(),
                language: "typescript".to_string(),
                framework: "react".to_string(),
                template_repo: "org/golden-ts-react".to_string(),
                includes: vec![
                    "Dockerfile".to_string(),
                    "CI pipeline".to_string(),
                    "Testing setup".to_string(),
                    "Storybook".to_string(),
                ],
                description: "React frontend with TypeScript, testing, and Storybook".to_string(),
                recommended_for: vec!["frontend".to_string(), "web-app".to_string()],
                version: "1.0.0".to_string(),
            },
            GoldenPath {
                id: "gp-python-ml".to_string(),
                name: "Python ML Service".to_string(),
                language: "python".to_string(),
                framework: "fastapi".to_string(),
                template_repo: "org/golden-python-ml".to_string(),
                includes: vec![
                    "Dockerfile".to_string(),
                    "Model serving".to_string(),
                    "Feature store".to_string(),
                    "Monitoring".to_string(),
                ],
                description: "ML service with FastAPI, model serving, and monitoring".to_string(),
                recommended_for: vec!["ml".to_string(), "data-science".to_string()],
                version: "1.0.0".to_string(),
            },
        ];

        let self_service_actions = vec![
            SelfServiceAction {
                id: "ssa-create-db".to_string(),
                name: "Create Database".to_string(),
                description: "Provision a managed database instance".to_string(),
                template: InfraTemplate::Database,
                required_params: vec![
                    "engine".to_string(),
                    "size".to_string(),
                    "region".to_string(),
                ],
                approval_required: true,
                estimated_time_secs: 300,
            },
            SelfServiceAction {
                id: "ssa-create-cache".to_string(),
                name: "Create Cache".to_string(),
                description: "Provision a Redis or Memcached cache cluster".to_string(),
                template: InfraTemplate::Cache,
                required_params: vec!["engine".to_string(), "size".to_string()],
                approval_required: false,
                estimated_time_secs: 120,
            },
            SelfServiceAction {
                id: "ssa-create-queue".to_string(),
                name: "Create Message Queue".to_string(),
                description: "Provision a message queue (SQS, RabbitMQ, Kafka)".to_string(),
                template: InfraTemplate::MessageQueue,
                required_params: vec!["type".to_string(), "throughput".to_string()],
                approval_required: false,
                estimated_time_secs: 180,
            },
        ];

        IdpManager {
            catalog: Vec::new(),
            golden_paths,
            scorecards: Vec::new(),
            infra_requests: Vec::new(),
            teams: Vec::new(),
            onboarding_checklists: Vec::new(),
            platform_configs,
            plugins: Vec::new(),
            backstage_components: Vec::new(),
            backstage_templates: Vec::new(),
            self_service_actions,
        }
    }

    pub fn register_service(
        &mut self,
        name: &str,
        description: &str,
        owner_team: &str,
        tier: ServiceTier,
        repo: &str,
        language: &str,
        framework: &str,
    ) -> String {
        let id = format!("svc-{}", self.catalog.len() + 1);
        let entry = ServiceEntry {
            id: id.clone(),
            name: name.to_string(),
            description: description.to_string(),
            owner_team: owner_team.to_string(),
            tier,
            status: ServiceStatus::InDevelopment,
            repository: repo.to_string(),
            docs_url: None,
            api_spec_url: None,
            dependencies: Vec::new(),
            tags: Vec::new(),
            created_at: "2026-03-14".to_string(),
            language: language.to_string(),
            framework: framework.to_string(),
        };
        self.catalog.push(entry);

        // Associate service with owning team
        if let Some(team) = self.teams.iter_mut().find(|t| t.name == owner_team) {
            team.owned_services.push(id.clone());
        }

        id
    }

    pub fn update_service_status(&mut self, id: &str, status: ServiceStatus) -> bool {
        if let Some(svc) = self.catalog.iter_mut().find(|s| s.id == id) {
            svc.status = status;
            true
        } else {
            false
        }
    }

    pub fn search_catalog(&self, query: &str) -> Vec<&ServiceEntry> {
        let q = query.to_lowercase();
        self.catalog
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&q)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
                    || s.owner_team.to_lowercase().contains(&q)
                    || s.description.to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn search_catalog_by_tag(&self, tag: &str) -> Vec<&ServiceEntry> {
        let t = tag.to_lowercase();
        self.catalog
            .iter()
            .filter(|s| s.tags.iter().any(|st| st.to_lowercase() == t))
            .collect()
    }

    pub fn get_service(&self, id: &str) -> Option<&ServiceEntry> {
        self.catalog.iter().find(|s| s.id == id)
    }

    pub fn add_golden_path(
        &mut self,
        name: &str,
        language: &str,
        framework: &str,
        template_repo: &str,
        includes: Vec<String>,
        description: &str,
    ) -> String {
        let id = format!("gp-{}", self.golden_paths.len() + 1);
        let gp = GoldenPath {
            id: id.clone(),
            name: name.to_string(),
            language: language.to_string(),
            framework: framework.to_string(),
            template_repo: template_repo.to_string(),
            includes,
            description: description.to_string(),
            recommended_for: Vec::new(),
            version: "1.0.0".to_string(),
        };
        self.golden_paths.push(gp);
        id
    }

    pub fn get_golden_paths_for_language(&self, language: &str) -> Vec<&GoldenPath> {
        let lang = language.to_lowercase();
        self.golden_paths
            .iter()
            .filter(|gp| gp.language.to_lowercase() == lang)
            .collect()
    }

    pub fn evaluate_scorecard(&mut self, service_id: &str) -> Option<ScorecardResult> {
        let service = self.catalog.iter().find(|s| s.id == service_id)?;

        let base_score = match service.tier {
            ServiceTier::Gold => 0.9,
            ServiceTier::Silver => 0.75,
            ServiceTier::Bronze => 0.55,
            ServiceTier::Experimental => 0.35,
        };

        let mut scores = HashMap::new();
        let metrics = vec![
            (ScorecardMetric::Reliability, base_score),
            (ScorecardMetric::Security, base_score * 0.95),
            (ScorecardMetric::Documentation, base_score * 0.85),
            (ScorecardMetric::TestCoverage, base_score * 0.90),
            (ScorecardMetric::DeployFrequency, base_score * 0.80),
            (ScorecardMetric::LeadTime, base_score * 0.88),
            (ScorecardMetric::Mttr, base_score * 0.92),
            (ScorecardMetric::ChangeFailureRate, base_score * 0.87),
            (ScorecardMetric::CodeQuality, base_score * 0.93),
            (ScorecardMetric::Ownership, base_score * 0.95),
            (ScorecardMetric::Observability, base_score * 0.82),
        ];

        let mut total = 0.0;
        for (metric, score) in &metrics {
            scores.insert(metric.clone(), *score);
            total += score;
        }
        let overall = total / metrics.len() as f64;

        let grade = if overall >= 0.9 {
            "A"
        } else if overall >= 0.8 {
            "B"
        } else if overall >= 0.7 {
            "C"
        } else if overall >= 0.6 {
            "D"
        } else {
            "F"
        }
        .to_string();

        let mut recommendations = Vec::new();
        for (metric, score) in &metrics {
            if *score < 0.7 {
                recommendations.push(format!(
                    "Improve {:?}: current score {:.0}%",
                    metric,
                    score * 100.0
                ));
            }
        }
        if recommendations.is_empty() {
            recommendations.push("All metrics are healthy.".to_string());
        }

        let result = ScorecardResult {
            service_id: service_id.to_string(),
            scores,
            overall_score: overall,
            evaluated_at: "2026-03-14".to_string(),
            recommendations,
            grade,
        };
        self.scorecards.push(result.clone());
        Some(result)
    }

    pub fn request_infrastructure(
        &mut self,
        template: InfraTemplate,
        service_id: &str,
        config: HashMap<String, String>,
        requested_by: &str,
        cloud_provider: &str,
    ) -> String {
        let id = format!("infra-{}", self.infra_requests.len() + 1);
        let req = InfraRequest {
            id: id.clone(),
            template,
            service_id: service_id.to_string(),
            config,
            requested_by: requested_by.to_string(),
            status: InfraStatus::Requested,
            provisioned_at: None,
            cloud_provider: cloud_provider.to_string(),
        };
        self.infra_requests.push(req);
        id
    }

    pub fn provision_infrastructure(&mut self, request_id: &str) -> bool {
        if let Some(req) = self.infra_requests.iter_mut().find(|r| r.id == request_id) {
            req.status = InfraStatus::Active;
            req.provisioned_at = Some("2026-03-14T12:00:00Z".to_string());
            true
        } else {
            false
        }
    }

    pub fn create_team(&mut self, name: &str, members: Vec<String>) -> String {
        let id = format!("team-{}", self.teams.len() + 1);
        let team = TeamProfile {
            id: id.clone(),
            name: name.to_string(),
            members,
            owned_services: Vec::new(),
            slack_channel: None,
            on_call_schedule: None,
        };
        self.teams.push(team);
        id
    }

    pub fn add_team_member(&mut self, team_id: &str, member: &str) -> bool {
        if let Some(team) = self.teams.iter_mut().find(|t| t.id == team_id) {
            team.members.push(member.to_string());
            true
        } else {
            false
        }
    }

    pub fn start_onboarding(&mut self, team_id: &str) -> Option<String> {
        if !self.teams.iter().any(|t| t.id == team_id) {
            return None;
        }
        let steps: Vec<(OnboardingStep, bool)> =
            OnboardingStep::all().into_iter().map(|s| (s, false)).collect();
        let checklist = OnboardingChecklist {
            team_id: team_id.to_string(),
            steps,
            started_at: "2026-03-14".to_string(),
            completed_at: None,
            mentor: None,
        };
        self.onboarding_checklists.push(checklist);
        Some(team_id.to_string())
    }

    pub fn advance_onboarding(&mut self, team_id: &str, step: OnboardingStep) -> bool {
        if let Some(checklist) = self
            .onboarding_checklists
            .iter_mut()
            .find(|c| c.team_id == team_id)
        {
            for (s, done) in checklist.steps.iter_mut() {
                if *s == step {
                    *done = true;
                    // Check if all steps are complete
                    let all_done = checklist.steps.iter().all(|(_, d)| *d);
                    if all_done {
                        checklist.completed_at = Some("2026-03-14".to_string());
                    }
                    return true;
                }
            }
            false
        } else {
            false
        }
    }

    pub fn get_onboarding_progress(&self, team_id: &str) -> Option<(usize, usize)> {
        self.onboarding_checklists
            .iter()
            .find(|c| c.team_id == team_id)
            .map(|c| {
                let completed = c.steps.iter().filter(|(_, done)| *done).count();
                let total = c.steps.len();
                (completed, total)
            })
    }

    pub fn get_team_dashboard(&self, team_id: &str) -> Option<String> {
        let team = self.teams.iter().find(|t| t.id == team_id)?;
        let mut md = format!("# Team Dashboard: {}\n\n", team.name);
        md.push_str(&format!("**Members:** {}\n\n", team.members.join(", ")));

        md.push_str("## Owned Services\n\n");
        if team.owned_services.is_empty() {
            md.push_str("_No services registered._\n\n");
        } else {
            for svc_id in &team.owned_services {
                if let Some(svc) = self.get_service(svc_id) {
                    md.push_str(&format!(
                        "- **{}** ({:?}) — {:?}\n",
                        svc.name, svc.tier, svc.status
                    ));
                }
            }
            md.push('\n');
        }

        md.push_str("## Scorecards\n\n");
        let team_scorecards: Vec<&ScorecardResult> = self
            .scorecards
            .iter()
            .filter(|sc| team.owned_services.contains(&sc.service_id))
            .collect();
        if team_scorecards.is_empty() {
            md.push_str("_No scorecards evaluated._\n\n");
        } else {
            for sc in team_scorecards {
                md.push_str(&format!(
                    "- Service `{}`: Grade **{}** ({:.0}%)\n",
                    sc.service_id,
                    sc.grade,
                    sc.overall_score * 100.0
                ));
            }
            md.push('\n');
        }

        if let Some((completed, total)) = self.get_onboarding_progress(team_id) {
            md.push_str(&format!(
                "## Onboarding Progress\n\n{}/{} steps completed\n",
                completed, total
            ));
        }

        Some(md)
    }

    pub fn get_service_dependencies(&self, service_id: &str) -> Vec<&ServiceEntry> {
        if let Some(svc) = self.get_service(service_id) {
            svc.dependencies
                .iter()
                .filter_map(|dep_id| self.get_service(dep_id))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn export_catalog(&self) -> String {
        let mut md = String::from("# Service Catalog\n\n");
        if self.catalog.is_empty() {
            md.push_str("_No services registered._\n");
            return md;
        }
        md.push_str("| Name | Owner | Tier | Status | Language | Framework |\n");
        md.push_str("|------|-------|------|--------|----------|----------|\n");
        for svc in &self.catalog {
            md.push_str(&format!(
                "| {} | {} | {:?} | {:?} | {} | {} |\n",
                svc.name, svc.owner_team, svc.tier, svc.status, svc.language, svc.framework
            ));
        }
        md
    }

    pub fn calculate_dora_metrics(&self, team_id: &str) -> DoraMetrics {
        let team = self.teams.iter().find(|t| t.id == team_id);
        let service_count = team
            .map(|t| t.owned_services.len())
            .unwrap_or(0) as f64;

        // Mock metrics that scale with team's service count
        let factor = if service_count > 0.0 {
            1.0 / (1.0 + service_count * 0.1)
        } else {
            0.5
        };

        let deploy_freq = 5.0 + service_count * 2.0;
        let lead_time = 24.0 * factor;
        let mttr = 4.0 * factor;
        let cfr = 10.0 * factor;

        let rating = if deploy_freq >= 7.0 && lead_time < 24.0 && mttr < 4.0 && cfr < 15.0 {
            "Elite"
        } else if deploy_freq >= 3.0 && lead_time < 48.0 && mttr < 12.0 && cfr < 20.0 {
            "High"
        } else if deploy_freq >= 1.0 && lead_time < 168.0 && mttr < 48.0 && cfr < 30.0 {
            "Medium"
        } else {
            "Low"
        }
        .to_string();

        DoraMetrics {
            deploy_frequency_per_week: deploy_freq,
            lead_time_hours: lead_time,
            mttr_hours: mttr,
            change_failure_rate_pct: cfr,
            rating,
        }
    }

    pub fn enable_platform(&mut self, platform: IdpPlatform) -> bool {
        if let Some(cfg) = self
            .platform_configs
            .iter_mut()
            .find(|c| c.platform == platform)
        {
            cfg.enabled = true;
            true
        } else {
            false
        }
    }

    pub fn disable_platform(&mut self, platform: IdpPlatform) -> bool {
        if let Some(cfg) = self
            .platform_configs
            .iter_mut()
            .find(|c| c.platform == platform)
        {
            cfg.enabled = false;
            true
        } else {
            false
        }
    }

    pub fn get_enabled_platforms(&self) -> Vec<&IdpPlatformConfig> {
        self.platform_configs.iter().filter(|c| c.enabled).collect()
    }

    pub fn generate_backstage_catalog_info(&self, service_id: &str) -> Option<String> {
        let svc = self.get_service(service_id)?;
        let lifecycle = match svc.status {
            ServiceStatus::Active => "production",
            ServiceStatus::InDevelopment => "experimental",
            ServiceStatus::Deprecated => "deprecated",
            ServiceStatus::Maintenance => "production",
            ServiceStatus::Retired => "deprecated",
        };
        let yaml = format!(
            r#"apiVersion: backstage.io/v1alpha1
kind: Component
metadata:
  name: {}
  description: {}
  annotations:
    github.com/project-slug: {}
  tags:
{}
spec:
  type: service
  lifecycle: {}
  owner: {}
  system: default
"#,
            svc.name,
            svc.description,
            svc.repository,
            if svc.tags.is_empty() {
                "    []".to_string()
            } else {
                svc.tags
                    .iter()
                    .map(|t| format!("    - {}", t))
                    .collect::<Vec<_>>()
                    .join("\n")
            },
            lifecycle,
            svc.owner_team,
        );
        Some(yaml)
    }

    pub fn generate_backstage_template(
        &mut self,
        name: &str,
        title: &str,
        description: &str,
        owner: &str,
        language: &str,
    ) -> String {
        let template = BackstageTemplate {
            name: name.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            owner: owner.to_string(),
            steps: vec![
                BackstageTemplateStep {
                    id: "fetch".to_string(),
                    name: "Fetch Skeleton".to_string(),
                    action: "fetch:template".to_string(),
                    input: {
                        let mut m = HashMap::new();
                        m.insert("url".to_string(), "./skeleton".to_string());
                        m
                    },
                },
                BackstageTemplateStep {
                    id: "publish".to_string(),
                    name: "Publish to GitHub".to_string(),
                    action: "publish:github".to_string(),
                    input: {
                        let mut m = HashMap::new();
                        m.insert(
                            "repoUrl".to_string(),
                            "github.com?owner=org&repo=${{values.name}}".to_string(),
                        );
                        m
                    },
                },
                BackstageTemplateStep {
                    id: "register".to_string(),
                    name: "Register in Catalog".to_string(),
                    action: "catalog:register".to_string(),
                    input: {
                        let mut m = HashMap::new();
                        m.insert(
                            "catalogInfoUrl".to_string(),
                            "${{steps.publish.output.remoteUrl}}/catalog-info.yaml".to_string(),
                        );
                        m
                    },
                },
            ],
            parameters: vec![BackstageParameter {
                name: "name".to_string(),
                title: "Service Name".to_string(),
                param_type: "string".to_string(),
                required: true,
                description: "Name of the new service".to_string(),
            }],
        };

        let yaml = format!(
            r#"apiVersion: scaffolder.backstage.io/v1beta3
kind: Template
metadata:
  name: {}
  title: {}
  description: {}
spec:
  owner: {}
  type: service
  parameters:
    - title: Service Details
      properties:
        name:
          title: Service Name
          type: string
          description: Name of the new service
        language:
          title: Language
          type: string
          default: {}
  steps:
    - id: fetch
      name: Fetch Skeleton
      action: fetch:template
      input:
        url: ./skeleton
    - id: publish
      name: Publish to GitHub
      action: publish:github
      input:
        repoUrl: github.com?owner=org&repo=${{{{values.name}}}}
    - id: register
      name: Register in Catalog
      action: catalog:register
      input:
        catalogInfoUrl: ${{{{steps.publish.output.remoteUrl}}}}/catalog-info.yaml
"#,
            name, title, description, owner, language,
        );

        self.backstage_templates.push(template);
        yaml
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_backstage_component(
        &mut self,
        kind: &str,
        name: &str,
        namespace: &str,
        owner: &str,
        system: &str,
        lifecycle: &str,
        spec_type: &str,
    ) -> String {
        let id = format!("bsc-{}", self.backstage_components.len() + 1);
        let component = BackstageComponent {
            kind: kind.to_string(),
            name: name.to_string(),
            namespace: namespace.to_string(),
            owner: owner.to_string(),
            system: system.to_string(),
            lifecycle: lifecycle.to_string(),
            spec_type: spec_type.to_string(),
            annotations: HashMap::new(),
        };
        self.backstage_components.push(component);
        id
    }

    pub fn install_plugin(
        &mut self,
        name: &str,
        category: PluginCategory,
        platform: IdpPlatform,
        version: &str,
        description: &str,
    ) -> String {
        let id = format!("plugin-{}", self.plugins.len() + 1);
        let plugin = IdpPlugin {
            id: id.clone(),
            name: name.to_string(),
            category,
            platform,
            version: version.to_string(),
            description: description.to_string(),
            installed: true,
        };
        self.plugins.push(plugin);
        id
    }

    pub fn uninstall_plugin(&mut self, id: &str) -> bool {
        if let Some(plugin) = self.plugins.iter_mut().find(|p| p.id == id) {
            plugin.installed = false;
            true
        } else {
            false
        }
    }

    pub fn get_plugins_for_platform(&self, platform: &IdpPlatform) -> Vec<&IdpPlugin> {
        self.plugins
            .iter()
            .filter(|p| p.platform == *platform && p.installed)
            .collect()
    }

    pub fn add_self_service_action(
        &mut self,
        name: &str,
        description: &str,
        template: InfraTemplate,
        required_params: Vec<String>,
        approval_required: bool,
        estimated_time: u64,
    ) -> String {
        let id = format!("ssa-{}", self.self_service_actions.len() + 1);
        let action = SelfServiceAction {
            id: id.clone(),
            name: name.to_string(),
            description: description.to_string(),
            template,
            required_params,
            approval_required,
            estimated_time_secs: estimated_time,
        };
        self.self_service_actions.push(action);
        id
    }

    pub fn list_self_service_actions(&self) -> Vec<&SelfServiceAction> {
        self.self_service_actions.iter().collect()
    }

    pub fn generate_cycloid_blueprint(&self, service_id: &str) -> Option<String> {
        let svc = self.get_service(service_id)?;
        let blueprint = format!(
            r#"---
# Cycloid Blueprint for {}
version: "2"
name: {}
description: {}
author: {}
stack:
  technology: {}
  framework: {}
  repository: {}
config:
  pipeline:
    pipeline:
      path: pipeline/pipeline.yml
    variables:
      path: pipeline/variables.sample.yml
  terraform:
    main:
      path: terraform/main.tf
  ansible:
    deploy:
      path: ansible/deploy.yml
"#,
            svc.name,
            svc.name,
            svc.description,
            svc.owner_team,
            svc.language,
            svc.framework,
            svc.repository,
        );
        Some(blueprint)
    }

    pub fn generate_humanitec_score_file(&self, service_id: &str) -> Option<String> {
        let svc = self.get_service(service_id)?;
        let score = format!(
            r#"apiVersion: score.dev/v1b1
metadata:
  name: {}
containers:
  main:
    image: .
    variables:
      SERVICE_NAME: {}
      LANGUAGE: {}
      FRAMEWORK: {}
resources:
  dns:
    type: dns
  route:
    type: route
    params:
      host: ${{{{resources.dns.host}}}}
      path: /
      port: 8080
"#,
            svc.name, svc.name, svc.language, svc.framework,
        );
        Some(score)
    }

    pub fn generate_port_blueprint(&self, service_id: &str) -> Option<String> {
        let svc = self.get_service(service_id)?;
        let tier_str = match svc.tier {
            ServiceTier::Gold => "Gold",
            ServiceTier::Silver => "Silver",
            ServiceTier::Bronze => "Bronze",
            ServiceTier::Experimental => "Experimental",
        };
        let status_str = match svc.status {
            ServiceStatus::Active => "Active",
            ServiceStatus::Deprecated => "Deprecated",
            ServiceStatus::InDevelopment => "InDevelopment",
            ServiceStatus::Maintenance => "Maintenance",
            ServiceStatus::Retired => "Retired",
        };
        let blueprint = format!(
            r#"{{
  "identifier": "{}",
  "title": "{}",
  "blueprint": "microservice",
  "properties": {{
    "description": "{}",
    "language": "{}",
    "framework": "{}",
    "tier": "{}",
    "status": "{}",
    "repository": "{}"
  }},
  "relations": {{
    "team": "{}"
  }}
}}"#,
            svc.id,
            svc.name,
            svc.description,
            svc.language,
            svc.framework,
            tier_str,
            status_str,
            svc.repository,
            svc.owner_team,
        );
        Some(blueprint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_manager_has_platform_configs() {
        let mgr = IdpManager::new();
        assert_eq!(mgr.platform_configs.len(), 13);
        assert!(mgr.platform_configs.iter().all(|c| !c.enabled));
    }

    #[test]
    fn test_new_manager_has_golden_paths() {
        let mgr = IdpManager::new();
        assert_eq!(mgr.golden_paths.len(), 3);
    }

    #[test]
    fn test_new_manager_has_self_service_actions() {
        let mgr = IdpManager::new();
        assert_eq!(mgr.self_service_actions.len(), 3);
    }

    #[test]
    fn test_register_service() {
        let mut mgr = IdpManager::new();
        let id = mgr.register_service(
            "auth-api",
            "Authentication API",
            "platform-team",
            ServiceTier::Gold,
            "org/auth-api",
            "rust",
            "actix-web",
        );
        assert_eq!(id, "svc-1");
        assert_eq!(mgr.catalog.len(), 1);
        assert_eq!(mgr.catalog[0].name, "auth-api");
        assert_eq!(mgr.catalog[0].status, ServiceStatus::InDevelopment);
    }

    #[test]
    fn test_register_multiple_services() {
        let mut mgr = IdpManager::new();
        let id1 = mgr.register_service("svc-a", "A", "team-a", ServiceTier::Gold, "r", "rust", "axum");
        let id2 = mgr.register_service("svc-b", "B", "team-b", ServiceTier::Silver, "r", "go", "gin");
        assert_eq!(id1, "svc-1");
        assert_eq!(id2, "svc-2");
        assert_eq!(mgr.catalog.len(), 2);
    }

    #[test]
    fn test_update_service_status() {
        let mut mgr = IdpManager::new();
        let id = mgr.register_service("svc", "desc", "team", ServiceTier::Silver, "r", "go", "gin");
        assert!(mgr.update_service_status(&id, ServiceStatus::Active));
        assert_eq!(mgr.get_service(&id).unwrap().status, ServiceStatus::Active);
    }

    #[test]
    fn test_update_service_status_not_found() {
        let mut mgr = IdpManager::new();
        assert!(!mgr.update_service_status("nonexistent", ServiceStatus::Active));
    }

    #[test]
    fn test_search_catalog_by_name() {
        let mut mgr = IdpManager::new();
        mgr.register_service("auth-api", "Auth", "team", ServiceTier::Gold, "r", "rust", "actix");
        mgr.register_service("user-api", "Users", "team", ServiceTier::Silver, "r", "go", "gin");
        let results = mgr.search_catalog("auth");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "auth-api");
    }

    #[test]
    fn test_search_catalog_by_team() {
        let mut mgr = IdpManager::new();
        mgr.register_service("svc-a", "A", "alpha-team", ServiceTier::Gold, "r", "rust", "axum");
        mgr.register_service("svc-b", "B", "beta-team", ServiceTier::Silver, "r", "go", "gin");
        let results = mgr.search_catalog("alpha");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].owner_team, "alpha-team");
    }

    #[test]
    fn test_search_catalog_by_tag() {
        let mut mgr = IdpManager::new();
        let id = mgr.register_service("svc", "desc", "team", ServiceTier::Gold, "r", "rust", "actix");
        mgr.catalog[0].tags.push("backend".to_string());
        mgr.catalog[0].tags.push("api".to_string());
        let results = mgr.search_catalog_by_tag("backend");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
    }

    #[test]
    fn test_search_catalog_by_tag_no_match() {
        let mut mgr = IdpManager::new();
        mgr.register_service("svc", "desc", "team", ServiceTier::Gold, "r", "rust", "actix");
        let results = mgr.search_catalog_by_tag("frontend");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_empty_catalog() {
        let mgr = IdpManager::new();
        let results = mgr.search_catalog("anything");
        assert!(results.is_empty());
    }

    #[test]
    fn test_get_service() {
        let mut mgr = IdpManager::new();
        let id = mgr.register_service("svc", "desc", "team", ServiceTier::Gold, "r", "rust", "axum");
        assert!(mgr.get_service(&id).is_some());
        assert!(mgr.get_service("nope").is_none());
    }

    #[test]
    fn test_add_golden_path() {
        let mut mgr = IdpManager::new();
        let id = mgr.add_golden_path(
            "Go gRPC",
            "go",
            "grpc",
            "org/golden-go-grpc",
            vec!["Dockerfile".to_string(), "Makefile".to_string()],
            "Go gRPC service template",
        );
        assert!(id.starts_with("gp-"));
        assert_eq!(mgr.golden_paths.len(), 4); // 3 pre-populated + 1
    }

    #[test]
    fn test_get_golden_paths_for_language() {
        let mgr = IdpManager::new();
        let rust_paths = mgr.get_golden_paths_for_language("rust");
        assert_eq!(rust_paths.len(), 1);
        assert_eq!(rust_paths[0].name, "Rust API Service");
    }

    #[test]
    fn test_get_golden_paths_for_language_case_insensitive() {
        let mgr = IdpManager::new();
        let paths = mgr.get_golden_paths_for_language("Rust");
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn test_get_golden_paths_no_match() {
        let mgr = IdpManager::new();
        let paths = mgr.get_golden_paths_for_language("haskell");
        assert!(paths.is_empty());
    }

    #[test]
    fn test_evaluate_scorecard_gold() {
        let mut mgr = IdpManager::new();
        let id = mgr.register_service("svc", "desc", "team", ServiceTier::Gold, "r", "rust", "axum");
        let result = mgr.evaluate_scorecard(&id).unwrap();
        assert_eq!(result.grade, "B");
        assert!(result.overall_score > 0.7);
        assert_eq!(result.service_id, id);
    }

    #[test]
    fn test_evaluate_scorecard_experimental() {
        let mut mgr = IdpManager::new();
        let id = mgr.register_service(
            "exp",
            "experimental",
            "team",
            ServiceTier::Experimental,
            "r",
            "rust",
            "axum",
        );
        let result = mgr.evaluate_scorecard(&id).unwrap();
        assert!(result.overall_score < 0.5);
        assert!(!result.recommendations.is_empty());
    }

    #[test]
    fn test_evaluate_scorecard_not_found() {
        let mut mgr = IdpManager::new();
        assert!(mgr.evaluate_scorecard("nonexistent").is_none());
    }

    #[test]
    fn test_request_infrastructure() {
        let mut mgr = IdpManager::new();
        let id = mgr.request_infrastructure(
            InfraTemplate::Database,
            "svc-1",
            HashMap::new(),
            "alice",
            "aws",
        );
        assert_eq!(id, "infra-1");
        assert_eq!(mgr.infra_requests[0].status, InfraStatus::Requested);
    }

    #[test]
    fn test_provision_infrastructure() {
        let mut mgr = IdpManager::new();
        let id = mgr.request_infrastructure(
            InfraTemplate::Cache,
            "svc-1",
            HashMap::new(),
            "bob",
            "gcp",
        );
        assert!(mgr.provision_infrastructure(&id));
        assert_eq!(mgr.infra_requests[0].status, InfraStatus::Active);
        assert!(mgr.infra_requests[0].provisioned_at.is_some());
    }

    #[test]
    fn test_provision_infrastructure_not_found() {
        let mut mgr = IdpManager::new();
        assert!(!mgr.provision_infrastructure("nope"));
    }

    #[test]
    fn test_create_team() {
        let mut mgr = IdpManager::new();
        let id = mgr.create_team("backend-team", vec!["alice".to_string(), "bob".to_string()]);
        assert_eq!(id, "team-1");
        assert_eq!(mgr.teams[0].members.len(), 2);
    }

    #[test]
    fn test_add_team_member() {
        let mut mgr = IdpManager::new();
        let id = mgr.create_team("team", vec!["alice".to_string()]);
        assert!(mgr.add_team_member(&id, "charlie"));
        assert_eq!(mgr.teams[0].members.len(), 2);
    }

    #[test]
    fn test_add_team_member_not_found() {
        let mut mgr = IdpManager::new();
        assert!(!mgr.add_team_member("nope", "alice"));
    }

    #[test]
    fn test_start_onboarding() {
        let mut mgr = IdpManager::new();
        let team_id = mgr.create_team("team", vec!["alice".to_string()]);
        let result = mgr.start_onboarding(&team_id);
        assert!(result.is_some());
        assert_eq!(mgr.onboarding_checklists.len(), 1);
        assert_eq!(mgr.onboarding_checklists[0].steps.len(), 10);
    }

    #[test]
    fn test_start_onboarding_nonexistent_team() {
        let mut mgr = IdpManager::new();
        assert!(mgr.start_onboarding("nope").is_none());
    }

    #[test]
    fn test_advance_onboarding() {
        let mut mgr = IdpManager::new();
        let team_id = mgr.create_team("team", vec!["alice".to_string()]);
        mgr.start_onboarding(&team_id);
        assert!(mgr.advance_onboarding(&team_id, OnboardingStep::RepoSetup));
        let (completed, total) = mgr.get_onboarding_progress(&team_id).unwrap();
        assert_eq!(completed, 1);
        assert_eq!(total, 10);
    }

    #[test]
    fn test_advance_onboarding_completes() {
        let mut mgr = IdpManager::new();
        let team_id = mgr.create_team("team", vec!["alice".to_string()]);
        mgr.start_onboarding(&team_id);
        for step in OnboardingStep::all() {
            mgr.advance_onboarding(&team_id, step);
        }
        let (completed, total) = mgr.get_onboarding_progress(&team_id).unwrap();
        assert_eq!(completed, total);
        assert!(mgr.onboarding_checklists[0].completed_at.is_some());
    }

    #[test]
    fn test_get_onboarding_progress_not_found() {
        let mgr = IdpManager::new();
        assert!(mgr.get_onboarding_progress("nope").is_none());
    }

    #[test]
    fn test_get_team_dashboard() {
        let mut mgr = IdpManager::new();
        let team_id = mgr.create_team("platform", vec!["alice".to_string()]);
        mgr.register_service("svc", "desc", "platform", ServiceTier::Gold, "r", "rust", "axum");
        let dashboard = mgr.get_team_dashboard(&team_id).unwrap();
        assert!(dashboard.contains("platform"));
        assert!(dashboard.contains("svc"));
    }

    #[test]
    fn test_get_team_dashboard_not_found() {
        let mgr = IdpManager::new();
        assert!(mgr.get_team_dashboard("nope").is_none());
    }

    #[test]
    fn test_service_dependencies() {
        let mut mgr = IdpManager::new();
        let id1 = mgr.register_service("auth", "Auth", "team", ServiceTier::Gold, "r", "rust", "axum");
        let id2 = mgr.register_service("user", "Users", "team", ServiceTier::Silver, "r", "go", "gin");
        mgr.catalog[1].dependencies.push(id1.clone());
        let deps = mgr.get_service_dependencies(&id2);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "auth");
    }

    #[test]
    fn test_service_dependencies_empty() {
        let mut mgr = IdpManager::new();
        let id = mgr.register_service("svc", "desc", "team", ServiceTier::Gold, "r", "rust", "axum");
        let deps = mgr.get_service_dependencies(&id);
        assert!(deps.is_empty());
    }

    #[test]
    fn test_service_dependencies_not_found() {
        let mgr = IdpManager::new();
        let deps = mgr.get_service_dependencies("nope");
        assert!(deps.is_empty());
    }

    #[test]
    fn test_export_catalog() {
        let mut mgr = IdpManager::new();
        mgr.register_service("svc", "desc", "team", ServiceTier::Gold, "r", "rust", "axum");
        let md = mgr.export_catalog();
        assert!(md.contains("Service Catalog"));
        assert!(md.contains("svc"));
        assert!(md.contains("team"));
    }

    #[test]
    fn test_export_empty_catalog() {
        let mgr = IdpManager::new();
        let md = mgr.export_catalog();
        assert!(md.contains("No services registered"));
    }

    #[test]
    fn test_calculate_dora_metrics() {
        let mut mgr = IdpManager::new();
        let team_id = mgr.create_team("team", vec!["alice".to_string()]);
        mgr.register_service("svc", "desc", "team", ServiceTier::Gold, "r", "rust", "axum");
        let metrics = mgr.calculate_dora_metrics(&team_id);
        assert!(metrics.deploy_frequency_per_week > 0.0);
        assert!(metrics.lead_time_hours > 0.0);
        assert!(!metrics.rating.is_empty());
    }

    #[test]
    fn test_calculate_dora_metrics_no_team() {
        let mgr = IdpManager::new();
        let metrics = mgr.calculate_dora_metrics("nonexistent");
        assert_eq!(metrics.rating, "High");
    }

    #[test]
    fn test_enable_disable_platform() {
        let mut mgr = IdpManager::new();
        assert!(mgr.enable_platform(IdpPlatform::Backstage));
        assert_eq!(mgr.get_enabled_platforms().len(), 1);
        assert!(mgr.disable_platform(IdpPlatform::Backstage));
        assert_eq!(mgr.get_enabled_platforms().len(), 0);
    }

    #[test]
    fn test_enable_multiple_platforms() {
        let mut mgr = IdpManager::new();
        mgr.enable_platform(IdpPlatform::Backstage);
        mgr.enable_platform(IdpPlatform::Humanitec);
        mgr.enable_platform(IdpPlatform::Port);
        assert_eq!(mgr.get_enabled_platforms().len(), 3);
    }

    #[test]
    fn test_generate_backstage_catalog_info() {
        let mut mgr = IdpManager::new();
        let id = mgr.register_service("auth-api", "Auth service", "team-a", ServiceTier::Gold, "org/auth", "rust", "axum");
        mgr.update_service_status(&id, ServiceStatus::Active);
        let yaml = mgr.generate_backstage_catalog_info(&id).unwrap();
        assert!(yaml.contains("kind: Component"));
        assert!(yaml.contains("name: auth-api"));
        assert!(yaml.contains("lifecycle: production"));
        assert!(yaml.contains("owner: team-a"));
    }

    #[test]
    fn test_generate_backstage_catalog_info_not_found() {
        let mgr = IdpManager::new();
        assert!(mgr.generate_backstage_catalog_info("nope").is_none());
    }

    #[test]
    fn test_generate_backstage_template() {
        let mut mgr = IdpManager::new();
        let yaml = mgr.generate_backstage_template(
            "rust-svc",
            "Rust Service",
            "Scaffold a Rust service",
            "platform-team",
            "rust",
        );
        assert!(yaml.contains("kind: Template"));
        assert!(yaml.contains("name: rust-svc"));
        assert!(yaml.contains("owner: platform-team"));
        assert!(yaml.contains("fetch:template"));
        assert_eq!(mgr.backstage_templates.len(), 1);
    }

    #[test]
    fn test_register_backstage_component() {
        let mut mgr = IdpManager::new();
        let id = mgr.register_backstage_component(
            "Component",
            "my-svc",
            "default",
            "team-a",
            "my-system",
            "production",
            "service",
        );
        assert!(id.starts_with("bsc-"));
        assert_eq!(mgr.backstage_components.len(), 1);
        assert_eq!(mgr.backstage_components[0].name, "my-svc");
    }

    #[test]
    fn test_install_plugin() {
        let mut mgr = IdpManager::new();
        let id = mgr.install_plugin(
            "tech-radar",
            PluginCategory::Analytics,
            IdpPlatform::Backstage,
            "1.0.0",
            "Tech Radar visualization",
        );
        assert!(id.starts_with("plugin-"));
        assert_eq!(mgr.plugins.len(), 1);
        assert!(mgr.plugins[0].installed);
    }

    #[test]
    fn test_uninstall_plugin() {
        let mut mgr = IdpManager::new();
        let id = mgr.install_plugin(
            "tech-radar",
            PluginCategory::Analytics,
            IdpPlatform::Backstage,
            "1.0.0",
            "Tech Radar",
        );
        assert!(mgr.uninstall_plugin(&id));
        assert!(!mgr.plugins[0].installed);
    }

    #[test]
    fn test_uninstall_plugin_not_found() {
        let mut mgr = IdpManager::new();
        assert!(!mgr.uninstall_plugin("nope"));
    }

    #[test]
    fn test_get_plugins_for_platform() {
        let mut mgr = IdpManager::new();
        mgr.install_plugin("p1", PluginCategory::CiCd, IdpPlatform::Backstage, "1.0", "desc");
        mgr.install_plugin("p2", PluginCategory::Security, IdpPlatform::Backstage, "1.0", "desc");
        mgr.install_plugin("p3", PluginCategory::Monitoring, IdpPlatform::Cortex, "1.0", "desc");
        let bs_plugins = mgr.get_plugins_for_platform(&IdpPlatform::Backstage);
        assert_eq!(bs_plugins.len(), 2);
    }

    #[test]
    fn test_add_self_service_action() {
        let mut mgr = IdpManager::new();
        let id = mgr.add_self_service_action(
            "Create CDN",
            "Set up a CDN distribution",
            InfraTemplate::Cdn,
            vec!["domain".to_string()],
            true,
            600,
        );
        assert!(id.starts_with("ssa-"));
        assert_eq!(mgr.self_service_actions.len(), 4); // 3 pre-populated + 1
    }

    #[test]
    fn test_list_self_service_actions() {
        let mgr = IdpManager::new();
        let actions = mgr.list_self_service_actions();
        assert_eq!(actions.len(), 3);
    }

    #[test]
    fn test_generate_cycloid_blueprint() {
        let mut mgr = IdpManager::new();
        let id = mgr.register_service("payments", "Payment processing", "fintech", ServiceTier::Gold, "org/payments", "go", "gin");
        let blueprint = mgr.generate_cycloid_blueprint(&id).unwrap();
        assert!(blueprint.contains("Cycloid Blueprint"));
        assert!(blueprint.contains("payments"));
        assert!(blueprint.contains("technology: go"));
        assert!(blueprint.contains("framework: gin"));
    }

    #[test]
    fn test_generate_cycloid_blueprint_not_found() {
        let mgr = IdpManager::new();
        assert!(mgr.generate_cycloid_blueprint("nope").is_none());
    }

    #[test]
    fn test_generate_humanitec_score_file() {
        let mut mgr = IdpManager::new();
        let id = mgr.register_service("api", "API service", "team", ServiceTier::Silver, "org/api", "python", "fastapi");
        let score = mgr.generate_humanitec_score_file(&id).unwrap();
        assert!(score.contains("score.dev/v1b1"));
        assert!(score.contains("name: api"));
        assert!(score.contains("LANGUAGE: python"));
    }

    #[test]
    fn test_generate_humanitec_score_not_found() {
        let mgr = IdpManager::new();
        assert!(mgr.generate_humanitec_score_file("nope").is_none());
    }

    #[test]
    fn test_generate_port_blueprint() {
        let mut mgr = IdpManager::new();
        let id = mgr.register_service("gateway", "API Gateway", "infra", ServiceTier::Gold, "org/gw", "rust", "axum");
        let bp = mgr.generate_port_blueprint(&id).unwrap();
        assert!(bp.contains("\"blueprint\": \"microservice\""));
        assert!(bp.contains("\"language\": \"rust\""));
        assert!(bp.contains("\"tier\": \"Gold\""));
        assert!(bp.contains("\"team\": \"infra\""));
    }

    #[test]
    fn test_generate_port_blueprint_not_found() {
        let mgr = IdpManager::new();
        assert!(mgr.generate_port_blueprint("nope").is_none());
    }

    #[test]
    fn test_duplicate_service_names_allowed() {
        let mut mgr = IdpManager::new();
        let id1 = mgr.register_service("svc", "first", "team", ServiceTier::Gold, "r1", "rust", "axum");
        let id2 = mgr.register_service("svc", "second", "team", ServiceTier::Silver, "r2", "go", "gin");
        assert_ne!(id1, id2);
        assert_eq!(mgr.catalog.len(), 2);
    }

    #[test]
    fn test_team_service_association() {
        let mut mgr = IdpManager::new();
        let team_id = mgr.create_team("backend", vec!["alice".to_string()]);
        let svc_id = mgr.register_service("svc", "desc", "backend", ServiceTier::Gold, "r", "rust", "axum");
        let team = mgr.teams.iter().find(|t| t.id == team_id).unwrap();
        assert!(team.owned_services.contains(&svc_id));
    }

    #[test]
    fn test_scorecard_grade_levels() {
        let mut mgr = IdpManager::new();
        let gold_id = mgr.register_service("gold", "d", "t", ServiceTier::Gold, "r", "rust", "axum");
        let bronze_id = mgr.register_service("bronze", "d", "t", ServiceTier::Bronze, "r", "go", "gin");

        let gold_result = mgr.evaluate_scorecard(&gold_id).unwrap();
        let bronze_result = mgr.evaluate_scorecard(&bronze_id).unwrap();

        assert!(gold_result.overall_score > bronze_result.overall_score);
    }

    #[test]
    fn test_infra_request_lifecycle() {
        let mut mgr = IdpManager::new();
        let mut config = HashMap::new();
        config.insert("engine".to_string(), "postgres".to_string());
        config.insert("size".to_string(), "large".to_string());

        let req_id = mgr.request_infrastructure(
            InfraTemplate::Database,
            "svc-1",
            config,
            "alice",
            "aws",
        );
        assert_eq!(mgr.infra_requests[0].status, InfraStatus::Requested);

        mgr.provision_infrastructure(&req_id);
        assert_eq!(mgr.infra_requests[0].status, InfraStatus::Active);
        assert!(mgr.infra_requests[0].provisioned_at.is_some());
        assert_eq!(mgr.infra_requests[0].cloud_provider, "aws");
    }

    #[test]
    fn test_platform_name_display() {
        assert_eq!(IdpPlatform::Backstage.name(), "Backstage");
        assert_eq!(IdpPlatform::MiaPlatform.name(), "Mia Platform");
        assert_eq!(IdpPlatform::OpsLevel.name(), "OpsLevel");
        assert_eq!(IdpPlatform::MorpheusData.name(), "Morpheus Data");
        assert_eq!(IdpPlatform::CloudBolt.name(), "CloudBolt");
    }

    #[test]
    fn test_all_platforms_enumerated() {
        let all = IdpPlatform::all();
        assert_eq!(all.len(), 13);
    }

    #[test]
    fn test_backstage_catalog_lifecycle_mapping() {
        let mut mgr = IdpManager::new();
        let id = mgr.register_service("svc", "desc", "team", ServiceTier::Gold, "r", "rust", "axum");

        // InDevelopment -> experimental
        let yaml = mgr.generate_backstage_catalog_info(&id).unwrap();
        assert!(yaml.contains("lifecycle: experimental"));

        mgr.update_service_status(&id, ServiceStatus::Deprecated);
        let yaml = mgr.generate_backstage_catalog_info(&id).unwrap();
        assert!(yaml.contains("lifecycle: deprecated"));
    }

    #[test]
    fn test_plugins_uninstalled_not_in_platform_list() {
        let mut mgr = IdpManager::new();
        let id = mgr.install_plugin("p1", PluginCategory::CiCd, IdpPlatform::Backstage, "1.0", "desc");
        mgr.uninstall_plugin(&id);
        let plugins = mgr.get_plugins_for_platform(&IdpPlatform::Backstage);
        assert!(plugins.is_empty());
    }
}
