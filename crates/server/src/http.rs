use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use multiws_core::workspace::git::diff_file;
use protocol::Command;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state_bridge::ServerState;

pub async fn list_workspaces(State(state): State<ServerState>) -> impl IntoResponse {
    let mirror = state.mirror.read().await;
    Json(mirror.workspaces.clone())
}

#[derive(Debug, Deserialize)]
pub struct AddWorkspaceRequest {
    pub path: String,
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AddWorkspaceResponse {
    pub ok: bool,
}

pub async fn add_workspace(
    State(state): State<ServerState>,
    Json(body): Json<AddWorkspaceRequest>,
) -> impl IntoResponse {
    let path = body.path.trim().to_string();
    if path.is_empty() {
        return Json(AddWorkspaceResponse { ok: false });
    }

    let name = body.name.unwrap_or_else(|| {
        std::path::Path::new(&path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "workspace".to_string())
    });

    let ok = state
        .core
        .cmd_tx
        .send(Command::AddWorkspace { name, path })
        .await
        .is_ok();
    Json(AddWorkspaceResponse { ok })
}

pub async fn workspace_git(
    State(state): State<ServerState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let mirror = state.mirror.read().await;
    let git = mirror.git.get(&id).cloned().unwrap_or_default();
    Json(git)
}

#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    pub file: String,
}

pub async fn workspace_diff(
    State(state): State<ServerState>,
    Path(id): Path<Uuid>,
    Query(params): Query<DiffQuery>,
) -> impl IntoResponse {
    let repo_path = {
        let mirror = state.mirror.read().await;
        mirror.workspace_paths.get(&id).cloned()
    };

    if let Some(path) = repo_path {
        diff_file(std::path::Path::new(&path), &params.file)
            .await
            .unwrap_or_default()
    } else {
        String::new()
    }
}
