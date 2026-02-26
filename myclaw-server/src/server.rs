use crate::config::ServerConfig;
use crate::router::RouterHandle;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use myclaw_common::{ClientMessage, ServerMessage};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};
use uuid::Uuid;

pub async fn run(config: ServerConfig, router: RouterHandle) -> Result<()> {
    let addr = config.listen_addr();
    let listener = TcpListener::bind(&addr).await?;
    info!("Client WebSocket server listening on {addr}");

    loop {
        let (stream, peer) = listener.accept().await?;
        info!("New client connection from {peer}");
        let router = router.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, router).await {
                error!("Client {peer} error: {e}");
            }
            info!("Client {peer} disconnected");
        });
    }
}

async fn handle_client(stream: tokio::net::TcpStream, router: RouterHandle) -> Result<()> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    let (mut sink, mut stream) = ws.split();
    let session_id = Uuid::new_v4().to_string();
    let (client_tx, mut client_rx) = mpsc::channel::<ServerMessage>(64);
    router.register_client(session_id.clone(), client_tx.clone()).await;

    let status = ServerMessage::Status {
        gateway_connected: router.is_gateway_connected().await,
    };
    sink.send(Message::Text(serde_json::to_string(&status)?)).await?;
    info!("Client session {session_id} established");

    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_client_msg(&text, &router, &client_tx).await;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        sink.send(Message::Pong(data)).await?;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        warn!("Client WS error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
            Some(server_msg) = client_rx.recv() => {
                let json = serde_json::to_string(&server_msg)?;
                sink.send(Message::Text(json)).await?;
            }
        }
    }

    router.unregister_client(&session_id).await;
    Ok(())
}

async fn handle_client_msg(
    text: &str,
    router: &RouterHandle,
    client_tx: &mpsc::Sender<ServerMessage>,
) {
    let msg: ClientMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            warn!("Invalid client message: {e}");
            let _ = client_tx
                .send(ServerMessage::Error {
                    message: format!("Invalid message: {e}"),
                })
                .await;
            return;
        }
    };

    match msg {
        ClientMessage::Chat { id, content } => {
            info!("Chat request {id}: {content}");
            if let Err(e) = router
                .send_to_gateway(&id, &content, client_tx.clone())
                .await
            {
                warn!("Failed to forward to gateway: {e}");
                let _ = client_tx
                    .send(ServerMessage::Error {
                        message: e.to_string(),
                    })
                    .await;
            }
        }
        ClientMessage::Ping => {
            let _ = client_tx.send(ServerMessage::Pong).await;
        }
    }
}
