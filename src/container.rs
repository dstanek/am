use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::{NetworkMode, RuntimePreference, Vcs};
use crate::error::AmError;

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
    pub vcs_host: PathBuf,       // .git dir (git) or .jj dir (jj)
    pub gitconfig_host: PathBuf, // ~/.gitconfig
    pub ssh_host: PathBuf,       // ~/.ssh
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
                return Ok(ContainerRuntime { kind: RuntimeKind::Podman, bin });
            }
            if let Some(bin) = find_bin("docker", "AM_DOCKER_BIN") {
                return Ok(ContainerRuntime { kind: RuntimeKind::Docker, bin });
            }
            Err(AmError::ContainerRuntimeNotFound.into())
        }
        RuntimePreference::Podman => {
            find_bin("podman", "AM_PODMAN_BIN")
                .map(|bin| ContainerRuntime { kind: RuntimeKind::Podman, bin })
                .ok_or_else(|| AmError::RequestedContainerRuntimeNotFound("podman".to_string()).into())
        }
        RuntimePreference::Docker => {
            find_bin("docker", "AM_DOCKER_BIN")
                .map(|bin| ContainerRuntime { kind: RuntimeKind::Docker, bin })
                .ok_or_else(|| AmError::RequestedContainerRuntimeNotFound("docker".to_string()).into())
        }
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
        .map_err(|_| AmError::ConfigError("HOME env var not set".to_string()).into())
}

pub fn resolve_mounts(
    slug: &str,
    repo_root: &Path,
    vcs: &Vcs,
    agent: Option<&str>,
) -> Result<ContainerMounts> {
    let home = home_dir()?;
    let worktree_host = repo_root.join(".am").join("worktrees").join(slug);
    let vcs_host = match vcs {
        Vcs::Git => repo_root.join(".git"),
        Vcs::Jj => repo_root.join(".jj"),
    };
    let gitconfig_host = home.join(".gitconfig");
    let ssh_host = home.join(".ssh");
    let agent_auth = agent.map(resolve_agent_auth_mount).unwrap_or_default();

    Ok(ContainerMounts { worktree_host, vcs_host, gitconfig_host, ssh_host, agent_auth })
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
                    container_path: PathBuf::from("/root/.claude"),
                    mode: MountMode::ReadWrite,
                },
                AgentAuthMount {
                    host_path: home.join(".claude.json"),
                    container_path: PathBuf::from("/root/.claude.json"),
                    mode: MountMode::ReadWrite,
                },
            ]
        }
        "copilot" => vec![
            AgentAuthMount {
                // GitHub CLI auth token (required for Copilot authentication)
                host_path: home.join(".config").join("gh"),
                container_path: PathBuf::from("/root/.config/gh"),
                mode: MountMode::ReadOnly,
            },
            AgentAuthMount {
                host_path: home.join(".config").join("github-copilot"),
                container_path: PathBuf::from("/root/.config/github-copilot"),
                mode: MountMode::ReadOnly,
            },
        ],
        "gemini" => vec![AgentAuthMount {
            host_path: home.join(".gemini"),
            container_path: PathBuf::from("/root/.gemini"),
            mode: MountMode::ReadOnly,
        }],
        "codex" | "aider" => vec![], // env-var only, no filesystem mount
        unknown => {
            eprintln!(
                "warning: unknown agent preset '{unknown}' — no auth mount added. \
                 Use container.env to pass credentials manually."
            );
            vec![]
        }
    }
}

// ── Command building ──────────────────────────────────────────────────────────

