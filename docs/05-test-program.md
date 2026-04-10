# Test Program (baseline)

## Layers

1. **Unit** — `cargo test -p maverick-core` (protocol policy, storage policy, use cases with in-memory ports).
2. **Integration smoke** — `cargo test -p maverick-integration-tests` (cross-crate JSON/contracts).
3. **Contract** — envelope roundtrip tests in integration crate; extension schema version `EXTENSION_CONTRACT_VERSION`.
4. **Fault-injection** — next slice: DB busy, adapter timeout, burst ingress (not yet automated in CI beyond unit mocks).
5. **Soak** — next slice: long-run stability harness on reference hardware.

## Commands

```bash
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```
