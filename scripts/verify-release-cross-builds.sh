#!/usr/bin/env bash
# Reproduce Release workflow cross-compiles locally (aarch64 + armv7) using Docker.
# Matches packages and CFLAGS sysroots in .github/workflows/release.yml.
#
# Requirements: Docker (Linux, WSL2, or Docker Desktop).
# Usage: from repo root — bash scripts/verify-release-cross-builds.sh
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

IMAGE="${RUST_VERIFY_IMAGE:-rust:1-bookworm}"

echo "Using image: ${IMAGE}"
echo "Workspace: ${ROOT}"

docker run --rm \
  -v "${ROOT}:/workspace" \
  -w /workspace \
  "${IMAGE}" \
  bash -ce '
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
apt-get update -q
apt-get install -y --no-install-recommends \
  ca-certificates \
  curl \
  build-essential \
  pkg-config \
  gcc-aarch64-linux-gnu \
  libc6-dev-arm64-cross \
  gcc-arm-linux-gnueabihf \
  libc6-dev-armhf-cross

if ! command -v cargo >/dev/null 2>&1; then
  curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain stable
fi

if [[ -f "$HOME/.cargo/env" ]]; then
  . "$HOME/.cargo/env"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is not available after toolchain bootstrap" >&2
  exit 1
fi

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required to add cross compilation targets" >&2
  exit 1
fi

rustup target add aarch64-unknown-linux-gnu armv7-unknown-linux-gnueabihf

build_target() {
  local triple="$1"
  case "$triple" in
    aarch64-unknown-linux-gnu)
      export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
      export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
      export AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar
      export CFLAGS_aarch64_unknown_linux_gnu=--sysroot=/usr/aarch64-linux-gnu
      ;;
    armv7-unknown-linux-gnueabihf)
      export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc
      export CC_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-gcc
      export AR_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-ar
      export CFLAGS_armv7_unknown_linux_gnueabihf=--sysroot=/usr/arm-linux-gnueabihf
      ;;
    *)
      echo "unknown triple: $triple" >&2
      exit 1
      ;;
  esac
  cargo build --locked --release -p maverick-runtime-edge --target "$triple"
  cargo build --locked --release -p maverick-extension-tui --target "$triple"
}

echo "==> native x86_64 (sanity)"
cargo build --locked --release -p maverick-runtime-edge
cargo build --locked --release -p maverick-extension-tui

echo "==> aarch64-unknown-linux-gnu"
build_target aarch64-unknown-linux-gnu

echo "==> armv7-unknown-linux-gnueabihf"
build_target armv7-unknown-linux-gnueabihf

echo "OK: release cross-builds match CI expectations (see .github/workflows/release.yml)."
'
