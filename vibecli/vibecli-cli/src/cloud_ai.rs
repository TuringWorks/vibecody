//! Configurable cloud AI backends — serving, training, eval and routing.
//!
//! One catalog (`cloud_ai_catalog.toml`) describes every backend as data: base
//! URL template, request shape, auth kind, and which stages it serves. This
//! module resolves that data against the encrypted [`ProfileStore`] and performs
//! the request. Adding a cloud is a config edit; repointing one at a private
//! gateway is a single `base_url` change.
//!
//! Everything here performs real network work. There is no simulated training,
//! no synthetic eval scoring, and no mocked response path — an operation that
//! cannot be executed returns [`CloudError`] rather than a plausible-looking
//! value.
//!
//! ```text
//! catalog (toml)  ──►  Backend ──► resolve vars + credential ──► HTTP
//!                                        │
//!                        ProfileStore ───┘   (keys never touch disk in plaintext)
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::profile_store::ProfileStore;

/// Default catalog, embedded so a fresh install works with no files on disk.
const EMBEDDED_CATALOG: &str = include_str!("cloud_ai_catalog.toml");

/// User override. When present it replaces the embedded catalog wholesale.
fn user_catalog_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".vibecli").join("cloud_ai.toml"))
}

// ── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum CloudError {
    /// The catalog itself is malformed.
    Catalog(String),
    /// No backend with that id.
    UnknownBackend(String),
    /// Backend exists but does not serve this stage.
    StageUnsupported { backend: String, stage: Stage },
    /// A `{var}` in the URL template has no configured value.
    MissingVar { backend: String, var: String },
    /// No credential stored for this backend.
    MissingCredential { backend: String, credential: String },
    /// Credential is stored but not in the shape this auth kind needs.
    MalformedCredential { backend: String, expected: String },
    /// The remote call failed at the transport layer.
    Transport(String),
    /// The remote returned a non-success status.
    Remote { status: u16, body: String },
    /// The response did not contain what the shape promised.
    Response(String),
    /// The operation is real but unimplemented for this backend shape — never
    /// silently substituted with a fake result.
    Unsupported(String),
}

impl fmt::Display for CloudError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(m) => write!(f, "cloud catalog is invalid: {m}"),
            Self::UnknownBackend(id) => write!(f, "no cloud backend named '{id}' (see `vibecli cloud list`)"),
            Self::StageUnsupported { backend, stage } => {
                write!(f, "backend '{backend}' does not support the {stage} stage")
            }
            Self::MissingVar { backend, var } => write!(
                f,
                "backend '{backend}' needs '{var}' — set it with `vibecli cloud set {backend} {var} <value>`"
            ),
            Self::MissingCredential { backend, credential } => write!(
                f,
                "no credential for '{backend}' — store one with `vibecli set-key {credential} <value>`"
            ),
            Self::MalformedCredential { backend, expected } => {
                write!(f, "credential for '{backend}' is malformed; expected {expected}")
            }
            Self::Transport(m) => write!(f, "request failed: {m}"),
            Self::Remote { status, body } => write!(f, "provider returned HTTP {status}: {body}"),
            Self::Response(m) => write!(f, "unexpected response shape: {m}"),
            Self::Unsupported(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for CloudError {}

type Result<T> = std::result::Result<T, CloudError>;

// ── Stages ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Serve,
    Train,
    Eval,
    Route,
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Serve => "serve",
            Self::Train => "train",
            Self::Eval => "eval",
            Self::Route => "route",
        };
        f.write_str(s)
    }
}

impl Stage {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "serve" | "serving" | "inference" => Some(Self::Serve),
            "train" | "training" | "tune" | "finetune" => Some(Self::Train),
            "eval" | "evaluate" | "evaluation" => Some(Self::Eval),
            "route" | "routing" => Some(Self::Route),
            _ => None,
        }
    }
}

