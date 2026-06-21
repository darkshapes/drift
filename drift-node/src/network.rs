use std::time::{Duration, Instant};

use anyhow::Result;
use drift_proto::{read_message, write_message, DriftMessage, DRIFT_ALPN, LocalShardState, TrainConfig, ShardAssignment, RepoCommit};
use iroh::{Endpoint, EndpointAddr};
use iroh::endpoint::RelayMode;
use sha2::{Sha256, Digest};
use tracing::{error, info, warn};

pub async fn create_endpoint() -> Result<(Endpoint, EndpointAddr)> {
    let endpoint = Endpoint::empty_builder(RelayMode::Disabled)
        .alpns(vec![DRIFT_ALPN.to_vec()])
        .bind()
        .await?;

    let node_id = endpoint.id();
    info!(%node_id, "endpoint bound");

    let addr = endpoint.addr();
    Ok((endpoint, addr))
}

pub async fn handle_connection(
    endpoint: &Endpoint,
    conn: iroh::endpoint::Connection,
    node_info_msg: DriftMessage,
    node_id: &str,
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
    let mut standby_start: Option<Instant> = None;
    let mut training_ready_received = false;

    loop {
        if let Some(start) = standby_start {
            if start.elapsed() > Duration::from_secs(30) {
                warn!("TrainingReady timeout - no response within 30 seconds");
                anyhow::bail!("standby timeout");
            }
        }

        match read_message(&mut recv).await {
            Ok(msg) => match msg {
                DriftMessage::Ping => {
                    info!("received ping, sending pong");
                    write_message(&mut send, &DriftMessage::Pong).await?;
                }
                DriftMessage::TrainConfig(config) => {
                    if let Some(ref model_artifact) = config.model_artifact {
                        let mut hasher = Sha256::new();
                        hasher.update(model_artifact.as_bytes());
                        let result = hasher.finalize();
                        let commit_hash = format!("{:x}", result);
                        let dataset = config.dataset_urls.first().map(|s| s.as_str()).unwrap_or("none");
                        info!(
                            "received training config: model={}, dataset={}, repo_hash={:?}, commit_hash={}",
                            model_artifact,
                            dataset,
                            config.repo_hash,
                            commit_hash
                        );
                    } else {
                        info!(
                            "received training config: model={:?}, dataset_urls={}",
                            config.model_artifact,
                            config.dataset_urls.len()
                        );
                    }
                    cached_config = Some(config);

                    if let Some(ref model_art) = cached_config.clone() {
                        if let Some(repo_url) = model_art.model_artifact.clone() {
                            match get_git_commit(&repo_url).await {
                                Ok(commit) => {
                                    tracing::info!("repo_url from get_git_commit: {}", &repo_url);
                                    let secret_key = endpoint.secret_key();
                                    let message = format!("{}|{}|{}", node_id, commit, repo_url);
                                    let signature = secret_key.sign(message.as_bytes()).to_bytes().to_vec();
                                    tracing::info!(signature_len = signature.len(), "signature length");
                                    let repo_commit = RepoCommit {
                                        commit,
                                        repo_url,
                                        signature,
                                    };
                                    write_message(&mut send, &DriftMessage::RepoCommit(repo_commit)).await?;
                                    info!("sent RepoCommit to coordinator");
                                    standby_start = Some(Instant::now());
                                }
                                Err(e) => {
                                    warn!(error = %e, "failed to get git commit");
                                }
                            }
                        }
                    }
                }
                DriftMessage::ShardAssignment(shard) => {
                    if !training_ready_received {
                        warn!("received ShardAssignment before TrainingReady");
                        break;
                    }
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
                DriftMessage::TrainingReady => {
                    training_ready_received = true;
                    standby_start = None;
                    info!("received TrainingReady");
                }
                DriftMessage::TrainingCancel(cancel) => {
                    error!(
                        reason = %cancel.reason,
                        time = %cancel.time,
                        repo_url = %cancel.repo_url,
                        "TrainingCancel received from coordinator - shutting down"
                    );
                    break;
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

                let script = "/tmp/train.py";
                match crate::training::spawn_training_with_progress(
                    script,
                    config.model_artifact.as_ref().map(|s| s.as_str()),
                    &config.dataset_urls,
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

async fn get_git_commit(repo_url: &str) -> Result<String> {
    if !repo_url.contains("://") {
        let output = std::process::Command::new("git")
            .args(["-C", repo_url, "rev-parse", "HEAD"])
            .output()?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }
    let output = std::process::Command::new("git")
        .args(["ls-remote", repo_url, "HEAD"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("git ls-remote failed for {}", repo_url);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().next())
        .map(|hash| hash.to_string())
        .ok_or_else(|| anyhow::anyhow!("no HEAD ref found"))
}
