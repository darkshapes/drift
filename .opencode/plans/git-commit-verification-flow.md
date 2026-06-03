# Git Commit Verification Workflow - Implementation Plan

## Overview

This plan implements a strict commit verification protocol where:
1. Nodes send `RepoCommit` BEFORE receiving any training configuration
2. Coordinator verifies ALL signatures and ALL commits match
3. Coordinator broadcasts `TrainingReady` ONLY when all conditions met
4. Any failure triggers `TrainingCancel` to ALL nodes immediately
5. Nodes wait 30 seconds for `TrainingReady`, then cancel with "timeout"
6. No caching - fresh `git ls-remote` every time

---

## Protocol Message Flow

```
Node                              Coordinator
  |                                   |
  |--- (connect) -------------------->|
  |<-- (accept) ---------------------|
  |--- Ping ------------------------>|
  |<-- NodeInfo --------------------|
  |--- RepoCommit ------------------>| (sent immediately after NodeInfo)
  |                                   | (collect from all nodes)
  |                                   | (verify signatures)
  |                                   | (check all commits match)
  |<-- TrainingReady OR Cancel ------| (broadcast to ALL)
  |<-- TrainingCancel --------------->| (if any failure)
  |                                   |
  | (30s standby timeout)            |
  |                                   | (process ends after cancel)
```

---

## Required Changes

### File 1: `drift-proto/src/lib.rs`

#### Add `TrainingCancel` struct (after line 348, before `DriftMessage` enum)

```rust
/// Training cancellation message from coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingCancel {
    pub reason: String,
    pub time: String,   // RFC 3339 timestamp (ISO 8601 subset) when cancel sent
    pub repo_url: String,
}
```

#### Add `TrainingCancel` variant to `DriftMessage` enum (line 387, before `AuthChallenge`)

```rust
pub enum DriftMessage {
    // ... existing variants ...
    
    /// Coordinator broadcasts: all commits verified, begin training.
    TrainingReady,
    
    /// Coordinator broadcasts: commit verification failed, abort.
    TrainingCancel(TrainingCancel),
    
    /// Coordinator sends to node: "please authenticate"
    AuthChallenge(AuthMessage),
    
    // ... rest of variants ...
}
```

#### Update `fmt::Display for DriftMessage` (line 251, add TrainingCancel case)

```rust
Self::TrainingReady => write!(f, "TrainingReady"),
Self::TrainingCancel(c) => write!(f, "TrainingCancel(reason={}, time={}, repo={})", 
    c.reason, c.time, c.repo_url),
Self::RepoCommit(rc) => write!(f, "RepoCommit(commit={}, repo={})", 
    &rc.commit[..8.min(rc.commit.len())], rc.repo_url),
```

---

### File 2: `drift-cli/src/node.rs`

#### Modify `handle_connection()` function (lines 223-299)

**Current flow:**
- Receive Ping
- Send NodeInfo
- Loop receiving TrainConfig, ShardAssignment, etc.
- Start training immediately after ShardAssignment

**New flow:**
- Receive Ping
- Send NodeInfo
- **Immediately send RepoCommit** (before receiving anything else)
- Wait for TrainingReady OR TrainingCancel (30s timeout)
- **ONLY** after TrainingReady, receive TrainConfig and ShardAssignment
- Begin training

#### Code structure for new `handle_connection()`:

```rust
async fn handle_connection(
    conn: iroh::endpoint::Connection,
    node_info_msg: DriftMessage,
    endpoint: Endpoint,
) -> Result<()> {
    let remote = conn.remote_id();
    info!(%remote, "coordinator connected");

    let (mut send, mut recv) = conn.accept_bi().await?;

    // Wait for initial Ping
    let msg = read_message(&mut recv).await?;
    if !matches!(msg, DriftMessage::Ping) {
        anyhow::bail!("expected Ping, got {}", msg);
    }

    // Send node info
    write_message(&mut send, &node_info_msg).await?;
    info!("sent node info");

    // Extract repo_url from future config (will be received later)
    // For now, we need to send RepoCommit - but we don't have config yet
    // Solution: send empty RepoCommit, wait for config, then resend?
    // Better: wait for TrainConfig first, THEN send RepoCommit
    
    // NEW FLOW:
    // 1. Receive TrainConfig first (contains repo_url)
    // 2. Send RepoCommit with signed commit
    // 3. Wait for TrainingReady (30s timeout)
    // 4. Receive ShardAssignment
    // 5. Begin training

    let mut train_config = None;
    let mut repo_commit_sent = false;
    let mut shard_assignment = None;
    let standby_start = std::time::Instant::now();

    loop {
        match read_message(&mut recv).await {
            Ok(msg) => match msg {
                DriftMessage::Ping => {
                    write_message(&mut send, &DriftMessage::Pong).await?;
                }
                DriftMessage::TrainConfig(config) => {
                    info!(model = %config.model_path, epochs = config.epochs, "received config");
                    
                    // If this is first config, send RepoCommit
                    if !repo_commit_sent {
                        let repo_url = config.train_repo_url
                            .as_ref()
                            .ok_or_else(|| anyhow::bail!("No train_repo_url in config"))?;
                        
                        // Run git ls-remote fresh every time
                        let repo_path = find_local_repo(repo_url)
                            .ok_or_else(|| anyhow::bail!("Repo not found locally"))?;
                        
                        let commit_hash = run_git_ls_remote(&repo_path)
                            .ok_or_else(|| anyhow::bail!("git ls-remote failed"))?;
                        
                        // Sign commit + repo_url
                        let signature = sign_with_iroh_key(&commit_hash, repo_url)?;
                        
                        let repo_commit = RepoCommit {
                            commit: commit_hash,
                            repo_url: repo_url.clone(),
                            signature,
                        };
                        
                        write_message(&mut send, &DriftMessage::RepoCommit(repo_commit)).await?;
                        repo_commit_sent = true;
                    }
                    
                    train_config = Some(config);
                }
                DriftMessage::TrainingReady => {
                    info!("TrainingReady received, awaiting shard assignment");
                    // Now wait for ShardAssignment
                    continue;
                }
                DriftMessage::TrainingCancel(cancel) => {
                    error!("Training cancelled: {} (reason: {}, time: {})", 
                        cancel.repo_url, cancel.reason, cancel.time);
                    return Err(anyhow!("Training cancelled: {}", cancel.reason));
                }
                DriftMessage::ShardAssignment(s) => {
                    info!(shard_index = s.shard_index, size = s.size(), "received shard");
                    shard_assignment = Some(s);
                    
                    // Only start training if we received TrainingReady
                    if train_config.is_some() {
                        info!("starting training after TrainingReady");
                        run_training(
                            train_config.as_ref().unwrap(),
                            &mut send,
                            &mut recv,
                            shard_assignment.as_ref(),
                        ).await?;
                    } else {
                        return Err(anyhow!("ShardAssignment received without TrainingReady"));
                    }
                }
                DriftMessage::Heartbeat { .. } => {
                    write_message(&mut send, &DriftMessage::Heartbeat { uptime_secs: 0 }).await?;
                }
                DriftMessage::TrainComplete => {
                    info!("training complete");
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
        
        // Check 30s standby timeout
        if standby_start.elapsed() > std::time::Duration::from_secs(30) {
            return Err(anyhow!("Standby timeout: no TrainingReady after 30s"));
        }
    }

    Ok(())
}
```

#### Add helper functions (after line 524, in same file)

