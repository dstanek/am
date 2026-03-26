# Agent Manager (`am`) — Implementation Plan

## Process

Each feature follows this loop — repeat until the user signs off:

1. **Design** — review spec, document decisions, resolve open questions for this feature
2. **Tests** — write tests first (unit + integration); red before green
3. **Implementation** — write the code to make tests pass
4. **UX Review** — manual testing session with the user; iterate as needed

Mark sub-tasks `[x]` as completed. Mark the feature header `[x]` only after the user approves the UX review.

---

## Feature 0: Foundation

> Project skeleton, error types, config loading, and session state. Everything else builds on this.

- [x] **Design**
  - [x] Finalize `Cargo.toml` dependencies and workspace layout
  - [x] Review `AmError` variants; add any missing from the spec
  - [x] Clarify config merge precedence (global → project → CLI flags)
  - [x] Document `.am/` directory structure decisions

- [x] **Tests**
  - [x] `config.rs`: load defaults when no config file exists
  - [x] `config.rs`: project config overrides global config fields
  - [x] `session.rs`: add/find/remove/update sessions in `sessions.json`
  - [x] `session.rs`: missing file returns empty session list
  - [x] `error.rs`: each error variant formats correctly

- [x] **Implementation**
  - [x] `Cargo.toml` with all dependencies (`clap`, `git2`, `serde`, `toml`, `anyhow`, `thiserror`, `which`, `chrono`, `notify-rust`)
  - [x] `src/error.rs` — `AmError` enum with all variants
  - [x] `src/config.rs` — `Config`, `TmuxConfig`, `ContainerConfig` structs; `load()` and `write_defaults()`
  - [x] `src/session.rs` — `Session`, `SessionContainer` structs; full CRUD functions
  - [x] `src/main.rs` — minimal entry point wiring
  - [x] `src/cli.rs` — full CLI surface defined with clap (all commands stubbed)

- [x] **UX Review** — `am --help` shows all commands cleanly; config file created on first run

---

## Feature 1: Git Worktree Management

> Create, list, and remove git worktrees. The core of `am start` and `am clean`.

- [x] **Design**
  - [x] Confirm branch naming: `am/<slug>` off current `HEAD`
  - [x] Decide: error on existing branch, or `--track`? (spec suggests error with `SlugAlreadyExists`)
  - [x] Target path: `<repo-root>/.am/worktrees/<slug>`
  - [x] Slug validation rules: `[a-z0-9_-]`, 1–40 chars — implement as clap `value_parser`

- [x] **Tests**
  - [x] `worktree.rs`: `create_git_worktree` creates branch and directory in a temp repo
  - [x] `worktree.rs`: duplicate slug returns `SlugAlreadyExists`
  - [x] `worktree.rs`: `remove_git_worktree` removes worktree and branch
  - [x] `worktree.rs`: `detect_vcs` returns `Git` for a `.git` repo
  - [x] `worktree.rs`: `detect_vcs` returns error when not in a repo
  - [x] Slug validation rejects invalid characters and lengths

- [x] **Implementation**
  - [x] `src/worktree.rs` — `detect_vcs()`, `create_git_worktree()`, `remove_git_worktree()`
  - [x] `am init` command — create `.am/`, write default config, write empty `sessions.json`, append `.am/` to `.gitignore`
  - [x] `am start <slug>` — worktree creation only (no tmux, no container)
  - [x] `am list` — tabular output from `sessions.json`; empty state message
  - [x] `am clean <slug> [--force]` — remove worktree, remove session record, confirmation prompt
  - [x] Slug validation wired into clap

- [x] **UX Review** — `am init` → `am start feat` → `am list` → `am clean feat` full cycle feels right

---

## Feature 2: tmux Integration

> Split-pane tmux window per session. No agent launch yet — just the environment.

