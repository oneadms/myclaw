use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "myclaw", about = "MyClaw TUI Chat Client")]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "config/client.toml")]
    pub config: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClientConfig {
    pub server: ServerAddr,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerAddr {
    pub url: String,
}

impl ClientConfig {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
