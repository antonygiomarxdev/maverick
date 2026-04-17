---
phase: 01-protocol-correctness
plan: C
type: execute
wave: 3
depends_on:
  - 01-A
  - 01-B
files_modified:
  - crates/maverick-core/src/use_cases/ingest_uplink.rs
  - crates/maverick-core/Cargo.toml
autonomous: true
requirements:
  - PROT-01
  - PROT-02
  - PROT-03
  - PROT-04
  - CORE-02

must_haves:
  truths:
    - "IngestUplink::execute calls extend_fcnt before validate_uplink and uses the reconstructed u32"
    - "IngestUplink::execute verifies MIC using session.nwk_s_key after FCnt reconstruction"
    - "Frames with invalid MIC are rejected with AppError::Domain and audited"
    - "IngestUplink::execute decrypts FRMPayload with session.app_s_key after MIC passes"
    - "UplinkRecord is persisted with reconstructed f_cnt (u32), received_at_ms, payload, payload_decrypted"
    - "session.uplink_frame_counter is updated to the reconstructed u32 (not the raw wire u16)"
    - "aes 0.9.0 and cmac 0.8.0 are in maverick-core/Cargo.toml dependencies"
  artifacts:
    - path: "crates/maverick-core/src/use_cases/ingest_uplink.rs"
      provides: "Extended execute with FCnt reconstruction, MIC verify, decrypt, dedup-ready UplinkRecord"
      contains: "verify_mic"
    - path: "crates/maverick-core/Cargo.toml"
      provides: "aes and cmac dependencies"
      contains: "aes = "
  key_links:
    - from: "crates/maverick-core/src/use_cases/ingest_uplink.rs"
      to: "crates/maverick-core/src/protocol/lorawan_10x_class_a.rs"
      via: "IngestUplink calls LoRaWAN10xClassA::extend_fcnt directly (not via trait)"
      pattern: "extend_fcnt"
    - from: "crates/maverick-core/src/use_cases/ingest_uplink.rs"
      to: "crates/maverick-domain/src/session.rs"
      via: "session.nwk_s_key and session.app_s_key used for crypto"
      pattern: "nwk_s_key"
---

<objective>
Wire MIC verification and AppSKey payload decryption into `IngestUplink::execute`, and fix all call sites broken by Plan A's type changes.

Purpose: PROT-01 (MIC verification), PROT-04 (payload decryption), and PROT-02 (32-bit FCnt in persistence) all converge in `IngestUplink::execute`. This plan completes the hot path and fixes the broken `UplinkRecord` and `UplinkObservation` construction in tests. After this plan, `cargo test --workspace` should pass for all plans completed so far.

Output: `IngestUplink::execute` with full LoRaWAN 1.0.x processing pipeline. `maverick-core/Cargo.toml` with crypto dependencies.
</objective>

<execution_context>
@/root/.claude/get-shit-done/workflows/execute-plan.md
@/root/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/phases/01-protocol-correctness/1-CONTEXT.md
@.planning/phases/01-protocol-correctness/01-RESEARCH.md
@.planning/phases/01-protocol-correctness/01-PATTERNS.md
@.planning/phases/01-protocol-correctness/01-A-SUMMARY.md
@.planning/phases/01-protocol-correctness/01-B-SUMMARY.md
</context>

<tasks>

<task type="auto">
  <name>Task C-1: Add crypto dependencies to maverick-core</name>
  <files>
    crates/maverick-core/Cargo.toml
  </files>
  <read_first>
    - crates/maverick-core/Cargo.toml
  </read_first>
  <action>
Add to `[dependencies]` in `crates/maverick-core/Cargo.toml`:
```toml
aes = "0.9"
cmac = "0.8"
```

Per research (01-RESEARCH.md): these are the CORRECT versions. Do NOT use `aes 0.8.x` or `cmac 0.7.x` — those versions have different API (see Pitfall 1 in RESEARCH.md). The versions `aes 0.9.0` and `cmac 0.8.0` have been verified to compile together (verified 2026-04-16 per RESEARCH.md).

