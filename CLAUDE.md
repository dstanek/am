# CLAUDE.md

## Project Overview

`am` (Agent Manager) is a Rust CLI tool that creates isolated environments for coding agents (Claude Code, GitHub Copilot, Gemini, Codex, Aider, etc.). Each session gets its own git worktree or jj workspace, a dedicated tmux window with split panes, and optional containerization via Podman or Docker.

## Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run all tests (run after every change)
cargo clippy -- -D warnings  # Lint (run after every change)
cargo run -- <command>   # Run (e.g., cargo run -- start my-feature)
make build-claude        # Build Claude Code Docker image
make build-copilot       # Build Copilot Docker image
```

## Architecture

**Modules:**
- `cli.rs` — clap CLI; slug validation (1–40 chars, lowercase/digits/hyphens/underscores)
- `config.rs` — layered config: CLI flags → env vars → `.am/config.toml` → `~/.config/am/config.toml` → defaults
- `error.rs` — `AmError` enum via `thiserror`; all functions return `anyhow::Result<T>`
- `session.rs` — session CRUD; state in `.am/sessions.json`
- `worktree.rs` — git (`git worktree add`) and jj (`jj workspace add`) operations
- `tmux.rs` — tmux window/pane management
- `container.rs` — Podman/Docker lifecycle; mount resolution; agent auth presets
- `main.rs` — command handlers (`cmd_init`, `cmd_start`, `cmd_list`, `cmd_attach`, `cmd_run`, `cmd_destroy`, `cmd_generate_config`)

**VCS detection:** checks `.jj/` first, falls back to `.git`, errors if neither found.

**Container mounts:** git repos use `GIT_DIR`/`GIT_WORK_TREE` env vars; jj repos mirror host path structure. See `container.rs`.

**Agent auth presets** (`claude`, `copilot`, `gemini`, `codex`, `aider`) mount credentials at runtime. Unknown agent names are raw executable commands with no auth.

## Testing

- `tempfile` crate for isolated test directories
- Mock tmux via `AM_TMUX_BIN`; mock container runtimes via `AM_PODMAN_BIN`/`AM_DOCKER_BIN`
- Tests mutating env vars use a mutex to serialize execution

**After every code change:** run `cargo test` and `cargo clippy -- -D warnings`. Fix any failures before proceeding.

## Path Handling Strategy

Preserve type safety as long as possible by keeping Path/PathBuf/OsStr until converting to String is absolutely necessary:

**Hierarchy (in order of preference):**
1. `Path`/`PathBuf`/`OsStr` — Keep internal code type-safe (no early conversion)
2. `&Path` / `&OsStr` — Use in function parameters (not `&str`)
3. `.as_path()` / `.as_os_str()` — Borrow without copying
4. `.display()` — For format strings in logging/printing (never panics, handles UTF-8)
5. `.to_string_lossy()` — When `Cow<str>` or owned `String` is needed (command args, mounting)
6. `.to_str()?` — Only for critical UTF-8 requirements with proper error handling
7. `String` — Last resort (owned copy)

**Practical rules:**
- ✅ `fn foo(path: &Path)` — NOT `fn foo(path: &str)`
- ✅ Use `.display()` for error messages and logging
- ✅ Convert to String only at boundaries (Command args, container mounts)
- ✅ Inline conversions: `cmd.arg(path.to_string_lossy())` rather than intermediate variables
- ❌ Don't convert early: avoid `let path_str = path.to_string_lossy(); ... use later`

This approach improves type safety, clarity (conversions are visible at call sites), and performance (fewer allocations).

## Version Control

Use `jj` commands (not `git`). Commits use **Conventional Commits**: `type(scope): description`.

Use `jj commit -m "..."` (not `jj describe`) to leave the working copy clean. Append this footer to every commit message:

```
---
Co-Piloted-By: am via Claude Code
```

For other trailer types (autonomous, review), see `docs/reference/commit-trailers.md`.

## Key Reference Files

- `SPEC.md` — full technical specification
- `PLAN.md` — implementation status
- `config.md` — configuration reference
