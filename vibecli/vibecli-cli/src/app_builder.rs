//! App Builder module — Bolt.new-inspired features for VibeCody.
//!
//! Provides template management, AI-enhanced spec generation, project scaffolding,
//! resource provisioning, and managed backend configuration.
//!
//! ## Key Components
//! - `AppTemplate` / `TeamTemplateStore` — Reusable project templates with team sharing
//! - `AIEnhancer` — Convert rough ideas into structured `EnhancedSpec`s via heuristics
//! - `AppProvisioner` — Auto-provision databases, auth, hosting, and env files
//! - `AppScaffolder` — Generate full project file trees from templates or specs
//! - `ManagedBackend` — Unified backend config → docker-compose / deployment manifests

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

// ── TemplateCategory ──────────────────────────────────────────────────────────

/// Category of an app template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TemplateCategory {
    #[default]
    Web,
    Mobile,
    Api,
    FullStack,
    Landing,
    Dashboard,
}

impl fmt::Display for TemplateCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Web => write!(f, "web"),
            Self::Mobile => write!(f, "mobile"),
            Self::Api => write!(f, "api"),
            Self::FullStack => write!(f, "full_stack"),
            Self::Landing => write!(f, "landing"),
            Self::Dashboard => write!(f, "dashboard"),
        }
    }
}

// ── AppTemplate ───────────────────────────────────────────────────────────────

/// A reusable project template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTemplate {
    /// Unique template identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of what this template provides.
    pub description: String,
    /// Template category.
    pub category: TemplateCategory,
    /// Technology stack labels (e.g. "React", "Node.js", "PostgreSQL").
    pub tech_stack: Vec<String>,
    /// File map: relative path → file content.
    pub files: HashMap<String, String>,
    /// Optional database schema SQL.
    #[serde(default)]
    pub database_schema: Option<String>,
    /// Required environment variable names.
    #[serde(default)]
    pub env_vars: Vec<String>,
}

impl AppTemplate {
    /// Create a new template with the minimum required fields.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        category: TemplateCategory,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            category,
            tech_stack: vec![],
            files: HashMap::new(),
            database_schema: None,
            env_vars: vec![],
        }
    }

    /// Builder: set description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Builder: add a file to the template.
    pub fn with_file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.files.insert(path.into(), content.into());
        self
    }

    /// Builder: set tech stack.
    pub fn with_tech_stack(mut self, stack: Vec<String>) -> Self {
        self.tech_stack = stack;
        self
    }

    /// Builder: set env vars.
    pub fn with_env_vars(mut self, vars: Vec<String>) -> Self {
        self.env_vars = vars;
        self
    }

    /// Builder: set database schema.
    pub fn with_database_schema(mut self, schema: impl Into<String>) -> Self {
        self.database_schema = Some(schema.into());
        self
    }
}

impl fmt::Display for AppTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} ({}) — {} files, stack: {}",
            self.id,
            self.name,
            self.category,
            self.files.len(),
            if self.tech_stack.is_empty() {
                "none".to_string()
            } else {
                self.tech_stack.join(", ")
            }
        )
    }
}

// ── TeamTemplateStore ─────────────────────────────────────────────────────────

/// Manages saving, loading, and sharing of project templates.
///
/// Templates are stored as JSON files in `~/.vibecli/templates/`.
pub struct TeamTemplateStore {
    templates_dir: PathBuf,
}

impl TeamTemplateStore {
    /// Create a store using the default templates directory.
    pub fn new() -> Self {
        let home = dirs_home();
        Self {
            templates_dir: home.join(".vibecli").join("templates"),
        }
    }

    /// Create a store rooted at a custom directory (useful for testing).
    pub fn with_dir(dir: PathBuf) -> Self {
        Self { templates_dir: dir }
    }

    /// Ensure the templates directory exists.
    fn ensure_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.templates_dir)?;
        Ok(())
    }

    /// Save a template from a workspace. The template is serialized as JSON.
    pub fn save_template(&self, _workspace: &Path, template: &AppTemplate) -> Result<()> {
        self.ensure_dir()?;
        let path = self.templates_dir.join(format!("{}.json", template.id));
        let json = serde_json::to_string_pretty(template)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// List all available templates.
    pub fn list_templates(&self) -> Vec<AppTemplate> {
        if !self.templates_dir.is_dir() {
            return vec![];
        }
        let mut templates = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.templates_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(tmpl) = serde_json::from_str::<AppTemplate>(&content) {
                            templates.push(tmpl);
                        }
                    }
                }
            }
        }
        templates.sort_by(|a, b| a.id.cmp(&b.id));
        templates
    }

    /// Load a single template by ID.
    pub fn load_template(&self, id: &str) -> Option<AppTemplate> {
        let path = self.templates_dir.join(format!("{}.json", id));
        if !path.is_file() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Delete a template by ID. Returns true if it existed.
    pub fn delete_template(&self, id: &str) -> bool {
        let path = self.templates_dir.join(format!("{}.json", id));
        if path.is_file() {
            std::fs::remove_file(&path).is_ok()
        } else {
            false
        }
    }

    /// Export a template to a JSON string.
    pub fn export_template(&self, id: &str) -> Option<String> {
        let tmpl = self.load_template(id)?;
        serde_json::to_string_pretty(&tmpl).ok()
    }

    /// Import a template from a JSON string.
    pub fn import_template(&self, json: &str) -> Result<AppTemplate> {
        let template: AppTemplate = serde_json::from_str(json)
            .map_err(|e| anyhow::anyhow!("Invalid template JSON: {}", e))?;
        if template.id.is_empty() {
            anyhow::bail!("Template ID cannot be empty");
        }
        self.save_template(Path::new("."), &template)?;
        Ok(template)
    }
}

// ── DatabaseType / AuthType / HostingTarget ───────────────────────────────────

/// Database types for provisioning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseType {
    #[default]
    Sqlite,
    Postgres,
    Supabase,
}

impl fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite => write!(f, "sqlite"),
            Self::Postgres => write!(f, "postgres"),
            Self::Supabase => write!(f, "supabase"),
        }
    }
}

/// Authentication providers for provisioning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    #[default]
    Jwt,
    OAuth,
    Supabase,
}

impl fmt::Display for AuthType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jwt => write!(f, "jwt"),
            Self::OAuth => write!(f, "oauth"),
            Self::Supabase => write!(f, "supabase"),
        }
    }
}

/// Hosting targets for deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HostingTarget {
    #[default]
    Vercel,
    Netlify,
    Railway,
    BoltHost,
}

impl fmt::Display for HostingTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vercel => write!(f, "vercel"),
            Self::Netlify => write!(f, "netlify"),
            Self::Railway => write!(f, "railway"),
            Self::BoltHost => write!(f, "bolt_host"),
        }
    }
}

// ── ProvisionConfig ───────────────────────────────────────────────────────────

/// Configuration for auto-provisioning project resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionConfig {
    /// Whether to provision a database (and which type).
    pub database: Option<DatabaseType>,
    /// Whether to provision auth (and which provider).
    pub auth: Option<AuthType>,
    /// Hosting target for deployment config.
    pub hosting: Option<HostingTarget>,
    /// Whether to generate SEO boilerplate.
    #[serde(default)]
    pub seo: bool,
    /// Whether to add Stripe payment integration.
    #[serde(default)]
    pub stripe: bool,
}

impl ProvisionConfig {
    /// Create an empty provision config.
    pub fn new() -> Self {
        Self {
            database: None,
            auth: None,
            hosting: None,
            seo: false,
            stripe: false,
        }
    }

    /// Builder: set database.
    pub fn with_database(mut self, db: DatabaseType) -> Self {
        self.database = Some(db);
        self
    }

    /// Builder: set auth.
    pub fn with_auth(mut self, auth: AuthType) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Builder: set hosting.
    pub fn with_hosting(mut self, host: HostingTarget) -> Self {
        self.hosting = Some(host);
        self
    }

    /// Builder: enable SEO.
    pub fn with_seo(mut self) -> Self {
        self.seo = true;
        self
    }

    /// Builder: enable Stripe.
    pub fn with_stripe(mut self) -> Self {
        self.stripe = true;
        self
    }
}

impl Default for ProvisionConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ── AppProvisioner ────────────────────────────────────────────────────────────

/// Auto-provisions project resources: databases, auth, hosting, env files.
pub struct AppProvisioner;