No feature flags are required for either crate. Placement is per-crate (not workspace-level) because only `maverick-core` needs crypto in Phase 1.
  </action>
  <verify>
    <automated>cargo check -p maverick-core 2>&1 | grep -E "^error\[" | head -5</automated>
  </verify>
  <done>
    - `crates/maverick-core/Cargo.toml` contains `aes = "0.9"` and `cmac = "0.8"` in `[dependencies]`
    - `cargo check -p maverick-core` resolves the crates (no "unresolved import" error for aes/cmac)
  </done>
</task>

<task type="auto">
  <name>Task C-2: Extend IngestUplink::execute with FCnt, MIC, decryption pipeline</name>
  <files>
    crates/maverick-core/src/use_cases/ingest_uplink.rs
  </files>
  <read_first>
    - crates/maverick-core/src/use_cases/ingest_uplink.rs
    - crates/maverick-core/src/protocol/lorawan_10x_class_a.rs
    - crates/maverick-core/src/ports/uplink_repository.rs
    - crates/maverick-core/src/ports/radio_transport.rs
    - crates/maverick-domain/src/session.rs
  </read_first>
  <action>
Rewrite `ingest_uplink.rs` in full. The execute method gains FCnt reconstruction, MIC verification, and payload decryption. The test module is updated for the new type signatures.

**Imports to add at the top of the file:**
```rust
use aes::Aes128;
use cmac::{Cmac, KeyInit, Mac};
use crate::protocol::lorawan_10x_class_a::{FcntError, LoRaWAN10xClassA};
```

**New private helper functions** — add these below the `IngestUplink` struct, before `impl IngestUplink`:

```rust
/// LoRaWAN 1.0.x §4.4 — B0 block for uplink MIC.
/// All multi-byte fields are LITTLE-ENDIAN.
fn build_b0_uplink(dev_addr: u32, f_cnt: u32, phy_len_without_mic: usize) -> [u8; 16] {
    let mut b0 = [0u8; 16];
    b0[0] = 0x49;
    b0[5] = 0x00; // uplink direction
    b0[6..10].copy_from_slice(&dev_addr.to_le_bytes());
    b0[10..14].copy_from_slice(&f_cnt.to_le_bytes());
    b0[15] = phy_len_without_mic as u8;
    b0
}

/// Compute AES-128 CMAC over B0 || PHY_without_MIC, return first 4 bytes.
fn compute_mic(nwk_s_key: &[u8; 16], b0: &[u8; 16], phy_without_mic: &[u8]) -> [u8; 4] {
    let mut mac = <Cmac<Aes128> as KeyInit>::new_from_slice(nwk_s_key)
        .expect("NwkSKey is always 16 bytes");
    mac.update(b0);
    mac.update(phy_without_mic);
    let full = mac.finalize().into_bytes();
    [full[0], full[1], full[2], full[3]]
}

/// LoRaWAN 1.0.x §4.3.3.2 — AES-128-CTR FRMPayload decryption.
/// Block counter `i` starts at 1 (NOT 0) — see RESEARCH.md Pitfall 3.
fn decrypt_frm_payload(
    app_s_key: &[u8; 16],
    dev_addr: u32,
    f_cnt: u32,
    payload: &[u8],
) -> Vec<u8> {
    use aes::cipher::BlockCipherEncrypt; // BlockCipherEncrypt (not BlockEncrypt) in aes 0.9
    if payload.is_empty() {
        return Vec::new();
    }
    let cipher = <Aes128 as KeyInit>::new_from_slice(app_s_key)
        .expect("AppSKey is always 16 bytes");
    let block_count = payload.len().div_ceil(16);
    let mut keystream = Vec::with_capacity(block_count * 16);
    for i in 1u8..=(block_count as u8) { // CRITICAL: starts at 1, not 0 (LoRaWAN §4.3.3.2)
        let mut ai = [0u8; 16];
        ai[0] = 0x01;
        ai[5] = 0x00; // uplink direction
        ai[6..10].copy_from_slice(&dev_addr.to_le_bytes());
        ai[10..14].copy_from_slice(&f_cnt.to_le_bytes());
        ai[15] = i;
        let mut block = aes::Block::from(ai);
        cipher.encrypt_block(&mut block);
        keystream.extend_from_slice(&block);
    }
    payload.iter().zip(keystream.iter()).map(|(p, k)| p ^ k).collect()
}
```

