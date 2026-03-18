use std::path::{Path, PathBuf};

use anyhow::Result;
use git2::{BranchType, Repository, Signature, WorktreeAddOptions};

use crate::config::Vcs;
use crate::error::AmError;

/// Detect which VCS the directory at `repo_root` uses.
pub fn detect_vcs(repo_root: &Path) -> Result<Vcs> {
    if repo_root.join(".jj").exists() {
        Ok(Vcs::Jj)
    } else if repo_root.join(".git").exists() {
        Ok(Vcs::Git)
    } else {
        Err(AmError::NotInRepo.into())
    }
}

/// Create a git worktree for `slug` at `<repo-root>/.am/worktrees/<slug>`.
/// Creates branch `am/<slug>` off HEAD. Errors with `SlugAlreadyExists` if
/// the branch already exists.
pub fn create_git_worktree(slug: &str, repo_root: &Path) -> Result<PathBuf> {
    let repo = Repository::open(repo_root)
        .map_err(|e| AmError::WorktreeError(e.to_string()))?;

    let branch_name = format!("am/{slug}");

    // Fail fast if branch already exists
    if repo.find_branch(&branch_name, BranchType::Local).is_ok() {
        return Err(AmError::SlugAlreadyExists(slug.to_string()).into());
    }

    // Resolve HEAD to a commit
    let head = repo
        .head()
        .map_err(|e| AmError::WorktreeError(format!("cannot resolve HEAD: {e}")))?;
    let commit = head
        .peel_to_commit()
        .map_err(|e| AmError::WorktreeError(format!("HEAD is not a commit: {e}")))?;

    // Create the branch at HEAD
    let branch = repo
        .branch(&branch_name, &commit, false)
        .map_err(|e| AmError::WorktreeError(format!("failed to create branch {branch_name}: {e}")))?;
    let branch_ref = branch.into_reference();

    // Ensure parent directory exists
    let worktree_path = repo_root.join(".am").join("worktrees").join(slug);
    if let Some(parent) = worktree_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Add the worktree pointing at the new branch
    let mut opts = WorktreeAddOptions::new();
    opts.reference(Some(&branch_ref));
    repo.worktree(slug, &worktree_path, Some(&opts))
        .map_err(|e| AmError::WorktreeError(format!("failed to add worktree: {e}")))?;

    Ok(worktree_path)
}

/// Remove the git worktree for `slug` and delete the `am/<slug>` branch.
pub fn remove_git_worktree(slug: &str, repo_root: &Path) -> Result<()> {
    let repo = Repository::open(repo_root)
        .map_err(|e| AmError::WorktreeError(e.to_string()))?;

    // Remove the directory FIRST — once it's gone git treats the worktree as
    // invalid, which lets prune succeed without special flags.
    let worktree_path = repo_root.join(".am").join("worktrees").join(slug);
    if worktree_path.exists() {
        std::fs::remove_dir_all(&worktree_path)
            .map_err(|e| AmError::WorktreeError(format!("failed to remove directory: {e}")))?;
    }

    // Prune the worktree registration from .git/worktrees/
    if let Ok(wt) = repo.find_worktree(slug) {
        let mut prune_opts = git2::WorktreePruneOptions::new();
        prune_opts.working_tree(true);
        let _ = wt.prune(Some(&mut prune_opts));
    }

    // Delete the branch via its ref directly — `branch.delete()` can refuse if
    // git still considers the branch checked out somewhere.
    let ref_name = format!("refs/heads/am/{slug}");
    if let Ok(mut reference) = repo.find_reference(&ref_name) {
        reference
            .delete()
            .map_err(|e| AmError::WorktreeError(format!("failed to delete branch am/{slug}: {e}")))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Init a repo and make an initial commit so HEAD exists.
    fn init_repo_with_commit(dir: &Path) {
        let repo = Repository::init(dir).unwrap();
        let sig = Signature::now("test", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
                .unwrap();
        }
    }

    #[test]
    fn detect_vcs_returns_git_for_git_repo() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        assert_eq!(detect_vcs(tmp.path()).unwrap(), Vcs::Git);
    }

    #[test]
    fn detect_vcs_returns_jj_for_jj_repo() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".jj")).unwrap();
        assert_eq!(detect_vcs(tmp.path()).unwrap(), Vcs::Jj);
    }

    #[test]
    fn detect_vcs_prefers_jj_when_both_present() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        std::fs::create_dir(tmp.path().join(".jj")).unwrap();
        assert_eq!(detect_vcs(tmp.path()).unwrap(), Vcs::Jj);
    }

    #[test]
    fn detect_vcs_errors_when_no_repo() {
        let tmp = TempDir::new().unwrap();
        let err = detect_vcs(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("repository"));
    }

    #[test]
    fn create_git_worktree_creates_branch_and_directory() {
        let tmp = TempDir::new().unwrap();
        init_repo_with_commit(tmp.path());

        let worktree_path = create_git_worktree("feat", tmp.path()).unwrap();

        assert!(worktree_path.exists(), "worktree directory should exist");
        assert_eq!(
            worktree_path,
            tmp.path().join(".am").join("worktrees").join("feat")
        );

        // Branch should exist
        let repo = Repository::open(tmp.path()).unwrap();
        assert!(repo.find_branch("am/feat", BranchType::Local).is_ok());
    }

    #[test]
    fn create_git_worktree_duplicate_slug_errors() {
        let tmp = TempDir::new().unwrap();
        init_repo_with_commit(tmp.path());

        create_git_worktree("feat", tmp.path()).unwrap();
        let err = create_git_worktree("feat", tmp.path()).unwrap_err();
        assert!(err.to_string().contains("feat"));
    }

    #[test]
    fn remove_git_worktree_removes_directory_and_branch() {
        let tmp = TempDir::new().unwrap();
        init_repo_with_commit(tmp.path());

        let worktree_path = create_git_worktree("feat", tmp.path()).unwrap();
        assert!(worktree_path.exists());

        remove_git_worktree("feat", tmp.path()).unwrap();

        assert!(!worktree_path.exists(), "worktree directory should be gone");

        let repo = Repository::open(tmp.path()).unwrap();
        assert!(
            repo.find_branch("am/feat", BranchType::Local).is_err(),
            "branch should be deleted"
        );
    }
}
