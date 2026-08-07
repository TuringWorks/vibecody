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
            Self::UnknownBackend(id) => write!(f, "no cloud backend named '{id}' (see `vibecli --cloud-ai list`)"),
            Self::StageUnsupported { backend, stage } => {
                write!(f, "backend '{backend}' does not support the {stage} stage")
            }
            Self::MissingVar { backend, var } => write!(
                f,
                "backend '{backend}' needs '{var}' — set it with `vibecli --cloud-ai set {backend} {var} <value>`"
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
    /// Renamed explicitly: the derived name would be `open_ai_chat`, and the
    /// catalog should read the way the vendors spell it.
    #[serde(rename = "openai_chat")]
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
        // A blank value is missing, not present. Without this, clearing a var
        // (`--cloud-ai set custom base_url ""`) renders a hostless URL like
        // "/chat/completions" and the backend still reports ready — the failure
        // then surfaces as an opaque request error instead of "set this var".
        let value = vars
            .get(name)
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| CloudError::MissingVar {
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
    Profile {
        store: ProfileStore,
        profile_id: String,
    },
    Memory {
        vars: HashMap<String, HashMap<String, String>>,
        creds: HashMap<String, String>,
    },
}

impl ConfigSource {
    pub fn profile() -> Result<Self> {
        let store = ProfileStore::new().map_err(CloudError::Catalog)?;
        Ok(Self::Profile {
            store,
            profile_id: "default".to_string(),
        })
    }

    pub fn memory() -> Self {
        Self::Memory {
            vars: HashMap::new(),
            creds: HashMap::new(),
        }
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
        Self {
            catalog,
            config,
            http,
        }
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
        let endpoint = backend
            .endpoint(stage)
            .ok_or(CloudError::StageUnsupported {
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
            return Err(CloudError::Remote {
                status: status.as_u16(),
                body,
            });
        }
        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| CloudError::Response(format!("IAM token: {e}")))?;
        let token = json
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CloudError::Response("IAM response had no access_token".into()))?
            .to_string();
        let ttl = json
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600);

        if let Ok(mut guard) = cache.lock() {
            guard.insert(
                api_key.to_string(),
                CachedToken {
                    token: token.clone(),
                    expires_at_unix: now_unix() + ttl,
                },
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

// ── AWS SigV4 ────────────────────────────────────────────────────────────────

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    // Hmac accepts a key of any length; the error case is unreachable.
    let mut mac = match <Hmac<Sha256>>::new_from_slice(key) {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn derive_signing_key(secret: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k = hmac_sha256(format!("AWS4{secret}").as_bytes(), date.as_bytes());
    let k = hmac_sha256(&k, region.as_bytes());
    let k = hmac_sha256(&k, service.as_bytes());
    hmac_sha256(&k, b"aws4_request")
}

/// Build a SigV4 `Authorization` header for a JSON POST.
///
/// Split out and tested against the AWS documentation example so the signing
/// path is verifiable without a live account.
#[allow(clippy::too_many_arguments)]
fn sigv4_authorization(
    access_key: &str,
    secret_key: &str,
    region: &str,
    service: &str,
    host: &str,
    path: &str,
    payload: &[u8],
    datetime: &str,
) -> String {
    let date = &datetime[..8.min(datetime.len())];
    let payload_hash = sha256_hex(payload);
    let canonical_headers =
        format!("content-type:application/json\nhost:{host}\nx-amz-date:{datetime}\n");
    let signed_headers = "content-type;host;x-amz-date";
    let canonical_request =
        format!("POST\n{path}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}");
    let scope = format!("{date}/{region}/{service}/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{datetime}\n{scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );
    let signing_key = derive_signing_key(secret_key, date, region, service);
    let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes()));
    format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{scope}, SignedHeaders={signed_headers}, Signature={signature}"
    )
}

fn amz_datetime() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

// ── Chat ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatResponse {
    pub backend_id: String,
    pub model: String,
    pub text: String,
    pub latency_ms: u64,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

/// Render a generic chat request onto a provider's wire format.
fn chat_body(
    shape: ApiShape,
    req: &ChatRequest,
    vars: &HashMap<String, String>,
) -> serde_json::Value {
    let messages: Vec<serde_json::Value> = req
        .messages
        .iter()
        .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
        .collect();
    match shape {
        ApiShape::OpenAiChat => {
            let mut v = serde_json::json!({"model": req.model, "messages": messages});
            if let Some(max) = req.max_tokens {
                v["max_tokens"] = serde_json::json!(max);
            }
            v
        }
        ApiShape::BedrockConverse => {
            // Bedrock Converse nests content as typed blocks and carries the
            // model in the URL rather than the body.
            let msgs: Vec<serde_json::Value> = req
                .messages
                .iter()
                .filter(|m| m.role != "system")
                .map(|m| serde_json::json!({"role": m.role, "content": [{"text": m.content}]}))
                .collect();
            let mut v = serde_json::json!({"messages": msgs});
            if let Some(max) = req.max_tokens {
                v["inferenceConfig"] = serde_json::json!({"maxTokens": max});
            }
            let system: Vec<serde_json::Value> = req
                .messages
                .iter()
                .filter(|m| m.role == "system")
                .map(|m| serde_json::json!({"text": m.content}))
                .collect();
            if !system.is_empty() {
                v["system"] = serde_json::json!(system);
            }
            v
        }
        ApiShape::OciChat => serde_json::json!({
            "compartmentId": vars.get("compartment_id").cloned().unwrap_or_default(),
            "servingMode": {"servingType": "ON_DEMAND", "modelId": req.model},
            "chatRequest": {
                "apiFormat": "GENERIC",
                "messages": req.messages.iter().map(|m| serde_json::json!({
                    "role": m.role.to_uppercase(),
                    "content": [{"type": "TEXT", "text": m.content}]
                })).collect::<Vec<_>>(),
                "maxTokens": req.max_tokens.unwrap_or(1024),
            }
        }),
        ApiShape::WatsonxChat => {
            let mut v = serde_json::json!({
                "model_id": req.model,
                "messages": messages,
                "project_id": vars.get("project_id").cloned().unwrap_or_default(),
            });
            if let Some(max) = req.max_tokens {
                v["max_tokens"] = serde_json::json!(max);
            }
            v
        }
        // Training shapes never carry a chat body.
        _ => serde_json::json!({}),
    }
}

/// Pull assistant text out of a provider response.
fn chat_text(shape: ApiShape, body: &serde_json::Value) -> Option<String> {
    match shape {
        ApiShape::OpenAiChat => body
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        ApiShape::BedrockConverse => body
            .pointer("/output/message/content/0/text")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        ApiShape::OciChat => body
            .pointer("/chatResponse/choices/0/message/content/0/text")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        ApiShape::WatsonxChat => body
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        _ => None,
    }
}

fn usage_pair(shape: ApiShape, body: &serde_json::Value) -> (Option<u64>, Option<u64>) {
    let g = |p: &str| body.pointer(p).and_then(|v| v.as_u64());
    match shape {
        ApiShape::BedrockConverse => (g("/usage/inputTokens"), g("/usage/outputTokens")),
        _ => (
            g("/usage/prompt_tokens").or_else(|| g("/usage/input_tokens")),
            g("/usage/completion_tokens").or_else(|| g("/usage/output_tokens")),
        ),
    }
}

impl CloudClient {
    /// Attach auth to a request. Performs real credential work — IBM exchanges
    /// an IAM token over the network, AWS signs the payload, GCP may shell out
    /// to `gcloud`.
    async fn authorize(
        &self,
        builder: reqwest::RequestBuilder,
        resolved: &Resolved,
        url: &str,
        payload: &[u8],
    ) -> Result<reqwest::RequestBuilder> {
        let cred = resolved.credential.clone().unwrap_or_default();
        match resolved.auth {
            AuthKind::None => Ok(builder),
            AuthKind::Bearer | AuthKind::OciToken => {
                Ok(builder.header("Authorization", format!("Bearer {cred}")))
            }
            AuthKind::ApiKeyHeader => {
                let header = resolved.auth_header.clone().ok_or_else(|| {
                    CloudError::Catalog(format!(
                        "backend '{}' uses api_key_header but declares no auth_header",
                        resolved.backend_id
                    ))
                })?;
                Ok(builder.header(header, cred))
            }
            AuthKind::IbmIam => {
                let token = self.ibm_iam_token(&cred).await?;
                Ok(builder.header("Authorization", format!("Bearer {token}")))
            }
            AuthKind::GcpBearer => {
                let token = self.gcp_token(&cred)?;
                Ok(builder.header("Authorization", format!("Bearer {token}")))
            }
            AuthKind::AwsSigv4 => {
                let creds = AwsCredential::parse(&cred, &resolved.backend_id)?;
                let region =
                    resolved
                        .vars
                        .get("region")
                        .cloned()
                        .ok_or_else(|| CloudError::MissingVar {
                            backend: resolved.backend_id.clone(),
                            var: "region".into(),
                        })?;
                let service = resolved
                    .auth_service
                    .clone()
                    .unwrap_or_else(|| "bedrock".into());
                let parsed = reqwest::Url::parse(url)
                    .map_err(|e| CloudError::Transport(format!("bad URL {url}: {e}")))?;
                let host = parsed.host_str().unwrap_or_default().to_string();
                let datetime = amz_datetime();
                let auth = sigv4_authorization(
                    &creds.access_key,
                    &creds.secret_key,
                    &region,
                    &service,
                    &host,
                    parsed.path(),
                    payload,
                    &datetime,
                );
                let mut b = builder
                    .header("X-Amz-Date", datetime)
                    .header("Authorization", auth);
                if let Some(tok) = creds.session_token {
                    b = b.header("X-Amz-Security-Token", tok);
                }
                Ok(b)
            }
        }
    }

    /// Send a chat completion to a backend. Real request, real response.
    pub async fn chat(&self, backend_id: &str, req: &ChatRequest) -> Result<ChatResponse> {
        let mut resolved = self.resolve(backend_id, Stage::Serve)?;

        // Bedrock carries the model in the path.
        if resolved.api == ApiShape::BedrockConverse {
            resolved.url = resolved.url.replace("{model}", &req.model);
        }

        let body = chat_body(resolved.api, req, &resolved.vars);
        let payload = serde_json::to_vec(&body)
            .map_err(|e| CloudError::Response(format!("serializing request: {e}")))?;

        let builder = self
            .http
            .post(&resolved.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(payload.clone());
        let builder = self
            .authorize(builder, &resolved, &resolved.url, &payload)
            .await?;

        let started = SystemTime::now();
        let resp = builder
            .send()
            .await
            .map_err(|e| CloudError::Transport(e.to_string()))?;
        let latency_ms = started.elapsed().map(|d| d.as_millis() as u64).unwrap_or(0);

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(CloudError::Remote {
                status: status.as_u16(),
                body: text.chars().take(600).collect(),
            });
        }
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
            CloudError::Response(format!(
                "{e}: {}",
                text.chars().take(200).collect::<String>()
            ))
        })?;
        let out = chat_text(resolved.api, &json).ok_or_else(|| {
            CloudError::Response(format!(
                "no assistant text in {} response",
                resolved.backend_id
            ))
        })?;
        let (input_tokens, output_tokens) = usage_pair(resolved.api, &json);

        Ok(ChatResponse {
            backend_id: resolved.backend_id,
            model: req.model.clone(),
            text: out,
            latency_ms,
            input_tokens,
            output_tokens,
        })
    }
}

