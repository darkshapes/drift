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
            repo,
            peers,
            script,
            model_path,
            dataset_path,
            dataset,
            batch_size,
            learning_rate,
            epochs,
            dataset_size,
            checkpoint_dir,
            resume,
        } => {
            coord::train(
                repo,
                peers,
                script,
                model_path,
                dataset_path,
                dataset,
                batch_size,
                learning_rate,
                epochs,
                dataset_size,
                checkpoint_dir,
                resume,
            )
            .await
        }
        Commands::Status => node::status().await,
    }
}
