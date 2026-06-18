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
  - Generic CPU-only system (limited throughput)

#### Software

- [`just`](https://github.com/casey/just#packages) to build library
- [Rust](https://rust-lang.org) 1.75+ toolchain

#### Experience

- Familiarity with command-line interfaces (CLI)
- Basic understanding of ML training loops

### Build

From root folder:

```sh
just build-drift
```

Or manually:

```sh
cd drift
cargo build --release
ln -s <path/to/clone>/target/release/drift $HOME/.local/bin/drift
ln -s <path/to/clone>/target/release/drift-coord $HOME/.local/bin/drift-coord
ln -s <path/to/clone>/target/release/drift-node $HOME/.local/bin/drift-node
```

Restart shell after creating symlinks.

### Manual Installation

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

| Flag                      | Description                                       |
| ------------------------- | ------------------------------------------------- |
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

| library     | tasks |
| ----------- | ----- |
| drift-cli   | A, B  |
| drift-coord | C     |
| drift-node  | D     |
| drift-auth  | E     |

```
                       ,----------------------------.     TrainConfig
                      | ,--------------------------. |    ShardAssignment
                      || ,------------------------. || C. CheckpointInfo
                      ||| ,----------------------. |||    TrainingReady/TrainingCancel
                      |||| ,--------------------. ||||    NoMoreWork
                     ╲|||||╱                     |||||    AssignNext
         drift-cli -> drift-node -------.        |||||
         drift-cli -> drift-node --------\       |||||
A. Join  drift-cli -> drift-node --------- >= drift-coord
         drift-cli -> drift-node --------/        ╱|╲
         drift-cli -> drift-node -------'          |
             |        | ╱|╲        D.              | B. Train
             |       ╲|╱ |          NodeInfo       |
             |     drift-auth       TrainProgress  |
             |     E. RepoCommit    AskForMoreWork |
             |           			Heartbeat      |
             | 		                               |
              `-----------------------------------'
```

### Protocol Flow (ALPN: `drift/0`)

1. Coordinator initiates QUIC connection to node, sends `Ping` and broadcasts training repo path in `TrainConfig`,
2. Node responds with `RepoCommit` containing the commit hash of the current branch signed by the node Iroh key.
3. Coordinator receives all node commit hashes. Training ends with `TrainingCancel` broadcast unless all commits match, signaled by `TrainingReady` broadcast.
4. Node responds to `TrainingReady` with `NodeInfo` containing GPU name, VRAM, compute capability (or shuts down if `TrainingCancel`)
5. Coordinator broadcasts `ShardAssignment` to all nodes.
6. Nodes download training scripts, data, and execute at their own pace.
7. Nodes stream `TrainProgress` updates back to coordinator
8. If a node fails, work is held by coordinator as `AssignNext`, awaiting next free node to `AskForMoreWork`
9. On receiving NoMoreWork or timeout, nodes shut down.

All traffic is encrypted end-to-end via QUIC.<br>
NAT hole-punching is handled automatically by iroh, with relay fallback.<br>
Messages are length-prefixed JSON over QUIC bidirectional streams.<br>

### Project Structure

```
drift/
├── Cargo.toml              # Workspace manifest
│
├── drift-auth/             # Authentication library
│   └── src/lib.rs          # Ed25519, token validation, LRU cache
│
├── drift-proto/            # Protocol definitions
│   ├── src/lib.rs           # Message types, framing, ALPN "drift/0"
│   └── tests/
│       ├── integration.rs   # Full handshake test suite
│       ├── training.rs      # End-to-end training pipeline
│       └── stress.rs        # Bulk message and gradient tests
│
├── drift-coord/           # Coordinator binary
│   └── src/
│       ├── main.rs           # CLI: coord, train commands
│       ├── scheduler.rs      # Shard assignment by GPU capability
│       ├── checkpoint.rs     # Checkpoint management
│       └── monitor.rs        # Health monitoring, progress display
│
├── drift-node/            # Node binary
│   └── src/
│       ├── main.rs          # CLI: join, status
│       ├── gpu.rs           # GPU detection (nvidia-smi, apple metal)
│       ├── network.rs       # iroh endpoint, connection handling
│       └── training.rs      # Local training loop subprocess
│
├── drift-cli/              # Unified CLI entry point
│   └── src/
│       ├── main.rs          # CLI: join, train, status, coord
│       ├── node.rs          # Node lifecycle logic
│       └── coord.rs         # Coordinator logic
│
└── examples/
    ├── mock_train.py       # Mock training script for testing
    └── train.yaml          # Example training configuration
```

### Purpose

Distributed training coordination for consumer hardware (GPUs and CPUs) that:

- Avoids gradient sharing and allgather operations entirely
- Trains nodes independently in a ring-free, GLOO-free, NVLink-free architecture
- Supports Apple Silicon Metal, NVIDIA CUDA, AMD ROCm, and Vulkan backends
- Communicates over QUIC-encrypted iroh tunnels with automatic NAT hole-punching

### Libraries
