use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct RelayConfig {
    pub relay: ListenConfig,
}

#[derive(Debug, Deserialize)]
pub struct ListenConfig {
    pub server_listen: String,
    pub agent_listen: String,
}

impl RelayConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
