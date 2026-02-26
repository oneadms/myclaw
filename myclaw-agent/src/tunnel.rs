use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use myclaw_common::RelayFrame;
use crate::config::AgentSettings;

/// Connect to relay, perform handshake, then connect to local gateway
/// and bridge messages bidirectionally.
pub async fn run_tunnel(cfg: &AgentSettings) -> anyhow::Result<()> {
    // --- Connect to relay ---
    info!("tunnel: connecting to relay at {}", cfg.relay_url);
    let (relay_ws, _) = connect_async(&cfg.relay_url).await?;
    let (mut relay_tx, mut relay_rx) = relay_ws.split();
    info!("tunnel: connected to relay");

    // --- Handshake ---
    let hello = serde_json::to_string(&RelayFrame::AgentHello {
        agent_id: cfg.agent_id.clone(),
    })?;
    relay_tx.send(Message::Text(hello)).await?;

    match relay_rx.next().await {
        Some(Ok(Message::Text(text))) => {
            match serde_json::from_str::<RelayFrame>(&text) {
                Ok(RelayFrame::AgentWelcome { agent_id }) => {
                    info!("tunnel: relay welcomed agent '{}'", agent_id);
                }
                _ => {
                    anyhow::bail!("unexpected relay response: {}", text);
                }
            }
        }
        other => {
            anyhow::bail!("relay handshake failed: {:?}", other);
        }
    }

    // --- Connect to local gateway ---
    info!("tunnel: connecting to gateway at {}", cfg.gateway_url);
    let (gw_ws, _) = connect_async(&cfg.gateway_url).await?;
    let (mut gw_tx, mut gw_rx) = gw_ws.split();
    info!("tunnel: connected to gateway");

    // --- Bidirectional bridge ---
    // relay → gateway
    let relay_to_gw = async {
        while let Some(Ok(msg)) = relay_rx.next().await {
            match msg {
                Message::Text(text) => {
                    if gw_tx.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    };

    // gateway → relay
    let gw_to_relay = async {
        while let Some(Ok(msg)) = gw_rx.next().await {
            match msg {
                Message::Text(text) => {
                    if relay_tx.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    };

    tokio::select! {
        _ = relay_to_gw => {
            warn!("tunnel: relay connection closed");
        }
        _ = gw_to_relay => {
            warn!("tunnel: gateway connection closed");
        }
    }

    Ok(())
}
