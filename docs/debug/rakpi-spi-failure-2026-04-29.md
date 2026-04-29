# RAK Pi SPI Failure — Root Cause Analysis

**Date:** 2026-04-29
**System:** Raspberry Pi 4 Model B Rev 1.5 + RAK2287 (SX1302 + SX1250)
**Reporter:** OpenCode (debugging session)
**Status:** ✅ Root cause identified — cross-compiled release binary breaks SPI initialization

---

## Executive Summary

The `maverick-edge.service` is in a crash loop on the RAK Pi because the **auto-update mechanism installed a GitHub release binary that fails at SPI concentrator initialization**. A locally-built binary from the same source code works correctly.

**Impact:** The auto-update feature (Phase 11) is actively harmful on SPI-enabled gateways until the cross-compilation issue is resolved.

---

## Evidence Timeline

| Time (CST) | Event |
|------------|-------|
| Apr 17 14:03 | Locally-built binary (`/home/pi/maverick/target/release/maverick-edge`) — **WORKS** |
| Apr 17 16:52 | GitHub release v1.0.2 published (cross-compiled aarch64) — **BROKEN** |
| Apr 17 17:19 | v1.0.1 installed via `install-linux.sh` — unknown SPI status |
| Apr 29 01:28 | Auto-update: "Already on latest version: 1.0.2" + created backup |
| Apr 29 02:27 | `/usr/local/bin/maverick-edge` modified to unknown binary (4,058,408 bytes) — **BROKEN** (`lgw_start failed: -1`) |
| Apr 29 06:28 | Auto-update: "Update complete" — downloaded release v1.0.2 |
| Apr 29 08:44 | Service in crash loop (restart counter: 143+) |

---

## Binary Comparison

| Binary | Source | Size | md5 | SPI Result |
|--------|--------|------|-----|------------|
| `/home/pi/maverick/target/release/maverick-edge` | Native build on Pi (Apr 17) | 3,888,776 | `ce0dd0a4...` | ✅ `listen timeout` (init success) |
| `/usr/local/bin/maverick-edge.bak` | Release v1.0.2 download (Apr 29) | 3,992,872 | `366169fc...` | ❌ `lgw_rxrf_setconf chain 0 failed: -1` |
| `/usr/local/bin/maverick-edge` | Unknown (Apr 29 02:27) | 4,058,408 | `1a2dfafc...` | ❌ `lgw_start failed: -1` |
| `/tmp/maverick-release-test/maverick-edge` | Fresh release v1.0.2 extract | 3,992,872 | `366169fc...` | ❌ `lgw_rxrf_setconf chain 0 failed: -1` |

**Conclusion:** The cross-compiled GitHub release binary is non-functional for SPI. The natively-compiled binary works.

---

## Error Details

### Cross-compiled release binary (v1.0.2)
```
Opening SPI communication interface
Note: chip version is 0x05 (v0.5)
ERROR: Failed to set SX1250_0 in STANDBY_RC mode
ERROR: failed to setup radio 0
bind failed: infrastructure: lgw_rxrf_setconf chain 0 failed: -1
```

**Failing function:** `lgw_rxrf_setconf()` in libloragw HAL  
**Failure point:** Radio frontend (SX1250) configuration after chip detection succeeds  
**Chip detection:** Works (reads `0x05` = SX1302 v0.5)

### Unknown binary (current, /usr/local/bin/maverick-edge)
```
Opening SPI communication interface
Note: chip version is 0x05 (v0.5)
ERROR: Failed to set SX1250_0 in STANDBY_RC mode
ERROR: failed to setup radio 0
bind failed: infrastructure: lgw_start failed: -1
```

**Failing function:** `lgw_start()` — earlier in init chain  
**This binary is NOT the release binary** (different size, different md5, different error)

### Working binary (native build)
```
{"detail":"listen timeout","failed":0,"ingested":0,"listen_bind":"/dev/spidev0.0","parsed":0,"received":0,"timeout_ms":5000}
```

**Result:** SPI init succeeds, waits 5s for packets (no LoRa traffic = timeout is expected)

---

