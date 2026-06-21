---
library_name: drift
license_name: MPL-2.0 + Commons Clause 1.0
language: en
compatibility:
  - macos
  - linux
---

# Drift

Drift coordinates distributed model training across geo-distributed nodes using encrypted peer-to-peer (p2p) networking via [iroh](https://github.com/n0-computer/iroh). Unlike traditional distributed training that shares gradients and tensor products over the network, each node on this fork trains independently at its own pace without behaving as if it were a tightly synchronized datacenter cluster.

## Setup

### Minimum Requirements

#### Hardware

- Any computer capable of running Rust with network connectivity:
  - Apple M-series Mac with Metal GPU
  - Linux PC with NVIDIA/AMD/Vulkan-compatible GPU
  - CPU-only systems of Linux or Apple flavor (no GPU required)

#### Software

- [Rust](https://rust-lang.org) 1.75+ toolchain

#### Experience

- Familiarity with command-line interfaces (CLI)
- Basic understanding of ML training loops

### Installation

The library can be installed independently:

````sh
# Clone and build
git clone https://github/com/darkshapes/drift
cd drift
cargo build --release

or managed by COVEN:

```sh
git clone https://tangled.org/did:plc:okz7ln6fsh4edhazevnrwsyi coven    # Clone and build
just build-drift
```

Manual COVEN partial installation:
```sh
cd coven/drift
cargo build --release
ln -s $PWD/target/release/drift $HOME/.local/bin/drift            # Symlink binaries
ln -s $PWD/target/release/drift-node $HOME/.local/bin/drift-node
ln -s $PWD/target/release/drift-coord $HOME/.local/bin/drift-coord
````

Ensure `$HOME/.local/bin` is in your PATH.

## Usage Guide

Create an available node

```sh
# Start drift coordinator daemon
drift join   # Can be started on several machines
```

Start training using a specific training repo

```
# Specify peer nodes directly
drift train --peers 4e110...,de4db33f --repo https://github.com/darkshapes/ati
```

### Monitoring Progress

```sh
# Check session status
drift status
```

### Debug Mode

```sh
RUST_LOG=debug ./target/release/drift join
```

### Full Command Reference

### Environment Variables

Environment variables for training are loaded from `.env.shared` in the current working directory by default. Create this file to inject environment variables into the training process.

```sh
# Example .env.shared
HF_TOKEN=your_token_here
WANDB_API_KEY=your_wandb_key
```

```sh
# Override the default env file location
drift train --env-file /path/to/custom.env --repo https://github.com/org/repo
```

| Flag                      | Description                                       |
| ------------------------- | ------------------------------------------------- |
| `--env-file <path>`       | Path to environment file (default: `.env.shared` in cwd) |
| `--join <token>`          | Join session via invitation token                 |
| `train`                   | Enter training mode as coordinated peer           |
| `--peer <id>`             | Connect to specific peer node by ID               |
| `--model-path <path>`     | Path to model checkpoint file (.pt, .safetensors) |
| `--dataset-path <path>`   | Path to training dataset directory                |
| `--epochs <n>`            | Number of training epochs (default: 10)           |
| `--batch-size <n>`        | Batch size per step (default: 32)                 |
| `--resume`                | Resume from last checkpoint                       |
| `--checkpoint-dir <path>` | Directory for checkpoint files                    |

## Architecture

### End-to-End Operation

| Library                        | Purpose                                                      |
| ------------------------------ | ------------------------------------------------------------ |
| [drift-auth](../drift-auth/)   | Ed25519 key generation, signing, token validation, LRU cache |
| [drift-proto](../drift-proto/) | Message types, serialization, protocol framing, ALPN         |
| [drift-coord](../drift-coord/) | Peer discovery, session negotiation, training orchestration  |
| [drift-node](../drift-node/)   | node runtime, VRAM tracking, status monitoring               |

```
                       ,----------------------------.
                      | ,--------------------------. |
                      || ,------------------------. ||   C.
                      ||| ,----------------------. |||   TrainConfig
                      |||| ,--------------------. ||||   ShardAssignment
                      |||||                      |||||   CheckpointInfo
                      |||||     drift-auth<--.   |||||   TrainingReady/TrainingCancel
                     ╲|||||╱       E.         |  |||||   NoMoreWork
                      ╲|||╱        RepoCommit |  |||||   AssignNext
              A. Join  ╲|╱                    |  |||||
    drift-cli ------> drift-node--------.     |  |||||
    drift-cli ------> drift-node---------\    |  |||||
    drift-cli ------> drift-node---------- >= |drift-coord
    drift-cli ------> drift-node---------/        ╱|╲
    drift-cli ------> drift-node--------'          |
                |                  D.              | B. Train
                |                  NodeInfo        |
                |                  TrainProgress   |
                |                  AskForMoreWork  |
                |                  Heartbeat       |
                |                                  |
                 `--------------------------------'
```

| library     | tasks |
| ----------- | ----- |
| drift-cli   | A, B  |
| drift-coord | C     |
| drift-node  | D     |
| drift-auth  | E     |

### Communication Protocol

```
COORDINATOR TASK                          NODE TASK
────────────────────────────────────────────────────────
cli launch ──────────────────────→                                ┓
send Ping                                                         ┃ initialization
                       ←────────────────── receive Ping           ┛
                       send NodeInfo                              ┓
receive NodeInfo ──────────────────────→                          ┃ exchange
send TrainConfig                                                  ┃ specifications
                       ←────────────────── receive TrainConfig    ┛
                       send RepoCommit                            ┓ verification
receive RepoCommit────────────────────→                           ┛ handshake
send TrainingReady, ShardAssignment
                       ←────────────────── receive TrainingReady, ┓ verification
                      send TrainProgress        ShardAssignment   ┛ success
                                                                    -or-
receive RepoCommit──────────────────────────┐                     ┓ verification
send TrainingCancel                         │                     ┛ failure
                                            │
                                            ↓
                                           end
```

#### Protocol Flow (ALPN: `drift/0`)

1. Coordinator initiates QUIC connection to node, sends `Ping`
2. Node gets Ping, sends `NodeInfo` containing GPU name, VRAM, compute capability
3. Coordinator broadcasts training repo path in `TrainConfig`,
4. Node responds with `RepoCommit` containing training information signed by the node's Iroh key.
5. Coordinator receives all node commit hashes. Training ends with `TrainingCancel` broadcast unless peer information matches, signaled by `TrainingReady` broadcast.
6. Node responds to `TrainingReady` with TrainProgress or shuts down if `TrainingCancel`
7. Coordinator broadcasts `ShardAssignment` to all nodes.
8. Nodes downloads data, and execute at their own pace.
9. Nodes stream `TrainProgress` updates back to coordinator
10. If a node fails, its work is queued for redistribution by the coordinator as `AssignNext`.
11. When any node completes its task it will `AskForMoreWork`.
12. On receiving NoMoreWork or timeout, nodes shut down.

All traffic is encrypted end-to-end via QUIC.<br>
NAT hole-punching is handled automatically by iroh, with relay fallback.<br>
Messages are length-prefixed JSON over QUIC bidirectional streams.<br>
Shard count is fixed at initialization. It cannot be changed during training yet.

#### Verification Handshake

During verification on the coordinator:

1. Per-node signature verification: Each node's node_id, commit, and repo_url are compared together against that node's signature
2. Cross-node consistency check: All nodes' commit hashes are compared against each other to ensure they match

more information in [CONTRIBUTING](CONTRIBUTING.md)
