use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "myclaw-server", about = "MyClaw OpenClaw Channel Server")]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "config/server.toml")]
    pub config: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub server: ListenConfig,
    pub gateway: GatewayConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListenConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GatewayConfig {
    pub url: String,
    pub node_id: String,
    pub heartbeat_interval_secs: u64,
    pub reconnect_base_ms: u64,
    pub reconnect_max_ms: u64,
}

impl ServerConfig {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}
