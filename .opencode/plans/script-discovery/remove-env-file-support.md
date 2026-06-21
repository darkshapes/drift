# Plan: Remove `--env_file` and .env Support

**Objective**: Remove all environment variable file loading functionality from TrainConfig, drift-node, drift-coord, and drift-cli. This includes removing the `env_file` field, `env_vars` field, `EnvVars` message variant, and all related parsing logic.

---

## Stage 1: Remove from `drift-proto` (Core Protocol)

**Goal**: Remove `env_file`, `env_vars`, `update_env_vars()`, and `EnvVars` message from the protocol definition.

| Step | Action | File | Lines | Status |
|------|--------|------|-------|--------|
| 1.1  | Remove `pub env_file: Option<String>` field from `TrainConfig` | `drift-proto/src/lib.rs` | 317 | ☐ |
| 1.2  | Remove `pub env_vars: Option<HashMap<String, String>>` field from `TrainConfig` | `drift-proto/src/lib.rs` | 319-323 | ☐ |
| 1.3  | Remove `update_env_vars()` method from `impl TrainConfig` | `drift-proto/src/lib.rs` | 327-329 | ☐ |
| 1.4  | Remove `EnvVars(HashMap<String, String>)` variant from `DriftMessage` enum | `drift-proto/src/lib.rs` | 440 | ☐ |

**Stage Completion**: ☐  
**Verification**: `grep "env_file\|env_vars\|EnvVars" drift-proto/src/lib.rs` returns no matches

---

## Stage 2: Remove from `drift-coord` (Coordinator)

**Goal**: Remove env file parsing logic, CLI argument, and the `env.rs` module.

| Step | Action | File | Lines | Status |
|------|--------|------|-------|--------|
| 2.1  | Remove import `use drift_coord::env::{parse_env_file, filter_sensitive_keys}` | `drift-coord/src/main.rs` | 2 | ☐ |
| 2.2  | Remove `env_file: Option<String>` CLI arg from `Commands::Train` | `drift-coord/src/main.rs` | 66-68 | ☐ |
| 2.3  | Remove `env_file: Option<String>` parameter from `train()` function | `drift-coord/src/main.rs` | 124 | ☐ |
| 2.4  | Remove env file parsing block (if let Some(ref path) = env_file { ... }) | `drift-coord/src/main.rs` | 172-186 | ☐ |
| 2.5  | Delete `drift-coord/src/env.rs` module file | `drift-coord/src/env.rs` | all | ☐ |
| 2.6  | Delete `drift-coord/tests/parse_env_file.rs` test file | `drift-coord/tests/parse_env_file.rs` | all | ☐ |
| 2.7  | Delete `drift-coord/tests/env_vars_transmission.rs` test file | `drift-coord/tests/env_vars_transmission.rs` | all | ☐ |

**Stage Completion**: ☐  
**Verification**: 
- `ls drift-coord/src/env.rs` returns "file not found"
- `grep "parse_env_file\|filter_sensitive_keys" drift-coord/` returns no matches

---

## Stage 3: Remove from `drift-cli` (Command-Line Interface)

**Goal**: Remove env file CLI argument and parsing logic from coordinator module.

| Step | Action | File | Lines | Status |
|------|--------|------|-------|--------|
| 3.1  | Remove `env_file: Option<String>` from `Commands::Train` | `drift-cli/src/cli.rs` | 50 | ☐ |
| 3.2  | Remove `env_file` from command pattern match in `main.rs` | `drift-cli/src/main.rs` | 37, 52 | ☐ |
| 3.3  | Remove `env_file: Option<String>` parameter from `train()` function | `drift-cli/src/coord.rs` | 29 | ☐ |
| 3.4  | Remove env file parsing logic (lines 36-58) | `drift-cli/src/coord.rs` | 36-58 | ☐ |
| 3.5  | Remove `parse_env_file()` function | `drift-cli/src/coord.rs` | 666-686 | ☐ |
| 3.6  | Remove `filter_sensitive_keys()` function | `drift-cli/src/coord.rs` | 688-695 | ☐ |
| 3.7  | Remove `env_file: env_file` from `TrainConfig` construction | `drift-cli/src/coord.rs` | 110 | ☐ |

**Stage Completion**: ☐  
**Verification**: `grep "parse_env_file\|filter_sensitive_keys\|env_file" drift-cli/src/` returns no matches

