# maverick-adapter-radio-spi

Direct SPI uplink path for Semtech SX1302/SX1303 concentrators (Phase 2).

## Feature `spi`

Enable on the edge binary: `cargo build -p maverick-runtime-edge --features spi`.

Without `spi`, this crate compiles an empty API surface (workspace `cargo test` stays lightweight).

## libloragw status

`SpiUplinkSource::next_batch` currently runs a **placeholder** blocking poll: it checks that `spi_path` exists, sleeps for the configured read interval, and returns `UplinkReceive::Idle`. Wiring Semtech **libloragw** (`lgw_receive` → `UplinkObservation`) is tracked as the next integration step (vendored C sources + `build.rs`, per `.planning/phases/02-radio-abstraction-spi/02-RESEARCH.md`).

## Cross-compilation

ARM release builds need the same sysroot toolchain headers as other native code in this repo (see `.github/workflows/release.yml`). When libloragw is vendored, add any extra `libc` / math linkage notes here.