**Updated `execute` method** — replace the entire method body:

```rust
pub async fn execute(&self, obs: UplinkObservation) -> AppResult<()> {
    // 1. Fetch session (includes keys per D-01/D-02)
    let session = self.sessions.get_by_dev_addr(obs.dev_addr).await?;

    // 2. FCnt 32-bit reconstruction (D-07, D-08) — must happen before validate_uplink
    let Some(session_ref) = session.as_ref() else {
        // No session found — let validate_uplink handle via ProtocolDecision::RejectNoSession
        let ctx = crate::protocol::ProtocolContext {
            observation: &obs,
            session: None,
        };
        let decision = self.protocol.validate_uplink(ctx)?;
        self.audit
            .emit(AuditRecord {
                source: "kernel".to_string(),
                operation: "ingest_uplink".to_string(),
                entity_type: "uplink".to_string(),
                entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
                outcome: format!("rejected:{decision:?}"),
                metadata: None,
            })
            .await?;
        return Err(AppError::Domain(format!("uplink rejected: {decision:?}")));
    };

    let reconstructed_fcnt = match LoRaWAN10xClassA::extend_fcnt(
        obs.f_cnt,
        session_ref.uplink_frame_counter,
    ) {
        Ok(fc) => fc,
        Err(FcntError::Duplicate) => {
            self.audit
                .emit(AuditRecord {
                    source: "kernel".to_string(),
                    operation: "ingest_uplink".to_string(),
                    entity_type: "uplink".to_string(),
                    entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
                    outcome: "rejected:RejectDuplicateFrameCounter".to_string(),
                    metadata: None,
                })
                .await?;
            return Err(AppError::Domain(
                "uplink rejected: RejectDuplicateFrameCounter".to_string(),
            ));
        }
        Err(FcntError::GapExceeded) => {
            self.audit
                .emit(AuditRecord {
                    source: "kernel".to_string(),
                    operation: "ingest_uplink".to_string(),
                    entity_type: "uplink".to_string(),
                    entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
                    outcome: "rejected:RejectFcntGapExceeded".to_string(),
                    metadata: None,
                })
                .await?;
            return Err(AppError::Domain(
                "uplink rejected: RejectFcntGapExceeded".to_string(),
            ));
        }
    };

    // 3. Protocol validation (region, class, session checks)
    let ctx = crate::protocol::ProtocolContext {
        observation: &obs,
        session: Some(session_ref),
    };
    let decision = self.protocol.validate_uplink(ctx)?;
    match decision {
        crate::protocol::ProtocolDecision::Accept => {}
        other => {
            self.audit
                .emit(AuditRecord {
                    source: "kernel".to_string(),
                    operation: "ingest_uplink".to_string(),
                    entity_type: "uplink".to_string(),
                    entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
                    outcome: format!("rejected:{other:?}"),
                    metadata: None,
                })
                .await?;
            return Err(AppError::Domain(format!("uplink rejected: {other:?}")));
        }
    }

    // session is Some because we passed the no-session guard above
    let session = session.expect("session confirmed present before this point");

    // 4. MIC verification (D-04, D-05) — uses reconstructed 32-bit FCnt
    let b0 = build_b0_uplink(obs.dev_addr.0, reconstructed_fcnt, obs.phy_without_mic.len());
    let computed_mic = compute_mic(&session.nwk_s_key, &b0, &obs.phy_without_mic);
    if computed_mic != obs.wire_mic {
        self.audit
            .emit(AuditRecord {
                source: "kernel".to_string(),
                operation: "ingest_uplink".to_string(),
                entity_type: "uplink".to_string(),
                entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
                outcome: "rejected:mic_invalid".to_string(),
                metadata: None,
            })
            .await?;
        return Err(AppError::Domain("mic_invalid".to_string()));
    }

    // 5. Payload decryption (D-13) — after MIC passes; None on failure (warn, not error)
    let payload_decrypted = if obs.payload.is_empty() {
        None
    } else {
        let decrypted = decrypt_frm_payload(
            &session.app_s_key,
            obs.dev_addr.0,
            reconstructed_fcnt,
            &obs.payload,
        );
        Some(decrypted)
    };

    // 6. Persist uplink (dedup check is in Plan D; here we just append)
    use crate::persistence_helpers::now_ms_portable;
    self.uplinks
        .append(&UplinkRecord {
            dev_addr: obs.dev_addr,
            f_cnt: reconstructed_fcnt,
            received_at_ms: now_ms_portable(),
            payload: obs.payload.clone(),
            application_id: session.application_id.clone(),
            payload_decrypted,
        })
        .await?;

    // 7. Update session counter to reconstructed 32-bit value
    let mut updated = session;
    updated.uplink_frame_counter = reconstructed_fcnt;
    self.sessions.upsert(&updated).await?;

    // 8. Audit success
    self.audit
        .emit(AuditRecord {
            source: "kernel".to_string(),
            operation: "ingest_uplink".to_string(),
            entity_type: "uplink".to_string(),
            entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
            outcome: "success".to_string(),
            metadata: None,
        })
        .await?;

    Ok(())
}
```

