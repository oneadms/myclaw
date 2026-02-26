mod config;
mod bridge;
mod server_side;
mod agent_side;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use clap::Parser;
use tracing::info;

use bridge::BridgeHandle;
use config::RelayConfig;

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value = "config/relay.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("myclaw_relay=debug,info")
        .init();

    let cli = Cli::parse();
    let cfg = RelayConfig::load(&cli.config)?;

    info!("relay: server_listen={}, agent_listen={}",
        cfg.relay.server_listen, cfg.relay.agent_listen);

    let bridge = Arc::new(RwLock::new(BridgeHandle::new()));

    let server_listener = TcpListener::bind(&cfg.relay.server_listen).await?;
    let agent_listener = TcpListener::bind(&cfg.relay.agent_listen).await?;

    info!("relay: listening");

    tokio::select! {
        r = server_side::run(server_listener, bridge.clone()) => {
            r?;
        }
        r = agent_side::run(agent_listener, bridge.clone()) => {
            r?;
        }
    }

    Ok(())
}