pub fn build_run_command(
    runtime: &ContainerRuntime,
    image: &str,
    mounts: &ContainerMounts,
    env_passthrough: &[String],
    extra_env: &[(&str, &str)],
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

    // Worktree mount — same path inside the container as on the host
    let worktree_str = mounts.worktree_host.to_string_lossy();
    cmd.push("-v".to_string());
    cmd.push(mount_str(&mounts.worktree_host, &worktree_str, MountMode::ReadWrite, selinux));

    // VCS dir mount — same path inside the container as on the host
    let vcs_str = mounts.vcs_host.to_string_lossy();
    cmd.push("-v".to_string());
    cmd.push(mount_str(&mounts.vcs_host, &vcs_str, MountMode::ReadWrite, selinux));

    // ~/.gitconfig
    cmd.push("-v".to_string());
    cmd.push(mount_str(&mounts.gitconfig_host, "/root/.gitconfig", MountMode::ReadOnly, selinux));

    // ~/.ssh
    cmd.push("-v".to_string());
    cmd.push(mount_str(&mounts.ssh_host, "/root/.ssh", MountMode::ReadOnly, selinux));

    // Agent auth mounts
    for auth in &mounts.agent_auth {
        cmd.push("-v".to_string());
        cmd.push(mount_str(
            &auth.host_path,
            auth.container_path.to_str().unwrap_or("/root/.agent"),
            auth.mode.clone(),
            selinux,
        ));
    }

    // Extra env vars (e.g. from config)
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
    cmd.push(worktree_str.into_owned());

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
        let mounts = resolve_mounts("feat", &repo_root, &Vcs::Git, None).unwrap();

        assert_eq!(mounts.worktree_host, repo_root.join(".am/worktrees/feat"));
        assert_eq!(mounts.vcs_host, repo_root.join(".git"));
        assert_eq!(mounts.gitconfig_host, tmp.path().join(".gitconfig"));
        assert_eq!(mounts.ssh_host, tmp.path().join(".ssh"));
        assert!(mounts.agent_auth.is_empty());

        std::env::remove_var("HOME");
    }

    #[test]
    fn resolve_mounts_includes_agent_auth_for_claude() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());
        std::env::remove_var("CLAUDE_CONFIG_DIR");

        let mounts = resolve_mounts("feat", tmp.path(), &Vcs::Git, Some("claude")).unwrap();
        assert_eq!(mounts.agent_auth.len(), 2);
        assert_eq!(mounts.agent_auth[0].host_path, tmp.path().join(".claude"));
        assert_eq!(mounts.agent_auth[0].container_path, PathBuf::from("/root/.claude"));

        std::env::remove_var("HOME");
    }

    // ── build_run_command ─────────────────────────────────────────────────────

    fn podman_runtime() -> ContainerRuntime {
        ContainerRuntime { kind: RuntimeKind::Podman, bin: PathBuf::from("/usr/bin/podman") }
    }

    fn docker_runtime() -> ContainerRuntime {
        ContainerRuntime { kind: RuntimeKind::Docker, bin: PathBuf::from("/usr/bin/docker") }
    }

    #[test]
    fn build_run_command_includes_required_flags() {
        let tmp = TempDir::new().unwrap();
        let mounts = make_mounts(tmp.path());
        let worktree = tmp.path().join("worktrees/feat").to_string_lossy().into_owned();
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
        assert!(!joined.contains("GIT_WORK_TREE"), "GIT_WORK_TREE should not be set");
    }

    #[test]
    fn build_run_command_includes_all_mounts() {
        let tmp = TempDir::new().unwrap();
        let mounts = make_mounts(tmp.path());
        let worktree = tmp.path().join("worktrees/feat").to_string_lossy().into_owned();
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
        assert!(joined.contains("/root/.gitconfig"));
        assert!(joined.contains("/root/.ssh"));
    }

    #[test]
    fn build_run_command_mounts_use_host_paths() {
        let tmp = TempDir::new().unwrap();
        let mounts = make_mounts(tmp.path());
        let worktree = tmp.path().join("worktrees/feat").to_string_lossy().into_owned();
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
        assert!(joined.contains(&format!("{worktree}:{worktree}")), "worktree mount should use host path: {joined}");
        assert!(joined.contains(&format!("{git}:{git}")), "vcs mount should use host path: {joined}");
        assert!(joined.contains(&format!("--workdir {worktree}")), "workdir should be worktree path: {joined}");
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
            assert!(joined.contains(",z"), "expected ,z on Linux+Podman, got: {joined}");
        } else {
            assert!(!joined.contains(",z"), "unexpected ,z on non-Linux, got: {joined}");
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
        assert!(!joined.contains(",z"), "Docker should never have ,z: {joined}");
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
        assert_eq!(mounts[0].container_path, PathBuf::from("/root/.claude"));
        assert_eq!(mounts[0].mode, MountMode::ReadWrite);
        assert_eq!(mounts[1].host_path, tmp.path().join(".claude.json"));
        assert_eq!(mounts[1].container_path, PathBuf::from("/root/.claude.json"));
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
        assert_eq!(mounts[0].container_path, PathBuf::from("/root/.claude"));
        assert_eq!(mounts[0].mode, MountMode::ReadWrite);
        assert_eq!(mounts[1].host_path, tmp.path().join(".claude.json"));
        assert_eq!(mounts[1].container_path, PathBuf::from("/root/.claude.json"));

        std::env::remove_var("CLAUDE_CONFIG_DIR");
        std::env::remove_var("HOME");
    }

    #[test]
    fn build_run_command_includes_claude_mount_when_preset_active() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        let mut mounts = make_mounts(tmp.path());
        mounts.agent_auth = vec![AgentAuthMount {
            host_path: tmp.path().join(".claude"),
            container_path: PathBuf::from("/root/.claude"),
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
        assert!(joined.contains("/root/.claude"), "expected claude mount, got: {joined}");

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
        assert!(paths.contains(&tmp.path().join(".config").join("gh")), "missing gh config");
        assert!(paths.contains(&tmp.path().join(".config").join("github-copilot")), "missing github-copilot config");

        for m in &mounts {
            assert_eq!(m.mode, MountMode::ReadOnly, "copilot mounts should be read-only");
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
        assert!(container_paths.contains(&PathBuf::from("/root/.config/gh")));
        assert!(container_paths.contains(&PathBuf::from("/root/.config/github-copilot")));

        std::env::remove_var("HOME");
    }

    #[test]
    fn codex_and_aider_return_no_mount() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        assert!(resolve_agent_auth_mount("codex").is_empty());
        assert!(resolve_agent_auth_mount("aider").is_empty());

        std::env::remove_var("HOME");
    }
}
