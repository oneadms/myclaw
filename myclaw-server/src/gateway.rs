use crate::config::GatewayConfig;
use crate::router::RouterHandle;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use myclaw_common::GatewayFrame;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

pub async fn run(config: GatewayConfig, router: RouterHandle) -> Result<()> {
    let mut backoff_ms = config.reconnect_base_ms;

    loop {
        match connect_and_run(&config, &router).await {
            Ok(()) => {
                info!("Gateway connection closed normally");
                backoff_ms = config.reconnect_base_ms;
            }
            Err(e) => {
                error!("Gateway connection error: {e}");
            }
        }

        router.set_gateway_connected(false).await;
        warn!("Reconnecting to gateway in {backoff_ms}ms...");
        sleep(Duration::from_millis(backoff_ms)).await;
        backoff_ms = (backoff_ms * 2).min(config.reconnect_max_ms);
    }
}

async fn connect_and_run(config: &GatewayConfig, router: &RouterHandle) -> Result<()> {
    info!("Connecting to gateway: {}", config.url);
    let (ws, _) = tokio_tungstenite::connect_async(&config.url).await?;
    let (mut sink, mut stream) = ws.split();

    // Send connect handshake
    let connect_frame = GatewayFrame::connect(&config.node_id);
    let msg = serde_json::to_string(&connect_frame)?;
    sink.send(Message::Text(msg)).await?;
    info!("Sent connect handshake as node: {}", config.node_id);

    // Wait for connected ack
    let _gateway_session = match stream.next().await {
        Some(Ok(Message::Text(text))) => {
            let frame: GatewayFrame = serde_json::from_str(&text)?;
            match frame {
                GatewayFrame::Connected { session_id } => {
                    info!("Gateway connected, session: {session_id}");
                    router.set_gateway_session(session_id.clone()).await;
                    router.set_gateway_connected(true).await;
                    session_id
                }
                GatewayFrame::Error { message } => {
                    anyhow::bail!("Gateway rejected: {message}");
                }
                _ => anyhow::bail!("Unexpected frame during handshake"),
            }
        }
        Some(Ok(_)) => anyhow::bail!("Non-text frame during handshake"),
        Some(Err(e)) => return Err(e.into()),
        None => anyhow::bail!("Connection closed during handshake"),
    };

    // Channel for outbound messages to gateway
    let (gw_tx, mut gw_rx) = tokio::sync::mpsc::channel::<String>(64);
    router.set_gateway_sender(Some(gw_tx)).await;

    let heartbeat_interval = Duration::from_secs(config.heartbeat_interval_secs);

    loop {
        tokio::select! {
            // Incoming from gateway
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_gateway_msg(&text, router).await?;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        sink.send(Message::Pong(data)).await?;
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        info!("Gateway connection closed");
                        break;
                    }
                    Some(Err(e)) => return Err(e.into()),
                    _ => {}
                }
            }
            // Outbound to gateway
            Some(msg) = gw_rx.recv() => {
                sink.send(Message::Text(msg)).await?;
            }
            // Heartbeat
            _ = sleep(heartbeat_interval) => {
                let ping = serde_json::to_string(&GatewayFrame::ping_now())?;
                sink.send(Message::Text(ping)).await?;
                debug!("Sent heartbeat ping");
            }
        }
    }

    router.set_gateway_sender(None).await;
    Ok(())
}

async fn handle_gateway_msg(text: &str, router: &RouterHandle) -> Result<()> {
    let frame: GatewayFrame = serde_json::from_str(text)?;
    match frame {
        GatewayFrame::ChatResponse {
            request_id,
            content,
            done,
            ..
        } => {
            router.dispatch_reply(&request_id, &content, done).await;
        }
        GatewayFrame::Pong { .. } => {
            debug!("Received pong from gateway");
        }
        GatewayFrame::Error { message } => {
            warn!("Gateway error: {message}");
        }
        _ => {
            debug!("Unhandled gateway frame: {text}");
        }
    }
    Ok(())
}