// ── Catalog types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Catalog {
    pub schema_version: u32,
    #[serde(default, rename = "backend")]
    pub backends: Vec<Backend>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Backend {
    pub id: String,
    pub display_name: String,
    pub auth: AuthKind,
    /// ProfileStore api-key provider name holding this backend's credential.
    pub credential: String,
    #[serde(default)]
    pub stages: Vec<Stage>,
    #[serde(default)]
    pub required_vars: Vec<String>,
    #[serde(default)]
    pub auth_header: Option<String>,
    #[serde(default)]
    pub auth_service: Option<String>,
    #[serde(default)]
    pub verified: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub serve: Option<Endpoint>,
    #[serde(default)]
    pub train: Option<Endpoint>,
}

impl Backend {
    pub fn endpoint(&self, stage: Stage) -> Option<&Endpoint> {
        match stage {
            Stage::Serve => self.serve.as_ref(),
            Stage::Train => self.train.as_ref(),
            // Eval and Route are orchestrated locally on top of Serve.
            Stage::Eval | Stage::Route => self.serve.as_ref(),
        }
    }

    pub fn supports(&self, stage: Stage) -> bool {
        match stage {
            Stage::Eval | Stage::Route => self.serve.is_some(),
            other => self.stages.contains(&other) && self.endpoint(other).is_some(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Endpoint {
    pub api: ApiShape,
    pub base_url: String,
    pub path: String,
    #[serde(default)]
    pub query: Option<String>,
}

/// Request/response shape. Determines how a generic chat or training request is
/// rendered onto this provider's wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiShape {
    OpenAiChat,
    BedrockConverse,
    OciChat,
    WatsonxChat,
    AzureFineTune,
    VertexTuningJob,
    BedrockCustomizationJob,
    WatsonxTraining,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthKind {
    Bearer,
    ApiKeyHeader,
    IbmIam,
    GcpBearer,
    AwsSigv4,
    OciToken,
    None,
}

// ── Catalog loading ──────────────────────────────────────────────────────────

impl Catalog {
    /// Parse the embedded default catalog.
    pub fn embedded() -> Result<Self> {
        toml::from_str(EMBEDDED_CATALOG).map_err(|e| CloudError::Catalog(e.to_string()))
    }

    /// Load `~/.vibecli/cloud_ai.toml` when present, else the embedded default.
    pub fn load() -> Result<Self> {
        match user_catalog_path().filter(|p| p.exists()) {
            Some(path) => {
                let text = std::fs::read_to_string(&path)
                    .map_err(|e| CloudError::Catalog(format!("{}: {e}", path.display())))?;
                toml::from_str(&text)
                    .map_err(|e| CloudError::Catalog(format!("{}: {e}", path.display())))
            }
            None => Self::embedded(),
        }
    }

    pub fn get(&self, id: &str) -> Result<&Backend> {
        self.backends
            .iter()
            .find(|b| b.id == id)
            .ok_or_else(|| CloudError::UnknownBackend(id.to_string()))
    }

    /// Backends able to serve `stage`, in catalog order.
    pub fn for_stage(&self, stage: Stage) -> Vec<&Backend> {
        self.backends.iter().filter(|b| b.supports(stage)).collect()
    }
}

// ── Template resolution ──────────────────────────────────────────────────────

/// Substitute `{var}` placeholders. Returns the first missing variable rather
/// than emitting a URL with a literal brace in it.
fn render(template: &str, vars: &HashMap<String, String>, backend: &str) -> Result<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let close = after.find('}').ok_or_else(|| {
            CloudError::Catalog(format!("unterminated '{{' in template for '{backend}'"))
        })?;
        let name = &after[..close];
        let value = vars.get(name).ok_or_else(|| CloudError::MissingVar {
            backend: backend.to_string(),
            var: name.to_string(),
        })?;
        out.push_str(value);
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// A backend with its variables and credential resolved — ready to call.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub backend_id: String,
    pub stage: Stage,
    pub url: String,
    pub api: ApiShape,
    pub auth: AuthKind,
    pub auth_header: Option<String>,
    pub auth_service: Option<String>,
    pub credential: Option<String>,
    pub vars: HashMap<String, String>,
}

// ── Config source ────────────────────────────────────────────────────────────

/// Where per-backend variables and credentials come from. Backed by the
/// encrypted ProfileStore in production; the in-memory variant exists so the
/// resolution logic is testable without touching the real store.
pub enum ConfigSource {
    Profile { store: ProfileStore, profile_id: String },
    Memory { vars: HashMap<String, HashMap<String, String>>, creds: HashMap<String, String> },
}

impl ConfigSource {
    pub fn profile() -> Result<Self> {
        let store = ProfileStore::new().map_err(CloudError::Catalog)?;
        Ok(Self::Profile { store, profile_id: "default".to_string() })
    }

    pub fn memory() -> Self {
        Self::Memory { vars: HashMap::new(), creds: HashMap::new() }
    }

    pub fn with_var(mut self, backend: &str, key: &str, value: &str) -> Self {
        if let Self::Memory { vars, .. } = &mut self {
            vars.entry(backend.to_string())
                .or_default()
                .insert(key.to_string(), value.to_string());
        }
        self
    }

    pub fn with_credential(mut self, credential: &str, value: &str) -> Self {
        if let Self::Memory { creds, .. } = &mut self {
            creds.insert(credential.to_string(), value.to_string());
        }
        self
    }

    fn var(&self, backend: &str, key: &str) -> Option<String> {
        match self {
            Self::Profile { store, profile_id } => store
                .get_provider_config(profile_id, backend, key)
                .ok()
                .flatten(),
            Self::Memory { vars, .. } => vars.get(backend).and_then(|m| m.get(key)).cloned(),
        }
    }

    fn credential(&self, name: &str) -> Option<String> {
        match self {
            Self::Profile { store, profile_id } => {
                store.get_api_key(profile_id, name).ok().flatten()
            }
            Self::Memory { creds, .. } => creds.get(name).cloned(),
        }
    }

    pub fn set_var(&self, backend: &str, key: &str, value: &str) -> Result<()> {
        match self {
            Self::Profile { store, profile_id } => store
                .set_provider_config(profile_id, backend, key, value)
                .map_err(CloudError::Catalog),
            Self::Memory { .. } => Err(CloudError::Unsupported(
                "in-memory config source is read-only".into(),
            )),
        }
    }
}

// ── Client ───────────────────────────────────────────────────────────────────

pub struct CloudClient {
    catalog: Catalog,
    config: ConfigSource,
    http: reqwest::Client,
}

impl CloudClient {
    pub fn new(catalog: Catalog, config: ConfigSource) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self { catalog, config, http }
    }

    /// Production constructor: embedded/user catalog + encrypted store.
    pub fn open() -> Result<Self> {
        Ok(Self::new(Catalog::load()?, ConfigSource::profile()?))
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Resolve a backend for a stage: check support, fill template variables,
    /// and fetch the credential. Fails loudly on anything missing.
    pub fn resolve(&self, backend_id: &str, stage: Stage) -> Result<Resolved> {
        let backend = self.catalog.get(backend_id)?;
        if !backend.supports(stage) {
            return Err(CloudError::StageUnsupported {
                backend: backend_id.to_string(),
                stage,
            });
        }
        let endpoint = backend.endpoint(stage).ok_or(CloudError::StageUnsupported {
            backend: backend_id.to_string(),
            stage,
        })?;

        // Collect declared variables plus anything referenced in the templates.
        let mut vars: HashMap<String, String> = HashMap::new();
        for name in backend
            .required_vars
            .iter()
            .cloned()
            .chain(template_vars(&endpoint.base_url))
            .chain(template_vars(&endpoint.path))
            .chain(endpoint.query.iter().flat_map(|q| template_vars(q)))
        {
            if name == "model" {
                continue; // supplied per-request, not per-backend
            }
            if let Some(v) = self.config.var(&backend.id, &name) {
                vars.insert(name, v);
            }
        }

        let credential = self.config.credential(&backend.credential);
        if credential.is_none() && backend.auth != AuthKind::None {
            return Err(CloudError::MissingCredential {
                backend: backend.id.clone(),
                credential: backend.credential.clone(),
            });
        }

        let mut url = format!(
            "{}{}",
            render(&endpoint.base_url, &vars, &backend.id)?,
            render(&endpoint.path, &vars, &backend.id).unwrap_or_else(|_| endpoint.path.clone())
        );
        if let Some(q) = &endpoint.query {
            url.push('?');
            url.push_str(&render(q, &vars, &backend.id)?);
        }

        Ok(Resolved {
            backend_id: backend.id.clone(),
            stage,
            url,
            api: endpoint.api,
            auth: backend.auth,
            auth_header: backend.auth_header.clone(),
            auth_service: backend.auth_service.clone(),
            credential,
            vars,
        })
    }

    /// Report which backends are ready to use, and why the others are not.
    pub fn readiness(&self, stage: Stage) -> Vec<Readiness> {
        self.catalog
            .for_stage(stage)
            .into_iter()
            .map(|b| match self.resolve(&b.id, stage) {
                Ok(r) => Readiness {
                    backend_id: b.id.clone(),
                    display_name: b.display_name.clone(),
                    ready: true,
                    detail: r.url,
                },
                Err(e) => Readiness {
                    backend_id: b.id.clone(),
                    display_name: b.display_name.clone(),
                    ready: false,
                    detail: e.to_string(),
                },
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Readiness {
    pub backend_id: String,
    pub display_name: String,
    pub ready: bool,
    pub detail: String,
}

/// Extract `{var}` names from a template.
fn template_vars(template: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                out.push(after[..close].to_string());
                rest = &after[close + 1..];
            }
            None => break,
        }
    }
    out
}

// ── Auth ─────────────────────────────────────────────────────────────────────

/// An IAM token with its expiry, so a long agent session exchanges once.
#[derive(Debug, Clone)]
struct CachedToken {
    token: String,
    expires_at_unix: u64,
}

static IAM_CACHE: std::sync::OnceLock<std::sync::Mutex<HashMap<String, CachedToken>>> =
    std::sync::OnceLock::new();

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl CloudClient {
    /// Exchange an IBM Cloud API key for an IAM bearer token. Real HTTP, cached
    /// until 60s before expiry.
    async fn ibm_iam_token(&self, api_key: &str) -> Result<String> {
        let cache = IAM_CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
        if let Ok(guard) = cache.lock() {
            if let Some(hit) = guard.get(api_key) {
                if hit.expires_at_unix > now_unix() + 60 {
                    return Ok(hit.token.clone());
                }
            }
        }

        let resp = self
            .http
            .post("https://iam.cloud.ibm.com/identity/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .body(format!(
                "grant_type=urn:ibm:params:oauth:grant-type:apikey&apikey={}",
                urlencoding::encode(api_key)
            ))
            .send()
            .await
            .map_err(|e| CloudError::Transport(e.to_string()))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(CloudError::Remote { status: status.as_u16(), body });
        }
        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| CloudError::Response(format!("IAM token: {e}")))?;
        let token = json
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CloudError::Response("IAM response had no access_token".into()))?
            .to_string();
        let ttl = json.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(3600);

        if let Ok(mut guard) = cache.lock() {
            guard.insert(
                api_key.to_string(),
                CachedToken { token: token.clone(), expires_at_unix: now_unix() + ttl },
            );
        }
        Ok(token)
    }

