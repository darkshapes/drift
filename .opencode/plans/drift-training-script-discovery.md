# Drift Training: Script Discovery & Execution Plan

## Context

- **Entry**: `drift train --peers <ids> --repo <url>`
- **Flow**: Coordinator clones repo → spawns drift-node processes → coordinates training
- **Script discovery**: After `TrainingReady` received, discover `ati_plug` from `pyproject.toml`

---

## Decisions

| Item                | Value                                                                                                      |
| ------------------- | ---------------------------------------------------------------------------------------------------------- |
| Repo clone location | `~/.local/share/drift/<repo-suffix>` or `~/.local/share/covn/<repo-suffix>`                                |
| Dataset locations   | URL downloads and and existing folders of `~/.local/share/drift/datasets/` `~/.local/share/covn/datasets/` |
| Script entrypoint   | `[project.scripts]` → `ati_plug = ...`                                                                     |
| Error handling      | Send `TrainingCancel` if discovery fails                                                                   |
| Parse targets       | `[project.scripts]` AND `[tool.uv.scripts]`                                                                |

---

## Stage 1: Add `dataset_urls` to `TrainConfig` and CLI

**Goal**: Support `--dataset` multi-arg, pass URLs + `gpu_cc` to nodes

### Steps

| #   | Task                                                   | File                         | Lines   | Done |
| --- | ------------------------------------------------------ | ---------------------------- | ------- | ---- |
| 1   | Add `dataset_urls: Vec<String>` field to `TrainConfig` | `drift-proto/src/lib.rs`     | 259-298 | ☐    |
| 2   | Add `--dataset` repeated arg to `Train` command        | `drift-cli/src/main.rs`      | 46-64   | ☐    |
| 3   | Pass `dataset_urls` to `coord::train()`                | `drift-cli/src/main.rs`      | 105-119 | ☐    |
| 4   | Populate `dataset_urls` in `TrainConfig` construction  | `drift-cli/src/coord.rs`     | 64-75   | ☐    |
| 5   | Add `gpu_compute_capability` to spawned Python args    | `drift-node/src/training.rs` | 92-100  | ☐    |

**Verifiable**: `cargo check drift-proto && cargo check drift-cli && cargo check drift-node` pass

---

## Stage 2: Implement Script Discovery on Node

**Goal**: After `TrainingReady`, clone repo and parse `pyproject.toml` for `ati_plug`

### Steps

| #   | Task                                                             | File                                 | Lines   | Done |
| --- | ---------------------------------------------------------------- | ------------------------------------ | ------- | ---- |
| 1   | Add repo clone to `~/.local/share/drift/<suffix>`                | `drift-node/src/network.rs`          | 106-118 | ☐    |
| 2   | Add `discover_script_entrypoint()` helper (parse TOML)           | `drift-node/src/script_discovery.rs` | new     | ☐    |
| 3   | Parse `[project.scripts]` and `[tool.uv.scripts]` for `ati_plog` | `drift-node/src/script_discovery.rs` | new     | ☐    |
| 4   | On failure, send `TrainingCancel` to coordinator                 | `drift-node/src/network.rs`          | 106-118 | ☐    |
| 5   | On success, update `TrainConfig.script_entrypoint`               | `drift-node/src/network.rs`          | 106-118 | ☐    |
| 6   | Save updated `LocalShardState` to disk                           | `drift-node/src/network.rs`          | 106-118 | ☐    |

**Verifiable**: Node connects, receives `TrainingReady`, repo cloned, entrypoint discovered

---

## Stage 3: Spawn Python with Discovered Entrypoint

**Goal**: Use discovered `src.main:ati` entrypoint with proper venv activation

### Steps

| #   | Task                                                    | File                         | Lines  | Done |
| --- | ------------------------------------------------------- | ---------------------------- | ------ | ---- |
| 1   | Resolve `src.main:ati` to full path                     | `drift-node/src/training.rs` | 92-100 | ☐    |
| 2   | Add `--dataset-url` and `--gpu-cc` args to Python spawn | `drift-node/src/training.rs` | 92-100 | ☐    |
| 3   | Update `spawn_training_with_progress()` call sites      | `drift-node/src/main.rs`     | 95-118 | ☐    |
| 4   | Update resume flow with discovered entrypoint           | `drift-node/src/main.rs`     | 95-118 | ☐    |

**Verifiable**: `python src.main:ati --model-path X --dataset-url Y --gpu-cc 8.9 ...` spawns

---

## Stage 4: Tests

**Goal**: Coverage for new code paths

### Steps

| #   | Task                                              | File                 | Lines | Done |
| --- | ------------------------------------------------- | -------------------- | ----- | ---- |
| 1   | `drift-proto/tests/dataset_urls_serialization.rs` | `drift-proto/tests/` | new   | ☐    |
| 2   | `drift-node/tests/script_discovery.rs`            | `drift-node/tests/`  | new   | ☐    |
| 3   | `drift-node/tests/script_discovery_failure.rs`    | `drift-node/tests/`  | new   | ☐    |
| 4   | `drift-cli/tests/dataset_multi_arg.rs`            | `drift-cli/tests/`   | new   | ☐    |

**Verifiable**: `cargo test` passes with new tests

---

## Stage 5: Integration Test

**Goal**: Full flow from CLI to Python spawn

### Steps

| #   | Task                                           | File                               | Lines              | Done |
| --- | ---------------------------------------------- | ---------------------------------- | ------------------ | ---- |
| 1   | `drift-proto/tests/integration.rs` update      | `drift-proto/tests/integration.rs` | 138, 158, 271, 290 | ☐    |
| 2   | Add dataset_urls to existing test configs      | `drift-proto/tests/integration.rs` | various            | ☐    |
| 3   | Add script_entrypoint to existing test configs | `drift-proto/tests/integration.rs` | various            | ☐    |

**Verifiable**: `cargo test --test integration` passes

---

## Notes

- Use `toml` crate for `pyproject.toml` parsing (add to `Cargo.toml` deps)
- Repo suffix = last path component of URL (e.g., `github.com/user/repo` → `repo`)
- Keep `dataset_path: String` for local fallback compatibility
- `gpu_compute_capability` already exists in `NodeInfo`, pass via `TrainConfig`
