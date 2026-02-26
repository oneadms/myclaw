use tokio::sync::mpsc;

pub type Tx = mpsc::UnboundedSender<String>;
/// Shared state that bridges server-side and agent-side connections.
pub struct BridgeHandle {
    pub server_tx: Option<Tx>,
    pub agent_tx: Option<Tx>,
}

impl BridgeHandle {
    pub fn new() -> Self {
        Self {
            server_tx: None,
            agent_tx: None,
        }
    }
}