```rust
/// Find local repo path for given URL. Splits at domain boundaries (.com/, .org/, etc.)
fn find_local_repo(repo_url: &str) -> Option<std::path::PathBuf> {
    // Extract repo name by splitting at domain boundaries
    let repo_name = repo_url
        .split('/')
        .collect::<Vec<_>>()
        .windows(2)
        .find(|w| w[0].ends_with(".com") || w[0].ends_with(".org") || 
                w[0].ends_with(".network") || w[0].ends_with(".net"))
        .map(|w| w[1])
        .unwrap_or_else(|| repo_url.split('/').last().unwrap_or("repo"));
    
    // Check ~/.local/state/covn/<repo>
    if let Some(home) = std::env::var_os("HOME") {
        let covn_path = std::path::Path::new(&home)
            .join(".local")
            .join("state")
            .join("covn");
        if covn_path.exists() {
            let repo_path = covn_path.join(repo_name);
            if repo_path.exists() {
                return Some(repo_path);
            }
        }
    }
    
    // Check ~/.local/state/drift/<repo>
    if let Some(home) = std::env::var_os("HOME") {
        let drift_path = std::path::Path::new(&home)
            .join(".local")
            .join("state")
            .join("drift");
        if drift_path.exists() {
            let repo_path = drift_path.join(repo_name);
            if repo_path.exists() {
                return Some(repo_path);
            }
        }
    }
    
    None
}

/// Run git ls-remote and extract commit hash.
fn run_git_ls_remote(repo_path: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["ls-remote", repo_path.to_string_lossy().as_ref(), "HEAD"])
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse: "abc123  HEAD" -> extract "abc123"
    stdout.lines().next()
        .map(|line| line.split_whitespace().next()?.to_string())
        .flatten()
}

/// Sign commit + repo_url with iroh keypair.
fn sign_with_iroh_key(commit: &str, repo_url: &str) -> anyhow::Result<Vec<u8>> {
    // Get iroh keypair from endpoint
    // Sign: commit + repo_url
    // Return: Vec<u8> signature
    // TODO: implement actual signing
    Ok(vec![])
}
```

---

### File 3: `drift-cli/src/coord.rs`

#### Modify `train()` function (lines 13-336)

**Current flow:**
- Connect to peers
- Receive NodeInfo
- Send TrainConfig, ShardAssignment
- Monitor training

**New flow:**
- Connect to peers
- Receive NodeInfo
- **Collect RepoCommit from each node** (with 30s timeout per node)
- **Verify all signatures**
- **Check all commits match**
- **If any failure:** broadcast TrainingCancel to ALL nodes, exit
- **If all match:** broadcast TrainingReady to ALL nodes
- **Then** send TrainConfig (with git_commit field) and ShardAssignment

#### Code structure for new `train()`:

