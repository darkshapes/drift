# Env Vars Inline Transmission Plan

**Objective**: Drift-coord parses `.env` file and sends env var pairs inline to all drift-node peers via new `EnvVars` message type, rather than nodes reading their own `.env` files.

**Security Model**:

- `env_vars` field marked with `#[serde(skip_serializing)]` to prevent disk persistence
- Filter sensitive keys (`*_KEY*`, `*_SECRET*`, `*_TOKEN*`, `*_PASSWORD*`, `*_PASS*`, `*_AUTH*`)
- Env vars transmitted in-memory only, never written to checkpoints or logs

---

## Stage 1: Protocol Definition (drift-proto)

**Goal**: Add `env_vars` field to `TrainConfig` and `DriftMessage::EnvVars` message type

| Step | Task                                                                                               | File                     | Lines   | Status |
| ---- | -------------------------------------------------------------------------------------------------- | ------------------------ | ------- | ------ |
| 1.1  | Add `env_vars: Option<HashMap<String, String>>` to `TrainConfig` with `#[serde(skip_serializing)]` | `drift-proto/src/lib.rs` | 320-327 | ☑      |
| 1.2  | Add `DriftMessage::EnvVars(HashMap<String, String>)` variant                                       | `drift-proto/src/lib.rs` | 440 | ☑      |
| 1.3  | Add `update_env_vars` method to `TrainConfig`                                                      | `drift-proto/src/lib.rs` | 327-329 | ☑      |

**Verification**:

- [x] `cargo check` passes in `drift-proto/`
- [x] `TrainConfig` has `env_vars` field
- [x] `DriftMessage` enum has `EnvVars` variant

---

## Stage 2: Env Var Parsing Utility (drift-coord)

**Goal**: Implement `.env` file parsing and sensitive key filtering in coordinator

| Step | Task                                                                 | File                      | Lines   | Status |
| ---- | -------------------------------------------------------------------- | ------------------------- | ------- | ------ |
| 2.1  | Add `parse_env_file(path: &str) -> HashMap<String, String>` function | `drift-coord/src/env.rs` | 7-26   | ☑      |
| 2.2  | Add `filter_sensitive_keys(env_vars: HashMap) -> HashMap` function   | `drift-coord/src/env.rs` | 28-63  | ☑      |
| 2.3  | Update `train()` to call parser on `--env-file` arg                  | `drift-coord/src/main.rs` | 173-180 | ☑      |
| 2.4  | Attach `env_vars` to `TrainConfig` before sending to nodes           | `drift-coord/src/main.rs` | 180-181 | ☑      |

**Verification**:

- [x] `cargo check` passes in `drift-coord/`
- [x] `parse_env_file` handles comments, empty lines, `KEY=VALUE` format
- [x] `filter_sensitive_keys` removes keys matching patterns

---

## Stage 3: CLI Coord Update (drift-cli)

**Goal**: Mirror env var parsing in `drift-cli/src/coord.rs` for CLI-based training

| Step | Task                                                               | File                     | Lines   | Status |
| ---- | ------------------------------------------------------------------ | ------------------------ | ------- | ------ |
| 3.1  | Add `parse_env_file` and `filter_sensitive_keys` to `coord.rs`     | `drift-cli/src/coord.rs` | 655-700 | ☑      |
| 3.2  | Update `train()` to parse `--env-file` and attach to `TrainConfig` | `drift-cli/src/coord.rs` | 34-38   | ☑      |
| 3.3  | Send `env_vars` inline with `TrainConfig` message                  | `drift-cli/src/coord.rs` | 38      | ☑      |

**Verification**:

- [x] `cargo check` passes in `drift-cli/`
- [x] CLI `--env-file` arg populates `env_vars` in config

---

## Stage 4: Node-Side Env Var Consumption (drift-cli/node.rs)

**Goal**: Remove local `.env` parsing from node, use received `env_vars`

| Step | Task                                                    | File                    | Lines   | Status |
| ---- | ------------------------------------------------------- | ----------------------- | ------- | ------ |
| 4.1  | Remove `parse_env_file` from `node.rs` (moved to coord) | `drift-cli/src/node.rs` | N/A   | ☑      |
| 4.2  | Update `run_real_training()` to use `config.env_vars`   | `drift-cli/src/node.rs` | 350  | ☑      |
| 4.3  | Prepend env vars to spawn command as `KEY=VALUE` prefix | `drift-cli/src/node.rs` | 350  | ☑      |

**Verification**:

