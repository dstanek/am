use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ── Enum types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Vcs {
    #[default]
    Git,
    Jj,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PaneSide {
    #[default]
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SplitDirection {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RuntimePreference {
    #[default]
    Auto,
    Podman,
    Docker,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NetworkMode {
    #[default]
    Full,
    None,
}

// ── Config structs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmuxConfig {
    pub agent_pane: PaneSide,
    pub split: SplitDirection,
    pub split_percent: u8,
}

impl Default for TmuxConfig {
    fn default() -> Self {
        Self {
            agent_pane: PaneSide::Left,
            split: SplitDirection::Horizontal,
            split_percent: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    pub enabled: bool,
    pub runtime: RuntimePreference,
    pub image: Option<String>,
    pub agent: Option<String>,
    pub network: NetworkMode,
    pub env: Vec<String>,
    pub startup_delay_ms: u64,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            runtime: RuntimePreference::Auto,
            image: None,
            agent: None,
            network: NetworkMode::Full,
            env: Vec::new(),
            startup_delay_ms: 500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub vcs: Vcs,
    pub agent: Option<String>,
    pub editor: Option<String>,
    pub tmux: TmuxConfig,
    pub container: ContainerConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vcs: Vcs::Git,
            agent: None,
            editor: None,
            tmux: TmuxConfig::default(),
            container: ContainerConfig::default(),
        }
    }
}

// ── TOML file shapes (partial overrides allowed) ──────────────────────────────

#[derive(Debug, Deserialize, Default)]
struct FileDefaults {
    vcs: Option<Vcs>,
    agent: Option<String>,
    editor: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct FileTmux {
    agent_pane: Option<PaneSide>,
    split: Option<SplitDirection>,
    split_percent: Option<u8>,
}

#[derive(Debug, Deserialize, Default)]
struct FileContainer {
    enabled: Option<bool>,
    runtime: Option<RuntimePreference>,
    image: Option<String>,
    agent: Option<String>,
    network: Option<NetworkMode>,
    env: Option<Vec<String>>,
    startup_delay_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
struct FileConfig {
    #[serde(default)]
    defaults: FileDefaults,
    #[serde(default)]
    tmux: FileTmux,
    #[serde(default)]
    container: FileContainer,
}

fn apply_file_config(base: &mut Config, file: FileConfig) {
    if let Some(v) = file.defaults.vcs {
        base.vcs = v;
    }
    if let Some(v) = file.defaults.agent {
        if !v.is_empty() {
            base.agent = Some(v);
        }
    }
    if let Some(v) = file.defaults.editor {
        if !v.is_empty() {
            base.editor = Some(v);
        }
    }
    if let Some(v) = file.tmux.agent_pane {
        base.tmux.agent_pane = v;
    }
    if let Some(v) = file.tmux.split {
        base.tmux.split = v;
    }
    if let Some(v) = file.tmux.split_percent {
        base.tmux.split_percent = v;
    }
    if let Some(v) = file.container.enabled {
        base.container.enabled = v;
    }
    if let Some(v) = file.container.runtime {
        base.container.runtime = v;
    }
    if let Some(v) = file.container.image {
        if !v.is_empty() {
            base.container.image = Some(v);
        }
    }
    if let Some(v) = file.container.agent {
        if !v.is_empty() {
            base.container.agent = Some(v);
        }
    }
    if let Some(v) = file.container.network {
        base.container.network = v;
    }
    if let Some(v) = file.container.env {
        base.container.env = v;
    }
    if let Some(v) = file.container.startup_delay_ms {
        base.container.startup_delay_ms = v;
    }
}

fn parse_config_file(path: &Path) -> Result<FileConfig> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading config file {}", path.display()))?;
    let file: FileConfig = toml::from_str(&text)
        .with_context(|| format!("parsing config file {}", path.display()))?;
    Ok(file)
}

/// Returns the global config path: `~/.config/am/config.toml`
pub fn global_config_path() -> Option<PathBuf> {
    dirs_path().map(|d| d.join("config.toml"))
}

fn dirs_path() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".config").join("am"))
}

/// Write the default config file at `path` (creates parent directories as needed).
pub fn write_defaults(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = r#"[defaults]
vcs = "git"            # "git" | "jj"
agent = ""             # default agent, e.g. "claude"
editor = ""            # default editor, e.g. "nvim"

[tmux]
agent_pane = "left"    # "left" | "right"
split = "horizontal"   # "horizontal" | "vertical"
split_percent = 50

[container]
enabled = true
runtime = "auto"       # "auto" | "podman" | "docker"
image = ""
agent = ""
network = "full"       # "full" | "none"
env = []
startup_delay_ms = 500
"#;
    std::fs::write(path, content)?;
    Ok(())
}

/// Load config by merging global → project.
/// If neither file exists, returns compiled-in defaults.
/// `override_global_path` is used in tests to inject an explicit global config path.
pub fn load(project_config_path: Option<&Path>) -> Result<Config> {
    load_with_global(global_config_path().as_deref(), project_config_path)
}

pub fn load_with_global(global_path: Option<&Path>, project_config_path: Option<&Path>) -> Result<Config> {
    let mut config = Config::default();

    // Apply global config if it exists
    if let Some(global_path) = global_path {
        if global_path.exists() {
            let file = parse_config_file(global_path)?;
            apply_file_config(&mut config, file);
        }
    }

    // Apply project config if provided and exists
    if let Some(path) = project_config_path {
        if path.exists() {
            let file = parse_config_file(path)?;
            apply_file_config(&mut config, file);
        }
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_toml(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn defaults_when_no_config_files() {
        let tmp = TempDir::new().unwrap();
        let nonexistent_global = tmp.path().join("global.toml");
        let nonexistent_project = tmp.path().join("project.toml");

        let config = load_with_global(Some(&nonexistent_global), Some(&nonexistent_project)).unwrap();

        assert_eq!(config.vcs, Vcs::Git);
        assert!(config.agent.is_none());
        assert_eq!(config.tmux.split_percent, 50);
        assert!(config.container.enabled);
        assert_eq!(config.container.runtime, RuntimePreference::Auto);
        assert!(config.container.image.is_none());
    }

    #[test]
    fn project_config_overrides_global() {
        let tmp = TempDir::new().unwrap();

        let global_path = write_toml(tmp.path(), "global.toml", r#"
[defaults]
agent = "codex"
[container]
image = "global-image"
"#);

        let project_path = write_toml(tmp.path(), "project.toml", r#"
[defaults]
agent = "claude"
[container]
image = "project-image"
"#);

        let config = load_with_global(Some(&global_path), Some(&project_path)).unwrap();

        assert_eq!(config.agent.as_deref(), Some("claude"));
        assert_eq!(config.container.image.as_deref(), Some("project-image"));
    }

    #[test]
    fn project_config_inherits_unset_global_fields() {
        let tmp = TempDir::new().unwrap();

        let global_path = write_toml(tmp.path(), "global.toml", r#"
[defaults]
agent = "claude"
[tmux]
split_percent = 70
"#);

        // Project config only sets image, doesn't touch agent or split_percent
        let project_path = write_toml(tmp.path(), "project.toml", r#"
[container]
image = "myimage"
"#);

        let config = load_with_global(Some(&global_path), Some(&project_path)).unwrap();

        assert_eq!(config.agent.as_deref(), Some("claude"));
        assert_eq!(config.tmux.split_percent, 70);
        assert_eq!(config.container.image.as_deref(), Some("myimage"));
    }

    #[test]
    fn write_defaults_creates_file_and_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nested").join("dir").join("config.toml");
        write_defaults(&path).unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[defaults]"));
        assert!(content.contains("[tmux]"));
        assert!(content.contains("[container]"));
    }

    #[test]
    fn write_defaults_content_is_valid_toml() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        write_defaults(&path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: Result<toml::Value, _> = toml::from_str(&content);
        assert!(parsed.is_ok(), "default config is not valid TOML");
    }
}
