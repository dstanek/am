# Commands

Complete reference for all `am` commands.

---

## `am init`

Initialize a new `am` project in the current repository.

**Usage**

```sh
am init
```

**What it does**

- Creates the `.am/` directory at the repository root
- Writes a default `.am/config.toml` with all settings commented out
- Creates an empty `.am/sessions.json` to hold session state
- Appends `.am/` to `.gitignore` (creates `.gitignore` if it does not exist)

Running `am init` in a directory that is not a git or jj repository is an error. Running it a second time in the same repository is safe — existing files are not overwritten.

!!! note
    `am init` must be run from inside a git or jj repository. `am` detects `.jj/` first; if not found it checks for `.git/`. If neither is present, the command exits with an error.

---

## `am start <slug>`

Create a new isolated agent session.

**Usage**

```sh
am start <slug> [OPTIONS]
```

**Arguments**

| Argument | Description |
|---|---|
| `<slug>` | Session name. Must be 1–40 characters, using only lowercase letters (`a–z`), digits (`0–9`), hyphens (`-`), and underscores (`_`). |

**Options**

| Option | Description |
|---|---|
| `--agent <AGENT>` | Agent command to launch in the session's agent pane. Overrides the `agent` value from config. Must be one of the built-in agent integrations: `claude`, `copilot`, `gemini`, `codex`, `aider`. |
| `--no-container` | Disable container isolation for this session. The agent command will run directly in the tmux pane instead of inside a container. |

**What it does**

1. Validates the slug
2. Creates a git worktree at `.am/worktrees/<slug>` on a new `am/<slug>` branch (or a jj workspace if the repo uses jj)
3. If inside tmux: opens a new window named `am-<slug>` with a split pane; sets up the agent pane and the shell pane
4. If container is enabled: launches the container with the appropriate mounts and environment variables
5. Sends the agent command to the agent pane after the configured startup delay
6. Records the session in `.am/sessions.json`

If `am start` is run outside of tmux, it creates the worktree and then launches the container directly (replacing the current shell process via `exec()`). No tmux window is created.

---

## `am list`

List all active sessions for the current project.

**Usage**

```sh
am list
```

Reads from `.am/sessions.json` and prints a table of all recorded sessions. If there are no sessions, prints a friendly message instead.

**Output columns**

| Column | Description |
|---|---|
| `SLUG` | The session name |
| `AGENT` | The agent command configured for this session |
| `WINDOW` | The tmux window name (`am-<slug>`) |
| `CREATED` | Human-readable relative time since the session was created |

**Example output**

```
SLUG     AGENT    WINDOW       CREATED
feat     claude   am-feat      12 min ago
bugfix   claude   am-bugfix    3 min ago
```

---

## `am attach <slug>`

Attach to an existing session's tmux window.

**Usage**

```sh
am attach <slug>
```

Switches the current tmux client to the `am-<slug>` window. If the window does not exist (for example, after a system restart), `am attach` creates a new window and split for the session — it does not error.

!!! warning "Requires tmux"
    `am attach` must be run from inside a tmux session. If `$TMUX` is not set, the command exits with an error. To get a terminal inside an existing session without tmux, navigate directly to `.am/worktrees/<slug>`.

---

## `am run <slug> <agent>`

Send an agent command to a session's agent pane.

**Usage**

```sh
am run <slug> <agent>
```

Uses `tmux send-keys` to send the specified agent command to the agent pane of the `am-<slug>` window, followed by Enter. This is useful for (re)starting an agent in a session that was started without one, or after the agent process has exited.

**Example**

```sh
am run feat claude
```

!!! warning "Requires tmux"
    `am run` must be run from inside a tmux session. If `$TMUX` is not set, the command exits with an error.

---

## `am clean <slug>`

Stop and remove an agent session.

**Usage**

```sh
am clean <slug> [OPTIONS]
```

**Options**

| Option | Description |
|---|---|
| `--force`, `-f` | Skip the confirmation prompt and proceed immediately. |

**What it does**

1. Stops the container (`podman stop am-<slug>` or equivalent)
2. Removes the container (`podman rm am-<slug>` or equivalent)
3. Kills the tmux window `am-<slug>` (skipped if the window no longer exists)
4. Removes the git worktree at `.am/worktrees/<slug>` and deletes the `am/<slug>` branch
5. Removes the session record from `.am/sessions.json`

Without `--force`, `am` prints a summary of what will be removed and asks for confirmation. This is the only destructive command in `am` and cannot be undone — the worktree and branch are permanently deleted.

---

## `am generate-config`

Print a fully-documented configuration template to stdout.

**Usage**

```sh
am generate-config
am generate-config > ~/.config/am/config.toml
```

Prints a complete `config.toml` template with every supported setting, its default value, and an explanatory comment. All settings are commented out so that the compiled-in defaults apply unless explicitly uncommented.

Useful for seeding either the global config or a project config:

```sh
# Create global config
mkdir -p ~/.config/am
am generate-config > ~/.config/am/config.toml

# Create project config (am init does this automatically)
am generate-config > .am/config.toml
```

---

## Slug validation

Slugs are the short names used to identify sessions. The following rules apply:

- **Length:** 1–40 characters
- **Characters:** only lowercase letters (`a–z`), digits (`0–9`), hyphens (`-`), and underscores (`_`)
- **Pattern:** `[a-z0-9_-]{1,40}`

Slugs that do not match these rules are rejected immediately by `am start` with an error message, before any side effects occur.

**Valid examples**

```
feat
fix-auth
my_feature
v2
release-2026-03
```

**Invalid examples**

```
MyFeature       # uppercase letters not allowed
fix auth        # spaces not allowed
-leading-dash   # must start with a letter or digit
```
