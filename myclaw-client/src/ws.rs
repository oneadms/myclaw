use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use myclaw_common::{ClientMessage, ServerMessage};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info, warn};

/// Run the WebSocket connection loop.
/// - `outbound_rx`: messages from TUI to send to server
/// - `inbound_tx`: messages from server to forward to TUI
pub async fn run(
    url: &str,
    mut outbound_rx: mpsc::Receiver<ClientMessage>,
    inbound_tx: mpsc::Sender<ServerMessage>,
) -> Result<()> {
    info!("Connecting to server: {url}");
    let (ws, _) = tokio_tungstenite::connect_async(url).await?;
    let (mut sink, mut stream) = ws.split();
    info!("Connected to server");

    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ServerMessage>(&text) {
                            Ok(server_msg) => {
                                if inbound_tx.send(server_msg).await.is_err() {
                                    debug!("TUI closed, stopping WS");
                                    break;
                                }
                            }
                            Err(e) => warn!("Bad server msg: {e}"),
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        sink.send(Message::Pong(data)).await?;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        warn!("WS error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
            Some(client_msg) = outbound_rx.recv() => {
                let json = serde_json::to_string(&client_msg)?;
                sink.send(Message::Text(json)).await?;
            }
        }
    }

    Ok(())
}