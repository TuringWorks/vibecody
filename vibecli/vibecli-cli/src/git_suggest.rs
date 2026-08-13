//! "This file isn't in git — want a repository?"
//!
//! Editing code that no version control is watching is a bad afternoon waiting
//! to happen, and the agent writes files fast enough that the first `git init`
//! often comes too late to recover the interesting intermediate state. This
//! module decides *whether* to raise that, and remembers a "no" so it is asked
//! once per directory rather than after every write.
//!
//! The decision ([`evaluate`]) is a pure function over a path, a workspace root
//! and a "has this been declined?" predicate, so every branch is testable
//! without a database or a terminal. Persistence ([`DeclineLog`]) is a thin
//! shell around it.
//!
//! Two rules the shape of this module exists to enforce:
//!
//! * **Discovery, not `open`.** Whether a file is under version control is
//!   answered by walking up to the enclosing repo. `vibe_core::git::is_git_repo`
//!   opens a path directly and so reports "no repo" for every subdirectory of
//!   an ordinary checkout — using it here would offer to create a nested repo
//!   inside the user's existing one on the first edit to `src/`.
//! * **Silence is not consent.** A prompt that cannot be answered — piped
//!   stdin, ACP stdio mode, a daemon — must resolve to "don't init", never to
//!   "go ahead". Callers get that by only ever asking when
//!   [`Decision::Offer`] comes back *and* they hold a usable terminal.

use crate::sync_ext::LockRecover;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The panel name under which declines are stored in the profile database.
const PANEL: &str = "git_suggest";

/// Paths written since the last drain.
///
/// The write happens deep inside a tool call, where there is no terminal to
/// prompt on and no safe moment to block; the question has to be asked later,
/// between turns. So the tool path only *records*, and the REPL drains this
/// between turns and decides. Effects at the edge, per AGENTS.md.
fn edited_paths() -> &'static Mutex<BTreeSet<PathBuf>> {
    static EDITED: OnceLock<Mutex<BTreeSet<PathBuf>>> = OnceLock::new();
    EDITED.get_or_init(|| Mutex::new(BTreeSet::new()))
}

/// How many paths to retain between drains.
///
/// The REPL drains every turn, so it never comes close. The daemon shares this
/// tool executor and **never** drains — without a bound, a long-lived
/// `vibecli --serve` would accumulate one `PathBuf` per file written for its
/// entire life. The tradeoff is explicit: past the cap, later writes are not
/// recorded, so a turn that edits more than this many files could miss an
/// untracked one. Deciding to suggest a repo is worth far less than a daemon
/// that leaks.
const MAX_RECORDED: usize = 256;

/// Record that `path` was written by a tool. Cheap and non-blocking: this runs
/// on every agent file write, so it must never touch git or the database.
pub fn note_edited(path: &Path) {
    let mut set = edited_paths().lock_recover();
    if set.len() < MAX_RECORDED {
        set.insert(path.to_path_buf());
    }
}

/// Take everything recorded since the last call, leaving the set empty.
pub fn take_edited() -> Vec<PathBuf> {
    std::mem::take(&mut *edited_paths().lock_recover())
        .into_iter()
        .collect()
}

/// What, if anything, to raise with the user about a path that was just edited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// The path is inside this repository. Nothing to say.
    AlreadyTracked { root: PathBuf },
    /// Offer to create a repository at `dir`.
    Offer { dir: PathBuf },
    /// The user has already declined for `dir`; stay quiet.
    Declined { dir: PathBuf },
    /// Not worth asking about, and why. Kept distinct from `Declined` so a
    /// caller can tell "the user said no" from "we would never ask here".
    Skip { reason: SkipReason },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// The edited path is not under the workspace root, so we have no
    /// defensible directory to propose as the repository root.
    OutsideWorkspace,
    /// The workspace root is somewhere we refuse to create a repository —
    /// the home directory, the filesystem root, a non-directory.
    UnsuitableLocation(String),
}

impl Decision {
    /// The directory a repository would be created in, when there is an offer.
    pub fn offered_dir(&self) -> Option<&Path> {
        match self {
            Decision::Offer { dir } => Some(dir),
            _ => None,
        }
    }
}

