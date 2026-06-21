

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "drift",
    version,
    about = "P2P distributed training. Plug your GPU into the mesh."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[clap(name = "join")]
    Join {
        #[arg(long)]
        name: Option<String>,
    },
    #[clap(name = "train")]
    Train {
        #[arg(long)]
        peers: Vec<String>,

        #[arg(long, default_value = "train.yaml")]
        config: String,

        #[arg(long)]
        train_repo_url: Option<String>,

        #[arg(long)]
        model_artifact: Option<String>,

        #[arg(long, value_delimiter = ',')]
        dataset_urls: Vec<String>,

        #[arg(long, default_value = "false")]
        resume: bool,
    },
    #[clap(name = "status")]
    Status,
}