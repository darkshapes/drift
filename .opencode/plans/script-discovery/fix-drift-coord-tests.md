# Fix drift-coord Test Compilation Issues

## Summary
drift-coord tests fail to compile due to missing `env_vars` field in `TrainConfig` initializers.

---

## Stage 1: Add `env_vars` Field to Test Configs

| Step | Completion | Code Lines |
|------|------------|------------|
| Add `env_vars: None` to coordinator_tests.rs line 22 config | ☐ 0% | drift-coord/tests/coordinator_tests.rs:22-39 |
| Add `env_vars: None` to auth.rs create_train_config() helper | ☐ 0% | drift-coord/src/auth.rs:216-229 |

### Checklist
- [ ] ☐ coordinator_tests.rs:22 config updated with `env_vars: None`
- [ ] ☐ auth.rs:216 create_train_config() updated with `env_vars: None`

### Verification
```bash
cd /Users/f784e/Documents/darkshapes/drift && cargo test --package drift-coord --no-run
```
Expected: compilation succeeds with no errors.

---

## Stage 2: Clean Up Unused Imports

| Step | Completion | Code Lines |
|------|------------|------------|
| Remove unused `NodeStatus` import at line 120 | ☐ 0% | drift-coord/tests/coordinator_tests.rs:120 |
| Remove unused `PeerEntry` import at line 152 | ☐ 0% | drift-coord/tests/coordinator_tests.rs:152 |
| Prefix `parent` variable with underscore at line 99 | ☐ 0% | drift-coord/tests/coordinator_tests.rs:99 |

### Checklist
- [ ] ☐ Line 120: `use drift_coord::peer_registry::PeerRegistry;` (remove NodeStatus)
- [ ] ☐ Line 152: `use drift_coord::peer_registry::PeerRegistry;` (remove PeerEntry)
- [ ] ☐ Line 99: `if let Some(_parent) = reg_path.parent() {`

### Verification
```bash
cd /Users/f784e/Documents/darkshapes/drift && cargo test --package drift-coord --no-run 2>&1 | grep -E "(error|warning)"
```
Expected: no errors, warnings reduced to 0.

---

## Stage 3: Final Verification

| Step | Completion | Code Lines |
|------|------------|------------|
| Run all drift-coord tests | ☐ 0% | drift-coord/tests/*.rs |

### Checklist
- [ ] ☐ All tests compile and pass

### Verification
```bash
cd /Users/f784e/Documents/darkshapes/drift && cargo test --package drift-coord
```
Expected: all tests pass, 0 errors, 0 warnings.

---

## Implementation Notes

### Changes Required
1. **drift-coord/tests/coordinator_tests.rs:22-39**
   - Add `env_vars: None,` after `training_spawn_cmd: None,`

2. **drift-coord/src/auth.rs:216-229**
   - Add `env_vars: None,` to `create_train_config()` function

### Why This Fix Works
The `TrainConfig` struct in `drift-proto` now includes `env_vars: Option<HashMap<String, String>>` field marked with `#[serde(default)]`. While serde provides a default during deserialization, explicit struct initialization requires all fields to be specified.

### Risk Assessment
- **Low risk**: Adding `None` to optional field
- **No behavioral change**: Tests remain functionally identical
- **Backwards compatible**: Uses default/None value

---

## Status
☐ Ready for implementation
