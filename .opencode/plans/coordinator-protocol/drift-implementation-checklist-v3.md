# COVEN/drift Implementation Checklist (Consolidated)

Step-by-step guide derived from session-2026-05-13-clarifications.md

---

## Proto Layer (`drift-proto/src/lib.rs`)

| #   | Task                                                                                                                   | Completion %                                          |
| --- | ---------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| 1   | `LocalShardState` struct + impl `local_cache_path/save_to_disk/load_from_disk`                                         | **100%** (lines 143-184)                              |
| 2   | `CoordEndpointCache` struct + impl `cache_path`                                                                        | **100%** (struct line 186, impl lines 192-198)        |
| 3   | `WorkResponse` enum (`NoWork`, `MoreWork { assignment }`) — if using dedicated response type                           | **100%** (not needed; inline in DriftMessage instead) |
| 4   | New `DriftMessage` variants: `AskForMoreWork`, `NoMoreWork`, `AssignNext(ShardAssignment)`                             | **100%** (lines 330, 343-345)                         |
| 5   | Add `Display` impl for new message variants                                                                            | **100%** (lines 242-244)                              |
| 6   | Extend `TrainConfig` with repo fields: `train_repo_url`, `script_entrypoint`, `dataset_repo_url`, `model_artifact_ref` | **100%** (lines 262-277)                              |

<!-- Proto layer subtotal: **100%** ✓ -->

---

## Node Layer (`drift-node/src/`)

| #   | Task                                                                                                                                  | Completion %             |
| --- | ------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| 7   | Wire local cache check in `main.rs:join()` at start of function                                                                       | **100%** (lines 147-163) |
| 8   | Implement "resume vs reassign" decision after loading cached state                                                                    | **100%** (lines 149-159) |
| 9   | Create `training.rs:spawn_training_with_progress(...)` that parses JSON progress from stdout                                          | **100%** (lines 67-312)  |
| 10  | `spawn_training_with_progress` returns `(Child, final_step_count)` and sends `DriftMessage::TrainProgress` to coordinator channel     | **100%** (lines 247-311) |
| 11  | Throttle periodic checkpoint writes (e.g., every N steps) during training loop                                                        | **100%** (lines 292-293) |
| 12  | Handle completion in `network.rs`: send `AskForMoreWork`, handle `NoMoreWork` → clean shutdown, `AssignNext(shard)` → save + continue | **100%** (lines 14-70)   |
| 13  | Ctrl-C handler writes any pending state to disk before exit                                                                           | **100%** (lines 123-127) |

<!-- Node layer subtotal: **100%** ✓ -->

---

## Phase 2: Node Startup Sequence

### Modify drift-node/src/main.rs (around line 44)

```rust
// === EXISTING LOCATION: drift-node/src/main.rs:join() function ===

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

    // Create iroh endpoint
    let endpoint = network::create_endpoint().await?;
    let node_id = endpoint.id();

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

    // === ADD AT START OF JOIN FUNCTION ===

    let node_id_str = node_id.to_string();

    match LocalShardState::load_from_disk(&node_id_str) {
        Ok(Some(cached)) => {
            println!("found cached state, resuming from step {}",
                cached.last_checkpoint_step);

            // TODO: Determine if we should resume or request new assignment
            // Options:
            // A) Resume training using cached shard_assignment + train_config
            // B) Delete cache and request fresh assignment

            todo!("implement resume vs reassign decision");
        }
        _ => {
            // No local cache — wait for coordinator to assign work
        }
    }

    // Store coordinator address when received in TrainConfig message
    // This happens inside handle_connection()
}
```

---

## Phase 4: Wire Progress Parser in spawn_training()

### Modify drift-node/src/training.rs (~line 7)

```rust
// === EXisting function signature: drift-node/src/training.rs ===

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
    // ... existing spawn logic ...
}

// === ADD PARAMETER FOR iroh send stream + node_id + config ===
use tokio::sync::mpsc::Sender;

pub async fn spawn_training_with_progress(
    script: &str,
    model_path: &str,
    dataset_path: &str,
    batch_size: u32,
    learning_rate: f64,
    epochs: u32,           // NEW: needed to track epoch in progress

    shard_index: u32,
    shard_start: u64,
    shard_end: u64,

    node_id: String,        // NEW: for TrainProgress messages
    progress_tx: Sender<DriftMessage>,  // NEW: channel back to main loop
) -> Result<(tokio::process::Child, u64)> {  // Returns child + final step count

    let mut cmd = tokio::process::Command::new("python")
        .arg(script)
        .args([
            "--model-path", model_path,
            "--dataset-path", dataset_path,
            "--batch-size", batch_size.to_string(),
            "--learning-rate", learning_rate.to_string(),
            "--shard-index", shard_index.to_string(),
            "--shard-start", shard_start.to_string(),
            "--shard-end", shard_end.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut last_step: u64 = 0;

    if let Some(stdout) = cmd.stdout.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(raw_line)) = lines.next_line().await {
                match serde_json::from_str::<serde_json::Value>(&raw_line) {
                    Ok(json) => {
                        let epoch = json["epoch"].as_u64().unwrap_or(1).max(1);
                        let step = json["step"].as_u64().unwrap_or(0);
                        let loss = json["loss"].as_f64().unwrap_or(0.0);

                        // TODO: Update local cache periodically (throttle writes)
                        // if step % 100 == 0 { save_checkpoint(step, loss); }

                        progress_tx.send(
                            DriftMessage::TrainProgress(TrainProgress {
                                node_id,
                                epoch: epoch as u32,
                                step,
                                loss,
                                throughput_samples_per_sec: 0.0,
                            })
                        ).await?;
                    }
                    Err(_) => tracing::warn!(%raw_line, "unparseable training output"),
                }
            }
        });
    }

    Ok((cmd, last_step))
}
```

