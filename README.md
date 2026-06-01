---
library_name: drift
license_name: MPL-2.0 + Commons Clause 1.0
language: en
compatibility:
  - macos
  - linux
---

# Drift

Drift coordinates distributed model training across COVEN peer nodes using encrypted peer-to-peer (p2p) networking via [iroh](https://github.com/n0-computer/iroh). Unlike traditional distributed training that shares gradients and tensor products over the network, each node trains independently at its own pace without synchronization overhead.

## Overview

### Purpose

Distributed training coordination for consumer hardware (GPUs and CPUs) that:

- Avoids gradient sharing and allgather operations entirely
- Trains nodes independently in a ring-free, GLOO-free, NVLink-free architecture
- Supports Apple Silicon Metal, NVIDIA CUDA, AMD ROCm, and Vulkan backends
- Communicates over QUIC-encrypted iroh tunnels with automatic NAT hole-punching

## End-to-End Operation

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Coordinator  │◄───│   Node A    │◄───│   Node B     │
└─────────────┘     └─────────────┘     └─────────────┘
        │                     │                     │
   ALPN: drift/0         ping/pong           TrainConfig/ShardAssignment
```

### Protocol Flow (ALPN: `drift/0`)

1. **Handshake**: Coordinator initiates QUIC connection to node, sends `Ping`
2. **Discovery**: Node responds with `NodeInfo` containing GPU name, VRAM, compute capability
3. **Configuration**: Coordinator sends `TrainConfig`, `ShardAssignment`
4. **Training Loop**:
   - Nodes train locally at their own pace
   - Nodes send periodic `BarrierSync` per step
   - Coordinator replies `BarrierReady` for checkpoint coordination
5. **Progress**: Node streams `TrainProgress` updates back to coordinator
6. **Finalization**: Checkpoint aggregation on coordinated barrier

All traffic is encrypted end-to-end via QUIC. NAT hole-punching is handled automatically by iroh, with relay fallback. Messages are length-prefixed JSON over QUIC bidirectional streams.

## Architecture

### Libraries

| Library                        | Purpose                                                      |
| ------------------------------ | ------------------------------------------------------------ |
| [drift-auth](../drift-auth/)   | Ed25519 key generation, signing, token validation, LRU cache |
| [drift-proto](../drift-proto/) | Message types, serialization, protocol framing, ALPN         |
| [drift-coord](../drift-coord/) | Peer discovery, session negotiation, training orchestration  |
| [drift-node](../drift-node/)   | COVEN node runtime, VRAM tracking, status monitoring         |

### CLI Binaries

```
┌──────────────┐     ┌─────────────────┐     ┌───────────────────┐
│  drift-cli   │     │   drift-node    │     │    drift-coord    │
├────────────────────────────────────────────────────────────────┤
│         Unified entry point (join/train/status)                │
└──────────────┘     └─────────────────┘     └───────────────────┘
```

## Development Notes

### What's Removed

- All shared memory operations
- Gradient synchronization and ring scatter-reduce
- Allgather collectives
- Torch Distributed / DDP functions
- Tensor products shared over network
- NVLink-aware tensor sharding

### What's Added

- Apple device recognition and Metal GPU detection
- Independent local training with checkpoint coordination
- Periodic barrier sync without gradient exchange

### Build Artifacts

Drift builds to `target/release/`. Binary artifacts should be moved, copied, or symlinked to a static folder:

```
drift/target/release/drift           # Main CLI binary
drift/target/release/drift-node      # Node binary
drift/target/release/drift-coord     # Coordinator binary
```

On MacOS, building `drift` may require permission from `integration`, `stress`, and `training` packages.

## Setup

The library is managed by nocturne through `covn`, and otherwise remains independent of other libraries. Install using the root build command.

## Minimum Requirements

### Hardware

- Any computer capable of running Rust with network connectivity:
  - Apple M-series Mac with Metal GPU
  - Linux PC with NVIDIA/AMD/Vulkan-compatible GPU
  - Generic CPU-only system (limited throughput)

### Software

- [`just`](https://github.com/casey/just#packages) to build library
- [Rust](https://rust-lang.org) 1.75+ toolchain

### Experience

- Familiarity with command-line interfaces (CLI)
- Basic understanding of ML training loops

## Usage Guide

### Starting a Session as Coordinator

```sh
# Start drift coordinator daemon
drift coord --port 7842 &
```

### Joining as Training Peer

```sh
# Join an existing COVEN session
drift join --token eyJ...V19

# Or specify peer nodes directly
drift train --peers 4e110...,de4db33f --epochs 10
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
| `--join <token>`          | Join COVEN session via invitation token           |
| `train`                   | Enter training mode as coordinated peer           |
| `--peer <id>`             | Connect to specific peer node by ID               |
| `--model-path <path>`     | Path to model checkpoint file (.pt, .safetensors) |
| `--dataset-path <path>`   | Path to training dataset directory                |
| `--epochs <n>`            | Number of training epochs (default: 10)           |
| `--batch-size <n>`        | Batch size per step (default: 32)                 |
| `--resume`                | Resume from last checkpoint                       |
| `--checkpoint-dir <path>` | Directory for checkpoint files                    |

## Build

From coven root folder:

```
just build-drift
```

Or manually:

```
cd drift
cargo build --release
ln -s <path/to/clone>/coven/drift/target/release/drift $HOME/.local/bin/drift
ln -s <path/to/clone>/coven/drift/target/release/drift-coord $HOME/.local/bin/drift-coord
ln -s <path/to/clone>/coven/drift/target/release/drift-node $HOME/.local/bin/drift-node
```

Restart shell after creating symlinks.

## Manual Installation

For development builds without covn:

```sh
# Clone and build
git clone https://tangled.org/yzzxyz.roomy.chat/coven
cd coven/drift
cargo build --release

# Symlink binaries
ln -s $PWD/target/release/drift $HOME/.local/bin/drift
ln -s $PWD/target/release/drift-node $HOME/.local/bin/drift-node
ln -s $PWD/target/release/drift-coord $HOME/.local/bin/drift-coord
```

Ensure `$HOME/.local/bin` is in your PATH.

## Project Structure

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
│       ├── integration.rs    # Full handshake test suite
│       ├── training.rs      # End-to-end training pipeline
│       └── stress.rs        # Bulk message and gradient tests
│
├── drift-coord/           # Coordinator binary
│   └── src/
│       ├── main.rs           # CLI: coord, train commands
│       ├── scheduler.rs     # Shard assignment by GPU capability
│       ├── checkpoint.rs     # Checkpoint management
│       └── monitor.rs       # Health monitoring, progress display
│
├── drift-node/            # Node binary
│   └── src/
│       ├── main.rs          # CLI: join, status
│       ├── gpu.rs           # GPU detection (nvidia-smi, apple metal)
│       ├── network.rs      # iroh endpoint, connection handling
│       └── training.rs     # Local training loop subprocess
│
├── drift-cli/              # Unified CLI entry point
│   └── src/
│       ├── main.rs          # CLI: join, train, status, coord
│       ├── node.rs           # Node lifecycle logic
│       └── coord.rs           # Coordinator logic
│
└── examples/
    ├── mock_train.py       # Mock training script for testing
    └── train.yaml           # Example training configuration
```

## Testing

```sh
# Run all tests
cargo test --workspace

# Or run specific library tests
cd drift-proto && cargo test
```
