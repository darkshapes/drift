# Part 2 Implementation Checklist for COVEN/drift Architecture Clarifications

Step-by-step guide derived from session-2026-05-13-clarifications.md

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

---

## Phase 6: Coordinator Peer Registry

### Create drift-coord/src/peer_registry.rs (new file)

```rust
// === NEW FILE: drift-coord/src/peer_registry.rs ===

use std::collections::{HashMap, VecDeque};
use serde::{Deserialize, Serialize};

/// Persistent registry of all nodes in a training run.
/// Written to /var/lib/drift/nodes.toml by the coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRegistry {
    /// All known peers and their current state.
    pub peers: HashMap<String, PeerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerEntry {
    /// iroh peer ID for reaching this node.
    pub did_hash_address: String,

    /// The shard originally assigned to this node at start of run.
    pub original_shard: ShardAssignment,

    /// Current status within the training run.
    pub status: NodeStatus,

    /// Last time we received any message from this node.
    #[serde(default)]
    pub last_seen: Option<time::Instant>,
}

/// Possible states for a peer's participation in training.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Connected and actively training.
    Active,
    /// Completed its assignment and asked for more work.
    Idle,
    /// Hasn't responded to ping within timeout — may need reassignment.
    Stale { since: Instant },
    /// Marked as failed; shard should be reassigned when possible.
    Failed { unclaimed_since: Instant },
    /// Explicitly completed all work with no more shards remaining.
    Done,
}

impl PeerRegistry {
    pub fn new() -> Self {
        Self { peers: HashMap::new() }
    }

    /// Path where coordinator writes registry (/var/lib/drift/nodes.toml)
    pub fn state_path() -> std::path::PathBuf {
        std::path::PathBuf
            .from("/var/lib/drift/nodes.toml")
    }

    pub fn save_to_disk(&self) -> anyhow::Result<()> {
        let path = Self::state_path();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        serde_json::to_writer(std::fs::file(path), self)?;
        Ok(())
    }

    pub fn load_from_disk() -> anyhow::Result<Self> {
        let path = Self::state_path();

        if !path.exists() {
            return Ok(Self::new());
        }

        let file = std::fs::open(path)?;
        let reader = std::io::BufReader::new(file);
        let registry: PeerRegistry = serde_json::from_reader(reader)?;
        Ok(registry)
    }

    // === COORDINATOR SIDE: reassignment logic ===

    /// Find the first failed node's shard that needs reassignment.
    pub fn pop_failed_shard(&mut self) -> Option<ShardAssignment> {
        for (_, entry) in self.peers.iter_mut() {
            if matches!(entry.status, NodeStatus::Failed { .. }) {
                entry.status = NodeStatus::Idle;  // Mark as now idle
                return Some(entry.original_shard.clone());
            }
        }
        None
    }
}
```

---

## Phase 7: Coordinator Restart Recovery

### Modify drift-coord/src/main.rs (around line 44)

```rust
// === LOCATION: drift-coord/src/main.rs:~line 30 ===

#[tokio::main]
async fn main() -> Result<()> {
    // ... existing tracing init ...

    // On restart, try to load persisted peer registry + pending shards
    let registry = match PeerRegistry::load_from_disk().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("could not read persistent state: {} — starting fresh", e);
            PeerRegistry::new()
        }
    };

    println!(
        "loaded {} known peers from previous run",
        registry.peers.len()
    );

    // Check which nodes are still reachable vs need reassignment
    for (node_id, entry) in registry.peers.iter() {
        print!("{}: {}", node_id[..12.min(node_id.len())], entry.status);

        match &entry.status {
            NodeStatus::Active | NodeStatus::Stale { .. } => {
                if Instant::now().duration_since(
                    entry.last_seen.unwrap_or(Instant::now())
                ).as_secs() > STALE_THRESHOLD_SECS
                {
                    println!(" (stale since {})", entry.original_shard.shard_index);
                }
            }
            _ => {}
        }
    }

    // Start listening for connections...
}
```