// ── Training ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSpec {
    /// Base model id, in the target cloud's own naming.
    pub base_model: String,
    /// Where the training data lives, in the form the cloud expects
    /// (an uploaded file id, a `gs://` / `s3://` URI, or an asset id).
    pub training_data: String,
    #[serde(default)]
    pub validation_data: Option<String>,
    #[serde(default)]
    pub suffix: Option<String>,
    #[serde(default)]
    pub hyperparameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrainingJob {
    pub backend_id: String,
    pub job_id: String,
    pub status: String,
    pub raw: serde_json::Value,
}

/// Render a training spec onto a cloud's job-submission body.
fn training_body(
    shape: ApiShape,
    spec: &TrainingSpec,
    vars: &HashMap<String, String>,
) -> Result<serde_json::Value> {
    match shape {
        ApiShape::AzureFineTune => {
            let mut v = serde_json::json!({
                "model": spec.base_model,
                "training_file": spec.training_data,
            });
            if let Some(val) = &spec.validation_data {
                v["validation_file"] = serde_json::json!(val);
            }
            if let Some(sfx) = &spec.suffix {
                v["suffix"] = serde_json::json!(sfx);
            }
            if !spec.hyperparameters.is_empty() {
                v["hyperparameters"] = serde_json::json!(spec.hyperparameters);
            }
            Ok(v)
        }
        ApiShape::VertexTuningJob => Ok(serde_json::json!({
            "baseModel": spec.base_model,
            "supervisedTuningSpec": {
                "trainingDatasetUri": spec.training_data,
                "validationDatasetUri": spec.validation_data,
            },
            "tunedModelDisplayName": spec.suffix.clone()
                .unwrap_or_else(|| "vibecody-tuned".to_string()),
        })),
        ApiShape::BedrockCustomizationJob => {
            let role = vars.get("role_arn").ok_or_else(|| CloudError::MissingVar {
                backend: "aws".into(),
                var: "role_arn".into(),
            })?;
            let output = vars
                .get("output_uri")
                .ok_or_else(|| CloudError::MissingVar {
                    backend: "aws".into(),
                    var: "output_uri".into(),
                })?;
            let name = spec
                .suffix
                .clone()
                .unwrap_or_else(|| format!("vibecody-{}", now_unix()));
            Ok(serde_json::json!({
                "jobName": name,
                "customModelName": name,
                "roleArn": role,
                "baseModelIdentifier": spec.base_model,
                "trainingDataConfig": {"s3Uri": spec.training_data},
                "outputDataConfig": {"s3Uri": output},
                "hyperParameters": spec.hyperparameters,
            }))
        }
        ApiShape::WatsonxTraining => {
            let project = vars
                .get("project_id")
                .ok_or_else(|| CloudError::MissingVar {
                    backend: "ibm".into(),
                    var: "project_id".into(),
                })?;
            Ok(serde_json::json!({
                "name": spec.suffix.clone().unwrap_or_else(|| "vibecody-tuning".into()),
                "project_id": project,
                "prompt_tuning": {
                    "base_model": {"model_id": spec.base_model},
                    "task_id": "generation",
                },
                "training_data_references": [{
                    "type": "container",
                    "location": {"path": spec.training_data},
                }],
            }))
        }
        other => Err(CloudError::Unsupported(format!(
            "api shape {other:?} is not a training shape"
        ))),
    }
}

