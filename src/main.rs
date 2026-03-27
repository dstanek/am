mod cli;
mod config;
mod container;
mod error;
mod session;
mod tmux;
mod worktree;

use std::io::Write;
use std::path::PathBuf;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Init => cmd_init(),
        Commands::Start { slug, agent, no_container } => {
            cmd_start(&slug, agent.as_deref(), no_container)
        }
        Commands::List => cmd_list(),
        Commands::Attach { slug } => cmd_attach(&slug),
        Commands::Run { slug, agent } => cmd_run(&slug, &agent),
        Commands::Clean { slug, force } => cmd_clean(&slug, force),
        Commands::GenerateConfig => cmd_generate_config(),
    }
}

// ── am init ───────────────────────────────────────────────────────────────────

fn cmd_init() -> anyhow::Result<()> {
    let repo_root = find_repo_root()?;

    let am_dir = repo_root.join(".am");
    std::fs::create_dir_all(&am_dir)?;

    let config_path = am_dir.join("config.toml");
    if !config_path.exists() {
        config::write_defaults(&config_path)?;
        println!("Created .am/config.toml");
    } else {
        println!(".am/config.toml already exists, skipping");
    }

    let sessions_path = am_dir.join("sessions.json");
    if !sessions_path.exists() {
        std::fs::write(&sessions_path, "{\"sessions\":[]}\n")?;
        println!("Created .am/sessions.json");
    }

    let gitignore_path = repo_root.join(".gitignore");
    let already_ignored = if gitignore_path.exists() {
        let content = std::fs::read_to_string(&gitignore_path)?;
        content.lines().any(|l| l.trim() == ".am/" || l.trim() == ".am")
    } else {
        false
    };
    if !already_ignored {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitignore_path)?;
        file.write_all(b".am/\n")?;
        println!("Added .am/ to .gitignore");
    }

    println!("am initialized. Run 'am start <slug>' to create your first session.");
    Ok(())
}

// ── am start ──────────────────────────────────────────────────────────────────

