use axum::extract::ws::{Message, WebSocket};
use protocol::Event;
use tokio::sync::broadcast::error::RecvError;

use crate::state_bridge::ServerState;

/// Upgrades a raw WebSocket connection into a full-duplex session.
///
/// Replays the current mirror snapshot to the client immediately, then
/// enters a select loop forwarding core events to the client and commands
/// from the client to the core.
pub async fn handle_socket(mut socket: WebSocket, state: ServerState) {
    if replay_snapshot(&mut socket, &state).await.is_err() {
        return;
    }
    run_event_loop(&mut socket, &state).await;
}

/// Sends all accumulated snapshot events to the newly connected client.
/// Returns `Err` if the socket closes during replay.
async fn replay_snapshot(socket: &mut WebSocket, state: &ServerState) -> Result<(), ()> {
    let mirror = state.mirror.read().await;
    for event in mirror.snapshot_events() {
        send_event(socket, &event).await?;
    }
    Ok(())
}

/// Drives the bidirectional WebSocket event loop until the connection closes.
async fn run_event_loop(socket: &mut WebSocket, state: &ServerState) {
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
                        if send_event(socket, &event).await.is_err() {
                            break;
                        }
                    }
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                }
            }
        }
    }
}

/// Serialises an event to JSON and sends it over the WebSocket.
async fn send_event(socket: &mut WebSocket, event: &Event) -> Result<(), ()> {
    let payload = serde_json::to_string(event).map_err(|_| ())?;
    socket
        .send(Message::Text(payload.into()))
        .await
        .map_err(|_| ())
}