impl AppProvisioner {
    /// Generate database schema file and connection config.
    pub fn provision_database(workspace: &Path, db_type: &DatabaseType) -> Result<Vec<(String, String)>> {
        let mut files = Vec::new();

        match db_type {
            DatabaseType::Sqlite => {
                files.push((
                    "db/schema.sql".to_string(),
                    "-- SQLite schema\n\
                     CREATE TABLE IF NOT EXISTS users (\n  \
                       id INTEGER PRIMARY KEY AUTOINCREMENT,\n  \
                       email TEXT NOT NULL UNIQUE,\n  \
                       name TEXT NOT NULL,\n  \
                       created_at DATETIME DEFAULT CURRENT_TIMESTAMP\n\
                     );\n\n\
                     CREATE TABLE IF NOT EXISTS sessions (\n  \
                       id TEXT PRIMARY KEY,\n  \
                       user_id INTEGER NOT NULL REFERENCES users(id),\n  \
                       expires_at DATETIME NOT NULL\n\
                     );\n"
                    .to_string(),
                ));
                files.push((
                    "src/db.ts".to_string(),
                    "import Database from 'better-sqlite3';\n\n\
                     const db = new Database(process.env.DATABASE_URL || './data.db');\n\
                     db.pragma('journal_mode = WAL');\n\n\
                     export default db;\n"
                    .to_string(),
                ));
            }
            DatabaseType::Postgres => {
                files.push((
                    "db/schema.sql".to_string(),
                    "-- PostgreSQL schema\n\
                     CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";\n\n\
                     CREATE TABLE users (\n  \
                       id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),\n  \
                       email TEXT NOT NULL UNIQUE,\n  \
                       name TEXT NOT NULL,\n  \
                       created_at TIMESTAMPTZ DEFAULT NOW()\n\
                     );\n\n\
                     CREATE TABLE sessions (\n  \
                       id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),\n  \
                       user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,\n  \
                       expires_at TIMESTAMPTZ NOT NULL\n\
                     );\n"
                    .to_string(),
                ));
                files.push((
                    "src/db.ts".to_string(),
                    "import { Pool } from 'pg';\n\n\
                     const pool = new Pool({\n  \
                       connectionString: process.env.DATABASE_URL,\n\
                     });\n\n\
                     export default pool;\n"
                    .to_string(),
                ));
            }
            DatabaseType::Supabase => {
                files.push((
                    "db/schema.sql".to_string(),
                    "-- Supabase schema (PostgreSQL + RLS)\n\
                     CREATE TABLE users (\n  \
                       id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n  \
                       email TEXT NOT NULL UNIQUE,\n  \
                       name TEXT NOT NULL,\n  \
                       created_at TIMESTAMPTZ DEFAULT NOW()\n\
                     );\n\n\
                     ALTER TABLE users ENABLE ROW LEVEL SECURITY;\n\n\
                     CREATE POLICY \"Users can view own data\" ON users\n  \
                       FOR SELECT USING (auth.uid() = id);\n"
                    .to_string(),
                ));
                files.push((
                    "src/db.ts".to_string(),
                    "import { createClient } from '@supabase/supabase-js';\n\n\
                     const supabase = createClient(\n  \
                       process.env.SUPABASE_URL!,\n  \
                       process.env.SUPABASE_ANON_KEY!\n\
                     );\n\n\
                     export default supabase;\n"
                    .to_string(),
                ));
            }
        }

        // Write files to workspace
        for (rel_path, content) in &files {
            let full_path = workspace.join(rel_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full_path, content)?;
        }

        Ok(files)
    }

    /// Generate auth boilerplate files.
    pub fn provision_auth(workspace: &Path, auth_type: &AuthType) -> Result<Vec<(String, String)>> {
        let mut files = Vec::new();

        match auth_type {
            AuthType::Jwt => {
                files.push((
                    "src/auth.ts".to_string(),
                    "import jwt from 'jsonwebtoken';\n\n\
                     const SECRET = process.env.JWT_SECRET || 'dev-secret';\n\n\
                     export function signToken(payload: Record<string, unknown>): string {\n  \
                       return jwt.sign(payload, SECRET, { expiresIn: '7d' });\n\
                     }\n\n\
                     export function verifyToken(token: string): Record<string, unknown> {\n  \
                       return jwt.verify(token, SECRET) as Record<string, unknown>;\n\
                     }\n\n\
                     export function authMiddleware(req: any, res: any, next: any) {\n  \
                       const token = req.headers.authorization?.replace('Bearer ', '');\n  \
                       if (!token) return res.status(401).json({ error: 'No token provided' });\n  \
                       try {\n    \
                         req.user = verifyToken(token);\n    \
                         next();\n  \
                       } catch {\n    \
                         res.status(401).json({ error: 'Invalid token' });\n  \
                       }\n\
                     }\n"
                    .to_string(),
                ));
            }
            AuthType::OAuth => {
                files.push((
                    "src/auth.ts".to_string(),
                    "import passport from 'passport';\n\
                     import { Strategy as GoogleStrategy } from 'passport-google-oauth20';\n\n\
                     passport.use(new GoogleStrategy({\n  \
                       clientID: process.env.GOOGLE_CLIENT_ID!,\n  \
                       clientSecret: process.env.GOOGLE_CLIENT_SECRET!,\n  \
                       callbackURL: '/auth/google/callback',\n\
                     }, (accessToken, refreshToken, profile, done) => {\n  \
                       done(null, profile);\n\
                     }));\n\n\
                     passport.serializeUser((user, done) => done(null, user));\n\
                     passport.deserializeUser((user, done) => done(null, user as any));\n\n\
                     export default passport;\n"
                    .to_string(),
                ));
            }
            AuthType::Supabase => {
                files.push((
                    "src/auth.ts".to_string(),
                    "import { createClient } from '@supabase/supabase-js';\n\n\
                     const supabase = createClient(\n  \
                       process.env.SUPABASE_URL!,\n  \
                       process.env.SUPABASE_ANON_KEY!\n\
                     );\n\n\
                     export async function signUp(email: string, password: string) {\n  \
                       return supabase.auth.signUp({ email, password });\n\
                     }\n\n\
                     export async function signIn(email: string, password: string) {\n  \
                       return supabase.auth.signInWithPassword({ email, password });\n\
                     }\n\n\
                     export async function signOut() {\n  \
                       return supabase.auth.signOut();\n\
                     }\n\n\
                     export async function getUser() {\n  \
                       return supabase.auth.getUser();\n\
                     }\n"
                    .to_string(),
                ));
            }
        }

        for (rel_path, content) in &files {
            let full_path = workspace.join(rel_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full_path, content)?;
        }

        Ok(files)
    }

    /// Generate deployment configuration for a hosting target.
    pub fn provision_hosting(workspace: &Path, host: &HostingTarget) -> Result<Vec<(String, String)>> {
        let mut files = Vec::new();

        match host {
            HostingTarget::Vercel => {
                files.push((
                    "vercel.json".to_string(),
                    "{\n  \
                       \"buildCommand\": \"npm run build\",\n  \
                       \"outputDirectory\": \"dist\",\n  \
                       \"framework\": null,\n  \
                       \"rewrites\": [\n    \
                         { \"source\": \"/api/(.*)\", \"destination\": \"/api/$1\" },\n    \
                         { \"source\": \"/(.*)\", \"destination\": \"/index.html\" }\n  \
                       ]\n\
                     }\n"
                    .to_string(),
                ));
            }
            HostingTarget::Netlify => {
                files.push((
                    "netlify.toml".to_string(),
                    "[build]\n  \
                       command = \"npm run build\"\n  \
                       publish = \"dist\"\n\n\
                     [[redirects]]\n  \
                       from = \"/*\"\n  \
                       to = \"/index.html\"\n  \
                       status = 200\n"
                    .to_string(),
                ));
            }
            HostingTarget::Railway => {
                files.push((
                    "railway.json".to_string(),
                    "{\n  \
                       \"$schema\": \"https://railway.app/railway.schema.json\",\n  \
                       \"build\": {\n    \
                         \"builder\": \"NIXPACKS\"\n  \
                       },\n  \
                       \"deploy\": {\n    \
                         \"startCommand\": \"npm start\",\n    \
                         \"healthcheckPath\": \"/health\",\n    \
                         \"restartPolicyType\": \"ON_FAILURE\"\n  \
                       }\n\
                     }\n"
                    .to_string(),
                ));
            }
            HostingTarget::BoltHost => {
                files.push((
                    "bolt.json".to_string(),
                    "{\n  \
                       \"name\": \"my-app\",\n  \
                       \"runtime\": \"node\",\n  \
                       \"build\": \"npm run build\",\n  \
                       \"start\": \"npm start\",\n  \
                       \"regions\": [\"us-east-1\"],\n  \
                       \"env\": {}\n\
                     }\n"
                    .to_string(),
                ));
            }
        }

        for (rel_path, content) in &files {
            let full_path = workspace.join(rel_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full_path, content)?;
        }

        Ok(files)
    }

    /// Generate a .env.example file from a provision config.
    pub fn generate_env_file(workspace: &Path, config: &ProvisionConfig) -> Result<String> {
        let mut lines = vec!["# Environment Variables".to_string(), "# Copy to .env and fill in values".to_string(), String::new()];

        if let Some(db) = &config.database {
            match db {
                DatabaseType::Sqlite => {
                    lines.push("DATABASE_URL=./data.db".to_string());
                }
                DatabaseType::Postgres => {
                    lines.push("DATABASE_URL=postgres://user:password@localhost:5432/mydb".to_string());
                }
                DatabaseType::Supabase => {
                    lines.push("SUPABASE_URL=https://your-project.supabase.co".to_string());
                    lines.push("SUPABASE_ANON_KEY=your-anon-key".to_string());
                    lines.push("SUPABASE_SERVICE_KEY=your-service-key".to_string());
                }
            }
        }

        if let Some(auth) = &config.auth {
            lines.push(String::new());
            match auth {
                AuthType::Jwt => {
                    lines.push("JWT_SECRET=change-me-in-production".to_string());
                }
                AuthType::OAuth => {
                    lines.push("GOOGLE_CLIENT_ID=your-client-id".to_string());
                    lines.push("GOOGLE_CLIENT_SECRET=your-client-secret".to_string());
                }
                AuthType::Supabase => {
                    // Already covered by database Supabase vars if present
                    if config.database != Some(DatabaseType::Supabase) {
                        lines.push("SUPABASE_URL=https://your-project.supabase.co".to_string());
                        lines.push("SUPABASE_ANON_KEY=your-anon-key".to_string());
                    }
                }
            }
        }

        if config.stripe {
            lines.push(String::new());
            lines.push("STRIPE_SECRET_KEY=sk_test_...".to_string());
            lines.push("STRIPE_PUBLISHABLE_KEY=pk_test_...".to_string());
            lines.push("STRIPE_WEBHOOK_SECRET=whsec_...".to_string());
        }

        lines.push(String::new());
        lines.push("NODE_ENV=development".to_string());
        lines.push("PORT=3000".to_string());

        let content = lines.join("\n") + "\n";
        let env_path = workspace.join(".env.example");
        std::fs::write(&env_path, &content)?;
        Ok(content)
    }
}