### Stale detection logic (around line 277)

```rust
// === LOCATION: drift-coord/src/main.rs stale detector task ===

const STALE_THRESHOLD_SECS: u64 = 30;

tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(15));

    loop {
        interval.tick().await;

        let now = Instant::now();
        let mut any_active = false;

        for (id, last) in seen.iter() {
            if matches!(registry.peers.get(id)?.status, NodeStatus::Active) {
                any_active = true;

                if now.duration_since(*last).as_secs() > STALE_THRESHOLD_SECS {
                    warn!(node = %id, "node appears stale");

                    if let Some(entry) = registry.peers.get_mut(id) {
                        match &entry.status {
                            NodeStatus::Active => {
                                entry.status = NodeStatus::Stale {
                                    since: *last,
                                };

                                // Capture unclaimed shard for later reassignment
                                // The coordinator will give this to next idle node that asks
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        registry.save_to_disk();  // Persist state changes

        if !any_active && registry.is_done() {
            println!("all nodes complete — shutting down...");
            break;
        }
    }
});
```

---

## Phase 8: Shard Reassignment on TrainComplete

### Modify message handler around lines 249-265

```rust
// === LOCATION: drift-coord/src/main.rs listener task (~line 250) ===

match read_message(&mut recv).await? {
    Ok(DriftMessage::TrainProgress(p)) => {
        seen.lock().await.insert(node_id.clone(), Instant::now());

        // Update peer's last_seen in registry
        if let Some(entry) = registry.peers.get_mut(&node_id) {
            entry.last_seen = Some(now);

            // If was marked Stale, return it to Active
            if matches!(entry.status, NodeStatus::Stale { .. }) {
                entry.status = NodeStatus::Active;
            }
        }

        print_progress_update(p.step, p.loss);
    }

    Ok(DriftMessage::AskForMoreWork) => {
        info!(%node_id, "node asked for more work");

        let mut sent_assignment = false;

        // First check: any failed nodes waiting for reassignment?
        if let Some(shard) = registry.pop_failed_shard() {
            write_message(&mut send, &DriftMessage::AssignNext(shard)).await?;
            println!("reassigned shard {} from a failed node", shard.shard_index);
            sent_assignment = true;
        }

        // Second: any pending shards left in queue?
        if !sent_assignment && !registry.pending.is_empty() {
            if let Some(shard) = registry.pop_pending_assignment() {
                write_message(&mut send, &DriftMessage::AssignNext(shard)).await?;
                println!("assigned next shard {}", shard.shard_index);
                sent_assignment = true;
            }
        }

        // Nothing left — tell node to shut down
        if !sent_assignment {
            info!(%node_id, "no more work available");
            write_message(&mut send, &DriftMessage::NoMoreWork).await?;

            if let Some(entry) = registry.peers.get_mut(&node_id) {
                entry.status = NodeStatus::Done;
            }
        } else {
            if let Some(entry) = registry.peers.get_mut(&node_id) {
                entry.status = NodeStatus::Idle;  // Waiting for reassignment
            }
        }
    }

    other => { /* ... existing handling */ }
}
```

---

## Phase 9: Coordinator Persistence on Shutdown

### Add signal handler for clean shutdown (around line 110)

```rust
// === LOCATION: drift-coord/src/main.rs main loop (~line 105) ===

let coord_state = Arc::new(coord_registry);

tokio::select! {
    _ = accept_loop => {}
    _ = tokio::signal::ctrl_c() => {
        println!("\nshutting down coordinator...");

        // Persist state before exiting
        coord_registry.save_to_disk();

        // Send graceful disconnect message to all known peers?
        for (_, entry) in coord_registry.peers.iter() {
            eprintln!(
                "would notify {} of shutdown",
                entry.did_hash_address[..12]
            );
            // TODO: Try sending NoMoreWork to each active node
        }
    }
}

// At program exit, ensure persistence happened
registry.save_to_disk();  // final save
Ok(())
```

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
