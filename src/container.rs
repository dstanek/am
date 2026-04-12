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

#[derive(Debug, Clone)]
pub struct AgentAuthMount {
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    pub mode: MountMode,
}

#[derive(Debug, Clone)]
pub struct ContainerMounts {
    pub worktree_host: PathBuf,
    pub vcs_host: PathBuf,                    // .git dir (git) or .jj dir (jj)
    pub colocated_git_host: Option<PathBuf>,  // .git for colocated jj+git repos
    pub gitconfig_host: PathBuf,              // .am/gitconfig (or override)
    pub ssh_host: PathBuf,                    // ~/.ssh
    pub agent_auth: Vec<AgentAuthMount>,
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
    std::env::var("HOME")
        .map(PathBuf::from)
        .with_context(|| "HOME environment variable not set — cannot resolve user home directory for mounts")?
        .canonicalize()
        .with_context(|| "failed to resolve HOME path — does the directory exist and is it accessible?")
}

pub fn resolve_mounts(
    slug: &str,
    repo_root: &Path,
    vcs: &Vcs,
    agent: Option<&str>,
    gitconfig: Option<&Path>,
    ssh: Option<&Path>,
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
    let agent_auth = agent.map(resolve_agent_auth_mount).unwrap_or_default();

    Ok(ContainerMounts {
        worktree_host,
        vcs_host,
        colocated_git_host,
        gitconfig_host,
        ssh_host,
        agent_auth,
    })
}

pub fn resolve_agent_auth_mount(agent: &str) -> Vec<AgentAuthMount> {
    let home = match home_dir() {
        Ok(h) => h,
        Err(_) => return vec![],
    };
    match agent {
        "claude" => {
            // Config dir: use CLAUDE_CONFIG_DIR if set, otherwise ~/.claude
            let config_host = std::env::var("CLAUDE_CONFIG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join(".claude"));
            vec![
                AgentAuthMount {
                    host_path: config_host,
                    container_path: PathBuf::from("/home/am/.claude"),
                    mode: MountMode::ReadWrite,
                },
                AgentAuthMount {
                    host_path: home.join(".claude.json"),
                    container_path: PathBuf::from("/home/am/.claude.json"),
                    mode: MountMode::ReadWrite,
                },
            ]
        }
        "copilot" => vec![
            AgentAuthMount {
                // GitHub CLI auth token (required for Copilot authentication)
                host_path: home.join(".config").join("gh"),
                container_path: PathBuf::from("/home/am/.config/gh"),
                mode: MountMode::ReadOnly,
            },
            AgentAuthMount {
                host_path: home.join(".config").join("github-copilot"),
                container_path: PathBuf::from("/home/am/.config/github-copilot"),
                mode: MountMode::ReadOnly,
            },
        ],
        "gemini" => vec![AgentAuthMount {
            host_path: home.join(".gemini"),
            container_path: PathBuf::from("/home/am/.gemini"),
            mode: MountMode::ReadOnly,
        }],
        "codex" => vec![], // env-var only, no filesystem mount
        _unknown => vec![],          // treat as a raw launch command — no auth mount
    }
}

/// Returns the extra CLI flags needed to run an agent in autonomous mode.
/// Unknown agents get no flags — they must be configured by the user.
pub fn agent_auto_flags(agent: &str) -> Vec<String> {
    match agent {
        "claude" => vec!["--dangerously-skip-permissions".to_string()],
        _ => vec![],
    }
}

/// Runs `gh auth token` and returns the token string.
fn get_gh_token() -> Result<String> {
    let output = std::process::Command::new("gh")
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

/// Returns extra environment variables to inject into the container for the given agent.
pub fn agent_extra_env(agent: &str) -> Result<Vec<(String, String)>> {
    match agent {
        "copilot" => {
            let token = get_gh_token()?;
            Ok(vec![("GH_TOKEN".to_string(), token)])
        }
        _ => Ok(vec![]),
    }
}

/// Validate that a known agent has its required credential directories
/// present on the host. Unknown values are treated as raw commands and always pass.
/// Call this early in `am start` before any side effects.
pub fn validate_agent(agent: &str) -> Result<()> {
    let home = home_dir()?;
    let config_dir = std::env::var("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join(".claude"));

    let (required, label) = match agent {
        "claude" => (vec![config_dir], "claude"),
        "copilot" => (vec![home.join(".config").join("gh")], "copilot"),
        "gemini" => (vec![home.join(".gemini")], "gemini"),
        "codex" => return Ok(()), // env-var only, no filesystem check
        unknown => {
            return Err(AmError::ConfigError(format!(
                "unknown agent '{unknown}' — valid agents are: claude, copilot, gemini, codex",
            ))
            .into())
        }
    };

    for path in &required {
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "agent '{label}' requires directory to exist: {path}\n\
                 Make sure {label} is installed and authenticated on this system",
                path = path.display()
            ))
            .with_context(|| format!(
                "checking agent credentials for '{label}' at {}",
                path.display()
            ));
        }
    }
    Ok(())
}

// ── Command building ──────────────────────────────────────────────────────────

