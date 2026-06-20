

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
        repo: Option<String>,
        #[arg(long, value_delimiter = ',')]
        peers: Vec<String>,
        #[arg(long)]
        script: Option<String>,
        #[arg(long, default_value = "model.safetensors")]
        model_path: String,
        #[arg(long, default_value = "data/")]
        dataset_path: String,
        #[arg(long)]
        dataset: Vec<String>,
        #[arg(long, default_value = "32")]
        batch_size: u32,
        #[arg(long, default_value = "0.001")]
        learning_rate: f64,
        #[arg(long, default_value = "10")]
        epochs: u32,
        #[arg(long, default_value = "1000000")]
        dataset_size: u64,
        #[arg(long, default_value = "checkpoints/")]
        checkpoint_dir: String,
        #[arg(long, default_value = "false")]
        resume: bool,
        #[arg(long)]
        env_file: Option<String>,
    },
    #[clap(name = "status")]
    Status,
}