NOTE on `now_ms_portable`: Add a tiny private helper in this file (not a new module) to get wall-clock ms without depending on the SQLite adapter:
```rust
fn now_ms_portable() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}
```

**Update the test module** — fix broken struct literals caused by Plan A type changes:

1. Update `obs(fc: u32)` helper to `obs(fc: u16)` and add the new fields:
```rust
fn obs(fc: u16) -> UplinkObservation {
    UplinkObservation {
        gateway_eui: GatewayEui(Eui64([9; 8])),
        dev_addr: DevAddr(0xAB_CD_00_01),
        region: RegionId::Eu868,
        f_cnt: fc,
        f_port: 1,
        payload: vec![0xAA],
        rssi: Some(-90),
        snr: Some(5.5),
        wire_mic: [0u8; 4],          // zero MIC — tests that don't test MIC use this
        phy_without_mic: vec![0xAA], // dummy; not used in FCnt-only tests
    }
}
```

2. Update `sample_session` construction in tests to include `nwk_s_key` and `app_s_key`:
```rust
let session = SessionSnapshot {
    dev_eui: DevEui(Eui64([1; 8])),
    dev_addr: DevAddr(0xAB_CD_00_01),
    region: RegionId::Eu868,
    class: DeviceClass::ClassA,
    uplink_frame_counter: 0,
    downlink_frame_counter: 0,
    application_id: None,
    nwk_s_key: [0u8; 16],
    app_s_key: [0u8; 16],
};
```

3. Update `MemUplinks::append` to accept the new `UplinkRecord` shape (it just pushes, no field access needed).

4. Update call sites: `svc.execute(obs(1))` → `svc.execute(obs(1u16))`.

