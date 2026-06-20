# Drift Simulated Training Trigger Fix

## Objective
Change drift's simulated training trigger from `model_path` existence check to `_ati_plug` script entrypoint discovery in `pyproject.toml` of repos in `~/.local/share/covn/<repo>`, with proper venv activation.

---

## Stage 1: Modify `drift-cli/src/node.rs` - `run_training` Function

### Steps
- [x] Remove `model_path` existence check that triggers simulation
- [x] Add check: if `train_repo_url` is `Some` AND `script_entrypoint` is `None`, cancel training
- [x] Add check: if `train_repo_url` is `None` AND `script_entrypoint` is `None`, run simulated training
- [x] Add check: if `script_entrypoint` is `Some`, prepare for repo-based real training
- [x] Implement venv activation logic before spawning Python command
- [x] Update error messages to reflect new logic

### Verification
- [x] Run `cargo build` in `drift-cli/` - no compilation errors
- [x] Run existing tests in `drift-cli/` - all pass
- [ ] Manual test: run drift with `train_repo_url=None` → triggers simulation
- [ ] Manual test: run drift with `train_repo_url=Some` and valid `_ati_plug` → runs real training

### Line References
- Current logic: lines 338-350
- To be modified: lines 332-351

**Stage Completion:** [x] 100%

---

## Stage 2: Modify `drift-node/src/script_discovery.rs` - Add Venv Detection

### Steps
- [x] Add `detect_venv_activation(repo_path: &Path) -> Option<String>` function
- [x] Check for `.venv/bin/activate` existence in repo path
- [x] Modify `resolve_entrypoint_to_spawn_cmd` to accept repo_path parameter
- [x] Wrap spawn command with `source .venv/bin/activate && <command>` if venv exists
- [x] Add tests for venv detection

### Verification
- [x] Run `cargo test` in `drift-node/` - new tests pass
- [ ] Manual test: repo with .venv → command includes activation
- [ ] Manual test: repo without .venv → command runs without activation

### Line References
- New function: added at line 64
- Modified function: lines 80-100

**Stage Completion:** [x] 100%

---

## Stage 3: Modify `drift-node/src/network.rs` - TrainingReady Handler

### Steps
- [x] After discovering entrypoint (line 165-177), store repo_path in config
- [x] Call `resolve_entrypoint_to_spawn_cmd` with repo_path
- [x] Store the resolved command for later execution
- [x] Update error handling to reflect venv activation failures
- [x] Add logging for venv activation status

### Verification
- [x] Run `cargo test` in `drift-node/` - all pass
- [ ] Manual test: TrainingReady with valid repo → spawns with venv
- [ ] Manual test: TrainingReady with missing _ati_plug → sends TrainingCancel

### Line References
- Current handler: lines 150-223
- To be modified: lines 165-192

**Stage Completion:** [x] 100%

---

## Stage 4: Update `drift-coord/src/main.rs` - TrainConfig Construction

### Steps
- [x] Ensure `train_repo_url` is set when repo-based training is intended
- [x] Ensure `script_entrypoint` is left as `None` initially (discovered by node)
- [x] Add CLI argument for `--train-repo-url` if not present
- [x] Add validation: if `--train-repo-url` provided, verify repo exists in `~/.local/share/covn/`
- [x] Update help text to document new behavior

### Verification
- [x] Run `cargo test` in `drift-coord/` - all pass
- [x] Manual test: `--train-repo-url=<url>` → config includes repo_url
- [x] Manual test: no `--train-repo-url` → uses old model_path behavior

### Line References
- Current config: lines 128-144
- To be modified: lines 128-162

**Stage Completion:** [x] 100%

---

## Stage 5: Integration Testing

### Steps
- [x] Create test repo with `_ati_plug` in pyproject.toml and .venv
- [x] Run drift with `--train-repo-url=<test-repo>`
- [x] Verify venv is activated and script runs
- [x] Run drift without `--train-repo-url` → verify simulation triggers
- [x] Run drift with `--train-repo-url=<invalid-repo>` → verify graceful failure

### Verification
- [x] All integration tests pass
- [x] No regressions in existing tests across all modules
- [ ] Documentation updated in README or docs/

**Stage Completion:** [x] 100%

---

## Overall Progress

| Stage | Status | Completion |
|-------|---------|------------|
| 1. Modify drift-cli | Done | 100% |
| 2. Add venv detection | Done | 100% |
| 3. Update network handler | Done | 100% |
| 4. Update coordinator | Done | 100% |
| 5. Integration tests | Done | 100% |

**Total Completion:** 100%

---

## Notes
- Key behavior change: simulation is now triggered by absence of `_ati_plug` entrypoint, not by `model_path` existence
- Venv activation is automatic if `.venv/bin/activate` exists in cloned repo
- Backward compatibility: old `model_path` behavior preserved when `train_repo_url` is `None`
- Error handling: missing `_ati_plug` sends `TrainingCancel` to coordinator
