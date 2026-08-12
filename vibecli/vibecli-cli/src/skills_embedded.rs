//! Embedded skill catalogue — the shipped `skills/*.md` tree compiled into
//! the binary, plus the one resolver that decides which directory the
//! catalogue loads from.
//!
//! ## Why this exists
//!
//! `skills_dir_default()` used to end at `${CARGO_MANIFEST_DIR}/skills`, a
//! path baked in at **compile time**. Release binaries are built in CI, so
//! every installed `vibecli` carried `/Users/runner/work/vibecody/vibecody/
//! vibecli/vibecli-cli/skills` — a directory that exists on no user's
//! machine. The documented next fallback, `<exe>/../share/vibecli/skills`,
//! was a convention nothing implemented: `release.yml` tars the bare
//! executable and the installer copies it to `~/.local/bin`, so no sibling
//! `share/` tree ever accompanies it. Net effect: `list_skills`,
//! `/v1/skilllens/skills`, and the SkillForge panel returned **zero** skills
//! on every install, while working fine in-tree — which is why it looked
//! like a regression rather than a packaging gap.
//!
//! Embedding is the only fallback that survives how the binary is actually
//! distributed (a single file, copied anywhere).
//!
//! ## Extraction, not in-memory parsing
//!
//! The catalogue's consumers want *files*: `skillforge_index` re-reads
//! `skill.path` to render a skill body and hands it to
//! `LensSkill::from_file` for scoring. Rather than teach three call sites
//! that a skill might have no path — and invite a `unwrap_or_default()`
//! that silently renders an empty body — the embedded tree is extracted
//! once to a per-version cache directory and the catalogue loads from
//! there like any other directory. One code path, and every reported path
//! points at a file that genuinely exists.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use include_dir::{include_dir, Dir};

/// The shipped catalogue, compiled into the binary. Includes the
/// `claude-code-prompts/` subtree; only top-level `*.md` files are loaded
/// as skills (`SkillCatalog::load_from` is non-recursive), but the whole
/// tree is extracted so relative references inside a skill still resolve.
static EMBEDDED: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/skills");

/// Cached result of the extraction step — the real error is kept, not
/// discarded, so a failure reports its own cause instead of a guess. Cached
/// because `list_skills` resolves the directory on every call.
static EXTRACTED: OnceLock<Result<PathBuf, String>> = OnceLock::new();

/// Number of top-level `*.md` files embedded in the binary. Used as the
/// completeness fingerprint for the extraction cache.
pub fn embedded_skill_count() -> usize {
    EMBEDDED
        .files()
        .filter(|f| f.path().extension().and_then(|e| e.to_str()) == Some("md"))
        .count()
}

/// Where the extracted copy lives: `~/.vibecli/bundled-skills/<version>`.
///
/// Version-scoped so an upgraded binary never serves the previous
/// release's catalogue, and deliberately *not* `~/.vibecli/skills` — that
/// path is the user's promoted-override dir (`skillforge_index::
/// promote_dir_for`) and must not be clobbered by extraction.
fn extraction_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| {
        h.join(".vibecli")
            .join("bundled-skills")
            .join(env!("CARGO_PKG_VERSION"))
    })
}

/// Marker written **after** every file lands, so a run interrupted
/// mid-extraction is retried rather than mistaken for a complete cache.
/// Contents are the fingerprint the next run compares against.
fn marker_path(dir: &Path) -> PathBuf {
    dir.join(".extracted")
}

fn fingerprint() -> String {
    format!("{} {}", env!("CARGO_PKG_VERSION"), embedded_skill_count())
}

/// True when `dir` already holds a complete extraction of *this* binary's
/// catalogue.
fn is_extracted(dir: &Path) -> bool {
    std::fs::read_to_string(marker_path(dir))
        .map(|s| s.trim() == fingerprint())
        .unwrap_or(false)
}