fn get_host_uid_gid() -> Option<(u32, u32)> {
    let uid = std::process::Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u32>().ok())?;
    let gid = std::process::Command::new("id")
        .arg("-g")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u32>().ok())?;
    Some((uid, gid))
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
        cmd.push(mount_str(git, &git.to_string_lossy(), MountMode::ReadWrite, selinux));
    }

    // ~/.gitconfig — only mount if the file exists
    if mounts.gitconfig_host.exists() {
        cmd.push("-v".to_string());
        cmd.push(mount_str(
            &mounts.gitconfig_host,
            "/home/am/.gitconfig",
            MountMode::ReadOnly,
            selinux,
        ));
    }

    // ~/.ssh — only mount if the directory exists
    if mounts.ssh_host.exists() {
        cmd.push("-v".to_string());
        cmd.push(mount_str(
            &mounts.ssh_host,
            "/home/am/.ssh",
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
        let flags = agent_auto_flags("claude");
        assert_eq!(flags, vec!["--dangerously-skip-permissions"]);
    }

    #[test]
    fn agent_auto_flags_unknown_agent_returns_empty() {
        assert!(agent_auto_flags("codex").is_empty());
        assert!(agent_auto_flags("my-custom-agent").is_empty());
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

    fn make_mounts(tmp: &Path) -> ContainerMounts {
        ContainerMounts {
            worktree_host: tmp.join("worktrees/feat"),
            vcs_host: tmp.join(".git"),
            colocated_git_host: None,
            gitconfig_host: tmp.join(".gitconfig"),
            ssh_host: tmp.join(".ssh"),
            agent_auth: vec![],
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
        let mounts = resolve_mounts("feat", &repo_root, &Vcs::Git, None, None, None).unwrap();

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
        let mounts = resolve_mounts("feat", &repo_root, &Vcs::Jj, None, None, None).unwrap();

        assert_eq!(mounts.colocated_git_host, Some(repo_root.join(".git")));

        std::env::remove_var("HOME");
    }

    #[test]
    fn resolve_mounts_jj_non_colocated_no_git_host() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        let repo_root = tmp.path().join("repo");
        let mounts = resolve_mounts("feat", &repo_root, &Vcs::Jj, None, None, None).unwrap();

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
    fn resolve_mounts_includes_agent_auth_for_claude() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());
        std::env::remove_var("CLAUDE_CONFIG_DIR");

        let mounts = resolve_mounts("feat", tmp.path(), &Vcs::Git, Some("claude"), None, None).unwrap();
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

    // ── Feature 4: Claude auth mount ──────────────────────────────────────────

    #[test]
    fn resolve_agent_auth_mount_claude_defaults_to_dot_claude() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());
        std::env::remove_var("CLAUDE_CONFIG_DIR");

        let mounts = resolve_agent_auth_mount("claude");
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
    fn resolve_agent_auth_mount_claude_uses_claude_config_dir_when_set() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        let custom_config = tmp.path().join("custom-claude-config");
        std::env::set_var("HOME", tmp.path());
        std::env::set_var("CLAUDE_CONFIG_DIR", &custom_config);

        let mounts = resolve_agent_auth_mount("claude");
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

    // ── validate_agent ─────────────────────────────────────────────────

    #[test]
    fn validate_agent_claude_ok_when_dir_exists() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        std::fs::create_dir(tmp.path().join(".claude")).unwrap();

        assert!(validate_agent("claude").is_ok());

        std::env::remove_var("HOME");
    }

    #[test]
    fn validate_agent_claude_fails_when_dir_missing() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());
        std::env::remove_var("CLAUDE_CONFIG_DIR");

        assert!(validate_agent("claude").is_err());

        std::env::remove_var("HOME");
    }

    #[test]
    fn validate_agent_copilot_ok_when_gh_dir_exists() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());
        std::fs::create_dir_all(tmp.path().join(".config").join("gh")).unwrap();

        assert!(validate_agent("copilot").is_ok());

        std::env::remove_var("HOME");
    }

    #[test]
    fn validate_agent_copilot_fails_when_gh_dir_missing() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        assert!(validate_agent("copilot").is_err());

        std::env::remove_var("HOME");
    }

    #[test]
    fn validate_agent_gemini_ok_when_dir_exists() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());
        std::fs::create_dir(tmp.path().join(".gemini")).unwrap();

        assert!(validate_agent("gemini").is_ok());

        std::env::remove_var("HOME");
    }

    #[test]
    fn validate_agent_gemini_fails_when_dir_missing() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        assert!(validate_agent("gemini").is_err());

        std::env::remove_var("HOME");
    }

    #[test]
    fn validate_agent_codex_always_ok() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        assert!(validate_agent("codex").is_ok());

        std::env::remove_var("HOME");
    }

    #[test]
    fn validate_agent_unknown_errors() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        let err = validate_agent("my-custom-agent").unwrap_err();
        assert!(err.to_string().contains("my-custom-agent"));

        std::env::remove_var("HOME");
    }

    // ── Feature 6: Copilot auth mount ─────────────────────────────────────────

    #[test]
    fn resolve_agent_auth_mount_copilot_returns_both_dirs() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        let mounts = resolve_agent_auth_mount("copilot");
        assert_eq!(mounts.len(), 2);

        let paths: Vec<_> = mounts.iter().map(|m| m.host_path.clone()).collect();
        assert!(
            paths.contains(&tmp.path().join(".config").join("gh")),
            "missing gh config"
        );
        assert!(
            paths.contains(&tmp.path().join(".config").join("github-copilot")),
            "missing github-copilot config"
        );

        for m in &mounts {
            assert_eq!(
                m.mode,
                MountMode::ReadOnly,
                "copilot mounts should be read-only"
            );
        }

        std::env::remove_var("HOME");
    }

    #[test]
    fn resolve_agent_auth_mount_copilot_container_paths_match() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        let mounts = resolve_agent_auth_mount("copilot");
        let container_paths: Vec<_> = mounts.iter().map(|m| m.container_path.clone()).collect();
        assert!(container_paths.contains(&PathBuf::from("/home/am/.config/gh")));
        assert!(container_paths.contains(&PathBuf::from("/home/am/.config/github-copilot")));

        std::env::remove_var("HOME");
    }

    #[test]
    fn codex_returns_no_mount() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        assert!(resolve_agent_auth_mount("codex").is_empty());

        std::env::remove_var("HOME");
    }
}
