# COVEN/drift Architecture Clarifications

Collected during planning session on 2026-05-13.

---

## 1. Coordinator Behavior (Clarified)

The coordinator is a passive receiver that:

- Sends initial `ShardAssignment` messages to each node at startup
- Displays aggregate training progress to operator
- Tracks peer addresses for reconnection across restart cycles
- Is NOT required for nodes to complete their work once assigned

### Coordinator Responsibilities (only while alive)

| #   | Responsibility                                               |
| --- | ------------------------------------------------------------ |
| 1   | Send initial ShardAssignment to each participant             |
| 2   | Reassign failed node's shard when asked by an idle node      |
| 3   | Display training status to the human running the coordinator |

### Coordinator Non-Responsibilities

- Nodes do not require coordinator after receiving initial assignment
- Coordinator going down does NOT halt compute on active nodes
- Each worker completes independently; progress may be lost if coord dies

### Coordinator Restart Recovery

On restart, coordinator reads persisted state and:

1. Identifies surviving nodes from peer registry (`/var/lib/drift/nodes.toml`)
2. Marks unfinished shards as pending reassignment
3. Awaits new connections from surviving nodes
4. Distributes unclaimed shards as nodes ask for more work

---

## 2. Node Behavior (Clarified)

Each worker node is fully autonomous once it has a `ShardAssignment`.

### Startup Sequence

```
ON STARTUP:
    1. Check local cache: ~/.drift/shard-{node_id}.toml

    IF cached AND NOT completed:
        Resume training from cached shard WITHOUT contacting coordinator

    ELSE IF not cached OR marked complete:
        Ignore

        Request new assignment FROM coordinator:
        - Connect using provided iroh peer address
        - Wait for ShardAssignment message
        - Cache assignment locally before starting (overwrite if exists)

        Store coordinator endpoint: ~/.drift/coordinator.toml
```

### Normal Operation Loop

```
WHILE running:
    1. Receive TrainConfig { script_path, dataset_repo_url, model_path }

    2. spawn_training() runs Python subprocess with proper args

    3. Read stdout line-by-line via BufReader:
       - Parse JSON progress output (step, loss, epoch)

    4. Send TrainProgress over iroh TO coordinator
       - Do NOT wait for acknowledgment before continuing

    5. On completion signal received (or timeout):
       - Write completion record to disk (~/.drift/done-{shard_index})
       - Send TrainComplete to coordinator if available
       - Attempt N retries waiting for response (NO_WORK or more work)
       - If no reply after N attempts → shutdown independently
```

### Node Shutdown Conditions

| Condition                          | Action                                           |
| ---------------------------------- | ------------------------------------------------ |
| Receives "no work" from coord      | Shutdown cleanly                                 |
| No response after N retry attempts | Assume coord dead → shutdown independently       |
| Training completes successfully    | Complete compute task regardless of coordination |

---

## 3. Shard Assignment Requirements (Clarified)

Each shard must be assigned EXACTLY once across the entire training run.

### Original Assignment Rules

- Coordinator pre-computes all shards upfront based on VRAM weighting
- Shards are stored in a queue and dispatched as nodes connect
- Each node receives its assignment AND caches it locally

### Reassignment After Failure — CRITICAL CONSTRAINT

```
ORIGINAL assignments computed once:
   [(node_a: [0-8000]), (node_b: [8001-24000]), (node_c: [24001-32000])]

When node_c fails at time T:
   node_c was mid-shard, has completed subset of [24001-32000]

ACCEPTABLE options:
   A) Give node_c's REMAINING unprocessed range to another node

   B) Reassign the FULL original range [24001-32000] to any idle node

   C) Wait for node_c to reconnect and continue where it left off

NOT acceptable:
   - Recomputing via weighted VRAM division (causes different boundaries)
   - Assigning overlapping ranges (violates "exactly once" invariant)
```

### State Persistence Per Node

```rust
// ~/.drift/shard-{node_id}.toml format
struct LocalShardState {
    shard_assignment: ShardAssignment,
    last_checkpoint_step: u64,
    completion_percentage: f32,
}

// Written after each TrainProgress message
// Used on restart to resume from checkpoint
}
```

---

## 4. Autonomous Architecture with Persistence

