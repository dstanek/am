use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::{NetworkMode, RuntimePreference, Vcs};
use crate::error::AmError;

// Path handling strategy (preserve type safety as long as possible):
// - Keep as Path/PathBuf in internal code
// - Use &Path in function parameters (not &str)
// - Convert to String only at boundaries (Command args, logging, display)
// - Prefer .display() for format strings (never panics, handles UTF-8)
// - Use .to_string_lossy() only when String ownership is needed
// - Use .to_str()? only for critical UTF-8 requirements with error handling

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeKind {
    Podman,
    Docker,
}

impl std::fmt::Display for RuntimeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeKind::Podman => write!(f, "podman"),
            RuntimeKind::Docker => write!(f, "docker"),
        }
    }
}

/// A known agent preset. Adding a new variant here causes exhaustive-match
/// errors in all agent-specific functions below, enforcing that every site
/// is kept in sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownAgent {
    Claude,
    Copilot,
    Gemini,
    Codex,
}

impl KnownAgent {
    /// Parse a string into a `KnownAgent`, returning a descriptive error for
    /// unknown names. This replaces the old `validate_agent_name` function.
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s {
            "claude" => Ok(KnownAgent::Claude),
            "copilot" => Ok(KnownAgent::Copilot),
            "gemini" => Ok(KnownAgent::Gemini),
            "codex" => Ok(KnownAgent::Codex),
            unknown => Err(AmError::ConfigError(format!(
                "unknown agent '{unknown}' — valid agents are: claude, copilot, gemini, codex",
            ))
            .into()),
        }
    }
}