```rust
pub async fn train(
    repo: String,
    peer_ids: Vec<String>,
    _script: Option<String>,
    model_path: String,
    dataset_path: String,
    batch_size: u32,
    learning_rate: f64,
    epochs: u32,
    dataset_size: u64,
    checkpoint_dir: String,
    resume: bool,
) -> Result<()> {
    if peer_ids.is_empty() {
        anyhow::bail!("no peers specified. Use --peers <node_id1>,<node_id2>");
    }

    let started = Instant::now();
    println!("drift coordinator starting");

    // ... resume checkpoint logic ...

    println!("  Peers: {}", peer_ids.len());

    let endpoint = Endpoint::builder()
        .alpns(vec![DRIFT_ALPN.to_vec()])
        .bind()
        .await?;

    let coord_id = endpoint.id();
    info!(coord_id = %coord_id, "coordinator endpoint bound");

    let train_config = TrainConfig {
        model_path,
        dataset_path,
        batch_size,
        learning_rate,
        epochs,
        train_repo_url: Some(repo.clone()),
        script_entrypoint: None,
        dataset_repo_url: None,
        model_artifact_ref: None,
        enable_auth: false,
        auth_threshold: 3,
    };

    // Connect to each peer and collect node info
    let mut node_infos: Vec<NodeInfo> = Vec::new();
    let mut connections: Vec<(SendStream, RecvStream)> = Vec::new();

    for peer_id_str in &peer_ids {
        let public_key = PublicKey::from_str(peer_id_str)
            .with_context(|| format!("invalid node ID: {}", peer_id_str))?;

        let conn = tokio::time::timeout(
            Duration::from_secs(30),
            endpoint.connect(public_key, DRIFT_ALPN),
        )
        .await
        .with_context(|| format!("connection to {} timed out after 30s", peer_id_str))?
        .with_context(|| format!("failed to connect to {}", peer_id_str))?;

        let (mut send, mut recv) = conn.open_bi().await?;

        // Send Ping
        write_message(&mut send, &DriftMessage::Ping).await?;

        // Receive NodeInfo
        let msg = read_message(&mut recv).await?;
        match msg {
            DriftMessage::NodeInfo(info) => {
                println!("  Connected: {}", info);
                node_infos.push(info);
            }
            other => {
                warn!(%other, "expected NodeInfo, skipping peer");
                continue;
            }
        }

        connections.push((send, recv));
    }

    if node_infos.is_empty() {
        anyhow::bail!("no peers responded with node info");
    }

    // Collect RepoCommit from each node (30s timeout per node)
    let mut repo_commits: Vec<(String, RepoCommit)> = Vec::new();
    let standby_start = Instant::now();

    for (i, (mut send, mut recv)) in connections.iter_mut().enumerate() {
        let node_id = node_infos[i].node_id.clone();
        let commit_start = Instant::now();

        loop {
            // Check 30s timeout
            if commit_start.elapsed() > Duration::from_secs(30) {
                // Timeout! Send TrainingCancel to ALL nodes and exit
                broadcast_training_cancel(
                    &connections,
                    &format!("Node {} did not send RepoCommit after 30s", node_id),
                    &repo,
                ).await?;
                return Err(anyhow!("Node {} timeout", node_id));
            }

            match read_message(&mut recv).await {
                Ok(DriftMessage::RepoCommit(commit)) => {
                    // Verify signature
                    verify_repo_commit(&commit, &node_id)?;
                    repo_commits.push((node_id.clone(), commit));
                    break;
                }
                Ok(other) => {
                    warn!(%other, "unexpected message from node {}", node_id);
                }
                Err(e) => {
                    // Connection error! Send TrainingCancel to ALL and exit
                    broadcast_training_cancel(
                        &connections,
                        &format!("Node {} connection error: {}", node_id, e),
                        &repo,
                    ).await?;
                    return Err(anyhow!("Node {} error: {}", node_id, e));
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    // Check all commits match
    let commits: Vec<&String> = repo_commits.iter().map(|(_, c)| &c.commit).collect();
    let unique_commits: std::collections::HashSet<_> = commits.iter().collect();

    if unique_commits.len() != 1 {
        // Conflict! Send TrainingCancel to ALL and exit
        broadcast_training_cancel(
            &connections,
            &format!("Commit hash mismatch detected: {} different commits", unique_commits.len()),
            &repo,
        ).await?;
        return Err(anyhow!("Commit mismatch: {} different commits", unique_commits.len()));
    }

    let agreed_commit = commits[0].clone();

    // All verified! Broadcast TrainingReady to ALL nodes
    for (send, _) in &connections {
        write_message(send, &DriftMessage::TrainingReady).await?;
    }

    // NOW send TrainConfig (with git_commit) and ShardAssignment
    let assignments = assign_shards(&node_infos, dataset_size);
    let total_vram: u64 = node_infos.iter().map(|n| n.gpu_vram_mb).sum();

    println!();
    println!("All peers connected in {:.1}s", started.elapsed().as_secs_f64());
    println!();
    println!("Commit verified: {}", agreed_commit);
    println!("Broadcasting TrainingReady...");
    println!();

    for (i, (send, _)) in connections.iter_mut().enumerate() {
        let mut config = train_config.clone();
        config.git_commit = Some(agreed_commit.clone());
        write_message(send, &DriftMessage::TrainConfig(config)).await?;
        write_message(send, &DriftMessage::ShardAssignment(assignments[i].clone())).await?;
        info!(node = %node_infos[i].node_id, "sent config, shard");
    }

    // ... rest of training monitoring logic ...
}

/// Broadcast TrainingCancel to all connected nodes. Exits immediately after broadcast.
async fn broadcast_training_cancel(
    connections: &[(SendStream, RecvStream)],
    reason: &str,
    repo_url: &str,
) -> Result<()> {
    let cancel = TrainingCancel {
        reason: reason.to_string(),
        time: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        repo_url: repo_url.to_string(),
    };
    for (send, _) in connections {
        let _ = write_message(send, &DriftMessage::TrainingCancel(cancel.clone())).await;
    }
    // Exit immediately after broadcast
    std::process::exit(1);
}

/// Verify RepoCommit signature.
fn verify_repo_commit(commit: &RepoCommit, node_id: &str) -> Result<()> {
    // Parse node_id as PublicKey
    let pubkey = PublicKey::from_str(node_id)
        .map_err(|_| anyhow::anyhow!("Invalid node ID: {}", node_id))?;
    
    // Verify: sign(commit + repo_url) matches signature
    // TODO: implement actual verification using iroh pubkey
    Ok(())
}
```

