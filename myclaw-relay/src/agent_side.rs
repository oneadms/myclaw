use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::net::TcpListener;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use myclaw_common::RelayFrame;
use crate::bridge::{BridgeHandle, Tx};

/// Accept a single agent-side (myclaw-agent) WebSocket connection.
/// Perform RelayFrame handshake, then bridge messages bidirectionally.
pub async fn run(listener: TcpListener, bridge: Arc<RwLock<BridgeHandle>>) -> anyhow::Result<()> {
    info!("agent_side: waiting for myclaw-agent connection");

    loop {
        let (stream, addr) = listener.accept().await?;
        info!("agent_side: incoming connection from {}", addr);

        let ws = accept_async(stream).await?;
        let (mut ws_tx, mut ws_rx) = ws.split();

        // --- Handshake: expect AgentHello ---
        let agent_id = match ws_rx.next().await {
            Some(Ok(Message::Text(text))) => {
                match serde_json::from_str::<RelayFrame>(&text) {
                    Ok(RelayFrame::AgentHello { agent_id }) => {
                        info!("agent_side: agent hello from '{}'", agent_id);
                        agent_id
                    }
                    _ => {
                        warn!("agent_side: expected AgentHello, got: {}", text);
                        continue;
                    }
                }
            }
            _ => {
                warn!("agent_side: connection closed before handshake");
                continue;
            }
        };

        // Send AgentWelcome
        let welcome = serde_json::to_string(&RelayFrame::AgentWelcome {
            agent_id: agent_id.clone(),
        })?;
        if ws_tx.send(Message::Text(welcome)).await.is_err() {
            warn!("agent_side: failed to send welcome");
            continue;
        }
        info!("agent_side: agent '{}' registered", agent_id);

        // Create channel: server_side will send to this tx
        let (tx, mut rx): (Tx, _) = tokio::sync::mpsc::unbounded_channel();

        {
            let mut b = bridge.write().await;
            b.agent_tx = Some(tx);
        }

        let bridge_read = bridge.clone();
        let bridge_cleanup = bridge.clone();

        // Task: forward from rx → ws (server → agent)
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_tx.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        // Read from ws → forward to server_tx (agent → server)
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(text) => {
                    let b = bridge_read.read().await;
                    if let Some(ref server_tx) = b.server_tx {
                        if server_tx.send(text).is_err() {
                            warn!("agent_side: server_tx send failed");
                        }
                    } else {
                        warn!("agent_side: no server connected, dropping message");
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }

        send_task.abort();
        {
            let mut b = bridge_cleanup.write().await;
            b.agent_tx = None;
        }
        warn!("agent_side: agent '{}' disconnected", agent_id);
    }
}
