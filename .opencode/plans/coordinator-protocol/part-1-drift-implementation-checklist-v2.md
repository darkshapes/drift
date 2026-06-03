# Part 1Implementation Checklist for COVEN/drift Architecture Clarifications

Step-by-step guide derived from session-2026-05-13-clarifications.md

---

## Pre-requisites

### Files to Create/Modify

```
drift-proto/src/lib.rs      — Add persistence types + message extensions
drift-node/src/main.rs        — Node startup + persistence logic
drift-coord/src/main.rs      — Coordinator peer registry + restart recovery
```

### New Types (add to drift-proto/src/lib.rs)

```rust
// === NEW: Persistence Types ===

/// Persistent state written by each node per training run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalShardState {
    pub shard_assignment: ShardAssignment,
    pub train_config: TrainConfig,
    pub last_checkpoint_step: u64,
    pub completion_percentage: f32,
}

impl LocalShardState {
    /// Path where node writes its persistent state (~/.drift/shard-{node_id}.toml)
    pub fn local_cache_path(node_id: &str) -> std::path::PathBuf {
        dirs::home_dir()
            .map(|h| h / ".drift" / format!("shard-{}", node_id))
            .unwrap_or_else(|| PathBuf::from(format!("/tmp/drift-shard-{}", node_id)))
    }
}

/// Cached endpoint for reaching coordinator after initial connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordEndpointCache {
    pub did_hash_address: String,
    pub public_key_or_secret: Option<String>,
}

impl CoordEndpointCache {
    /// Path where each node caches coordinator address (~/.drift/coordinator.toml)
    pub fn cache_path() -> std::path::PathBuf {
        dirs::home_dir()
            .and_then(|h| Some(h / ".drift"))
            .map(|p| p / "coordinator.toml")
            .unwrap_or_else(|| PathBuf::from("/tmp/drift-coordinator.toml"))
    }
}
```

### New Message Variants (add to DriftMessage enum)

```rust
/// Coordinator response when a node asks for more work after completing shard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkResponse {
    NoWork,
    MoreWork { assignment: ShardAssignment },
}

// Add to DriftMessage enum:
pub enum DriftMessage {
    // ... existing variants ...

    // === NEW ===

    /// Node requests another assignment after finishing current shard.
    AskForMoreWork,

    /// Coordinator responds with no remaining work — node should shutdown.
    NoMoreWork,

    /// Coordinator responds with next shard to assign to this node.
    AssignNext(ShardAssignment),
}
```

---

## Phase 1: Persistence Layer

### Add to drift-proto/src/lib.rs

```rust
// === EXISTING LOCATION: drift-proto/src/lib.rs (~line 200) ===

impl ShardAssignment {
    pub fn size(&self) -> u64 {
        self.shard_end.saturating_sub(self.shard_start)
    }

    // === ADD THIS METHOD ===
    pub fn save_to_disk(&self, node_id: &str) -> anyhow::Result<()> {
        let path = LocalShardState::local_cache_path(node_id);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let state = LocalShardState {
            shard_assignment: self.clone(),
            train_config: TrainConfig { /* placeholder until received */ },
            last_checkpoint_step: 0,
            completion_percentage: 0.0,
        };

        serde_json::to_writer(std::fs::file(path)?)?;
        Ok(())
    }

    pub fn load_from_disk(node_id: &str) -> anyhow::Result<Option<LocalShardState>> {
        let path = LocalShardState::local_cache_path(node_id);

        if !path.exists() {
            return Ok(None);
        }

        let file = std::fs::open(path)?;
        let reader = std::io::BufReader::new(file);
        let state: LocalShardState = serde_json::from_reader(reader)?;
        Ok(Some(state))
    }
}
```

---

## Phase 2: Node Startup Sequence

### Modify drift-node/src/main.rs (around line 44)

```rust
// === EXISTING LOCATION: drift-node/src/main.rs:join() function ===

async fn join(name: Option<String>) -> Result<()> {
    // ... existing GPU detection and endpoint creation ...

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

## Phase 3: Extended TrainConfig with Repository URLs

### Extend TrainConfig struct in drift-proto/src/lib.rs (~line 186)

```rust
// === EXISTING LOCATION: drift-proto/src/lib.rs (around line 186) ===
// Current definition:

pub struct TrainConfig {
    pub model_path: String,
    pub dataset_path: String,
    pub batch_size: u32,
    pub learning_rate: f64,
    pub epochs: u32,
}

// === EXTEND TO INCLUDE REPO FIELDS ===

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainConfig {
    // Existing fields
    pub model_path: String,
    pub dataset_path: String,
    pub batch_size: u32,
    pub learning_rate: f64,
    pub epochs: u32,

    // New fields for distributed repo-based training
    /// URL of the repository containing the training script.
    /// Node should clone this and run the specified entrypoint.
    #[serde(default)]
    pub train_repo_url: Option<String>,

    /// Path within the cloned train_repo to execute (e.g., "train.py").
    /// If not set, falls back to existing --script CLI argument behavior.
    #[serde(default)]
    pub script_entrypoint: Option<String>,

    /// HuggingFace repo ID or Git URL for the dataset.
    /// Node should download/clone this before starting training.
    #[serde(default)]
    pub dataset_repo_url: Option<String>,

    /// Optional path within dataset_repo for fine-tuning from local base model.
    #[serde(default)]
    pub model_artifact_ref: Option<String>,
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

## Execution Order

1. **Add persistence types** to `drift-proto/src/lib.rs`
   - LocalShardState struct with serde derive
   - CoordEndpointCache struct
   - New DriftMessage variants (AskForMoreWork, NoMoreWork, AssignNext)
2. **Implement local cache read/write** on ShardAssignment
3. **Wire progress parser** into `spawn_training_with_progress()` in drift-node/src/training.rs
4. **Extend TrainConfig** with repo URL fields
5. **Create peer registry module** at drift-coord/src/peer_registry.rs
6. **Modify stale detector** to update PeerRegistry status field
7. **Handle AskForMoreWork** in coordinator listener task
8. **Write completion logic**: no more work → NoMoreWork response
9. **Add ctrl-c handler** that persists state before exit

---

## Verification Checklist

- [ ] Unit test: Node saves shard to disk, reloads same assignment after restart
- [ ] Integration test: Node completes and requests more work; coord responds with next shard or NoMoreWork
- [ ] Edge case: Multiple nodes ask for more work simultaneously — only one gets the reassigned shard
- [ ] Race condition: Coordinator restarts while node is mid-training — node should reconnect without issue
- [ ] Persistence: Ctrl-C during training run, restart shows last known state from registry
