# Bugfix: drift-cli Script Discovery and Execution

## Problem
When `pyproject.toml` contains:
```toml
[project.scripts]
template_ati_plug = "ati.__init__:main"
```
Drift should run `template_ati_plug`. Instead, it fails with:
```
ERROR drift::node: connection error: train_repo_url is set but script_entrypoint is missing. Ensure _ati_plug is defined in pyproject.toml.
```

## Root Cause
1. **Missing Discovery**: `drift-cli/src/node.rs` does not perform script discovery from `pyproject.toml`. It expects `script_entrypoint` to be provided in `TrainConfig`, but the coordinator always sends `None`.
2. **Misleading Error**: The error message references `_ati_plug` specifically, but the discovery logic in `drift-node` supports any key ending in `ati_plug`.
3. **Incorrect Execution**: `run_real_training` uses `model_path` instead of the discovered entrypoint.

---

## Stage 1: Port Script Discovery to drift-cli

| Step | Description | Status | Line Numbers |
|------|-------------|--------|--------------|
| 1.1 | Copy `discover_script_entrypoint`, `find_ati_plug`, `resolve_entrypoint_to_spawn_cmd`, `detect_venv_activation` from `drift-node/src/script_discovery.rs` to `drift-cli/src/node.rs` | 0% | `drift-cli/src/node.rs:~340-440` |
| 1.2 | Add `toml = "0.5"` dependency to `drift-cli/Cargo.toml` | 0% | `drift-cli/Cargo.toml:22` |
| 1.3 | Verify compilation with `cargo check` | 0% | N/A |

**Completion State**: `drift-cli` has local script discovery functions matching `drift-node` behavior.

---

## Stage 2: Integrate Discovery into Node Connection Handler

| Step | Description | Status | Line Numbers |
|------|-------------|--------|--------------|
| 2.1 | After receiving `TrainingReady` (line 286), call `discover_script_entrypoint` with the repo path | 0% | `drift-cli/src/node.rs:286-290` |
| 2.2 | Store discovered entrypoint in `train_config.script_entrypoint` and `train_config.training_spawn_cmd` | 0% | `drift-cli/src/node.rs:245-248` |
| 2.3 | On discovery failure, send error to coordinator or return early | 0% | `drift-cli/src/node.rs:290-295` |

**Completion State**: `handle_connection` discovers and populates `script_entrypoint` before calling `run_training`.

---

## Stage 3: Fix Training Execution to Use Entrypoint

| Step | Description | Status | Line Numbers |
|------|-------------|--------|--------------|
| 3.1 | Modify `run_real_training` to use `config.training_spawn_cmd` if available | 0% | `drift-cli/src/node.rs:364-370` |
| 3.2 | Fall back to `model_path` only if `training_spawn_cmd` is missing | 0% | `drift-cli/src/node.rs:370-375` |
| 3.3 | Ensure venv activation is applied when present | 0% | `drift-cli/src/node.rs:375-380` |

**Completion State**: Real training executes the discovered script entrypoint, not just `model_path`.

---

## Stage 4: Improve Error Messages and Validation

| Step | Description | Status | Line Numbers |
|------|-------------|--------|--------------|
| 4.1 | Update error message at line 340 to reference "ati_plug script" instead of "_ati_plug" | 0% | `drift-cli/src/node.rs:340` |
| 4.2 | Add validation that entrypoint format is `module:function` before execution | 0% | `drift-cli/src/node.rs:360-365` |
| 4.3 | Add warning when `model_path` is used as fallback (legacy mode) | 0% | `drift-cli/src/node.rs:370` |

**Completion State**: Error messages accurately reflect supported patterns; legacy fallback is logged.

---

## Verification

- [ ] Manual test: `drift` with `train_repo_url=Some` and `template_ati_plug` in `pyproject.toml` → runs real training successfully
- [ ] Manual test: `drift` with missing `ati_plug` script → sends appropriate error to coordinator
- [ ] Unit test: `discover_script_entrypoint` finds `template_ati_plug`, `my_ati_plug`, `_ati_plug`
- [ ] Unit test: `resolve_entrypoint_to_spawn_cmd` generates correct `python -c` command
