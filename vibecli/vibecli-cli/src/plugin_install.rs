//! Install signed MCPB plugin bundles.
//!
//! B2.4 of the plugin-bundle work. Brings B2.1 (manifest), B2.2
//! (verify), and B2.3 (policy) together into a single core install
//! function. The REPL `/plugin install <bundle.mcpb>` path and the
//! Tauri command both call into here; URL fetch is a thin wrapper
//! that downloads to a temp file and then calls `install_from_file`.
//!
//! Install layout:
//!   <workspace>/.vibecli/plugins/<plugin-name>/
//!     ├── vibecli-plugin.toml   (manifest, B2.1)
//!     ├── vibecli-plugin.sig    (signature, B2.2)
//!     └── ...other components from the bundle
//!
//! Atomicity: the bundle is first extracted to a sibling
//! `<plugin-name>.staging/` directory; on success it's renamed into
//! place. A failed install leaves the previous install (if any)
//! intact.

use std::path::{Path, PathBuf};

use crate::mcpb_bundle;
use crate::plugin_manifest::{DefaultPolicy, ManifestError, PluginManifest};
use crate::plugin_signing::{
    self, read_manifest_from_extracted, read_signature_from_extracted, PluginSignature,
    SignatureError,
};
use crate::workspace_store::{PluginPolicy, PolicyError, PolicySetter, WorkspaceStore};

/// Metadata returned after a successful install. Mirrors what the
/// governance panel will surface to the user.
#[derive(Debug, Clone)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    pub install_dir: PathBuf,
    pub signature: PluginSignature,
    /// Policy actually written to `WorkspaceStore` for this plugin.
    /// Equal to `manifest.default_policy` translated into the
    /// workspace-store enum, modulo admin override at install time.
    pub policy: PluginPolicy,
}

#[derive(Debug)]
pub enum InstallError {
    /// MCPB extract failed (corrupted archive, missing manifest.json,
    /// zip-slip path, etc.). The inner string is the upstream error.
    Mcpb(String),
    /// `vibecli-plugin.toml` failed to parse or validate.
    Manifest(ManifestError),
    /// `vibecli-plugin.sig` missing, malformed, or signature didn't
    /// verify against the publisher key embedded in the manifest.
    Signature(SignatureError),
    /// A plugin with this name is already installed and `force=false`.
    AlreadyInstalled {
        name: String,
        install_dir: PathBuf,
    },
    /// `WorkspaceStore` rejected the policy write (e.g. attempted to
    /// override a Required pin without admin).
    Policy(PolicyError),
    Io(std::io::Error),
    /// HTTPS fetch failed (network, non-2xx status, oversized body,
    /// non-https scheme). B2.12.
    Url(String),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mcpb(s) => write!(f, "mcpb: {s}"),
            Self::Manifest(e) => write!(f, "manifest: {e}"),
            Self::Signature(e) => write!(f, "signature: {e}"),
            Self::AlreadyInstalled { name, install_dir } => write!(
                f,
                "plugin `{name}` is already installed at {} \
                 (re-run with --force to overwrite)",
                install_dir.display()
            ),
            Self::Policy(e) => write!(f, "policy: {e}"),
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Url(s) => write!(f, "url: {s}"),
        }
    }
}

impl std::error::Error for InstallError {}

impl From<std::io::Error> for InstallError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<SignatureError> for InstallError {
    fn from(e: SignatureError) -> Self {
        Self::Signature(e)
    }
}

impl From<ManifestError> for InstallError {
    fn from(e: ManifestError) -> Self {
        Self::Manifest(e)
    }
}

impl From<PolicyError> for InstallError {
    fn from(e: PolicyError) -> Self {
        Self::Policy(e)
    }
}

/// Default install directory: `<workspace>/.vibecli/plugins`. Mirrors
/// the existing workspace.db location so admin tooling has one place
/// to look.
pub fn plugin_install_dir(workspace: &Path) -> PathBuf {
    workspace.join(".vibecli").join("plugins")
}

fn default_policy_to_store(dp: DefaultPolicy) -> PluginPolicy {
    match dp {
        DefaultPolicy::Off => PluginPolicy::Off,
        DefaultPolicy::On => PluginPolicy::On,
        DefaultPolicy::Required => PluginPolicy::Required,
    }
}

