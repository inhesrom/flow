use axum::extract::ws::{Message, WebSocket};
use tokio::sync::broadcast::error::RecvError;

use crate::state_bridge::ServerState;

pub async fn handle_socket(mut socket: WebSocket, state: ServerState) {
    let mut evt_rx = state.core.evt_tx.subscribe();

    loop {
        tokio::select! {
            maybe_msg = socket.recv() => {
                match maybe_msg {
                    Some(Ok(Message::Text(txt))) => {
                        if let Ok(cmd) = serde_json::from_str::<protocol::Command>(&txt) {
                            let _ = state.core.cmd_tx.send(cmd).await;
                        }
                    }
                    Some(Ok(Message::Binary(_))) => {}
                    Some(Ok(Message::Ping(_))) => {}
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) => break,
                    Some(Err(_)) | None => break,
                }
            }
            evt = evt_rx.recv() => {
                match evt {
                    Ok(event) => {
                        if let Ok(payload) = serde_json::to_string(&event) {
                            if socket.send(Message::Text(payload.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                }
            }
        }
    }
}
