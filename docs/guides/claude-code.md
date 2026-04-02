# Claude Code

Claude Code is the recommended agent for use with `am`. This guide covers building a container image, configuring `am`, and running your first session.

---

## Prerequisites

- **Anthropic credentials** — either set `ANTHROPIC_API_KEY` in your environment, or authenticate interactively with `claude auth login` on your host machine before starting a session.
- **`~/.claude/` must exist on the host** — `am` mounts this directory into the container at runtime. If you have not yet run `claude auth login`, do so now and verify that `~/.claude/` is present.

---

## Container image

`am` mounts `~/.claude` from your host into the container as read-only at runtime. This means your credentials are never baked into the image — you can share or publish the image safely.

### Pull the pre-built image (recommended)

A ready-to-use image is published to the GitHub Container Registry:

```sh
# Docker
docker pull ghcr.io/dstanek/am-claude:latest

# Podman
podman pull ghcr.io/dstanek/am-claude:latest
```

!!! tip "Pin to a specific version"
    For reproducible environments, replace `:latest` with a specific release tag (e.g. `ghcr.io/dstanek/am-claude:0.2.0`). Check the [releases page](https://github.com/dstanek/am/releases) for available tags.

### Build from source

If you need to add project-specific tools, the Dockerfiles in this repository are a good starting point:

- `dockerfiles/Dockerfile.claude` — full image with git, jj, ripgrep, jq, neovim, and common dev tools
- `dockerfiles/Dockerfile.claude-minimal` — minimal image with only git and the Claude binary; use this as a base for project-specific images

The minimal image:

```dockerfile
FROM ubuntu:25.10

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    ca-certificates curl git \
 && rm -rf /var/lib/apt/lists/*

RUN userdel -r ubuntu 2>/dev/null || true \
 && useradd -m -u 1000 -s /bin/bash am \
 && mkdir -p /workspace && chown am:am /workspace

USER am
ENV HOME=/home/am

RUN curl -fsSL https://claude.ai/install.sh | bash

ENV PATH="/home/am/.local/bin:${PATH}"
ENV DISABLE_AUTOUPDATER=1

WORKDIR /workspace
```

Build it with Docker or Podman from the repository root:

```sh
# Docker
docker build -f dockerfiles/Dockerfile.claude -t am-claude:latest .

# Podman
podman build -f dockerfiles/Dockerfile.claude -t am-claude:latest .
```

The build takes a few minutes on the first run. Subsequent builds are fast if the base layer is cached.

---

## Project configuration

Set the agent in `.am/config.toml`:

```toml
[defaults]
agent = "claude"
```

`am` automatically selects the container image based on the agent — the built-in default for `claude` is `ghcr.io/dstanek/am-claude-minimal:latest`. With `agent` set in config you do not need to pass `--agent claude` on every `am start` invocation.

To use a different image (e.g. the full image or a custom one), override it under `[agents.claude]`:

```toml
[defaults]
agent = "claude"

[agents.claude]
image = "ghcr.io/dstanek/am-claude:latest"
```

---

## Starting a session

If `agent` is set in config:

```sh
am start feat
```

Or override the config with an explicit flag:

```sh
am start feat --agent claude
```

`am` will:

1. Create the `am/feat` git worktree (or jj workspace)
2. Open a split-pane tmux window named `am-feat`
3. Launch the container in the agent pane with all credential mounts
4. After a brief startup pause, send `claude` to the agent pane to start Claude Code

---

## What gets mounted

`am` automatically mounts the following paths when the `claude` agent integration is active:

| Host path | Container path | Mode |
|---|---|---|
| `.am/worktrees/<slug>` | `/workspace` | read-write |
| `<repo-root>/.git` | `/mainrepo/.git` | read-write |
| `~/.gitconfig` | `/root/.gitconfig` | read-only |
| `~/.ssh` | `/root/.ssh` | read-only |
| `~/.claude` | `/root/.claude` | read-only |

`am` also injects the `GIT_DIR` and `GIT_WORK_TREE` environment variables into the container so that `git` commands issued from `/workspace` operate correctly against the worktree. From inside the container, running `git status` or `git commit` will behave as if you are in a normal checkout of the `am/<slug>` branch.

!!! note "jj repositories"
    For jj repositories, `am` uses a different mount layout that mirrors the host path structure. `GIT_DIR`/`GIT_WORK_TREE` are not injected — jj does not use them. The container's working directory is still set to the worktree path.

---

## Tips

**Claude Code is not authenticated yet**

If you see an authentication error when the container starts, run `claude auth login` on your host machine first, then restart the session:

```sh
am destroy feat --force
am start feat --agent claude
```

**Passing `ANTHROPIC_API_KEY` directly**

If you prefer to authenticate via API key rather than the `~/.claude` credential store, add it to the container's environment in `.am/config.toml`:

```toml
[defaults]
agent = "claude"

[container]
env = ["ANTHROPIC_API_KEY"]
```

This passes the value of `ANTHROPIC_API_KEY` from your current shell environment into the container. The key is never stored anywhere on disk inside the container.

**Customizing the image**

Use `dockerfiles/Dockerfile.claude-minimal` as a base and layer your project's language runtimes and tools on top. The only hard requirement is that `claude` is on the container's `PATH`. See the [Custom Container Images](custom-images.md) guide for patterns and ready-to-use examples.