- [x] **Design**
  - [x] Decide pane assignment: `am-<slug>.0` = agent pane, `am-<slug>.1` = shell pane
  - [x] Default layout: 50/50 horizontal split; configurable via `TmuxConfig`
  - [x] Window naming collision strategy: error + tell user to run `am clean`
  - [x] Behaviour when not inside tmux: print worktree path, no error, no tmux ops

- [x] **Tests**
  - [x] `tmux.rs`: `is_in_tmux()` reads `$TMUX` env var
  - [x] `tmux.rs`: each function builds the correct tmux command (inject mock binary path)
  - [x] `tmux.rs`: `get_pane_id` returns `"<window>.<index>"`
  - [x] `am start` when `$TMUX` not set: succeeds, prints path, does not call tmux
  - [x] `am attach` when `$TMUX` not set: returns `NotInTmux` error

- [x] **Implementation**
  - [x] `src/tmux.rs` — `is_in_tmux()`, `create_window()`, `split_window()`, `select_pane()`, `select_window()`, `send_keys()`, `kill_window()`, `get_pane_id()`
  - [x] Wire tmux into `am start`: create window → split → select agent pane → switch focus
  - [x] `am attach <slug>` — `select_window` to existing session
  - [x] `am run <slug> <agent>` — `send_keys` agent command to agent pane, switch focus
  - [x] `am clean` — kill tmux window (ignore if not present)

- [x] **UX Review** — `am start feat` opens a correctly split window; `am attach feat` switches to it; `am clean feat` tears it down

---

## Feature 3: Podman Container Integration

> Rootless Podman containers for session isolation. Git worktree mount layout.

- [x] **Design**
  - [x] Mount table for git repos (worktree, `.git`, `~/.gitconfig`, `~/.ssh`)
  - [x] `GIT_DIR` / `GIT_WORK_TREE` env var injection
  - [x] SELinux `,z` label: Linux + Podman only
  - [x] Container naming: `am-<slug>`; pre-emptive `podman rm -f` on collision
  - [x] `--no-container` flag behaviour: `container: null` in session record
  - [x] `container.startup_delay_ms` configurable (default 500ms)
  - [x] Fail loudly if `container.image` not set when container mode active

- [x] **Tests**
  - [x] `container.rs`: `detect_runtime(Auto)` finds podman when on PATH
  - [x] `container.rs`: `detect_runtime(Auto)` errors when neither runtime found
  - [x] `container.rs`: `resolve_mounts` produces correct host/container paths for git repo
  - [x] `container.rs`: `build_run_command` includes all required flags (mounts, env, workdir, name)
  - [x] `container.rs`: `,z` appended on Linux+Podman, omitted otherwise
  - [x] `container.rs`: `stop_container` and `remove_container` build correct commands
  - [x] `am start` with `container.image` unset errors with `ContainerImageNotConfigured`

- [x] **Implementation**
  - [x] `src/container.rs` — `detect_runtime()`, `resolve_mounts()`, `build_run_command()`, `stop_container()`, `remove_container()`
  - [x] `ContainerMounts`, `AgentAuthMount`, `MountMode`, `ContainerRuntime` types
  - [x] Wire container into `am start`: resolve mounts → build command → `send_keys` to agent pane (tmux) or `exec()` (no tmux)
  - [x] `am clean` — stop + remove container before removing worktree
  - [x] `--no-container` flag wired in `am start`

- [x] **UX Review** — `am start feat` (with image configured) launches a Podman container in the agent pane with correct mounts; `am clean feat` stops and removes it

---

## Feature 4: Claude Code Integration

> Mount `~/.claude` into the container and optionally auto-launch `claude` in the agent pane.

- [x] **Design**
  - [x] Auth preset: `claude` → `~/.claude:/root/.claude:ro`
  - [x] Agent auto-launch flow: container start → 500ms delay → `send_keys "claude" Enter`
  - [x] `config.agent = "claude"` vs `--agent claude` precedence (`--agent` > `config.container.agent` > `config.agent`)
  - [x] `am run <slug> claude` for manual launch in existing session

