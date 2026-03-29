# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`am` (Agent Manager) is a Rust CLI tool that creates isolated environments for coding agents (Claude Code, GitHub Copilot, Gemini, Codex, Aider, etc.). Each session gets its own git worktree or jj workspace, a dedicated tmux window with split panes, and optional containerization via Podman or Docker.

## Commands

```bash
cargo build                  # Debug build
cargo build --release        # Release build
cargo test                   # Run all tests
cargo test <module>          # Run tests in specific module (e.g., cargo test config)
cargo test -- --nocapture    # Show test output
cargo run -- <command>       # Run (e.g., cargo run -- start my-feature)
make build-claude            # Build Claude Code Docker image
make build-copilot           # Build Copilot Docker image
```

## Architecture

**Modules:**
- `cli.rs` — clap-based CLI definitions; slug validation (1–40 chars, lowercase/digits/hyphens/underscores)
- `config.rs` — layered config loading (CLI flags → env vars → project `.am/config.toml` → global `~/.config/am/config.toml` → defaults)
- `error.rs` — unified `AmError` enum via `thiserror`; all functions return `anyhow::Result<T>`
- `session.rs` — session CRUD; state persisted to `.am/sessions.json`
- `worktree.rs` — git worktree (`git worktree add`) and jj workspace (`jj workspace add`) operations
- `tmux.rs` — tmux window/pane creation and management
- `container.rs` — Podman/Docker container lifecycle; mount resolution; agent auth presets
- `main.rs` — command handler functions (`cmd_init`, `cmd_start`, `cmd_list`, `cmd_attach`, `cmd_run`, `cmd_destroy`, `cmd_generate_config`)

**VCS detection:** checks for `.jj/` first, falls back to `.git`, errors if neither found.

**Container mount layouts differ for git vs jj repos** — see `container.rs` for specifics. Key: git repos use `GIT_DIR`/`GIT_WORK_TREE` env vars to point the agent at the worktree; jj repos mirror the host path structure.

**Agent auth presets** (`claude`, `copilot`, `gemini`, `codex`, `aider`) mount credentials at runtime — no secrets baked into images. Unknown agent names are treated as raw executable commands with no auth.

## Testing Patterns

- `tempfile` crate for isolated test directories
- Mock tmux via `AM_TMUX_BIN` env var; mock container runtimes via `AM_PODMAN_BIN`/`AM_DOCKER_BIN`
- Tests that mutate env vars use a mutex to serialize execution (see existing tests for pattern)

**After every code change:** run `cargo test` to verify no compiler errors and all tests pass before considering the task complete. Fix any failures before proceeding.

## Version Control

This repo uses **jj (Jujutsu)**. Use `jj` commands instead of `git` for all VCS operations.

Commits use **Conventional Commits** format: `type(scope): description` (e.g., `feat(container): add Codex auth preset`, `fix(config): handle missing home dir`). After successfully implementing a feature, create a commit using `jj commit -m "..."` (not `jj describe`) so the working copy is left clean and empty.

Commit messages should end with a footer separated by `---`. Use the trailer that matches how the agent was involved:

| Trailer | When to use |
|---|---|
| `Co-Piloted-By` | Interactive session — agent wrote or modified code with the user directing |
| `Auto-Piloted-By` | Autonomous session — agent worked independently (`--auto`) |
| `Co-Reviewed-By` | Interactive review — agent reviewed code with the user |
| `Auto-Reviewed-By` | Autonomous review — agent reviewed code independently |

The value is always `am via <agent name>`. Claude Code sessions are interactive by default, so the standard trailer is:

```
---
Co-Piloted-By: am via Claude Code
```

See `docs/reference/commit-trailers.md` for full documentation and examples.

## Key Reference Files

- `SPEC.md` — full technical specification with function signatures and step-by-step command flows
- `PLAN.md` — implementation status; pending: Codex/Gemini agent integration, polish/distribution
- `config.md` — configuration reference with all env vars and settings