fn training_ids(shape: ApiShape, body: &serde_json::Value) -> (Option<String>, Option<String>) {
    let s = |p: &str| body.pointer(p).and_then(|v| v.as_str()).map(str::to_string);
    match shape {
        ApiShape::AzureFineTune => (s("/id"), s("/status")),
        ApiShape::VertexTuningJob => (s("/name"), s("/state")),
        ApiShape::BedrockCustomizationJob => (s("/jobArn"), Some("InProgress".to_string())),
        ApiShape::WatsonxTraining => (s("/metadata/id"), s("/entity/status/state")),
        _ => (None, None),
    }
}

impl CloudClient {
    /// Submit a real training job. No local simulation: this either reaches the
    /// cloud's job API and returns its job id, or it errors.
    pub async fn submit_training(
        &self,
        backend_id: &str,
        spec: &TrainingSpec,
    ) -> Result<TrainingJob> {
        let resolved = self.resolve(backend_id, Stage::Train)?;
        let body = training_body(resolved.api, spec, &resolved.vars)?;
        let payload = serde_json::to_vec(&body)
            .map_err(|e| CloudError::Response(format!("serializing training request: {e}")))?;

        let builder = self
            .http
            .post(&resolved.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(payload.clone());
        let builder = self
            .authorize(builder, &resolved, &resolved.url, &payload)
            .await?;

        let resp = builder
            .send()
            .await
            .map_err(|e| CloudError::Transport(e.to_string()))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(CloudError::Remote {
                status: status.as_u16(),
                body: text.chars().take(600).collect(),
            });
        }
        let json: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::json!({}));
        let (job_id, job_status) = training_ids(resolved.api, &json);

        Ok(TrainingJob {
            backend_id: resolved.backend_id,
            job_id: job_id.ok_or_else(|| {
                CloudError::Response("training response carried no job id".into())
            })?,
            status: job_status.unwrap_or_else(|| "submitted".to_string()),
            raw: json,
        })
    }

    /// Poll a training job. Real GET against the same job collection.
    pub async fn training_status(&self, backend_id: &str, job_id: &str) -> Result<TrainingJob> {
        let resolved = self.resolve(backend_id, Stage::Train)?;
        let url = match resolved.url.split_once('?') {
            Some((base, query)) => format!("{base}/{job_id}?{query}"),
            None => format!("{}/{job_id}", resolved.url),
        };
        let builder = self.http.get(&url).header("Accept", "application/json");
        let builder = self.authorize(builder, &resolved, &url, b"").await?;
        let resp = builder
            .send()
            .await
            .map_err(|e| CloudError::Transport(e.to_string()))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(CloudError::Remote {
                status: status.as_u16(),
                body: text.chars().take(600).collect(),
            });
        }
        let json: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::json!({}));
        let (_, job_status) = training_ids(resolved.api, &json);
        Ok(TrainingJob {
            backend_id: resolved.backend_id,
            job_id: job_id.to_string(),
            status: job_status.unwrap_or_else(|| "unknown".to_string()),
            raw: json,
        })
    }
}

// ── Eval ─────────────────────────────────────────────────────────────────────

/// How a case's output is judged. Every variant is a deterministic function of
/// the real model output — there is no sampled or estimated score.
///
/// The TOML form names the check directly — `expect = { contains = "Paris" }`
/// — rather than requiring a `kind` discriminator. See [`ExpectSpec`] for the
/// wire shape and why it is a separate type.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(try_from = "ExpectSpec", into = "ExpectSpec")]
pub enum Expectation {
    /// Output must equal `value` after trimming.
    Exact { value: String },
    /// Output must contain `value`.
    Contains { value: String },
    /// Output must contain none of `values`.
    Excludes { values: Vec<String> },
    /// Output must parse as JSON and have a non-null value at `pointer`.
    JsonPointer { pointer: String },
    /// Output is written to `file`, then `command` runs; exit code 0 passes.
    /// This is how a code-generation case is scored: by executing it.
    Command { file: String, command: String },
}

/// The on-disk shape of an `expect` table: every check as an optional key.
///
/// An untagged enum would parse the same TOML, but a typo (`contian = "..."`)
/// fails every variant and serde reports only "data did not match any variant".
/// A flat struct with `deny_unknown_fields` names the bad key and lists the
/// valid ones, which is the difference between a fixable error and a puzzle.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excludes: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_pointer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

impl TryFrom<ExpectSpec> for Expectation {
    type Error = String;

    fn try_from(s: ExpectSpec) -> std::result::Result<Self, Self::Error> {
        // `file` is a modifier on `command`, not a check of its own, so it is
        // not counted here — otherwise the valid two-key command form would
        // read as two competing checks.
        let checks = [
            s.exact.is_some(),
            s.contains.is_some(),
            s.excludes.is_some(),
            s.json_pointer.is_some(),
            s.command.is_some(),
        ]
        .iter()
        .filter(|present| **present)
        .count();
        if checks > 1 {
            return Err("an expect table sets exactly one of `exact`, `contains`, `excludes`, `json_pointer`, or `command`".into());
        }
        match (s.exact, s.contains, s.excludes, s.json_pointer, s.command) {
            (Some(value), _, _, _, _) => Ok(Self::Exact { value }),
            (_, Some(value), _, _, _) => Ok(Self::Contains { value }),
            (_, _, Some(values), _, _) => Ok(Self::Excludes { values }),
            (_, _, _, Some(pointer), _) => Ok(Self::JsonPointer { pointer }),
            (_, _, _, _, Some(command)) => Ok(Self::Command {
                // Default the scratch filename so the common case is one key.
                file: s.file.unwrap_or_else(|| "output.txt".to_string()),
                command,
            }),
            _ => Err("an expect table needs one of `exact`, `contains`, `excludes`, `json_pointer`, or `command`".into()),
        }
    }
}

