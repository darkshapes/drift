## Contributing

Accept [Code of Conduct](./CODE_OF_CONDUCT.md).
Ask questions and suggest improvements to the Code of Conduct.
Do not participate if you do not accept the terms of the Code of Conduct.

Fork, then clone your new repo.<br>
Replace the **\< >** placeholders with your information:

```sh
git clone https://tangled.org/<your-name.homeserver.xyz>/coven
```

Set ORIGIN for changes to your fork as:

```sh
git@tangled.org:did:plc:<your-plc-identifier-here>/coven
```

Validate inputs and external state early. Extract common logic, prefer simple solutions. Clear error messages.

Include this header in every rust file

// SPDX-License-Identifier: MPL-2.0 AND LicenseRef-Commons-Clause-License-Condition-1.0
// <!-- // /*  d a r k s h a p e s */ -->

Rust Tools:

````
cargo test
cargo build
cargo clippy
cargo xwin clippy       // For linux to windows compatibility
cargo update --precise` // make lockfile changes.
## Testing

```sh
# Run all tests
cargo test --workspace

# Or run specific library tests
cd drift-proto && cargo test
````

## Design Notes

Model training in a timely way demands a tremendous number of special calculations.<br>
For efficiency, these calculations should run parallel, complementary to the workings of GPUs.<br>
Typically nodes are handed one step of work and have to sync, processing the result amongst all other nodes before continuing to the next.<br>
One training run might require several hundred thousand steps.<br>

### Example A. Homogenous Datcenter Training Flow

Datacenter: 8xH100 GPUs<br>
NVIDIA H100 is a rack-mounted TPU for high-performance serving. The cluster is identical - same chips, manufacturer, & generation. Price: ~25-30k USD (x8).<br>

#### Result

Each node finishes calculation at nearly the same time.<br>
Connectivity becomes only bottleneck (mitigated by high-performance optical cabling and transceivers).<br>
Each step = on the scale of nanoseconds.<br>

### Example B. Heterogeneous Mesh Network Training Flow

```
NVIDIA 4090 CUDA    ^ trains fast
NVIDIA 3070 CUDA    |
M2 Ultra METAL      |
7800M ROCM/HIP      |
Intel i9 CPU        | trains slow
```

```sql
    3070
     |     i9
    ...  /
        \
         |-- M2 Ultra
        /
    '''  \
     |     7800
    4090
```

Each node is consumer-grade hardware of a different design, generation, bus location or manufactrer. Price: couple hundred to couple thousand USD. (x1)

#### Result

Calculations arrive staggered. 4090 node finishes, waits for 3070, which waits for M2 Ultra or 7800 or both. All wait for i9 CPU.<br>
Speed of cluster processing is reduced to the speed of the slowest node : the i9.<br>
Added latency for each step across large geographic distance.<br>
Each step = on the scale between microseconds and minutes.<br>

### Purpose

Distributed training coordination for consumer hardware (GPUs and CPUs) that:

- Avoids gradient sharing and allgather operations entirely
- Trains nodes independently in a ring-free, GLOO-free, NVLink-free architecture
- Supports Apple Silicon Metal, NVIDIA CUDA, AMD ROCm, and Vulkan backends
- Communicates over QUIC-encrypted iroh tunnels with automatic NAT hole-punching

### Solutions

The process and performance issues of the examples can be addressed in a few ways:

1. Sparsity or removal of communication between steps. This is the fundamental approach of drift.

2. Use proportional allocation of task or data so slower devices have less total work to do. Any useful work that could be done quickly on a CPU such as experiment tracking or traditional ML could be allocated to CPU nodes without them being "a burden" on other nodes. This is our approach to data in drift.

3. Remove the assumption that all nodes must finish. It should be possible to use the partially trained model even if some shards are missing, mathematically a well supported thing to do with flow matching - equivalent to just changing the solver you're using. This we are keen to try but haven't explored yet.

### What's Removed

- All shared memory operations
- Gradient synchronization and ring scatter-reduce
- Allgather collectives
- Tensor products shared over network
- NVLink-aware tensor sharding
- Python, including Torch Distributed / DDP functions

### What's Added

- Apple device recognition and Metal GPU detection
- Independent local training with checkpoint coordination
- Periodic barrier sync without gradient exchange

