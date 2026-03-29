//! One-click managed deployment for VibeCody.
//!
//! Deploys projects to Vercel, Netlify, Fly.io, Railway, Render, or
//! self-hosted Docker. Closes the gap vs Lovable / Bolt Cloud / v0.
//!
//! REPL commands: `/deploy create|list|status|rollback|domain|health|logs`

use std::collections::HashMap;

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum DeployPlatform {
    Vercel,
    Netlify,
    FlyIo,
    Railway,
    Render,
    DockerSelfHosted,
}

impl DeployPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Vercel => "vercel",
            Self::Netlify => "netlify",
            Self::FlyIo => "fly.io",
            Self::Railway => "railway",
            Self::Render => "render",
            Self::DockerSelfHosted => "docker-self-hosted",
        }
    }
}

impl std::fmt::Display for DeployPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DetectedFramework {
    NextJs,
    Vite,
    Remix,
    Gatsby,
    Nuxt,
    SvelteKit,
    Astro,
    CreateReactApp,
    Static,
    Express,
    FastApi,
    Rails,
    Django,
    Unknown,
}

impl std::fmt::Display for DetectedFramework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NextJs => write!(f, "next.js"),
            Self::Vite => write!(f, "vite"),
            Self::Remix => write!(f, "remix"),
            Self::Gatsby => write!(f, "gatsby"),
            Self::Nuxt => write!(f, "nuxt"),
            Self::SvelteKit => write!(f, "sveltekit"),
            Self::Astro => write!(f, "astro"),
            Self::CreateReactApp => write!(f, "create-react-app"),
            Self::Static => write!(f, "static"),
            Self::Express => write!(f, "express"),
            Self::FastApi => write!(f, "fastapi"),
            Self::Rails => write!(f, "rails"),
            Self::Django => write!(f, "django"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeployStatus {
    Pending,
    Building,
    Deploying,
    Live,
    Failed,
    RolledBack,
    Preview,
}

impl std::fmt::Display for DeployStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Building => write!(f, "building"),
            Self::Deploying => write!(f, "deploying"),
            Self::Live => write!(f, "live"),
            Self::Failed => write!(f, "failed"),
            Self::RolledBack => write!(f, "rolled-back"),
            Self::Preview => write!(f, "preview"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeployError {
    PlatformNotConfigured(String),
    BuildFailed(String),
    DeployFailed(String),
    HealthCheckFailed(String),
    RollbackFailed(String),
    DomainConfigError(String),
    ProjectNotFound(String),
    InvalidConfig(String),
    DuplicateDeployment(String),
}

impl std::fmt::Display for DeployError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlatformNotConfigured(msg) => write!(f, "platform not configured: {msg}"),
            Self::BuildFailed(msg) => write!(f, "build failed: {msg}"),
            Self::DeployFailed(msg) => write!(f, "deploy failed: {msg}"),
            Self::HealthCheckFailed(msg) => write!(f, "health check failed: {msg}"),
            Self::RollbackFailed(msg) => write!(f, "rollback failed: {msg}"),
            Self::DomainConfigError(msg) => write!(f, "domain config error: {msg}"),
            Self::ProjectNotFound(msg) => write!(f, "project not found: {msg}"),
            Self::InvalidConfig(msg) => write!(f, "invalid config: {msg}"),
            Self::DuplicateDeployment(msg) => write!(f, "duplicate deployment: {msg}"),
        }
    }
}

// === Data Structures ===

#[derive(Debug, Clone)]
pub struct DeployConfig {
    pub default_platform: DeployPlatform,
    pub auto_detect_framework: bool,
    pub env_file: String,
    pub preview_deploys: bool,
    pub auto_rollback_on_error: bool,
    pub health_check_timeout_secs: u64,
}

