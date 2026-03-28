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

`am` mounts your GitHub credentials from the host at runtime, so nothing sensitive is baked into the image. The `dockerfiles/Dockerfile.copilot` file included in this repository installs the GitHub CLI and the Copilot CLI package:

```dockerfile
FROM ubuntu:25.10

ENV DEBIAN_FRONTEND=noninteractive

RUN <<EOF
set -e

apt-get update && apt-get install -y \
    ca-certificates curl wget gnupg git build-essential \
    python3 python3-pip python3-venv ripgrep fd-find jq unzip less neovim ssh
ln -s /usr/bin/fdfind /usr/local/bin/fd

curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
    | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
    > /etc/apt/sources.list.d/github-cli.list
apt-get update && apt-get install -y gh

curl -fsSL https://deb.nodesource.com/setup_lts.x | bash -
apt-get install -y nodejs

npm install -g @github/copilot

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
docker build -f dockerfiles/Dockerfile.copilot -t am-copilot:latest .

# Podman
podman build -f dockerfiles/Dockerfile.copilot -t am-copilot:latest .
```

---

## Project configuration

Set the image and agent in `.am/config.toml`:

```toml
[container]
image = "am-copilot:latest"
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
am clean feat --force
am start feat --agent copilot
```

**Two credential directories are mounted**

Unlike some agents that use a single credentials file, Copilot requires both `~/.config/gh` (for the GitHub OAuth token) and `~/.config/github-copilot` (for Copilot-specific configuration). If either directory is missing from your host, the agent will fail to authenticate. Both are created when you run `gh auth login` on a machine with Copilot access.

**Customizing the image**

The included Dockerfile installs Node.js 22+ (required by `@github/copilot`), the GitHub CLI, and common developer tools. Add your project's language runtimes or toolchain to the `apt-get install` block as needed.
