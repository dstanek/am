# Feature 13: Polish & Distribution (Remaining Work)

Remaining items from the Polish & Distribution feature. The bulk of this feature is already
done (global config, env var overrides, `am generate-config`, integration tests, config.md).
What's left is cross-platform build verification, a README, an error message audit, and
one remaining integration test.

## Remaining Items

### Integration Test: Full Flow

Write an integration test that exercises `am start` → `am list` → `am clean` as a single
end-to-end flow without tmux or containers (plain worktree mode):

- `am init` in a temp git repo
- `am start <slug>` (outside tmux, `--no-container`)
- `am list` shows the session
- `am clean <slug> --force` removes it
- `am list` shows no sessions

This test should live in `tests/` alongside the existing integration tests.

### README

Write `README.md` at the repo root covering:

1. **What it is** — one-paragraph overview (agent-agnostic isolated session manager)
2. **Install** — `cargo install --path .` and eventual binary download (placeholder)
3. **Quick start** — `am init` → `am start feat --agent claude` → `am attach feat` → `am clean feat`
4. **Configuration** — point to `config.md`; show a minimal `~/.config/am/config.toml`
5. **Supported agents** — table: claude, codex, copilot, gemini, aider
6. **Example Dockerfile** — a minimal Dockerfile that installs claude and works with `am`

### Error Message Audit

Review every user-facing error and status message for clarity and actionability:

- Each error should tell the user what went wrong AND what to do next
- `ContainerImageNotConfigured` → suggest setting `container.image` in config
- `ContainerRuntimeNotFound` → include Podman install URL
- `SlugAlreadyExists` → suggest `am clean <slug>` or `am attach <slug>`
- `NotInTmux` → explain that the command requires an active tmux session
- `SlugNotFound` → suggest `am list` to see valid slugs

### Cross-Platform Build Verification

Verify `cargo build --release` produces a working binary on:

- Linux x86_64
- macOS arm64 (Apple Silicon)
- macOS x86_64 (Intel)

Document any platform-specific build requirements or CI configuration needed.

## Tests

- Integration test: `am start` → `am list` → `am clean` full flow (no tmux, no container)

## Acceptance Criteria

- Integration test passes
- `README.md` exists and covers install, quick start, config, agents, and example Dockerfile
- All error messages include a suggested next action
- `cargo build --release` verified on Linux x86_64 (CI) and macOS arm64 (manual)
