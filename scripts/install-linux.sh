#!/usr/bin/env bash
set -euo pipefail

REPO_OWNER="antonygiomarxdev"
REPO_NAME="maverick"
INSTALL_DIR="/usr/local/bin"
VERSION="latest"

usage() {
  cat <<EOF
install-linux.sh - install maverick-edge on Linux

Usage:
  $0 [--version <tag|latest>] [--install-dir <path>]

Examples:
  $0 --version latest
  $0 --version v0.1.0 --install-dir /usr/local/bin
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      VERSION="${2:-}"
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "this installer supports Linux only" >&2
  exit 1
fi

ARCH_RAW="$(uname -m)"
case "${ARCH_RAW}" in
  x86_64)
    TARGET="x86_64-unknown-linux-gnu"
    ;;
  aarch64|arm64)
    TARGET="aarch64-unknown-linux-gnu"
    ;;
  armv7l|armv7)
    TARGET="armv7-unknown-linux-gnueabihf"
    ;;
  *)
    echo "unsupported architecture: ${ARCH_RAW}" >&2
    exit 1
    ;;
esac

if [[ "${VERSION}" == "latest" ]]; then
  VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest" | sed -n 's/.*"tag_name": "\([^"]*\)".*/\1/p' | head -n 1)"
fi

if [[ -z "${VERSION}" ]]; then
  echo "failed to resolve release version" >&2
  exit 1
fi

ASSET="maverick-${TARGET}.tar.gz"
SHA_FILE="${ASSET}.sha256"
BASE_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${VERSION}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "Installing maverick-edge ${VERSION} (${TARGET})..."
curl -fsSL "${BASE_URL}/${ASSET}" -o "${TMP_DIR}/${ASSET}"
curl -fsSL "${BASE_URL}/${SHA_FILE}" -o "${TMP_DIR}/${SHA_FILE}"

(
  cd "${TMP_DIR}"
  sha256sum -c "${SHA_FILE}"
)

tar -xzf "${TMP_DIR}/${ASSET}" -C "${TMP_DIR}"
chmod +x "${TMP_DIR}/maverick-edge"

if [[ ! -w "${INSTALL_DIR}" ]]; then
  sudo install -m 0755 "${TMP_DIR}/maverick-edge" "${INSTALL_DIR}/maverick-edge"
else
  install -m 0755 "${TMP_DIR}/maverick-edge" "${INSTALL_DIR}/maverick-edge"
fi

echo "Installed: ${INSTALL_DIR}/maverick-edge"
echo "Run smoke checks:"
echo "  maverick-edge --help"
echo "  maverick-edge status"
