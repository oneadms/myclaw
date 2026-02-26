mod config;
mod tunnel;

use std::path::PathBuf;
use clap::Parser;
use tracing::{info, error};

use config::AgentConfig;

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value = "config/agent.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("myclaw_agent=debug,info")
        .init();

    let cli = Cli::parse();
    let cfg = AgentConfig::load(&cli.config)?;

    info!("agent: relay={}, gateway={}, id={}",
        cfg.agent.relay_url, cfg.agent.gateway_url, cfg.agent.agent_id);

    let mut backoff = cfg.agent.reconnect_base_ms;

    loop {
        match tunnel::run_tunnel(&cfg.agent).await {
            Ok(()) => {
                info!("agent: tunnel closed, reconnecting...");
                backoff = cfg.agent.reconnect_base_ms;
            }
            Err(e) => {
                error!("agent: tunnel error: {}", e);
            }
        }

        info!("agent: reconnecting in {}ms", backoff);
        tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
        backoff = (backoff * 2).min(cfg.agent.reconnect_max_ms);
    }
}