// ── EstimatedComplexity ───────────────────────────────────────────────────────

/// Estimated complexity for AI enhancer output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EstimatedComplexity {
    #[default]
    Simple,
    Medium,
    Complex,
}

impl fmt::Display for EstimatedComplexity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Simple => write!(f, "simple"),
            Self::Medium => write!(f, "medium"),
            Self::Complex => write!(f, "complex"),
        }
    }
}

// ── EnhancedSpec ──────────────────────────────────────────────────────────────

/// A structured specification generated from a rough idea by `AIEnhancer`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSpec {
    /// Project title.
    pub title: String,
    /// Expanded description.
    pub description: String,
    /// User stories derived from the idea.
    pub user_stories: Vec<String>,
    /// Recommended tech stack.
    pub tech_stack_recommendation: Vec<String>,
    /// Suggested database schema.
    #[serde(default)]
    pub database_schema_suggestion: Option<String>,
    /// Suggested API endpoints.
    pub api_endpoints: Vec<String>,
    /// Suggested UI components.
    pub ui_components: Vec<String>,
    /// Estimated complexity.
    pub estimated_complexity: EstimatedComplexity,
}

impl EnhancedSpec {
    /// Create a minimal spec with a title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: String::new(),
            user_stories: vec![],
            tech_stack_recommendation: vec![],
            database_schema_suggestion: None,
            api_endpoints: vec![],
            ui_components: vec![],
            estimated_complexity: EstimatedComplexity::Simple,
        }
    }
}

impl fmt::Display for EnhancedSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "# {}", self.title)?;
        if !self.description.is_empty() {
            writeln!(f, "\n{}", self.description)?;
        }
        if !self.user_stories.is_empty() {
            writeln!(f, "\n## User Stories")?;
            for story in &self.user_stories {
                writeln!(f, "- {}", story)?;
            }
        }
        if !self.tech_stack_recommendation.is_empty() {
            writeln!(f, "\n## Tech Stack")?;
            writeln!(f, "{}", self.tech_stack_recommendation.join(", "))?;
        }
        if !self.api_endpoints.is_empty() {
            writeln!(f, "\n## API Endpoints")?;
            for ep in &self.api_endpoints {
                writeln!(f, "- {}", ep)?;
            }
        }
        if !self.ui_components.is_empty() {
            writeln!(f, "\n## UI Components")?;
            for comp in &self.ui_components {
                writeln!(f, "- {}", comp)?;
            }
        }
        writeln!(f, "\nComplexity: {}", self.estimated_complexity)?;
        Ok(())
    }
}

// ── AIEnhancer ────────────────────────────────────────────────────────────────

/// Converts rough ideas into structured `EnhancedSpec`s using heuristic analysis.
///
/// No LLM call is needed — uses keyword matching and pattern detection.
pub struct AIEnhancer;

impl AIEnhancer {
    /// Enhance a raw idea string into a structured spec using heuristics.
    pub fn enhance_prompt(raw_idea: &str) -> EnhancedSpec {
        let idea_lower = raw_idea.to_lowercase();
        let words: Vec<&str> = raw_idea.split_whitespace().collect();

        // Determine title
        let title = Self::extract_title(raw_idea);

        // Determine description
        let description = format!(
            "A project built from the idea: \"{}\". This spec was auto-generated via heuristic analysis.",
            raw_idea.trim()
        );

        // Detect features and patterns
        let has_auth = Self::matches_any(&idea_lower, &["auth", "login", "sign up", "register", "user", "account", "password"]);
        let has_payments = Self::matches_any(&idea_lower, &["payment", "stripe", "checkout", "billing", "subscription", "pricing"]);
        let has_database = Self::matches_any(&idea_lower, &["database", "store", "crud", "data", "persist", "save", "table", "schema"]);
        let has_api = Self::matches_any(&idea_lower, &["api", "rest", "graphql", "endpoint", "backend", "server"]);
        let has_ui = Self::matches_any(&idea_lower, &["user interface", "frontend", "front-end", "dashboard", " page", "landing", " form ", "component", "react", "vue", "svelte", " ui "]);
        let has_mobile = Self::matches_any(&idea_lower, &["mobile", "ios", "android", "react native", "flutter", "app"]);
        let has_ecommerce = Self::matches_any(&idea_lower, &["ecommerce", "e-commerce", "shop", "cart", "product", "order", "inventory"]);
        let has_chat = Self::matches_any(&idea_lower, &["chat", "message", "real-time", "realtime", "websocket"]);
        let has_blog = Self::matches_any(&idea_lower, &["blog", "post", "article", "cms", "content"]);
        let has_analytics = Self::matches_any(&idea_lower, &["analytics", "dashboard", "metrics", "chart", "graph", "visualization"]);

        // Build user stories
        let mut user_stories = Vec::new();
        if has_auth {
            user_stories.push("As a user, I can create an account and log in securely".to_string());
            user_stories.push("As a user, I can reset my password via email".to_string());
        }
        if has_ecommerce {
            user_stories.push("As a customer, I can browse products and add them to my cart".to_string());
            user_stories.push("As a customer, I can complete a purchase with secure checkout".to_string());
            user_stories.push("As an admin, I can manage product inventory".to_string());
        }
        if has_blog {
            user_stories.push("As an author, I can create and publish blog posts".to_string());
            user_stories.push("As a reader, I can browse and search articles".to_string());
        }
        if has_chat {
            user_stories.push("As a user, I can send and receive messages in real-time".to_string());
        }
        if has_analytics {
            user_stories.push("As an admin, I can view key metrics on a dashboard".to_string());
            user_stories.push("As an analyst, I can filter and export data visualizations".to_string());
        }
        if has_api && user_stories.is_empty() {
            user_stories.push("As a developer, I can call API endpoints to manage resources".to_string());
        }
        if user_stories.is_empty() {
            user_stories.push(format!("As a user, I can interact with the {} system", title.to_lowercase()));
        }

        // Determine tech stack
        let mut tech_stack = Vec::new();
        if has_mobile {
            tech_stack.push("React Native".to_string());
            tech_stack.push("Expo".to_string());
        } else if has_ui {
            tech_stack.push("React".to_string());
            tech_stack.push("TypeScript".to_string());
            tech_stack.push("Tailwind CSS".to_string());
        }
        if has_api || has_database || has_auth {
            tech_stack.push("Node.js".to_string());
            tech_stack.push("Express".to_string());
        }
        if has_database {
            tech_stack.push("PostgreSQL".to_string());
            tech_stack.push("Prisma".to_string());
        }
        if has_payments {
            tech_stack.push("Stripe".to_string());
        }
        if has_chat {
            tech_stack.push("Socket.io".to_string());
        }
        if tech_stack.is_empty() {
            tech_stack.push("TypeScript".to_string());
            tech_stack.push("Node.js".to_string());
        }

        // Database schema suggestion
        let database_schema_suggestion = if has_database || has_auth || has_ecommerce {
            let mut schema = String::from("-- Auto-suggested schema\n");
            if has_auth {
                schema.push_str("CREATE TABLE users (id SERIAL PRIMARY KEY, email TEXT UNIQUE, name TEXT, created_at TIMESTAMPTZ DEFAULT NOW());\n");
            }
            if has_ecommerce {
                schema.push_str("CREATE TABLE products (id SERIAL PRIMARY KEY, name TEXT, price DECIMAL, description TEXT, stock INT);\n");
                schema.push_str("CREATE TABLE orders (id SERIAL PRIMARY KEY, user_id INT REFERENCES users(id), total DECIMAL, status TEXT, created_at TIMESTAMPTZ DEFAULT NOW());\n");
            }
            if has_blog {
                schema.push_str("CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT, body TEXT, author_id INT REFERENCES users(id), published_at TIMESTAMPTZ);\n");
            }
            if has_chat {
                schema.push_str("CREATE TABLE messages (id SERIAL PRIMARY KEY, sender_id INT REFERENCES users(id), content TEXT, created_at TIMESTAMPTZ DEFAULT NOW());\n");
            }
            Some(schema)
        } else {
            None
        };

        // API endpoints
        let mut api_endpoints = Vec::new();
        if has_auth {
            api_endpoints.push("POST /api/auth/register".to_string());
            api_endpoints.push("POST /api/auth/login".to_string());
            api_endpoints.push("POST /api/auth/logout".to_string());
            api_endpoints.push("GET /api/auth/me".to_string());
        }
        if has_ecommerce {
            api_endpoints.push("GET /api/products".to_string());
            api_endpoints.push("POST /api/products".to_string());
            api_endpoints.push("POST /api/orders".to_string());
            api_endpoints.push("GET /api/orders/:id".to_string());
        }
        if has_blog {
            api_endpoints.push("GET /api/posts".to_string());
            api_endpoints.push("POST /api/posts".to_string());
            api_endpoints.push("GET /api/posts/:id".to_string());
        }
        if has_chat {
            api_endpoints.push("GET /api/messages".to_string());
            api_endpoints.push("POST /api/messages".to_string());
        }
        if has_analytics {
            api_endpoints.push("GET /api/metrics".to_string());
            api_endpoints.push("GET /api/analytics/summary".to_string());
        }
        if api_endpoints.is_empty() && has_api {
            api_endpoints.push("GET /api/health".to_string());
            api_endpoints.push("GET /api/resources".to_string());
            api_endpoints.push("POST /api/resources".to_string());
        }

        // UI components
        let mut ui_components = Vec::new();
        if has_auth {
            ui_components.push("LoginForm".to_string());
            ui_components.push("RegisterForm".to_string());
            ui_components.push("UserProfile".to_string());
        }
        if has_ecommerce {
            ui_components.push("ProductGrid".to_string());
            ui_components.push("ShoppingCart".to_string());
            ui_components.push("CheckoutForm".to_string());
        }
        if has_analytics {
            ui_components.push("MetricsDashboard".to_string());
            ui_components.push("ChartWidget".to_string());
            ui_components.push("DataTable".to_string());
        }
        if has_blog {
            ui_components.push("PostList".to_string());
            ui_components.push("PostEditor".to_string());
        }
        if has_chat {
            ui_components.push("ChatWindow".to_string());
            ui_components.push("MessageList".to_string());
            ui_components.push("MessageInput".to_string());
        }
        if has_ui && ui_components.is_empty() {
            ui_components.push("Header".to_string());
            ui_components.push("MainContent".to_string());
            ui_components.push("Footer".to_string());
        }

        // Estimate complexity
        let feature_count = [has_auth, has_payments, has_database, has_api, has_ui, has_mobile, has_ecommerce, has_chat, has_blog, has_analytics]
            .iter()
            .filter(|&&x| x)
            .count();
        let estimated_complexity = if feature_count >= 3 || words.len() > 50 {
            EstimatedComplexity::Complex
        } else if feature_count >= 2 || words.len() > 20 {
            EstimatedComplexity::Medium
        } else {
            EstimatedComplexity::Simple
        };

        EnhancedSpec {
            title,
            description,
            user_stories,
            tech_stack_recommendation: tech_stack,
            database_schema_suggestion,
            api_endpoints,
            ui_components,
            estimated_complexity,
        }
    }

