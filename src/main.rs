mod cli;
mod config;
mod error;
mod session;
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
        Commands::Start { slug, agent, editor, no_container } => {
            cmd_start(&slug, agent.as_deref(), editor.as_deref(), no_container)
        }
        Commands::List => cmd_list(),
        Commands::Attach { slug } => {
            eprintln!("am attach {slug} (not yet implemented)");
            Ok(())
        }
        Commands::Done { slug, message: _ } => {
            eprintln!("am done {slug} (not yet implemented)");
            Ok(())
        }
        Commands::Run { slug, agent } => {
            eprintln!("am run {slug} {agent} (not yet implemented)");
            Ok(())
        }
        Commands::Clean { slug, force } => cmd_clean(&slug, force),
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

fn cmd_start(slug: &str, _agent: Option<&str>, _editor: Option<&str>, _no_container: bool) -> anyhow::Result<()> {
    let repo_root = find_repo_root()?;
    let sessions = session::load_sessions(&repo_root)?;

    if session::find_session(&sessions, slug).is_some() {
        return Err(error::AmError::SlugAlreadyExists(slug.to_string()).into());
    }

    let worktree_path = worktree::create_git_worktree(slug, &repo_root)?;

    let new_session = session::Session {
        slug: slug.to_string(),
        branch: format!("am/{slug}"),
        worktree_path: worktree_path.clone(),
        tmux_window: format!("am-{slug}"),
        agent_pane: format!("am-{slug}.0"),
        shell_pane: format!("am-{slug}.1"),
        created_at: chrono::Utc::now(),
        status: session::SessionStatus::Active,
        container: None,
    };
    session::add_session(&repo_root, new_session)?;

    println!("Started session '{slug}'");
    println!("  worktree: {}", worktree_path.display());
    println!("  branch:   am/{slug}");
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

    // Column widths (with minimums)
    let slug_w = sessions.iter().map(|s| s.slug.len()).max().unwrap_or(4).max(4);
    let path_w = sessions.iter().map(|s| s.worktree_path.display().to_string().len()).max().unwrap_or(8).max(8);

    println!(
        "{:<slug_w$}  {:<8}  {:<9}  {:<path_w$}  {:<10}  CREATED",
        "SLUG", "STATUS", "CONTAINER", "WORKTREE", "WINDOW",
    );
    println!("{}", "-".repeat(slug_w + 8 + 9 + path_w + 10 + 19 + 10));

    for s in &sessions {
        let status = match s.status {
            session::SessionStatus::Active => "active",
            session::SessionStatus::Done => "done",
        };
        let container = s.container.as_ref().map(|c| c.runtime.as_str()).unwrap_or("—");
        let created = s.created_at.format("%Y-%m-%d %H:%M").to_string();
        println!(
            "{:<slug_w$}  {:<8}  {:<9}  {:<path_w$}  {:<10}  {}",
            s.slug,
            status,
            container,
            s.worktree_path.display(),
            s.tmux_window,
            created,
        );
    }
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

    // Remove worktree (ignore error if already gone)
    let _ = worktree::remove_git_worktree(slug, &repo_root);

    // Remove session record
    session::remove_session(&repo_root, slug)?;

    println!("Cleaned session '{slug}'.");
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn find_repo_root() -> anyhow::Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join(".git").exists() || dir.join(".jj").exists() {
            return Ok(dir);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return Err(error::AmError::NotInRepo.into()),
        }
    }
}