fn cmd_start(slug: &str, agent_flag: Option<&str>, no_container: bool) -> anyhow::Result<()> {
    let repo_root = find_repo_root()?;
    let sessions = session::load_sessions(&repo_root)?;

    if session::find_session(&sessions, slug).is_some() {
        return Err(error::AmError::SlugAlreadyExists(slug.to_string()).into());
    }

    // Load config
    let project_config_path = repo_root.join(".am").join("config.toml");
    let cfg = config::load_with_global(
        config::global_config_path().as_deref(),
        Some(&project_config_path),
    )?;

    // Effective agent: --agent flag > config.container.agent > config.agent
    let effective_agent = agent_flag
        .map(str::to_string)
        .or_else(|| cfg.container.agent.clone())
        .or_else(|| cfg.agent.clone());

    // ── Early validation (fail before any side effects) ───────────────────────

    // 1. VCS
    let vcs = worktree::detect_vcs(&repo_root)?;

    // 2. Agent credentials
    if let Some(ref agent) = effective_agent {
        container::validate_agent(agent)?;
    }

    // 3. Container config
    let use_container = cfg.container.enabled && !no_container;
    let (_runtime, container_cmd, session_container) = if use_container {
        let image = cfg.container.image.as_deref()
            .filter(|s| !s.is_empty())
            .ok_or(error::AmError::ContainerImageNotConfigured)?;

        let runtime = container::detect_runtime(cfg.container.runtime.clone())?;

        // Pre-emptively remove any leftover container from a previous run
        container::remove_if_exists(&runtime, &format!("am-{slug}"));

        let mounts = container::resolve_mounts(slug, &repo_root, &vcs, effective_agent.as_deref())?;

        let extra_env: Vec<(&str, &str)> = vec![];
        let mut cmd = container::build_run_command(
            &runtime,
            image,
            &mounts,
            &cfg.container.env,
            &extra_env,
            &cfg.container.network,
            &format!("am-{slug}"),
        );

        // Outside tmux: append the agent as the container CMD so it launches directly
        if !tmux::is_in_tmux() {
            if let Some(ref agent) = effective_agent {
                cmd.push(agent.clone());
            }
        }

        let sc = session::SessionContainer {
            runtime: format!("{:?}", runtime.kind).to_lowercase(),
            image: image.to_string(),
            container_id: None,
        };
        (Some(runtime), Some(cmd), Some(sc))
    } else {
        (None, None, None)
    };

    let worktree_path = match vcs {
        config::Vcs::Git => worktree::create_git_worktree(slug, &repo_root)?,
        config::Vcs::Jj => worktree::create_jj_workspace(slug, &repo_root)?,
    };
    let window_name = format!("am-{slug}");

    if tmux::is_in_tmux() {
        tmux::create_window(&window_name, &worktree_path)
            .map_err(|e| anyhow::anyhow!(
                "{e}\nHint: a window named '{window_name}' may already exist — run 'am clean {slug}' first"
            ))?;
        tmux::split_window(&window_name, &worktree_path, &cfg.tmux.split)?;

        // Send container run command to agent pane (pane 0)
        if let Some(ref cmd) = container_cmd {
            let full_cmd = cmd.join(" ");
            tmux::send_keys(&tmux::get_pane_id(&window_name, 0), &full_cmd)?;

            // Auto-launch agent inside the container after startup delay
            if let Some(ref agent) = effective_agent {
                std::thread::sleep(std::time::Duration::from_millis(cfg.container.startup_delay_ms));
                tmux::send_keys(&tmux::get_pane_id(&window_name, 0), agent)?;
            }
        }

        tmux::select_pane(&tmux::get_pane_id(&window_name, 0))?;
        tmux::select_window(&window_name)?;
    } else if let Some(ref cmd) = container_cmd {
        // Not in tmux — exec the container directly, replacing this process
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .exec();
        // exec() only returns on failure
        return Err(error::AmError::ContainerError(format!("failed to exec container: {err}")).into());
    } else {
        println!("Note: not inside tmux — no window opened. Run 'am attach {slug}' from inside tmux to open one.");
    }

    let new_session = session::Session {
        slug: slug.to_string(),
        branch: format!("am/{slug}"),
        worktree_path: worktree_path.clone(),
        tmux_window: window_name,
        agent_pane: format!("am-{slug}.0"),
        shell_pane: format!("am-{slug}.1"),
        created_at: chrono::Utc::now(),
        container: session_container,
    };
    session::add_session(&repo_root, new_session)?;

    println!("Started session '{slug}'");
    println!("  worktree:  {}", worktree_path.display());
    println!("  branch:    am/{slug}");
    if use_container {
        println!("  container: am-{slug}");
    }
    Ok(())
}

// ── am list ───────────────────────────────────────────────────────────────────

fn cmd_list() -> anyhow::Result<()> {
    let repo_root = find_repo_root()?;
    let sessions = session::load_sessions(&repo_root)?;

    if sessions.is_empty() {
        println!("No active sessions. Run 'am start <slug>' to begin.");
        return Ok(());
    }

    let slug_w = sessions.iter().map(|s| s.slug.len()).max().unwrap_or(4).max(4);
    let path_w = sessions.iter().map(|s| s.worktree_path.display().to_string().len()).max().unwrap_or(8).max(8);

    println!(
        "{:<slug_w$}  {:<9}  {:<path_w$}  {:<10}  CREATED",
        "SLUG", "CONTAINER", "WORKTREE", "WINDOW",
    );
    println!("{}", "-".repeat(slug_w + 9 + path_w + 10 + 19 + 8));

    for s in &sessions {
        let container = s.container.as_ref().map(|c| c.runtime.as_str()).unwrap_or("—");
        let created = s.created_at.format("%Y-%m-%d %H:%M").to_string();
        println!(
            "{:<slug_w$}  {:<9}  {:<path_w$}  {:<10}  {}",
            s.slug,
            container,
            s.worktree_path.display(),
            s.tmux_window,
            created,
        );
    }
    Ok(())
}

// ── am attach ────────────────────────────────────────────────────────────────

