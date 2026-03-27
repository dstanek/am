# Aider Integration

Auth preset for Aider — env-var only, no filesystem mount.

## Background

`am` supports named agent presets. Aider authenticates via environment variables
(`ANTHROPIC_API_KEY` or `OPENAI_API_KEY` depending on which model backend is used).
No filesystem credential directory is needed.

This is similar to the Codex preset but Aider supports two possible API keys.

## Spec

### Auth Preset Table Entry

| Preset key | Host path | Container path | Notes |
|---|---|---|---|
| `aider` | _(none — env var only)_ | — | Uses `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` |

### Behaviour

- `resolve_agent_auth_mount("aider")` returns `None` (no filesystem mount)
- When the `aider` preset is active, both `ANTHROPIC_API_KEY` and `OPENAI_API_KEY` are
  automatically added to the env pass-through list (only keys that are actually set in the
  host environment will be passed through; unset vars are silently skipped)
- `build_run_command` includes `-e ANTHROPIC_API_KEY -e OPENAI_API_KEY` in the generated
  command

### Agent launch

`config.agent = "aider"` (or `--agent aider`) activates the preset. After the container
starts, a second `send_keys` fires after `startup_delay_ms` to run `aider` in the agent pane.

## Design

- No mount needed; auth via env var pass-through only
- Both `ANTHROPIC_API_KEY` and `OPENAI_API_KEY` are injected (Aider uses whichever is set)
- `validate_agent("aider")` must pass

## Tests

- `resolve_agent_auth_mount("aider")` returns `None`
- `build_run_command` includes `-e ANTHROPIC_API_KEY` and `-e OPENAI_API_KEY` when aider
  preset is active
- `am start feat --agent aider` sends correct container command to agent pane
- `validate_agent("aider")` does not error

## Implementation

1. Add `aider` branch in `resolve_agent_auth_mount()` returning `None`
2. When `agent_preset == "aider"`, append both `"ANTHROPIC_API_KEY"` and `"OPENAI_API_KEY"`
   to the effective env pass-through list before building the run command
3. Ensure `validate_agent()` accepts `"aider"` as a valid value

## Acceptance Criteria

- `am start feat --agent aider` passes both API key vars into the container
- No spurious mount errors
- `am list` shows `aider` as the agent for the session
