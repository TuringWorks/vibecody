//! Git integration

use anyhow::Result;
use git2::Repository;
use std::path::Path;

pub struct GitOps;

impl GitOps {
    pub fn get_status(repo_path: &Path) -> Result<Vec<(String, String)>> {
        let repo = Repository::open(repo_path)?;
        let statuses = repo.statuses(None)?;
        
        let mut result = Vec::new();
        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                let status = match entry.status() {
                    s if s.is_wt_new() => "New",
                    s if s.is_wt_modified() => "Modified",
                    s if s.is_wt_deleted() => "Deleted",
                    _ => "Unknown",
                };
                result.push((path.to_string(), status.to_string()));
            }
        }
        
        Ok(result)
    }

    pub fn get_current_branch(repo_path: &Path) -> Result<String> {
        let repo = Repository::open(repo_path)?;
        let head = repo.head()?;
        
        Ok(head.shorthand().unwrap_or("HEAD").to_string())
    }

    pub fn get_diff(repo_path: &Path) -> Result<String> {
        let repo = Repository::open(repo_path)?;
        let mut diff_opts = git2::DiffOptions::new();
        
        // Get diff of index (staged) vs HEAD
        let head = repo.head()?;
        let tree = head.peel_to_tree()?;
        let diff_cached = repo.diff_tree_to_index(Some(&tree), Some(&repo.index()?), Some(&mut diff_opts))?;
        
        // Get diff of working directory (unstaged) vs index
        let diff_uncached = repo.diff_index_to_workdir(None, Some(&mut diff_opts))?;
        
        let mut diff_string = String::new();
        
        // Format cached diff
        diff_cached.print(git2::DiffFormat::Patch, |_, _, line| {
            let content = std::str::from_utf8(line.content()).unwrap_or("");
            let prefix = match line.origin() {
                '+' => "+",
                '-' => "-",
                ' ' => " ",
                _ => "",
            };
            diff_string.push_str(&format!("{}{}", prefix, content));
            true
        })?;
        
        // Format uncached diff
        diff_uncached.print(git2::DiffFormat::Patch, |_, _, line| {
            let content = std::str::from_utf8(line.content()).unwrap_or("");
            let prefix = match line.origin() {
                '+' => "+",
                '-' => "-",
                ' ' => " ",
                _ => "",
            };
            diff_string.push_str(&format!("{}{}", prefix, content));
            true
        })?;
        
        Ok(diff_string)
    }

    pub fn is_git_repo(path: &Path) -> bool {
        Repository::open(path).is_ok()
    }
}
