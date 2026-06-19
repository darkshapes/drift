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
             |           			      Heartbeat      |
             | 		                                 |
              `-----------------------------------'
```

### Communication Protocol

```
COORDINATOR TASK                          NODE TASK
────────────────────────────────────────────────────────
cli launch ──────────────────────→
send Ping
                       ←────────────────── receive Ping
                       send NodeInfo
receive NodeInfo ──────────────────────→
send TrainConfig
                       ←────────────────── receive TrainConfig
                       send RepoCommit                            ┓ verification
receive RepoCommit────────────────────→                           ┛ handshake
send TrainingReady, ShardAssignment
                       ←────────────────── receive TrainingReady,
                                                  ShardAssignment
                       send TrainProgress
(or)
send TrainingCancel
───────────────────────────────────────────┐
                                           │
                                           │
                                           ↓
                                          end
```

#### Protocol Flow (ALPN: `drift/0`)

1. Coordinator initiates QUIC connection to node, sends `Ping`
2. Node gets Ping, sends `NodeInfo` containing GPU name, VRAM, compute capability
3. Coordinator broadcasts training repo path in `TrainConfig`,
4. Node responds with `RepoCommit` containing the commit hash of the current training code branch signed by the node Iroh key.
5. Coordinator receives all node commit hashes. Training ends with `TrainingCancel` broadcast unless all commits match, signaled by `TrainingReady` broadcast.
6. Node responds to `TrainingReady` with or shuts down if `TrainingCancel`
7. Coordinator broadcasts `ShardAssignment` to all nodes.
8. Nodes download training scripts, data, and execute at their own pace.
9. Nodes stream `TrainProgress` updates back to coordinator
10. If a node fails, work is held by coordinator as `AssignNext`, awaiting next free node to `AskForMoreWork`
11. On receiving NoMoreWork or timeout, nodes shut down.

All traffic is encrypted end-to-end via QUIC.<br>
NAT hole-punching is handled automatically by iroh, with relay fallback.<br>
Messages are length-prefixed JSON over QUIC bidirectional streams.<br>

#### Verification Handshake

During verification on the coordinator:

1. Per-node signature verification: Each node's node_id, commit, and repo_url are compared together against that node's signature
2. Cross-node consistency check: All nodes' commit hashes are compared against each other to ensure they match

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

### References

https://arxiv.org/abs/2007.14390 Flower: A Friendly Federated Learning Research Framework<br>
https://arxiv.org/abs/2103.03239 Moshpit SGD: Communication-Efficient Decentralized Training on Heterogeneous Unreliable Devices<br>
https://arxiv.org/abs/2103.16257 Model-Contrastive Federated Learning<br>
https://arxiv.org/abs/2106.10207 Distributed Deep Learning in Open Collaborations<br>
https://arxiv.org/abs/2311.08105 DiLoCo: Distributed Low-Communication Training of Language Models<br>
https://arxiv.org/abs/2402.01862 Parametric Feature Transfer: One-shot Federated Learning<br>
https://arxiv.org/abs/2402.19481 DistriFusion: Distributed Parallel Inference for High-Resolution<br>
https://arxiv.org/abs/2406.01566 Helix: Serving Large Language Models over Heterogeneous GPUs<br>
https://arxiv.org/abs/2407.07852 OpenDiLoCo: An Open-Source Framework for Globally Distributed Low-Communication Training<br>
https://arxiv.org/abs/2501.05450 Decentralized Diffusion Models<br>
https://arxiv.org/abs/2504.00952 Personalized Federated Training of Diffusion Models with Privacy<br>
https://arxiv.org/abs/2504.17096 Sailor: Automating Distributed Training over Dynamic, Heterogeneous<br>
https://arxiv.org/abs/2505.15306 Multiple Weaks Win Single Strong: Large Language Models Ensemble<br>
https://arxiv.org/abs/2506.14202 DiffusionBlocks: Block-wise Neural Network Training via Diffusion<br>
https://arxiv.org/abs/2507.00507 Towards Resource-Efficient Serverless LLM Inference with SLINFER<br>
https://arxiv.org/abs/2509.26182 Parallax: Efficient LLM Inference Service over Decentralized Environment<br>
https://arxiv.org/abs/2601.03184 Decentralized Autoregressive Generation<br>
https://arxiv.org/abs/2601.06857 MoE-DisCo:Low Economy Cost Training Mixture-of-Experts Models<br>
https://arxiv.org/abs/2601.16863 Mixture-of-Models: Unifying Heterogeneous Agents via N-Way Self-Eval<br>
https://arxiv.org/abs/2602.02192 ECHO-2: A Large-Scale Distributed Rollout Framework<br>
https://arxiv.org/abs/2602.02685 Expert-Data Alignment Governs Generation Quality in Decentralized<br>
https://arxiv.org/abs/2602.08387 Modalities, a PyTorch-native Framework For Large-scale LLM Training<br>
https://arxiv.org/abs/2603.06741 Heterogeneous Decentralized Diffusion Models<br>
https://arxiv.org/abs/2603.08163 Covenant-72B: Pre-Training a 72B LLM with Trustless Peers<br>
https://arxiv.org/abs/2604.14561 CoCoDiff: Optimizing Collective Communications for Distributed<br>
https://arxiv.org/abs/2604.21428 Decoupled DiLoCo for Resilient Distributed Pre-training<br>
https://arxiv.org/abs/2605.06663 EMO: Pretraining Mixture of Experts for Emergent Modularity<br>