---

## Testing Strategy

### Test 1: All nodes with same commit
- Setup: 2 nodes, both with commit "abc123"
- Expected: Coordinator broadcasts TrainingReady, nodes proceed to training

### Test 2: Nodes with different commits
- Setup: Node A has "abc123", Node B has "def456"
- Expected: Coordinator sends TrainingCancel with "Commit hash mismatch" to ALL nodes

### Test 3: Invalid signature
- Setup: Node sends RepoCommit with fake signature
- Expected: Coordinator sends TrainingCancel with "unauthorized" to ALL nodes

### Test 4: Node timeout (no RepoCommit)
- Setup: Node does not send RepoCommit within 30s
- Expected: Coordinator sends TrainingCancel with timeout reason to ALL nodes

### Test 5: Coordinator crash before TrainingReady
- Setup: Coordinator exits without sending TrainingReady
- Expected: Nodes timeout after 30s, error with "Standby timeout"

### Test 6: Fresh git ls-remote every time
- Setup: Change commit between two node joins
- Expected: Second join uses new commit, not cached value

---

## Implementation Order

1. **Phase 1: Protocol changes** (`drift-proto/src/lib.rs`)
   - Add `TrainingCancel` struct
   - Add `TrainingCancel` variant to `DriftMessage`
   - Update `Display` implementations

2. **Phase 2: Node changes** (`drift-cli/src/node.rs`)
   - Modify `handle_connection()` to wait for TrainingReady
   - Add `find_local_repo()` helper
   - Add `run_git_ls_remote()` helper
   - Add `sign_with_iroh_key()` helper

3. **Phase 3: Coordinator changes** (`drift-cli/src/coord.rs`)
   - Modify `train()` to collect RepoCommit before sending config
   - Add `broadcast_training_cancel()` helper
   - Add `verify_repo_commit()` helper

4. **Phase 4: Tests** (`drift-proto/tests/repo_commit_verification.rs`)
   - Add integration tests for new flow
   - Add timeout tests
   - Add mismatch tests

---

## Summary

This plan implements a strict commit verification protocol with:
- **NO caching** - fresh `git ls-remote` every time
- **NO training** until TrainingReady is received
- **30s timeout** for RepoCommit collection and TrainingReady waiting
- **Immediate cancel** to ALL nodes on any failure
- **Coordinator exits** after sending TrainingCancel

The key insight is that the entire flow must be reordered:
- **OLD:** NodeInfo → TrainConfig → ShardAssignment → Training
- **NEW:** NodeInfo → RepoCommit → (verify all) → TrainingReady → TrainConfig → ShardAssignment → Training
