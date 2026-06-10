use std::process::Stdio;
use std::sync::Arc;

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc::Sender, Mutex};
use tracing::{error, info, warn};

use drift_proto::{DriftMessage, TrainProgress, LocalShardState};

pub const CHECKPOINT_THROTTLE_INTERVAL: u64 = 100;

pub async fn spawn_training(
    script: &str,
    model_path: &str,
    dataset_path: &str,
    batch_size: u32,
    learning_rate: f64,
    shard_index: u32,
    shard_start: u64,
    shard_end: u64,
) -> Result<tokio::process::Child> {
    info!(
        script,
        shard_index, shard_start, shard_end, "spawning training subprocess"
    );

    let mut child = tokio::process::Command::new("python")
        .arg(script)
        .arg("--model-path")
        .arg(model_path)
        .arg("--dataset-path")
        .arg(dataset_path)
        .arg("--batch-size")
        .arg(batch_size.to_string())
        .arg("--learning-rate")
        .arg(learning_rate.to_string())
        .arg("--shard-index")
        .arg(shard_index.to_string())
        .arg("--shard-start")
        .arg(shard_start.to_string())
        .arg("--shard-end")
        .arg(shard_end.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                info!(target: "training", "{}", line);
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                error!(target: "training", "{}", line);
            }
        });
    }

    Ok(child)
}

pub async fn spawn_training_with_progress(
    script: &str,
    model_path: &str,
    dataset_path: &str,
    dataset_urls: &[String],
    batch_size: u32,
    learning_rate: f64,
    epochs: u32,
    shard_index: u32,
    shard_start: u64,
    shard_end: u64,
    node_id: String,
    gpu_compute_capability: f64,
    progress_tx: Sender<DriftMessage>,
    cached_state: Option<LocalShardState>,
) -> Result<(tokio::process::Child, u64)> {
    info!(
        script,
        shard_index, shard_start, shard_end, "spawning training with progress"
    );

    let last_step = Arc::new(Mutex::new(0u64));
    let last_step_clone = last_step.clone();
    let node_id_clone = node_id.clone();

   let use_shell = script.contains(' ');
    let mut base_cmd = tokio::process::Command::new(if use_shell { "sh" } else { "python" });
    if use_shell {
        base_cmd.arg("-c").arg(script);
    } else {
        base_cmd.arg(script);
    }
    base_cmd.arg("--model-path").arg(model_path)
        .arg("--dataset-path").arg(dataset_path);
    for url in dataset_urls {
        base_cmd.arg("--dataset-url").arg(url);
    }
    base_cmd.arg("--batch-size").arg(batch_size.to_string())
        .arg("--learning-rate").arg(learning_rate.to_string())
        .arg("--epochs").arg(epochs.to_string())
        .arg("--shard-index").arg(shard_index.to_string())
        .arg("--shard-start").arg(shard_start.to_string())
        .arg("--shard-end").arg(shard_end.to_string())
        .arg("--gpu-cc").arg(gpu_compute_capability.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = base_cmd.spawn()?;

    let progress_tx_stdby = progress_tx.clone();

    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(raw_line)) = lines.next_line().await {
                match serde_json::from_str::<serde_json::Value>(&raw_line) {
                    Ok(json) => {
                        let epoch = json["epoch"].as_u64().unwrap_or(1).max(1);
                        let step = json["step"].as_u64().unwrap_or(0);
                        let loss = json["loss"].as_f64().unwrap_or(0.0);

                        *last_step_clone.lock().await = step;

                        if should_write_checkpoint(step) {
                            if let Some(state) = &cached_state {
                                let mut updated = state.clone();
                                updated.last_checkpoint_step = step;
                                updated.completion_percentage = if shard_end > 0 {
                                    (step as f64 / shard_end as f64).min(1.0) as f32
                                } else {
                                    0.0
                                };
                                if let Err(e) = updated.save_to_disk(&node_id_clone) {
                                    warn!(step, error = %e, "failed to write checkpoint");
                                }
                            }
                        }

                        let _ = progress_tx_stdby.send(
                            DriftMessage::TrainProgress(TrainProgress {
                                node_id: node_id_clone.clone(),
                                epoch: epoch as u32,
                                step,
                                loss,
                                throughput_samples_per_sec: 0.0,
                            })
                        ).await;
                    }
                    Err(_) => {
                        warn!(%raw_line, "unparseable training output");
                    }
                }
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                error!(target: "training", "{}", line);
            }
        });
    }

    let final_step = *last_step.lock().await;
    Ok((child, final_step))
}

pub fn should_write_checkpoint(step: u64) -> bool {
    step > 0 && step % CHECKPOINT_THROTTLE_INTERVAL == 0
}
