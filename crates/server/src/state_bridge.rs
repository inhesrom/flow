use std::{collections::HashMap, sync::Arc};

use multiws_core::CoreHandle;
use protocol::{Event, GitState, WorkspaceId, WorkspaceSummary};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct ServerState {
    pub core: Arc<CoreHandle>,
    pub mirror: Arc<RwLock<MirrorState>>,
}

#[derive(Default)]
pub struct MirrorState {
    pub workspaces: Vec<WorkspaceSummary>,
    pub workspace_paths: HashMap<WorkspaceId, String>,
    pub git: HashMap<WorkspaceId, GitState>,
}

impl ServerState {
    pub fn new(core: CoreHandle) -> Self {
        let core = Arc::new(core);
        let mirror = Arc::new(RwLock::new(MirrorState::default()));
        let mut evt_rx = core.evt_tx.subscribe();
        let mirror_task = mirror.clone();

        tokio::spawn(async move {
            while let Ok(evt) = evt_rx.recv().await {
                apply_event(&mirror_task, evt).await;
            }
        });

        Self { core, mirror }
    }
}

async fn apply_event(mirror: &Arc<RwLock<MirrorState>>, evt: Event) {
    let mut state = mirror.write().await;
    match evt {
        Event::WorkspaceList { items } => {
            state.workspace_paths = items
                .iter()
                .map(|w| (w.id, w.path.clone()))
                .collect::<HashMap<_, _>>();
            state.workspaces = items;
        }
        Event::WorkspaceGitUpdated { id, git } => {
            state.git.insert(id, git);
        }
        _ => {}
    }
}
