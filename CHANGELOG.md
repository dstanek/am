## [0.5.0] - 2026-04-16

### 🚀 Features

- *(container)* Make container user configurable
- *(container)* Implement OPENAI_API_KEY injection for Codex agent

### 🐛 Bug Fixes

- *(release)* Create Homebrew tap PR via REST API
- Address code review findings across multiple modules
- *(container)* Use Display instead of Debug for RuntimeKind serialization
- *(cli)* Enforce slug must start with a letter or digit
- *(tmux)* Validate AM_TMUX_BIN override exists before using it
- *(config)* Validate container.env entries at config load time

### 🚜 Refactor

- *(main)* Eliminate duplicated Session construction in cmd_start
- *(container)* Introduce KnownAgent enum for compile-time agent enforcement
- *(worktree)* Resolve git binary once per public function
- *(tmux,worktree)* Resolve binary once per public function
- *(command)* Add run_built_command variants; inline path args in run_git
- *(worktree)* Inline path args in git_worktree_has_changes

### 📚 Documentation

- Correct container mount layout description in CLAUDE.md and specs/v1.md
- Remove stale startup_delay_ms and GIT_DIR references from docs
- *(configuration)* Add missing container settings to reference table
- *(backlog)* Update Codex and Gemini integration status
- *(claude)* Add command.rs to Architecture module list
- *(specs)* Sync v1.md with current implementation

### 🎨 Styling

- *(error)* Remove extra blank line in error.rs
## [0.4.0] - 2026-04-12

### 🚀 Features

- *(config)* Respect XDG_CONFIG_HOME for global config path

### 🐛 Bug Fixes

- *(main)* Escape single quotes in cd commands sent via send_keys
- *(worktree)* Replace unwrap_or(".") with to_string_lossy() in run_git
- *(worktree)* Handle binary names in AM_GIT_BIN and AM_JJ_BIN

### 🚜 Refactor

- *(config)* Remove unimplemented startup_delay_ms option
- *(container)* Split validate_agent into name and credential checks
- *(tmux)* Change tmux_bin() return type from String to PathBuf
- *(main)* Merge detect_vcs into find_repo_root

### 📚 Documentation

- Fix container mount paths in concepts and claude-code guides
- Remove GIT_DIR/GIT_WORK_TREE injection claims and fix remaining stale paths
- *(commands)* Fix am list output columns and example
- *(commands)* Fix --agent valid values and add --auto to am start options
- *(release)* Add main branch guard to release command

### 🧪 Testing

- Add missing coverage for git_worktree_has_changes and gemini auth mount

### ⚙️ Miscellaneous Tasks

- Remove aider agent — no plans to implement
- Sync version to 0.3.0 and update installation docs
- *(worktree)* Make binary resolution robust in worktree copy
- *(release)* V0.4.0
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
- *(release)* V0.3.0
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