impl std::fmt::Display for KnownAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnownAgent::Claude => write!(f, "claude"),
            KnownAgent::Copilot => write!(f, "copilot"),
            KnownAgent::Gemini => write!(f, "gemini"),
            KnownAgent::Codex => write!(f, "codex"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContainerRuntime {
    pub kind: RuntimeKind,
    pub bin: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MountMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAuthMount {
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    pub mode: MountMode,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgentAuth {
    pub mounts: Vec<AgentAuthMount>,
    pub env: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct ContainerMounts {
    pub worktree_host: PathBuf,
    pub vcs_host: PathBuf,                   // .git dir (git) or .jj dir (jj)
    pub colocated_git_host: Option<PathBuf>, // .git for colocated jj+git repos
    pub gitconfig_host: PathBuf,             // .am/gitconfig (or override)
    pub ssh_host: PathBuf,                   // ~/.ssh
    pub agent_auth: Vec<AgentAuthMount>,
    pub container_user: String, // username inside the container
}

// ── Runtime detection ─────────────────────────────────────────────────────────

fn find_bin(name: &str, env_override: &str) -> Option<PathBuf> {
    // If the env var is set, use it exclusively — don't fall back to which.
    // This lets tests inject a nonexistent path to simulate "not found".
    if let Ok(path) = std::env::var(env_override) {
        let p = PathBuf::from(path);
        return if p.exists() { Some(p) } else { None };
    }
    which::which(name).ok()
}

pub fn detect_runtime(preference: RuntimePreference) -> Result<ContainerRuntime> {
    match preference {
        RuntimePreference::Auto => {
            if let Some(bin) = find_bin("podman", "AM_PODMAN_BIN") {
                return Ok(ContainerRuntime {
                    kind: RuntimeKind::Podman,
                    bin,
                });
            }
            if let Some(bin) = find_bin("docker", "AM_DOCKER_BIN") {
                return Ok(ContainerRuntime {
                    kind: RuntimeKind::Docker,
                    bin,
                });
            }
            Err(AmError::ContainerRuntimeNotFound.into())
        }
        RuntimePreference::Podman => find_bin("podman", "AM_PODMAN_BIN")
            .map(|bin| ContainerRuntime {
                kind: RuntimeKind::Podman,
                bin,
            })
            .ok_or_else(|| AmError::RequestedContainerRuntimeNotFound("podman".to_string()).into()),
        RuntimePreference::Docker => find_bin("docker", "AM_DOCKER_BIN")
            .map(|bin| ContainerRuntime {
                kind: RuntimeKind::Docker,
                bin,
            })
            .ok_or_else(|| AmError::RequestedContainerRuntimeNotFound("docker".to_string()).into()),
    }
}

// ── SELinux label ─────────────────────────────────────────────────────────────

fn use_selinux_labels(runtime: &ContainerRuntime) -> bool {
    cfg!(target_os = "linux") && runtime.kind == RuntimeKind::Podman
}

fn mount_str(host: &Path, container: &str, mode: MountMode, selinux: bool) -> String {
    let mode_str = match mode {
        MountMode::ReadOnly => "ro",
        MountMode::ReadWrite => "rw",
    };
    if selinux {
        format!("{}:{}:{},z", host.display(), container, mode_str)
    } else {
        format!("{}:{}:{}", host.display(), container, mode_str)
    }
}

// ── Mount resolution ──────────────────────────────────────────────────────────

fn home_dir() -> Result<PathBuf> {
    std::env::var("HOME").map(PathBuf::from).with_context(|| {
        "HOME environment variable not set — cannot resolve user home directory for mounts"
    })
}

pub fn resolve_mounts(
    slug: &str,
    repo_root: &Path,
    vcs: &Vcs,
    agent_auth: Vec<AgentAuthMount>,
    gitconfig: Option<&Path>,
    ssh: Option<&Path>,
    container_user: &str,
) -> Result<ContainerMounts> {
    let home = home_dir()?;
    let worktree_host = repo_root.join(".am").join("worktrees").join(slug);
    let vcs_host = match vcs {
        Vcs::Git => repo_root.join(".git"),
        Vcs::Jj => repo_root.join(".jj"),
    };
    // For colocated jj+git repos, .git holds the git object store used as the
    // jj backend and must be mounted alongside .jj.
    let colocated_git_host = if matches!(vcs, Vcs::Jj) {
        let git = repo_root.join(".git");
        if git.is_dir() {
            Some(git)
        } else {
            None
        }
    } else {
        None
    };
    let gitconfig_host = gitconfig
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| repo_root.join(".am").join("gitconfig"));
    let ssh_host = ssh
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| home.join(".ssh"));

    Ok(ContainerMounts {
        worktree_host,
        vcs_host,
        colocated_git_host,
        gitconfig_host,
        ssh_host,
        agent_auth,
        container_user: container_user.to_string(),
    })
}

fn resolve_agent_auth_mounts(
    agent: KnownAgent,
    container_user: &str,
) -> Result<Vec<AgentAuthMount>> {
    Ok(match agent {
        KnownAgent::Claude => {
            let home = home_dir()?;
            let home_in_container = format!("/home/{container_user}");
            // Config dir: use CLAUDE_CONFIG_DIR if set, otherwise ~/.claude
            let config_host = std::env::var("CLAUDE_CONFIG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join(".claude"));
            vec![
                AgentAuthMount {
                    host_path: config_host,
                    container_path: PathBuf::from(format!("{home_in_container}/.claude")),
                    mode: MountMode::ReadWrite,
                },
                AgentAuthMount {
                    host_path: home.join(".claude.json"),
                    container_path: PathBuf::from(format!("{home_in_container}/.claude.json")),
                    mode: MountMode::ReadWrite,
                },
            ]
        }
        KnownAgent::Copilot => {
            let home = home_dir()?;
            let home_in_container = format!("/home/{container_user}");
            vec![
                AgentAuthMount {
                    // GitHub CLI auth token (required for Copilot authentication)
                    host_path: home.join(".config").join("gh"),
                    container_path: PathBuf::from(format!("{home_in_container}/.config/gh")),
                    mode: MountMode::ReadOnly,
                },
                AgentAuthMount {
                    host_path: home.join(".config").join("github-copilot"),
                    container_path: PathBuf::from(format!(
                        "{home_in_container}/.config/github-copilot"
                    )),
                    mode: MountMode::ReadOnly,
                },
            ]
        }
        KnownAgent::Gemini => {
            let home = home_dir()?;
            let home_in_container = format!("/home/{container_user}");
            vec![AgentAuthMount {
                host_path: home.join(".gemini"),
                container_path: PathBuf::from(format!("{home_in_container}/.gemini")),
                mode: MountMode::ReadOnly,
            }]
        }
        KnownAgent::Codex => vec![], // env-var only, no filesystem mount
    })
}

/// Returns the extra CLI flags needed to run an agent in autonomous mode.
pub fn agent_auto_flags(agent: KnownAgent) -> Vec<String> {
    match agent {
        KnownAgent::Claude => vec!["--dangerously-skip-permissions".to_string()],
        KnownAgent::Copilot | KnownAgent::Gemini | KnownAgent::Codex => vec![],
    }
}

/// Runs `gh auth token` and returns the token string.
fn get_gh_token() -> Result<String> {
    let gh = find_bin("gh", "AM_GH_BIN").ok_or_else(|| {
        anyhow::anyhow!("failed to execute 'gh auth token' — is GitHub CLI installed?")
    })?;
    let output = std::process::Command::new(gh)
        .args(["auth", "token"])
        .output()
        .with_context(|| "failed to execute 'gh auth token' — is GitHub CLI installed?")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(anyhow::anyhow!(
            "GitHub CLI authentication failed — run 'gh auth login' to authenticate\n\
             Error: {stderr}"
        ))
        .with_context(|| "retrieving GitHub authentication token for Copilot");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn required_env_var(agent: KnownAgent, key: &str, example: &str) -> Result<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "agent '{agent}' requires {key} to be set in the environment\n\
                 Export it before running: export {key}={example}"
            )
        })
}

fn ensure_required_paths(agent: KnownAgent, required: &[PathBuf]) -> Result<()> {
    for path in required {
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "agent '{agent}' requires path to exist: {path}\n\
                 Make sure {agent} is installed and authenticated on this system",
                path = path.display()
            ))
            .with_context(|| {
                format!(
                    "checking agent credentials for '{agent}' at {}",
                    path.display()
                )
            });
        }
    }
    Ok(())
}