    /// Generate a project file structure from an enhanced spec.
    pub fn generate_project_structure(spec: &EnhancedSpec) -> Vec<(String, String)> {
        let mut files = Vec::new();

        // package.json
        let deps: Vec<String> = spec.tech_stack_recommendation.iter()
            .filter_map(|t| match t.to_lowercase().as_str() {
                "react" => Some("\"react\": \"^18.2.0\""),
                "express" => Some("\"express\": \"^4.18.0\""),
                "stripe" => Some("\"stripe\": \"^14.0.0\""),
                "socket.io" => Some("\"socket.io\": \"^4.7.0\""),
                "prisma" => Some("\"@prisma/client\": \"^5.0.0\""),
                _ => None,
            })
            .map(String::from)
            .collect();

        let slug = spec.title.to_lowercase().replace(' ', "-");
        files.push((
            "package.json".to_string(),
            format!(
                "{{\n  \"name\": \"{slug}\",\n  \"version\": \"0.1.0\",\n  \"private\": true,\n  \"scripts\": {{\n    \"dev\": \"tsx watch src/index.ts\",\n    \"build\": \"tsc\",\n    \"start\": \"node dist/index.js\"\n  }},\n  \"dependencies\": {{\n    {deps}\n  }}\n}}\n",
                slug = slug,
                deps = deps.join(",\n    ")
            ),
        ));

        // src/index.ts
        files.push((
            "src/index.ts".to_string(),
            format!(
                "// {} — entry point\nconsole.log('Starting {}...');\n",
                spec.title, spec.title
            ),
        ));

        // Generate component files
        for comp in &spec.ui_components {
            let filename = format!("src/components/{}.tsx", comp);
            files.push((
                filename,
                format!(
                    "import React from 'react';\n\nexport function {comp}() {{\n  return <div className=\"{slug}\">{comp}</div>;\n}}\n",
                    comp = comp,
                    slug = comp.to_lowercase()
                ),
            ));
        }

        // .gitignore
        files.push((
            ".gitignore".to_string(),
            "node_modules/\ndist/\n.env\n.env.local\n*.db\n.DS_Store\n".to_string(),
        ));

        // README
        files.push((
            "README.md".to_string(),
            format!(
                "# {}\n\n{}\n\n## Tech Stack\n\n{}\n\n## Getting Started\n\n```bash\nnpm install\nnpm run dev\n```\n",
                spec.title,
                spec.description,
                spec.tech_stack_recommendation.join(", ")
            ),
        ));

        files
    }

    // ── private helpers ──

    fn extract_title(raw: &str) -> String {
        let trimmed = raw.trim();
        // Take first sentence or first 60 chars, whichever is shorter
        let first_sentence = trimmed
            .split(['.', '!', '\n'])
            .next()
            .unwrap_or(trimmed)
            .trim();
        if first_sentence.len() > 60 {
            format!("{}...", &first_sentence[..57])
        } else if first_sentence.is_empty() {
            "Untitled Project".to_string()
        } else {
            first_sentence.to_string()
        }
    }

    fn matches_any(text: &str, keywords: &[&str]) -> bool {
        keywords.iter().any(|kw| text.contains(kw))
    }
}

// ── AppScaffolder ─────────────────────────────────────────────────────────────

/// Generates full project file trees from templates or enhanced specs.
pub struct AppScaffolder;

impl AppScaffolder {
    /// Scaffold a project from an `AppTemplate`, writing all files to workspace.
    pub fn scaffold_from_template(workspace: &Path, template: &AppTemplate) -> Result<Vec<String>> {
        std::fs::create_dir_all(workspace)?;
        let mut created = Vec::new();

        for (rel_path, content) in &template.files {
            let full_path = workspace.join(rel_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full_path, content)?;
            created.push(rel_path.clone());
        }

        // Write database schema if present
        if let Some(schema) = &template.database_schema {
            let schema_path = workspace.join("db/schema.sql");
            if let Some(parent) = schema_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&schema_path, schema)?;
            created.push("db/schema.sql".to_string());
        }

        // Write .env.example from template env_vars
        if !template.env_vars.is_empty() {
            let env_content: String = template
                .env_vars
                .iter()
                .map(|v| format!("{}=\n", v))
                .collect();
            std::fs::write(workspace.join(".env.example"), &env_content)?;
            created.push(".env.example".to_string());
        }