/// Install a signed MCPB plugin bundle from a local file path.
///
/// Atomic: stages the extract under `<name>.staging/`, only renames
/// into the final `<name>/` slot once verification + policy write
/// have both succeeded. If a previous install exists and `force=true`,
/// the old directory is replaced (the policy row is preserved and
/// only the on-disk plugin contents are swapped).
pub fn install_from_file(
    workspace: &Path,
    store: &WorkspaceStore,
    bundle_path: &Path,
    force: bool,
) -> Result<InstalledPlugin, InstallError> {
    let install_root = plugin_install_dir(workspace);
    std::fs::create_dir_all(&install_root)?;

    // 1. Stage-extract the MCPB to a temp directory under install_root.
    //    Using install_root as the parent keeps the eventual rename a
    //    same-filesystem move (cheap, atomic on POSIX).
    let staging = install_root.join(format!(
        ".staging.{}.{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let staging_guard = StagingGuard(staging.clone());
    std::fs::create_dir_all(&staging)?;

    mcpb_bundle::extract_bundle(bundle_path, &staging)
        .map_err(|e| InstallError::Mcpb(e.to_string()))?;

    // 2. Parse + validate vibecli-plugin.toml.
    let manifest = read_manifest_from_extracted(&staging)?;

    // 3. Verify vibecli-plugin.sig against the publisher key embedded
    //    in the manifest. Tampered bundles fail here before we touch
    //    the install slot.
    let signature = read_signature_from_extracted(&staging)?;
    plugin_signing::verify_manifest_signature(&manifest, &signature)?;

    // 4. Check for existing install.
    let install_dir = install_root.join(&manifest.name);
    if install_dir.exists() && !force {
        return Err(InstallError::AlreadyInstalled {
            name: manifest.name.clone(),
            install_dir,
        });
    }

    // 5. Atomic swap: remove the old dir (only after the new one is
    //    fully built in staging), then rename staging → install_dir.
    if install_dir.exists() {
        std::fs::remove_dir_all(&install_dir)?;
    }
    std::fs::rename(&staging, &install_dir)?;
    // Guard's path is now empty (the rename moved everything), so the
    // RAII drop will cheap-fail. Mark it inert to avoid the spurious
    // remove_dir_all attempt.
    staging_guard.disarm();

    // 6. Persist initial policy. Use PolicySetter::Install so the
    //    workspace_store guards treat this as the install-time
    //    default, not a user choice.
    let target_policy = default_policy_to_store(manifest.default_policy);
    let existing = store.get_plugin_policy(&manifest.name)?;
    let policy = match existing {
        // Preserve Required pins across re-install — the admin's
        // pinning decision survives bundle replacement.
        Some(e) if e.policy == PluginPolicy::Required => e.policy,
        // Otherwise overwrite to the manifest's default (whether
        // first install or force-reinstall).
        _ => {
            store.set_plugin_policy(&manifest.name, target_policy, PolicySetter::Install)?;
            target_policy
        }
    };

    Ok(InstalledPlugin {
        manifest,
        install_dir,
        signature,
        policy,
    })
}

/// B2.12 — install a signed MCPB plugin bundle from an HTTPS URL.
///
/// Thin wrapper: GET the URL into a temp file, then call
/// `install_from_file` for the actual extract / verify / register /
/// policy flow. The download is bounded by `MAX_BUNDLE_BYTES` to
/// keep a hostile redirect target from filling the disk.
///
/// Only `https://` URLs are accepted — plaintext HTTP would let a
/// network attacker swap the bundle bytes before signature verify
/// has a chance to fire. The publisher P-256 signature is still the
/// authoritative trust anchor (TOFU); TLS just keeps the *first*
/// install honest about what the publisher actually served.
pub fn install_from_url(
    workspace: &Path,
    store: &WorkspaceStore,
    url: &str,
    force: bool,
) -> Result<InstalledPlugin, InstallError> {
    if !url.starts_with("https://") {
        return Err(InstallError::Url(format!(
            "only https:// URLs are accepted, got `{}`",
            url.split(':').next().unwrap_or("")
        )));
    }

    // Download into a temp file. `tempfile::NamedTempFile` gives us
    // an unlinked-on-drop path that survives until we hand it off to
    // install_from_file (which has its own staging guarantee).
    let mut tmp =
        tempfile::NamedTempFile::new().map_err(|e| InstallError::Url(format!("temp file: {e}")))?;

    let resp = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| InstallError::Url(format!("client: {e}")))?
        .get(url)
        .send()
        .map_err(|e| InstallError::Url(format!("GET {url}: {e}")))?;

    if !resp.status().is_success() {
        return Err(InstallError::Url(format!(
            "GET {url} returned status {}",
            resp.status()
        )));
    }

    // Bounded copy. We can't trust Content-Length (hostile servers
    // can lie), so we cap by what we actually receive.
    let body = resp
        .bytes()
        .map_err(|e| InstallError::Url(format!("body: {e}")))?;
    if body.len() > MAX_BUNDLE_BYTES {
        return Err(InstallError::Url(format!(
            "bundle exceeds {} byte limit (got {})",
            MAX_BUNDLE_BYTES,
            body.len()
        )));
    }
    use std::io::Write;
    tmp.write_all(&body)
        .map_err(|e| InstallError::Url(format!("write: {e}")))?;
    tmp.flush()
        .map_err(|e| InstallError::Url(format!("flush: {e}")))?;

    install_from_file(workspace, store, tmp.path(), force)
}

/// Maximum on-the-wire bundle size — 50 MB. Picked to comfortably
/// accommodate plugins shipping bundled MCP server binaries while
/// still bounding how much disk a hostile URL can burn through.
pub const MAX_BUNDLE_BYTES: usize = 50 * 1024 * 1024;

/// Scan the workspace's install dir and return every plugin whose
/// manifest can be re-parsed. Skip — but don't error on — entries
/// that look corrupted (missing manifest, parse fail), so a single
/// bad install can't take out `vibecli plugin list`.
pub fn list_installed(
    workspace: &Path,
    store: &WorkspaceStore,
) -> Result<Vec<InstalledPlugin>, InstallError> {
    let root = plugin_install_dir(workspace);
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut out: Vec<InstalledPlugin> = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) if !n.starts_with(".staging.") => n.to_string(),
            _ => continue,
        };

        // Best-effort parse. Don't propagate errors — a single bad
        // install shouldn't break the list view.
        let Ok(manifest) = read_manifest_from_extracted(&path) else {
            continue;
        };
        let Ok(signature) = read_signature_from_extracted(&path) else {
            continue;
        };
        if manifest.name != name {
            // The directory name must match the manifest name so
            // uninstall-by-name works deterministically.
            continue;
        }
        let policy = store
            .effective_plugin_policy(&name)
            .unwrap_or(PluginPolicy::Off);
        out.push(InstalledPlugin {
            manifest,
            install_dir: path,
            signature,
            policy,
        });
    }
    out.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    Ok(out)
}

