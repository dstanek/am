## [0.3.0] - 2026-04-11

### 🚀 Features

- *(docker)* Add minimal images, example Dockerfiles, and publish both
- *(config)* Make gitconfig and ssh mount paths configurable
- *(container)* Skip mounting config files that don't exist on the host
- *(init,container)* Scaffold .am/gitconfig and mount it in containers
- *(start)* Require am init before starting a container session
- *(container)* Inject GH_TOKEN for Copilot agent via gh CLI
- *(tmux)* Cd shell pane into worktree on start, restore on destroy
- *(config)* Tie container image selection to agent name

### 🐛 Bug Fixes

- *(config)* Use fully qualified minimal image as default
- *(config)* Remove unnecessary unsafe blocks from env var calls in tests
- *(tmux,container)* Replace to_str().unwrap_or() with to_string_lossy()
- *(config)* Validate split_percent is in range 1-99
- *(container)* Improve error messages and use anyhow::Context for richer errors
- *(docker)* Pre-create ~/.config with correct ownership

### 💼 Other

- Add release profile optimizations for smaller binary size

### 🚜 Refactor

- Extract shared command execution logic into command module
- *(config)* Extract apply_opt helpers to reduce repetition in apply_file_config
- *(tmux)* Remove unused create_window_with_shell_cmd function
- *(session)* Split Session struct into VcsMetadata and TmuxMetadata sub-structs
- *(worktree)* Replace git2 library with git CLI subprocess calls

### 📚 Documentation

- Document path handling strategy in CLAUDE.md and source modules
- Add FAQ, troubleshooting, CI/CD integration, and quick-reference guides

### ⚙️ Miscellaneous Tasks

- *(docker)* Switch Claude image to native installer
## [0.2.0] - 2026-04-01

### 🚀 Features

- *(dockerfiles)* Install jj in agent container images
- *(ci)* Add workflow to publish container images to GHCR
- *(start)* Hide container command echo and clear pane on exit
- *(cli)* Add --version flag
- *(container)* Add project-specific dev image and custom images guide
- *(cli)* Add --auto flag for autonomous agent mode
- *(worktree)* Warn about uncommitted git changes before destroy
- *(tmux)* Rename current window instead of creating a new one

### 🐛 Bug Fixes

- *(ci)* Force actions to use Node 24 runtime
- *(container)* Mount .git for colocated jj+git repos
- *(main)* Traverse past jj workspaces when finding repo root
- *(worktree)* Improve error message for unborn HEAD in git repos
- *(tmux)* Launch container via send-keys instead of window shell cmd
- *(container)* Run agents as non-root user in containers
- *(destroy)* Preserve session record when worktree removal fails

### 🚜 Refactor

- *(cli)* Rename `clean` command to `destroy`

### 📚 Documentation

- Add MkDocs Material site with full documentation
- *(index)* Rewrite landing page to lead with problem, not features
- *(plan)* Document autonomous mode and team orchestration backlog items
- Add commit trailers reference page and update CLAUDE.md
- Consolidate outstanding work into BACKLOG.md
- Point to GHCR images for claude and copilot
- *(claude)* Require running tests after every code change
- *(claude)* Trim CLAUDE.md to reduce token usage
- *(quick-start)* Move prerequisites to the top

### 🧪 Testing

- *(config)* Guard file-only tests against env var contamination

### ⚙️ Miscellaneous Tasks

- Update repo references from dstanek/am to dstanek/agent-manager
- *(ci)* Add Clippy lint enforcement
- *(release)* V0.2.0
## [0.1.2] - 2026-03-28

### 🐛 Bug Fixes

- *(deps)* Vendor OpenSSL to fix cross-compilation on macOS

### ⚙️ Miscellaneous Tasks

- *(release)* Update release command for jj push workflow
- *(release)* V0.1.2
## [0.1.1] - 2026-03-28

### 🐛 Bug Fixes

- *(release)* Fix macos-13 runner deprecation and deb copyright warning
- *(main)* Fix Windows build by gating unix-only exec() behind cfg(unix)

### ⚙️ Miscellaneous Tasks

- *(release)* V0.1.1
## [0.1.0] - 2026-03-28

### 🚀 Features

- Initial scaffolding
- *(worktree)* Add git workspace integration
- *(tmux)* Add tmux integration
- *(container)* Add podman integration
- *(container)* Add claude code integration
- *(container)* Add docker integration
- *(container)* Add copilot integration
- *(worktree)* Add jj integration
- *(config)* Layered config with global file, env vars, and generate-config command
- *(ci)* Add release infrastructure with cargo-release and git-cliff
- *(install)* Add install script and instructions

### 🐛 Bug Fixes

- *(worktree)* Ensure clean removes the worktree
- *(container)* Use host paths for git mounts
- *(container)* Check existence before rm to avoid false-positive warning
- *(container)* Ensure invalid agents are caught early
- *(deps)* Use correct vendored-libgit2 feature for git2 crate
- *(build)* Resolve all compiler warnings
- *(release)* Remove invalid [workspace] section from release.toml
- *(release)* Allow release from detached HEAD for jj repos
- *(release)* Use login shell for pre-release-hook so git-cliff is on PATH
- *(release)* Use absolute path for git-cliff in pre-release-hook

### 💼 Other

- *(container)* Add Claude image Dockerfile

### 🚜 Refactor

- *(container)* Remove 'preset' terminology throughout

### 📚 Documentation

- Add initial project spec
- Add initial project plan
- *(plan)* Document known limitation
- Add CLAUDE.md with project guidance
- *(claude)* Use jj commit instead of jj describe for clean working copy
- *(specs)* Split SPEC.md into per-feature spec files

### 🧪 Testing

- *(integration)* Add Gherkin-based integration tests with cucumber

### ⚙️ Miscellaneous Tasks

- *(config)* Set ubuntu 25.10 as default image
- *(container)* Clean up error messages
- Remove notification requirement
- Add .am to the gitignore
- *(release)* V0.1.0
