# Configuration

`am` supports layered configuration so you can set machine-wide defaults in a global file, override them per-project, and further override specific values with environment variables or CLI flags at runtime. All layers are optional — if nothing is set, compiled-in defaults apply.

---

## Precedence order

Later entries win (highest precedence last):

1. **Compiled-in defaults** — built into the `am` binary; always present as a fallback
2. **Global config** (`~/.config/am/config.toml`) — machine-wide defaults for all projects
3. **Project config** (`.am/config.toml`) — per-repository overrides
4. **Environment variables** — `AM_*` variables override both config files; useful in CI or for one-off changes without editing files
5. **CLI flags** (`--agent`, `--no-container` on `am start`) — highest precedence; affect only the single invocation

---

## Global config

**Path:** `~/.config/am/config.toml`

The global config sets machine-wide defaults that apply to every project. It is loaded on every `am` invocation before the project config is applied.

Generate a fully-documented template and place it in the standard location:

```sh
mkdir -p ~/.config/am
am generate-config > ~/.config/am/config.toml
```

Open the file and uncomment any values you want to change from the compiled-in defaults. Lines that remain commented out have no effect — the compiled-in default is used instead.

---

## Project config

**Path:** `.am/config.toml` (relative to the repository root)

The project config overrides global defaults for a specific repository. It is created automatically by `am init`. All lines are commented out by default so that global defaults flow through unchanged — uncomment only the keys you actually want to override.

```sh
# Initialize a project (creates .am/config.toml and .am/sessions.json)
am init
```

A minimal project config that sets the agent looks like this:

```toml
[defaults]
agent = "claude"
```

Selecting the agent also selects the container image — `am` ships with built-in image defaults for `claude` and `copilot`. You do not need to configure the image separately unless you are using a custom one.

---

## Environment variables

Environment variables override both the global and project configs and are useful for CI pipelines, Docker-in-Docker setups, or temporary one-off overrides without editing any files. Unknown or malformed values are silently ignored.

| Variable | Config key | Values | Example |
|---|---|---|---|
| `AM_VCS` | `defaults.vcs` | `git`, `jj` | `AM_VCS=jj` |
| `AM_AGENT` | `defaults.agent` | any non-empty string | `AM_AGENT=claude` |
| `AM_TMUX_AGENT_PANE` | `tmux.agent_pane` | `left`, `right` | `AM_TMUX_AGENT_PANE=right` |
| `AM_TMUX_SPLIT` | `tmux.split` | `horizontal`, `vertical` | `AM_TMUX_SPLIT=vertical` |
| `AM_TMUX_SPLIT_PERCENT` | `tmux.split_percent` | integer 1–99 | `AM_TMUX_SPLIT_PERCENT=30` |
| `AM_CONTAINER_ENABLED` | `container.enabled` | `true`/`1`/`yes`, `false`/`0`/`no` | `AM_CONTAINER_ENABLED=false` |
| `AM_CONTAINER_RUNTIME` | `container.runtime` | `auto`, `podman`, `docker` | `AM_CONTAINER_RUNTIME=docker` |
| `AM_CONTAINER_IMAGE` | `container.image` | any non-empty string | `AM_CONTAINER_IMAGE=my-image:latest` — overrides the image for all agents |
| `AM_CONTAINER_NETWORK` | `container.network` | `full`, `none` | `AM_CONTAINER_NETWORK=none` |
| `AM_CONTAINER_STARTUP_DELAY_MS` | `container.startup_delay_ms` | non-negative integer | `AM_CONTAINER_STARTUP_DELAY_MS=1000` |

---

## CLI flags

The `am start` command accepts flags that act as the highest-precedence overrides for a single session:

| Flag | Description |
|---|---|
| `--agent <AGENT>` | Override the agent command for this session only. Must be a known agent integration: `claude`, `copilot`, `gemini`, `codex`, `aider`. |
| `--no-container` | Disable container isolation for this session. The agent runs directly in the tmux pane. |

---

## Settings reference

### `[defaults]`

Top-level defaults that apply across all sessions unless overridden.

| Key | Type | Default | Description | Valid Values |
|---|---|---|---|---|
| `vcs` | string | `"git"` | Version control system used to create worktrees or workspaces | `"git"`, `"jj"` |
| `agent` | string | `""` | Default agent launched in the agent pane; also selects the container image via `[agents.<name>]`; empty means no agent is auto-launched | Any known agent name, e.g. `"claude"`, `"copilot"` |

### `[agents.<name>]`

Per-agent configuration. `am` ships with compiled-in image defaults for `claude` and `copilot`; define an entry here to override them or to add images for other agents.

| Key | Type | Default | Description |
|---|---|---|---|
| `image` | string | see below | Container image to use when this agent is selected |

**Compiled-in defaults:**

| Agent | Default image |
|---|---|
| `claude` | `ghcr.io/dstanek/am-claude-minimal:latest` |
| `copilot` | `ghcr.io/dstanek/am-copilot-minimal:latest` |

Example — override the claude image and add a gemini entry:

```toml
[agents.claude]
image = "my-org/am-claude:v2"

[agents.gemini]
image = "my-org/am-gemini:latest"
```

Agent entries are **merged** across config layers: global config entries extend the compiled-in defaults, and project config entries extend the global ones. Only keys you set in a later layer are overridden — other agents keep their values from earlier layers.

### `[tmux]`

Controls how the tmux window and panes are arranged for each session.

| Key | Type | Default | Description | Valid Values |
|---|---|---|---|---|
| `agent_pane` | string | `"left"` | Which pane receives the agent command after the split | `"left"`, `"right"` |
| `split` | string | `"horizontal"` | Direction of the tmux pane split | `"horizontal"`, `"vertical"` |
| `split_percent` | integer | `50` | Percentage of the window given to the first (agent) pane | 1–99 |

### `[container]`

Controls container lifecycle and what gets mounted or exposed inside the container.

| Key | Type | Default | Description | Valid Values |
|---|---|---|---|---|
| `enabled` | boolean | `true` | Whether to run sessions inside a container | `true`, `false` |
| `runtime` | string | `"auto"` | Container runtime to use; `"auto"` tries Podman first, then Docker | `"auto"`, `"podman"`, `"docker"` |
| `image` | string | `""` | Override image for all agents; takes priority over `[agents.<name>].image`; leave unset to use the per-agent default | Any valid image reference |
| `network` | string | `"full"` | Network access mode for the container | `"full"` (unrestricted internet access), `"none"` (no network) |
| `env` | list of strings | `[]` | Extra environment variables passed into the container from the host shell | e.g. `["ANTHROPIC_API_KEY", "FOO=bar"]` |
| `startup_delay_ms` | integer | `500` | Milliseconds to wait after container start before sending the agent command to the pane | Any non-negative integer |

!!! tip "Choosing `startup_delay_ms`"
    The default 500 ms is usually enough for a pre-pulled image on a local machine. If you are pulling a large image or running on a slow host, increase this value (e.g. `startup_delay_ms = 2000`) to avoid sending the agent command before the container shell is ready.

!!! note "Image selection"
    In most cases you do not need to set `container.image`. `am` resolves the image from the active agent via `[agents.<name>].image`, with built-in defaults for `claude` and `copilot`. Set `container.image` only when you want a single image to apply regardless of which agent is selected.