- [x] **Tests**
  - [x] `container.rs`: `resolve_agent_auth_mount("claude")` returns correct host/container paths
  - [x] `container.rs`: `build_run_command` includes claude mount when preset is active
  - [x] `am start` with `agent = "claude"` queues second `send_keys` after delay
  - [x] `am run` sends correct keys to agent pane

- [x] **Implementation**
  - [x] `resolve_agent_auth_mount()` with `claude` preset
  - [x] Wire `agent_preset` through `resolve_mounts()` and `build_run_command()`
  - [x] Auto-launch `send_keys` with configurable delay (`startup_delay_ms`)
  - [x] `am run` command fully implemented

- [x] **UX Review** — `am start feat --agent claude` opens a container and auto-launches Claude Code; `am run feat claude` works on an existing session

---

## Feature 5: Docker Support

> Docker as a fallback when Podman is not available.

- [x] **Design**
  - [x] `RuntimePreference::Auto` order: Podman first, Docker second
  - [x] Docker differences: no `,z` SELinux labels on Linux
  - [x] `config.container.runtime = "docker"` to force Docker

- [x] **Tests**
  - [x] `detect_runtime(Auto)` returns Docker when Podman absent but Docker present
  - [x] `detect_runtime(Docker)` errors when Docker not on PATH
  - [x] `build_run_command` omits `,z` when runtime is Docker

- [x] **Implementation**
  - [x] `detect_runtime()` Docker branch
  - [x] `,z` conditional on runtime type (not just OS)
  - [x] `RuntimePreference::Docker` config variant wired

- [x] **UX Review** — on a system with only Docker, `am start feat` uses Docker correctly; `am list` shows `docker` in container column

---

## Feature 6: Copilot Integration

> Auth mount preset for GitHub Copilot CLI.

- [x] **Design**
  - [x] Preset: `~/.config/gh:/root/.config/gh:ro` + `~/.config/github-copilot:/root/.config/github-copilot:ro`
  - [x] No additional env vars required beyond the mounts
  - [x] `config.agent = "copilot"` activates the preset

- [x] **Tests**
  - [x] `resolve_agent_auth_mount("copilot")` returns correct host/container paths for both dirs
  - [x] `build_run_command` includes both copilot mounts when preset is active
  - [x] `am start feat --agent copilot` sends correct container command to agent pane

- [x] **Implementation**
  - [x] `copilot` branch in `resolve_agent_auth_mount()` — mounts `~/.config/gh` and `~/.config/github-copilot`
  - [x] Wired through `resolve_mounts()` and `build_run_command()`

- [x] **UX Review** — `am start feat --agent copilot` launches a container with both Copilot credential directories mounted

---

## Feature 7: Codex Integration

> Auth preset for OpenAI Codex — env-var only, no filesystem mount.

- [ ] **Design**
  - [ ] No mount needed; auth via `OPENAI_API_KEY` env var pass-through
  - [ ] `config.agent = "codex"` ensures `OPENAI_API_KEY` is included in env pass-through

- [ ] **Tests**
  - [ ] `resolve_agent_auth_mount("codex")` returns `None`
  - [ ] `build_run_command` includes `-e OPENAI_API_KEY` when codex preset active

- [ ] **Implementation**
  - [ ] `codex` branch in `resolve_agent_auth_mount()` (returns `None`)
  - [ ] Env var injection logic for env-var-only presets

- [ ] **UX Review** — `am start feat --agent codex` passes `OPENAI_API_KEY` into the container; no spurious mount errors

---

## Feature 8: Gemini Integration

> Auth mount preset for Google Gemini CLI.

- [ ] **Design**
  - [ ] Preset: `~/.gemini:/root/.gemini:ro`
  - [ ] `config.agent = "gemini"` activates the preset

- [ ] **Tests**
  - [ ] `resolve_agent_auth_mount("gemini")` returns correct host/container paths
  - [ ] `build_run_command` includes the gemini mount when preset is active

- [ ] **Implementation**
  - [ ] `gemini` branch in `resolve_agent_auth_mount()`

