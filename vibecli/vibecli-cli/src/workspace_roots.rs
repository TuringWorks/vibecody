//! Multi-root workspace permission resolver.
//!
//! Phase 53 P0 (A6 from v13 fitgap): extend `--add-dir` from read-only
//! to read+write. The agent loop accepts a `workspace_root: <path>`
//! field on each tool call; this module is the data layer that decides
//! whether a given absolute path is in-scope for the active workspace
//! roots, which root it belongs to, and what permission tier the root
//! carries.
//!
//! Path normalisation collapses `.` and `..` without touching the
//! filesystem — defensive against `add_dir + ../../etc/passwd` probes.

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRootPermission {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRoot {
    pub path: PathBuf,
    pub permission: WorkspaceRootPermission,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceRoots {
    roots: Vec<WorkspaceRoot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    OutOfScope(PathBuf),
    ReadOnly(PathBuf),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::OutOfScope(p) => {
                write!(f, "path is outside every workspace root: {}", p.display())
            }
            ResolveError::ReadOnly(p) => write!(f, "workspace root is read-only: {}", p.display()),
        }
    }
}

impl std::error::Error for ResolveError {}

impl WorkspaceRoots {
    pub fn new(roots: Vec<WorkspaceRoot>) -> Self {
        // Sort by path length descending so longest-prefix wins for
        // nested roots.
        let mut roots = roots;
        roots.sort_by(|a, b| b.path.as_os_str().len().cmp(&a.path.as_os_str().len()));
        Self { roots }
    }
    pub fn len(&self) -> usize {
        self.roots.len()
    }
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }
    pub fn all(&self) -> &[WorkspaceRoot] {
        &self.roots
    }
    pub fn resolve(&self, path: &Path) -> Result<&WorkspaceRoot, ResolveError> {
        let normalised = normalise(path);
        for root in &self.roots {
            let root_n = normalise(&root.path);
            if normalised.starts_with(&root_n) {
                return Ok(root);
            }
        }
        Err(ResolveError::OutOfScope(path.to_path_buf()))
    }
    pub fn check_write(&self, path: &Path) -> Result<(), ResolveError> {
        let root = self.resolve(path)?;
        match root.permission {
            WorkspaceRootPermission::ReadWrite => Ok(()),
            WorkspaceRootPermission::ReadOnly => Err(ResolveError::ReadOnly(root.path.clone())),
        }
    }
    pub fn check_read(&self, path: &Path) -> Result<(), ResolveError> {
        self.resolve(path).map(|_| ())
    }
}

fn normalise(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rw(path: &str) -> WorkspaceRoot {
        WorkspaceRoot {
            path: PathBuf::from(path),
            permission: WorkspaceRootPermission::ReadWrite,
        }
    }
    fn ro(path: &str) -> WorkspaceRoot {
        WorkspaceRoot {
            path: PathBuf::from(path),
            permission: WorkspaceRootPermission::ReadOnly,
        }
    }

    #[test]
    fn resolves_path_to_owning_root_in_two_root_config() {
        let roots = WorkspaceRoots::new(vec![rw("/proj/a"), rw("/proj/b")]);
        assert_eq!(
            roots
                .resolve(Path::new("/proj/a/src/main.rs"))
                .unwrap()
                .path,
            PathBuf::from("/proj/a")
        );
        assert_eq!(
            roots.resolve(Path::new("/proj/b/lib.rs")).unwrap().path,
            PathBuf::from("/proj/b")
        );
    }

    #[test]
    fn longest_prefix_wins_for_nested_roots() {
        let roots = WorkspaceRoots::new(vec![rw("/proj"), ro("/proj/a")]);
        let r = roots.resolve(Path::new("/proj/a/x.rs")).unwrap();
        assert_eq!(r.path, PathBuf::from("/proj/a"));
        assert_eq!(r.permission, WorkspaceRootPermission::ReadOnly);
    }

    #[test]
    fn out_of_scope_path_in_three_root_config_errors() {
        let roots = WorkspaceRoots::new(vec![rw("/proj/a"), rw("/proj/b"), ro("/vendored/libc")]);
        let err = roots.resolve(Path::new("/etc/passwd")).unwrap_err();
        assert_eq!(err, ResolveError::OutOfScope(PathBuf::from("/etc/passwd")));
    }

    #[test]
    fn check_write_rejects_read_only_roots_in_three_root_config() {
        let roots = WorkspaceRoots::new(vec![rw("/proj/a"), rw("/proj/b"), ro("/vendored/libc")]);
        roots.check_write(Path::new("/proj/a/src/main.rs")).unwrap();
        roots.check_write(Path::new("/proj/b/lib.rs")).unwrap();
        let err = roots
            .check_write(Path::new("/vendored/libc/string.c"))
            .unwrap_err();
        assert_eq!(err, ResolveError::ReadOnly(PathBuf::from("/vendored/libc")));
        roots
            .check_read(Path::new("/vendored/libc/string.c"))
            .unwrap();
    }

    #[test]
    fn parent_dir_traversal_is_normalised_and_cannot_escape_root() {
        let roots = WorkspaceRoots::new(vec![rw("/proj/a")]);
        let err = roots
            .resolve(Path::new("/proj/a/sub/../../../etc/passwd"))
            .unwrap_err();
        assert!(matches!(err, ResolveError::OutOfScope(_)), "got {err:?}");
    }

    #[test]
    fn parent_dir_within_root_resolves_to_same_root() {
        let roots = WorkspaceRoots::new(vec![rw("/proj/a")]);
        let r = roots.resolve(Path::new("/proj/a/sub/../main.rs")).unwrap();
        assert_eq!(r.path, PathBuf::from("/proj/a"));
    }
}
