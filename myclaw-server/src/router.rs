use myclaw_common::{GatewayFrame, ServerMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, warn};

/// Shared handle to the router state
#[derive(Clone)]
pub struct RouterHandle {
    inner: Arc<RwLock<RouterState>>,
}

struct RouterState {
    /// Sender to gateway WS
    gateway_tx: Option<mpsc::Sender<String>>,
    /// Gateway session ID
    gateway_session: Option<String>,
    /// Whether gateway is connected
    gateway_connected: bool,
    /// request_id → client session sender
    pending: HashMap<String, mpsc::Sender<ServerMessage>>,
    /// client session_id → sender
    clients: HashMap<String, mpsc::Sender<ServerMessage>>,
}

pub struct Router;

impl Router {
    pub fn new() -> (RouterHandle, Self) {
        let state = RouterState {
            gateway_tx: None,
            gateway_session: None,
            gateway_connected: false,
            pending: HashMap::new(),
            clients: HashMap::new(),
        };
        let handle = RouterHandle {
            inner: Arc::new(RwLock::new(state)),
        };
        (handle, Router)
    }
}

impl RouterHandle {
    pub async fn set_gateway_connected(&self, connected: bool) {
        self.inner.write().await.gateway_connected = connected;
    }

    pub async fn set_gateway_session(&self, session: String) {
        self.inner.write().await.gateway_session = Some(session);
    }

    pub async fn set_gateway_sender(&self, tx: Option<mpsc::Sender<String>>) {
        self.inner.write().await.gateway_tx = tx;
    }

    pub async fn is_gateway_connected(&self) -> bool {
        self.inner.read().await.gateway_connected
    }

    pub async fn register_client(
        &self,
        session_id: String,
        tx: mpsc::Sender<ServerMessage>,
    ) {
        self.inner.write().await.clients.insert(session_id, tx);
    }

    pub async fn unregister_client(&self, session_id: &str) {
        let mut state = self.inner.write().await;
        state.clients.remove(session_id);
        state.pending.retain(|_, v| !v.is_closed());
    }

    /// Forward a client chat message to the gateway
    pub async fn send_to_gateway(
        &self,
        request_id: &str,
        content: &str,
        client_tx: mpsc::Sender<ServerMessage>,
    ) -> anyhow::Result<()> {
        let state = self.inner.read().await;
        let gw_tx = state
            .gateway_tx
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Gateway not connected"))?;
        let session = state
            .gateway_session
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No gateway session"))?;

        let frame = GatewayFrame::chat_request(request_id, session, content);
        let msg = serde_json::to_string(&frame)?;
        gw_tx.send(msg).await?;
        drop(state);

        // Register pending request
        self.inner
            .write()
            .await
            .pending
            .insert(request_id.to_string(), client_tx);
        Ok(())
    }

    /// Dispatch a gateway reply to the appropriate client
    pub async fn dispatch_reply(
        &self,
        request_id: &str,
        content: &str,
        done: bool,
    ) {
        let state = self.inner.read().await;
        if let Some(tx) = state.pending.get(request_id) {
            let msg = ServerMessage::ChatReply {
                id: uuid::Uuid::new_v4().to_string(),
                request_id: request_id.to_string(),
                content: content.to_string(),
                done,
            };
            if tx.send(msg).await.is_err() {
                warn!("Client disconnected for request {request_id}");
            }
        } else {
            debug!("No pending request for {request_id}");
        }
        drop(state);

        if done {
            self.inner.write().await.pending.remove(request_id);
        }
    }
}
