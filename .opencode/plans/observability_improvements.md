# Plan: Train Observability Improvements

This plan outlines the additions of progress indicators and shard division visibility to the `train` command in `drift-cli/src/coord.rs`.

## Stage 1: Peer Connectivity Progress

Add progress indicators to the peer connection phase to show how many nodes are being connected.

- [ ] Step-by-step:
  1. Add a stage header before the peer connection loop.
  2. Modify the connection loop to include `[i+1/total]` progress in the "Connecting to..." message.

| Step               | Completion | Line Reference              |
| :----------------- | :--------: | :-------------------------- |
| Stage Header       |     0%     |                             |
| Progress Indicator |     0%     | `drift-cli/src/coord.rs:90` |

**Completion State**: Console shows `==> Stage: Peer Connectivity` and `[1/N] Connecting to...` for each peer.

---

## Stage 2: Shard Division Visibility

Provide a detailed view of how the dataset is partitioned across the participating nodes.

- [ ] Step-by-step:
  1. Add a stage header before printing shard assignments.
  2. Implement a table print loop that iterates through `assignments` and displays `Node ID`, `Shard Index`, `Start Offset`, and `End Offset`.

| Step         | Completion | Line Reference               |
| :----------- | :--------: | :--------------------------- |
| Stage Header |     0%     |                              |
| Shard Table  |     0%     | `drift-cli/src/coord.rs:129` |

**Completion State**: Console displays a formatted table of dataset shards assigned to each node before training starts.

---

## Stage 3: Configuration & Verification Progress

Add progress indicators to the configuration broadcast and repository commit verification phases.

- [ ] Step-by-step:
  1. Add a stage header for "Configuration Broadcast".
  2. Update the config send loop to show progress.
  3. Add a stage header for "Consistency Verification".
  4. Update the commit verification loop to show `[i+1/total]` progress.

| Step            | Completion | Line Reference               |
| :-------------- | :--------: | :--------------------------- |
| Config Header   |     0%     |                              |
| Config Progress |     0%     | `drift-cli/src/coord.rs:133` |
| Verify Header   |     0%     |                              |
| Verify Progress |     0%     | `drift-cli/src/coord.rs:148` |

**Completion State**: Console shows clear stage transitions and `[i/N]` progress for both config delivery and commit verification.