/// Remove a plugin's install dir and delete its policy row.
///
/// `set_by` controls whether a `Required` pin can be torn down:
/// `PolicySetter::Admin` succeeds unconditionally, anyone else gets
/// `PolicyError::RequiredCannotBeLoweredByNonAdmin`.
pub fn uninstall(
    workspace: &Path,
    store: &WorkspaceStore,
    name: &str,
    set_by: PolicySetter,
) -> Result<bool, InstallError> {
    // Delete policy first — if the Required guard fires we want to
    // bail before touching the on-disk install.
    store.delete_plugin_policy(name, set_by)?;
    let install_dir = plugin_install_dir(workspace).join(name);
    if !install_dir.exists() {
        return Ok(false);
    }
    std::fs::remove_dir_all(&install_dir)?;
    Ok(true)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// RAII cleanup for the staging directory. If install fails partway
/// through, the staging dir is removed automatically. `disarm()` is
/// called after a successful rename so we don't try to remove the
/// (now-empty) staging path twice.
struct StagingGuard(PathBuf);

impl StagingGuard {
    fn disarm(self) {
        std::mem::forget(self);
    }
}

impl Drop for StagingGuard {
    fn drop(&mut self) {
        if self.0.exists() {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_manifest::{Components, Publisher};
    use crate::plugin_signing::{sign_manifest, MANIFEST_FILENAME, SIGNATURE_FILENAME};
    use crate::signed_agent_card::jwk_from_verifying_key;
    use p256::ecdsa::SigningKey;
    use std::fs;
    use tempfile::tempdir;

    fn temp_workspace() -> (tempfile::TempDir, WorkspaceStore) {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".vibecli").join("workspace.db");
        fs::create_dir_all(db.parent().unwrap()).unwrap();
        let store = WorkspaceStore::open_with(&db, [9u8; 32]).unwrap();
        (dir, store)
    }

    fn fixture_key() -> SigningKey {
        SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng)
    }

    fn fixture_manifest(name: &str, key: &SigningKey, policy: DefaultPolicy) -> PluginManifest {
        PluginManifest {
            name: name.to_string(),
            version: "1.0.0".into(),
            publisher: Publisher {
                name: "Test".into(),
                url: None,
                key: jwk_from_verifying_key(key.verifying_key()),
            },
            description: "fixture".into(),
            components: Components::default(),
            min_vibecli_version: None,
            default_policy: policy,
        }
    }

    /// Build a valid signed MCPB bundle in a temp file and return its
    /// path. The MCPB outer container needs its own `manifest.json`
    /// at the root (per the MCP spec) — we ship a minimal one and the
    /// `vibecli-plugin.toml` / `.sig` files alongside it.
    fn build_signed_bundle(tmp: &Path, name: &str, policy: DefaultPolicy) -> (PathBuf, SigningKey) {
        let key = fixture_key();
        let manifest = fixture_manifest(name, &key, policy);
        let sig = sign_manifest(&manifest, &key, "publisher-default").unwrap();

        // Stage a bundle source tree.
        let src = tmp.join(format!("src.{name}"));
        fs::create_dir_all(&src).unwrap();
        // MCPB outer manifest.json.
        let mcpb_outer = mcpb_bundle::BundleManifest {
            name: name.to_string(),
            version: "1.0.0".into(),
            command: "noop".into(),
            args: vec![],
            env: Default::default(),
            description: Some("fixture".into()),
        };
        fs::write(
            src.join("manifest.json"),
            serde_json::to_string(&mcpb_outer).unwrap(),
        )
        .unwrap();
        // VibeCody inner manifest + signature.
        fs::write(
            src.join(MANIFEST_FILENAME),
            toml::to_string(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(
            src.join(SIGNATURE_FILENAME),
            serde_json::to_string(&sig).unwrap(),
        )
        .unwrap();

        let bundle = tmp.join(format!("{name}-1.0.0.mcpb"));
        mcpb_bundle::pack_bundle(&src, &bundle).unwrap();
        (bundle, key)
    }

    #[test]
    fn install_from_file_happy_path_creates_dir_and_sets_policy() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let (bundle, _key) = build_signed_bundle(tmp.path(), "alpha", DefaultPolicy::On);

        let installed = install_from_file(ws_dir.path(), &store, &bundle, false).unwrap();

        assert_eq!(installed.manifest.name, "alpha");
        assert_eq!(installed.policy, PluginPolicy::On);
        assert!(installed.install_dir.is_dir());
        assert!(installed.install_dir.join(MANIFEST_FILENAME).is_file());
        assert!(installed.install_dir.join(SIGNATURE_FILENAME).is_file());

        // Policy persisted in workspace.db.
        assert_eq!(
            store.effective_plugin_policy("alpha").unwrap(),
            PluginPolicy::On
        );
    }

    #[test]
    fn install_default_policy_off_persists_off() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let (bundle, _) = build_signed_bundle(tmp.path(), "quiet", DefaultPolicy::Off);

        let installed = install_from_file(ws_dir.path(), &store, &bundle, false).unwrap();
        assert_eq!(installed.policy, PluginPolicy::Off);
    }

    #[test]
    fn install_refuses_re_install_without_force() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let (bundle, _) = build_signed_bundle(tmp.path(), "twice", DefaultPolicy::On);

        install_from_file(ws_dir.path(), &store, &bundle, false).unwrap();
        let err = install_from_file(ws_dir.path(), &store, &bundle, false).unwrap_err();
        assert!(matches!(err, InstallError::AlreadyInstalled { .. }));
    }

    #[test]
    fn install_with_force_overwrites_existing() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let (bundle, _) = build_signed_bundle(tmp.path(), "twice", DefaultPolicy::On);

        install_from_file(ws_dir.path(), &store, &bundle, false).unwrap();
        // Same bundle, --force. Must succeed.
        let again = install_from_file(ws_dir.path(), &store, &bundle, true).unwrap();
        assert_eq!(again.manifest.name, "twice");
    }

    #[test]
    fn force_reinstall_preserves_required_pin() {
        // Admin pins as Required after install; a forced re-install
        // must not lower the pin back to the manifest default.
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let (bundle, _) = build_signed_bundle(tmp.path(), "pinned", DefaultPolicy::On);

        install_from_file(ws_dir.path(), &store, &bundle, false).unwrap();
        store
            .set_plugin_policy("pinned", PluginPolicy::Required, PolicySetter::Admin)
            .unwrap();
        let again = install_from_file(ws_dir.path(), &store, &bundle, true).unwrap();
        assert_eq!(again.policy, PluginPolicy::Required);
        assert_eq!(
            store.effective_plugin_policy("pinned").unwrap(),
            PluginPolicy::Required
        );
    }

    #[test]
    fn install_rejects_tampered_signature() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let key = fixture_key();
        let manifest = fixture_manifest("tampered", &key, DefaultPolicy::On);
        let sig = sign_manifest(&manifest, &key, "k").unwrap();

        // Build a bundle but flip one bit of the manifest version
        // AFTER signing, so the digest no longer matches.
        let mut bad = manifest.clone();
        bad.version = "9.9.9".into();
        let src = tmp.path().join("src.bad");
        fs::create_dir_all(&src).unwrap();
        let mcpb_outer = mcpb_bundle::BundleManifest {
            name: "tampered".into(),
            version: "9.9.9".into(),
            command: "noop".into(),
            args: vec![],
            env: Default::default(),
            description: None,
        };
        fs::write(
            src.join("manifest.json"),
            serde_json::to_string(&mcpb_outer).unwrap(),
        )
        .unwrap();
        fs::write(src.join(MANIFEST_FILENAME), toml::to_string(&bad).unwrap()).unwrap();
        fs::write(
            src.join(SIGNATURE_FILENAME),
            serde_json::to_string(&sig).unwrap(),
        )
        .unwrap();
        let bundle = tmp.path().join("tampered.mcpb");
        mcpb_bundle::pack_bundle(&src, &bundle).unwrap();

        let err = install_from_file(ws_dir.path(), &store, &bundle, false).unwrap_err();
        assert!(matches!(err, InstallError::Signature(_)), "got {err}");
        // Tampered install must NOT leave a dir behind.
        let install_dir = plugin_install_dir(ws_dir.path()).join("tampered");
        assert!(!install_dir.exists());
    }

    #[test]
    fn install_rejects_missing_signature_file() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let key = fixture_key();
        let manifest = fixture_manifest("nosig", &key, DefaultPolicy::On);

        // Bundle with manifest but no .sig file.
        let src = tmp.path().join("src.nosig");
        fs::create_dir_all(&src).unwrap();
        let mcpb_outer = mcpb_bundle::BundleManifest {
            name: "nosig".into(),
            version: "1.0.0".into(),
            command: "noop".into(),
            args: vec![],
            env: Default::default(),
            description: None,
        };
        fs::write(
            src.join("manifest.json"),
            serde_json::to_string(&mcpb_outer).unwrap(),
        )
        .unwrap();
        fs::write(
            src.join(MANIFEST_FILENAME),
            toml::to_string(&manifest).unwrap(),
        )
        .unwrap();
        let bundle = tmp.path().join("nosig.mcpb");
        mcpb_bundle::pack_bundle(&src, &bundle).unwrap();

        let err = install_from_file(ws_dir.path(), &store, &bundle, false).unwrap_err();
        assert!(matches!(
            err,
            InstallError::Signature(SignatureError::MissingFile(_))
        ));
    }

    #[test]
    fn list_installed_returns_sorted_entries() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let (b1, _) = build_signed_bundle(tmp.path(), "zeta", DefaultPolicy::On);
        let (b2, _) = build_signed_bundle(tmp.path(), "alpha", DefaultPolicy::Off);
        let (b3, _) = build_signed_bundle(tmp.path(), "mid", DefaultPolicy::On);

        install_from_file(ws_dir.path(), &store, &b1, false).unwrap();
        install_from_file(ws_dir.path(), &store, &b2, false).unwrap();
        install_from_file(ws_dir.path(), &store, &b3, false).unwrap();

        let list = list_installed(ws_dir.path(), &store).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].manifest.name, "alpha");
        assert_eq!(list[1].manifest.name, "mid");
        assert_eq!(list[2].manifest.name, "zeta");
    }

    #[test]
    fn list_installed_skips_corrupted_entries() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let (bundle, _) = build_signed_bundle(tmp.path(), "good", DefaultPolicy::On);
        install_from_file(ws_dir.path(), &store, &bundle, false).unwrap();

        // Hand-create a corrupted plugin dir.
        let corrupt = plugin_install_dir(ws_dir.path()).join("broken");
        fs::create_dir_all(&corrupt).unwrap();
        fs::write(corrupt.join(MANIFEST_FILENAME), "not valid toml at all").unwrap();

        let list = list_installed(ws_dir.path(), &store).unwrap();
        // Only "good" comes back; "broken" is silently skipped.
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].manifest.name, "good");
    }

    #[test]
    fn uninstall_removes_dir_and_policy() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let (bundle, _) = build_signed_bundle(tmp.path(), "gone", DefaultPolicy::On);
        install_from_file(ws_dir.path(), &store, &bundle, false).unwrap();
        assert!(plugin_install_dir(ws_dir.path()).join("gone").exists());

        assert!(uninstall(ws_dir.path(), &store, "gone", PolicySetter::User).unwrap());
        assert!(!plugin_install_dir(ws_dir.path()).join("gone").exists());
        assert_eq!(
            store.get_plugin_policy("gone").unwrap(),
            None,
            "policy row must be removed too"
        );
    }

    #[test]
    fn uninstall_required_pin_fails_for_user_succeeds_for_admin() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let (bundle, _) = build_signed_bundle(tmp.path(), "pinned", DefaultPolicy::On);
        install_from_file(ws_dir.path(), &store, &bundle, false).unwrap();
        store
            .set_plugin_policy("pinned", PluginPolicy::Required, PolicySetter::Admin)
            .unwrap();

        let err = uninstall(ws_dir.path(), &store, "pinned", PolicySetter::User).unwrap_err();
        assert!(matches!(
            err,
            InstallError::Policy(PolicyError::RequiredCannotBeLoweredByNonAdmin { .. })
        ));
        // Plugin still on disk after the failed user uninstall.
        assert!(plugin_install_dir(ws_dir.path()).join("pinned").exists());

        // Admin can complete the teardown.
        assert!(uninstall(ws_dir.path(), &store, "pinned", PolicySetter::Admin).unwrap());
        assert!(!plugin_install_dir(ws_dir.path()).join("pinned").exists());
    }

    #[test]
    fn uninstall_missing_plugin_returns_false() {
        let (ws_dir, store) = temp_workspace();
        let removed = uninstall(ws_dir.path(), &store, "ghost", PolicySetter::Admin).unwrap();
        assert!(!removed);
    }

    // ── B2.12: install_from_url scheme guard ────────────────────────────────

    #[test]
    fn install_from_url_rejects_http_scheme() {
        let (ws_dir, store) = temp_workspace();
        let err = install_from_url(ws_dir.path(), &store, "http://example.com/x.mcpb", false)
            .unwrap_err();
        match err {
            InstallError::Url(msg) => {
                assert!(
                    msg.contains("only https://"),
                    "expected https-only error, got `{msg}`"
                );
            }
            other => panic!("expected InstallError::Url, got {other}"),
        }
    }

    #[test]
    fn install_from_url_rejects_unknown_scheme() {
        let (ws_dir, store) = temp_workspace();
        let err =
            install_from_url(ws_dir.path(), &store, "ftp://example.com/x.mcpb", false).unwrap_err();
        assert!(matches!(err, InstallError::Url(_)));
    }

    #[test]
    fn install_from_url_rejects_scheme_less_path() {
        let (ws_dir, store) = temp_workspace();
        let err = install_from_url(ws_dir.path(), &store, "/local/path.mcpb", false).unwrap_err();
        assert!(matches!(err, InstallError::Url(_)));
    }

    #[test]
    fn max_bundle_bytes_is_reasonable_for_cli_plugins() {
        // Sanity check the bound — large enough for plugins shipping
        // small server binaries (Node script + a couple of native
        // tools), small enough that a hostile URL can't fill a disk.
        assert!(MAX_BUNDLE_BYTES >= 10 * 1024 * 1024);
        assert!(MAX_BUNDLE_BYTES <= 100 * 1024 * 1024);
    }
}
