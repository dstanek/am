use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::AmError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContainer {
    pub runtime: String,
    pub image: String,
    pub container_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub slug: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub tmux_window: String,
    pub agent_pane: String,
    pub shell_pane: String,
    pub created_at: DateTime<Utc>,
    pub status: SessionStatus,
    pub container: Option<SessionContainer>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct SessionFile {
    sessions: Vec<Session>,
}

fn sessions_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".am").join("sessions.json")
}

pub fn load_sessions(repo_root: &Path) -> Result<Vec<Session>> {
    let path = sessions_path(repo_root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading sessions file {}", path.display()))?;
    let file: SessionFile = serde_json::from_str(&text)
        .with_context(|| format!("parsing sessions file {}", path.display()))?;
    Ok(file.sessions)
}

pub fn save_sessions(repo_root: &Path, sessions: &[Session]) -> Result<()> {
    let path = sessions_path(repo_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = SessionFile {
        sessions: sessions.to_vec(),
    };
    let text = serde_json::to_string_pretty(&file)?;
    std::fs::write(&path, text)?;
    Ok(())
}

pub fn find_session<'a>(sessions: &'a [Session], slug: &str) -> Option<&'a Session> {
    sessions.iter().find(|s| s.slug == slug)
}

pub fn add_session(repo_root: &Path, session: Session) -> Result<()> {
    let mut sessions = load_sessions(repo_root)?;
    if find_session(&sessions, &session.slug).is_some() {
        return Err(AmError::SlugAlreadyExists(session.slug.clone()).into());
    }
    sessions.push(session);
    save_sessions(repo_root, &sessions)
}

pub fn remove_session(repo_root: &Path, slug: &str) -> Result<()> {
    let mut sessions = load_sessions(repo_root)?;
    let before = sessions.len();
    sessions.retain(|s| s.slug != slug);
    if sessions.len() == before {
        return Err(AmError::SlugNotFound(slug.to_string()).into());
    }
    save_sessions(repo_root, &sessions)
}

pub fn update_session_status(repo_root: &Path, slug: &str, status: SessionStatus) -> Result<()> {
    let mut sessions = load_sessions(repo_root)?;
    let session = sessions
        .iter_mut()
        .find(|s| s.slug == slug)
        .ok_or_else(|| AmError::SlugNotFound(slug.to_string()))?;
    session.status = status;
    save_sessions(repo_root, &sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_session(slug: &str) -> Session {
        Session {
            slug: slug.to_string(),
            branch: format!("am/{slug}"),
            worktree_path: PathBuf::from(format!(".am/worktrees/{slug}")),
            tmux_window: format!("am-{slug}"),
            agent_pane: format!("am-{slug}.0"),
            shell_pane: format!("am-{slug}.1"),
            created_at: Utc::now(),
            status: SessionStatus::Active,
            container: None,
        }
    }

    #[test]
    fn missing_sessions_file_returns_empty_list() {
        let tmp = TempDir::new().unwrap();
        let sessions = load_sessions(tmp.path()).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn add_and_find_session() {
        let tmp = TempDir::new().unwrap();
        let session = make_session("feat");
        add_session(tmp.path(), session.clone()).unwrap();

        let sessions = load_sessions(tmp.path()).unwrap();
        assert_eq!(sessions.len(), 1);
        let found = find_session(&sessions, "feat").unwrap();
        assert_eq!(found.slug, "feat");
        assert_eq!(found.branch, "am/feat");
    }

    #[test]
    fn find_session_returns_none_for_missing_slug() {
        let sessions = vec![make_session("feat")];
        assert!(find_session(&sessions, "missing").is_none());
    }

    #[test]
    fn add_duplicate_slug_errors() {
        let tmp = TempDir::new().unwrap();
        add_session(tmp.path(), make_session("feat")).unwrap();
        let err = add_session(tmp.path(), make_session("feat")).unwrap_err();
        assert!(err.to_string().contains("feat"));
    }

    #[test]
    fn remove_session_success() {
        let tmp = TempDir::new().unwrap();
        add_session(tmp.path(), make_session("feat")).unwrap();
        add_session(tmp.path(), make_session("bugfix")).unwrap();

        remove_session(tmp.path(), "feat").unwrap();

        let sessions = load_sessions(tmp.path()).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].slug, "bugfix");
    }

    #[test]
    fn remove_nonexistent_slug_errors() {
        let tmp = TempDir::new().unwrap();
        let err = remove_session(tmp.path(), "ghost").unwrap_err();
        assert!(err.to_string().contains("ghost"));
    }

    #[test]
    fn update_session_status_persists() {
        let tmp = TempDir::new().unwrap();
        add_session(tmp.path(), make_session("feat")).unwrap();

        update_session_status(tmp.path(), "feat", SessionStatus::Done).unwrap();

        let sessions = load_sessions(tmp.path()).unwrap();
        assert_eq!(sessions[0].status, SessionStatus::Done);
    }

    #[test]
    fn update_nonexistent_session_errors() {
        let tmp = TempDir::new().unwrap();
        let err = update_session_status(tmp.path(), "ghost", SessionStatus::Done).unwrap_err();
        assert!(err.to_string().contains("ghost"));
    }

    #[test]
    fn sessions_roundtrip_json() {
        let tmp = TempDir::new().unwrap();
        let mut s = make_session("feat");
        s.container = Some(SessionContainer {
            runtime: "podman".to_string(),
            image: "myimage:latest".to_string(),
            container_id: Some("abc123".to_string()),
        });
        add_session(tmp.path(), s).unwrap();

        let loaded = load_sessions(tmp.path()).unwrap();
        let c = loaded[0].container.as_ref().unwrap();
        assert_eq!(c.runtime, "podman");
        assert_eq!(c.container_id.as_deref(), Some("abc123"));
    }
}
