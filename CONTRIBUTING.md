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

Model training in a timely way demands a tremendous number of special calculations.
For efficiency, these calculations should run parallel, complementary to the workings of GPUs.
Typically nodes are handed one step of work and have to sync, processing the result amongst all other nodes before continuing to the next.
One training run might require several hundred thousand steps.

Training Flow:
Datacenter: 8xH100 GPUs
NVIDIA H100 is a rack-mounted TPU for high-performance serving. The cluster is identical - same chips, manufacturer, & generation. Price: ~25-30k USD (x8).

Result:
Each node finishes calculation at nearly the same time.
Connectivity becomes only bottleneck (mitigated by high-performance optical cabling and transceivers).
Each step = on the scale of nanoseconds.

Mesh Network:
NVIDIA 4090 CUDA ^ trains fast
NVIDIA 3070 CUDA |
M2 Ultra METAL |
7800M ROCM/HIP |
Intel i9 CPU | trains slow

    3070
     |     i9
    ...  /
        \
         |-- M2 Ultra
        /
    ```  \
     |     7800
    4090

    Each node is consumer-grade hardware of a different design, generation, bus location or manufactrer.  Price: couple hundred to couple thousand USD. (x1)

Result:
Calculations arrive staggered. 4090 node finishes, waits for 3070, which waits for M2 Ultra or 7800 or both. All wait for i9 CPU.
Speed of cluster processing is reduced to the speed of the slowest node : the i9.
Added latency for each step across large geographic distance.
Each step = on the scale between microseconds and minutes.

https://arxiv.org/abs/2007.14390 Flower: A Friendly Federated Learning Research Framework
https://arxiv.org/abs/2103.03239 Moshpit SGD: Communication-Efficient Decentralized Training on Heterogeneous Unreliable Devices
https://arxiv.org/abs/2103.16257 Model-Contrastive Federated Learning
https://arxiv.org/abs/2106.10207 Distributed Deep Learning in Open Collaborations
https://arxiv.org/abs/2311.08105 DiLoCo: Distributed Low-Communication Training of Language Models
https://arxiv.org/abs/2402.01862 Parametric Feature Transfer: One-shot Federated Learning
https://arxiv.org/abs/2402.19481 DistriFusion: Distributed Parallel Inference for High-Resolution
https://arxiv.org/abs/2406.01566 Helix: Serving Large Language Models over Heterogeneous GPUs
https://arxiv.org/abs/2407.07852 OpenDiLoCo: An Open-Source Framework for Globally Distributed Low-Communication Training
https://arxiv.org/abs/2501.05450 Decentralized Diffusion Models
https://arxiv.org/abs/2504.00952 Personalized Federated Training of Diffusion Models with Privacy
https://arxiv.org/abs/2504.17096 Sailor: Automating Distributed Training over Dynamic, Heterogeneous
https://arxiv.org/abs/2505.15306 Multiple Weaks Win Single Strong: Large Language Models Ensemble
https://arxiv.org/abs/2506.14202 DiffusionBlocks: Block-wise Neural Network Training via Diffusion
https://arxiv.org/abs/2507.00507 Towards Resource-Efficient Serverless LLM Inference with SLINFER
https://arxiv.org/abs/2509.26182 Parallax: Efficient LLM Inference Service over Decentralized Environment
https://arxiv.org/abs/2601.03184 Decentralized Autoregressive Generation
https://arxiv.org/abs/2601.06857 MoE-DisCo:Low Economy Cost Training Mixture-of-Experts Models
https://arxiv.org/abs/2601.16863 Mixture-of-Models: Unifying Heterogeneous Agents via N-Way Self-Eval
https://arxiv.org/abs/2602.02192 ECHO-2: A Large-Scale Distributed Rollout Framework
https://arxiv.org/abs/2602.02685 Expert-Data Alignment Governs Generation Quality in Decentralized
https://arxiv.org/abs/2602.08387 Modalities, a PyTorch-native Framework For Large-scale LLM Training
https://arxiv.org/abs/2603.06741 Heterogeneous Decentralized Diffusion Models
https://arxiv.org/abs/2603.08163 Covenant-72B: Pre-Training a 72B LLM with Trustless Peers
https://arxiv.org/abs/2604.14561 CoCoDiff: Optimizing Collective Communications for Distributed
https://arxiv.org/abs/2604.21428 Decoupled DiLoCo for Resilient Distributed Pre-training
https://arxiv.org/abs/2605.06663 EMO: Pretraining Mixture of Experts for Emergent Modularity

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
drift/target/release/drift           # Main CLI binary
drift/target/release/drift-node      # Node binary
drift/target/release/drift-coord     # Coordinator binary
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