## Root Cause Hypothesis

The cross-compiled binary fails because of a **compilation or linking difference** between the CI cross-build and the native Pi build. The libloragw C code is sensitive to:

1. **Struct padding/layout** — `lgw_rxrf_setconf` takes a `struct lgw_conf_rxrf_s`. Different compiler flags between cross-gcc and native gcc may produce incompatible struct layouts for SPI register configuration.
2. **Compiler optimization** — The cross-compiler (`aarch64-linux-gnu-gcc`) may optimize SPI timing-critical code differently than the native compiler, causing the SX1250 to not respond correctly to initialization sequences.
3. **config.h generation** — The CI runs `./scripts/gen_config_h.sh` before building. The local build on the Pi may have used a different `config.h` or the script may not exist/behave differently.
4. **Static linking of C code** — The Rust `cc::Build` compiles libloragw C sources inline. Cross-compilation with `--sysroot` may produce object code that behaves differently at runtime due to architecture-specific inline assembly or compiler intrinsics.

**Most likely:** The `lgw_rxrf_setconf` function writes SPI register values computed from a struct. If struct padding or field ordering differs between cross-compiled and native builds, the register values will be wrong, causing the SX1250 to reject the configuration.

---

## Additional Findings

### 1. Auto-update script bug: no pre-flight verification
The update script (`/usr/local/bin/maverick-update.sh`) downloads and installs the new binary **without testing it first**. It should:
- Download to a temp location
- Run `maverick-edge radio ingest-once` to verify SPI init works
- Only replace the production binary if verification passes

### 2. Auto-update script bug: service stop without restart on no-op
Even when no update is available, the script calls `systemctl stop maverick-edge.service` and then `systemctl start maverick-edge.service`. If the script crashes between stop and start, the service stays down.

### 3. Missing config.h in local repo
The local repo (`/home/pi/maverick/`) does not have `config.h` in `crates/maverick-adapter-radio-spi/libloragw/libloragw/inc/`. The nanobot workspace (`/home/pi/.nanobot/workspace/maverick-core/`) does. This suggests the local binary may have been built with a different config.h path or the build.rs generated it dynamically.

### 4. Unknown binary at 02:27
The current `/usr/local/bin/maverick-edge` (4,058,408 bytes) does not match any known binary:
- Not the release download (3,992,872 bytes)
- Not the local build (3,888,776 bytes)
- Not the debug build (74MB)

This binary may have been installed manually or by another process between 01:34 and 02:27.

---

## Immediate Fix

### Option A: Restore working binary (recommended)
```bash
sudo cp /home/pi/maverick/target/release/maverick-edge /usr/local/bin/maverick-edge
sudo chmod 755 /usr/local/bin/maverick-edge
sudo systemctl restart maverick-edge.service
```

### Option B: Disable auto-update temporarily
Edit `/etc/maverick/maverick.toml`:
```toml
[update]
mode = "dev"  # or comment out to disable
```

### Option C: Pin to working version
Modify the update script to skip releases until the SPI issue is resolved.

---

## Long-term Fixes (for v1.1)

### Fix 1: Add SPI smoke test to release CI
Before publishing a release, the CI should:
1. Build the binary
2. Run it against a real SPI device or emulator
3. Verify `lgw_start()` succeeds

### Fix 2: Add pre-install verification to update script
The update script should:
1. Download to temp location
2. Run `maverick-edge radio ingest-once` (or a new `maverick-edge verify-spi` command)
3. Only replace if verification passes
4. Auto-rollback on failure

### Fix 3: Investigate cross-compilation struct layout
- Compare `sizeof(struct lgw_conf_rxrf_s)` and field offsets between cross-compiled and native builds
- Add `#[repr(C)]` assertions or static asserts in the Rust FFI bindings
- Consider using `bindgen` to generate Rust structs from C headers to ensure layout compatibility

### Fix 4: Create GitHub Issue
Create a GitHub issue to track this cross-compilation SPI failure with:
- Binary hashes
- Error logs
- Working vs failing binary comparison
- CI build logs

---

## Verification Commands

