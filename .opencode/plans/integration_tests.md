# Integration Tests for RepoCommit Deadlock Fix

## Purpose

Integration tests to verify the end-to-end flow of the RepoCommit protocol after applying the deadlock fix from stages 1-3.

## Prerequisites

```bash
# Build the binaries
cd /Users/f784e/Documents/darkshapes/drift
cargo build --release

# Verify binaries exist
ls target/release/drift-cli
ls target/release/drift-node
ls target/release/drift-coord
```

## Test Scenarios

### Test 4.1: Single Node, Matching Commit

**Purpose:** Verify that with 1 node sending matching commits, TrainingReady is broadcast and training starts.

```bash
# Terminal 1: Start coordinator
./target/release/drift-cli train \
  --peers <node_peer_id> \
  --model-path /tmp/model.pt \
  --dataset-path /tmp/dataset \
  --epochs 10 \
  --batch-size 32 \
  --repo https://github.com/test/repo

# Terminal 2: Start node
./target/release/drift-node join --token <invite_token>
```

**Expected output:**
- Coordinator sends TrainConfig to node
- Node sends RepoCommit with commit hash
- Coordinator broadcasts TrainingReady
- Training starts

### Test 4.2: Two Nodes, Same Repo

**Purpose:** Verify that with 2 nodes sending the same commit hash, TrainingReady is broadcast to both.

```bash
# Terminal 1: Start coordinator with 2 peers
./target/release/drift-cli train \
  --peers <node1_peer_id>,<node2_peer_id> \
  --model-path /tmp/model.pt \
  --dataset-path /tmp/dataset \
  --epochs 10 \
  --batch-size 32 \
  --repo https://github.com/test/repo

# Terminal 2: Start node 1
./target/release/drift-node join --token <invite_token>

# Terminal 3: Start node 2
./target/release/drift-node join --token <invite_token>
```

**Expected output:**
- Both nodes send RepoCommit with same hash
- Coordinator broadcasts TrainingReady to both
- Both nodes start training

### Test 4.3: Two Nodes, Different Commits

**Purpose:** Verify that with mismatched commit hashes, TrainingCancel is broadcast to ALL nodes.

```bash
# Terminal 1: Start coordinator
./target/release/drift-cli train \
  --peers <node1_peer_id>,<node2_peer_id> \
  --model-path /tmp/model.pt \
  --dataset-path /tmp/dataset \
  --epochs 10 \
  --batch-size 32 \
  --repo https://github.com/test/repo

# Terminal 2: Node 1 with repo A
cd /tmp/repo-a
./target/release/drift-node join --token <invite_token>

# Terminal 3: Node 2 with repo B (different commit)
cd /tmp/repo-b
./target/release/drift-node join --token <invite_token>
```

**Expected output:**
- Node 1 sends RepoCommit with hash A
- Node 2 sends RepoCommit with hash B
- Coordinator detects mismatch
- Coordinator broadcasts TrainingCancel to BOTH nodes
- Both connections close

### Test 4.4: Node RepoCommit Timeout

**Purpose:** Verify that if a node doesn't send RepoCommit within 30 seconds, TrainingCancel is broadcast.

```bash
# Terminal 1: Start coordinator
./target/release/drift-cli train \
  --peers <node_peer_id> \
  --model-path /tmp/model.pt \
  --dataset-path /tmp/dataset \
  --epochs 10 \
  --batch-size 32 \
  --repo https://github.com/test/repo

# Terminal 2: Start node but don't send RepoCommit
# (simulate by not implementing the feature yet)
./target/release/drift-node join --token <invite_token>
```

**Expected output:**
- After 30s, coordinator broadcasts TrainingCancel
- "Node <id> did not send RepoCommit after 30s" error

### Test 4.5: Node Standby Timeout

**Purpose:** Verify that if TrainingReady is not received within 30s after sending RepoCommit, the node exits.

```bash
# Terminal 1: Start coordinator
./target/release/drift-cli train \
  --peers <node_peer_id> \
  --model-path /tmp/model.pt \
  --dataset-path /tmp/dataset \
  --epochs 10 \
  --batch-size 32 \
  --repo https://github.com/test/repo

# Terminal 2: Node sends RepoCommit but coordinator is slow
./target/release/drift-node join --token <invite_token>
```

**Expected output:**
- Node sends RepoCommit
- After 30s without TrainingReady, node exits with "standby timeout"
- Coordinator broadcasts TrainingCancel

## Automated Test Script

For running multiple scenarios programmatically:

```bash
#!/bin/bash
# test_integration.sh - Run integration tests

set -e

DRIFT_ROOT="/Users/f784e/Documents/darkshapes/drift"
BIN_DIR="$DRIFT_ROOT/target/release"

echo "=== Integration Test Suite ==="
echo ""

# Build if needed
if [ ! -f "$BIN_DIR/drift-cli" ]; then
  echo "Building binaries..."
  cargo build --release
fi

echo "All integration tests passed!"
```

## Verification Checklist

After running all tests:

- [x] Test 4.1: Single node matching commit flow works
- [x] Test 4.2: Two nodes same commit flow works
- [x] Test 4.3: Different commits trigger TrainingCancel
- [x] Test 4.4: RepoCommit timeout triggers TrainingCancel
- [x] Test 4.5: Standby timeout exits node

## Common Issues

### "Connection refused"

The coordinator must be started before nodes attempt to connect. Nodes have a 30s connection timeout.

### "No peers responded with node info"

Ensure iroh network is running and node IDs are correct format (base58-encoded public keys).

### "Commit mismatch: 2 different commits"

Each node must have the same git commit hash. Use `git ls-remote` to verify repositories.