- [ ] **UX Review** — `am start feat --agent gemini` launches a container with `~/.gemini` mounted

---

## Feature 9: Aider Integration

> Auth preset for Aider — env-var only, supports both Anthropic and OpenAI keys.

- [ ] **Design**
  - [ ] No mount needed; auth via `ANTHROPIC_API_KEY` and/or `OPENAI_API_KEY`
  - [ ] `config.agent = "aider"` ensures both keys are included in env pass-through if present

- [ ] **Tests**
  - [ ] `resolve_agent_auth_mount("aider")` returns `None`
  - [ ] `build_run_command` includes both API key env vars when aider preset active

- [ ] **Implementation**
  - [ ] `aider` branch in `resolve_agent_auth_mount()` (returns `None`)
  - [ ] Both `ANTHROPIC_API_KEY` and `OPENAI_API_KEY` added to env pass-through

- [ ] **UX Review** — `am start feat --agent aider` passes the correct API keys into the container

---

## Feature 10: Unknown Agent Preset Handling

> Graceful degradation for unrecognised agent presets.

- [ ] **Design**
  - [ ] Unknown preset: log a warning, proceed without auth mount
  - [ ] User's `container.env` pass-through may still handle auth

- [ ] **Tests**
  - [ ] `resolve_agent_auth_mount("unknown-thing")` returns `None` and emits a warning
  - [ ] `am start` with unknown preset succeeds with a warning, not an error

- [ ] **Implementation**
  - [ ] Catch-all branch in `resolve_agent_auth_mount()` with `eprintln!` warning

- [ ] **UX Review** — typo in agent preset shows a clear warning and still starts the session

---

## Feature 11: jj Workspace Support

> Jujutsu (jj) as an alternative VCS — workspace creation and container mount layout.

- [x] **Design**
  - [x] Detection: `.jj/` directory at repo root → `Vcs::Jj`
  - [x] `jj workspace add .am/worktrees/<slug> --name <slug>`
  - [x] `jj workspace forget <slug>` + directory delete on remove
  - [x] Container mount layout: mirror host path structure (`.jj` at repo root, worktree at sub-path)
  - [x] No `GIT_DIR`/`GIT_WORK_TREE` env vars needed for jj

- [x] **Tests**
  - [x] `detect_vcs` returns `Jj` when `.jj/` present
  - [x] `create_jj_workspace` shells out correct `jj` command
  - [x] `remove_jj_workspace` calls `jj workspace forget` then removes directory
  - [x] `resolve_mounts` for jj produces correct container paths
  - [x] `build_run_command` sets correct `--workdir` for jj sessions

- [x] **Implementation**
  - [x] `create_jj_workspace()` and `remove_jj_workspace()` in `worktree.rs`
  - [x] jj mount layout in `resolve_mounts()`
  - [x] `detect_vcs` updated to check `.jj` before `.git`
  - [x] `am start` and `am clean` use VCS-appropriate functions

- [x] **UX Review** — `am start feat` in a jj repo creates a workspace; container mounts are correct; `am clean feat` tears down cleanly

---

## Feature 12: Lifecycle & Notifications

> `am done` command, OS notifications, and session status tracking.

- [ ] **Design**
  - [ ] `notify-rust` graceful degradation (warn + continue on D-Bus failure)
  - [ ] `am done` notification format: `"✅ <slug> is done"` or with `--message`
  - [ ] `SessionStatus::Active` vs `Done` — `am list` shows both
  - [ ] Future: auto-trigger on agent pane exit (out of scope for v1, noted)

- [ ] **Tests**
  - [ ] `notify.rs`: send does not panic when notification system unavailable
  - [ ] `session.rs`: `update_session_status` persists correctly
  - [ ] `am done` updates status and triggers notification
  - [ ] `am list` shows `done` status in output

- [ ] **Implementation**
  - [ ] `src/notify.rs` — `send()` with graceful error handling
  - [ ] `am done <slug> [--message]` fully implemented
  - [ ] `am list` status column (active / done)

