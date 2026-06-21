mod cli;
mod node;
mod coord;
mod ipc;

use anyhow::Result;
use clap::Parser;
use drift_cli::cli::Cli;
use drift_cli::cli::Commands;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Join { name } => node::join(name).await,
        Commands::Train {
            peers,
            config: _,
            train_repo_url,
            model_artifact,
            dataset_urls,
            resume,
        } => {
            coord::train(
                peers,
                train_repo_url,
                model_artifact,
                dataset_urls,
                resume,
            )
            .await
        }
        Commands::Status => node::status().await,
    }
}