fn cmd_attach(slug: &str) -> anyhow::Result<()> {
    let repo_root = find_repo_root()?;
    let sessions = session::load_sessions(&repo_root)?;

    let s = session::find_session(&sessions, slug)
        .ok_or_else(|| error::AmError::SlugNotFound(slug.to_string()))?;

    if !tmux::is_in_tmux() {
        return Err(error::AmError::NotInTmux.into());
    }

    let window_name = format!("am-{slug}");

    // Try to switch to an existing window; if it's not there, create it.
    if tmux::select_window(&window_name).is_err() {
        let project_config_path = repo_root.join(".am").join("config.toml");
        let cfg = config::load_with_global(
            config::global_config_path().as_deref(),
            Some(&project_config_path),
        )?;
        tmux::create_window(&window_name, &s.worktree_path)
            .map_err(|e| anyhow::anyhow!(
                "{e}\nHint: a window named '{window_name}' may already exist — run 'am clean {slug}' first"
            ))?;
        tmux::split_window(&window_name, &s.worktree_path, &cfg.tmux.split)?;
        tmux::select_pane(&tmux::get_pane_id(&window_name, 0))?;
        tmux::select_window(&window_name)?;
        println!("Opened new window for session '{slug}'.");
    } else {
        println!("Attached to session '{slug}'.");
    }
    Ok(())
}

// ── am run ────────────────────────────────────────────────────────────────────

fn cmd_run(slug: &str, agent: &str) -> anyhow::Result<()> {
    let repo_root = find_repo_root()?;
    let sessions = session::load_sessions(&repo_root)?;

    let s = session::find_session(&sessions, slug)
        .ok_or_else(|| error::AmError::SlugNotFound(slug.to_string()))?;

    if !tmux::is_in_tmux() {
        return Err(error::AmError::NotInTmux.into());
    }

    tmux::send_keys(&s.agent_pane, agent)?;
    tmux::select_window(&s.tmux_window)?;
    println!("Launched '{agent}' in session '{slug}'.");
    Ok(())
}

// ── am clean ─────────────────────────────────────────────────────────────────

fn cmd_clean(slug: &str, force: bool) -> anyhow::Result<()> {
    let repo_root = find_repo_root()?;
    let sessions = session::load_sessions(&repo_root)?;

    if session::find_session(&sessions, slug).is_none() {
        return Err(error::AmError::SlugNotFound(slug.to_string()).into());
    }

    if !force {
        print!("Remove worktree and kill tmux window for '{slug}'? [y/N] ");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Stop and remove container if one was recorded for this session
    if let Some(s) = session::find_session(&sessions, slug) {
        if let Some(ref sc) = s.container {
            let pref = match sc.runtime.as_str() {
                "docker" => config::RuntimePreference::Docker,
                _ => config::RuntimePreference::Podman,
            };
            if let Ok(rt) = container::detect_runtime(pref) {
                let _ = container::stop_container(&rt, &format!("am-{slug}"));
                let _ = container::remove_container(&rt, &format!("am-{slug}"));
            }
        }
    }

    // Kill tmux window (ignore error — window may not exist)
    let _ = tmux::kill_window(&format!("am-{slug}"));

    // Remove worktree — warn but continue if it's already gone
    let vcs = worktree::detect_vcs(&repo_root).unwrap_or(config::Vcs::Git);
    let remove_result = match vcs {
        config::Vcs::Git => worktree::remove_git_worktree(slug, &repo_root),
        config::Vcs::Jj => worktree::remove_jj_workspace(slug, &repo_root),
    };
    if let Err(e) = remove_result {
        eprintln!("warning: could not fully remove worktree: {e}");
    }

    // Remove session record
    session::remove_session(&repo_root, slug)?;

    println!("Cleaned session '{slug}'.");
    Ok(())
}

// ── am generate-config ────────────────────────────────────────────────────────

fn cmd_generate_config() -> anyhow::Result<()> {
    print!("{}", config::global_config_template());
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn find_repo_root() -> anyhow::Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        // .git in a worktree is a FILE pointing back to the main repo;
        // only stop when we find it as a DIRECTORY (the actual repo root).
        if dir.join(".jj").is_dir() || dir.join(".git").is_dir() {
            return Ok(dir);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return Err(error::AmError::NotInRepo.into()),
        }
    }
}