    /// Resolve a GCP OAuth access token: the stored credential if it looks like
    /// a token, otherwise shell out to `gcloud auth print-access-token`.
    fn gcp_token(&self, credential: &str) -> Result<String> {
        let trimmed = credential.trim();
        if !trimmed.is_empty() && trimmed != "gcloud" {
            return Ok(trimmed.to_string());
        }
        let out = std::process::Command::new("gcloud")
            .args(["auth", "print-access-token"])
            .output()
            .map_err(|e| {
                CloudError::Transport(format!("gcloud auth print-access-token failed: {e}"))
            })?;
        if !out.status.success() {
            return Err(CloudError::Transport(format!(
                "gcloud auth print-access-token exited {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

/// AWS credential split from the stored `ACCESS_KEY:SECRET[:TOKEN]` form.
#[derive(Debug, Clone)]
pub struct AwsCredential {
    pub access_key: String,
    pub secret_key: String,
    pub session_token: Option<String>,
}

impl AwsCredential {
    pub fn parse(raw: &str, backend: &str) -> Result<Self> {
        let parts: Vec<&str> = raw.split(':').collect();
        match parts.as_slice() {
            [a, s] => Ok(Self {
                access_key: (*a).trim().to_string(),
                secret_key: (*s).trim().to_string(),
                session_token: None,
            }),
            [a, s, t] => Ok(Self {
                access_key: (*a).trim().to_string(),
                secret_key: (*s).trim().to_string(),
                session_token: Some((*t).trim().to_string()),
            }),
            _ => Err(CloudError::MalformedCredential {
                backend: backend.to_string(),
                expected: "ACCESS_KEY_ID:SECRET_ACCESS_KEY[:SESSION_TOKEN]".into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_parses() {
        let c = Catalog::embedded().expect("embedded catalog must parse");
        assert_eq!(c.schema_version, 1);
        assert!(c.backends.len() >= 7, "expected the seven named clouds + escape hatches");
    }

    #[test]
    fn every_named_cloud_is_present() {
        let c = Catalog::embedded().unwrap();
        for id in ["digitalocean", "azure", "google", "aws", "oracle", "ibm", "akamai"] {
            assert!(c.get(id).is_ok(), "catalog is missing '{id}'");
        }
    }

    #[test]
    fn every_backend_serves_at_least_one_stage() {
        let c = Catalog::embedded().unwrap();
        for b in &c.backends {
            assert!(
                b.supports(Stage::Serve) || b.supports(Stage::Train),
                "backend '{}' declares no usable stage",
                b.id
            );
        }
    }

    #[test]
    fn declared_stages_have_endpoints() {
        let c = Catalog::embedded().unwrap();
        for b in &c.backends {
            for stage in &b.stages {
                assert!(
                    b.endpoint(*stage).is_some(),
                    "backend '{}' declares stage {stage} with no endpoint block",
                    b.id
                );
            }
        }
    }

    #[test]
    fn template_vars_are_extracted() {
        let v = template_vars("https://{region}-x.example/{project}/a{model}");
        assert_eq!(v, vec!["region", "project", "model"]);
    }

    #[test]
    fn render_substitutes_and_reports_the_missing_one() {
        let mut vars = HashMap::new();
        vars.insert("region".to_string(), "us-east".to_string());
        assert_eq!(
            render("https://{region}.example", &vars, "x").unwrap(),
            "https://us-east.example"
        );
        let err = render("https://{region}/{project}", &vars, "x").unwrap_err();
        match err {
            CloudError::MissingVar { var, .. } => assert_eq!(var, "project"),
            other => panic!("expected MissingVar, got {other}"),
        }
    }

    #[test]
    fn resolve_builds_a_concrete_url() {
        let cfg = ConfigSource::memory()
            .with_var("google", "project", "proj-1")
            .with_var("google", "region", "us-central1")
            .with_credential("google_vertex", "ya29.token");
        let client = CloudClient::new(Catalog::embedded().unwrap(), cfg);
        let r = client.resolve("google", Stage::Serve).unwrap();
        assert_eq!(
            r.url,
            "https://us-central1-aiplatform.googleapis.com/v1/projects/proj-1/locations/us-central1/endpoints/openapi/chat/completions"
        );
        assert_eq!(r.auth, AuthKind::GcpBearer);
    }

    #[test]
    fn resolve_renders_query_strings() {
        let cfg = ConfigSource::memory()
            .with_var("ibm", "region", "us-south")
            .with_var("ibm", "project_id", "p1")
            .with_var("ibm", "api_version", "2024-10-08")
            .with_credential("ibm_watsonx", "apikey");
        let client = CloudClient::new(Catalog::embedded().unwrap(), cfg);
        let r = client.resolve("ibm", Stage::Serve).unwrap();
        assert_eq!(
            r.url,
            "https://us-south.ml.cloud.ibm.com/ml/v1/text/chat?version=2024-10-08"
        );
    }

    #[test]
    fn missing_credential_is_an_error_not_a_silent_anonymous_call() {
        let cfg = ConfigSource::memory().with_var("digitalocean", "x", "y");
        let client = CloudClient::new(Catalog::embedded().unwrap(), cfg);
        match client.resolve("digitalocean", Stage::Serve) {
            Err(CloudError::MissingCredential { backend, .. }) => assert_eq!(backend, "digitalocean"),
            other => panic!("expected MissingCredential, got {other:?}"),
        }
    }

    #[test]
    fn missing_var_names_the_variable_and_the_fix() {
        let cfg = ConfigSource::memory().with_credential("azure_openai", "k");
        let client = CloudClient::new(Catalog::embedded().unwrap(), cfg);
        let err = client.resolve("azure", Stage::Serve).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("vibecli cloud set azure"), "unhelpful error: {msg}");
    }

    #[test]
    fn unsupported_stage_is_rejected() {
        let cfg = ConfigSource::memory().with_credential("digitalocean", "k");
        let client = CloudClient::new(Catalog::embedded().unwrap(), cfg);
        match client.resolve("digitalocean", Stage::Train) {
            Err(CloudError::StageUnsupported { backend, stage }) => {
                assert_eq!(backend, "digitalocean");
                assert_eq!(stage, Stage::Train);
            }
            other => panic!("expected StageUnsupported, got {other:?}"),
        }
    }

    #[test]
    fn train_capable_backends_are_the_expected_four() {
        let c = Catalog::embedded().unwrap();
        let mut ids: Vec<&str> = c.for_stage(Stage::Train).iter().map(|b| b.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec!["aws", "azure", "google", "ibm"]);
    }

    #[test]
    fn aws_credential_parses_both_forms() {
        let two = AwsCredential::parse("AKIA:secret", "aws").unwrap();
        assert_eq!(two.access_key, "AKIA");
        assert!(two.session_token.is_none());
        let three = AwsCredential::parse("AKIA:secret:tok", "aws").unwrap();
        assert_eq!(three.session_token.as_deref(), Some("tok"));
        assert!(AwsCredential::parse("nope", "aws").is_err());
    }

    #[test]
    fn readiness_explains_why_a_backend_is_not_usable() {
        let cfg = ConfigSource::memory().with_credential("digitalocean", "k");
        let client = CloudClient::new(Catalog::embedded().unwrap(), cfg);
        let rows = client.readiness(Stage::Serve);
        let du = rows.iter().find(|r| r.backend_id == "digitalocean").unwrap();
        assert!(du.ready, "digitalocean should be ready with just a credential");
        let az = rows.iter().find(|r| r.backend_id == "azure").unwrap();
        assert!(!az.ready);
        assert!(az.detail.contains("resource") || az.detail.contains("credential"));
    }

    #[test]
    fn stage_parses_common_synonyms() {
        assert_eq!(Stage::parse("inference"), Some(Stage::Serve));
        assert_eq!(Stage::parse("finetune"), Some(Stage::Train));
        assert_eq!(Stage::parse("Evaluation"), Some(Stage::Eval));
        assert_eq!(Stage::parse("nonsense"), None);
    }
}