fn resolve_agent_auth(agent: KnownAgent, container_user: &str) -> Result<AgentAuth> {
    match agent {
        KnownAgent::Claude => {
            let mounts = resolve_agent_auth_mounts(agent, container_user)?;
            let required = mounts
                .first()
                .map(|mount| vec![mount.host_path.clone()])
                .unwrap_or_default();
            ensure_required_paths(agent, &required)?;
            Ok(AgentAuth {
                mounts,
                env: vec![],
            })
        }
        KnownAgent::Copilot => {
            let mounts = resolve_agent_auth_mounts(agent, container_user)?;
            let required = mounts
                .iter()
                .find(|mount| mount.host_path.ends_with(Path::new(".config/gh")))
                .map(|mount| vec![mount.host_path.clone()])
                .unwrap_or_default();
            ensure_required_paths(agent, &required)?;
            let token = get_gh_token()?;
            Ok(AgentAuth {
                mounts,
                env: vec![("GH_TOKEN".to_string(), token)],
            })
        }
        KnownAgent::Gemini => {
            let mounts = resolve_agent_auth_mounts(agent, container_user)?;
            let required = mounts
                .first()
                .map(|mount| vec![mount.host_path.clone()])
                .unwrap_or_default();
            ensure_required_paths(agent, &required)?;
            Ok(AgentAuth {
                mounts,
                env: vec![],
            })
        }
        KnownAgent::Codex => Ok(AgentAuth {
            mounts: vec![],
            env: vec![(
                "OPENAI_API_KEY".to_string(),
                required_env_var(agent, "OPENAI_API_KEY", "sk-...")?,
            )],
        }),
    }
}

/// Resolve and validate a known agent's authentication requirements before the
/// container is launched. This performs all preflight checks and returns the
/// mounts and environment variables needed for the actual runtime command.
pub fn preflight_agent_auth(agent: KnownAgent, container_user: &str) -> Result<AgentAuth> {
    resolve_agent_auth(agent, container_user)
}

// ── Command building ──────────────────────────────────────────────────────────

#[cfg(unix)]
fn get_host_uid_gid() -> Option<(u32, u32)> {
    extern "C" {
        fn getuid() -> u32;
        fn getgid() -> u32;
    }
    // SAFETY: getuid/getgid have no preconditions and are always safe to call.
    Some(unsafe { (getuid(), getgid()) })
}

#[cfg(not(unix))]
fn get_host_uid_gid() -> Option<(u32, u32)> {
    None
}

pub fn build_run_command(
    runtime: &ContainerRuntime,
    image: &str,
    mounts: &ContainerMounts,
    env_passthrough: &[String],
    extra_env: &[(String, String)],
    network: &NetworkMode,
    container_name: &str,
) -> Vec<String> {
    let container_user = &mounts.container_user;
    let selinux = use_selinux_labels(runtime);
    let mut cmd = vec![
        runtime.bin.to_string_lossy().into_owned(),
        "run".to_string(),
        "--rm".to_string(),
        "-it".to_string(),
        "--name".to_string(),
        container_name.to_string(),
    ];

    // Run as the host user so bind-mounted files are readable/writable
    if let Some((uid, gid)) = get_host_uid_gid() {
        match runtime.kind {
            RuntimeKind::Podman => {
                cmd.push(format!("--userns=keep-id:uid={uid},gid={gid}"));
            }
            RuntimeKind::Docker => {
                cmd.push("--user".to_string());
                cmd.push(format!("{uid}:{gid}"));
            }
        }
    }

    // Worktree mount — same path inside the container as on the host
    cmd.push("-v".to_string());
    cmd.push(mount_str(
        &mounts.worktree_host,
        &mounts.worktree_host.to_string_lossy(),
        MountMode::ReadWrite,
        selinux,
    ));

    // VCS dir mount — same path inside the container as on the host
    cmd.push("-v".to_string());
    cmd.push(mount_str(
        &mounts.vcs_host,
        &mounts.vcs_host.to_string_lossy(),
        MountMode::ReadWrite,
        selinux,
    ));

    // Colocated jj+git: mount the git object store alongside .jj
    if let Some(ref git) = mounts.colocated_git_host {
        cmd.push("-v".to_string());
        cmd.push(mount_str(
            git,
            &git.to_string_lossy(),
            MountMode::ReadWrite,
            selinux,
        ));
    }

    // ~/.gitconfig — only mount if the file exists
    if mounts.gitconfig_host.exists() {
        cmd.push("-v".to_string());
        cmd.push(mount_str(
            &mounts.gitconfig_host,
            &format!("/home/{container_user}/.gitconfig"),
            MountMode::ReadOnly,
            selinux,
        ));
    }

    // ~/.ssh — only mount if the directory exists
    if mounts.ssh_host.exists() {
        cmd.push("-v".to_string());
        cmd.push(mount_str(
            &mounts.ssh_host,
            &format!("/home/{container_user}/.ssh"),
            MountMode::ReadOnly,
            selinux,
        ));
    }

    // Agent auth mounts — only mount if the path exists
    for auth in &mounts.agent_auth {
        if auth.host_path.exists() {
            cmd.push("-v".to_string());
            cmd.push(mount_str(
                &auth.host_path,
                &auth.container_path.to_string_lossy(),
                auth.mode.clone(),
                selinux,
            ));
        }
    }

    // Extra env vars (e.g. agent-specific tokens)
    for (key, val) in extra_env {
        cmd.push("-e".to_string());
        cmd.push(format!("{key}={val}"));
    }

    // Host env pass-through
    for var in env_passthrough {
        cmd.push("-e".to_string());
        cmd.push(var.clone());
    }

    // Network
    if matches!(network, NetworkMode::None) {
        cmd.push("--network".to_string());
        cmd.push("none".to_string());
    }

    // Working directory — same as worktree host path
    cmd.push("--workdir".to_string());
    cmd.push(mounts.worktree_host.to_string_lossy().into_owned());

    cmd.push(image.to_string());
    cmd
}

