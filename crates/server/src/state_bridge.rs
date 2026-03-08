use std::{collections::HashMap, sync::Arc};

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use anvl_core::CoreHandle;
use protocol::{Event, GitState, TerminalKind, WorkspaceId, WorkspaceSummary};
use tokio::sync::RwLock;

/// Maximum raw bytes stored per terminal tab (1 MB).
const MAX_HISTORY_BYTES: usize = 1_048_576;

#[derive(Clone)]
pub struct ServerState {
    pub core: Arc<CoreHandle>,
    pub mirror: Arc<RwLock<MirrorState>>,
}

pub struct TerminalHistoryEntry {
    pub kind: TerminalKind,
    pub bytes: Vec<u8>,
}

#[derive(Default)]
pub struct MirrorState {
    pub workspaces: Vec<WorkspaceSummary>,
    pub workspace_paths: HashMap<WorkspaceId, String>,
    pub git: HashMap<WorkspaceId, GitState>,
    pub terminal_history: HashMap<(WorkspaceId, String), TerminalHistoryEntry>,
}

impl MirrorState {
    /// Build a list of replay events that bring a freshly-connected client
    /// up to speed without waiting for the next periodic broadcast.
    pub fn snapshot_events(&self) -> Vec<Event> {
        let mut events = Vec::new();

        if !self.workspaces.is_empty() {
            events.push(Event::WorkspaceList {
                items: self.workspaces.clone(),
            });
        }

        for (id, git) in &self.git {
            events.push(Event::WorkspaceGitUpdated {
                id: *id,
                git: git.clone(),
            });
        }

        // One TerminalOutput per tab with non-empty bytes.
        // Do NOT emit TerminalStarted; the TUI handler resets terminal state on
        // that event which would discard the replayed bytes.
        for ((ws_id, tab_id), entry) in &self.terminal_history {
            if entry.bytes.is_empty() {
                continue;
            }
            events.push(Event::TerminalOutput {
                id: *ws_id,
                kind: entry.kind,
                tab_id: Some(tab_id.clone()),
                data_b64: B64.encode(&entry.bytes),
            });
        }

        events
    }
}

impl ServerState {
    /// Creates a new `ServerState`, wrapping the provided core handle and
    /// spawning a background task that keeps the mirror in sync with core events.
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

/// Resolves a terminal tab identifier, falling back to the kind's default name.
fn resolve_tab(tab_id: Option<String>, kind: TerminalKind) -> String {
    tab_id.unwrap_or_else(|| kind_default_tab(kind))
}

/// Returns the canonical default tab identifier for a given terminal kind.
fn kind_default_tab(kind: TerminalKind) -> String {
    match kind {
        TerminalKind::Agent => "agent".to_string(),
        TerminalKind::Shell => "shell".to_string(),
    }
}

/// Retrieves or creates a terminal history entry for the given workspace and tab.
fn get_or_create_entry<'a>(
    state: &'a mut MirrorState,
    id: WorkspaceId,
    tab: String,
    kind: TerminalKind,
) -> &'a mut TerminalHistoryEntry {
    state
        .terminal_history
        .entry((id, tab))
        .or_insert_with(|| TerminalHistoryEntry {
            kind,
            bytes: Vec::new(),
        })
}

/// Trims the front of `bytes` so its length does not exceed `max_bytes`.
fn enforce_history_cap(bytes: &mut Vec<u8>, max_bytes: usize) {
    if bytes.len() > max_bytes {
        let excess = bytes.len() - max_bytes;
        bytes.drain(..excess);
    }
}

/// Applies a single core event to the mirror state, keeping it synchronised
/// with the live workspace and terminal state broadcast by `CoreHandle`.
async fn apply_event(mirror: &Arc<RwLock<MirrorState>>, evt: Event) {
    let mut state = mirror.write().await;
    match evt {
        Event::WorkspaceList { items } => apply_workspace_list(&mut state, items),
        Event::WorkspaceGitUpdated { id, git } => apply_git_updated(&mut state, id, git),
        Event::TerminalStarted { id, kind, tab_id } => {
            apply_terminal_started(&mut state, id, kind, tab_id);
        }
        Event::TerminalOutput {
            id,
            kind,
            tab_id,
            data_b64,
        } => {
            apply_terminal_output(&mut state, id, kind, tab_id, data_b64);
        }
        Event::TerminalExited {
            id,
            kind,
            tab_id,
            code,
        } => {
            apply_terminal_exited(&mut state, id, kind, tab_id, code);
        }
        _ => {}
    }
}

fn apply_workspace_list(state: &mut MirrorState, items: Vec<WorkspaceSummary>) {
    state.workspace_paths = items
        .iter()
        .map(|w| (w.id, w.path.clone()))
        .collect::<HashMap<_, _>>();
    state.workspaces = items;
}

fn apply_git_updated(state: &mut MirrorState, id: WorkspaceId, git: GitState) {
    state.git.insert(id, git);
}

fn apply_terminal_started(
    state: &mut MirrorState,
    id: WorkspaceId,
    kind: TerminalKind,
    tab_id: Option<String>,
) {
    let tab = resolve_tab(tab_id, kind);
    state.terminal_history.insert(
        (id, tab),
        TerminalHistoryEntry {
            kind,
            bytes: Vec::new(),
        },
    );
}

fn apply_terminal_output(
    state: &mut MirrorState,
    id: WorkspaceId,
    kind: TerminalKind,
    tab_id: Option<String>,
    data_b64: String,
) {
    let tab = resolve_tab(tab_id, kind);
    let entry = get_or_create_entry(state, id, tab, kind);
    if let Ok(decoded) = B64.decode(&data_b64) {
        entry.bytes.extend_from_slice(&decoded);
        enforce_history_cap(&mut entry.bytes, MAX_HISTORY_BYTES);
    }
}

fn apply_terminal_exited(
    state: &mut MirrorState,
    id: WorkspaceId,
    kind: TerminalKind,
    tab_id: Option<String>,
    code: Option<i32>,
) {
    let tab = resolve_tab(tab_id, kind);
    let marker = format!("\r\n[process exited with code {}]\r\n", code.unwrap_or(-1));
    let entry = get_or_create_entry(state, id, tab, kind);
    entry.bytes.extend_from_slice(marker.as_bytes());
}
