use std::sync::Arc;
use drift_node::{gpu, network, training};

use anyhow::Result;
use clap::{Parser, Subcommand};
use drift_proto::{DriftMessage, NodeInfo, LocalShardState};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "drift-node", version, about = "P2P distributed training node")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Join the drift swarm and wait for a coordinator
    Join {
        /// Optional human-readable name for this node
        #[arg(long)]
        name: Option<String>,
    },
    /// Show node status
    Status,
}

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
        Commands::Join { name } => join(name).await,
        Commands::Status => status().await,
    }
}

pub enum ResumeDecision {
    Resume,
    RequestAssignment,
}

fn decide_resume_or_reassign(state: &LocalShardState) -> ResumeDecision {
    if state.last_checkpoint_step > 0 {
        ResumeDecision::Resume
    } else {
        ResumeDecision::RequestAssignment
    }
}

async fn join(name: Option<String>) -> Result<()> {
    let gpus = gpu::detect_gpu_info().await;
    let first_gpu = gpus.first();
    let (gpu_name, gpu_vram, gpu_cc) = if let Some(gpu) = first_gpu {
        (gpu.name.clone(), gpu.vram_mb, gpu.compute_capability.clone())
    } else {
        (
            "CPU-only (no GPU detected)".to_string(),
            0,
            "0.0".to_string(),
        )
    };
    let total_vram: u64 = if gpus.is_empty() {
        0
    } else {
        gpus.iter().map(|g| g.vram_mb).sum()
    };

    let endpoint = Arc::new(network::create_endpoint().await?);
    let node_id = endpoint.id();

    let node_id_str = node_id.to_string();

  match LocalShardState::load_from_disk(&node_id_str) {
        Ok(Some(cached)) => {
            println!("found cached state, resuming from step {}",
                cached.last_checkpoint_step);

            match decide_resume_or_reassign(&cached) {
                ResumeDecision::Resume => {
                    println!("decided to resume training");
                    let shard = &cached.shard_assignment;
                    println!("  shard {} [{}, {})", shard.shard_index, shard.shard_start, shard.shard_end);
                    let config = &cached.train_config;
                    println!("  config: epochs={}, batch={}", config.epochs, config.batch_size);
                    info!(step = cached.last_checkpoint_step, "will resume from checkpoint");

                    let script: String = config.script_entrypoint.as_ref().unwrap_or(&"/tmp/train.py".to_string()).to_string();
                    let (progress_tx, _progress_rx) = mpsc::channel(16);
                    let gpu_cc = config.gpu_compute_capability.unwrap_or(0.0);

                    match training::spawn_training_with_progress(
                        &script,
                        &config.model_path,
                        &config.dataset_path,
                        &config.dataset_urls,
                        config.batch_size,
                        config.learning_rate,
                        config.epochs,
                        shard.shard_index,
                        shard.shard_start,
                        shard.shard_end,
                        node_id_str.clone(),
                        gpu_cc,
                        progress_tx,
                        Some(cached.clone()),
                    ).await {
                        Ok((_child, final_step)) => {
                            println!("training completed at step {}", final_step);
                        }
                        Err(e) => {
                            error!(error = %e, "training failed");
                        }
                    }
                }
                ResumeDecision::RequestAssignment => {
                    println!("decided to request fresh assignment");
                    let cache_path = LocalShardState::local_cache_path(&node_id_str);
                    if cache_path.exists() {
                        if let Err(e) = std::fs::remove_file(&cache_path) {
                            warn!(error = %e, "failed to delete cached state");
                        }
                    }
                    println!("cache deleted, will request new assignment from coordinator");
                }
            }
        }
        _ => {
            println!("no local cache found, waiting for coordinator to assign work");
        }
    }

    let short_id = node_id.to_string();
    let display_name = name.unwrap_or_else(|| {
        if short_id.chars().count() > 12 {
            short_id.chars().take(12).collect::<String>()
        } else {
            short_id.clone()
        }
    });
    println!("drift node started");
    println!("  Node ID:  {}", node_id);
    println!("  Name:     {}", display_name);
    if gpus.len() <= 1 {
        println!("  GPU:      {} ({} MB VRAM)", gpu_name, gpu_vram);
    } else {
        println!("  GPUs:     {} devices ({} MB total VRAM)", gpus.len(), total_vram);
        for (i, gpu) in gpus.iter().enumerate() {
            println!("    [{}] {} ({} MB)", i, gpu.name, gpu.vram_mb);
        }
    }

    let node_info_msg = DriftMessage::NodeInfo(NodeInfo {
        node_id: node_id.to_string(),
        gpu_name,
        gpu_vram_mb: total_vram.max(gpu_vram),
        gpu_compute_capability: gpu_cc,
        available: true,
    });

    let accept_loop = async {
        loop {
            let ep = endpoint.clone();
            let incoming = match ep.accept().await {
                Some(incoming) => incoming,
                None => {
                    info!("endpoint closed");
                    break;
                }
            };

            let node_info = node_info_msg.clone();
            let node_id_copy = node_id_str.clone();
            tokio::spawn(async move {
                match incoming.await {
                    Ok(conn) => {
                        if let Err(e) = network::handle_connection(&*ep, conn, node_info, &node_id_copy).await {
                            error!("connection handler error: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("failed to accept connection: {}", e);
                    }
                }
            });
        }
    };

    tokio::select! {
        _ = accept_loop => {}
        _ = tokio::signal::ctrl_c() => {
            println!();
            println!("shutting down...");

            let node_id_str = node_id.to_string();
            if let Ok(Some(cached)) = LocalShardState::load_from_disk(&node_id_str) {
                cached.save_to_disk(&node_id_str)?;
            }
        }
    }

    endpoint.close().await;
    Ok(())
}

async fn status() -> Result<()> {
    let nvidia_gpus = gpu::detect_gpus().await?;
    let apple_gpus = gpu::detect_gpu_info().await;
    
    let mut gpus: Vec<_> = nvidia_gpus;
    gpus.extend(apple_gpus);
    let driver = gpu::driver_version().await;

    println!("drift node status");
    println!("---");
    if let Some(v) = driver {
        println!("  Driver: {}", v);
    }
    if gpus.is_empty() {
        println!("  GPUs: none detected");
    } else {
        let total_vram: u64 = gpus.iter().map(|g| g.vram_mb).sum();
        for (i, gpu) in gpus.iter().enumerate() {
            println!(
                "  GPU {}: {} ({} MB VRAM, compute {})",
                i, gpu.name, gpu.vram_mb, gpu.compute_capability
            );
        }
        if gpus.len() > 1 {
            println!("  Total: {} MB VRAM across {} devices", total_vram, gpus.len());
        }
    }

    Ok(())
}
