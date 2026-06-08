# Drift Test System: Step-Through Demo

## Overview

A dedicated test binary that demonstrates the full drift train execution lifecycle with step-by-step output, multiple scenarios, and configurable output formats.

## Architecture

### New `drift-demo` Binary
Located at `drift/tests/drift_demo.rs` - a standalone executable that orchestrates the full train lifecycle.

**Critical**: The demo calls actual `coven/drift/` components:
- `drift_cli::coord::train()` - real train coordination
- `drift_node::join()` - real node join logic
- `drift_auth::Aggregator` - real signature aggregation
- `drift_proto::TrainConfig` - real message types
- `iroh` - real iroh networking

No mocking of core components - only Python training subprocess is stubbed.

### Execution Phases

Each phase is logged with structured output:

```
[PHASE] Component: details
```

1. **[INIT]** Coordinator binds endpoint, nodes join the iroh swarm
2. **[CONNECT]** Peer connections established, NodeInfo exchange
3. **[CONFIG]** TrainConfig + ShardAssignment sent to each node
4. **[AUTH]** Signature aggregation (if enabled)
5. **[TRAIN]** Progress monitoring loop
6. **[CHECKPOINT]** Checkpoint saving logic
7. **[COMPLETE]** TrainComplete, summary output

### Scenarios

- `happy_path` - 3 nodes, all succeed
- `node_failure` - 3 nodes, 1 fails mid-training
- `stale_node` - 3 nodes, 1 stops sending progress
- `auth_mismatch` - nodes disagree on repo_hash
- `auth_timeout` - signatures arrive late
- `threshold_edge` - exactly m-of-n signatures

### Output Modes

- Human-readable: Pretty `PHASE COMPONENT: details` format
- JSON: `{"phase": "...", "component": "...", "details": {...}}`

### Training Modes

- `--training=stub` - fast stub (default for demo)
- `--training=real` - actual Python training

## File Structure

```
drift/tests/
├── drift_demo.rs      # Main demo binary
├── scenarios.rs       # Scenario definitions
├── harness.rs         # Multi-node orchestration
├── python_stub.py     # Minimal training stub
└── step_logging.rs    # Tracing integration
```

## Verification

- `cargo test` runs all scenarios
- Direct binary run shows step-through output
- Compare actual vs expected at each phase boundary