### Build Artifacts

Drift builds to `target/release/`. Binary artifacts should be moved, copied, or symlinked to a static folder:

```

drift/target/release/drift # Main CLI binary
drift/target/release/drift-node # Node binary
drift/target/release/drift-coord # Coordinator binary

```

On MacOS, building `drift` may require permission from `integration`, `stress`, and `training` packages.

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

### Components Locations

```
drift-cli
Hardware detection
Compute capability
Initialize connection
Simulate training
Launch training
Find local repos
Git ls-remote

drift-proto
Message structs and constants
NodeInfo /// CPU, GPU, Architecture, and rank
TrainConfig /// Repo URL, Dataset path, checkpoint path, auth threshold
ShardAssignment /// Division of data by compute per node
TrainProgress /// Node Training Session Status
CheckpointInfo /// Resume Training
Ping /// Check response
Pong /// Check response
Heartbeat /// Connection Keepalive
TrainComplete /// Coordinator signals training is complete.
TrainingReady /// Coordinator signals nodes to begin training.
TrainingCancel /// Coordinator broadcasts: commit verification failed, abort.
RepoCommit /// Node sends commit info for verification.
AskForMoreWork /// Request any incomplete tasks
NoMoreWork /// No incomplete tasks available, shut down
AssignNext /// Next incomplete task, begin

drift-auth
Sign RepoCommit

drift-node
Receive all from drift-coord
Send
NodeInfo
TrainProgress
Pong
Heartbeat
RepoCommit
AskForMoreWork

drift-coord
Receive all from drift-node
Send
TrainConfig
ShardAssignment
Ping
TrainingReady
TrainingCancel
NoMoreWork
AssignNext
```

## Roadmap

Stop needs to account for more than one instance of drift or mesh

Falling out of sync when switching commands (ie mesh/parllax)

Add dash menu option

~60-70% of drift-auth code is test-only or unused in production

- used components: message types, signature aggregation, repo commit verification.
- not used: Replay protection, AuthConfig, CoordinatorAuth

4. Parse progress from stdout (JSON or DRIFT_PROGRESS format)
5. Write checkpoints to local cache for resume support

### Small tasks

- shut down inference for training
- --headless, --gguf, --port
- migrate negate dataset loading to nocturne
- begin work on tahoe-lafs file store

### Larger Tasks

- swappable pytorch
- package the project
- diffusion splitting
- glaze share
- checkpoint specific saving
- container/vm options (smolvm)

## References

https://arxiv.org/abs/2007.14390 Flower: A Friendly Federated Learning Research Framework<br>
https://arxiv.org/abs/2103.03239 Moshpit SGD: Communication-Efficient Decentralized Training on Heterogeneous Unreliable Devices<br>
https://arxiv.org/abs/2103.16257 Model-Contrastive Federated Learning<br>
https://arxiv.org/abs/2106.10207 Distributed Deep Learning in Open Collaborations<br>
https://arxiv.org/abs/2301.11913 SWARM Parallelism: Training Large Models Can Be Surprisingly <br>
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
https://arxiv.org/abs/2601.22442 AsyncMesh: Fully Asynchronous Optimization for Data and Pipeline <br>
https://arxiv.org/abs/2602.02192 ECHO-2: A Large-Scale Distributed Rollout Framework<br>
https://arxiv.org/abs/2602.02685 Expert-Data Alignment Governs Generation Quality in Decentralized<br>
https://arxiv.org/abs/2602.08387 Modalities, a PyTorch-native Framework For Large-scale LLM Training<br>
https://arxiv.org/abs/2603.06741 Heterogeneous Decentralized Diffusion Models<br>
https://arxiv.org/abs/2603.08163 Covenant-72B: Pre-Training a 72B LLM with Trustless Peers<br>
https://arxiv.org/abs/2604.14561 CoCoDiff: Optimizing Collective Communications for Distributed<br>
https://arxiv.org/abs/2604.21428 Decoupled DiLoCo for Resilient Distributed Pre-training<br>
https://arxiv.org/abs/2605.06663 EMO: Pretraining Mixture of Experts for Emergent Modularity<br>
https://arxiv.org/abs/2605.30852 Speculative Pipeline Decoding: Higher-Accuracy and Zero-Bubble <br>

```

```