/// Write every embedded file under `dir`, preserving the subtree layout.
fn write_tree(dir: &Path, node: &Dir<'_>) -> Result<()> {
    for sub in node.dirs() {
        let target = dir.join(sub.path());
        std::fs::create_dir_all(&target)
            .with_context(|| format!("create_dir_all {}", target.display()))?;
        write_tree(dir, sub)?;
    }
    for file in node.files() {
        let target = dir.join(file.path());
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create_dir_all {}", parent.display()))?;
        }
        std::fs::write(&target, file.contents())
            .with_context(|| format!("write {}", target.display()))?;
    }
    Ok(())
}

/// Best-effort removal of extractions left behind by other versions, so
/// the cache doesn't grow one full copy per release. Failures are ignored
/// — a stale directory is wasted disk, not a broken catalogue.
fn prune_other_versions(current: &Path) {
    let Some(parent) = current.parent() else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() && p != current {
            let _ = std::fs::remove_dir_all(&p);
        }
    }
}

/// Extract the embedded catalogue to its cache directory, returning the
/// path. Idempotent: a complete extraction of the same version is reused.
pub fn ensure_extracted() -> Result<PathBuf> {
    let dir = extraction_dir().context("no home directory — cannot extract bundled skills")?;
    if is_extracted(&dir) {
        return Ok(dir);
    }
    std::fs::create_dir_all(&dir).with_context(|| format!("create_dir_all {}", dir.display()))?;
    write_tree(&dir, &EMBEDDED)?;
    std::fs::write(marker_path(&dir), fingerprint())
        .with_context(|| format!("write {}", marker_path(&dir).display()))?;
    prune_other_versions(&dir);
    Ok(dir)
}

/// How the catalogue directory was chosen. Surfaced by `vibecli doctor`
/// so an empty catalogue names its own cause instead of just reporting
/// zero skills.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillsDirOrigin {
    /// `VIBECLI_SKILLS_DIR` was set — used verbatim, no fallback.
    EnvOverride,
    /// `${CARGO_MANIFEST_DIR}/skills` exists — an in-tree build.
    Manifest,
    /// `<exe>/../share/vibecli/skills` exists — a packaged install.
    Packaged,
    /// Extracted from the binary's embedded copy.
    Embedded,
    /// Nothing on disk and extraction failed; the path is the (missing)
    /// manifest dir so the loader reports a real error rather than
    /// pretending the catalogue is legitimately empty.
    Unavailable(String),
}

/// Resolve the skills directory and say how it was chosen.
///
/// Precedence:
///   1. `VIBECLI_SKILLS_DIR` — explicit operator/test override, verbatim.
///   2. `${CARGO_MANIFEST_DIR}/skills` — in-tree builds, so edits to a
///      skill file take effect without a rebuild.
///   3. `<exe>/../share/vibecli/skills` — distro packages that do lay out
///      a `share/` tree.
///   4. The embedded copy, extracted to `~/.vibecli/bundled-skills/<ver>`.
pub fn resolve_skills_dir_with_origin() -> (PathBuf, SkillsDirOrigin) {
    if let Ok(p) = std::env::var("VIBECLI_SKILLS_DIR") {
        if !p.is_empty() {
            return (PathBuf::from(p), SkillsDirOrigin::EnvOverride);
        }
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("skills");
    if manifest_dir.is_dir() {
        return (manifest_dir, SkillsDirOrigin::Manifest);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("../share/vibecli/skills");
            if candidate.is_dir() {
                return (candidate, SkillsDirOrigin::Packaged);
            }
        }
    }
    match EXTRACTED.get_or_init(|| ensure_extracted().map_err(|e| format!("{e:#}"))) {
        Ok(dir) => (dir.clone(), SkillsDirOrigin::Embedded),
        Err(why) => (
            manifest_dir,
            SkillsDirOrigin::Unavailable(format!("cannot extract embedded skills: {why}")),
        ),
    }
}

