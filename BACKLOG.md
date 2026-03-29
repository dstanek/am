# Backlog

Outstanding work for `am`. Items are grouped by theme and roughly ordered by priority.

---

## Agent Integrations

### Feature 7: Codex Integration
> Spec: [`specs/codex-integration.md`](specs/codex-integration.md)

Env-var-only auth preset for OpenAI Codex — no filesystem mount needed.

- [ ] Design: no mount; auth via `OPENAI_API_KEY` env var pass-through; `validate_agent("codex")` must pass
- [ ] Tests: `resolve_agent_auth_mount("codex")` returns `None`; `build_run_command` includes `-e OPENAI_API_KEY` when codex preset active
- [ ] Implementation: `codex` branch in `resolve_agent_auth_mount()`; env-var injection logic for env-var-only presets
- [ ] UX Review: `am start feat --agent codex` passes `OPENAI_API_KEY` into the container; no spurious mount errors

---

### Feature 8: Gemini Integration
> Spec: [`specs/gemini-integration.md`](specs/gemini-integration.md)

Auth mount preset for Google Gemini CLI: `~/.gemini:/root/.gemini:ro`.

- [ ] Design: preset `~/.gemini:/root/.gemini:ro`; `validate_agent("gemini")` must pass
- [ ] Tests: `resolve_agent_auth_mount("gemini")` returns correct host/container paths; `build_run_command` includes gemini mount when preset active
- [ ] Implementation: `gemini` branch in `resolve_agent_auth_mount()`
- [ ] UX Review: `am start feat --agent gemini` launches a container with `~/.gemini` mounted read-only; graceful handling if `~/.gemini` doesn't exist (warn, don't fail)

---

### Feature 9: Aider Integration
> Spec: [`specs/aider-integration.md`](specs/aider-integration.md)

Env-var-only auth preset for Aider — passes through both `ANTHROPIC_API_KEY` and `OPENAI_API_KEY`.

- [ ] Design: no mount; both `ANTHROPIC_API_KEY` and `OPENAI_API_KEY` in env pass-through; only inject keys that are actually set on the host
- [ ] Tests: `resolve_agent_auth_mount("aider")` returns `None`; `build_run_command` includes both `-e ANTHROPIC_API_KEY` and `-e OPENAI_API_KEY` when aider preset active
- [ ] Implementation: `aider` branch in `resolve_agent_auth_mount()` (returns `None`); env var injection for both keys
- [ ] UX Review: `am start feat --agent aider` passes both API key vars into the container; no spurious mount errors

---

## Polish & Distribution
> Spec: [`specs/polish-and-distribution.md`](specs/polish-and-distribution.md)

### Integration test: full flow

Write an integration test that exercises `am init` → `am start` → `am list` → `am destroy` as a single end-to-end flow in a temp git repo, outside tmux with `--no-container`. Lives in `tests/`.

### README

Write `README.md` at the repo root covering:
1. What it is — one-paragraph overview
2. Install — `cargo install --path .` and eventual binary download placeholder
3. Quick start — `am init` → `am start feat --agent claude` → `am attach feat` → `am destroy feat`
4. Configuration — pointer to `config.md`; minimal `~/.config/am/config.toml` example
5. Supported agents — table: claude, codex, copilot, gemini, aider
6. Example Dockerfile — minimal image that installs `claude` and works with `am`

### Error message audit

Review every user-facing error for clarity and actionability — each should tell the user what went wrong AND what to do next:

- `ContainerImageNotConfigured` → suggest setting `container.image` in config
- `ContainerRuntimeNotFound` → include Podman install URL
- `SlugAlreadyExists` → suggest `am destroy <slug>` or `am attach <slug>`
- `NotInTmux` → explain that the command requires an active tmux session
- `SlugNotFound` → suggest `am list` to see valid slugs

### Cross-platform build verification

Verify `cargo build --release` produces a working binary on:
- Linux x86_64
- macOS arm64 (Apple Silicon)
- macOS x86_64 (Intel)

Document any platform-specific build requirements or CI configuration needed.

---

## Bug Fixes

### Context-aware user messages
> Spec: [`specs/context-aware-messages.md`](specs/context-aware-messages.md)

Commands currently emit tmux-specific language (e.g. "kill tmux window") even when `$TMUX` is not set.

- Introduce a `Messages` trait (or `TmuxMessages`/`PlainMessages` structs) chosen once at startup from `tmux::is_in_tmux()`
- Thread it through command functions; remove any scattered inline `is_in_tmux()` checks used only for string selection
- Audit all `println!`, `eprintln!`, and confirmation prompts in command handlers
- Tests: `am destroy <slug> --force` outside tmux does not mention "window" or "pane"; inside tmux it does

---

## Future (v2)

### Autonomous mode (`--auto` flag)

Add `--auto` to `am start`. In autonomous mode the agent works without waiting for user input between steps. Sets a flag in the session record; commit trailer becomes `Auto-Piloted-By`.

### Team orchestration (`--team` flag)

Add `--team` to `am start` to launch and coordinate multiple agents working toward a shared goal. Each agent gets its own isolated session and branch; `am` handles orchestration. Open questions: goal specification, result surfacing, how many agents and what slugs.

### Agent completion detection + OS notifications

Automatically detect when an agent pane exits or enters a waiting-for-input state and send an OS notification. Requires watching pane exit events via tmux hooks or a background thread.

### Per-session SSH deploy keys

Generate an SSH deploy key per session and inject it into the container, replacing the current `~/.ssh` read-only mount. Improves isolation and avoids exposing the user's main SSH credentials.

### Hooks

Run user-defined shell commands on session lifecycle events (start, attach, destroy, agent exit). Configured in project or global config.

### Versioned documentation

Add [`mike`](https://github.com/jimporter/mike) alongside the existing MkDocs setup to deploy versioned docs to GitHub Pages (e.g. `/0.1/`, `/0.2/`). Defer until breaking changes start appearing between minor versions or users begin pinning to older releases.