---

## Stage 4: Remove from `drift-node` (Node Runtime)

**Goal**: Remove `env_vars` parameter from training spawn calls.

| Step | Action | File | Lines | Status |
|------|--------|------|-------|--------|
| 4.1  | Remove `config.env_vars.clone()` parameter from `spawn_training_with_progress()` call | `drift-node/src/main.rs` | 117 | ☐ |
| 4.2  | Remove `env_prefix` logic and `format_env_prefix_hashmap()` usage | `drift-cli/src/node.rs` | 405-418 | ☐ |
| 4.3  | Update `spawn_training_with_progress()` signature to remove `env_vars` parameter | `drift-node/src/training.rs` | TBD | ☐ |

**Stage Completion**: ☐  
**Verification**: `grep "env_vars\|env_prefix" drift-node/src/` returns no matches

---

## Stage 5: Update Test Files

**Goal**: Remove `env_file: None` from all test file TrainConfig constructions and delete obsolete tests.

| Step | Action | File | Lines | Status |
|------|--------|------|-------|--------|
| 5.1  | Remove `env_file: None,` from all `TrainConfig` constructions | `drift-proto/tests/integration.rs` | 147, 173, 292, 317 | ☑ |
| 5.2  | Remove `env_file: None,` from all `TrainConfig` constructions | `drift-proto/tests/dataset_urls_serialization.rs` | 21, 46, 76, 107, 140, 168, 198 | ☑ |
| 5.3  | Remove `env_file: None,` from `TrainConfig` construction | `drift-proto/tests/node_startup.rs` | 43 | ☑ |
| 5.4  | Remove `env_file: None,` from `TrainConfig` construction | `drift-proto/tests/training.rs` | 128 | ☑ |
| 5.5  | Remove `env_file: None,` from `TrainConfig` constructions | `drift-proto/tests/repo_commit_verification.rs` | 76, 106 | ☑ |
| 5.6  | Delete entire test file (no longer needed) | `drift-proto/tests/env_vars_no_persistence.rs` | all | ☑ |
| 5.7  | Remove `env_file: None,` from `TrainConfig` construction | `drift-coord/tests/coordinator_tests.rs` | 38 | ☑ |

**Stage Completion**: ☑  
**Verification**: `grep "env_file:" drift-proto/tests/ drift-coord/tests/` returns no matches

---

## Stage 6: Update Documentation

**Goal**: Remove all references to `--env-file` flag and `.env` file usage from documentation.

| Step | Action | File | Lines | Status |
|------|--------|------|-------|--------|
| 6.1  | Remove ".env file" section and examples | `CONTRIBUTING.md` | 39-65 | ☐ |
| 6.2  | Remove `--env-file` flag documentation and examples | `README.md` | 95-110 | ☐ |

**Stage Completion**: ☐  
**Verification**: `grep -i "env-file\|\.env" README.md CONTRIBUTING.md` returns no matches

---

## Stage 7: Verification and Testing

**Goal**: Ensure all changes compile and tests pass.

| Step | Action | Command | Expected Result | Status |
|------|--------|---------|----------------|--------|
| 7.1  | Check for remaining references | `rg "env_file|env_vars|EnvVars" --include="*.rs"` | No matches in source files | ☑ |
| 7.2  | Compile workspace | `cargo check` | No errors | ☑ |
| 7.3  | Run tests | `cargo test` | All tests pass | ☑ |
| 7.4  | Verify module removal | `ls drift-coord/src/env.rs` | File not found | ☑ |

**Stage Completion**: ☑  
**Verification**: All `cargo test` commands pass with exit code 0

---

## Summary

| Stage | Description | Completion |
|-------|-------------|-----------|
| 1 | Remove from `drift-proto` | ☐ |
| 2 | Remove from `drift-coord` | ☐ |
| 3 | Remove from `drift-cli` | ☐ |
| 4 | Remove from `drift-node` | ☐ |
| 5 | Update test files | ☑ |
| 6 | Update documentation | ☐ |
| 7 | Verification and testing | ☑ |

**Total Estimated Changes**: ~73 line modifications/deletions across 15+ files  
**Risk Level**: Low - purely subtractive changes, no new logic  
**Dependencies**: None - all changes are internal to the codebase