/// Decide what to raise about `edited_path`, given the workspace it belongs to.
///
/// `is_declined` is passed in rather than read from disk so the rules can be
/// tested exhaustively and so a caller with its own memory (a UI that has
/// already dismissed a banner this session) can supply it.
///
/// The repository we offer to create is always the **workspace root**, never
/// the edited file's own directory: a user editing `src/lib.rs` wants one repo
/// at the project root, not one inside `src/`.
pub fn evaluate(
    edited_path: &Path,
    workspace_root: &Path,
    is_declined: impl Fn(&Path) -> bool,
) -> Decision {
    // Walking up matters here: the edited file is usually several directories
    // below the repo root, and `is_git_repo` on its parent would say "no".
    if let Some(root) = vibe_core::git::discover_repo_root(edited_path) {
        return Decision::AlreadyTracked { root };
    }

    // Compare resolved paths — on macOS a workspace reached via `/var/...` and
    // an edited path reported as `/private/var/...` are the same directory, and
    // a raw `starts_with` would call that "outside the workspace".
    let root = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());
    let edited = edited_path
        .canonicalize()
        .unwrap_or_else(|_| edited_path.to_path_buf());
    if !edited.starts_with(&root) {
        return Decision::Skip {
            reason: SkipReason::OutsideWorkspace,
        };
    }

    if let Some(objection) = vibe_core::git::repo_location_objection(&root) {
        return Decision::Skip {
            reason: SkipReason::UnsuitableLocation(objection),
        };
    }

    if is_declined(&root) {
        return Decision::Declined { dir: root };
    }

    Decision::Offer { dir: root }
}

/// Remembers which directories the user has already declined a repository for.
///
/// Stored in the encrypted profile database (`~/.vibecli/profile_settings.db`)
/// rather than in the workspace: a directory the user has just told us not to
/// put a repo in is not a directory we should start writing state files into.
pub struct DeclineLog {
    store: crate::profile_store::ProfileStore,
    profile_id: String,
}

impl DeclineLog {
    /// Open the log for the default profile.
    pub fn open() -> Result<Self, String> {
        let store = crate::profile_store::ProfileStore::new()?;
        let profile_id = store.get_default_profile_id()?;
        Ok(Self { store, profile_id })
    }

    /// Open against an explicit database path and key — for tests, so they
    /// never touch the developer's real profile database.
    pub fn open_with(path: &PathBuf, key: [u8; 32], profile_id: &str) -> Result<Self, String> {
        Ok(Self {
            store: crate::profile_store::ProfileStore::open_with(path, key)?,
            profile_id: profile_id.to_string(),
        })
    }

    /// The storage key for a directory. Canonicalised so the same directory
    /// reached by two different paths is one entry, not two.
    fn key(dir: &Path) -> String {
        dir.canonicalize()
            .unwrap_or_else(|_| dir.to_path_buf())
            .to_string_lossy()
            .into_owned()
    }

    /// Whether the user has already declined a repository for `dir`.
    ///
    /// A database that cannot be read reports `false`. That errs toward asking
    /// a question the user may have already answered, which is a smaller harm
    /// than silently withholding version control — but it is a real tradeoff,
    /// not an oversight.
    pub fn is_declined(&self, dir: &Path) -> bool {
        self.store
            .get(&self.profile_id, PANEL, &Self::key(dir))
            .ok()
            .flatten()
            .is_some_and(|v| v == "declined")
    }

    /// Record that the user does not want a repository in `dir`.
    pub fn decline(&self, dir: &Path) -> Result<(), String> {
        self.store
            .set(&self.profile_id, PANEL, &Self::key(dir), "declined")
    }

    /// Forget a decline, so the suggestion can be offered again.
    pub fn reset(&self, dir: &Path) -> Result<(), String> {
        match self.store.delete(&self.profile_id, PANEL, &Self::key(dir)) {
            Ok(()) => Ok(()),
            // Deleting something that was never recorded is the desired state,
            // not a failure worth surfacing to the caller.
            Err(e) if e.contains("no rows") => Ok(()),
            Err(e) => Err(e),
        }
    }
}

/// The single directory worth offering a repository for, across every path
/// recorded this turn, or `None` if there is nothing to raise.
///
/// Many writes in one turn are the normal case and they nearly always share a
/// workspace, so this collapses them to one question rather than one per file.
pub fn pending_offer(
    edited: &[PathBuf],
    workspace_root: &Path,
    log: &DeclineLog,
) -> Option<PathBuf> {
    edited.iter().find_map(|p| {
        evaluate(p, workspace_root, |d| log.is_declined(d))
            .offered_dir()
            .map(Path::to_path_buf)
    })
}

/// What came of offering to create a repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfferOutcome {
    /// The user was not asked — no terminal to ask on.
    NotAsked,
    /// The user said no; recorded so we do not ask again for this directory.
    Declined,
    /// A repository now exists at this root.
    Initialized(PathBuf),
    /// `git init` was attempted and failed. Reported, never swallowed.
    Failed(String),
}

