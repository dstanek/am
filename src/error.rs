use thiserror::Error;

#[derive(Debug, Error)]
pub enum AmError {
    #[error("not in a git or jj repository")]
    NotInRepo,

    #[error("not inside a tmux session (run inside tmux or use 'tmux new-session' first)")]
    NotInTmux,

    #[error("slug '{0}' already exists — run 'am clean {0}' first")]
    SlugAlreadyExists(String),

    #[error("slug '{0}' not found — run 'am list' to see active sessions")]
    SlugNotFound(String),

    #[error("worktree error: {0}")]
    WorktreeError(String),

    #[error("tmux error: {0}")]
    TmuxError(String),

    #[error("no container runtime found — install Podman (https://podman.io/getting-started/installation) or Docker (https://docs.docker.com/get-docker/)")]
    ContainerRuntimeNotFound,

    #[error("container.image is not configured — set it in .am/config.toml or ~/.config/am/config.toml")]
    ContainerImageNotConfigured,

    #[error("container error: {0}")]
    ContainerError(String),

    #[error("config error: {0}")]
    ConfigError(String),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_in_repo_formats_correctly() {
        let e = AmError::NotInRepo;
        assert!(e.to_string().contains("git or jj repository"));
    }

    #[test]
    fn not_in_tmux_formats_correctly() {
        let e = AmError::NotInTmux;
        assert!(e.to_string().contains("tmux"));
    }

    #[test]
    fn slug_already_exists_includes_slug() {
        let e = AmError::SlugAlreadyExists("feat".to_string());
        let msg = e.to_string();
        assert!(msg.contains("feat"));
        assert!(msg.contains("am clean feat"));
    }

    #[test]
    fn slug_not_found_includes_slug() {
        let e = AmError::SlugNotFound("missing".to_string());
        let msg = e.to_string();
        assert!(msg.contains("missing"));
        assert!(msg.contains("am list"));
    }

    #[test]
    fn worktree_error_includes_message() {
        let e = AmError::WorktreeError("branch exists".to_string());
        assert!(e.to_string().contains("branch exists"));
    }

    #[test]
    fn tmux_error_includes_message() {
        let e = AmError::TmuxError("window not found".to_string());
        assert!(e.to_string().contains("window not found"));
    }

    #[test]
    fn container_runtime_not_found_mentions_podman() {
        let e = AmError::ContainerRuntimeNotFound;
        assert!(e.to_string().contains("Podman"));
    }

    #[test]
    fn container_image_not_configured_mentions_config() {
        let e = AmError::ContainerImageNotConfigured;
        assert!(e.to_string().contains("container.image"));
    }

    #[test]
    fn container_error_includes_message() {
        let e = AmError::ContainerError("exit code 1".to_string());
        assert!(e.to_string().contains("exit code 1"));
    }

    #[test]
    fn config_error_includes_message() {
        let e = AmError::ConfigError("invalid toml".to_string());
        assert!(e.to_string().contains("invalid toml"));
    }

    #[test]
    fn io_error_converts_from_std() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let e: AmError = io_err.into();
        assert!(e.to_string().contains("file missing"));
    }
}
