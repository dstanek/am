# Custom Container Images

`am` works with any container image — the images published by this project are a convenient starting point, not a requirement. A **custom image** lets you bake in exactly the tools your project needs so every agent session starts in a fully-prepared environment without any extra setup.

---

## When to use a custom image

The stock `am-claude` and `am-copilot` images contain a general-purpose set of tools (git, Node.js, ripgrep, jq, etc.). They are sufficient for many projects, but you will want a custom image when:

- Your project requires a specific language runtime (Rust, Go, Python, JVM, …) or version
- You need project-specific tools (linters, build systems, test runners)
- You want to pre-cache heavy dependencies so agent sessions start faster
- You need precise control over what is installed in the agent's environment

Custom images are recommended for any project where agents will actually build, test, or run code.

---

## The only hard requirement

Whatever base image you choose, **the agent binary must be present on the container's `PATH`**. Credentials are never baked in — `am` mounts them from your host at session start.

| Agent | Required binary |
|---|---|
| `claude` | `claude` |
| `copilot` | `gh` with the Copilot extension |
| `gemini` | `gemini` |
| `codex` | `codex` |
| `aider` | `aider` |

For unknown agent names (a raw command string), whatever binary you pass must exist in the image.

---

## Option 1: Extend an `am` image

The simplest approach is to layer your project's tooling on top of an existing `am` image. The base image already has the agent installed; you just add what your project needs.

The `am` project itself does this — `dockerfiles/Dockerfile.am-dev` is used to develop and test `am` inside a Claude Code container:

```dockerfile
FROM am-claude:latest

# Build tools needed for git2's vendored libgit2 + openssl
RUN <<EOF
set -e
apt-get update && apt-get install -y \
    cmake \
    perl \
    pkg-config \
    libssl-dev
rm -rf /var/lib/apt/lists/*
EOF

# Install Rust via rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

# Configure git identity — required by tests that run `git commit`
RUN git config --global user.email "dev@example.com" \
 && git config --global user.name "Dev"

# Pre-fetch dependencies so incremental builds are fast
WORKDIR /workspace
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src tests \
 && echo 'fn main() {}' > src/main.rs \
 && echo 'fn main() {}' > tests/cucumber.rs \
 && cargo fetch \
 && rm -rf src tests
```

A few patterns worth noting:

- **Layer on top of the agent image** — you get jj, git, and Claude Code for free.
- **Pre-fetch dependencies** — copy only the manifest files and run your package manager's fetch/download step. The source tree is mounted at runtime, so the cached layers survive rebuilds.
- **Set any globals that tests need** — here, `git config --global` provides the commit identity that `cargo test` requires.

Build it with:

```sh
make build-am-dev   # uses Podman or Docker automatically

# or directly:
podman build -f dockerfiles/Dockerfile.am-dev -t am-dev:latest .
docker build -f dockerfiles/Dockerfile.am-dev -t am-dev:latest .
```

---

## Option 2: Build from scratch

You do not have to use an `am` base image at all. Start from any image you like, install your language toolchain and project tools, then install the agent on top.

Here is a minimal example for a Python project using Claude Code:

```dockerfile
FROM python:3.12-slim

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    curl gnupg git ripgrep \
 && curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
 && apt-get install -y nodejs \
 && npm install -g @anthropic-ai/claude-code \
 && rm -rf /var/lib/apt/lists/*

# Install project dependencies
COPY requirements.txt /tmp/requirements.txt
RUN pip install --no-cache-dir -r /tmp/requirements.txt

WORKDIR /workspace
```

The key steps are:
1. Start from whatever base image suits your project.
2. Install the agent binary (here, `@anthropic-ai/claude-code` via npm).
3. Pre-install project dependencies against a static manifest file (not the live source tree, which is mounted at runtime).

---

## Configuring `am` to use your image

Set `image` (and optionally `agent`) in `.am/config.toml`:

```toml
[container]
image = "am-dev:latest"
agent = "claude"
```

Or pass it per-invocation:

```sh
am start my-feature --image am-dev:latest --agent claude
```

The `agent` key tells `am` which credential mount preset to use. It does not have to match the image name — an image named `am-dev` can still use the `claude` preset.

---

## Tips

**Keep credentials out of the image**

Never `RUN` an authentication command (`claude auth login`, `gh auth login`, etc.) inside a Dockerfile. `am` mounts credentials from your host at session start. Baking them into an image makes the image unsafe to share or publish.

**Rebuilding after dependency changes**

When you update `Cargo.toml`, `package.json`, `requirements.txt`, or similar manifests, rebuild the image to refresh the pre-cached layer. The image acts as a warm dependency cache — it is worth keeping up to date.

**Testing the image manually**

Before using an image with `am`, verify it works standalone:

```sh
podman run --rm -it am-dev:latest bash
# inside the container:
cargo --version
claude --version
```
