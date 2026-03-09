use std::{collections::HashMap, path::PathBuf, time::Instant};

use protocol::{AttentionLevel, Route, SshTarget, WorkspaceId};

use crate::workspace::{GitState, WorkspaceTerminals};

pub struct AppState {
    pub route: Route,
    pub workspaces: HashMap<WorkspaceId, Workspace>,
    pub ordered_ids: Vec<WorkspaceId>,
    pub started_at: Instant,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            route: Route::Home,
            workspaces: HashMap::new(),
            ordered_ids: Vec::new(),
            started_at: Instant::now(),
        }
    }
}

pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub path: PathBuf,
    pub ssh: Option<SshTarget>,
    pub git: GitState,
    pub attention: AttentionLevel,
    pub terminals: WorkspaceTerminals,
    pub last_activity: Instant,
}
