mod config;
mod gateway;
mod router;
mod server;

use clap::Parser;
use config::{Cli, ServerConfig};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("myclaw_server=debug,info")
        .init();

    let cli = Cli::parse();
    let config = ServerConfig::load(&cli.config)?;
    info!("Loaded config from {:?}", cli.config);

    let (router_handle, _router) = router::Router::new();

    let gw_config = config.gateway.clone();
    let gw_router = router_handle.clone();
    let gw_task = tokio::spawn(async move {
        gateway::run(gw_config, gw_router).await
    });

    let srv_config = config.clone();
    let srv_task = tokio::spawn(async move {
        server::run(srv_config, router_handle).await
    });

    tokio::select! {
        r = gw_task => r??,
        r = srv_task => r??,
    }

    Ok(())
}
