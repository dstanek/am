# Feature 8: Gemini Integration

Auth mount preset for Google Gemini CLI.

## Background

`am` supports named agent presets that configure credential access for known agents. This
feature adds the `gemini` preset, which mounts `~/.gemini` into the container read-only
so the Gemini CLI can authenticate without any user intervention.

## Spec

### Auth Preset Table Entry

| Preset key | Host path | Container path | Notes |
|---|---|---|---|
| `gemini` | `~/.gemini` | `/root/.gemini` | Gemini CLI |

### Behaviour

- `resolve_agent_auth_mount("gemini")` returns an `AgentAuthMount` with:
  - `host_path`: `~/.gemini` (expanded to absolute path)
  - `container_path`: `/root/.gemini`
  - Mount mode: read-only
- `build_run_command` includes `-v /home/user/.gemini:/root/.gemini:ro,z` (with `,z` on
  Linux + Podman per the existing SELinux label rule)

### Agent launch

`config.agent = "gemini"` (or `--agent gemini`) activates the preset. When running inside
tmux, the container command is sent to the agent pane via `send_keys`; outside tmux the
process is replaced via `exec`.

## Design

- Preset: `~/.gemini:/root/.gemini:ro`
- `config.agent = "gemini"` activates the preset
- Validation: `validate_agent("gemini")` must pass

## Tests

- `resolve_agent_auth_mount("gemini")` returns correct host/container paths
- `build_run_command` includes the gemini mount when preset is active
- `am start feat --agent gemini` sends correct container command with the mount
- `validate_agent("gemini")` does not error

## Implementation

1. Add `gemini` branch in `resolve_agent_auth_mount()` returning an `AgentAuthMount`
   with `host_path = ~/.gemini` and `container_path = /root/.gemini`
2. Ensure `validate_agent()` accepts `"gemini"` as a valid value

## Acceptance Criteria

- `am start feat --agent gemini` launches a container with `~/.gemini` mounted read-only
- The mount appears correctly in the `podman run` / `docker run` command
- No errors if `~/.gemini` exists; graceful handling if it doesn't (warn, don't fail)
