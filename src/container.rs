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
}

#[derive(Debug, Clone)]
pub struct ContainerMounts {
    pub worktree_host: PathBuf,
    pub vcs_host: PathBuf,       // .git dir (git) or .jj dir (jj)
    pub gitconfig_host: PathBuf, // ~/.gitconfig
    pub ssh_host: PathBuf,       // ~/.ssh
    pub agent_auth: Option<AgentAuthMount>,
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
                .ok_or_else(|| AmError::ContainerRuntimeNotFound.into())
        }
        RuntimePreference::Docker => {
            find_bin("docker", "AM_DOCKER_BIN")
                .map(|bin| ContainerRuntime { kind: RuntimeKind::Docker, bin })
                .ok_or_else(|| AmError::ContainerRuntimeNotFound.into())
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
    agent_preset: Option<&str>,
) -> Result<ContainerMounts> {
    let home = home_dir()?;
    let worktree_host = repo_root.join(".am").join("worktrees").join(slug);
    let vcs_host = match vcs {
        Vcs::Git => repo_root.join(".git"),
        Vcs::Jj => repo_root.join(".jj"),
    };
    let gitconfig_host = home.join(".gitconfig");
    let ssh_host = home.join(".ssh");
    let agent_auth = agent_preset.and_then(|p| resolve_agent_auth_mount(p));

    Ok(ContainerMounts { worktree_host, vcs_host, gitconfig_host, ssh_host, agent_auth })
}

pub fn resolve_agent_auth_mount(agent_preset: &str) -> Option<AgentAuthMount> {
    let home = home_dir().ok()?;
    match agent_preset {
        "claude" => Some(AgentAuthMount {
            host_path: home.join(".claude"),
            container_path: PathBuf::from("/root/.claude"),
        }),
        "copilot" => Some(AgentAuthMount {
            host_path: home.join(".config").join("github-copilot"),
            container_path: PathBuf::from("/root/.config/github-copilot"),
        }),
        "gemini" => Some(AgentAuthMount {
            host_path: home.join(".gemini"),
            container_path: PathBuf::from("/root/.gemini"),
        }),
        "codex" | "aider" => None, // env-var only, no filesystem mount
        unknown => {
            eprintln!(
                "warning: unknown agent preset '{unknown}' — no auth mount added. \
                 Use container.env to pass credentials manually."
            );
            None
        }
    }
}

// ── Command building ──────────────────────────────────────────────────────────

pub fn build_run_command(
    runtime: &ContainerRuntime,
    image: &str,
    slug: &str,
    mounts: &ContainerMounts,
    env_passthrough: &[String],
    extra_env: &[(&str, &str)],
    network: &NetworkMode,
    working_dir: &str,
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

    // Worktree mount
    cmd.push("-v".to_string());
    cmd.push(mount_str(&mounts.worktree_host, "/workspace", MountMode::ReadWrite, selinux));

    // VCS dir mount (.git or .jj)
    cmd.push("-v".to_string());
    cmd.push(mount_str(&mounts.vcs_host, "/mainrepo/.git", MountMode::ReadWrite, selinux));

    // ~/.gitconfig
    cmd.push("-v".to_string());
    cmd.push(mount_str(&mounts.gitconfig_host, "/root/.gitconfig", MountMode::ReadOnly, selinux));

    // ~/.ssh
    cmd.push("-v".to_string());
    cmd.push(mount_str(&mounts.ssh_host, "/root/.ssh", MountMode::ReadOnly, selinux));

    // Agent auth mount (if any)
    if let Some(auth) = &mounts.agent_auth {
        cmd.push("-v".to_string());
        cmd.push(mount_str(
            &auth.host_path,
            auth.container_path.to_str().unwrap_or("/root/.agent"),
            MountMode::ReadOnly,
            selinux,
        ));
    }

    // GIT_DIR and GIT_WORK_TREE for git repos
    cmd.push("-e".to_string());
    cmd.push(format!("GIT_DIR=/mainrepo/.git/worktrees/{slug}"));
    cmd.push("-e".to_string());
    cmd.push("GIT_WORK_TREE=/workspace".to_string());

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

    // Working directory
    cmd.push("--workdir".to_string());
    cmd.push(working_dir.to_string());

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
pub fn remove_if_exists(runtime: &ContainerRuntime, container_name: &str) {
    if run_container_cmd(runtime, &["rm", "-f", container_name]).is_ok() {
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
            agent_auth: None,
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
        assert!(mounts.agent_auth.is_none());

        std::env::remove_var("HOME");
    }

    #[test]
    fn resolve_mounts_includes_agent_auth_for_claude() {
        let _g = lock_env();
        let tmp = TempDir::new().unwrap();
        std::env::set_var("HOME", tmp.path());

        let mounts = resolve_mounts("feat", tmp.path(), &Vcs::Git, Some("claude")).unwrap();
        let auth = mounts.agent_auth.unwrap();
        assert_eq!(auth.host_path, tmp.path().join(".claude"));
        assert_eq!(auth.container_path, PathBuf::from("/root/.claude"));

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
        let cmd = build_run_command(
            &podman_runtime(),
            "ubuntu:25.10",
            "feat",
            &mounts,
            &[],
            &[],
            &NetworkMode::Full,
            "/workspace",
            "am-feat",
        );

        let joined = cmd.join(" ");
        assert!(joined.contains("run"), "missing 'run'");
        assert!(joined.contains("--rm"));
        assert!(joined.contains("-it"));
        assert!(joined.contains("--name am-feat"));
        assert!(joined.contains("/workspace"));
        assert!(joined.contains("--workdir /workspace"));
        assert!(joined.contains("ubuntu:25.10"));
        assert!(joined.contains("GIT_DIR=/mainrepo/.git/worktrees/feat"));
        assert!(joined.contains("GIT_WORK_TREE=/workspace"));
    }

    #[test]
    fn build_run_command_includes_all_mounts() {
        let tmp = TempDir::new().unwrap();
        let mounts = make_mounts(tmp.path());
        let cmd = build_run_command(
            &docker_runtime(),
            "ubuntu:25.10",
            "feat",
            &mounts,
            &[],
            &[],
            &NetworkMode::Full,
            "/workspace",
            "am-feat",
        );
        let joined = cmd.join(" ");
        assert!(joined.contains("/workspace"));
        assert!(joined.contains("/mainrepo/.git"));
        assert!(joined.contains("/root/.gitconfig"));
        assert!(joined.contains("/root/.ssh"));
    }

    #[test]
    fn build_run_command_selinux_z_on_linux_podman() {
        let tmp = TempDir::new().unwrap();
        let mounts = make_mounts(tmp.path());
        let cmd = build_run_command(
            &podman_runtime(),
            "ubuntu:25.10",
            "feat",
            &mounts,
            &[],
            &[],
            &NetworkMode::Full,
            "/workspace",
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
            "feat",
            &mounts,
            &[],
            &[],
            &NetworkMode::Full,
            "/workspace",
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
            "feat",
            &mounts,
            &[],
            &[],
            &NetworkMode::None,
            "/workspace",
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
            "feat",
            &mounts,
            &["ANTHROPIC_API_KEY".to_string()],
            &[],
            &NetworkMode::Full,
            "/workspace",
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
}
