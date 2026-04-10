# Test Program (baseline)

## Layers

1. **Unit** — `cargo test -p maverick-core` (protocol policy, storage policy, use cases with in-memory ports).
2. **Integration smoke** — `cargo test -p maverick-integration-tests` (cross-crate JSON/contracts, SQLite persistence + retention + reopen).
3. **Contract** — envelope roundtrip tests in integration crate; extension schema version `EXTENSION_CONTRACT_VERSION`.
4. **Fault-injection** — SQLite `SQLITE_BUSY` / concurrent writer covered in `tests/persistence_sqlite.rs` (`sqlite_concurrent_transaction_waits_on_busy_then_succeeds`). Radio transport coverage now includes timeout + circuit breaker + half-open recovery + GWMP parse path + burst parse (`cargo test -p maverick-adapter-radio-udp`) and cross-crate resilience/parse-failure continuity in `tests/radio_transport_resilience.rs`.
5. **Soak** — long-run stability harness on reference hardware (not yet automated).

### SQLite / persistence tests

- Crate: `maverick-adapter-persistence-sqlite` is built with `[lib] test = false` to avoid an extra empty/harness binary on some Windows lockdown setups; persistence tests live under `maverick-integration-tests/tests/persistence_sqlite.rs`.
- Evidence log: `docs/slice-2-evidence.md`, `docs/slice-3-evidence.md`, `docs/slice-4-evidence.md`.

## Commands

```bash
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```
