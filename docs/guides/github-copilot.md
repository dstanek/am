# GitHub Copilot

This guide covers building a container image for GitHub Copilot, configuring `am`, and running your first session.

---

## Prerequisites

- **GitHub CLI authenticated on your host** — run `gh auth login` before starting a session and complete the OAuth flow.
- **`~/.config/gh/` and `~/.config/github-copilot/` must exist on the host** — `am` mounts both of these directories into the container at runtime. They are created automatically when you authenticate with `gh auth login`.

Verify that authentication is working on your host before proceeding:

```sh
gh auth status
```

---

## Container image

`am` mounts your GitHub credentials from the host at runtime, so nothing sensitive is baked into the image.

### Pull the pre-built image (recommended)

A ready-to-use image is published to the GitHub Container Registry:

```sh
# Docker
docker pull ghcr.io/dstanek/am-copilot:latest

# Podman
podman pull ghcr.io/dstanek/am-copilot:latest
```

!!! tip "Pin to a specific version"
    For reproducible environments, replace `:latest` with a specific release tag (e.g. `ghcr.io/dstanek/am-copilot:0.2.0`). Check the [releases page](https://github.com/dstanek/am/releases) for available tags.

### Build from source

If you need to add project-specific tools, the Dockerfiles in this repository are a good starting point:

- `dockerfiles/Dockerfile.copilot` — full image with git, jj, ripgrep, jq, neovim, and common dev tools
- `dockerfiles/Dockerfile.copilot-minimal` — minimal image with only git, the GitHub CLI, and the Copilot binary; use this as a base for project-specific images

The minimal image:

```dockerfile
FROM ubuntu:25.10

ENV DEBIAN_FRONTEND=noninteractive

RUN <<EOF
set -e
apt-get update && apt-get install -y ca-certificates curl git

curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
    | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
    > /etc/apt/sources.list.d/github-cli.list
apt-get update && apt-get install -y gh

curl -fsSL https://deb.nodesource.com/setup_lts.x | bash -
apt-get install -y nodejs
npm install -g @github/copilot

rm -rf /var/lib/apt/lists/*
EOF

RUN userdel -r ubuntu 2>/dev/null || true \
 && useradd -m -u 1000 -s /bin/bash am \
 && mkdir -p /workspace && chown am:am /workspace

USER am
ENV HOME=/home/am
WORKDIR /workspace
```

Build it with Docker or Podman from the repository root:

```sh
# Docker
docker build -f dockerfiles/Dockerfile.copilot -t am-copilot:latest .

# Podman
podman build -f dockerfiles/Dockerfile.copilot -t am-copilot:latest .
```

---

## Project configuration

Set the image and agent in `.am/config.toml`:

```toml
[container]
image = "ghcr.io/dstanek/am-copilot:latest"
agent = "copilot"
```

With `agent = "copilot"` in config, `am start` will automatically activate the Copilot agent integration and launch the agent without requiring any extra flags.

---

## Starting a session

```sh
am start feat --agent copilot
```

Or, with `agent = "copilot"` in `.am/config.toml`:

```sh
am start feat
```

`am` will:

1. Create the `am/feat` git worktree (or jj workspace)
2. Open a split-pane tmux window named `am-feat`
3. Launch the container in the agent pane with credential directories mounted
4. After a brief startup pause, send `copilot` to the agent pane

---

## What gets mounted

`am` automatically mounts the following paths when the `copilot` agent integration is active:

| Host path | Container path | Mode |
|---|---|---|
| `.am/worktrees/<slug>` | `/workspace` | read-write |
| `<repo-root>/.git` | `/mainrepo/.git` | read-write |
| `~/.gitconfig` | `/root/.gitconfig` | read-only |
| `~/.ssh` | `/root/.ssh` | read-only |
| `~/.config/gh` | `/root/.config/gh` | read-only |
| `~/.config/github-copilot` | `/root/.config/github-copilot` | read-only |

The Copilot agent integration mounts two credential directories: `~/.config/gh` holds the GitHub CLI authentication tokens, and `~/.config/github-copilot` stores Copilot-specific settings and cached tokens. Both are required for the agent to function.

`am` injects `GIT_DIR` and `GIT_WORK_TREE` into the container environment so that git operations from `/workspace` target the correct worktree branch.

!!! note "jj repositories"
    For jj repositories, the mount layout mirrors the host path structure instead. `GIT_DIR` and `GIT_WORK_TREE` are not set. The working directory inside the container is the jj workspace path.

---

## Tips

**Authentication not working inside the container**

If Copilot reports authentication errors when the session starts, check your host authentication first:

```sh
gh auth status
gh copilot --version
```

Re-authenticate on the host if needed (`gh auth login`), then restart the session:

```sh
am destroy feat --force
am start feat --agent copilot
```

**Two credential directories are mounted**

Unlike some agents that use a single credentials file, Copilot requires both `~/.config/gh` (for the GitHub OAuth token) and `~/.config/github-copilot` (for Copilot-specific configuration). If either directory is missing from your host, the agent will fail to authenticate. Both are created when you run `gh auth login` on a machine with Copilot access.

**Customizing the image**

Use `dockerfiles/Dockerfile.copilot-minimal` as a base and layer your project's language runtimes and tools on top. The only hard requirements are `gh` and the Copilot extension on the container's `PATH`. See the [Custom Container Images](custom-images.md) guide for patterns and ready-to-use examples.
