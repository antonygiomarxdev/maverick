---
plan: 01-C
phase: 01-protocol-correctness
status: complete
tasks_completed: 2
tasks_total: 2
requirements_covered:
  - PROT-01
  - PROT-02
  - PROT-03
  - PROT-04
  - CORE-02
---

## Summary

Wired MIC verification (AES-128 CMAC) and FRMPayload decryption (AES-128-CTR) into `IngestUplink::execute`, with full 32-bit FCnt reconstruction before both. Added `aes = "0.9"` and `cmac = "0.8"` to maverick-core. Integration tests updated to compute valid MIC fixtures.

## Tasks

### Task C-1: Add crypto dependencies

Added to `crates/maverick-core/Cargo.toml`:
- `aes = "0.9"` and `cmac = "0.8"` (no feature flags required)
- `tracing = { workspace = true }` (needed for dedup debug logging added in D)

### Task C-2: Extend IngestUplink::execute

**New private helpers** in `ingest_uplink.rs`:
- `build_b0_uplink(dev_addr, f_cnt, phy_len)` — B0 block for MIC computation (LE fields per spec §4.4)
- `compute_mic(nwk_s_key, b0, phy_without_mic)` — AES-128 CMAC, first 4 bytes
- `decrypt_frm_payload(app_s_key, dev_addr, f_cnt, payload)` — AES-128-CTR, counter starts at 1 per §4.3.3.2
- `now_ms_portable()` — wall-clock ms without depending on SQLite adapter

**execute pipeline** (order matters):
1. Fetch session
2. No-session fast path → `RejectNoSession`
3. `extend_fcnt` → `reconstructed_fcnt` (u32)
4. `validate_uplink` for region/class checks
5. `compute_mic` using `session.nwk_s_key` + `reconstructed_fcnt` — reject if mismatch
6. `decrypt_frm_payload` using `session.app_s_key`
7. `UplinkRecord::append` with `reconstructed_fcnt`, `received_at_ms`, `payload_decrypted`
8. Session counter updated to `reconstructed_fcnt`

**Tests added/updated**:
- `obs_with_valid_mic()` helper computes correct CMAC for zero NwkSKey test fixtures
- `ingest_happy_path_updates_session_and_uplink` — uses valid MIC
- `ingest_rejects_bad_fcnt` — FCnt=5 with session=5 → Duplicate (before MIC)
- `ingest_rejects_bad_mic` — wrong wire_mic → `AppError::Domain("mic_invalid")`

Integration tests patched to compute valid MIC using `build_b0_uplink`/`compute_mic` (pub-exported).

## Key Files

### Modified
- `crates/maverick-core/Cargo.toml` — aes, cmac, tracing deps
- `crates/maverick-core/src/use_cases/ingest_uplink.rs` — full pipeline
- `crates/maverick-core/src/use_cases/mod.rs` — re-export build_b0_uplink, compute_mic
- `crates/maverick-integration-tests/tests/operator_local_gateway_e2e.rs` — MIC fix
- `crates/maverick-integration-tests/tests/persistence_sqlite.rs` — MIC fix

## Self-Check: PASSED

- `grep "extend_fcnt\|compute_mic\|decrypt_frm_payload" ingest_uplink.rs` ✓
- `grep "aes\|cmac" maverick-core/Cargo.toml` ✓
- `cargo test -p maverick-core` 16 passed ✓
- `cargo test --workspace` all pass ✓