- [x] Node no longer reads `.env` from cwd
- [x] Spawn command includes env vars from config

---

## Stage 5: Drift-Node Training Update

**Goal**: Update `drift-node/src/training.rs` to accept env vars from config

| Step | Task                                                                                      | File                         | Lines  | Status |
| ---- | ----------------------------------------------------------------------------------------- | ---------------------------- | ------ | ------ |
| 5.1  | Add `env_vars: Option<HashMap<String, String>>` param to `spawn_training_with_progress()` | `drift-node/src/training.rs` | 88   | ☑      |
| 5.2  | Inject env vars into subprocess command                                                   | `drift-node/src/training.rs` | 102-115 | ☑      |

**Verification**:

- [x] `cargo check` passes in `drift-node/`
- [x] Training subprocess receives env vars

---

## Stage 6: Integration Testing

**Goal**: Verify end-to-end env var transmission and usage

| Step | Task                                                    | File                                           | Lines  | Status |
| ---- | ------------------------------------------------------- | ---------------------------------------------- | ------ | ------ |
| 6.1  | Add test: coordinator parses `.env`, sends to mock node | `drift-coord/tests/env_vars_transmission.rs`   | 1-69   | ☑      |
| 6.2  | Add test: node receives env_vars, uses in spawn_cmd     | `drift-node/tests/env_vars_injection.rs`       | 1-92   | ☑      |
| 6.3  | Add test: sensitive keys are filtered                   | `drift-coord/tests/sensitive_key_filtering.rs` | 1-94   | ☑      |
| 6.4  | Add test: env_vars not serialized in checkpoints        | `drift-proto/tests/env_vars_no_persistence.rs` | 1-54   | ☑      |

**Verification**:

- [x] All new tests pass
- [x] `cargo test` passes in all packages

---

## Stage 7: Security Hardening

**Goal**: Ensure env vars are not logged or persisted

| Step | Task                                                  | File                                           | Lines   | Status |
| ---- | ---------------------------------------------------- | ---------------------------------------------- | ------- | ------ |
| 7.1  | Add `#[serde(skip_serializing)]` to `env_vars` field  | `drift-proto/src/lib.rs`                       | 258-317 | ☑      |
| 7.2  | Add log redaction for env_vars in tracing calls       | `drift-coord/src/main.rs`                      | 107-165 | ☑      |
| 7.3  | Add assertion: env_vars not in serialized TrainConfig | `drift-proto/tests/env_vars_no_persistence.rs` | 1-50    | ☑      |

**Verification**:

- [x] Checkpoint JSON does not contain `env_vars` key
- [x] Logs show `<redacted>` for env var values

---

## Summary

| Stage                      | Completion | Verified |
| -------------------------- | ---------- | -------- |
| 1. Protocol Definition     | ☑          | ☑        |
| 2. Env Var Parsing (coord) | ☑          | ☑        |
| 3. CLI Coord Update        | ☑          | ☑        |
| 4. Node-Side Consumption   | ☑          | ☑        |
| 5. Drift-Node Training     | ☑          | ☑        |
| 6. Integration Testing     | ☑          | ☑        |
| 7. Security Hardening      | ☑          | ☑        |

**Overall Progress**: 100%

All 7 stages are implemented and verified. The `env_vars` field with `skip_serializing` attribute exists in `TrainConfig` (drift-proto/src/lib.rs:320-327), the `EnvVars` message variant exists in `DriftMessage` (drift-proto/src/lib.rs:440), and both `parse_env_file` and `filter_sensitive_keys` functions are properly implemented in drift-coord/src/env.rs and mirrored in drift-cli/src/coord.rs. Node-side parsing has been removed and replaced with inline env var transmission. Integration tests and security hardening assertions are in place.

---

## Risk Mitigation

| Risk                        | Mitigation                        | Status |
| --------------------------- | --------------------------------- | ------ |
| Env vars leaked in logs     | Redact in tracing calls           | ☑      |
| Env vars persisted to disk  | `skip_serializing` attribute      | ☑      |
| Sensitive keys transmitted  | Filter by pattern match           | ☑      |
| MITM attack on env vars     | Rely on iroh transport encryption | ☐      |
| Checkpoint contains secrets | Assert no `env_vars` in JSON      | ☑      |

---

## Rollback Plan

If issues arise:

1. Revert `TrainConfig` to use `env_file: Option<String>` (path-only)
2. Restore node-side `.env` file parsing
3. Remove `env_vars` field and `EnvVars` message type

**Rollback Command**: `git checkout <prior-commit>`
