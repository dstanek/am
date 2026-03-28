## [0.1.1] - 2026-03-28

### 🐛 Bug Fixes

- *(release)* Fix macos-13 runner deprecation and deb copyright warning
- *(main)* Fix Windows build by gating unix-only exec() behind cfg(unix)
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
