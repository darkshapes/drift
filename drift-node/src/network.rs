use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::Result;
use drift_auth::crypto::sign_repo_commit;
use drift_proto::{read_message, write_message, DriftMessage, DRIFT_ALPN, LocalShardState, TrainConfig, ShardAssignment, RepoCommit};
use ed25519_dalek::SigningKey;
use iroh::Endpoint;
use sha2::{Sha256, Digest};
use tracing::{info, warn};

static NODE_SIGNING_KEY: Mutex<Option<Vec<u8>> = Mutex::new(None);

pub fn set_signing_key(key: Vec<u8>) {
    let mut guard = NODE_SIGNING_KEY.lock().unwrap();
    *guard = Some(key);
}

pub async fn load_or_create_signing_key() -> Result<Vec<u8>> {
    let path = signing_key_path();
    if path.exists() {
        let data = std::fs::read(&path)?;
        if data.len() == 32 {
            Ok(data);
        } else {
            anyhow::bail!("invalid signing key file size");
        }
    } else {
        let key: Vec<u8> = (0..32).map(|_| rand::random()).collect();
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, key)?;
        Ok(key)
    }
}

fn signing_key_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".into());
    std::path::PathBuf::from(home).join(".drift/identity/signing_key")
}

pub fn get_signing_key() -> Option<Vec<u8>> {
    let guard = NODE_SIGNING_KEY.lock().unwrap();
    guard.clone()
}

pub async fn create_endpoint() -> Result<Endpoint> {
    let endpoint = Endpoint::builder()
        .alpns(vec![DRIFT_ALPN.to_vec()])
        .bind()
        .await?;

    let node_id = endpoint.id();
    info!(%node_id, "endpoint bound");

    Ok(endpoint)
}

pub async fn create_endpoint_with_signing_key(signing_key: Vec<u8>) -> Result<Endpoint> {
    set_signing_key(signing_key);
    let endpoint = Endpoint::builder()
        .alpns(vec![DRIFT_ALPN.to_vec()])
        .bind()
        .await?;

    let node_id = endpoint.id();
    info!(%node_id, "endpoint bound with signing key");

    Ok(endpoint)
}

pub async fn handle_connection(
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

                    if let Some(ref train_repo_url) = cached_config.clone() {
                        if let Some(repo_url) = train_repo_url.train_repo_url.clone() {
                            match get_git_commit(&repo_url).await {
                                Ok(commit) => {
                                    let signing_key = get_signing_key();
                                    let signature = if let Some(key_bytes) = signing_key {
                                        if key_bytes.len() == 32 {
                                            let seed = [u8; 32];
                                            seed.copy_from_slice(&key_bytes);
                                            let keypair = SigningKey::from_bytes(&seed);
                                            sign_repo_commit(node_id, &commit, &repo_url, &keypair).to_bytes().to_vec()
                                        } else {
                                            Vec::new()
                                        }
                                    } else {
                                        Vec::new()
                                    };
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
                    info!("received TrainingReady - discovering script entrypoint");
                    let repo_url = if let Some(ref config) = cached_config {
                        config.train_repo_url.clone()
                    } else {
                        None
                    };
                    if let Some(url) = repo_url {
                        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
                        let base = std::path::PathBuf::from(home).join(".local/share/drift");
                        match crate::script_discovery::clone_repo_to_drift_cache(&url, &base).await {
                            Ok(cloned_path) => {
                                match crate::script_discovery::discover_script_entrypoint(&cloned_path) {
                                    Ok(entrypoint) => {
                                        info!(entrypoint = %entrypoint, "discovered script entrypoint");
                                        if let Some(ref mut config) = cached_config {
                                            config.script_entrypoint = Some(entrypoint);
                                        } else {
                                            let mut new_config = drift_proto::TrainConfig::default();
                                            new_config.script_entrypoint = Some(entrypoint);
                                            cached_config = Some(new_config);
                                        }
                                    }
                                    Err(e) => {
                                        warn!(error = %e, "failed to discover entrypoint");
                                        let now = std::time::SystemTime::now()
                                            .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                            .map(|d| d.as_secs().to_string())
                                            .unwrap_or_else(|_| "0".to_string());
                                        let cancel = drift_proto::DriftMessage::TrainingCancel(drift_proto::TrainingCancel {
                                            reason: format!("entrypoint not found: {}", e),
                                            time: now,
                                            repo_url: url,
                                        });
                                        write_message(&mut send, &cancel).await?;
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "failed to clone repo");
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                    .map(|d| d.as_secs().to_string())
                                    .unwrap_or_else(|_| "0".to_string());
                                let cancel = drift_proto::DriftMessage::TrainingCancel(drift_proto::TrainingCancel {
                                    reason: format!("clone failed: {}", e),
                                    time: now,
                                    repo_url: url,
                                });
                                write_message(&mut send, &cancel).await?;
                                break;
                            }
                        }
                    } else {
                        warn!("TrainingReady received without train_repo_url in config");
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::SystemTime::UNIX_EPOCH)
                            .map(|d| d.as_secs().to_string())
                            .unwrap_or_else(|_| "0".to_string());
                        let cancel = drift_proto::DriftMessage::TrainingCancel(drift_proto::TrainingCancel {
                            reason: "no train_repo_url in config".to_string(),
                            time: now,
                            repo_url: "".to_string(),
                        });
                        write_message(&mut send, &cancel).await?;
                        break;
                    }
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
                let gpu_cc = config.gpu_compute_capability.unwrap_or(0.0);
                match crate::training::spawn_training_with_progress(
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
                    node_id.to_string(),
                    gpu_cc,
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
    use tokio::process::Command;

    let output = Command::new("git")
        .arg("ls-remote")
        .arg("HEAD")
        .arg(repo_url)
        .output()
        .await?;

    if !output.status.success() {
        anyhow::bail!("git ls-remote failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let commit = stdout.split_whitespace().next().unwrap_or("").to_string();

    if commit.is_empty() {
        anyhow::bail!("empty commit hash from git ls-remote");
    }

    Ok(commit)
}
