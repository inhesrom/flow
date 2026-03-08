pub mod http;
pub mod state_bridge;
pub mod ws;

use std::net::SocketAddr;

use axum::{
    extract::ws::WebSocketUpgrade,
    extract::State,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use anvl_core::CoreHandle;
use state_bridge::ServerState;

async fn index() -> impl IntoResponse {
    Html(include_str!("../../../web/index.html"))
}

async fn app_js() -> Response {
    (
        [("content-type", "application/javascript; charset=utf-8")],
        include_str!("../../../web/app.js"),
    )
        .into_response()
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<ServerState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws::handle_socket(socket, state))
}

pub async fn run_server_with_core(core: CoreHandle, port: u16) -> anyhow::Result<()> {
    let state = ServerState::new(core);

    let app = Router::new()
        .route("/", get(index))
        .route("/app.js", get(app_js))
        .route(
            "/api/workspaces",
            get(http::list_workspaces).post(http::add_workspace),
        )
        .route("/api/workspace/{id}/git", get(http::workspace_git))
        .route("/api/workspace/{id}/diff", get(http::workspace_diff))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