impl Default for DeployConfig {
    fn default() -> Self {
        Self {
            default_platform: DeployPlatform::Vercel,
            auto_detect_framework: true,
            env_file: ".env".to_string(),
            preview_deploys: true,
            auto_rollback_on_error: true,
            health_check_timeout_secs: 60,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameworkDetection {
    pub framework: DetectedFramework,
    pub build_command: String,
    pub output_dir: String,
    pub dev_command: String,
    pub install_command: String,
}

#[derive(Debug, Clone)]
pub struct Deployment {
    pub id: String,
    pub project_name: String,
    pub platform: DeployPlatform,
    pub url: Option<String>,
    pub preview_url: Option<String>,
    pub status: DeployStatus,
    pub commit_ref: Option<String>,
    pub branch: String,
    pub created_at: u64,
    pub completed_at: Option<u64>,
    pub build_logs: Vec<String>,
    pub env_vars: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DomainConfig {
    pub domain: String,
    pub ssl_enabled: bool,
    pub redirect_www: bool,
    pub custom_headers: Vec<(String, String)>,
}

impl Default for DomainConfig {
    fn default() -> Self {
        Self {
            domain: String::new(),
            ssl_enabled: true,
            redirect_www: true,
            custom_headers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub url: String,
    pub status_code: u16,
    pub response_time_ms: u64,
    pub healthy: bool,
    pub checked_at: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RollbackInfo {
    pub deployment_id: String,
    pub rollback_to_id: String,
    pub reason: String,
    pub timestamp: u64,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct DeployCommand {
    pub platform: DeployPlatform,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub description: String,
}

// === DeployManager ===

pub struct DeployManager {
    config: DeployConfig,
    deployments: Vec<Deployment>,
    domains: HashMap<String, DomainConfig>,
    next_id: u64,
}

impl DeployManager {
    pub fn new(config: DeployConfig) -> Self {
        Self {
            config,
            deployments: Vec::new(),
            domains: HashMap::new(),
            next_id: 1,
        }
    }

    /// Detect framework from a list of project file paths.
    pub fn detect_framework(project_files: &[String]) -> FrameworkDetection {
        let has = |name: &str| project_files.iter().any(|f| f.contains(name));

        if has("next.config") {
            return FrameworkDetection {
                framework: DetectedFramework::NextJs,
                build_command: "next build".to_string(),
                output_dir: ".next".to_string(),
                dev_command: "next dev".to_string(),
                install_command: "npm install".to_string(),
            };
        }
        if has("remix.config") || has("app/root.tsx") {
            return FrameworkDetection {
                framework: DetectedFramework::Remix,
                build_command: "remix build".to_string(),
                output_dir: "build".to_string(),
                dev_command: "remix dev".to_string(),
                install_command: "npm install".to_string(),
            };
        }
        if has("gatsby-config") {
            return FrameworkDetection {
                framework: DetectedFramework::Gatsby,
                build_command: "gatsby build".to_string(),
                output_dir: "public".to_string(),
                dev_command: "gatsby develop".to_string(),
                install_command: "npm install".to_string(),
            };
        }
        if has("nuxt.config") {
            return FrameworkDetection {
                framework: DetectedFramework::Nuxt,
                build_command: "nuxt build".to_string(),
                output_dir: ".nuxt".to_string(),
                dev_command: "nuxt dev".to_string(),
                install_command: "npm install".to_string(),
            };
        }
        if has("svelte.config") {
            return FrameworkDetection {
                framework: DetectedFramework::SvelteKit,
                build_command: "vite build".to_string(),
                output_dir: "build".to_string(),
                dev_command: "vite dev".to_string(),
                install_command: "npm install".to_string(),
            };
        }
        if has("astro.config") {
            return FrameworkDetection {
                framework: DetectedFramework::Astro,
                build_command: "astro build".to_string(),
                output_dir: "dist".to_string(),
                dev_command: "astro dev".to_string(),
                install_command: "npm install".to_string(),
            };
        }
        if has("vite.config") {
            return FrameworkDetection {
                framework: DetectedFramework::Vite,
                build_command: "vite build".to_string(),
                output_dir: "dist".to_string(),
                dev_command: "vite dev".to_string(),
                install_command: "npm install".to_string(),
            };
        }
        if has("react-scripts") || has("public/index.html") {
            return FrameworkDetection {
                framework: DetectedFramework::CreateReactApp,
                build_command: "react-scripts build".to_string(),
                output_dir: "build".to_string(),
                dev_command: "react-scripts start".to_string(),
                install_command: "npm install".to_string(),
            };
        }
        if has("Gemfile") && has("config/routes.rb") {
            return FrameworkDetection {
                framework: DetectedFramework::Rails,
                build_command: "bundle exec rails assets:precompile".to_string(),
                output_dir: "public".to_string(),
                dev_command: "bundle exec rails server".to_string(),
                install_command: "bundle install".to_string(),
            };
        }
        if has("manage.py") && has("settings.py") {
            return FrameworkDetection {
                framework: DetectedFramework::Django,
                build_command: "python manage.py collectstatic --noinput".to_string(),
                output_dir: "staticfiles".to_string(),
                dev_command: "python manage.py runserver".to_string(),
                install_command: "pip install -r requirements.txt".to_string(),
            };
        }
        if has("requirements.txt") && has("main.py") {
            return FrameworkDetection {
                framework: DetectedFramework::FastApi,
                build_command: "echo 'no build step'".to_string(),
                output_dir: ".".to_string(),
                dev_command: "uvicorn main:app --reload".to_string(),
                install_command: "pip install -r requirements.txt".to_string(),
            };
        }
        if has("package.json") && has("server.js") {
            return FrameworkDetection {
                framework: DetectedFramework::Express,
                build_command: "echo 'no build step'".to_string(),
                output_dir: ".".to_string(),
                dev_command: "node server.js".to_string(),
                install_command: "npm install".to_string(),
            };
        }
        if has("index.html") {
            return FrameworkDetection {
                framework: DetectedFramework::Static,
                build_command: "echo 'no build step'".to_string(),
                output_dir: ".".to_string(),
                dev_command: "npx serve .".to_string(),
                install_command: "echo 'no install step'".to_string(),
            };
        }

        FrameworkDetection {
            framework: DetectedFramework::Unknown,
            build_command: "echo 'unknown framework'".to_string(),
            output_dir: ".".to_string(),
            dev_command: "echo 'unknown framework'".to_string(),
            install_command: "echo 'unknown framework'".to_string(),
        }
    }

    /// Start a new deployment, returning the deploy ID.
    pub fn deploy(
        &mut self,
        project_name: &str,
        platform: DeployPlatform,
        branch: &str,
    ) -> Result<String, DeployError> {
        // Check for duplicate active deployment on same project+branch+platform.
        let duplicate = self.deployments.iter().any(|d| {
            d.project_name == project_name
                && d.branch == branch
                && d.platform == platform
                && matches!(
                    d.status,
                    DeployStatus::Pending | DeployStatus::Building | DeployStatus::Deploying
                )
        });
        if duplicate {
            return Err(DeployError::DuplicateDeployment(format!(
                "{project_name} on {platform} ({branch}) already deploying"
            )));
        }

        let id = format!("deploy-{}", self.next_id);
        self.next_id += 1;

        let deployment = Deployment {
            id: id.clone(),
            project_name: project_name.to_string(),
            platform,
            url: None,
            preview_url: None,
            status: DeployStatus::Pending,
            commit_ref: None,
            branch: branch.to_string(),
            created_at: current_timestamp(),
            completed_at: None,
            build_logs: Vec::new(),
            env_vars: Vec::new(),
            error: None,
        };

        self.deployments.push(deployment);
        Ok(id)
    }

    pub fn get_deployment(&self, id: &str) -> Option<&Deployment> {
        self.deployments.iter().find(|d| d.id == id)
    }

    pub fn list_deployments(&self) -> Vec<&Deployment> {
        self.deployments.iter().collect()
    }

    pub fn list_active_deployments(&self) -> Vec<&Deployment> {
        self.deployments
            .iter()
            .filter(|d| {
                matches!(
                    d.status,
                    DeployStatus::Pending
                        | DeployStatus::Building
                        | DeployStatus::Deploying
                        | DeployStatus::Live
                        | DeployStatus::Preview
                )
            })
            .collect()
    }

    /// Generate the CLI command for a given platform and framework.
    pub fn generate_deploy_command(
        platform: DeployPlatform,
        framework: &FrameworkDetection,
    ) -> DeployCommand {
        match platform {
            DeployPlatform::Vercel => DeployCommand {
                platform: DeployPlatform::Vercel,
                args: vec![
                    "vercel".to_string(),
                    "deploy".to_string(),
                    "--prod".to_string(),
                ],
                env: vec![],
                description: format!("Deploy {} project to Vercel", framework.framework),
            },
            DeployPlatform::Netlify => DeployCommand {
                platform: DeployPlatform::Netlify,
                args: vec![
                    "netlify".to_string(),
                    "deploy".to_string(),
                    "--prod".to_string(),
                    "--dir".to_string(),
                    framework.output_dir.clone(),
                ],
                env: vec![],
                description: format!("Deploy {} project to Netlify", framework.framework),
            },
            DeployPlatform::FlyIo => DeployCommand {
                platform: DeployPlatform::FlyIo,
                args: vec![
                    "fly".to_string(),
                    "deploy".to_string(),
                ],
                env: vec![],
                description: format!("Deploy {} project to Fly.io", framework.framework),
            },
            DeployPlatform::Railway => DeployCommand {
                platform: DeployPlatform::Railway,
                args: vec![
                    "railway".to_string(),
                    "up".to_string(),
                ],
                env: vec![],
                description: format!("Deploy {} project to Railway", framework.framework),
            },
            DeployPlatform::Render => DeployCommand {
                platform: DeployPlatform::Render,
                args: vec![
                    "render".to_string(),
                    "deploy".to_string(),
                ],
                env: vec![],
                description: format!("Deploy {} project to Render", framework.framework),
            },
            DeployPlatform::DockerSelfHosted => DeployCommand {
                platform: DeployPlatform::DockerSelfHosted,
                args: vec![
                    "docker".to_string(),
                    "compose".to_string(),
                    "up".to_string(),
                    "-d".to_string(),
                    "--build".to_string(),
                ],
                env: vec![],
                description: format!(
                    "Deploy {} project via Docker Compose",
                    framework.framework
                ),
            },
        }
    }

    /// Generate a `vercel.json` config string.
    pub fn generate_vercel_config(framework: &FrameworkDetection) -> String {
        format!(
            r#"{{
  "buildCommand": "{}",
  "outputDirectory": "{}",
  "devCommand": "{}",
  "installCommand": "{}",
  "framework": "{}"
}}"#,
            framework.build_command,
            framework.output_dir,
            framework.dev_command,
            framework.install_command,
            framework.framework,
        )
    }

    /// Generate a `netlify.toml` config string.
    pub fn generate_netlify_config(framework: &FrameworkDetection) -> String {
        format!(
            r#"[build]
  command = "{}"
  publish = "{}"

[dev]
  command = "{}"
"#,
            framework.build_command, framework.output_dir, framework.dev_command,
        )
    }

    /// Generate a `fly.toml` config string.
    pub fn generate_fly_config(framework: &FrameworkDetection, app_name: &str) -> String {
        format!(
            r#"app = "{app_name}"
primary_region = "iad"

[build]
  builder = "heroku/buildpacks:22"

[env]
  BUILD_COMMAND = "{}"
  OUTPUT_DIR = "{}"

[[services]]
  internal_port = 8080
  protocol = "tcp"

  [[services.ports]]
    port = 80
    handlers = ["http"]

  [[services.ports]]
    port = 443
    handlers = ["tls", "http"]

  [[services.http_checks]]
    interval = 10000
    grace_period = "5s"
    method = "GET"
    path = "/health"
    protocol = "http"
    timeout = 2000
"#,
            framework.build_command, framework.output_dir,
        )
    }

    /// Generate a Dockerfile for the detected framework.
    pub fn generate_dockerfile(framework: &FrameworkDetection) -> String {
        match framework.framework {
            DetectedFramework::NextJs
            | DetectedFramework::Vite
            | DetectedFramework::Remix
            | DetectedFramework::Gatsby
            | DetectedFramework::Nuxt
            | DetectedFramework::SvelteKit
            | DetectedFramework::Astro
            | DetectedFramework::CreateReactApp => {
                format!(
                    r#"FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN {}
COPY . .
RUN {}

FROM nginx:alpine
COPY --from=builder /app/{} /usr/share/nginx/html
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
"#,
                    framework.install_command, framework.build_command, framework.output_dir,
                )
            }
            DetectedFramework::Express => {
                r#"FROM node:20-alpine
WORKDIR /app
COPY package*.json ./
RUN npm install --production
COPY . .
EXPOSE 8080
CMD ["node", "server.js"]
"#
                .to_string()
            }
            DetectedFramework::FastApi => {
                r#"FROM python:3.12-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY . .
EXPOSE 8080
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8080"]
"#
                .to_string()
            }
            DetectedFramework::Rails => {
                r#"FROM ruby:3.3-slim
WORKDIR /app
COPY Gemfile Gemfile.lock ./
RUN bundle install --without development test
COPY . .
RUN bundle exec rails assets:precompile
EXPOSE 3000
CMD ["bundle", "exec", "rails", "server", "-b", "0.0.0.0"]
"#
                .to_string()
            }
            DetectedFramework::Django => {
                r#"FROM python:3.12-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY . .
RUN python manage.py collectstatic --noinput
EXPOSE 8080
CMD ["gunicorn", "config.wsgi:application", "--bind", "0.0.0.0:8080"]
"#
                .to_string()
            }
            DetectedFramework::Static => {
                r#"FROM nginx:alpine
COPY . /usr/share/nginx/html
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
"#
                .to_string()
            }
            DetectedFramework::Unknown => {
                r#"FROM ubuntu:22.04
WORKDIR /app
COPY . .
EXPOSE 8080
CMD ["echo", "Configure your start command"]
"#
                .to_string()
            }
        }
    }

    /// Mark a deployment as complete (Live).
    pub fn complete_deployment(&mut self, id: &str, url: &str) -> Result<(), DeployError> {
        let deploy = self
            .deployments
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or_else(|| DeployError::ProjectNotFound(id.to_string()))?;

        deploy.status = DeployStatus::Live;
        deploy.url = Some(url.to_string());
        deploy.completed_at = Some(current_timestamp());
        Ok(())
    }

    /// Mark a deployment as failed.
    pub fn fail_deployment(&mut self, id: &str, error: &str) -> Result<(), DeployError> {
        let deploy = self
            .deployments
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or_else(|| DeployError::ProjectNotFound(id.to_string()))?;

        deploy.status = DeployStatus::Failed;
        deploy.error = Some(error.to_string());
        deploy.completed_at = Some(current_timestamp());
        Ok(())
    }

    /// Simulate a health check against a URL.
    pub fn check_health(url: &str) -> HealthCheck {
        // In production this would make an actual HTTP request; here we
        // simulate a healthy response for well-formed URLs.
        let healthy = url.starts_with("https://") || url.starts_with("http://");
        HealthCheck {
            url: url.to_string(),
            status_code: if healthy { 200 } else { 0 },
            response_time_ms: if healthy { 42 } else { 0 },
            healthy,
            checked_at: current_timestamp(),
            error: if healthy {
                None
            } else {
                Some("invalid url scheme".to_string())
            },
        }
    }

    /// Roll back a deployment to the previous successful deploy.
    pub fn rollback(&mut self, deploy_id: &str, reason: &str) -> Result<RollbackInfo, DeployError> {
        let deploy = self
            .deployments
            .iter()
            .find(|d| d.id == deploy_id)
            .ok_or_else(|| DeployError::ProjectNotFound(deploy_id.to_string()))?;

        let project = deploy.project_name.clone();
        let platform = deploy.platform.clone();

        // Find the most recent Live deployment for the same project+platform
        // that is NOT the one being rolled back.
        let previous = self
            .deployments
            .iter()
            .filter(|d| {
                d.project_name == project
                    && d.platform == platform
                    && d.id != deploy_id
                    && d.status == DeployStatus::Live
            })
            .next_back();

        let rollback_to_id = match previous {
            Some(prev) => prev.id.clone(),
            None => {
                return Err(DeployError::RollbackFailed(
                    "no previous successful deployment found".to_string(),
                ));
            }
        };

        // Mark the current deployment as rolled back.
        if let Some(d) = self.deployments.iter_mut().find(|d| d.id == deploy_id) {
            d.status = DeployStatus::RolledBack;
        }

        Ok(RollbackInfo {
            deployment_id: deploy_id.to_string(),
            rollback_to_id,
            reason: reason.to_string(),
            timestamp: current_timestamp(),
            success: true,
        })
    }

    /// Configure a custom domain for a deployment.
    pub fn configure_domain(
        &mut self,
        deploy_id: &str,
        domain: DomainConfig,
    ) -> Result<(), DeployError> {
        if !self.deployments.iter().any(|d| d.id == deploy_id) {
            return Err(DeployError::ProjectNotFound(deploy_id.to_string()));
        }
        if domain.domain.is_empty() {
            return Err(DeployError::DomainConfigError(
                "domain name cannot be empty".to_string(),
            ));
        }
        self.domains.insert(deploy_id.to_string(), domain);
        Ok(())
    }

    /// Get all deployments for a given project, ordered by creation time.
    pub fn get_deploy_history(&self, project_name: &str) -> Vec<&Deployment> {
        self.deployments
            .iter()
            .filter(|d| d.project_name == project_name)
            .collect()
    }

    /// Estimate build time in seconds based on framework.
    pub fn estimate_build_time(framework: &DetectedFramework) -> u64 {
        match framework {
            DetectedFramework::NextJs => 120,
            DetectedFramework::Vite => 30,
            DetectedFramework::Remix => 90,
            DetectedFramework::Gatsby => 150,
            DetectedFramework::Nuxt => 100,
            DetectedFramework::SvelteKit => 45,
            DetectedFramework::Astro => 40,
            DetectedFramework::CreateReactApp => 60,
            DetectedFramework::Static => 5,
            DetectedFramework::Express => 15,
            DetectedFramework::FastApi => 20,
            DetectedFramework::Rails => 90,
            DetectedFramework::Django => 30,
            DetectedFramework::Unknown => 60,
        }
    }

    /// Append a build log line to a deployment.
    pub fn add_build_log(&mut self, deploy_id: &str, log: &str) -> Result<(), DeployError> {
        let deploy = self
            .deployments
            .iter_mut()
            .find(|d| d.id == deploy_id)
            .ok_or_else(|| DeployError::ProjectNotFound(deploy_id.to_string()))?;

        deploy.build_logs.push(log.to_string());
        Ok(())
    }
}

/// Simple monotonic-ish timestamp helper (seconds since epoch).
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    fn default_manager() -> DeployManager {
        DeployManager::new(DeployConfig::default())
    }

    // --- Config defaults ---

    #[test]
    fn test_config_defaults() {
        let cfg = DeployConfig::default();
        assert_eq!(cfg.default_platform, DeployPlatform::Vercel);
        assert!(cfg.auto_detect_framework);
        assert_eq!(cfg.env_file, ".env");
        assert!(cfg.preview_deploys);
        assert!(cfg.auto_rollback_on_error);
        assert_eq!(cfg.health_check_timeout_secs, 60);
    }

    #[test]
    fn test_domain_config_defaults() {
        let dc = DomainConfig::default();
        assert!(dc.ssl_enabled);
        assert!(dc.redirect_www);
        assert!(dc.custom_headers.is_empty());
    }

    // --- Platform enum ---

    #[test]
    fn test_platform_as_str() {
        assert_eq!(DeployPlatform::Vercel.as_str(), "vercel");
        assert_eq!(DeployPlatform::Netlify.as_str(), "netlify");
        assert_eq!(DeployPlatform::FlyIo.as_str(), "fly.io");
        assert_eq!(DeployPlatform::Railway.as_str(), "railway");
        assert_eq!(DeployPlatform::Render.as_str(), "render");
        assert_eq!(DeployPlatform::DockerSelfHosted.as_str(), "docker-self-hosted");
    }

    #[test]
    fn test_platform_display() {
        assert_eq!(format!("{}", DeployPlatform::FlyIo), "fly.io");
    }

    // --- Framework detection ---

    #[test]
    fn test_detect_nextjs() {
        let files = vec!["package.json".into(), "next.config.js".into()];
        let det = DeployManager::detect_framework(&files);
        assert_eq!(det.framework, DetectedFramework::NextJs);
        assert_eq!(det.build_command, "next build");
        assert_eq!(det.output_dir, ".next");
    }

    #[test]
    fn test_detect_vite() {
        let files = vec!["package.json".into(), "vite.config.ts".into()];
        let det = DeployManager::detect_framework(&files);
        assert_eq!(det.framework, DetectedFramework::Vite);
        assert_eq!(det.output_dir, "dist");
    }

    #[test]
    fn test_detect_remix() {
        let files = vec!["package.json".into(), "remix.config.js".into()];
        let det = DeployManager::detect_framework(&files);
        assert_eq!(det.framework, DetectedFramework::Remix);
    }

    #[test]
    fn test_detect_sveltekit() {
        let files = vec!["package.json".into(), "svelte.config.js".into()];
        let det = DeployManager::detect_framework(&files);
        assert_eq!(det.framework, DetectedFramework::SvelteKit);
    }

    #[test]
    fn test_detect_astro() {
        let files = vec!["package.json".into(), "astro.config.mjs".into()];
        let det = DeployManager::detect_framework(&files);
        assert_eq!(det.framework, DetectedFramework::Astro);
    }

    #[test]
    fn test_detect_static() {
        let files = vec!["index.html".into(), "style.css".into()];
        let det = DeployManager::detect_framework(&files);
        assert_eq!(det.framework, DetectedFramework::Static);
    }

    #[test]
    fn test_detect_express() {
        let files = vec!["package.json".into(), "server.js".into()];
        let det = DeployManager::detect_framework(&files);
        assert_eq!(det.framework, DetectedFramework::Express);
    }

    #[test]
    fn test_detect_fastapi() {
        let files = vec!["requirements.txt".into(), "main.py".into()];
        let det = DeployManager::detect_framework(&files);
        assert_eq!(det.framework, DetectedFramework::FastApi);
    }

    #[test]
    fn test_detect_rails() {
        let files = vec!["Gemfile".into(), "config/routes.rb".into()];
        let det = DeployManager::detect_framework(&files);
        assert_eq!(det.framework, DetectedFramework::Rails);
    }

    #[test]
    fn test_detect_django() {
        let files = vec!["manage.py".into(), "settings.py".into(), "requirements.txt".into()];
        let det = DeployManager::detect_framework(&files);
        assert_eq!(det.framework, DetectedFramework::Django);
    }

    #[test]
    fn test_detect_unknown() {
        let files = vec!["README.md".into()];
        let det = DeployManager::detect_framework(&files);
        assert_eq!(det.framework, DetectedFramework::Unknown);
    }

    // --- Deploy lifecycle ---

    #[test]
    fn test_deploy_creates_pending() {
        let mut mgr = default_manager();
        let id = mgr.deploy("myapp", DeployPlatform::Vercel, "main").unwrap();
        let d = mgr.get_deployment(&id).unwrap();
        assert_eq!(d.status, DeployStatus::Pending);
        assert_eq!(d.project_name, "myapp");
        assert_eq!(d.branch, "main");
    }

    #[test]
    fn test_deploy_lifecycle_pending_to_live() {
        let mut mgr = default_manager();
        let id = mgr.deploy("app", DeployPlatform::Netlify, "main").unwrap();
        assert_eq!(mgr.get_deployment(&id).unwrap().status, DeployStatus::Pending);

        mgr.complete_deployment(&id, "https://app.netlify.app").unwrap();
        let d = mgr.get_deployment(&id).unwrap();
        assert_eq!(d.status, DeployStatus::Live);
        assert_eq!(d.url.as_deref(), Some("https://app.netlify.app"));
        assert!(d.completed_at.is_some());
    }

    #[test]
    fn test_deploy_lifecycle_fail() {
        let mut mgr = default_manager();
        let id = mgr.deploy("app", DeployPlatform::Vercel, "main").unwrap();
        mgr.fail_deployment(&id, "OOM during build").unwrap();
        let d = mgr.get_deployment(&id).unwrap();
        assert_eq!(d.status, DeployStatus::Failed);
        assert_eq!(d.error.as_deref(), Some("OOM during build"));
    }

    #[test]
    fn test_duplicate_deployment_error() {
        let mut mgr = default_manager();
        mgr.deploy("app", DeployPlatform::Vercel, "main").unwrap();
        let res = mgr.deploy("app", DeployPlatform::Vercel, "main");
        assert!(matches!(res, Err(DeployError::DuplicateDeployment(_))));
    }

    #[test]
    fn test_complete_nonexistent_deployment() {
        let mut mgr = default_manager();
        let res = mgr.complete_deployment("nope", "https://x.com");
        assert!(matches!(res, Err(DeployError::ProjectNotFound(_))));
    }

    #[test]
    fn test_fail_nonexistent_deployment() {
        let mut mgr = default_manager();
        let res = mgr.fail_deployment("nope", "err");
        assert!(matches!(res, Err(DeployError::ProjectNotFound(_))));
    }

    // --- Deploy commands per platform ---

    #[test]
    fn test_deploy_command_vercel() {
        let fw = DeployManager::detect_framework(&["next.config.js".into()]);
        let cmd = DeployManager::generate_deploy_command(DeployPlatform::Vercel, &fw);
        assert_eq!(cmd.args[0], "vercel");
        assert!(cmd.description.contains("Vercel"));
    }

    #[test]
    fn test_deploy_command_netlify() {
        let fw = DeployManager::detect_framework(&["vite.config.ts".into()]);
        let cmd = DeployManager::generate_deploy_command(DeployPlatform::Netlify, &fw);
        assert_eq!(cmd.args[0], "netlify");
        assert!(cmd.args.contains(&"dist".to_string()));
    }

    #[test]
    fn test_deploy_command_flyio() {
        let fw = DeployManager::detect_framework(&["package.json".into(), "server.js".into()]);
        let cmd = DeployManager::generate_deploy_command(DeployPlatform::FlyIo, &fw);
        assert_eq!(cmd.args[0], "fly");
    }

    #[test]
    fn test_deploy_command_railway() {
        let fw = DeployManager::detect_framework(&["index.html".into()]);
        let cmd = DeployManager::generate_deploy_command(DeployPlatform::Railway, &fw);
        assert_eq!(cmd.args[0], "railway");
    }

    #[test]
    fn test_deploy_command_docker() {
        let fw = DeployManager::detect_framework(&["index.html".into()]);
        let cmd = DeployManager::generate_deploy_command(DeployPlatform::DockerSelfHosted, &fw);
        assert_eq!(cmd.args[0], "docker");
        assert!(cmd.args.contains(&"--build".to_string()));
    }

    // --- Config generation ---

    #[test]
    fn test_generate_vercel_config() {
        let fw = DeployManager::detect_framework(&["next.config.js".into()]);
        let cfg = DeployManager::generate_vercel_config(&fw);
        assert!(cfg.contains("\"buildCommand\": \"next build\""));
        assert!(cfg.contains("\"outputDirectory\": \".next\""));
    }

    #[test]
    fn test_generate_netlify_config() {
        let fw = DeployManager::detect_framework(&["vite.config.ts".into()]);
        let cfg = DeployManager::generate_netlify_config(&fw);
        assert!(cfg.contains("command = \"vite build\""));
        assert!(cfg.contains("publish = \"dist\""));
    }

    #[test]
    fn test_generate_fly_config() {
        let fw = DeployManager::detect_framework(&["package.json".into(), "server.js".into()]);
        let cfg = DeployManager::generate_fly_config(&fw, "my-express-app");
        assert!(cfg.contains("app = \"my-express-app\""));
        assert!(cfg.contains("internal_port = 8080"));
    }

    #[test]
    fn test_generate_dockerfile_node() {
        let fw = DeployManager::detect_framework(&["next.config.js".into()]);
        let df = DeployManager::generate_dockerfile(&fw);
        assert!(df.contains("FROM node:20-alpine"));
        assert!(df.contains("next build"));
    }

    #[test]
    fn test_generate_dockerfile_fastapi() {
        let fw = DeployManager::detect_framework(&["requirements.txt".into(), "main.py".into()]);
        let df = DeployManager::generate_dockerfile(&fw);
        assert!(df.contains("FROM python:3.12-slim"));
        assert!(df.contains("uvicorn"));
    }

    #[test]
    fn test_generate_dockerfile_static() {
        let fw = DeployManager::detect_framework(&["index.html".into()]);
        let df = DeployManager::generate_dockerfile(&fw);
        assert!(df.contains("FROM nginx:alpine"));
    }

    // --- Health checks ---

    #[test]
    fn test_health_check_healthy() {
        let hc = DeployManager::check_health("https://myapp.vercel.app/health");
        assert!(hc.healthy);
        assert_eq!(hc.status_code, 200);
        assert!(hc.error.is_none());
    }

    #[test]
    fn test_health_check_unhealthy() {
        let hc = DeployManager::check_health("ftp://bad-url");
        assert!(!hc.healthy);
        assert_eq!(hc.status_code, 0);
        assert!(hc.error.is_some());
    }

    #[test]
    fn test_health_check_http() {
        let hc = DeployManager::check_health("http://localhost:3000");
        assert!(hc.healthy);
    }

    // --- Rollback ---

    #[test]
    fn test_rollback_success() {
        let mut mgr = default_manager();
        let id1 = mgr.deploy("app", DeployPlatform::Vercel, "main").unwrap();
        mgr.complete_deployment(&id1, "https://v1.app").unwrap();

        let id2 = mgr.deploy("app", DeployPlatform::Vercel, "main").unwrap();
        mgr.complete_deployment(&id2, "https://v2.app").unwrap();

        let info = mgr.rollback(&id2, "regression").unwrap();
        assert!(info.success);
        assert_eq!(info.rollback_to_id, id1);
        assert_eq!(mgr.get_deployment(&id2).unwrap().status, DeployStatus::RolledBack);
    }

    #[test]
    fn test_rollback_no_previous() {
        let mut mgr = default_manager();
        let id = mgr.deploy("app", DeployPlatform::Vercel, "main").unwrap();
        mgr.complete_deployment(&id, "https://v1.app").unwrap();

        let res = mgr.rollback(&id, "oops");
        assert!(matches!(res, Err(DeployError::RollbackFailed(_))));
    }

    #[test]
    fn test_rollback_nonexistent() {
        let mut mgr = default_manager();
        let res = mgr.rollback("nope", "reason");
        assert!(matches!(res, Err(DeployError::ProjectNotFound(_))));
    }

    // --- Domain configuration ---

    #[test]
    fn test_configure_domain() {
        let mut mgr = default_manager();
        let id = mgr.deploy("app", DeployPlatform::Vercel, "main").unwrap();
        let domain = DomainConfig {
            domain: "example.com".to_string(),
            ..DomainConfig::default()
        };
        mgr.configure_domain(&id, domain).unwrap();
        assert!(mgr.domains.contains_key(&id));
    }

    #[test]
    fn test_configure_domain_empty_name() {
        let mut mgr = default_manager();
        let id = mgr.deploy("app", DeployPlatform::Vercel, "main").unwrap();
        let domain = DomainConfig::default();
        let res = mgr.configure_domain(&id, domain);
        assert!(matches!(res, Err(DeployError::DomainConfigError(_))));
    }

    #[test]
    fn test_configure_domain_nonexistent_deploy() {
        let mut mgr = default_manager();
        let domain = DomainConfig {
            domain: "example.com".to_string(),
            ..DomainConfig::default()
        };
        let res = mgr.configure_domain("nope", domain);
        assert!(matches!(res, Err(DeployError::ProjectNotFound(_))));
    }

    // --- Deploy history ---

    #[test]
    fn test_deploy_history() {
        let mut mgr = default_manager();
        mgr.deploy("app", DeployPlatform::Vercel, "main").unwrap();
        mgr.complete_deployment("deploy-1", "https://v1.app").unwrap();
        mgr.deploy("app", DeployPlatform::Vercel, "feat").unwrap();
        mgr.deploy("other", DeployPlatform::Netlify, "main").unwrap();

        let history = mgr.get_deploy_history("app");
        assert_eq!(history.len(), 2);
    }

    // --- Active deployments ---

    #[test]
    fn test_active_deployments() {
        let mut mgr = default_manager();
        let id1 = mgr.deploy("a", DeployPlatform::Vercel, "main").unwrap();
        mgr.complete_deployment(&id1, "https://a.app").unwrap();

        mgr.deploy("b", DeployPlatform::Netlify, "main").unwrap();

        let id3 = mgr.deploy("c", DeployPlatform::FlyIo, "main").unwrap();
        mgr.fail_deployment(&id3, "boom").unwrap();

        let active = mgr.list_active_deployments();
        // id1 is Live (active), id2 is Pending (active), id3 is Failed (not active)
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn test_list_deployments() {
        let mut mgr = default_manager();
        mgr.deploy("a", DeployPlatform::Vercel, "main").unwrap();
        mgr.deploy("b", DeployPlatform::Netlify, "dev").unwrap();
        assert_eq!(mgr.list_deployments().len(), 2);
    }

    // --- Build logs ---

    #[test]
    fn test_add_build_log() {
        let mut mgr = default_manager();
        let id = mgr.deploy("app", DeployPlatform::Vercel, "main").unwrap();
        mgr.add_build_log(&id, "Installing dependencies...").unwrap();
        mgr.add_build_log(&id, "Build complete.").unwrap();
        let d = mgr.get_deployment(&id).unwrap();
        assert_eq!(d.build_logs.len(), 2);
        assert_eq!(d.build_logs[0], "Installing dependencies...");
    }

    #[test]
    fn test_add_build_log_nonexistent() {
        let mut mgr = default_manager();
        let res = mgr.add_build_log("nope", "log line");
        assert!(matches!(res, Err(DeployError::ProjectNotFound(_))));
    }

    // --- Build time estimation ---

    #[test]
    fn test_estimate_build_time_nextjs() {
        assert_eq!(DeployManager::estimate_build_time(&DetectedFramework::NextJs), 120);
    }

    #[test]
    fn test_estimate_build_time_static() {
        assert_eq!(DeployManager::estimate_build_time(&DetectedFramework::Static), 5);
    }

    #[test]
    fn test_estimate_build_time_vite_fast() {
        let t = DeployManager::estimate_build_time(&DetectedFramework::Vite);
        assert!(t < DeployManager::estimate_build_time(&DetectedFramework::NextJs));
    }

    // --- Error display ---

    #[test]
    fn test_deploy_error_display() {
        let e = DeployError::BuildFailed("OOM".to_string());
        assert_eq!(format!("{e}"), "build failed: OOM");
    }

    // --- Deploy status display ---

    #[test]
    fn test_deploy_status_display() {
        assert_eq!(format!("{}", DeployStatus::Live), "live");
        assert_eq!(format!("{}", DeployStatus::RolledBack), "rolled-back");
    }

    // --- Framework display ---

    #[test]
    fn test_detected_framework_display() {
        assert_eq!(format!("{}", DetectedFramework::NextJs), "next.js");
        assert_eq!(format!("{}", DetectedFramework::FastApi), "fastapi");
    }

    // --- Gatsby detection ---

    #[test]
    fn test_detect_gatsby() {
        let files = vec!["package.json".into(), "gatsby-config.js".into()];
        let det = DeployManager::detect_framework(&files);
        assert_eq!(det.framework, DetectedFramework::Gatsby);
    }

    // --- Nuxt detection ---

    #[test]
    fn test_detect_nuxt() {
        let files = vec!["package.json".into(), "nuxt.config.ts".into()];
        let det = DeployManager::detect_framework(&files);
        assert_eq!(det.framework, DetectedFramework::Nuxt);
    }

    // --- Render deploy command ---

    #[test]
    fn test_deploy_command_render() {
        let fw = DeployManager::detect_framework(&["index.html".into()]);
        let cmd = DeployManager::generate_deploy_command(DeployPlatform::Render, &fw);
        assert_eq!(cmd.args[0], "render");
    }
}
