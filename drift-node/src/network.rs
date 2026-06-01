use std::time::Duration;

use anyhow::Result;
use drift_proto::{read_message, write_message, DriftMessage, DRIFT_ALPN, LocalShardState, TrainConfig, ShardAssignment};
use iroh::Endpoint;
use sha2::{Sha256, Digest};
use tracing::{info, warn};

pub async fn create_endpoint() -> Result<Endpoint> {
    let endpoint = Endpoint::builder()
        .alpns(vec![DRIFT_ALPN.to_vec()])
        .bind()
        .await?;

    let node_id = endpoint.id();
    info!(%node_id, "endpoint bound");

    Ok(endpoint)
}

pub async fn handle_connection(
    conn: iroh::endpoint::Connection,
    node_info_msg: DriftMessage,
) -> Result<()> {
    let remote = conn.remote_id();
    info!(%remote, "accepted connection from coordinator");

    let (mut send, mut recv) = conn.accept_bi().await?;

    let msg = read_message(&mut recv).await?;
    match msg {
        DriftMessage::Ping => {
            info!("received initial ping from coordinator");
        }
        other => {
            anyhow::bail!("expected initial Ping, got {:?}", other);
        }
    }

    write_message(&mut send, &node_info_msg).await?;
    info!("sent node info to coordinator");

    let mut cached_config: Option<TrainConfig> = None;

    loop {
        match read_message(&mut recv).await {
            Ok(msg) => match msg {
                DriftMessage::Ping => {
                    info!("received ping, sending pong");
                    write_message(&mut send, &DriftMessage::Pong).await?;
                }
                DriftMessage::TrainConfig(config) => {
                    if let Some(ref train_repo_url) = config.train_repo_url {
                        let mut hasher = Sha256::new();
                        hasher.update(train_repo_url.as_bytes());
                        let result = hasher.finalize();
                        let commit_hash = format!("{:x}", result);
                        info!(
                            model = %config.model_path,
                            dataset = %config.dataset_path,
                            epochs = config.epochs,
                            commit_hash = %commit_hash,
                            "received training config"
                        );
                    } else {
                        info!(
                            model = %config.model_path,
                            dataset = %config.dataset_path,
                            epochs = config.epochs,
                            "received training config"
                        );
                    }
                    cached_config = Some(config);
                }
                DriftMessage::ShardAssignment(shard) => {
                    info!(
                        shard_index = shard.shard_index,
                        start = shard.shard_start,
                        end = shard.shard_end,
                        size = shard.size(),
                        "received shard assignment"
                    );

                    let node_id_for_save = if let DriftMessage::NodeInfo(ni) = &node_info_msg {
                        ni.node_id.clone()
                    } else {
                        "unknown".to_string()
                    };

                    let state = LocalShardState {
                        shard_assignment: shard,
                        train_config: cached_config.take().unwrap_or_default(),
                        last_checkpoint_step: 0,
                        completion_percentage: 0.0,
                    };
                    state.save_to_disk(&node_id_for_save).ok();
                    info!("saved shard to disk");
                }
                DriftMessage::Heartbeat { .. } => {
                    write_message(
                        &mut send,
                        &DriftMessage::Heartbeat { uptime_secs: 0 },
                    )
                    .await?;
                }
                DriftMessage::TrainComplete => {
                    info!("training complete signal received");
                    break;
                }
                other => {
                    info!(%other, "received message");
                }
            },
            Err(e) => {
                warn!("connection closed: {}", e);
                break;
            }
        }
    }

    Ok(())
}

const MAX_RETRIES: usize = 3;
const RETRY_DELAY_SECS: u64 = 5;

pub async fn handle_completion(
    node_id: &str,
    mut send: iroh::endpoint::SendStream,
    mut recv: iroh::endpoint::RecvStream,
) -> Result<()> {
    loop {
        let mut attempt = 0;
        let mut current_shard: Option<ShardAssignment> = None;

        for retry in 1..=MAX_RETRIES {
            attempt = retry;
            write_message(&mut send, &DriftMessage::AskForMoreWork).await?;
            println!("sent AskForMoreWork (attempt {})", retry);

            let response = tokio::time::timeout(
                Duration::from_secs(RETRY_DELAY_SECS),
                async { read_message(&mut recv).await },
            ).await?;

            match response {
                Ok(DriftMessage::NoMoreWork) => {
                    println!("coordinator has no more work — shutting down cleanly");

                    if let Ok(Some(cached)) = LocalShardState::load_from_disk(node_id) {
                        let _ = cached.save_to_disk(node_id);
                    }

                    return Ok(());
                }
                Ok(DriftMessage::AssignNext(shard)) => {
                    println!("received new assignment: shard {} [{}, {})",
                        shard.shard_index, shard.shard_start, shard.shard_end);

                    shard.save_to_disk(node_id)?;
                    current_shard = Some(shard);
                    break;
                }
                Ok(other) => {
                    warn!(%other, "unexpected response from coordinator");
                }
                Err(e) => {
                    warn!("failed to read response: {}", e);
                }
            }
        }

        if attempt >= MAX_RETRIES && current_shard.is_none() {
            eprintln!(
                "no response after {} attempts — assuming coordinator dead. Will shut down independently.",
                MAX_RETRIES
            );
            return Ok(());
        }

        if let Some(shard) = current_shard {
            if let Ok(Some(state)) = LocalShardState::load_from_disk(node_id) {
                let config = state.train_config.clone();
                let (progress_tx, _progress_rx) = tokio::sync::mpsc::channel(16);

                let script: String = config.script_entrypoint.as_ref().unwrap_or(&"/tmp/train.py".to_string()).to_string();
                match crate::training::spawn_training_with_progress(
                    &script,
                    &config.model_path,
                    &config.dataset_path,
                    config.batch_size,
                    config.learning_rate,
                    config.epochs,
                    shard.shard_index,
                    shard.shard_start,
                    shard.shard_end,
                    node_id.to_string(),
                    progress_tx,
                    Some(state),
                ).await {
                    Ok((_child, final_step)) => {
                        println!("training completed at step {}", final_step);
                    }
                    Err(e) => {
                        warn!(error = %e, "training failed");
                    }
                }
            }
        }
    }
}