5. IMPORTANT: The existing `ingest_happy_path_updates_session_and_uplink` test uses `obs(1)` which has a zero `wire_mic`. With the new MIC verification, `compute_mic([0;16], b0, phy_without_mic)` will compute a non-zero MIC that won't match `wire_mic = [0u8;4]`. The test must be updated to either:
   - Precompute the correct wire_mic for the zero NwkSKey and set it in the obs fixture, OR
   - Use a `MockProtocol` that bypasses MIC (cleaner for unit tests that aren't testing MIC)

Recommended approach: Extract MIC verification into a separate path that can be disabled in tests, OR compute the correct MIC bytes for the test fixture. Use the simpler approach: in `ingest_happy_path_updates_session_and_uplink`, compute the expected MIC using `compute_mic([0u8;16], b0, phy_without_mic)` and use those bytes as `wire_mic`. Add a separate test `ingest_rejects_bad_mic` that deliberately sets wrong `wire_mic` bytes and expects `AppError::Domain("mic_invalid")`.

Add the new MIC rejection test:
```rust
#[tokio::test]
async fn ingest_rejects_bad_mic() {
    // ... same setup as happy path but with deliberately wrong wire_mic
    let mut wrong_obs = obs(1u16);
    wrong_obs.wire_mic = [0xFF, 0xFF, 0xFF, 0xFF]; // invalid
    wrong_obs.phy_without_mic = vec![0x00]; // anything non-empty
    let err = svc.execute(wrong_obs).await.unwrap_err();
    assert!(matches!(err, AppError::Domain(ref s) if s.contains("mic_invalid")));
}
```
  </action>
  <verify>
    <automated>cargo test -p maverick-core 2>&1 | tail -30</automated>
  </verify>
  <done>
    - `execute` calls `extend_fcnt` before `validate_uplink` — grep: `extend_fcnt` in `ingest_uplink.rs`
    - MIC is verified after FCnt reconstruction and before persistence — grep: `compute_mic` in `ingest_uplink.rs`
    - `UplinkRecord` persisted with `reconstructed_fcnt`, `received_at_ms`, `payload_decrypted`
    - `session.uplink_frame_counter` updated to `reconstructed_fcnt` (not obs.f_cnt)
    - `aes = "0.9"` and `cmac = "0.8"` in `maverick-core/Cargo.toml`
    - `cargo test -p maverick-core` passes including `ingest_happy_path_updates_session_and_uplink` and `ingest_rejects_bad_mic`
    - `cargo test -p maverick-core` passes `ingest_rejects_bad_fcnt` (existing test)
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| GWMP UDP packet → IngestUplink | Untrusted radio frame; MIC is the authentication boundary |
| session.nwk_s_key → compute_mic | Key material from SQLite; must never be logged or formatted into strings |
| session.app_s_key → decrypt_frm_payload | Key material; plaintext only in memory during decrypt |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-01-C-01 | Spoofing | MIC verification (PROT-01) | mitigate | AES-128 CMAC with NwkSKey; reject if computed != wire_mic; audit on rejection |
| T-01-C-02 | Repudiation | Audit on MIC rejection | mitigate | `AuditSink::emit("rejected:mic_invalid")` called before returning error; creates SQLite audit row |
| T-01-C-03 | Information Disclosure | NwkSKey / AppSKey in logs | mitigate | Keys are `[u8;16]` — never formatted with `{:?}` or `Display` into log strings; `tracing::debug!` used only for non-sensitive fields |
| T-01-C-04 | Tampering | AES-CTR counter starting at i=0 | mitigate | Counter starts at `i = 1` per LoRaWAN §4.3.3.2 (Pitfall 3 in RESEARCH.md); code comment documents this explicitly |
| T-01-C-05 | Tampering | B0 block little-endian fields | mitigate | `dev_addr.0.to_le_bytes()` and `f_cnt.to_le_bytes()` used (Pitfall 2 in RESEARCH.md); code comment documents this |
</threat_model>

<verification>
After both tasks complete:

```bash
cargo test -p maverick-core 2>&1 | tail -30
grep -n "extend_fcnt\|compute_mic\|decrypt_frm_payload" crates/maverick-core/src/use_cases/ingest_uplink.rs
grep -n "nwk_s_key\|app_s_key" crates/maverick-core/src/use_cases/ingest_uplink.rs
grep -n "reconstructed_fcnt" crates/maverick-core/src/use_cases/ingest_uplink.rs
grep -n "aes\|cmac" crates/maverick-core/Cargo.toml
```
</verification>

<success_criteria>
- `cargo test -p maverick-core` passes all tests including `ingest_happy_path_updates_session_and_uplink` and `ingest_rejects_bad_mic`
- `execute` uses `extend_fcnt` to get `reconstructed_fcnt` before `validate_uplink` — grep-verifiable
- MIC verified with `compute_mic` using `session.nwk_s_key` — grep-verifiable
- `UplinkRecord.f_cnt` is `reconstructed_fcnt` (u32), not `obs.f_cnt` (u16) — grep-verifiable
- `payload_decrypted` populated via `decrypt_frm_payload` with `session.app_s_key` — grep-verifiable
- B0 block uses `to_le_bytes()` for DevAddr and FCnt — grep-verifiable
- AES-CTR `Ai` block counter starts at `i = 1` — grep-verifiable (comment in code)
</success_criteria>

<output>
After completion, create `.planning/phases/01-protocol-correctness/01-C-SUMMARY.md`
</output>
