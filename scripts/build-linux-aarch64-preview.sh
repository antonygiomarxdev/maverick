#!/usr/bin/env bash
# Run INSIDE Docker (see README block below). Builds aarch64 binaries into dist/pi-preview/.
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
apt-get update -q
apt-get install -y --no-install-recommends \
  ca-certificates curl build-essential pkg-config \
  gcc-aarch64-linux-gnu libc6-dev-arm64-cross

if ! command -v cargo >/dev/null 2>&1; then
  curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain stable
  # shellcheck source=/dev/null
  [[ -f "${HOME}/.cargo/env" ]] && . "${HOME}/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found" >&2
  exit 1
fi
rustup target add aarch64-unknown-linux-gnu

export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
export AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar
export CFLAGS_aarch64_unknown_linux_gnu=--sysroot=/usr/aarch64-linux-gnu

cd /workspace
cargo build --locked --release -p maverick-runtime-edge --target aarch64-unknown-linux-gnu
cargo build --locked --release -p maverick-extension-tui --target aarch64-unknown-linux-gnu

mkdir -p /workspace/dist/pi-preview
cp -a /workspace/target/aarch64-unknown-linux-gnu/release/maverick-edge /workspace/dist/pi-preview/
cp -a /workspace/target/aarch64-unknown-linux-gnu/release/maverick-edge-tui /workspace/dist/pi-preview/
chmod +x /workspace/dist/pi-preview/maverick-edge /workspace/dist/pi-preview/maverick-edge-tui
echo "OK: /workspace/dist/pi-preview ready."
