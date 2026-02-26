use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::net::TcpListener;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use crate::bridge::{BridgeHandle, Tx};

/// Accept a single server-side (myclaw-server) WebSocket connection.
/// Forward messages from server → agent, and from agent_rx → server.
pub async fn run(listener: TcpListener, bridge: Arc<RwLock<BridgeHandle>>) -> anyhow::Result<()> {
    info!("server_side: waiting for myclaw-server connection");

    loop {
        let (stream, addr) = listener.accept().await?;
        info!("server_side: myclaw-server connected from {}", addr);

        let ws = accept_async(stream).await?;
        let (mut ws_tx, mut ws_rx) = ws.split();

        // Create channel: agent_side will send to this tx, we read from rx and forward to ws
        let (tx, mut rx): (Tx, _) = tokio::sync::mpsc::unbounded_channel();

        // Register server_tx so agent_side can forward to us
        {
            let mut b = bridge.write().await;
            b.server_tx = Some(tx);
        }

        let bridge_read = bridge.clone();
        let bridge_cleanup = bridge.clone();

        // Task: forward from rx → ws (agent → server)
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_tx.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        // Read from ws → forward to agent_tx (server → agent)
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(text) => {
                    let b = bridge_read.read().await;
                    if let Some(ref agent_tx) = b.agent_tx {
                        if agent_tx.send(text).is_err() {
                            warn!("server_side: agent_tx send failed, agent disconnected?");
                        }
                    } else {
                        warn!("server_side: no agent connected, dropping message");
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }

        send_task.abort();
        {
            let mut b = bridge_cleanup.write().await;
            b.server_tx = None;
        }
        warn!("server_side: myclaw-server disconnected from {}", addr);
    }
}
