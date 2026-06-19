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

```
cargo test
cargo build
cargo clippy
cargo xwin clippy       // For linux to windows compatibility
cargo update --precise` // make lockfile changes.
```

## Testing

```sh
# Run all tests
cargo test --workspace

# Or run specific library tests
cd drift-proto && cargo test
```

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

## Roadmap

- migrate negate dataset loading to nocturne
- shut down inference for training
- headless, gguf, port cli options
- begin work on tahoe-lafs file store
- swappable pytorch
- package the project
- diffusion splitting
- glaze share
- checkpoint specific saving
- container/vm options (smolvm)

```
Drift Training Entry Points & Adjustment Locations
Based on the darkshapes/drift codebase, here's where training begins and where adjustments should be made:
Entry Point: covn train --peers <ids> --repo <url>
The flow starts in nocturne-cli and delegates to drift:
1. nocturne-cli clones repo to ~/.local/share/covn/<repo-suffix>

3. Node Spawns Python Training (drift-node/src/training.rs)
spawn_training_with_progress() (file:///Users/f784e/Documents/darkshapes/drift/drift-node/src/training.rs:72):
- Reads cached LocalShardState from disk
- Extracts script_entrypoint from TrainConfig (defaults to /tmp/train.py)
- Spawns Python subprocess with args:
python <script> --model-path <path> --dataset-path <path>
    --batch-size <n> --learning-rate <f64> --epochs <n>
    --shard-index <n> --shard-start <n> --shard-end <n>
- Monitors stdout for JSON progress logs
- Writes checkpoints to ~/.drift/shard-<node_id> every 100 steps
Where Adjustments Should Be Made
Location	File
CLI repo handling	drift-cli/src/main.rs:91
Coordinator config	drift-cli/src/coord.rs:64
Repo cloning	drift-node/src/network.rs:65
Script discovery	drift-node/src/training.rs:92
Python env setup	drift-node/src/training.rs:141
Progress parsing	drift-node/src/training.rs:119
Checkpoint save	drift-node/src/training.rs:163
Key Fields in TrainConfig (drift-proto/src/lib.rs)
pub struct TrainConfig {
    pub model_path: String,
    pub dataset_path: String,
    pub batch_size: u32,
    pub learning_rate: f64,
    pub epochs: u32,

    // Distributed repo training
    pub train_repo_url: Option<String>,      // ← Git URL to clone
    pub script_entrypoint: Option<String>,   // ← e.g., "train.py" or "covn:train"
    pub dataset_repo_url: Option<String>,    // ← Optional dataset source
    pub model_artifact_ref: Option<String>,  // ← Local base model path
    pub git_commit: Option<String>,          // ← Verified commit hash
    pub enable_auth: bool,
    pub auth_threshold: usize,
}
Recommended Implementation Order
1. Implement script discovery: search ~/.local/share/covn/<repo> for pyproject.toml
2. Extract entrypoint from [project.scripts] section (e.g., covn = nocturne.__main__:main)
3. Spawn Python with proper venv activation and script args
	Python training script be invoked from [project.scripts] in pyproject.toml of the repo, but ONLY  ONLY TrainingReady is received from the orchestrator
4. Parse progress from stdout (JSON or DRIFT_PROGRESS format)
5. Write checkpoints to local cache for resume support
```

## Components Locations

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

Remove from drift-proto

```
Vestigal
DRIFT_RING_ALPN

    AuthChallenge 	/// Coordinator sends to node: "please authenticate"
    AuthResponse     	/// Node sends signed auth message to coordinator
    AuthAggregate	/// Coordinator broadcasts aggregate back to all nodes
    pub model_path: String,
    pub dataset_path: String,
    pub batch_size: u32,
    pub learning_rate: f64,
    pub epochs: u32,
    pub script_entrypoint: Option<String>
    pub git_commit: Option<String>,
    pub gpu_compute_capability: Option<f64>
    pub auth_threshold: usize,




/// ALPN protocol identifier for drift coordinator<->node traffic.
pub const DRIFT_ALPN: &[u8] = b"drift/0";

/// ALPN protocol identifier for node<->node ring all-reduce traffic.
pub const DRIFT_RING_ALPN: &[u8] = b"drift-ring/0";

/// Maximum allowed message size (64 MB).
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024 * 1024;


    // New fields for distributed repo-based training
    /// URL of the repository containing the training script.
    /// Node should clone this and run the specified entrypoint.
    #[serde(default)]
    pub train_repo_url: Option<String>,

    /// HuggingFace repo ID or Git URL for the dataset.
    /// Node should download/clone this before starting training.
    #[serde(default)]
    pub dataset_repo_url: Option<String>,

    /// URLs for datasets (multiple datasets supported).
    #[serde(default)]
    pub dataset_urls: Vec<String>,

    /// Optional path within dataset_repo for fine-tuning from local base model.
    #[serde(default)]
    pub model_artifact_ref: Option<String>,

   /// Enable multi-signature authentication
    #[serde(default)]
    pub enable_auth: bool,

    /// Threshold for signature aggregation (e.g., 3 for 3-of-n).
    pub auth_threshold: usize




   /// Enable multi-signature authentication
    #[serde(default)]
    pub enable_auth: bool,

    /// Threshold for signature aggregation (e.g., 3 for 3-of-n).
    pub auth_threshold: usize,

    /// Agreed-upon git commit hash (set by coordinator after verification).
    #[serde(default)]
    pub git_commit: Option<String>,

    /// GPU compute capability (e.g., 8.9 for CUDA 8.9).
    #[serde(default)]
```