        created.sort();
        Ok(created)
    }

    /// Scaffold a project from an `EnhancedSpec`.
    pub fn scaffold_from_spec(workspace: &Path, spec: &EnhancedSpec) -> Result<Vec<String>> {
        std::fs::create_dir_all(workspace)?;
        let files = AIEnhancer::generate_project_structure(spec);
        let mut created = Vec::new();

        for (rel_path, content) in &files {
            let full_path = workspace.join(rel_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full_path, content)?;
            created.push(rel_path.clone());
        }

        // Generate docker-compose.yml
        let docker_content = Self::generate_docker_compose_for_spec(spec);
        std::fs::write(workspace.join("docker-compose.yml"), &docker_content)?;
        created.push("docker-compose.yml".to_string());

        // Generate CI config
        let ci_content = Self::generate_ci_config();
        let ci_dir = workspace.join(".github/workflows");
        std::fs::create_dir_all(&ci_dir)?;
        std::fs::write(ci_dir.join("ci.yml"), &ci_content)?;
        created.push(".github/workflows/ci.yml".to_string());

        created.sort();
        Ok(created)
    }

    fn generate_docker_compose_for_spec(spec: &EnhancedSpec) -> String {
        let mut services = String::from("version: '3.8'\n\nservices:\n");
        services.push_str("  app:\n    build: .\n    ports:\n      - \"3000:3000\"\n    env_file:\n      - .env\n");

        let has_pg = spec.tech_stack_recommendation.iter().any(|t| t.to_lowercase().contains("postgres"));
        if has_pg || spec.database_schema_suggestion.is_some() {
            services.push_str("    depends_on:\n      - db\n\n");
            services.push_str("  db:\n    image: postgres:16-alpine\n    environment:\n      POSTGRES_DB: app\n      POSTGRES_USER: app\n      POSTGRES_PASSWORD: secret\n    ports:\n      - \"5432:5432\"\n    volumes:\n      - pgdata:/var/lib/postgresql/data\n\n");
            services.push_str("volumes:\n  pgdata:\n");
        } else {
            services.push('\n');
        }

        services
    }

    fn generate_ci_config() -> String {
        "name: CI\n\non:\n  push:\n    branches: [main]\n  pull_request:\n    branches: [main]\n\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - uses: actions/setup-node@v4\n        with:\n          node-version: '20'\n      - run: npm ci\n      - run: npm run build\n      - run: npm test\n".to_string()
    }
}

// ── ManagedBackend ────────────────────────────────────────────────────────────

/// Unified backend configuration for managed deployments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    /// Hosting target.
    pub hosting: HostingTarget,
    /// Database type.
    pub database: Option<DatabaseType>,
    /// Auth provider.
    pub auth: Option<AuthType>,
    /// Enable Stripe payments.
    #[serde(default)]
    pub payments: bool,
    /// Enable SEO features.
    #[serde(default)]
    pub seo: bool,
    /// Enable analytics.
    #[serde(default)]
    pub analytics: bool,
}

impl BackendConfig {
    /// Create a new backend config with hosting target.
    pub fn new(hosting: HostingTarget) -> Self {
        Self {
            hosting,
            database: None,
            auth: None,
            payments: false,
            seo: false,
            analytics: false,
        }
    }

    /// Builder: set database.
    pub fn with_database(mut self, db: DatabaseType) -> Self {
        self.database = Some(db);
        self
    }

    /// Builder: set auth.
    pub fn with_auth(mut self, auth: AuthType) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Builder: enable payments.
    pub fn with_payments(mut self) -> Self {
        self.payments = true;
        self
    }

    /// Builder: enable SEO.
    pub fn with_seo(mut self) -> Self {
        self.seo = true;
        self
    }

    /// Builder: enable analytics.
    pub fn with_analytics(mut self) -> Self {
        self.analytics = true;
        self
    }
}

/// Generates unified backend configs, docker-compose files, and deployment manifests.
pub struct ManagedBackend;

impl ManagedBackend {
    /// Generate a unified JSON config file at `workspace/backend.config.json`.
    pub fn generate_backend_config(workspace: &Path, config: &BackendConfig) -> Result<String> {
        let json = serde_json::to_string_pretty(config)?;
        std::fs::create_dir_all(workspace)?;
        std::fs::write(workspace.join("backend.config.json"), &json)?;
        Ok(json)
    }

    /// Generate a docker-compose.yml with all services from the backend config.
    pub fn generate_docker_compose(config: &BackendConfig) -> String {
        let mut out = String::from("version: '3.8'\n\nservices:\n");

        // App service
        out.push_str("  app:\n    build: .\n    ports:\n      - \"3000:3000\"\n    env_file:\n      - .env\n");

        let mut deps = Vec::new();
        let mut volumes = Vec::new();

        // Database
        if let Some(db) = &config.database {
            match db {
                DatabaseType::Postgres => {
                    deps.push("db");
                    out.push_str("    depends_on:\n");
                    out.push_str("      - db\n\n");
                    out.push_str("  db:\n    image: postgres:16-alpine\n    environment:\n      POSTGRES_DB: app\n      POSTGRES_USER: app\n      POSTGRES_PASSWORD: secret\n    ports:\n      - \"5432:5432\"\n    volumes:\n      - pgdata:/var/lib/postgresql/data\n\n");
                    volumes.push("pgdata");
                }
                DatabaseType::Sqlite => {
                    // SQLite needs no extra service, but we add a volume
                    out.push_str("    volumes:\n      - sqlite-data:/app/data\n\n");
                    volumes.push("sqlite-data");
                }
                DatabaseType::Supabase => {
                    // Supabase is a managed service, no local container
                    out.push('\n');
                }
            }
        } else {
            out.push('\n');
        }

        // Analytics (Plausible)
        if config.analytics {
            out.push_str("  analytics:\n    image: plausible/analytics:latest\n    ports:\n      - \"8000:8000\"\n    environment:\n      BASE_URL: http://localhost:8000\n\n");
        }

        // Volumes
        if !volumes.is_empty() {
            out.push_str("volumes:\n");
            for vol in &volumes {
                out.push_str(&format!("  {}:\n", vol));
            }
        }

        out
    }

    /// Generate a deployment manifest (Kubernetes YAML).
    pub fn generate_deployment_manifest(config: &BackendConfig) -> String {
        let mut out = String::new();

        // Deployment
        out.push_str("apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: app\nspec:\n  replicas: 2\n  selector:\n    matchLabels:\n      app: main\n  template:\n    metadata:\n      labels:\n        app: main\n    spec:\n      containers:\n        - name: app\n          image: app:latest\n          ports:\n            - containerPort: 3000\n          envFrom:\n            - secretRef:\n                name: app-secrets\n");

        // Service
        out.push_str("---\napiVersion: v1\nkind: Service\nmetadata:\n  name: app-service\nspec:\n  selector:\n    app: main\n  ports:\n    - port: 80\n      targetPort: 3000\n  type: ClusterIP\n");

        // Database StatefulSet for Postgres
        if config.database == Some(DatabaseType::Postgres) {
            out.push_str("---\napiVersion: apps/v1\nkind: StatefulSet\nmetadata:\n  name: db\nspec:\n  serviceName: db\n  replicas: 1\n  selector:\n    matchLabels:\n      app: db\n  template:\n    metadata:\n      labels:\n        app: db\n    spec:\n      containers:\n        - name: postgres\n          image: postgres:16-alpine\n          ports:\n            - containerPort: 5432\n          volumeMounts:\n            - name: pgdata\n              mountPath: /var/lib/postgresql/data\n  volumeClaimTemplates:\n    - metadata:\n        name: pgdata\n      spec:\n        accessModes: [\"ReadWriteOnce\"]\n        resources:\n          requests:\n            storage: 10Gi\n");
        }

        // Ingress
        out.push_str("---\napiVersion: networking.k8s.io/v1\nkind: Ingress\nmetadata:\n  name: app-ingress\n  annotations:\n    nginx.ingress.kubernetes.io/rewrite-target: /\nspec:\n  rules:\n    - host: app.example.com\n      http:\n        paths:\n          - path: /\n            pathType: Prefix\n            backend:\n              service:\n                name: app-service\n                port:\n                  number: 80\n");

        out
    }
}

// ── Utility ───────────────────────────────────────────────────────────────────