```bash
# Check current binary
md5sum /usr/local/bin/maverick-edge

# Test SPI init
/usr/local/bin/maverick-edge radio ingest-once

# Check service status
systemctl status maverick-edge --no-pager

# View recent errors
journalctl -u maverick-edge --no-pager -n 20

# Test locally-built binary
/home/pi/maverick/target/release/maverick-edge radio ingest-once
```

---

## Related GitHub Issues to Create

1. **Issue #1: Cross-compiled aarch64 release binary fails SPI initialization on RAK2287**
   - Labels: `bug`, `spi`, `cross-compilation`, `hardware`
   - Assignee: TBD
   - Milestone: v1.1

2. **Issue #2: Auto-update script should verify binary before replacing**
   - Labels: `enhancement`, `update`, `reliability`
   - Milestone: v1.1

3. **Issue #3: Add SPI smoke test to release CI pipeline**
   - Labels: `ci`, `testing`, `spi`
   - Milestone: v1.1

---

---

## Update: Actual Root Cause Discovered

**Date:** 2026-04-29 (follow-up analysis)
**Status:** 🔄 Fix implemented, pending hardware verification

### Re-analysis

Further systematic debugging revealed that the **original root cause hypothesis (struct layout / cross-compilation) was incorrect**.

**Evidence that disproved struct layout theory:**
1. `git diff 06783b2..HEAD -- crates/maverick-adapter-radio-spi/libloragw/` shows **0 lines changed** — the C submodule is byte-for-byte identical
2. `git diff 06783b2..HEAD -- crates/maverick-adapter-radio-spi/src/lgw_bindings.rs` shows **0 lines changed** — Rust FFI bindings are identical
3. The Rust bindings already contain compile-time `static_assert` equivalents (`const _: () = { [...][size_of::<T>() - N]; };`) that verify struct sizes match
4. `lgw_init.rs` at commit `396acb8` (when first added) had `type_: 0` (LGW_RADIO_TYPE_NONE), which `lgw_rxrf_setconf` rejects — meaning SPI initialization **could never have worked** with the committed code

**Real root cause: Missing GPIO reset before `lgw_start()`**

The Semtech `lora_pkt_fwd` (packet forwarder) explicitly runs `./reset_lgw.sh start` before calling `lgw_start()`. Our `maverick-edge` binary never performed this hardware reset.

Without the GPIO reset:
- The SX1302 may be left in a bad state from a previous failed initialization
- SPI communication with the SX1302 itself works (version register reads OK)
- But SPI communication **through the SX1302 mux to the SX1250 radios fails**
- This manifests as `sx1250_setup()` failing with:
  ```
  ERROR: Failed to set SX1250_0 in STANDBY_RC mode
  ERROR: failed to setup radio 0
  ```

The "working" 3.9MB binary from Apr 17 was likely built **without `--features spi`** (hence smaller size), so it never called `lgw_start()` and never exposed this issue.

### Fix Applied

1. **Added `reset_concentrator()` to `lgw_init.rs`** — searches for and executes a reset script before `lgw_start()`
2. **Created `scripts/maverick-reset-spi.sh`** — GPIO reset script for CoreCell/RAK2287 reference design (GPIO 23, 18, 22, 13)
3. **Updated `deploy/systemd/maverick-edge.service`**:
   - Added `ExecStartPre=/usr/local/bin/maverick-reset-spi.sh start`
   - Added `SupplementaryGroups=gpio spi`
   - Added `/sys/class/gpio` to `ReadWritePaths`
4. **Updated release packaging** (`.github/workflows/release.yml`, `scripts/install-linux.sh`, `scripts/build-linux-aarch64-preview.sh`) to bundle and install the reset script

### Verification Needed

- [ ] Build cross-compiled binary and deploy to RAK Pi
- [ ] Confirm `maverick-edge radio ingest-once` returns `listen timeout` (not `lgw_start failed`)
- [ ] Confirm service runs stable for > 10 minutes

---

*Analysis completed: 2026-04-29*
*Fix committed: `54c0540`*
*Next action: Deploy to RAK Pi and verify SPI init succeeds*
