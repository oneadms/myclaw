use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub agent: AgentSettings,
}

#[derive(Debug, Deserialize)]
pub struct AgentSettings {
    pub relay_url: String,
    pub gateway_url: String,
    pub agent_id: String,
    pub reconnect_base_ms: u64,
    pub reconnect_max_ms: u64,
}

impl AgentConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
