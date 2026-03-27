# am Configuration

`am` supports layered configuration so you can set machine-wide defaults in a global file, override them per-project, and further override specific values with environment variables or CLI flags at runtime. All layers are optional — if nothing is set, compiled-in defaults apply.

## Precedence Order

Later entries win (highest precedence last):

1. Compiled-in defaults
2. Global config (`~/.config/am/config.toml`)
3. Project config (`.am/config.toml`)
4. Environment variables
5. CLI flags (`--agent`, `--no-container` on `am start`)

## Global Config

**Path:** `~/.config/am/config.toml`

The global config sets machine-wide defaults for all projects. It is loaded on every `am` invocation before the project config is applied.

To generate a fully documented template with all options and their defaults:

```sh
mkdir -p ~/.config/am
am generate-config > ~/.config/am/config.toml
```

Edit the file to uncomment and adjust whichever values you want to change from the compiled-in defaults.

## Project Config

**Path:** `.am/config.toml` (relative to the repository root)

The project config overrides global defaults for a specific repository. It is created automatically by `am init`. All lines are commented out by default so that global defaults flow through unchanged — uncomment only the keys you actually want to override.

To initialize a project:

```sh
am init
```

This creates `.am/config.toml` with a fully-commented template alongside `.am/sessions.json`.

## Environment Variables

Environment variables override both the global and project configs and are useful for CI or temporary one-off changes.

| Variable | Config key | Values | Example |
|---|---|---|---|
| `AM_VCS` | `defaults.vcs` | `git`, `jj` | `AM_VCS=jj` |
| `AM_AGENT` | `defaults.agent` | any non-empty string | `AM_AGENT=claude` |
| `AM_TMUX_AGENT_PANE` | `tmux.agent_pane` | `left`, `right` | `AM_TMUX_AGENT_PANE=right` |
| `AM_TMUX_SPLIT` | `tmux.split` | `horizontal`, `vertical` | `AM_TMUX_SPLIT=vertical` |
| `AM_TMUX_SPLIT_PERCENT` | `tmux.split_percent` | integer 1–99 | `AM_TMUX_SPLIT_PERCENT=30` |
| `AM_CONTAINER_ENABLED` | `container.enabled` | `true`/`1`/`yes`, `false`/`0`/`no` | `AM_CONTAINER_ENABLED=false` |
| `AM_CONTAINER_RUNTIME` | `container.runtime` | `auto`, `podman`, `docker` | `AM_CONTAINER_RUNTIME=docker` |
| `AM_CONTAINER_IMAGE` | `container.image` | any non-empty string | `AM_CONTAINER_IMAGE=ubuntu:24.04` |
| `AM_CONTAINER_AGENT` | `container.agent` | any non-empty string | `AM_CONTAINER_AGENT=codex` |
| `AM_CONTAINER_NETWORK` | `container.network` | `full`, `none` | `AM_CONTAINER_NETWORK=none` |
| `AM_CONTAINER_STARTUP_DELAY_MS` | `container.startup_delay_ms` | non-negative integer | `AM_CONTAINER_STARTUP_DELAY_MS=1000` |

Unknown or malformed values are silently ignored.

## CLI Flags

The `am start` command accepts flags that act as the highest-precedence overrides for a single session:

- `--agent <AGENT>` — override the agent command for this session
- `--no-container` — disable container isolation for this session

## Settings Reference

### `[defaults]`

| Key | Type | Default | Description | Valid Values |
|---|---|---|---|---|
| `vcs` | string | `"git"` | Version control system used to create worktrees | `"git"`, `"jj"` |
| `agent` | string | `""` | Default agent command launched in the agent pane | Any executable name, e.g. `"claude"`, `"codex"`, `"gemini"` |

### `[tmux]`

| Key | Type | Default | Description | Valid Values |
|---|---|---|---|---|
| `agent_pane` | string | `"left"` | Which pane receives the agent command | `"left"`, `"right"` |
| `split` | string | `"horizontal"` | Direction of the tmux pane split | `"horizontal"`, `"vertical"` |
| `split_percent` | integer | `50` | Percentage of the window given to the agent pane | 1–99 |

### `[container]`

| Key | Type | Default | Description | Valid Values |
|---|---|---|---|---|
| `enabled` | boolean | `true` | Whether to run sessions inside a container | `true`, `false` |
| `runtime` | string | `"auto"` | Container runtime to use (`"auto"` tries podman first, then docker) | `"auto"`, `"podman"`, `"docker"` |
| `image` | string | `"ubuntu:25.10"` | Container image to run; must be set when `enabled = true` | Any valid image reference |
| `agent` | string | `""` | Agent for containers; overrides `defaults.agent` in container context | Any executable name |
| `network` | string | `"full"` | Network access mode for the container | `"full"` (unrestricted), `"none"` (no network) |
| `env` | list of strings | `[]` | Extra environment variables passed into the container | e.g. `["FOO=bar", "BAZ=qux"]` |
| `startup_delay_ms` | integer | `500` | Milliseconds to wait after container start before sending the agent command | Any non-negative integer |