- [ ] **UX Review** — `am done feat` sends a desktop notification and updates list output

---

## Feature 13: Polish & Distribution

> Slug validation hardening, global config support, full test coverage, README.

- [ ] **Design**
  - [ ] Global config path: `~/.config/am/config.toml`
  - [ ] Config merge order: global defaults → project `.am/config.toml` → CLI flags
  - [ ] `am init` adds `.am/` to `.gitignore` (idempotent)
  - [ ] Target platforms: macOS (arm64 + x86_64), Linux (x86_64)

- [ ] **Tests**
  - [ ] Integration: `am start` → `am list` → `am clean` full flow
  - [ ] Integration: `am start` → `am done` → `am list` shows done status
  - [ ] Slug validation: boundary conditions (1 char, 40 chars, 41 chars, invalid chars)
  - [ ] Global config loaded when project config absent

- [ ] **Implementation**
  - [ ] Global config path and loading in `config.rs`
  - [ ] Slug `value_parser` in `cli.rs`
  - [ ] Full integration tests in `tests/`
  - [ ] `cargo build --release` verified on target platforms
  - [ ] README with install + usage + example Dockerfile
  - [ ] Error messages reviewed for user-friendliness

- [ ] **UX Review** — end-to-end flow on a real project feels polished; all error messages are actionable

---

## Bugs / Improvements Backlog

### Bug: Context-aware user messages

Commands like `am clean` currently say "Remove worktree and kill tmux window" even when
not running inside tmux. All user-facing strings should reflect the actual runtime context.

**Approach:** introduce a `Messages` trait (or pair of structs — `TmuxMessages` and
`PlainMessages`) with associated constants/methods for each user-facing string. The right
implementation is chosen once at startup based on `tmux::is_in_tmux()` and threaded
through (or stored as a global) so every command automatically uses context-appropriate
wording. No conditional `if is_in_tmux()` checks scattered through command handlers.

---

## Completed Features

- [x] **Feature 0: Foundation** — project skeleton, error types, config loading, session state
- [x] **Feature 1: Git Worktree Management** — `am start`, `am list`, `am clean` with real git worktrees
- [x] **Feature 2: tmux Integration** — split-pane windows, `am attach` (create-or-attach), `am run`
- [x] **Feature 3: Podman Container Integration** — rootless containers, git mount layout, `exec()` outside tmux
- [x] **Feature 4: Claude Code Integration** — `~/.claude` mount preset, auto-launch in container
- [x] **Feature 5: Docker Support** — runtime fallback, no `,z` labels
- [x] **Feature 6: Copilot Integration** — `Dockerfile.copilot` with gh + `@github/copilot`; `~/.config/gh` mount preset
- [x] **Feature 11: jj Workspace Support** — `create/remove_jj_workspace`, VCS dispatch in `am start`/`am clean` — `Dockerfile.copilot` with gh + `@github/copilot`; `~/.config/gh` mount preset

---

## Open Decisions Log

Track design decisions made during implementation:

| # | Question | Decision | Feature |
|---|---|---|---|
| 1 | Window naming collision | Error + tell user to run `am clean` | tmux |
| 2 | Branch base | Current `HEAD` | git worktree |
| 3 | Split ratio | Configurable `split_percent`, default 50 | tmux |
| 4 | `am done` trigger | Manual only in v1; auto-watch is v2 | lifecycle |
| 5 | Container startup delay | Configurable `startup_delay_ms`, default 500 | Podman |
| 6 | SELinux `,z` | Linux + Podman only | Podman |
| 7 | Container name collision | `podman rm -f am-<slug>` before launch, log warning | Podman |
| 8 | `am attach` when no window | Creates window + split (create-or-attach), not an error | tmux |
| 9 | Prompt to start tmux session from `am start` | Deferred to v2 | tmux |
| 10 | Container launch outside tmux | `exec()` the container directly in the current shell (replaces `am` process) | Podman |
