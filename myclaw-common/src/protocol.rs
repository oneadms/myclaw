use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Client → Server messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "chat")]
    Chat { id: String, content: String },
    #[serde(rename = "ping")]
    Ping,
}

/// Server → Client messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "chat_reply")]
    ChatReply {
        id: String,
        request_id: String,
        content: String,
        done: bool,
    },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "status")]
    Status { gateway_connected: bool },
}

/// Frames exchanged with OpenClaw Gateway (WebSocket :18789)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GatewayFrame {
    /// Handshake: node connects to gateway
    #[serde(rename = "connect")]
    Connect { role: String, node_id: String },
    /// Gateway acknowledges connection
    #[serde(rename = "connected")]
    Connected { session_id: String },
    /// Send a chat message to an agent
    #[serde(rename = "chat_request")]
    ChatRequest {
        request_id: String,
        session_id: String,
        content: String,
    },
    /// Streaming reply chunk from agent
    #[serde(rename = "chat_response")]
    ChatResponse {
        request_id: String,
        session_id: String,
        content: String,
        done: bool,
    },
    /// Heartbeat
    #[serde(rename = "ping")]
    Ping { timestamp: i64 },
    #[serde(rename = "pong")]
    Pong { timestamp: i64 },
    /// Error from gateway
    #[serde(rename = "error")]
    Error { message: String },
}

/// Relay ↔ Agent control frames
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RelayFrame {
    /// Agent registers with relay
    #[serde(rename = "agent_hello")]
    AgentHello { agent_id: String },
    /// Relay acknowledges agent
    #[serde(rename = "agent_welcome")]
    AgentWelcome { agent_id: String },
}

impl ClientMessage {
    pub fn new_chat(content: impl Into<String>) -> Self {
        Self::Chat {
            id: Uuid::new_v4().to_string(),
            content: content.into(),
        }
    }
}

impl GatewayFrame {
    pub fn connect(node_id: &str) -> Self {
        Self::Connect {
            role: "node".into(),
            node_id: node_id.into(),
        }
    }

    pub fn ping_now() -> Self {
        Self::Ping {
            timestamp: Utc::now().timestamp(),
        }
    }

    pub fn chat_request(
        request_id: &str,
        session_id: &str,
        content: &str,
    ) -> Self {
        Self::ChatRequest {
            request_id: request_id.into(),
            session_id: session_id.into(),
            content: content.into(),
        }
    }
}
