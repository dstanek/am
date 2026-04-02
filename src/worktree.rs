use std::path::{Path, PathBuf};

use anyhow::Result;
use git2::{BranchType, Repository, WorktreeAddOptions};

use crate::command;
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
    let head = repo.head().map_err(|e| {
        if e.code() == git2::ErrorCode::UnbornBranch {
            AmError::WorktreeError(
                "repository has no commits yet — make an initial commit before running 'am start'"
                    .to_string(),
            )
        } else {
            AmError::WorktreeError(format!("cannot resolve HEAD: {e}"))
        }
    })?;
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

/// Resolve the `jj` binary path, respecting the `AM_JJ_BIN` env override.
fn jj_bin() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("AM_JJ_BIN") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
        return Err(AmError::WorktreeError("jj binary not found (AM_JJ_BIN is set but does not exist)".to_string()).into());
    }
    which::which("jj").map_err(|_| AmError::WorktreeError(
        "jj not found on PATH — install from https://jj-vcs.github.io/jj/".to_string()
    ).into())
}

/// Run a `jj` subcommand with the given args, returning an error on non-zero exit.
fn run_jj(args: &[&str]) -> Result<()> {
    let bin = jj_bin()?;
    let bin_str = bin.to_string_lossy();
    command::run_command(&bin_str, args, AmError::WorktreeError)
}

/// Create a jj workspace for `slug` at `<repo-root>/.am/worktrees/<slug>`.
pub fn create_jj_workspace(slug: &str, repo_root: &Path) -> Result<PathBuf> {
    let worktree_path = repo_root.join(".am").join("worktrees").join(slug);
    if let Some(parent) = worktree_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let path_str = worktree_path.to_string_lossy();
    run_jj(&["workspace", "add", &path_str, "--name", slug])?;
    Ok(worktree_path)
}

/// Remove the jj workspace for `slug` and delete the workspace directory.
pub fn remove_jj_workspace(slug: &str, repo_root: &Path) -> Result<()> {
    run_jj(&["workspace", "forget", slug])?;
    let worktree_path = repo_root.join(".am").join("worktrees").join(slug);
    if worktree_path.exists() {
        std::fs::remove_dir_all(&worktree_path)
            .map_err(|e| AmError::WorktreeError(format!("failed to remove directory: {e}")))?;
    }
    Ok(())
}

/// Returns true if the git worktree at `worktree_path` has any uncommitted changes
/// (staged, unstaged, or untracked). Returns false if the path doesn't exist or
/// any error occurs — callers use this for a best-effort warning only.
pub fn git_worktree_has_changes(worktree_path: &Path) -> bool {
    let repo = match Repository::open(worktree_path) {
        Ok(r) => r,
        Err(_) => return false,
    };
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);
    let result = match repo.statuses(Some(&mut opts)) {
        Ok(statuses) => !statuses.is_empty(),
        Err(_) => false,
    };
    result
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
    use git2::Signature;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

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

    // ── jj helpers ────────────────────────────────────────────────────────────

    /// Create a fake `jj` script that logs its args and exits 0.
    fn fake_jj(dir: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let bin = dir.join("jj");
        std::fs::write(
            &bin,
            "#!/bin/sh\necho \"$*\" >> \"$AM_JJ_LOG\"\n",
        )
        .unwrap();
        let mut perms = std::fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin, perms).unwrap();
        bin
    }

    fn read_jj_log(log: &Path) -> String {
        std::fs::read_to_string(log).unwrap_or_default()
    }

    #[test]
    fn create_jj_workspace_runs_correct_command() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = TempDir::new().unwrap();
        let bin = fake_jj(tmp.path());
        let log = tmp.path().join("jj.log");
        std::env::set_var("AM_JJ_BIN", &bin);
        std::env::set_var("AM_JJ_LOG", &log);

        let repo_root = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_root).unwrap();
        create_jj_workspace("feat", &repo_root).unwrap();

        let out = read_jj_log(&log);
        assert!(out.contains("workspace"), "expected 'workspace': {out}");
        assert!(out.contains("add"), "expected 'add': {out}");
        assert!(out.contains("feat"), "expected slug 'feat': {out}");

        std::env::remove_var("AM_JJ_BIN");
        std::env::remove_var("AM_JJ_LOG");
    }

    #[test]
    fn create_jj_workspace_returns_correct_path() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = TempDir::new().unwrap();
        let bin = fake_jj(tmp.path());
        let log = tmp.path().join("jj.log");
        std::env::set_var("AM_JJ_BIN", &bin);
        std::env::set_var("AM_JJ_LOG", &log);

        let repo_root = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_root).unwrap();
        let path = create_jj_workspace("feat", &repo_root).unwrap();

        assert_eq!(path, repo_root.join(".am").join("worktrees").join("feat"));

        std::env::remove_var("AM_JJ_BIN");
        std::env::remove_var("AM_JJ_LOG");
    }

    #[test]
    fn remove_jj_workspace_calls_forget_and_removes_directory() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = TempDir::new().unwrap();
        let bin = fake_jj(tmp.path());
        let log = tmp.path().join("jj.log");
        std::env::set_var("AM_JJ_BIN", &bin);
        std::env::set_var("AM_JJ_LOG", &log);

        let repo_root = tmp.path().join("repo");
        let worktree_path = repo_root.join(".am").join("worktrees").join("feat");
        std::fs::create_dir_all(&worktree_path).unwrap();

        remove_jj_workspace("feat", &repo_root).unwrap();

        let out = read_jj_log(&log);
        assert!(out.contains("workspace"), "expected 'workspace': {out}");
        assert!(out.contains("forget"), "expected 'forget': {out}");
        assert!(out.contains("feat"), "expected slug 'feat': {out}");
        assert!(!worktree_path.exists(), "worktree directory should be removed");

        std::env::remove_var("AM_JJ_BIN");
        std::env::remove_var("AM_JJ_LOG");
    }

    #[test]
    fn remove_jj_workspace_succeeds_when_directory_already_gone() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = TempDir::new().unwrap();
        let bin = fake_jj(tmp.path());
        let log = tmp.path().join("jj.log");
        std::env::set_var("AM_JJ_BIN", &bin);
        std::env::set_var("AM_JJ_LOG", &log);

        let repo_root = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_root).unwrap();
        // Directory does not exist — should not error
        remove_jj_workspace("feat", &repo_root).unwrap();

        std::env::remove_var("AM_JJ_BIN");
        std::env::remove_var("AM_JJ_LOG");
    }

    #[test]
    fn create_git_worktree_errors_on_unborn_branch() {
        let tmp = TempDir::new().unwrap();
        // Init repo but make NO initial commit — HEAD is unborn
        Repository::init(tmp.path()).unwrap();

        let err = create_git_worktree("feat", tmp.path()).unwrap_err();
        assert!(
            err.to_string().contains("no commits yet"),
            "expected helpful unborn-branch message, got: {err}"
        );
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