The system is designed so nodes can complete work INDEPENDENTLY if coordinator dies.

### Filesystem Requirements

| Path                                 | Purpose                                            |
| ------------------------------------ | -------------------------------------------------- |
| `~/.drift/coordinator.toml`          | Persisted by EACH node — how to reach coord again  |
| `~/.drift/shard-{node_id}.toml`      | Persisted by EACH node — own assignment + progress |
| `/var/lib/drift/nodes.toml`          | Maintained by COORD only — peer registry + states  |
| `/var/lib/drift/pending-shards.toml` | Coordinator's queue of unassigned shards           |

### Resumption Paths

```
┌─────────────────┐       ┌──────────────────┐
│ Coordinator Up? │        │ Action                   │
├───────────────┼──────────────────────────────────────────┤
│ YES              │ Send TrainComplete → await response     │
│                  │ "NO" received → shutdown cleanly           │
│                  │ "more_work {shard}" → assign and continue │
│                  │ timeout (N retries) → shutdown independently│
├─────────────────┼──────────────────────────────────────────┤
│ NO               │ Node completes its assigned shard          │
│                  │ Writes completion record to disk            │
│                  │ Shuts down without coordination            │
└──────────────────┴──────────────────────────────────────────┘
```

---

## 5. Coordinator Peer Registry & Cached Endpoints

### What Each Side Remembers

#### Coordinator-Side (`/var/lib/drift/nodes.toml`)

```rust
struct PeerRegistry {
    peers: Vec<PeerEntry>,
}

struct PeerEntry {
    node_id: String,
    did_hash_address: iroh::PublicKey,   // How to reach this node via iroh

    original_shard: ShardAssignment,    // Originally assigned range
    current_status: NodeStatus,          // Active | Completed | Failed | Unknown

    last_seen: Option<Instant>,         // For staleness detection
}
```

#### Node-Side (`~/.drift/coordinator.toml`)

```rust
struct CoordEndpointCache {
    did_hash_address: iroh::PublicKey,   // How to reach coordinator via iroh

    public_key_or_secret: Option<String>, // Auth token if required
}
```

### Address Exchange Protocol

```
HASH ADDRESS SOURCES:
- Provided at runtime via CLI (--peers flag)
- Exchanged during initial handshake over iroh connection
- NOT persisted in message enums; must be explicitly handled

When a node needs to reconnect after completing its shard:
1. Read ~/.drift/coordinator.toml
2. Attempt iroh connection using cached endpoint
3. If fails → assume coord dead → shutdown independently
```

---

## 6. Corrected Message Flow: Training Initialization Over iroh

The coordinator sends the following messages TO each worker node:

### Session Initialization Sequence

```
Coordinator starts with peer list from CLI --peers flag

FOR EACH connected node:
    1. Send TrainConfig
       {
           script_path: "train.py",           // or repo entrypoint path
           train_repo_url: "https://github.com/darkshapes/IDIDiT" #
           dataset_repo_url: "https://huggingface.co/AHAT/a-single-hat",
           model_artifact_ref: Optional<"model.safetensors">,
           epochs: 10,
           batch_size: 32,
           learning_rate: 0.001
       }

    2. Send ShardAssignment
       {
           node_id: String,                   // This node's ID
           shard_index: u32,                  // Position in assignment order
           shard_start: u64,                    // Byte range start
           shard_end: u64                     // Byte range end (inclusive)
       }

    Node then runs spawn_training() with these values
```

### What Each Field Means for the Node

| Field                | Purpose                                                          |
| -------------------- | ---------------------------------------------------------------- |
| `script_path`        | Entry point file within training repo to execute                 |
| `dataset_repo_url`   | Git URL to clone; contains dataset + optional processing scripts |
| `model_artifact_ref` | If fine-tuning: artifact name/path within that repo              |

### Node Responsibility After Receiving Init Messages

```
ON receiving TrainConfig + ShardAssignment:

1. Clone/fetch dataset from dataset_repo_url

2. If model_artifact_ref present:
       Fetch/checkout model artifacts from same repo

3. Write init record to disk:
      ~/.drift/shard-{node_id}.toml = {TrainConfig, ShardAssignment}

4. Execute training via spawn_training():
   python(script_path, --dataset=data/, --shard-start=X, --shard-end=Y, ...)
```

