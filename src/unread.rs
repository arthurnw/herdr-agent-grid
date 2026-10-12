//! Shows panes marked by the `arthur.unread` plugin as `done`, matching the
//! green treatment a finished turn gets. The marks live in that plugin's
//! SQLite state; the grid reads them through the `sqlite3` CLI and treats any
//! failure (no plugin, no CLI, locked file) as "nothing marked".

use crate::model::Agent;
use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

const PLUGIN_ID: &str = "arthur.unread";
const OVERRIDABLE: [&str; 2] = ["idle", "unknown"];

pub fn db_path() -> PathBuf {
    if let Some(path) = env::var_os("HERDR_AGENT_GRID_UNREAD_DB") {
        return path.into();
    }
    // Herdr gives each plugin a sibling directory under one state root.
    let root = env::var_os("HERDR_PLUGIN_STATE_DIR")
        .map(PathBuf::from)
        .and_then(|dir| dir.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| {
            env::var_os("XDG_STATE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env::var("HOME").unwrap_or_default()).join(".local/state")
                })
                .join("herdr/plugins")
        });
    root.join(PLUGIN_ID).join("unread.sqlite3")
}

/// Marked pane ids, re-queried only when the database or its WAL changes.
pub struct UnreadMarks {
    path: PathBuf,
    stamp: Option<(Option<SystemTime>, Option<SystemTime>)>,
    panes: HashSet<String>,
}

impl Default for UnreadMarks {
    fn default() -> Self {
        Self::new(db_path())
    }
}

impl UnreadMarks {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            stamp: None,
            panes: HashSet::new(),
        }
    }

    pub fn panes(&mut self) -> &HashSet<String> {
        let mtime = |p: &Path| p.metadata().and_then(|m| m.modified()).ok();
        let mut wal = self.path.clone().into_os_string();
        wal.push("-wal");
        let stamp = (mtime(&self.path), mtime(Path::new(&wal)));
        if self.stamp != Some(stamp) {
            self.stamp = Some(stamp);
            self.panes = if stamp.0.is_some() {
                query(&self.path)
            } else {
                HashSet::new()
            };
        }
        &self.panes
    }

    pub fn apply(&mut self, agents: &mut [Agent]) {
        apply(agents, self.panes());
    }
}

fn query(path: &Path) -> HashSet<String> {
    let Ok(out) = Command::new("sqlite3")
        .arg("-readonly")
        .arg(path)
        .arg("SELECT pane_id FROM unread")
        .output()
    else {
        return HashSet::new();
    };
    if !out.status.success() {
        return HashSet::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

pub fn apply(agents: &mut [Agent], marked: &HashSet<String>) {
    for agent in agents {
        if OVERRIDABLE.contains(&agent.status.as_str()) && marked.contains(&agent.pane_id) {
            agent.status = "done".into();
            agent.unread = true;
        }
    }
}
