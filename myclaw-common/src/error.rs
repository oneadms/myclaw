use std::fmt;

#[derive(Debug)]
pub enum MyClawError {
    WebSocket(String),
    Protocol(String),
    Config(String),
    Gateway(String),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for MyClawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WebSocket(msg) => write!(f, "WebSocket error: {msg}"),
            Self::Protocol(msg) => write!(f, "Protocol error: {msg}"),
            Self::Config(msg) => write!(f, "Config error: {msg}"),
            Self::Gateway(msg) => write!(f, "Gateway error: {msg}"),
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Json(e) => write!(f, "JSON error: {e}"),
        }
    }
}

impl std::error::Error for MyClawError {}

impl From<std::io::Error> for MyClawError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for MyClawError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
