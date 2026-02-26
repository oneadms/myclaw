mod config;
mod tui;
mod ws;

use clap::Parser;
use config::{Cli, ClientConfig};
use tokio::sync::mpsc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("myclaw_client=debug,info")
        .init();

    let cli = Cli::parse();
    let config = ClientConfig::load(&cli.config)?;
    info!("Loaded config from {:?}", cli.config);

    let (outbound_tx, outbound_rx) = mpsc::channel(64);
    let (inbound_tx, inbound_rx) = mpsc::channel(64);

    let url = config.server.url.clone();
    let ws_task = tokio::spawn(async move {
        ws::run(&url, outbound_rx, inbound_tx).await
    });

    let tui_result = tui::run(inbound_rx, outbound_tx).await;

    ws_task.abort();
    tui_result
}
