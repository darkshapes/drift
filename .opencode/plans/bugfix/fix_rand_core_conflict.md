# Plan: Fix rand_core Version Conflict in Test Dependencies

## Stage 1: Add Dependency Overrides to Root Cargo.toml

### Step 1.1: Pin rand_core to 0.9
- [x] [100%] Add `[dependencies]` override for `rand_core = "0.9"`
- **Lines:** 28-30 in root `Cargo.toml`
- **Status:** ☑ 100%

### Step 1.2: Pin signature to 3.x
- [x] [100%] Add override for `signature = "3"`
- **Lines:** 28-30 in root `Cargo.toml`
- **Status:** ☑ 100%

### Step 1.3: Pin rand to 0.9
- [x] [100%] Add override for `rand = "0.9"`
- **Lines:** 28-30 in root `Cargo.toml`
- **Status:** ☑ 100%

### Verification
- `cargo tree -p rand_core` shows single version
- `cargo tree -p signature` shows single version
- `cargo tree -p rand` shows single version

---

## Stage 2: Verify ed25519-dalek Feature Configuration

### Step 2.1: Check drif-auth dependencies
- [x] [100%] Ensure `ed25519-dalek` is included with `rand_core` feature
- **Lines:** 15 in `drift-auth/Cargo.toml`
- **Status:** ☑ 100%

### Step 2.2: Check iroh dependencies
- [x] [100%] Verify `iroh` pulls `ed25519-dalek` with `rand_core` feature
- **Lines:** 175 in `vendor/iroh-0.96.0/Cargo.toml`
- **Status:** ☑ 100%

### Verification
- `cargo tree -p ed25519-dalek` shows single version
- Feature flags align across all consumers

---

## Stage 3: Run Tests and Validate Fix

### Step 3.1: Clean and rebuild
- [x] [100%] Run `cargo clean`
- [x] [100%] Run `cargo build --workspace`
- **Status:** ☑ 100%

### Step 3.2: Run tests
- [x] [100%] Run `cargo test --workspace`
- **Status:** ☑ 100%

### Verification
- 0 `OsRng: rand_core::CryptoRng` errors
- 0 trait mismatch errors
- All tests pass

---

## Stage 4: Manual Integration Test (if needed)

### Step 4.1: Test coordinator + node manually
- [ ] [100%] Create minimal test harness in `drift-coord`
- [ ] [100%] Verify signing works end-to-end
- **Status:** ☐ 0%

---

## Completion Criteria

Stage 1 complete when:
- [x] Single `rand_core` version in dependency graph
- [x] Single `signature` version in dependency graph
- [x] Single `rand` version in dependency graph

Stage 2 complete when:
- [x] `ed25519-dalek` feature flags verified
- [x] No feature flag conflicts

Stage 3 complete when:
- [x] All 150 `OsRng` errors resolved
- [x] `cargo test --workspace` passes

Stage 4 complete when:
- [ ] Manual integration test passes (if Stage 3 fails)