// ── Container lifecycle ───────────────────────────────────────────────────────

fn run_container_cmd(runtime: &ContainerRuntime, args: &[&str]) -> Result<()> {
    let output = std::process::Command::new(&runtime.bin)
        .args(args)
        .output()
        .map_err(|e| AmError::ContainerError(format!("failed to run container command: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AmError::ContainerError(if stderr.is_empty() {
            format!("container command exited with status {}", output.status)
        } else {
            stderr
        })
        .into());
    }
    Ok(())
}

pub fn stop_container(runtime: &ContainerRuntime, container_name: &str) -> Result<()> {
    // Ignore error — container may already be stopped
    let _ = run_container_cmd(runtime, &["stop", container_name]);
    Ok(())
}

pub fn remove_container(runtime: &ContainerRuntime, container_name: &str) -> Result<()> {
    run_container_cmd(runtime, &["rm", "-f", container_name])
}

/// Pre-emptively remove a container with this name (e.g. from a crashed
/// previous session), logging a warning if one existed.
/// NOTE: `podman/docker rm --force` exits 0 even when the container doesn't
/// exist, so we check existence first to avoid false-positive warnings.
pub fn remove_if_exists(runtime: &ContainerRuntime, container_name: &str) {
    let exists = std::process::Command::new(&runtime.bin)
        .args(["container", "inspect", container_name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if exists {
        let _ = run_container_cmd(runtime, &["rm", "-f", container_name]);
        eprintln!(
            "warning: removed existing container '{container_name}' from a previous unclean run"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn agent_auto_flags_claude_returns_skip_permissions() {
        let flags = agent_auto_flags(KnownAgent::Claude);
        assert_eq!(flags, vec!["--dangerously-skip-permissions"]);
    }

    #[test]
    fn agent_auto_flags_non_claude_agents_return_empty() {
        assert!(agent_auto_flags(KnownAgent::Codex).is_empty());
        assert!(agent_auto_flags(KnownAgent::Copilot).is_empty());
        assert!(agent_auto_flags(KnownAgent::Gemini).is_empty());
    }

    fn fake_runtime(kind: RuntimeKind, dir: &Path) -> ContainerRuntime {
        // Create a script that records its args and exits 0
        let bin = dir.join("mock_runtime");
        std::fs::write(&bin, "#!/bin/sh\necho \"$*\" >> \"$MOCK_CONTAINER_LOG\"\n").unwrap();
        let mut perms = std::fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin, perms).unwrap();
        ContainerRuntime { kind, bin }
    }

    fn fake_bin(dir: &Path, name: &str) -> PathBuf {
        let bin = dir.join(name);
        std::fs::write(&bin, "#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = std::fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin, perms).unwrap();
        bin
    }

    fn fake_gh(dir: &Path, body: &str) -> PathBuf {
        let bin = dir.join("gh");
        std::fs::write(&bin, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut perms = std::fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin, perms).unwrap();
        bin
    }

    fn make_mounts(tmp: &Path) -> ContainerMounts {
        ContainerMounts {
            worktree_host: tmp.join("worktrees/feat"),
            vcs_host: tmp.join(".git"),
            colocated_git_host: None,
            gitconfig_host: tmp.join(".gitconfig"),
            ssh_host: tmp.join(".ssh"),
            agent_auth: vec![],
            container_user: "am".to_string(),
        }
    }

    // ── detect_runtime ────────────────────────────────────────────────────────

    #[test]
    fn detect_runtime_auto_finds_podman() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        let podman = fake_bin(tmp.path(), "podman");
        std::env::set_var("AM_PODMAN_BIN", &podman);
        std::env::remove_var("AM_DOCKER_BIN");

        let rt = detect_runtime(RuntimePreference::Auto).unwrap();
        assert_eq!(rt.kind, RuntimeKind::Podman);
        assert_eq!(rt.bin, podman);

        std::env::remove_var("AM_PODMAN_BIN");
    }

    #[test]
    fn detect_runtime_auto_falls_back_to_docker() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        let docker = fake_bin(tmp.path(), "docker");
        std::env::set_var("AM_PODMAN_BIN", "/nonexistent/podman");
        std::env::set_var("AM_DOCKER_BIN", &docker);

        let rt = detect_runtime(RuntimePreference::Auto).unwrap();
        assert_eq!(rt.kind, RuntimeKind::Docker);

        std::env::remove_var("AM_PODMAN_BIN");
        std::env::remove_var("AM_DOCKER_BIN");
    }

    #[test]
    fn detect_runtime_auto_errors_when_neither_found() {
        let _g = lock_env();
        std::env::set_var("AM_PODMAN_BIN", "/nonexistent/podman");
        std::env::set_var("AM_DOCKER_BIN", "/nonexistent/docker");

        let err = detect_runtime(RuntimePreference::Auto).unwrap_err();
        assert!(err.to_string().contains("Podman"));

        std::env::remove_var("AM_PODMAN_BIN");
        std::env::remove_var("AM_DOCKER_BIN");
    }

    #[test]
    fn detect_runtime_explicit_podman_errors_when_not_found() {
        let _g = lock_env();
        std::env::set_var("AM_PODMAN_BIN", "/nonexistent/podman");

        let err = detect_runtime(RuntimePreference::Podman).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("podman"));

        std::env::remove_var("AM_PODMAN_BIN");
    }

    #[test]
    fn detect_runtime_explicit_docker_errors_when_not_found() {
        let _g = lock_env();
        std::env::set_var("AM_DOCKER_BIN", "/nonexistent/docker");

        let err = detect_runtime(RuntimePreference::Docker).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("docker"));

        std::env::remove_var("AM_DOCKER_BIN");
    }

    #[test]
    fn detect_runtime_explicit_docker_finds_docker() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        let docker = fake_bin(tmp.path(), "docker");
        std::env::set_var("AM_DOCKER_BIN", &docker);

        let rt = detect_runtime(RuntimePreference::Docker).unwrap();
        assert_eq!(rt.kind, RuntimeKind::Docker);
        assert_eq!(rt.bin, docker);

        std::env::remove_var("AM_DOCKER_BIN");
    }

    // ── resolve_mounts ────────────────────────────────────────────────────────

    #[test]
    fn resolve_mounts_git_paths() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        let repo_root = tmp.path().join("repo");
        let mounts =
            resolve_mounts("feat", &repo_root, &Vcs::Git, vec![], None, None, "am").unwrap();

        assert_eq!(mounts.worktree_host, repo_root.join(".am/worktrees/feat"));
        assert_eq!(mounts.vcs_host, repo_root.join(".git"));
        assert_eq!(mounts.gitconfig_host, repo_root.join(".am/gitconfig"));
        assert_eq!(mounts.ssh_host, tmp.path().join(".ssh"));
        assert!(mounts.agent_auth.is_empty());

        std::env::remove_var("HOME");
    }

    #[test]
    fn resolve_mounts_jj_colocated_sets_git_host() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        let repo_root = tmp.path().join("repo");
        std::fs::create_dir_all(repo_root.join(".git")).unwrap();
        let mounts =
            resolve_mounts("feat", &repo_root, &Vcs::Jj, vec![], None, None, "am").unwrap();

        assert_eq!(mounts.colocated_git_host, Some(repo_root.join(".git")));

        std::env::remove_var("HOME");
    }

    #[test]
    fn resolve_mounts_jj_non_colocated_no_git_host() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        let repo_root = tmp.path().join("repo");
        let mounts =
            resolve_mounts("feat", &repo_root, &Vcs::Jj, vec![], None, None, "am").unwrap();

        assert_eq!(mounts.colocated_git_host, None);

        std::env::remove_var("HOME");
    }

    #[test]
    fn build_run_command_mounts_colocated_git_when_set() {
        let tmp = TempDir::new().unwrap();
        let main_git = tmp.path().join("main/.git");
        std::fs::create_dir_all(&main_git).unwrap();
        let mut mounts = make_mounts(tmp.path());
        mounts.colocated_git_host = Some(main_git.clone());

        let cmd = build_run_command(
            &docker_runtime(),
            "ubuntu:25.10",
            &mounts,
            &[],
            &[],
            &NetworkMode::Full,
            "am-feat",
        );
        let joined = cmd.join(" ");
        assert!(
            joined.contains(main_git.to_string_lossy().as_ref()),
            "expected colocated git mount, got: {joined}"
        );
    }

    #[test]
    fn resolve_mounts_includes_preflighted_agent_auth_for_claude() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        std::fs::create_dir(tmp.path().join(".claude")).unwrap();

        let agent_auth = preflight_agent_auth(KnownAgent::Claude, "am").unwrap();
        let mounts = resolve_mounts(
            "feat",
            tmp.path(),
            &Vcs::Git,
            agent_auth.mounts,
            None,
            None,
            "am",
        )
        .unwrap();
        assert_eq!(mounts.agent_auth.len(), 2);
        assert_eq!(mounts.agent_auth[0].host_path, tmp.path().join(".claude"));
        assert_eq!(
            mounts.agent_auth[0].container_path,
            PathBuf::from("/home/am/.claude")
        );

        std::env::remove_var("HOME");
    }

    // ── build_run_command ─────────────────────────────────────────────────────

    fn podman_runtime() -> ContainerRuntime {
        ContainerRuntime {
            kind: RuntimeKind::Podman,
            bin: PathBuf::from("/usr/bin/podman"),
        }
    }

    fn docker_runtime() -> ContainerRuntime {
        ContainerRuntime {
            kind: RuntimeKind::Docker,
            bin: PathBuf::from("/usr/bin/docker"),
        }
    }

    #[test]
    fn build_run_command_includes_required_flags() {
        let tmp = TempDir::new().unwrap();
        let mounts = make_mounts(tmp.path());
        let worktree = tmp
            .path()
            .join("worktrees/feat")
            .to_string_lossy()
            .into_owned();
        let cmd = build_run_command(
            &podman_runtime(),
            "ubuntu:25.10",
            &mounts,
            &[],
            &[],
            &NetworkMode::Full,
            "am-feat",
        );

        let joined = cmd.join(" ");
        assert!(joined.contains("run"), "missing 'run'");
        assert!(joined.contains("--rm"));
        assert!(joined.contains("-it"));
        assert!(joined.contains("--name am-feat"));
        assert!(joined.contains(&worktree));
        assert!(joined.contains(&format!("--workdir {worktree}")));
        assert!(joined.contains("ubuntu:25.10"));
        assert!(!joined.contains("GIT_DIR"), "GIT_DIR should not be set");
        assert!(
            !joined.contains("GIT_WORK_TREE"),
            "GIT_WORK_TREE should not be set"
        );
    }

    #[test]
    fn build_run_command_includes_all_mounts() {
        let tmp = TempDir::new().unwrap();
        // Create the paths so the existence checks pass
        std::fs::write(tmp.path().join(".gitconfig"), "").unwrap();
        std::fs::create_dir_all(tmp.path().join(".ssh")).unwrap();
        let mounts = make_mounts(tmp.path());
        let worktree = tmp
            .path()
            .join("worktrees/feat")
            .to_string_lossy()
            .into_owned();
        let git = tmp.path().join(".git").to_string_lossy().into_owned();
        let cmd = build_run_command(
            &docker_runtime(),
            "ubuntu:25.10",
            &mounts,
            &[],
            &[],
            &NetworkMode::Full,
            "am-feat",
        );
        let joined = cmd.join(" ");
        assert!(joined.contains(&worktree), "missing worktree mount");
        assert!(joined.contains(&git), "missing vcs mount");
        assert!(joined.contains("/home/am/.gitconfig"));
        assert!(joined.contains("/home/am/.ssh"));
    }

    #[test]
    fn build_run_command_mounts_use_host_paths() {
        let tmp = TempDir::new().unwrap();
        let mounts = make_mounts(tmp.path());
        let worktree = tmp
            .path()
            .join("worktrees/feat")
            .to_string_lossy()
            .into_owned();
        let git = tmp.path().join(".git").to_string_lossy().into_owned();
        let cmd = build_run_command(
            &podman_runtime(),
            "ubuntu:25.10",
            &mounts,
            &[],
            &[],
            &NetworkMode::Full,
            "am-feat",
        );
        let joined = cmd.join(" ");
        // Container path should equal host path for worktree and vcs
        assert!(
            joined.contains(&format!("{worktree}:{worktree}")),
            "worktree mount should use host path: {joined}"
        );
        assert!(
            joined.contains(&format!("{git}:{git}")),
            "vcs mount should use host path: {joined}"
        );
        assert!(
            joined.contains(&format!("--workdir {worktree}")),
            "workdir should be worktree path: {joined}"
        );
    }

    #[test]
    fn build_run_command_selinux_z_on_linux_podman() {
        let tmp = TempDir::new().unwrap();
        let mounts = make_mounts(tmp.path());
        let cmd = build_run_command(
            &podman_runtime(),
            "ubuntu:25.10",
            &mounts,
            &[],
            &[],
            &NetworkMode::Full,
            "am-feat",
        );
        let joined = cmd.join(" ");
        // On Linux with Podman, all mounts should have ,z
        // On macOS they should not — test what the current platform does
        if cfg!(target_os = "linux") {
            assert!(
                joined.contains(",z"),
                "expected ,z on Linux+Podman, got: {joined}"
            );
        } else {
            assert!(
                !joined.contains(",z"),
                "unexpected ,z on non-Linux, got: {joined}"
            );
        }
    }

    #[test]
    fn build_run_command_no_selinux_z_for_docker() {
        let tmp = TempDir::new().unwrap();
        let mounts = make_mounts(tmp.path());
        let cmd = build_run_command(
            &docker_runtime(),
            "ubuntu:25.10",
            &mounts,
            &[],
            &[],
            &NetworkMode::Full,
            "am-feat",
        );
        let joined = cmd.join(" ");
        assert!(
            !joined.contains(",z"),
            "Docker should never have ,z: {joined}"
        );
    }

    #[test]
    fn build_run_command_network_none() {
        let tmp = TempDir::new().unwrap();
        let mounts = make_mounts(tmp.path());
        let cmd = build_run_command(
            &podman_runtime(),
            "ubuntu:25.10",
            &mounts,
            &[],
            &[],
            &NetworkMode::None,
            "am-feat",
        );
        assert!(cmd.contains(&"--network".to_string()));
        assert!(cmd.contains(&"none".to_string()));
    }

    #[test]
    fn build_run_command_env_passthrough() {
        let tmp = TempDir::new().unwrap();
        let mounts = make_mounts(tmp.path());
        let cmd = build_run_command(
            &podman_runtime(),
            "ubuntu:25.10",
            &mounts,
            &["ANTHROPIC_API_KEY".to_string()],
            &[],
            &NetworkMode::Full,
            "am-feat",
        );
        let joined = cmd.join(" ");
        assert!(joined.contains("ANTHROPIC_API_KEY"));
    }

    // ── stop / remove ─────────────────────────────────────────────────────────

    #[test]
    fn stop_container_sends_stop_command() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        let log = tmp.path().join("log");
        std::env::set_var("MOCK_CONTAINER_LOG", &log);
        let rt = fake_runtime(RuntimeKind::Podman, tmp.path());

        stop_container(&rt, "am-feat").unwrap();

        let out = std::fs::read_to_string(&log).unwrap_or_default();
        assert!(out.contains("stop"), "expected 'stop', got: {out}");
        assert!(out.contains("am-feat"));

        std::env::remove_var("MOCK_CONTAINER_LOG");
    }

    #[test]
    fn remove_container_sends_rm_command() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        let log = tmp.path().join("log");
        std::env::set_var("MOCK_CONTAINER_LOG", &log);
        let rt = fake_runtime(RuntimeKind::Podman, tmp.path());

        remove_container(&rt, "am-feat").unwrap();

        let out = std::fs::read_to_string(&log).unwrap_or_default();
        assert!(out.contains("rm"), "expected 'rm', got: {out}");
        assert!(out.contains("-f"));
        assert!(out.contains("am-feat"));

        std::env::remove_var("MOCK_CONTAINER_LOG");
    }

    // ── Feature 4: Claude auth resolution ─────────────────────────────────────

    #[test]
    fn resolve_agent_auth_claude_defaults_to_dot_claude() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());
        std::env::remove_var("CLAUDE_CONFIG_DIR");

        let mounts = resolve_agent_auth_mounts(KnownAgent::Claude, "am").unwrap();
        assert_eq!(mounts.len(), 2);
        assert_eq!(mounts[0].host_path, tmp.path().join(".claude"));
        assert_eq!(mounts[0].container_path, PathBuf::from("/home/am/.claude"));
        assert_eq!(mounts[0].mode, MountMode::ReadWrite);
        assert_eq!(mounts[1].host_path, tmp.path().join(".claude.json"));
        assert_eq!(
            mounts[1].container_path,
            PathBuf::from("/home/am/.claude.json")
        );
        assert_eq!(mounts[1].mode, MountMode::ReadWrite);

        std::env::remove_var("HOME");
    }

    #[test]
    fn resolve_agent_auth_claude_uses_claude_config_dir_when_set() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        let custom_config = tmp.path().join("custom-claude-config");
        std::env::set_var("HOME", tmp.path());
        std::env::set_var("CLAUDE_CONFIG_DIR", &custom_config);

        let mounts = resolve_agent_auth_mounts(KnownAgent::Claude, "am").unwrap();
        assert_eq!(mounts.len(), 2);
        assert_eq!(mounts[0].host_path, custom_config);
        assert_eq!(mounts[0].container_path, PathBuf::from("/home/am/.claude"));
        assert_eq!(mounts[0].mode, MountMode::ReadWrite);
        assert_eq!(mounts[1].host_path, tmp.path().join(".claude.json"));
        assert_eq!(
            mounts[1].container_path,
            PathBuf::from("/home/am/.claude.json")
        );

        std::env::remove_var("CLAUDE_CONFIG_DIR");
        std::env::remove_var("HOME");
    }

    #[test]
    fn build_run_command_includes_claude_mount_when_active() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        // Create the claude config dir so the existence check passes
        std::fs::create_dir_all(tmp.path().join(".claude")).unwrap();

        let mut mounts = make_mounts(tmp.path());
        mounts.agent_auth = vec![AgentAuthMount {
            host_path: tmp.path().join(".claude"),
            container_path: PathBuf::from("/home/am/.claude"),
            mode: MountMode::ReadWrite,
        }];

        let cmd = build_run_command(
            &podman_runtime(),
            "ubuntu:25.10",
            &mounts,
            &[],
            &[],
            &NetworkMode::Full,
            "am-feat",
        );
        let joined = cmd.join(" ");
        assert!(
            joined.contains("/home/am/.claude"),
            "expected claude mount, got: {joined}"
        );

        std::env::remove_var("HOME");
    }

    // ── KnownAgent::parse ─────────────────────────────────────────────

    #[test]
    fn known_agent_parse_known_agents_ok() {
        assert!(KnownAgent::parse("claude").is_ok());
        assert!(KnownAgent::parse("copilot").is_ok());
        assert!(KnownAgent::parse("gemini").is_ok());
        assert!(KnownAgent::parse("codex").is_ok());
    }

    #[test]
    fn known_agent_parse_unknown_errors() {
        let err = KnownAgent::parse("my-custom-agent").unwrap_err();
        assert!(err.to_string().contains("my-custom-agent"));
    }

    #[test]
    fn known_agent_display_matches_parse_input() {
        for agent in [
            KnownAgent::Claude,
            KnownAgent::Copilot,
            KnownAgent::Gemini,
            KnownAgent::Codex,
        ] {
            let s = agent.to_string();
            assert_eq!(KnownAgent::parse(&s).unwrap(), agent);
        }
    }

    // ── preflight_agent_auth ───────────────────────────────────────────

    #[test]
    fn preflight_agent_auth_claude_ok_when_dir_exists() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        std::fs::create_dir(tmp.path().join(".claude")).unwrap();

        assert!(preflight_agent_auth(KnownAgent::Claude, "am").is_ok());

        std::env::remove_var("HOME");
    }

    #[test]
    fn preflight_agent_auth_claude_fails_when_dir_missing() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());
        std::env::remove_var("CLAUDE_CONFIG_DIR");

        assert!(preflight_agent_auth(KnownAgent::Claude, "am").is_err());

        std::env::remove_var("HOME");
    }

    #[test]
    fn preflight_agent_auth_copilot_returns_mounts_and_env() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());
        let gh = fake_gh(tmp.path(), "echo gh-test-token");
        std::env::set_var("AM_GH_BIN", &gh);
        std::fs::create_dir_all(tmp.path().join(".config").join("gh")).unwrap();
        std::fs::create_dir_all(tmp.path().join(".config").join("github-copilot")).unwrap();

        let auth = preflight_agent_auth(KnownAgent::Copilot, "am").unwrap();
        assert_eq!(
            auth.env,
            vec![("GH_TOKEN".to_string(), "gh-test-token".to_string())]
        );
        assert_eq!(auth.mounts.len(), 2);

        std::env::remove_var("AM_GH_BIN");
        std::env::remove_var("HOME");
    }

    #[test]
    fn preflight_agent_auth_copilot_fails_when_gh_dir_missing() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        assert!(preflight_agent_auth(KnownAgent::Copilot, "am").is_err());

        std::env::remove_var("HOME");
    }

    #[test]
    fn preflight_agent_auth_gemini_ok_when_dir_exists() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());
        std::fs::create_dir(tmp.path().join(".gemini")).unwrap();

        assert!(preflight_agent_auth(KnownAgent::Gemini, "am").is_ok());

        std::env::remove_var("HOME");
    }

    #[test]
    fn preflight_agent_auth_gemini_fails_when_dir_missing() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        assert!(preflight_agent_auth(KnownAgent::Gemini, "am").is_err());

        std::env::remove_var("HOME");
    }

    #[test]
    fn preflight_agent_auth_codex_ok_when_key_set() {
        let _g = lock_env();
        std::env::set_var("OPENAI_API_KEY", "sk-test");

        let auth = preflight_agent_auth(KnownAgent::Codex, "am").unwrap();
        assert_eq!(
            auth.env,
            vec![("OPENAI_API_KEY".to_string(), "sk-test".to_string())]
        );
        assert!(auth.mounts.is_empty());

        std::env::remove_var("OPENAI_API_KEY");
    }

    #[test]
    fn preflight_agent_auth_codex_fails_when_key_missing() {
        let _g = lock_env();
        std::env::remove_var("OPENAI_API_KEY");

        let err = preflight_agent_auth(KnownAgent::Codex, "am").unwrap_err();
        assert!(err.to_string().contains("OPENAI_API_KEY"));
    }

    #[test]
    fn preflight_agent_auth_codex_fails_when_key_empty() {
        let _g = lock_env();
        std::env::set_var("OPENAI_API_KEY", "");

        let err = preflight_agent_auth(KnownAgent::Codex, "am").unwrap_err();
        assert!(err.to_string().contains("OPENAI_API_KEY"));

        std::env::remove_var("OPENAI_API_KEY");
    }

    #[test]
    fn build_run_command_includes_codex_api_key_env() {
        let tmp = TempDir::new().unwrap();
        let mounts = make_mounts(tmp.path());
        let cmd = build_run_command(
            &podman_runtime(),
            "ubuntu:25.10",
            &mounts,
            &[],
            &[("OPENAI_API_KEY".to_string(), "sk-test-key".to_string())],
            &NetworkMode::Full,
            "am-feat",
        );
        let joined = cmd.join(" ");
        assert!(joined.contains("-e OPENAI_API_KEY=sk-test-key"));
    }

    // ── Feature 6: agent auth resolution ──────────────────────────────────────

    #[test]
    fn resolve_agent_auth_copilot_returns_both_dirs() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        let auth_mounts = resolve_agent_auth_mounts(KnownAgent::Copilot, "am").unwrap();
        assert_eq!(auth_mounts.len(), 2);

        let paths: Vec<_> = auth_mounts.iter().map(|m| m.host_path.clone()).collect();
        assert!(
            paths.contains(&tmp.path().join(".config").join("gh")),
            "missing gh config"
        );
        assert!(
            paths.contains(&tmp.path().join(".config").join("github-copilot")),
            "missing github-copilot config"
        );

        for m in &auth_mounts {
            assert_eq!(
                m.mode,
                MountMode::ReadOnly,
                "copilot mounts should be read-only"
            );
        }

        std::env::remove_var("HOME");
    }

    #[test]
    fn resolve_agent_auth_copilot_container_paths_match() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        let auth_mounts = resolve_agent_auth_mounts(KnownAgent::Copilot, "am").unwrap();
        let container_paths: Vec<_> = auth_mounts
            .iter()
            .map(|m| m.container_path.clone())
            .collect();
        assert!(container_paths.contains(&PathBuf::from("/home/am/.config/gh")));
        assert!(container_paths.contains(&PathBuf::from("/home/am/.config/github-copilot")));

        std::env::remove_var("HOME");
    }

    #[test]
    fn resolve_agent_auth_gemini_returns_dot_gemini() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        let auth_mounts = resolve_agent_auth_mounts(KnownAgent::Gemini, "am").unwrap();
        assert_eq!(auth_mounts.len(), 1);
        assert_eq!(auth_mounts[0].host_path, tmp.path().join(".gemini"));
        assert_eq!(
            auth_mounts[0].container_path,
            PathBuf::from("/home/am/.gemini")
        );
        assert_eq!(auth_mounts[0].mode, MountMode::ReadOnly);

        std::env::remove_var("HOME");
    }

    #[test]
    fn codex_returns_no_mount() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        assert!(resolve_agent_auth_mounts(KnownAgent::Codex, "am")
            .unwrap()
            .is_empty());

        std::env::remove_var("HOME");
    }
}
