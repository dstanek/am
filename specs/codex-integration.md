# Feature 7: Codex Integration

Auth preset for OpenAI Codex — env-var only, no filesystem mount.

## Background

`am` supports named agent presets that configure credential access for known agents. Most
presets mount a credential directory into the container. Codex is different: it authenticates
entirely via the `OPENAI_API_KEY` environment variable, so no filesystem mount is needed.

The existing preset mechanism in `container.rs` already handles filesystem mounts. This
feature extends it to support env-var-only presets.

## Spec

### Auth Preset Table Entry

| Preset key | Host path | Container path | Notes |
|---|---|---|---|
| `codex` | _(none — env var only)_ | — | Uses `OPENAI_API_KEY` pass-through |

### Behaviour

- `resolve_agent_auth_mount("codex")` returns `None` (no filesystem mount)
- When the `codex` preset is active, `OPENAI_API_KEY` is automatically added to the env
  pass-through list (as if the user had added it to `container.env`)
- `build_run_command` must include `-e OPENAI_API_KEY` in the generated command

### Agent launch

`config.agent = "codex"` (or `--agent codex`) activates the preset. When running inside
tmux, the container command is sent to the agent pane via `send_keys`; outside tmux the
process is replaced via `exec`.

## Design

- No mount needed; auth via `OPENAI_API_KEY` env var pass-through
- `config.agent = "codex"` ensures `OPENAI_API_KEY` is included in env pass-through
- Validation: `validate_agent("codex")` must pass (not rejected as unknown)

## Tests

- `resolve_agent_auth_mount("codex")` returns `None`
- `build_run_command` includes `-e OPENAI_API_KEY` when codex preset is active
- `am start feat --agent codex` sends correct container command to agent pane
- `validate_agent("codex")` does not error

## Implementation

1. Add `codex` branch in `resolve_agent_auth_mount()` returning `None`
2. Add env-var injection logic for env-var-only presets: when `agent_preset == "codex"`,
   append `"OPENAI_API_KEY"` to the effective env pass-through list before building the
   run command
3. Ensure `validate_agent()` accepts `"codex"` as a valid value

## Acceptance Criteria

- `am start feat --agent codex` passes `OPENAI_API_KEY` into the container
- No spurious mount errors or warnings about missing credential directories
- `am list` shows `codex` as the agent for the session