---

## 7. Local Message Relay Sequence Between Python and Rust

Python subprocess communicates progress back to parent Rust process via STDOUT.

### Current Implementation (environment variables — TO BE DEPRECATED)

```
# OLD way - environment variable based
SPAWN COMMAND:
python train.py \
    --batch_size=32 \                    # From env var
    EPOCH=$EPOCH                        # Set by Rust before fork

PROGRESS:
# Python prints nothing useful;
# Must rely on checkpoint files or shared state
```

### New Implementation (structured JSON over stdout/stdin)

#### Rust Side (`spawn_training()` in drift-node/src/training.rs)

```rust
pub async fn spawn_training(
    script: &str,
    shard_start: u64,
    shard_end: u64,
    // ... other config
) -> Result<tokio::process::Child> {

    let mut child = tokio::process::Command::new("python")
        .arg(script)
        .args([
            "--shard-index", shard_index.to_string(),
            "--shard-start", shard_start.to_string(),
            "--shard-end", shard_end.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())   // IMPORTANT: Capture output streams
        .spawn()?;

    // Stream stdout for progress parsing
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(raw_line)) = lines.next_line().await {
                // Each line should be valid JSON: {"step":N,"loss":F}

                match serde_json::from_str::<serde_json::Value>(&raw_line) {
                    Ok(json) => {
                        let step = json["step"].as_u64().unwrap_or(0);
                        let loss = json["loss"].as_f64().unwrap_or(0.0);

                        // Send to coordinator over iroh
                        write_message(send, &DriftMessage::TrainProgress {
                            node_id: MY_NODE_ID,
                            epoch: 1,           // From config
                            step,
                            loss,
                            throughput_samples_per_sec: 0.0,  // Ignored for now
                        }).await?;
                    }
                    Err(_) => {
                        tracing::warn!(%raw_line, "unparseable training output");
                    }
                }
            }
        });
    }

    Ok(child)
}
```

#### Python Side (expected output format)

```python
# In train.py or any training script:
import json
import time

for epoch in range(epochs):
    for step in range(steps_per_epoch):
        # ... actual training logic ...

        progress = {
            "epoch": epoch + 1,
            "step": total_steps,
            "loss": current_loss(),
        }
        print(json.dumps(progress))  # Line-buffered JSON to stdout
```

### Why This Is Better Than Environment Variables

| Aspect                | Env Vars                         | Stdout JSON                     |
| --------------------- | -------------------------------- | ------------------------------- |
| Timing                | Set once at spawn                | Emitted continuously during run |
| Progress visibility   | None until checkpoint            | Real-time streaming             |
| Debugging             | Must parse env in both processes | Just cat the output file        |
| Ordering guarantees   | None                             | Line-by-line sequential         |
| Backpressure handling | None                             | BufReader handles naturally     |

---

## 8. Summary of Implementation Gaps vs Requirements

### What's Already Implemented

- [x] `DriftMessage` enum with all message types defined
- [x] iroh send/receive functions (`write_message`, `read_message`)
- [x] Basic connection establishment over iroh
- [x] Spawning python subprocess via tokio::process::Command
- [x] Shard assignment computation (VRAM-weighted)
- [x] Stale detection (15s interval, 30s timeout)

### What Needs To Be Added

#### High Priority

1. [ ] Add persistence layer — read/write local shard state files
2. [ ] Add coordinator endpoint cache — ~/.drift/coordinator.toml
3. [ ] Implement TrainConfig field for dataset_repo_url and script_path
4. [ ] Wire stdout line reader to parse progress JSON → TrainProgress messages
5. [ ] Implement completion retry logic before shutdown
6. [ ] Coordinator peer registry: /var/lib/drift/nodes.toml on restart

#### Medium Priority

7. [ ] Handle "NO_WORK" vs "more_work" responses from coord after TrainComplete
8. [ ] Node-side staleness detection as backup if not heard by coord
9. [ ] Proper shard reassignment that preserves exact original boundaries

#### Low Priority

10. [ ] Checkpoint upload/download between nodes and storage
11. [ ] Throughput calculation per node from step deltas

#### Optional

A. [ ] Ring all-reduce protocol for gradient synchronization
