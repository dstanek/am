mod cli;
mod config;
mod error;
mod session;

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
        Commands::Start { slug, agent: _, editor: _, no_container: _ } => {
            eprintln!("am start {slug} (not yet implemented)");
            Ok(())
        }
        Commands::List => {
            eprintln!("am list (not yet implemented)");
            Ok(())
        }
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
        Commands::Clean { slug, force: _ } => {
            eprintln!("am clean {slug} (not yet implemented)");
            Ok(())
        }
    }
}

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
        use std::io::Write;
        file.write_all(b".am/\n")?;
        println!("Added .am/ to .gitignore");
    }

    println!("am initialized. Run 'am start <slug>' to create your first session.");
    Ok(())
}

fn find_repo_root() -> anyhow::Result<std::path::PathBuf> {
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
