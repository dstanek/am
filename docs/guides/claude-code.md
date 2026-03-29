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

If you need to add project-specific tools, the `dockerfiles/Dockerfile.claude` file in this repository is a production-ready starting point:

```dockerfile
FROM ubuntu:25.10

ENV DEBIAN_FRONTEND=noninteractive

RUN <<EOF
set -e

apt-get update && apt-get install -y \
    ca-certificates curl wget gnupg git build-essential \
    python3 python3-pip python3-venv ripgrep fd-find jq unzip less neovim ssh
ln -s /usr/bin/fdfind /usr/local/bin/fd

curl -fsSL https://deb.nodesource.com/setup_lts.x | bash -
apt-get install -y nodejs

npm install -g @anthropic-ai/claude-code

JJ_VERSION=$(curl -fsSLI -o /dev/null -w '%{url_effective}' https://github.com/jj-vcs/jj/releases/latest | sed 's|.*/tag/||' | tr -d '[:space:]')
curl -fsSL "https://github.com/jj-vcs/jj/releases/download/${JJ_VERSION}/jj-${JJ_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
    | tar -xz -C /usr/local/bin

rm -rf /var/lib/apt/lists/*
EOF

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

Set the image and agent in `.am/config.toml`:

```toml
[container]
image = "ghcr.io/dstanek/am-claude:latest"
agent = "claude"
```

With `agent = "claude"` set in the config, you do not need to pass `--agent claude` on every `am start` invocation.

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
[container]
image = "ghcr.io/dstanek/am-claude:latest"
agent = "claude"
env = ["ANTHROPIC_API_KEY"]
```

This passes the value of `ANTHROPIC_API_KEY` from your current shell environment into the container. The key is never stored anywhere on disk inside the container.

**Customizing the image**

The included Dockerfile is a starting point. Add your project's language runtimes, linters, or build tools to the `apt-get install` block. The only hard requirement is that `claude` (the Claude Code CLI) is installed and available on the container's `PATH`.