impl From<Expectation> for ExpectSpec {
    fn from(e: Expectation) -> Self {
        match e {
            Expectation::Exact { value } => Self {
                exact: Some(value),
                ..Self::default()
            },
            Expectation::Contains { value } => Self {
                contains: Some(value),
                ..Self::default()
            },
            Expectation::Excludes { values } => Self {
                excludes: Some(values),
                ..Self::default()
            },
            Expectation::JsonPointer { pointer } => Self {
                json_pointer: Some(pointer),
                ..Self::default()
            },
            Expectation::Command { file, command } => Self {
                file: Some(file),
                command: Some(command),
                ..Self::default()
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvalCase {
    /// `name` is accepted as a synonym — both read naturally in a suite file.
    #[serde(alias = "name")]
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub system: Option<String>,
    pub expect: Expectation,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvalSuite {
    pub name: String,
    pub model: String,
    #[serde(default, rename = "case")]
    pub cases: Vec<EvalCase>,
}

impl EvalSuite {
    pub fn from_toml(text: &str) -> Result<Self> {
        toml::from_str(text).map_err(|e| CloudError::Catalog(format!("eval suite: {e}")))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EvalCaseResult {
    pub id: String,
    pub passed: bool,
    pub detail: String,
    pub latency_ms: u64,
    pub output: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvalReport {
    pub suite: String,
    pub backend_id: String,
    pub model: String,
    pub passed: usize,
    pub failed: usize,
    pub errored: usize,
    pub cases: Vec<EvalCaseResult>,
}

impl EvalReport {
    pub fn pass_rate(&self) -> f64 {
        let total = self.passed + self.failed + self.errored;
        if total == 0 {
            return 0.0;
        }
        self.passed as f64 / total as f64
    }
}

/// Score one real model output against its expectation.
///
/// Pure and total — the scoring rule is testable without a network, while the
/// *output* it scores always comes from a real call.
pub fn score(
    expect: &Expectation,
    output: &str,
    workdir: Option<&std::path::Path>,
) -> (bool, String) {
    match expect {
        Expectation::Exact { value } => {
            let ok = output.trim() == value.trim();
            (
                ok,
                if ok {
                    "exact match".into()
                } else {
                    format!("expected exactly {value:?}")
                },
            )
        }
        Expectation::Contains { value } => {
            let ok = output.contains(value.as_str());
            (
                ok,
                if ok {
                    "substring found".into()
                } else {
                    format!("missing {value:?}")
                },
            )
        }
        Expectation::Excludes { values } => {
            match values.iter().find(|v| output.contains(v.as_str())) {
                Some(hit) => (false, format!("contained forbidden {hit:?}")),
                None => (true, "no forbidden substrings".into()),
            }
        }
        Expectation::JsonPointer { pointer } => {
            match serde_json::from_str::<serde_json::Value>(output) {
                Ok(v) => match v.pointer(pointer) {
                    Some(found) if !found.is_null() => (true, format!("{pointer} present")),
                    _ => (false, format!("no value at {pointer}")),
                },
                Err(e) => (false, format!("output is not JSON: {e}")),
            }
        }
        Expectation::Command { file, command } => {
            let dir = match workdir {
                Some(d) => d.to_path_buf(),
                None => return (false, "command cases need a working directory".into()),
            };
            let target = dir.join(file);
            if let Some(parent) = target.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return (false, format!("could not create {}: {e}", parent.display()));
                }
            }
            if let Err(e) = std::fs::write(&target, output) {
                return (false, format!("could not write {}: {e}", target.display()));
            }
            match std::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(&dir)
                .output()
            {
                Ok(out) if out.status.success() => (true, "command exited 0".into()),
                Ok(out) => (
                    false,
                    format!(
                        "command exited {}: {}",
                        out.status.code().unwrap_or(-1),
                        String::from_utf8_lossy(&out.stderr)
                            .chars()
                            .take(300)
                            .collect::<String>()
                    ),
                ),
                Err(e) => (false, format!("command failed to start: {e}")),
            }
        }
    }
}

impl CloudClient {
    /// Run an eval suite against a serving backend.
    ///
    /// Each case is a real completion scored by a deterministic rule. A case
    /// whose request fails is counted as `errored` and reported — never scored
    /// as a pass, and never replaced with a synthetic result.
    pub async fn run_eval(
        &self,
        backend_id: &str,
        suite: &EvalSuite,
        workdir: Option<&std::path::Path>,
    ) -> Result<EvalReport> {
        // Fail fast on configuration before spending any tokens.
        let _ = self.resolve(backend_id, Stage::Serve)?;

        let mut cases = Vec::with_capacity(suite.cases.len());
        let (mut passed, mut failed, mut errored) = (0usize, 0usize, 0usize);

        for case in &suite.cases {
            let mut messages = Vec::new();
            if let Some(sys) = &case.system {
                messages.push(ChatMessage {
                    role: "system".into(),
                    content: sys.clone(),
                });
            }
            messages.push(ChatMessage {
                role: "user".into(),
                content: case.prompt.clone(),
            });

            let req = ChatRequest {
                model: suite.model.clone(),
                messages,
                max_tokens: Some(2048),
            };
            match self.chat(backend_id, &req).await {
                Ok(resp) => {
                    let (ok, detail) = score(&case.expect, &resp.text, workdir);
                    if ok {
                        passed += 1;
                    } else {
                        failed += 1;
                    }
                    cases.push(EvalCaseResult {
                        id: case.id.clone(),
                        passed: ok,
                        detail,
                        latency_ms: resp.latency_ms,
                        output: resp.text,
                    });
                }
                Err(e) => {
                    errored += 1;
                    cases.push(EvalCaseResult {
                        id: case.id.clone(),
                        passed: false,
                        detail: format!("request failed: {e}"),
                        latency_ms: 0,
                        output: String::new(),
                    });
                }
            }
        }

        Ok(EvalReport {
            suite: suite.name.clone(),
            backend_id: backend_id.to_string(),
            model: suite.model.clone(),
            passed,
            failed,
            errored,
            cases,
        })
    }
}

// ── Routing ──────────────────────────────────────────────────────────────────

/// How the router picks among configured backends.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "policy", rename_all = "snake_case")]
pub enum RoutePolicy {
    /// Try backends in the given order, first ready one wins.
    Ordered { order: Vec<String> },
    /// Cheapest configured `price_per_mtok` first.
    Cheapest,
    /// Catalog order.
    FirstReady,
}

impl Default for RoutePolicy {
    fn default() -> Self {
        Self::FirstReady
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteDecision {
    pub backend_id: String,
    pub reason: String,
    /// Backends skipped, and why — so a routing choice is always explainable.
    pub skipped: Vec<(String, String)>,
}

impl CloudClient {
    /// Choose a backend for a stage. Pure configuration logic over the
    /// resolvable set; no network call.
    pub fn route(&self, stage: Stage, policy: &RoutePolicy) -> Result<RouteDecision> {
        let mut skipped = Vec::new();

        let candidates: Vec<String> = match policy {
            RoutePolicy::Ordered { order } => order.clone(),
            _ => self
                .catalog
                .for_stage(stage)
                .iter()
                .map(|b| b.id.clone())
                .collect(),
        };

        let mut ready: Vec<(String, Option<f64>)> = Vec::new();
        for id in &candidates {
            match self.resolve(id, stage) {
                Ok(_) => {
                    let price = self
                        .config
                        .var(id, "price_per_mtok")
                        .and_then(|v| v.parse::<f64>().ok());
                    ready.push((id.clone(), price));
                }
                Err(e) => skipped.push((id.clone(), e.to_string())),
            }
        }

        let chosen = match policy {
            RoutePolicy::Cheapest => ready
                .iter()
                .filter(|(_, p)| p.is_some())
                .min_by(|a, b| {
                    a.1.unwrap_or(f64::MAX)
                        .partial_cmp(&b.1.unwrap_or(f64::MAX))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .or_else(|| ready.first())
                .cloned(),
            _ => ready.first().cloned(),
        };

        match chosen {
            Some((id, price)) => {
                let reason = match (policy, price) {
                    (RoutePolicy::Cheapest, Some(p)) => format!("cheapest configured at ${p}/Mtok"),
                    // Say that the policy could not be applied. Reporting a
                    // bare "first ready backend" for a `cheapest` request reads
                    // as though price was compared, when in fact none was set.
                    (RoutePolicy::Cheapest, None) => {
                        "first ready backend — no price configured on any ready backend, so \
                         there was nothing to compare (set one with \
                         `--cloud-ai set <backend> price_per_mtok <value>`)"
                            .into()
                    }
                    (RoutePolicy::Ordered { .. }, _) => "first ready in configured order".into(),
                    _ => "first ready backend".into(),
                };
                Ok(RouteDecision { backend_id: id, reason, skipped })
            }
            None => Err(CloudError::Unsupported(format!(
                "no backend is configured for the {stage} stage ({} candidate(s) skipped; run `vibecli --cloud-ai status` to see why)",
                skipped.len()
            ))),
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
        assert!(
            c.backends.len() >= 7,
            "expected the seven named clouds + escape hatches"
        );
    }

    #[test]
    fn every_named_cloud_is_present() {
        let c = Catalog::embedded().unwrap();
        for id in [
            "digitalocean",
            "azure",
            "google",
            "aws",
            "oracle",
            "ibm",
            "akamai",
        ] {
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
    fn a_blank_var_counts_as_missing_not_as_an_empty_substitution() {
        // Clearing a var must put the backend back into "needs configuration",
        // not leave it ready with a hostless URL.
        let mut vars = HashMap::new();
        vars.insert("base_url".to_string(), "   ".to_string());
        let err = render("{base_url}/chat/completions", &vars, "custom").unwrap_err();
        match err {
            CloudError::MissingVar { var, .. } => assert_eq!(var, "base_url"),
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
            Err(CloudError::MissingCredential { backend, .. }) => {
                assert_eq!(backend, "digitalocean")
            }
            other => panic!("expected MissingCredential, got {other:?}"),
        }
    }

    #[test]
    fn missing_var_names_the_variable_and_the_fix() {
        let cfg = ConfigSource::memory().with_credential("azure_openai", "k");
        let client = CloudClient::new(Catalog::embedded().unwrap(), cfg);
        let err = client.resolve("azure", Stage::Serve).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("vibecli --cloud-ai set azure"),
            "unhelpful error: {msg}"
        );
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
        let mut ids: Vec<&str> = c
            .for_stage(Stage::Train)
            .iter()
            .map(|b| b.id.as_str())
            .collect();
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
        let du = rows
            .iter()
            .find(|r| r.backend_id == "digitalocean")
            .unwrap();
        assert!(
            du.ready,
            "digitalocean should be ready with just a credential"
        );
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

#[cfg(test)]
mod exec_tests {
    use super::*;

    // ── SigV4 ────────────────────────────────────────────────────────────────
    // Signing is the one auth path we can verify offline: AWS publishes a
    // worked example, so the derived signing key is checked against it rather
    // than against our own output.

    #[test]
    fn sigv4_signing_key_matches_the_aws_worked_example() {
        // From AWS's "Examples of the signature calculation" documentation.
        let key = derive_signing_key(
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "20150830",
            "us-east-1",
            "iam",
        );
        assert_eq!(
            hex::encode(key),
            "c4afb1cc5771d871763a393e44b703571b55cc28424d1a5e86da6ed3c154a4b9"
        );
    }

    #[test]
    fn sigv4_authorization_has_the_required_parts() {
        let auth = sigv4_authorization(
            "AKIDEXAMPLE",
            "secret",
            "us-east-1",
            "bedrock",
            "bedrock-runtime.us-east-1.amazonaws.com",
            "/model/m/converse",
            b"{}",
            "20260806T000000Z",
        );
        assert!(auth.starts_with(
            "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20260806/us-east-1/bedrock/aws4_request"
        ));
        assert!(auth.contains("SignedHeaders=content-type;host;x-amz-date"));
        assert!(auth.contains("Signature="));
    }

    #[test]
    fn sigv4_signature_changes_with_the_payload() {
        let one = sigv4_authorization(
            "A",
            "s",
            "r",
            "bedrock",
            "h",
            "/p",
            b"{\"a\":1}",
            "20260806T000000Z",
        );
        let two = sigv4_authorization(
            "A",
            "s",
            "r",
            "bedrock",
            "h",
            "/p",
            b"{\"a\":2}",
            "20260806T000000Z",
        );
        assert_ne!(one, two, "signature must cover the body");
    }

    // ── Request shaping ──────────────────────────────────────────────────────

    #[test]
    fn openai_shape_is_flat() {
        let req = ChatRequest {
            model: "gpt-oss-120b".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            max_tokens: Some(64),
        };
        let body = chat_body(ApiShape::OpenAiChat, &req, &HashMap::new());
        assert_eq!(body["model"], "gpt-oss-120b");
        assert_eq!(body["messages"][0]["content"], "hi");
        assert_eq!(body["max_tokens"], 64);
    }

    #[test]
    fn bedrock_shape_nests_content_and_lifts_system() {
        let req = ChatRequest {
            model: "anthropic.claude-opus-5".into(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: "be terse".into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: "hi".into(),
                },
            ],
            max_tokens: Some(32),
        };
        let body = chat_body(ApiShape::BedrockConverse, &req, &HashMap::new());
        // Model rides in the URL, not the body.
        assert!(body.get("model").is_none());
        assert_eq!(body["messages"][0]["content"][0]["text"], "hi");
        assert_eq!(body["system"][0]["text"], "be terse");
        assert_eq!(body["inferenceConfig"]["maxTokens"], 32);
    }

    #[test]
    fn oci_and_watsonx_shapes_carry_their_scoping_ids() {
        let req = ChatRequest {
            model: "m".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            max_tokens: None,
        };
        let mut vars = HashMap::new();
        vars.insert(
            "compartment_id".to_string(),
            "ocid1.compartment".to_string(),
        );
        vars.insert("project_id".to_string(), "proj".to_string());

        let oci = chat_body(ApiShape::OciChat, &req, &vars);
        assert_eq!(oci["compartmentId"], "ocid1.compartment");
        assert_eq!(oci["chatRequest"]["messages"][0]["role"], "USER");

        let wx = chat_body(ApiShape::WatsonxChat, &req, &vars);
        assert_eq!(wx["project_id"], "proj");
        assert_eq!(wx["model_id"], "m");
    }

    // ── Response parsing ─────────────────────────────────────────────────────

    #[test]
    fn each_shape_extracts_its_own_response_text() {
        let openai = serde_json::json!({"choices":[{"message":{"content":"A"}}]});
        assert_eq!(
            chat_text(ApiShape::OpenAiChat, &openai).as_deref(),
            Some("A")
        );

        let bedrock = serde_json::json!({"output":{"message":{"content":[{"text":"B"}]}}});
        assert_eq!(
            chat_text(ApiShape::BedrockConverse, &bedrock).as_deref(),
            Some("B")
        );

        let oci = serde_json::json!({"chatResponse":{"choices":[{"message":{"content":[{"text":"C"}]}}]}});
        assert_eq!(chat_text(ApiShape::OciChat, &oci).as_deref(), Some("C"));
    }

    #[test]
    fn missing_text_is_none_not_empty_string() {
        // An empty string would read as a successful empty completion.
        assert_eq!(
            chat_text(ApiShape::OpenAiChat, &serde_json::json!({})),
            None
        );
    }

    #[test]
    fn usage_is_read_per_shape() {
        let bedrock = serde_json::json!({"usage":{"inputTokens":10,"outputTokens":3}});
        assert_eq!(
            usage_pair(ApiShape::BedrockConverse, &bedrock),
            (Some(10), Some(3))
        );
        let openai = serde_json::json!({"usage":{"prompt_tokens":7,"completion_tokens":2}});
        assert_eq!(
            usage_pair(ApiShape::OpenAiChat, &openai),
            (Some(7), Some(2))
        );
    }

    // ── Training ─────────────────────────────────────────────────────────────

    #[test]
    fn training_bodies_match_each_cloud() {
        let spec = TrainingSpec {
            base_model: "base".into(),
            training_data: "file-1".into(),
            validation_data: None,
            suffix: Some("run7".into()),
            hyperparameters: HashMap::new(),
        };
        let az = training_body(ApiShape::AzureFineTune, &spec, &HashMap::new()).unwrap();
        assert_eq!(az["training_file"], "file-1");
        assert_eq!(az["suffix"], "run7");

        let gc = training_body(ApiShape::VertexTuningJob, &spec, &HashMap::new()).unwrap();
        assert_eq!(gc["supervisedTuningSpec"]["trainingDatasetUri"], "file-1");

        let mut vars = HashMap::new();
        vars.insert("project_id".to_string(), "p".to_string());
        let ibm = training_body(ApiShape::WatsonxTraining, &spec, &vars).unwrap();
        assert_eq!(ibm["project_id"], "p");
    }

    #[test]
    fn bedrock_training_requires_role_and_output_and_says_so() {
        let spec = TrainingSpec {
            base_model: "b".into(),
            training_data: "s3://in".into(),
            validation_data: None,
            suffix: None,
            hyperparameters: HashMap::new(),
        };
        match training_body(ApiShape::BedrockCustomizationJob, &spec, &HashMap::new()) {
            Err(CloudError::MissingVar { var, .. }) => assert_eq!(var, "role_arn"),
            other => panic!("expected MissingVar(role_arn), got {other:?}"),
        }
    }

    #[test]
    fn a_chat_shape_is_rejected_as_a_training_shape() {
        let spec = TrainingSpec {
            base_model: "b".into(),
            training_data: "d".into(),
            validation_data: None,
            suffix: None,
            hyperparameters: HashMap::new(),
        };
        assert!(training_body(ApiShape::OpenAiChat, &spec, &HashMap::new()).is_err());
    }

    #[test]
    fn job_ids_are_read_per_cloud() {
        assert_eq!(
            training_ids(
                ApiShape::AzureFineTune,
                &serde_json::json!({"id":"ft-1","status":"running"})
            ),
            (Some("ft-1".into()), Some("running".into()))
        );
        assert_eq!(
            training_ids(
                ApiShape::WatsonxTraining,
                &serde_json::json!({"metadata":{"id":"t-1"},"entity":{"status":{"state":"pending"}}})
            ),
            (Some("t-1".into()), Some("pending".into()))
        );
    }

    // ── Eval scoring — deterministic, over real output ───────────────────────

    #[test]
    fn exact_and_contains_scoring() {
        assert!(score(&Expectation::Exact { value: "42".into() }, " 42 \n", None).0);
        assert!(!score(&Expectation::Exact { value: "42".into() }, "43", None).0);
        assert!(
            score(
                &Expectation::Contains {
                    value: "fn main".into()
                },
                "pub fn main() {}",
                None
            )
            .0
        );
    }

    #[test]
    fn excludes_names_the_offending_substring() {
        let (ok, detail) = score(
            &Expectation::Excludes {
                values: vec!["unwrap(".into()],
            },
            "let x = y.unwrap();",
            None,
        );
        assert!(!ok);
        assert!(
            detail.contains("unwrap("),
            "detail should name the hit: {detail}"
        );
    }

    #[test]
    fn json_pointer_scoring_requires_real_json() {
        let e = Expectation::JsonPointer {
            pointer: "/name".into(),
        };
        assert!(score(&e, r#"{"name":"x"}"#, None).0);
        assert!(!score(&e, r#"{"other":1}"#, None).0);
        assert!(!score(&e, "not json", None).0);
        // Null is absent, not present.
        assert!(!score(&e, r#"{"name":null}"#, None).0);
    }

    #[test]
    fn command_scoring_actually_executes() {
        let dir = std::env::temp_dir().join(format!("vibecody-eval-{}", now_unix()));
        std::fs::create_dir_all(&dir).unwrap();

        let pass = score(
            &Expectation::Command {
                file: "out.txt".into(),
                command: "grep -q hello out.txt".into(),
            },
            "hello world",
            Some(&dir),
        );
        assert!(pass.0, "{}", pass.1);

        let fail = score(
            &Expectation::Command {
                file: "out.txt".into(),
                command: "grep -q nothere out.txt".into(),
            },
            "hello world",
            Some(&dir),
        );
        assert!(!fail.0);
        assert!(
            fail.1.contains("exited"),
            "should report the exit code: {}",
            fail.1
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn command_scoring_without_a_workdir_fails_rather_than_passing() {
        let (ok, _) = score(
            &Expectation::Command {
                file: "f".into(),
                command: "true".into(),
            },
            "x",
            None,
        );
        assert!(
            !ok,
            "a case that cannot be executed must never count as a pass"
        );
    }

    #[test]
    fn eval_suite_parses_from_toml() {
        let suite = EvalSuite::from_toml(
            r#"
            name = "smoke"
            model = "gpt-oss-120b"

            [[case]]
            id = "adds"
            prompt = "2+2, digits only"
            expect = { exact = "4" }

            [[case]]
            name = "compiles"
            prompt = "a rust hello world"
            expect = { file = "m.rs", command = "rustc --edition 2021 m.rs -o m" }
            "#,
        )
        .unwrap();
        assert_eq!(suite.cases.len(), 2);
        assert_eq!(
            suite.cases[0].expect,
            Expectation::Exact { value: "4".into() }
        );
        // `name` is accepted wherever `id` is.
        assert_eq!(suite.cases[1].id, "compiles");
    }

    #[test]
    fn a_misspelled_expect_key_names_itself_and_the_valid_ones() {
        let err = EvalSuite::from_toml(
            r#"
            name = "smoke"
            model = "m"
            [[case]]
            id = "a"
            prompt = "p"
            expect = { contian = "Paris" }
            "#,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("contian"), "should name the bad key: {err}");
        assert!(
            err.contains("contains"),
            "should list the valid keys: {err}"
        );
    }

    #[test]
    fn two_competing_checks_in_one_expect_table_are_rejected() {
        let err = EvalSuite::from_toml(
            r#"
            name = "smoke"
            model = "m"
            [[case]]
            id = "a"
            prompt = "p"
            expect = { exact = "4", contains = "4" }
            "#,
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("exactly one"),
            "should explain the conflict: {err}"
        );
    }

    #[test]
    fn a_command_expectation_defaults_its_scratch_filename() {
        let suite = EvalSuite::from_toml(
            r#"
            name = "smoke"
            model = "m"
            [[case]]
            id = "a"
            prompt = "p"
            expect = { command = "test -s output.txt" }
            "#,
        )
        .unwrap();
        assert_eq!(
            suite.cases[0].expect,
            Expectation::Command {
                file: "output.txt".into(),
                command: "test -s output.txt".into()
            }
        );
    }

    #[test]
    fn pass_rate_counts_errors_against_the_total() {
        let r = EvalReport {
            suite: "s".into(),
            backend_id: "b".into(),
            model: "m".into(),
            passed: 3,
            failed: 1,
            errored: 1,
            cases: vec![],
        };
        assert!(
            (r.pass_rate() - 0.6).abs() < 1e-9,
            "errors must not be excluded from the denominator"
        );
    }

    // ── Routing ──────────────────────────────────────────────────────────────

    fn client_with(cfg: ConfigSource) -> CloudClient {
        CloudClient::new(Catalog::embedded().unwrap(), cfg)
    }

    #[test]
    fn routing_picks_the_first_ready_backend_and_explains_the_skips() {
        let cfg = ConfigSource::memory().with_credential("digitalocean", "k");
        let d = client_with(cfg)
            .route(Stage::Serve, &RoutePolicy::FirstReady)
            .unwrap();
        assert_eq!(d.backend_id, "digitalocean");
        assert!(
            !d.skipped.is_empty(),
            "unconfigured backends should be reported"
        );
        assert!(d
            .skipped
            .iter()
            .any(|(id, why)| id == "azure" && !why.is_empty()));
    }

    #[test]
    fn ordered_routing_respects_the_configured_order() {
        let cfg = ConfigSource::memory()
            .with_credential("digitalocean", "k")
            .with_credential("custom_cloud", "k")
            .with_var("custom", "base_url", "http://localhost:8000/v1");
        let d = client_with(cfg)
            .route(
                Stage::Serve,
                &RoutePolicy::Ordered {
                    order: vec!["custom".into(), "digitalocean".into()],
                },
            )
            .unwrap();
        assert_eq!(d.backend_id, "custom");
    }

    #[test]
    fn cheapest_routing_uses_configured_price() {
        let cfg = ConfigSource::memory()
            .with_credential("digitalocean", "k")
            .with_var("digitalocean", "price_per_mtok", "0.90")
            .with_credential("custom_cloud", "k")
            .with_var("custom", "base_url", "http://localhost:8000/v1")
            .with_var("custom", "price_per_mtok", "0.10");
        let d = client_with(cfg)
            .route(Stage::Serve, &RoutePolicy::Cheapest)
            .unwrap();
        assert_eq!(d.backend_id, "custom");
        assert!(
            d.reason.contains("0.1"),
            "reason should quote the price: {}",
            d.reason
        );
    }

    #[test]
    fn routing_with_nothing_configured_errors_with_a_next_step() {
        let err = client_with(ConfigSource::memory())
            .route(Stage::Serve, &RoutePolicy::FirstReady)
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("vibecli --cloud-ai status"),
            "should point at the diagnostic: {msg}"
        );
    }

    #[test]
    fn training_routing_only_considers_training_backends() {
        let cfg = ConfigSource::memory()
            .with_credential("digitalocean", "k") // serve-only
            .with_credential("ibm_watsonx", "k")
            .with_var("ibm", "region", "us-south")
            .with_var("ibm", "project_id", "p")
            .with_var("ibm", "api_version", "2024-10-08");
        let d = client_with(cfg)
            .route(Stage::Train, &RoutePolicy::FirstReady)
            .unwrap();
        assert_eq!(
            d.backend_id, "ibm",
            "a serve-only backend must never win a train route"
        );
    }
}

// ── CLI ──────────────────────────────────────────────────────────────────────

/// `vibecli --cloud-ai <subcommand> [args...]`
///
/// Everything needed to wire up a cloud: inspect the catalog, see exactly what
/// each backend still needs, set those values, and exercise each stage against
/// the real provider.
pub async fn run_cli(subcommand: &str, args: &[String]) -> anyhow::Result<()> {
    match subcommand {
        "list" => cli_list(),
        "status" => cli_status(args),
        "set" => cli_set(args),
        "chat" => cli_chat(args).await,
        "train" => cli_train(args).await,
        "job" => cli_job(args).await,
        "eval" => cli_eval(args).await,
        "route" => cli_route(args),
        // Asking for help is a success, not a usage error — `--cloud-ai help`
        // should print the same text without an "unknown subcommand" scold or
        // a non-zero exit.
        "help" | "--help" | "-h" => {
            print_cloud_usage();
            Ok(())
        }
        other => {
            eprintln!("Unknown cloud subcommand: {other}\n");
            print_cloud_usage();
            std::process::exit(1);
        }
    }
}

pub fn print_cloud_usage() {
    eprintln!(
        "\
Cloud AI backends — serving, training, eval and routing.

  vibecli --cloud-ai list
      Show every backend, the stages it serves, and how its endpoints were verified.

  vibecli --cloud-ai status [serve|train]
      Show which backends are ready, and exactly what each unready one is missing.

  vibecli --cloud-ai set <backend> <var> <value>
      Store a configuration value (region, project, base_url, price_per_mtok, ...).
      Credentials go in the encrypted store separately: vibecli set-key <name> <value>

  vibecli --cloud-ai chat <backend> <model> <prompt...>
      Send one real completion.

  vibecli --cloud-ai train <backend> <base-model> <training-data> [suffix]
      Submit a real training job and print its id.

  vibecli --cloud-ai job <backend> <job-id>
      Poll a training job.

  vibecli --cloud-ai eval <backend> <suite.toml> [workdir]
      Run an eval suite: real completions, deterministic scoring.

  vibecli --cloud-ai route <serve|train> [first-ready|cheapest|ordered:a,b,c]
      Show which backend a policy selects, and why the others were skipped."
    );
}

fn cli_list() -> anyhow::Result<()> {
    let catalog = Catalog::load()?;
    println!(
        "{:<15} {:<34} {:<12} {}",
        "ID", "PROVIDER", "STAGES", "ENDPOINTS VERIFIED"
    );
    println!("{}", "-".repeat(96));
    for b in &catalog.backends {
        let stages: Vec<String> = b.stages.iter().map(|s| s.to_string()).collect();
        println!(
            "{:<15} {:<34} {:<12} {}",
            b.id,
            b.display_name,
            stages.join(","),
            b.verified.clone().unwrap_or_else(|| "-".into())
        );
    }
    println!(
        "\n{} backends. Eval and routing run on top of any serve-capable backend.",
        catalog.backends.len()
    );
    Ok(())
}

fn cli_status(args: &[String]) -> anyhow::Result<()> {
    let stage = args
        .first()
        .and_then(|s| Stage::parse(s))
        .unwrap_or(Stage::Serve);
    let client = CloudClient::open()?;
    println!("Readiness for the {stage} stage:\n");
    let mut ready = 0;
    for row in client.readiness(stage) {
        if row.ready {
            ready += 1;
            println!("  [ready]   {:<14} {}", row.backend_id, row.detail);
        } else {
            println!("  [config]  {:<14} {}", row.backend_id, row.detail);
        }
    }
    println!("\n{ready} backend(s) ready.");
    Ok(())
}

fn cli_set(args: &[String]) -> anyhow::Result<()> {
    let [backend, key, value, ..] = args else {
        anyhow::bail!("usage: vibecli --cloud-ai set <backend> <var> <value>");
    };
    let catalog = Catalog::load()?;
    catalog.get(backend)?; // reject typos before writing
    let config = ConfigSource::profile()?;
    config.set_var(backend, key, value)?;
    if value.trim().is_empty() {
        println!("Cleared {backend}.{key} — the backend needs it set again before use");
    } else {
        println!("Set {backend}.{key}");
    }
    Ok(())
}

async fn cli_chat(args: &[String]) -> anyhow::Result<()> {
    let [backend, model, rest @ ..] = args else {
        anyhow::bail!("usage: vibecli --cloud-ai chat <backend> <model> <prompt...>");
    };
    if rest.is_empty() {
        anyhow::bail!("no prompt given");
    }
    let client = CloudClient::open()?;
    let req = ChatRequest {
        model: model.clone(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: rest.join(" "),
        }],
        max_tokens: Some(2048),
    };
    let resp = client.chat(backend, &req).await?;
    println!("{}", resp.text);
    eprintln!(
        "\n[{} | {} | {}ms | in {} / out {}]",
        resp.backend_id,
        resp.model,
        resp.latency_ms,
        resp.input_tokens
            .map(|t| t.to_string())
            .unwrap_or_else(|| "?".into()),
        resp.output_tokens
            .map(|t| t.to_string())
            .unwrap_or_else(|| "?".into()),
    );
    Ok(())
}

async fn cli_train(args: &[String]) -> anyhow::Result<()> {
    let [backend, base_model, training_data, rest @ ..] = args else {
        anyhow::bail!(
            "usage: vibecli --cloud-ai train <backend> <base-model> <training-data> [suffix]"
        );
    };
    let client = CloudClient::open()?;
    let spec = TrainingSpec {
        base_model: base_model.clone(),
        training_data: training_data.clone(),
        validation_data: None,
        suffix: rest.first().cloned(),
        hyperparameters: HashMap::new(),
    };
    let job = client.submit_training(backend, &spec).await?;
    println!(
        "Submitted to {}: job {} ({})",
        job.backend_id, job.job_id, job.status
    );
    println!(
        "Poll with: vibecli --cloud-ai job {} {}",
        job.backend_id, job.job_id
    );
    Ok(())
}

async fn cli_job(args: &[String]) -> anyhow::Result<()> {
    let [backend, job_id, ..] = args else {
        anyhow::bail!("usage: vibecli --cloud-ai job <backend> <job-id>");
    };
    let client = CloudClient::open()?;
    let job = client.training_status(backend, job_id).await?;
    println!("{}: {}", job.job_id, job.status);
    Ok(())
}

async fn cli_eval(args: &[String]) -> anyhow::Result<()> {
    let [backend, suite_path, rest @ ..] = args else {
        anyhow::bail!("usage: vibecli --cloud-ai eval <backend> <suite.toml> [workdir]");
    };
    let text = std::fs::read_to_string(suite_path)
        .map_err(|e| anyhow::anyhow!("reading {suite_path}: {e}"))?;
    let suite = EvalSuite::from_toml(&text)?;
    let workdir = rest.first().map(PathBuf::from);

    let client = CloudClient::open()?;
    let report = client.run_eval(backend, &suite, workdir.as_deref()).await?;

    for case in &report.cases {
        let mark = if case.passed { "pass" } else { "FAIL" };
        println!(
            "  [{mark}] {:<28} {} ({}ms)",
            case.id, case.detail, case.latency_ms
        );
    }
    println!(
        "\n{}: {}/{} passed ({:.0}%) - {} failed, {} errored, on {}",
        report.suite,
        report.passed,
        report.cases.len(),
        report.pass_rate() * 100.0,
        report.failed,
        report.errored,
        report.backend_id,
    );
    if report.errored > 0 {
        eprintln!(
            "\nNote: errored cases count as not-passed; they are never scored optimistically."
        );
    }
    Ok(())
}

fn cli_route(args: &[String]) -> anyhow::Result<()> {
    let stage = args
        .first()
        .and_then(|s| Stage::parse(s))
        .unwrap_or(Stage::Serve);
    let policy = match args.get(1).map(String::as_str) {
        Some("cheapest") => RoutePolicy::Cheapest,
        Some(spec) if spec.starts_with("ordered:") => RoutePolicy::Ordered {
            order: spec
                .trim_start_matches("ordered:")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        },
        _ => RoutePolicy::FirstReady,
    };
    let client = CloudClient::open()?;
    let decision = client.route(stage, &policy)?;
    println!("-> {} ({})", decision.backend_id, decision.reason);
    if !decision.skipped.is_empty() {
        println!("\nSkipped:");
        for (id, why) in &decision.skipped {
            println!("  {id:<14} {why}");
        }
    }
    Ok(())
}

// No `From<CloudError> for anyhow::Error` here: CloudError implements
// std::error::Error, so anyhow's blanket impl already covers `?`.