/// Get the user's home directory, falling back to `/tmp` if unavailable.
fn dirs_home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── Helper ──

    fn temp_dir() -> TempDir {
        TempDir::new().expect("failed to create temp dir")
    }

    fn sample_template() -> AppTemplate {
        AppTemplate::new("test-tmpl", "Test Template", TemplateCategory::Web)
            .with_description("A test template")
            .with_file("index.html", "<html></html>")
            .with_file("style.css", "body {}")
            .with_tech_stack(vec!["HTML".into(), "CSS".into()])
            .with_env_vars(vec!["API_KEY".into(), "SECRET".into()])
    }

    // ── TemplateCategory Display ──

    #[test]
    fn template_category_display() {
        assert_eq!(format!("{}", TemplateCategory::Web), "web");
        assert_eq!(format!("{}", TemplateCategory::Mobile), "mobile");
        assert_eq!(format!("{}", TemplateCategory::Api), "api");
        assert_eq!(format!("{}", TemplateCategory::FullStack), "full_stack");
        assert_eq!(format!("{}", TemplateCategory::Landing), "landing");
        assert_eq!(format!("{}", TemplateCategory::Dashboard), "dashboard");
    }

    // ── AppTemplate construction ──

    #[test]
    fn template_new_defaults() {
        let t = AppTemplate::new("id1", "Name", TemplateCategory::Api);
        assert_eq!(t.id, "id1");
        assert_eq!(t.name, "Name");
        assert_eq!(t.category, TemplateCategory::Api);
        assert!(t.description.is_empty());
        assert!(t.files.is_empty());
        assert!(t.tech_stack.is_empty());
        assert!(t.env_vars.is_empty());
        assert!(t.database_schema.is_none());
    }

    #[test]
    fn template_builders() {
        let t = sample_template();
        assert_eq!(t.description, "A test template");
        assert_eq!(t.files.len(), 2);
        assert_eq!(t.tech_stack, vec!["HTML", "CSS"]);
        assert_eq!(t.env_vars, vec!["API_KEY", "SECRET"]);
    }

    #[test]
    fn template_with_database_schema() {
        let t = AppTemplate::new("db", "DB", TemplateCategory::Api)
            .with_database_schema("CREATE TABLE x (id INT);");
        assert_eq!(t.database_schema.unwrap(), "CREATE TABLE x (id INT);");
    }

    #[test]
    fn template_display() {
        let t = sample_template();
        let display = format!("{}", t);
        assert!(display.contains("test-tmpl"));
        assert!(display.contains("Test Template"));
        assert!(display.contains("2 files"));
        assert!(display.contains("HTML, CSS"));
    }

    #[test]
    fn template_display_empty_stack() {
        let t = AppTemplate::new("x", "X", TemplateCategory::Web);
        let display = format!("{}", t);
        assert!(display.contains("none"));
    }

    #[test]
    fn template_serialize_roundtrip() {
        let t = sample_template();
        let json = serde_json::to_string(&t).expect("serialize");
        let t2: AppTemplate = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(t2.id, t.id);
        assert_eq!(t2.files.len(), t.files.len());
    }

    // ── TeamTemplateStore CRUD ──

    #[test]
    fn store_save_and_load_template() {
        let dir = temp_dir();
        let store = TeamTemplateStore::with_dir(dir.path().to_path_buf());
        let tmpl = sample_template();

        store.save_template(Path::new("."), &tmpl).expect("save");
        let loaded = store.load_template("test-tmpl");
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.name, "Test Template");
        assert_eq!(loaded.files.len(), 2);
    }

    #[test]
    fn store_list_templates() {
        let dir = temp_dir();
        let store = TeamTemplateStore::with_dir(dir.path().to_path_buf());

        let t1 = AppTemplate::new("aaa", "First", TemplateCategory::Web);
        let t2 = AppTemplate::new("bbb", "Second", TemplateCategory::Api);
        store.save_template(Path::new("."), &t1).expect("save");
        store.save_template(Path::new("."), &t2).expect("save");

        let list = store.list_templates();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "aaa"); // sorted
        assert_eq!(list[1].id, "bbb");
    }

    #[test]
    fn store_list_empty_dir() {
        let dir = temp_dir();
        let store = TeamTemplateStore::with_dir(dir.path().join("nonexistent"));
        let list = store.list_templates();
        assert!(list.is_empty());
    }

    #[test]
    fn store_delete_template() {
        let dir = temp_dir();
        let store = TeamTemplateStore::with_dir(dir.path().to_path_buf());
        let tmpl = sample_template();
        store.save_template(Path::new("."), &tmpl).expect("save");

        assert!(store.delete_template("test-tmpl"));
        assert!(store.load_template("test-tmpl").is_none());
    }

    #[test]
    fn store_delete_nonexistent() {
        let dir = temp_dir();
        let store = TeamTemplateStore::with_dir(dir.path().to_path_buf());
        assert!(!store.delete_template("no-such-template"));
    }

    #[test]
    fn store_export_template() {
        let dir = temp_dir();
        let store = TeamTemplateStore::with_dir(dir.path().to_path_buf());
        let tmpl = sample_template();
        store.save_template(Path::new("."), &tmpl).expect("save");

        let json = store.export_template("test-tmpl");
        assert!(json.is_some());
        let json = json.unwrap();
        assert!(json.contains("test-tmpl"));
        assert!(json.contains("Test Template"));
    }

    #[test]
    fn store_export_nonexistent() {
        let dir = temp_dir();
        let store = TeamTemplateStore::with_dir(dir.path().to_path_buf());
        assert!(store.export_template("nope").is_none());
    }

    #[test]
    fn store_import_template() {
        let dir = temp_dir();
        let store = TeamTemplateStore::with_dir(dir.path().to_path_buf());
        let tmpl = sample_template();
        let json = serde_json::to_string_pretty(&tmpl).expect("serialize");

        let imported = store.import_template(&json).expect("import");
        assert_eq!(imported.id, "test-tmpl");
        // Verify it was saved
        let loaded = store.load_template("test-tmpl");
        assert!(loaded.is_some());
    }

    #[test]
    fn store_import_invalid_json() {
        let dir = temp_dir();
        let store = TeamTemplateStore::with_dir(dir.path().to_path_buf());
        let result = store.import_template("not valid json");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid template JSON"));
    }

    #[test]
    fn store_import_empty_id() {
        let dir = temp_dir();
        let store = TeamTemplateStore::with_dir(dir.path().to_path_buf());
        let tmpl = AppTemplate::new("", "No ID", TemplateCategory::Web);
        let json = serde_json::to_string(&tmpl).expect("serialize");
        let result = store.import_template(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    // ── AIEnhancer — various prompt types ──

    #[test]
    fn enhance_landing_page() {
        let spec = AIEnhancer::enhance_prompt("Build a landing page for my SaaS startup");
        assert!(!spec.title.is_empty());
        assert!(spec.ui_components.iter().any(|c| c == "Header" || c == "MainContent"));
        assert_eq!(spec.estimated_complexity, EstimatedComplexity::Simple);
    }

    #[test]
    fn enhance_api_project() {
        let spec = AIEnhancer::enhance_prompt("Create a REST API with user authentication and database");
        assert!(spec.tech_stack_recommendation.iter().any(|t| t.contains("Node")));
        assert!(!spec.api_endpoints.is_empty());
        assert!(spec.api_endpoints.iter().any(|e| e.contains("/auth/")));
        assert!(spec.database_schema_suggestion.is_some());
    }

    #[test]
    fn enhance_dashboard() {
        let spec = AIEnhancer::enhance_prompt("Analytics dashboard with charts and metrics visualization");
        assert!(spec.ui_components.iter().any(|c| c == "MetricsDashboard" || c == "ChartWidget"));
        assert!(spec.api_endpoints.iter().any(|e| e.contains("metrics")));
    }

    #[test]
    fn enhance_ecommerce() {
        let spec = AIEnhancer::enhance_prompt("E-commerce shop with product catalog, shopping cart, user authentication, and Stripe payments");
        assert_eq!(spec.estimated_complexity, EstimatedComplexity::Complex);
        assert!(spec.tech_stack_recommendation.iter().any(|t| t == "Stripe"));
        assert!(spec.ui_components.iter().any(|c| c == "ProductGrid"));
        assert!(spec.ui_components.iter().any(|c| c == "ShoppingCart"));
        assert!(spec.user_stories.iter().any(|s| s.contains("cart")));
        assert!(spec.database_schema_suggestion.as_ref().unwrap().contains("products"));
    }

    #[test]
    fn enhance_blog() {
        let spec = AIEnhancer::enhance_prompt("Blog platform with posts and user authentication");
        assert!(spec.user_stories.iter().any(|s| s.contains("blog") || s.contains("publish")));
        assert!(spec.api_endpoints.iter().any(|e| e.contains("posts")));
        assert!(spec.ui_components.iter().any(|c| c == "PostList" || c == "PostEditor"));
    }

    #[test]
    fn enhance_chat_app() {
        let spec = AIEnhancer::enhance_prompt("Real-time chat application with user accounts and message history");
        assert!(spec.tech_stack_recommendation.iter().any(|t| t == "Socket.io"));
        assert!(spec.ui_components.iter().any(|c| c == "ChatWindow"));
        assert!(spec.api_endpoints.iter().any(|e| e.contains("messages")));
    }

    #[test]
    fn enhance_mobile_app() {
        let spec = AIEnhancer::enhance_prompt("Mobile app for iOS and Android");
        assert!(spec.tech_stack_recommendation.iter().any(|t| t.contains("React Native") || t.contains("Expo")));
    }

    #[test]
    fn enhance_empty_input() {
        let spec = AIEnhancer::enhance_prompt("");
        assert_eq!(spec.title, "Untitled Project");
        assert!(!spec.user_stories.is_empty()); // fallback story
        assert_eq!(spec.estimated_complexity, EstimatedComplexity::Simple);
    }

    #[test]
    fn enhance_long_input_is_complex() {
        let long = "Build a comprehensive platform with authentication, payments, database storage, API endpoints, real-time chat, analytics dashboard, blog system, e-commerce shop, user profiles, admin panel, and mobile support with push notifications";
        let spec = AIEnhancer::enhance_prompt(long);
        assert_eq!(spec.estimated_complexity, EstimatedComplexity::Complex);
    }

    #[test]
    fn enhance_title_truncation() {
        let long_idea = "This is a very long first sentence that goes on and on and exceeds the sixty character limit for titles quite significantly";
        let spec = AIEnhancer::enhance_prompt(long_idea);
        assert!(spec.title.len() <= 63); // 57 + "..."
        assert!(spec.title.ends_with("..."));
    }

    #[test]
    fn enhance_spec_display() {
        let spec = AIEnhancer::enhance_prompt("Dashboard with auth and database");
        let display = format!("{}", spec);
        assert!(display.contains("# "));
        assert!(display.contains("Complexity:"));
    }

    #[test]
    fn generate_project_structure_has_package_json() {
        let spec = AIEnhancer::enhance_prompt("API with database");
        let files = AIEnhancer::generate_project_structure(&spec);
        assert!(files.iter().any(|(p, _)| p == "package.json"));
        assert!(files.iter().any(|(p, _)| p == ".gitignore"));
        assert!(files.iter().any(|(p, _)| p == "README.md"));
        assert!(files.iter().any(|(p, _)| p == "src/index.ts"));
    }

    #[test]
    fn generate_project_structure_includes_components() {
        let spec = AIEnhancer::enhance_prompt("Dashboard with analytics charts");
        let files = AIEnhancer::generate_project_structure(&spec);
        assert!(files.iter().any(|(p, _)| p.starts_with("src/components/")));
    }

    // ── Provisioning — databases ──

    #[test]
    fn provision_sqlite() {
        let dir = temp_dir();
        let files = AppProvisioner::provision_database(dir.path(), &DatabaseType::Sqlite).expect("provision");
        assert!(files.iter().any(|(p, _)| p == "db/schema.sql"));
        assert!(files.iter().any(|(p, _)| p == "src/db.ts"));
        let schema = fs::read_to_string(dir.path().join("db/schema.sql")).expect("read");
        assert!(schema.contains("SQLite"));
        assert!(schema.contains("AUTOINCREMENT"));
    }

    #[test]
    fn provision_postgres() {
        let dir = temp_dir();
        let files = AppProvisioner::provision_database(dir.path(), &DatabaseType::Postgres).expect("provision");
        assert_eq!(files.len(), 2);
        let schema = fs::read_to_string(dir.path().join("db/schema.sql")).expect("read");
        assert!(schema.contains("PostgreSQL"));
        assert!(schema.contains("uuid_generate_v4"));
        let db_ts = fs::read_to_string(dir.path().join("src/db.ts")).expect("read");
        assert!(db_ts.contains("Pool"));
    }

    #[test]
    fn provision_supabase_db() {
        let dir = temp_dir();
        let files = AppProvisioner::provision_database(dir.path(), &DatabaseType::Supabase).expect("provision");
        assert_eq!(files.len(), 2);
        let schema = fs::read_to_string(dir.path().join("db/schema.sql")).expect("read");
        assert!(schema.contains("ROW LEVEL SECURITY"));
        let db_ts = fs::read_to_string(dir.path().join("src/db.ts")).expect("read");
        assert!(db_ts.contains("supabase"));
    }

    // ── Provisioning — auth ──

    #[test]
    fn provision_jwt_auth() {
        let dir = temp_dir();
        let files = AppProvisioner::provision_auth(dir.path(), &AuthType::Jwt).expect("provision");
        assert_eq!(files.len(), 1);
        let auth = fs::read_to_string(dir.path().join("src/auth.ts")).expect("read");
        assert!(auth.contains("jwt"));
        assert!(auth.contains("signToken"));
        assert!(auth.contains("authMiddleware"));
    }

    #[test]
    fn provision_oauth_auth() {
        let dir = temp_dir();
        let files = AppProvisioner::provision_auth(dir.path(), &AuthType::OAuth).expect("provision");
        assert_eq!(files.len(), 1);
        let auth = fs::read_to_string(dir.path().join("src/auth.ts")).expect("read");
        assert!(auth.contains("passport"));
        assert!(auth.contains("GoogleStrategy"));
    }

    #[test]
    fn provision_supabase_auth() {
        let dir = temp_dir();
        let files = AppProvisioner::provision_auth(dir.path(), &AuthType::Supabase).expect("provision");
        assert_eq!(files.len(), 1);
        let auth = fs::read_to_string(dir.path().join("src/auth.ts")).expect("read");
        assert!(auth.contains("signUp"));
        assert!(auth.contains("signIn"));
    }

    // ── Provisioning — hosting ──

    #[test]
    fn provision_vercel() {
        let dir = temp_dir();
        let files = AppProvisioner::provision_hosting(dir.path(), &HostingTarget::Vercel).expect("provision");
        assert_eq!(files.len(), 1);
        let content = fs::read_to_string(dir.path().join("vercel.json")).expect("read");
        assert!(content.contains("buildCommand"));
        assert!(content.contains("rewrites"));
    }

    #[test]
    fn provision_netlify() {
        let dir = temp_dir();
        AppProvisioner::provision_hosting(dir.path(), &HostingTarget::Netlify).expect("provision");
        let content = fs::read_to_string(dir.path().join("netlify.toml")).expect("read");
        assert!(content.contains("[build]"));
        assert!(content.contains("redirects"));
    }

    #[test]
    fn provision_railway() {
        let dir = temp_dir();
        AppProvisioner::provision_hosting(dir.path(), &HostingTarget::Railway).expect("provision");
        let content = fs::read_to_string(dir.path().join("railway.json")).expect("read");
        assert!(content.contains("NIXPACKS"));
        assert!(content.contains("healthcheckPath"));
    }

    #[test]
    fn provision_bolthost() {
        let dir = temp_dir();
        AppProvisioner::provision_hosting(dir.path(), &HostingTarget::BoltHost).expect("provision");
        let content = fs::read_to_string(dir.path().join("bolt.json")).expect("read");
        assert!(content.contains("\"runtime\": \"node\""));
    }

    // ── Provisioning — env file ──

    #[test]
    fn generate_env_file_postgres_jwt() {
        let dir = temp_dir();
        let config = ProvisionConfig::new()
            .with_database(DatabaseType::Postgres)
            .with_auth(AuthType::Jwt);
        let content = AppProvisioner::generate_env_file(dir.path(), &config).expect("env");
        assert!(content.contains("DATABASE_URL=postgres://"));
        assert!(content.contains("JWT_SECRET"));
        assert!(content.contains("NODE_ENV"));
        assert!(dir.path().join(".env.example").is_file());
    }

    #[test]
    fn generate_env_file_supabase_stripe() {
        let dir = temp_dir();
        let config = ProvisionConfig::new()
            .with_database(DatabaseType::Supabase)
            .with_stripe();
        let content = AppProvisioner::generate_env_file(dir.path(), &config).expect("env");
        assert!(content.contains("SUPABASE_URL"));
        assert!(content.contains("STRIPE_SECRET_KEY"));
        assert!(content.contains("STRIPE_WEBHOOK_SECRET"));
    }

    #[test]
    fn generate_env_file_minimal() {
        let dir = temp_dir();
        let config = ProvisionConfig::new();
        let content = AppProvisioner::generate_env_file(dir.path(), &config).expect("env");
        assert!(content.contains("NODE_ENV"));
        assert!(content.contains("PORT=3000"));
    }

    #[test]
    fn generate_env_file_sqlite() {
        let dir = temp_dir();
        let config = ProvisionConfig::new().with_database(DatabaseType::Sqlite);
        let content = AppProvisioner::generate_env_file(dir.path(), &config).expect("env");
        assert!(content.contains("DATABASE_URL=./data.db"));
    }

    #[test]
    fn generate_env_file_oauth() {
        let dir = temp_dir();
        let config = ProvisionConfig::new().with_auth(AuthType::OAuth);
        let content = AppProvisioner::generate_env_file(dir.path(), &config).expect("env");
        assert!(content.contains("GOOGLE_CLIENT_ID"));
        assert!(content.contains("GOOGLE_CLIENT_SECRET"));
    }

    #[test]
    fn generate_env_file_supabase_auth_no_db() {
        let dir = temp_dir();
        let config = ProvisionConfig::new().with_auth(AuthType::Supabase);
        let content = AppProvisioner::generate_env_file(dir.path(), &config).expect("env");
        assert!(content.contains("SUPABASE_URL"));
    }

    // ── Scaffolding ──

    #[test]
    fn scaffold_from_template() {
        let dir = temp_dir();
        let tmpl = sample_template();
        let created = AppScaffolder::scaffold_from_template(dir.path(), &tmpl).expect("scaffold");
        assert!(created.contains(&".env.example".to_string()));
        assert!(created.contains(&"index.html".to_string()));
        assert!(created.contains(&"style.css".to_string()));
        let html = fs::read_to_string(dir.path().join("index.html")).expect("read");
        assert_eq!(html, "<html></html>");
    }

    #[test]
    fn scaffold_from_template_with_schema() {
        let dir = temp_dir();
        let tmpl = AppTemplate::new("db-app", "DB App", TemplateCategory::Api)
            .with_database_schema("CREATE TABLE test (id INT);");
        let created = AppScaffolder::scaffold_from_template(dir.path(), &tmpl).expect("scaffold");
        assert!(created.contains(&"db/schema.sql".to_string()));
        let schema = fs::read_to_string(dir.path().join("db/schema.sql")).expect("read");
        assert!(schema.contains("CREATE TABLE test"));
    }

    #[test]
    fn scaffold_from_template_empty_files() {
        let dir = temp_dir();
        let tmpl = AppTemplate::new("empty", "Empty", TemplateCategory::Web);
        let created = AppScaffolder::scaffold_from_template(dir.path(), &tmpl).expect("scaffold");
        assert!(created.is_empty());
    }

    #[test]
    fn scaffold_from_spec() {
        let dir = temp_dir();
        let spec = AIEnhancer::enhance_prompt("API with database and auth");
        let created = AppScaffolder::scaffold_from_spec(dir.path(), &spec).expect("scaffold");
        assert!(created.iter().any(|p| p == "package.json"));
        assert!(created.iter().any(|p| p == "docker-compose.yml"));
        assert!(created.iter().any(|p| p.contains("ci.yml")));
        assert!(dir.path().join("package.json").is_file());
        assert!(dir.path().join("docker-compose.yml").is_file());
    }

    #[test]
    fn scaffold_from_spec_creates_components() {
        let dir = temp_dir();
        let spec = AIEnhancer::enhance_prompt("Dashboard with analytics and charts");
        let created = AppScaffolder::scaffold_from_spec(dir.path(), &spec).expect("scaffold");
        assert!(created.iter().any(|p| p.starts_with("src/components/")));
    }

    // ── ManagedBackend ──

    #[test]
    fn backend_config_generation() {
        let dir = temp_dir();
        let config = BackendConfig::new(HostingTarget::Vercel)
            .with_database(DatabaseType::Postgres)
            .with_auth(AuthType::Jwt)
            .with_payments()
            .with_seo()
            .with_analytics();
        let json = ManagedBackend::generate_backend_config(dir.path(), &config).expect("gen");
        assert!(json.contains("vercel"));
        assert!(json.contains("postgres"));
        assert!(json.contains("jwt"));
        assert!(dir.path().join("backend.config.json").is_file());
    }

    #[test]
    fn backend_docker_compose_postgres() {
        let config = BackendConfig::new(HostingTarget::Railway)
            .with_database(DatabaseType::Postgres);
        let compose = ManagedBackend::generate_docker_compose(&config);
        assert!(compose.contains("postgres:16-alpine"));
        assert!(compose.contains("pgdata"));
        assert!(compose.contains("depends_on"));
    }

    #[test]
    fn backend_docker_compose_sqlite() {
        let config = BackendConfig::new(HostingTarget::Vercel)
            .with_database(DatabaseType::Sqlite);
        let compose = ManagedBackend::generate_docker_compose(&config);
        assert!(compose.contains("sqlite-data"));
    }

    #[test]
    fn backend_docker_compose_supabase() {
        let config = BackendConfig::new(HostingTarget::Netlify)
            .with_database(DatabaseType::Supabase);
        let compose = ManagedBackend::generate_docker_compose(&config);
        // Supabase is managed, no db container
        assert!(!compose.contains("postgres:16"));
    }

    #[test]
    fn backend_docker_compose_analytics() {
        let config = BackendConfig::new(HostingTarget::Vercel).with_analytics();
        let compose = ManagedBackend::generate_docker_compose(&config);
        assert!(compose.contains("plausible"));
    }

    #[test]
    fn backend_docker_compose_no_db() {
        let config = BackendConfig::new(HostingTarget::Vercel);
        let compose = ManagedBackend::generate_docker_compose(&config);
        assert!(compose.contains("app:"));
        assert!(!compose.contains("postgres"));
    }

    #[test]
    fn backend_deployment_manifest() {
        let config = BackendConfig::new(HostingTarget::Railway)
            .with_database(DatabaseType::Postgres);
        let manifest = ManagedBackend::generate_deployment_manifest(&config);
        assert!(manifest.contains("kind: Deployment"));
        assert!(manifest.contains("kind: Service"));
        assert!(manifest.contains("kind: StatefulSet"));
        assert!(manifest.contains("kind: Ingress"));
    }

    #[test]
    fn backend_deployment_manifest_no_db() {
        let config = BackendConfig::new(HostingTarget::Vercel);
        let manifest = ManagedBackend::generate_deployment_manifest(&config);
        assert!(manifest.contains("kind: Deployment"));
        assert!(!manifest.contains("StatefulSet"));
    }

    #[test]
    fn backend_config_serialize_roundtrip() {
        let config = BackendConfig::new(HostingTarget::Railway)
            .with_database(DatabaseType::Postgres)
            .with_auth(AuthType::OAuth)
            .with_payments()
            .with_analytics();
        let json = serde_json::to_string(&config).expect("serialize");
        let config2: BackendConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config2.hosting, HostingTarget::Railway);
        assert_eq!(config2.database, Some(DatabaseType::Postgres));
        assert_eq!(config2.auth, Some(AuthType::OAuth));
        assert!(config2.payments);
        assert!(config2.analytics);
    }

    // ── Type Display impls ──

    #[test]
    fn database_type_display() {
        assert_eq!(format!("{}", DatabaseType::Sqlite), "sqlite");
        assert_eq!(format!("{}", DatabaseType::Postgres), "postgres");
        assert_eq!(format!("{}", DatabaseType::Supabase), "supabase");
    }

    #[test]
    fn auth_type_display() {
        assert_eq!(format!("{}", AuthType::Jwt), "jwt");
        assert_eq!(format!("{}", AuthType::OAuth), "oauth");
        assert_eq!(format!("{}", AuthType::Supabase), "supabase");
    }

    #[test]
    fn hosting_target_display() {
        assert_eq!(format!("{}", HostingTarget::Vercel), "vercel");
        assert_eq!(format!("{}", HostingTarget::Netlify), "netlify");
        assert_eq!(format!("{}", HostingTarget::Railway), "railway");
        assert_eq!(format!("{}", HostingTarget::BoltHost), "bolt_host");
    }

    #[test]
    fn estimated_complexity_display() {
        assert_eq!(format!("{}", EstimatedComplexity::Simple), "simple");
        assert_eq!(format!("{}", EstimatedComplexity::Medium), "medium");
        assert_eq!(format!("{}", EstimatedComplexity::Complex), "complex");
    }

    // ── ProvisionConfig defaults ──

    #[test]
    fn provision_config_default() {
        let config = ProvisionConfig::default();
        assert!(config.database.is_none());
        assert!(config.auth.is_none());
        assert!(config.hosting.is_none());
        assert!(!config.seo);
        assert!(!config.stripe);
    }

    // ── EnhancedSpec construction ──

    #[test]
    fn enhanced_spec_new_defaults() {
        let spec = EnhancedSpec::new("My App");
        assert_eq!(spec.title, "My App");
        assert!(spec.description.is_empty());
        assert!(spec.user_stories.is_empty());
        assert!(spec.api_endpoints.is_empty());
        assert!(spec.ui_components.is_empty());
        assert_eq!(spec.estimated_complexity, EstimatedComplexity::Simple);
    }

    // ── Edge cases ──

    #[test]
    fn enhance_whitespace_only_input() {
        let spec = AIEnhancer::enhance_prompt("   \n  \t  ");
        assert_eq!(spec.title, "Untitled Project");
    }

    #[test]
    fn scaffold_nested_template_paths() {
        let dir = temp_dir();
        let tmpl = AppTemplate::new("nested", "Nested", TemplateCategory::FullStack)
            .with_file("src/components/deep/Widget.tsx", "export const W = () => null;");
        let created = AppScaffolder::scaffold_from_template(dir.path(), &tmpl).expect("scaffold");
        assert!(created.contains(&"src/components/deep/Widget.tsx".to_string()));
        assert!(dir.path().join("src/components/deep/Widget.tsx").is_file());
    }

    #[test]
    fn backend_config_new_defaults() {
        let config = BackendConfig::new(HostingTarget::Vercel);
        assert_eq!(config.hosting, HostingTarget::Vercel);
        assert!(config.database.is_none());
        assert!(config.auth.is_none());
        assert!(!config.payments);
        assert!(!config.seo);
        assert!(!config.analytics);
    }

    #[test]
    fn enhance_medium_complexity() {
        let spec = AIEnhancer::enhance_prompt("Build a blog with user authentication");
        assert_eq!(spec.estimated_complexity, EstimatedComplexity::Medium);
    }
}