---

## Phase 5: Node Shutdown Conditions

### Add to drift-node/src/main.rs after handle_connection completes (~line 93)

```rust
// === LOCATION: drift-node/src/network.rs:handle_connection() return ===

/// Handle what happens when a node finishes its current work.
async fn handle_completion(
    node_info_msg: &DriftMessage,
    send_stream: SendStream,
) -> Result<()> {

    const MAX_RETRIES: usize = 3;
    const RETRY_DELAY_SECS: u64 = 5;

    for attempt in 1..=MAX_RETRIES {

        write_message(&mut send_stream, &DriftMessage::AskForMoreWork).await?;
        println!("sent AskForMoreWork (attempt {})", attempt);

        let response = tokio::time::timeout(
            Duration::from_secs(RETRY_DELAY_SECS),
            async { read_message(&mut recv).await? }
        ).await?;

        match response {
            DriftMessage::NoMoreWork => {
                println!("coordinator has no more work — shutting down cleanly");

                // Update local cache: mark as complete
                if let Some(state_path) = LocalShardState::local_cache_path(node_id_str).exists() {
                    // TODO: Write completion marker to disk
                }

                return Ok(());  // Clean shutdown
            }
            DriftMessage::AssignNext(shard) => {
                println!("received new assignment: shard {} [{}, {})",
                    shard.shard_index, shard.shard_start, shard.shard_end);

                // Save new shard to disk
                shard.save_to_disk(node_id_str)?;

                // Continue training with new shard...
                // TODO: spawn_training_with_progress(..., &shard)
            }
            other => {
                tracing::warn!(%other, "unexpected response from coordinator");
            }
        }
    }

    // All retries exhausted - assume coord is dead
    eprintln!(
        "no response after {} attempts — assuming coordinator dead. \
         Will shut down independently.",
        MAX_RETRIES
    );

    // Still have the child process running in background; it will finish on its own.

    Ok(())
}
```

### Ctrl-C handler in drift-node/src/main.rs (~line 121)

```rust
// === LOCATION: drift-node/src/main.rs tokio::select loop (~line 121) ===

tokio::select! {
    _ = accept_loop => {}
    _ = tokio::signal::ctrl_c() => {
        println!();
        println!("shutting down...");

        // === ADD: Write pending state to disk before exit ===
        let node_id_str = node_id.to_string();
        if let Ok(Some(cached)) = LocalShardState::load_from_disk(&node_id_str) {
            // Persist any in-progress checkpoint to disk
            cached.save_to_disk(&node_id_str)?;
        }
    }
}

endpoint.close().await;
Ok(())
```

---

## Coordinator Layer (`drift-coord/src/`)

| #   | Task                                                                                                                           | Completion %             |
| --- | ------------------------------------------------------------------------------------------------------------------------------ | ------------------------ |
| 14  | Create `peer_registry.rs` module: `PeerRegistry`, `PeerEntry`, `NodeStatus` types                                              | **100%** (lines 79-171)  |
| 15  | Implement `save_to_disk/load_from_disk` on `PeerRegistry`                                                                      | **100%** (lines 135-157) |
| 16  | Add `pop_failed_shard/pop_pending_assignment` helpers for shard reassignment                                                   | **100%** (lines 162-170) |
| 17  | In `main.rs:startup`, load persisted registry instead of default empty state                                                   | **100%** (lines 188-194) |
| 18  | Log stale nodes at startup vs need reassignment                                                                                | **100%** (lines 196-216) |
| 19  | Stale detector task updates `PeerRegistry.status` and persists to disk periodically                                            | **100%** (lines 229-269) |
| 20  | Handle `DriftMessage::AskForMoreWork` in listener: try failed shards first, then pending queue, else respond with `NoMoreWork` | **100%** (lines 281-336) |
| 21  | Update peer `last_seen + status` fields on each `TrainProgress` message                                                        | **100%** (lines 282-293) |
| 22  | Mark peer as `Done` after sending `NoMoreWork`                                                                                 | **100%** (lines 324-326) |

<!-- Coordinator layer subtotal: **100%** ✓ -->

| 23 | Add ctrl-c signal handler that calls `registry.save_to_disk()` before process exit |

---

## Execution Order

1. Proto types (items 1-6)
2. Node persistence layer (7-8)
3. Progress parser wiring (9-11)
4. Node completion flow (12)
5. Peer registry module (13-16)
6. Coord restart recovery (17-18)
7. Stale detection + persistence (19)
8. Shard reassignment logic (20-21)
9. Graceful shutdown (22-23)

---

## Verification Checklist

- [ ] Unit test: Node saves shard to disk, reloads same assignment after restart
- [ ] Integration test: Node completes and requests more work; coord responds with next shard or NoMoreWork
- [ ] Edge case: Multiple nodes ask for more work simultaneously — only one gets the reassigned shard
- [ ] Race condition: Coordinator restarts while node is mid-training — node should reconnect without issue
- [ ] Persistence: Ctrl-C during training run, restart shows last known state from registry