/// Resolve the skills directory. The single implementation — `mcp_server`
/// and `skillforge_index` both call this rather than keeping their own
/// copies of the fallback chain.
pub fn resolve_skills_dir() -> PathBuf {
    resolve_skills_dir_with_origin().0
}

/// Serialises every test that redirects `VIBECLI_SKILLS_DIR`.
///
/// `set_var` is process-global and cargo runs tests on parallel threads, so
/// one test would repoint the skills dir while another was mid-dispatch.
/// **One lock for the whole crate** — `mcp_server`'s skills tests take this
/// same mutex; two independent locks would not serialise against each other.
#[cfg(test)]
pub(crate) static SKILLS_DIR_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Poison-tolerant acquire of [`SKILLS_DIR_ENV_LOCK`] — one panicking test
/// must not cascade into every other skills test.
#[cfg(test)]
pub(crate) fn skills_dir_env_lock() -> std::sync::MutexGuard<'static, ()> {
    SKILLS_DIR_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of the module: the binary must carry the shipped
    /// catalogue, not an empty directory. If this drops to zero, every
    /// installed build silently lists no skills again.
    ///
    /// **Zero is the bug; there is deliberately no upper or lower bound
    /// beyond it.** A threshold like `> 1000` records what the catalogue
    /// happened to hold the day it was written — it goes stale on the next
    /// import and, worse, passes while a third of the tree is missing. How
    /// many skills ship is a product decision; what has to hold is that each
    /// one is reachable from the context it belongs to, which is
    /// [`every_embedded_skill_is_reachable_by_its_own_context`] below.
    #[test]
    fn embedded_tree_is_not_empty() {
        assert!(
            embedded_skill_count() > 0,
            "the binary carries no skills — every installed build would list none"
        );
    }

    /// Size does not matter, reachability does. The property, independent of
    /// how many skills ship: a skill is returned by a `list()` query for its
    /// own name, and for each trigger it declares — the two precise ways
    /// context reaches it.
    ///
    /// Checked over a fixed stride rather than the full cross product, which
    /// would be quadratic over every skill body.
    #[test]
    fn every_embedded_skill_is_retrievable_by_its_own_name_and_triggers() {
        let tmp = tempfile::tempdir().unwrap();
        write_tree(tmp.path(), &EMBEDDED).unwrap();
        let cat = crate::skill_catalog::SkillCatalog::load_from(tmp.path()).unwrap();

        // Every 47th skill — a fixed stride, so a failure reproduces.
        for skill in cat.all().iter().step_by(47) {
            let by_name = cat.list(None, Some(&skill.name));
            assert!(
                by_name.iter().any(|s| s.name == skill.name),
                "{} is not returned by a query for its own name",
                skill.name
            );

            for trigger in skill
                .frontmatter
                .triggers
                .iter()
                .filter(|t| !t.trim().is_empty())
            {
                let by_trigger = cat.list(None, Some(trigger));
                assert!(
                    by_trigger.iter().any(|s| s.name == skill.name),
                    "{} is not returned by a query for its own trigger {trigger:?}",
                    skill.name
                );
            }
        }
    }

    /// Triggers and category are the *precise* half of `skill_matches_query`
    /// — the half a caller can rely on. A skill with neither is not
    /// unreachable (the body is substring-matched too) but it is reachable
    /// only by accident: it never matches a category filter, and it surfaces
    /// for a free-text query only when the words happen to appear somewhere
    /// in its prose.
    ///
    /// Backfilled 2026-08-10: 157 pre-import skills carried no YAML
    /// frontmatter at all and parsed to `SkillFrontmatter::default()`. This
    /// test is what keeps the next one from slipping in — a skill file with
    /// no frontmatter is a silent regression everywhere else.
    #[test]
    fn every_embedded_skill_declares_triggers_and_a_category() {
        let tmp = tempfile::tempdir().unwrap();
        write_tree(tmp.path(), &EMBEDDED).unwrap();
        let cat = crate::skill_catalog::SkillCatalog::load_from(tmp.path()).unwrap();

        let bare: Vec<&str> = cat
            .all()
            .iter()
            .filter(|s| {
                s.frontmatter.triggers.iter().all(|t| t.trim().is_empty())
                    || s.frontmatter.category.is_none()
            })
            .map(|s| s.name.as_str())
            .collect();
        assert!(
            bare.is_empty(),
            "{} skill(s) declare no trigger or no category, so they match only by body text: {:?}",
            bare.len(),
            &bare[..bare.len().min(10)]
        );
    }

    #[test]
    fn extraction_writes_every_embedded_skill_and_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("bundled");
        std::fs::create_dir_all(&dir).unwrap();
        write_tree(&dir, &EMBEDDED).unwrap();
        std::fs::write(marker_path(&dir), fingerprint()).unwrap();

        let on_disk = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
            .count();
        assert_eq!(on_disk, embedded_skill_count());

        // A complete extraction of the same fingerprint is recognised.
        assert!(is_extracted(&dir));
    }

    /// End-to-end for the path a release binary takes: embedded bytes →
    /// extracted directory → parsed catalogue. `cargo test` runs in-tree,
    /// where the resolver picks the manifest dir and never touches the
    /// embedded copy, so without this the shipped path has no coverage at
    /// all — exactly how it shipped broken.
    ///
    /// Extracts to a `TempDir` rather than calling `ensure_extracted()`,
    /// which would write to the developer's real `~/.vibecli`.
    #[test]
    fn embedded_catalogue_parses_into_skills() {
        let tmp = tempfile::tempdir().unwrap();
        write_tree(tmp.path(), &EMBEDDED).unwrap();

        let cat = crate::skill_catalog::SkillCatalog::load_from(tmp.path()).unwrap();
        assert_eq!(
            cat.len(),
            embedded_skill_count(),
            "every embedded skill must parse"
        );
        // Categorised, not "categorised into at least N buckets" — how many
        // categories the catalogue uses is a product decision. What breaks
        // the category filter is having none at all.
        assert!(
            !cat.categories().is_empty(),
            "no skill carries a category — the category filter would match nothing"
        );
    }

    #[test]
    fn partial_extraction_is_not_mistaken_for_complete() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("bundled");
        std::fs::create_dir_all(&dir).unwrap();
        // Files present but no marker — must re-extract.
        std::fs::write(dir.join("a-skill.md"), "# partial").unwrap();
        assert!(!is_extracted(&dir));

        // Marker from a different build — must re-extract.
        std::fs::write(marker_path(&dir), "0.0.0 1").unwrap();
        assert!(!is_extracted(&dir));
    }

    #[test]
    fn prune_removes_other_versions_but_keeps_current() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("bundled-skills");
        let current = root.join("9.9.9");
        let stale = root.join("0.0.1");
        std::fs::create_dir_all(&current).unwrap();
        std::fs::create_dir_all(&stale).unwrap();

        prune_other_versions(&current);

        assert!(current.is_dir(), "current version must survive");
        assert!(!stale.exists(), "stale version must be pruned");
    }

    /// `VIBECLI_SKILLS_DIR` is an override, not a hint — it must be used
    /// verbatim even when it names a directory that does not exist, so an
    /// operator pointing at the wrong path gets an error rather than a
    /// silent fall-through to the embedded copy.
    #[test]
    fn env_override_wins_verbatim() {
        let _guard = skills_dir_env_lock();
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("nope");
        std::env::set_var("VIBECLI_SKILLS_DIR", &missing);
        let (dir, origin) = resolve_skills_dir_with_origin();
        std::env::remove_var("VIBECLI_SKILLS_DIR");

        assert_eq!(dir, missing);
        assert_eq!(origin, SkillsDirOrigin::EnvOverride);
    }
}