/// Ask about `dir`, and create the repository if the answer is yes.
///
/// `ask` returns `None` when the question cannot be put to anyone — piped
/// stdin, ACP stdio mode, a daemon. That resolves to [`OfferOutcome::NotAsked`]
/// and creates nothing: an unanswerable prompt is not consent, and it is also
/// not a decline, so nothing is written to the log and the user still gets
/// asked the first time they run interactively.
pub fn offer(
    dir: &Path,
    log: &DeclineLog,
    ask: impl FnOnce(&Path) -> Option<bool>,
) -> OfferOutcome {
    match ask(dir) {
        None => OfferOutcome::NotAsked,
        Some(false) => {
            // A failure to persist the decline would mean re-asking next turn,
            // which is exactly the nagging the user just opted out of — so say
            // so rather than pretending it stuck.
            match log.decline(dir) {
                Ok(()) => OfferOutcome::Declined,
                Err(e) => OfferOutcome::Failed(format!("could not record your choice: {e}")),
            }
        }
        Some(true) => match vibe_core::git::init_repo(dir) {
            Ok(root) => OfferOutcome::Initialized(root),
            Err(e) => OfferOutcome::Failed(e.to_string()),
        },
    }
}

/// Whether the GitHub CLI is installed *and* logged in.
///
/// Both halves matter: `gh` on PATH but unauthenticated turns an offer to
/// create a repository into an error the user did not ask for, so an offer is
/// only made when the command would actually be able to run.
pub fn gh_is_ready() -> bool {
    std::process::Command::new("gh")
        .args(["auth", "status"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Whether `repo` has at least one commit. A brand-new repository does not,
/// and pushing one is an error — worth saying plainly rather than discovering
/// through a failed push.
pub fn has_commits(repo: &Path) -> bool {
    std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "--verify", "HEAD"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Create a GitHub repository for `dir` and wire it up as `origin`.
///
/// Deliberately shells out to `gh` rather than reimplementing the GitHub API:
/// `gh` already holds the user's credentials, so this needs no token of its
/// own and no new place for one to leak. It does **not** push — a fresh repo
/// has nothing to push, and a push here would either fail or commit on the
/// user's behalf without being asked.
pub fn create_github_remote(dir: &Path, name: &str, private: bool) -> Result<String, String> {
    let visibility = if private { "--private" } else { "--public" };
    let out = std::process::Command::new("gh")
        .arg("-C")
        .arg(dir)
        .args(["repo", "create", name, visibility])
        .arg("--source")
        .arg(dir)
        .args(["--remote", "origin"])
        .output()
        .map_err(|e| format!("could not run gh: {e}"))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("gh repo create exited with {}", out.status)
        } else {
            stderr
        });
    }
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // gh prints the repo URL on success; fall back to the name rather than
    // inventing a URL we did not receive.
    Ok(stdout
        .lines()
        .find(|l| l.starts_with("http"))
        .unwrap_or(name)
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const TEST_KEY: [u8; 32] = [42u8; 32];

    /// The edit recorder is process-global, so the tests that drain it must not
    /// run alongside each other — otherwise one test's drain eats another's
    /// recorded paths and both fail intermittently. Poison-tolerant so one
    /// failing test does not cascade into the rest.
    static RECORDER_LOCK: Mutex<()> = Mutex::new(());

    fn log_in(dir: &TempDir) -> DeclineLog {
        DeclineLog::open_with(&dir.path().join("profile.db"), TEST_KEY, "default")
            .expect("open decline log")
    }

    // ── evaluate: the pure decision ──────────────────────────────────────────

    #[test]
    fn offers_a_repo_for_a_file_in_an_untracked_workspace() {
        let ws = TempDir::new().unwrap();
        let file = ws.path().join("main.rs");
        std::fs::write(&file, "fn main() {}").unwrap();

        let decision = evaluate(&file, ws.path(), |_| false);

        assert_eq!(
            decision.offered_dir().map(|d| d.canonicalize().unwrap()),
            Some(ws.path().canonicalize().unwrap())
        );
    }

    #[test]
    fn offers_the_workspace_root_not_the_files_own_directory() {
        let ws = TempDir::new().unwrap();
        let nested = ws.path().join("src").join("inner");
        std::fs::create_dir_all(&nested).unwrap();
        let file = nested.join("lib.rs");
        std::fs::write(&file, "// code").unwrap();

        let decision = evaluate(&file, ws.path(), |_| false);

        assert_eq!(
            decision.offered_dir().map(|d| d.canonicalize().unwrap()),
            Some(ws.path().canonicalize().unwrap()),
            "a repo belongs at the project root, not inside src/inner"
        );
    }

    #[test]
    fn stays_quiet_for_a_file_deep_inside_an_existing_repo() {
        let ws = TempDir::new().unwrap();
        vibe_core::git::init_repo(ws.path()).unwrap();
        let nested = ws.path().join("src").join("deeply").join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let file = nested.join("mod.rs");
        std::fs::write(&file, "// code").unwrap();

        // The regression guard: `is_git_repo(nested)` is false, so a
        // non-discovering check here would offer a repo inside a repo.
        assert!(!vibe_core::git::is_git_repo(&nested));
        assert!(matches!(
            evaluate(&file, ws.path(), |_| false),
            Decision::AlreadyTracked { .. }
        ));
    }

    #[test]
    fn respects_a_previous_decline() {
        let ws = TempDir::new().unwrap();
        let file = ws.path().join("main.rs");
        std::fs::write(&file, "fn main() {}").unwrap();

        let decision = evaluate(&file, ws.path(), |_| true);

        assert!(matches!(decision, Decision::Declined { .. }));
        assert_eq!(decision.offered_dir(), None);
    }

    #[test]
    fn skips_a_file_outside_the_workspace() {
        let ws = TempDir::new().unwrap();
        let elsewhere = TempDir::new().unwrap();
        let file = elsewhere.path().join("stray.rs");
        std::fs::write(&file, "// not ours").unwrap();

        assert_eq!(
            evaluate(&file, ws.path(), |_| false),
            Decision::Skip {
                reason: SkipReason::OutsideWorkspace
            }
        );
    }

    #[test]
    fn skips_when_the_workspace_root_is_an_unsuitable_location() {
        // "/" is refused as a repo location, and every path is under it, so the
        // outside-workspace check passes and the location check is what fires.
        let file = Path::new("/etc/hosts");
        let decision = evaluate(file, Path::new("/"), |_| false);

        assert!(
            matches!(
                decision,
                Decision::Skip {
                    reason: SkipReason::UnsuitableLocation(_)
                }
            ),
            "got {decision:?}"
        );
    }

    #[test]
    fn a_declined_directory_is_never_reported_as_an_offer() {
        let ws = TempDir::new().unwrap();
        let file = ws.path().join("a.rs");
        std::fs::write(&file, "//").unwrap();

        for decision in [
            evaluate(&file, ws.path(), |_| true),
            evaluate(&file, ws.path(), |_| false),
        ] {
            let offered = decision.offered_dir().is_some();
            let declined = matches!(decision, Decision::Declined { .. });
            assert!(!(offered && declined));
        }
    }

    // ── DeclineLog: persistence ──────────────────────────────────────────────

    #[test]
    fn a_fresh_directory_is_not_declined() {
        let tmp = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        assert!(!log_in(&tmp).is_declined(ws.path()));
    }

    #[test]
    fn decline_is_remembered() {
        let tmp = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        let log = log_in(&tmp);

        log.decline(ws.path()).unwrap();

        assert!(log.is_declined(ws.path()));
    }

    #[test]
    fn decline_survives_reopening_the_store() {
        let tmp = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        log_in(&tmp).decline(ws.path()).unwrap();

        // The whole point of persisting: a new session must not re-ask.
        assert!(log_in(&tmp).is_declined(ws.path()));
    }

    #[test]
    fn decline_is_scoped_to_one_directory() {
        let tmp = TempDir::new().unwrap();
        let declined = TempDir::new().unwrap();
        let other = TempDir::new().unwrap();
        let log = log_in(&tmp);

        log.decline(declined.path()).unwrap();

        assert!(log.is_declined(declined.path()));
        assert!(!log.is_declined(other.path()));
    }

    #[test]
    fn reset_makes_the_suggestion_available_again() {
        let tmp = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        let log = log_in(&tmp);
        log.decline(ws.path()).unwrap();

        log.reset(ws.path()).unwrap();

        assert!(!log.is_declined(ws.path()));
    }

    #[test]
    fn reset_of_a_directory_that_was_never_declined_is_not_an_error() {
        let tmp = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        assert!(log_in(&tmp).reset(ws.path()).is_ok());
    }

    #[test]
    fn the_log_drives_evaluate_end_to_end() {
        let tmp = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        let file = ws.path().join("main.rs");
        std::fs::write(&file, "fn main() {}").unwrap();
        let log = log_in(&tmp);

        assert!(evaluate(&file, ws.path(), |d| log.is_declined(d))
            .offered_dir()
            .is_some());

        log.decline(ws.path()).unwrap();

        assert!(matches!(
            evaluate(&file, ws.path(), |d| log.is_declined(d)),
            Decision::Declined { .. }
        ));
    }

    // ── offer: consent and outcome ───────────────────────────────────────────

    #[test]
    fn saying_yes_creates_the_repository() {
        let tmp = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        let log = log_in(&tmp);

        let outcome = offer(ws.path(), &log, |_| Some(true));

        assert!(
            matches!(outcome, OfferOutcome::Initialized(_)),
            "{outcome:?}"
        );
        assert!(vibe_core::git::is_inside_repo(ws.path()));
    }

    #[test]
    fn saying_no_creates_nothing_and_is_remembered() {
        let tmp = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        let log = log_in(&tmp);

        assert_eq!(
            offer(ws.path(), &log, |_| Some(false)),
            OfferOutcome::Declined
        );

        assert!(!ws.path().join(".git").exists(), "declining must not init");
        assert!(log.is_declined(ws.path()));
    }

    #[test]
    fn an_unanswerable_prompt_creates_nothing_and_records_nothing() {
        let tmp = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        let log = log_in(&tmp);

        // Headless: no terminal, so no answer. This must not be read as either
        // consent (init) or refusal (a stored decline that suppresses the
        // question forever once the user does get a terminal).
        assert_eq!(offer(ws.path(), &log, |_| None), OfferOutcome::NotAsked);

        assert!(!ws.path().join(".git").exists());
        assert!(!log.is_declined(ws.path()));
    }

    #[test]
    fn a_failed_init_is_reported_not_swallowed() {
        let tmp = TempDir::new().unwrap();
        let outer = TempDir::new().unwrap();
        vibe_core::git::init_repo(outer.path()).unwrap();
        let nested = outer.path().join("sub");
        std::fs::create_dir_all(&nested).unwrap();

        let outcome = offer(&nested, &log_in(&tmp), |_| Some(true));

        match outcome {
            OfferOutcome::Failed(e) => assert!(e.contains("already inside"), "{e}"),
            other => panic!("expected a reported failure, got {other:?}"),
        }
    }

    // ── the recorder ─────────────────────────────────────────────────────────

    #[test]
    fn pending_offer_collapses_many_edits_to_one_question() {
        let tmp = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        let src = ws.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let files: Vec<PathBuf> = ["a.rs", "b.rs", "c.rs"]
            .iter()
            .map(|n| {
                let p = src.join(n);
                std::fs::write(&p, "//").unwrap();
                p
            })
            .collect();

        let offered = pending_offer(&files, ws.path(), &log_in(&tmp)).unwrap();

        assert_eq!(
            offered.canonicalize().unwrap(),
            ws.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn pending_offer_is_none_when_everything_is_already_tracked() {
        let tmp = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        vibe_core::git::init_repo(ws.path()).unwrap();
        let file = ws.path().join("a.rs");
        std::fs::write(&file, "//").unwrap();

        assert!(pending_offer(&[file], ws.path(), &log_in(&tmp)).is_none());
    }

    #[test]
    fn the_recorder_is_bounded() {
        let _guard = RECORDER_LOCK.lock_recover();
        let _ = take_edited();
        let ws = TempDir::new().unwrap();

        for i in 0..(MAX_RECORDED + 50) {
            note_edited(&ws.path().join(format!("bounded_{i}.rs")));
        }
        let drained = take_edited();

        // The daemon never drains this; unbounded growth there is a leak.
        assert_eq!(drained.len(), MAX_RECORDED);
    }

    #[test]
    fn take_edited_drains_what_was_noted() {
        // The global recorder is process-wide, so this test claims it by
        // taking the lock and draining first.
        let _guard = RECORDER_LOCK.lock_recover();
        let _ = take_edited();
        let ws = TempDir::new().unwrap();
        let file = ws.path().join("recorded.rs");
        std::fs::write(&file, "//").unwrap();

        note_edited(&file);
        let first = take_edited();
        let second = take_edited();

        assert!(first.contains(&file), "expected {file:?} in {first:?}");
        assert!(second.is_empty(), "a drain must leave the set empty");
    }

    #[test]
    fn init_then_evaluate_stops_offering() {
        let ws = TempDir::new().unwrap();
        let file = ws.path().join("main.rs");
        std::fs::write(&file, "fn main() {}").unwrap();
        assert!(evaluate(&file, ws.path(), |_| false)
            .offered_dir()
            .is_some());

        vibe_core::git::init_repo(ws.path()).unwrap();

        // Accepting the offer must make the offer stop — otherwise the prompt
        // returns on the very next edit.
        assert!(matches!(
            evaluate(&file, ws.path(), |_| false),
            Decision::AlreadyTracked { .. }
        ));
    }
}
