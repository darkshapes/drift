# Diagnosis: rand_core Version Conflict in Test Dependencies

## Root Cause

The `OsRng: rand_core::CryptoRng` errors indicate **multiple versions of the `rand_core` crate** being pulled in during test compilation. This happens because:

1. `cargo build` and `cargo test` resolve dependencies differently (test deps have different feature flags)
2. `iroh` brings in one version of `rand_core` via `rand` crate
3. `ed25519_dalek` (vendored, pre-release) expects a different `rand_core` version

## Affected Files (test only)

| File                            | Error Count | Notes                                                  |
| ------------------------------- | ----------- | ------------------------------------------------------ |
| `drift-auth/src/messages.rs`    | 4           | Test fixtures using `SigningKey::generate(&mut OsRng)` |
| `drift-auth/src/aggregator.rs`  | 11          | Same pattern                                           |
| `drift-auth/src/node.rs`        | 15          | Same pattern                                           |
| `drift-auth/src/coordinator.rs` | 3           | Same pattern                                           |

## Diagnosis

- **Severity**: Pre-existing infrastructure issue, not caused by signing fix
- **Scope**: Tests only (build passes fine)
- **Root cause**: Dependency resolution differs between `dev` and `test` profiles
- **Confirmed**: Stages 1-2 of singing_bug_fix.md are correctly implemented

## Next Steps

| Priority | Action                  | Description                                                |
| -------- | ----------------------- | ---------------------------------------------------------- |
| 1        | Fix dep versions        | Ensure consistent `rand_core` version across all test deps |
| 2        | Re-run tests            | Verify fix resolves all 33 errors                          |
| 3        | Manual integration test | Run coordinator + node manually if tests still fail        |

## Completion Criteria

Stage 3 is complete when:

- Build succeeds (already verified)
- All 33 `OsRng` errors resolved
- `cargo test --workspace` passes without errors
