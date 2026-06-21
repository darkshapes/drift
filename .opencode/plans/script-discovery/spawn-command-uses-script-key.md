# Plan: Spawn Command Uses Script Key Directly

## Problem

Current spawn command constructs `python -c "from module import func; func()"` from entrypoint value. Should instead use the script **key** directly: `source activate && ati_plug`

/## Implementation

### Stage 1: Modify `find_ati_plug()` to return script key

- **Goal:** Return the script key (e.g., `ati_plug`) instead of the value (e.g., `ati.__init__:main`)
- **Current behavior:** Returns value from `[scripts]` table
- **New behavior:** Return key name for direct execution

| Step | Description                                                     | Status | Location                                   |
| ---- | --------------------------------------------------------------- | ------ | ------------------------------------------ |
| 1.1  | Modify `find_ati_plug()` to return script key instead of value  | 100%   | `drift-node/src/script_discovery.rs:16-67` |
| 1.2  | Update return type to return key for spawn command construction | 100%   | `drift-node/src/script_discovery.rs:16-67` |

**Stage 1 Completion:** [x] `find_ati_plug()` returns script key

### Stage 2: Modify `resolve_entrypoint_to_spawn_cmd()` to use script key

- **Goal:** Construct spawn command as `source <activate> && <script_key>`
- **Current:** `PYTHONPATH=/repo python -c "from module import func; func()"`
- **New:** `source /repo/.venv/bin/activate && ati_plug`

| Step | Description                                                    | Status | Location                                     |
| ---- | -------------------------------------------------------------- | ------ | -------------------------------------------- |
| 2.1  | Change function signature to accept script key                 | 100%   | `drift-node/src/script_discovery.rs:110-129` |
| 2.2  | Remove `python -c` command construction                        | 100%   | `drift-node/src/script_discovery.rs:123`     |
| 2.3  | Construct spawn command as `source <activate> && <script_key>` | 100%   | `drift-node/src/script_discovery.rs:124-128` |

**Stage 2 Completion:** [x] Spawn command uses script key directly

### Stage 3: Update `drift-cli/src/node.rs` (duplicate functions)

- **Goal:** Apply same changes to duplicate functions in CLI module

| Step | Description                                                  | Status | Location                        |
| ---- | ------------------------------------------------------------ | ------ | ------------------------------- |
| 3.1  | Modify `find_ati_plug()` to return script key                | 100%   | `drift-cli/src/node.rs:652-705` |
| 3.2  | Modify `resolve_entrypoint_to_spawn_cmd()` to use script key | 100%   | `drift-cli/src/node.rs:722-742` |

**Stage 3 Completion:** [x] CLI module updated

### Stage 4: Update callers to pass script key

- **Goal:** Ensure all call sites pass script key instead of entrypoint value

| Step | Description                                                    | Status | Location                        |
| ---- | -------------------------------------------------------------- | ------ | ------------------------------- |
| 4.1  | Update `discover_script_entrypoint()` call sites to return key | 100%   | `drift-cli/src/node.rs:640-650` |
| 4.2  | Update `run_training()` to use script key for spawn command    | 100%   | `drift-cli/src/node.rs:376-388` |

**Stage 4 Completion:** [x] All callers updated (tests updated to pass script keys)

### Stage 5: Verify spawn command format

- **Goal:** Confirm spawn command matches expected format

| Step | Description                                                                          | Status | Verification                      |
| ---- | ------------------------------------------------------------------------------------ | ------ | --------------------------------- |
| 5.1  | Test: `ati_plug = "ati.__init__:main"` → `source activate && ati_plug`               | 100%   | `cargo test --package drift-node` |
| 5.2  | Test: `template_ati_plug = "template:main"` → `source activate && template_ati_plug` | 100%   | `cargo test --package drift-node` |

**Stage 5 Completion:** [x] Spawn command format verified

## Completion State

Spawn command is constructed as `source <activate> && <script_key>` using the script key from `[scripts]` table, not a constructed `python -c` command.
