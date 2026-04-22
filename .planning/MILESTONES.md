# Milestones

## v1.0 MVP (Shipped: 2026-04-17)

**Phases completed:** 13 phases, 35+ plans, 175 commits
**Timeline:** 2026-04-08 → 2026-04-17 (9 days)
**Files changed:** 331 files, 39,119 insertions(+), 1,310 deletions(-)
**LOC:** ~57,823 (Rust + TOML)

**Key accomplishments:**

1. **Protocol Correctness** — MIC verification (AES-128 CMAC) with LoRaWAN spec test vectors, 32-bit FCnt reconstruction, NwkSKey/AppSKey per-session storage, duplicate detection, region inference (AU915/AS923)
2. **Radio Abstraction & SPI** — UplinkSource port trait enabling SPI ↔ UDP swap, circuit-breaker resilience, hexagonal architecture for all I/O
3. **Protocol Security** — AppSKey payload decryption (AES-128 CTR), full UDP ingest path: GWMP parse → session lookup → MIC verify → SQLite persist
4. **Class A Downlink** — RX1/RX2 scheduling design, LinkCheckAns MAC commands, SQLite-backed downlink queue persistence (survives restart)
5. **Process Supervision** — systemd Restart=always, watchdog for hung process detection, clean shutdown with WAL checkpoint
6. **TUI Device Management** — Wizard-based add/edit/remove, device list with last-seen and uplink count, autoprovision-pending promotion, lns-config.toml import
7. **Hardware Testing** — Full RAK Pi bring-up, hardware probe on startup (CPU/RAM/storage/arch), SPI concentrator detection
8. **libloragw SPI Integration** — Vendored sx1302_hal C sources, real `lgw_receive()` integration, bindgen FFI bindings
9. **Auto-Update Mechanism** — systemd timer + bash script for atomic self-updates on ARM gateways, HTTPS version checking, backup rotation
10. **CI Hardening** — Multi-arch Linux builds (x86_64, aarch64, armv7), release artifacts with SPI-enabled ARM binaries, sysroot cross-compilation detection

**Known gaps at close:**
- Phase 09-D (Auto-Detection Verification): integration tests pending hardware availability
- SEC-02 (SQLite key encryption): deferred to v1.1 (domain model refactor required)
- DWNL-01..DWNL-06 (full downlink SPI TX wiring): deferred to v1.1
- RADIO-01 (full SPI RX/TX on real ARM hardware): pending field testing

---

*For current status, see .planning/ROADMAP.md*
