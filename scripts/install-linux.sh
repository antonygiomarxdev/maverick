#!/usr/bin/env bash
set -euo pipefail

REPO_OWNER="antonygiomarxdev"
REPO_NAME="maverick"
INSTALL_DIR="/usr/local/bin"
VERSION="latest"

usage() {
  cat <<EOF
install-linux.sh - install maverick-edge (and maverick-edge-tui if present) on Linux

Usage:
  $0 [--version <tag|latest>] [--install-dir <path>]

One-liner (download and run in a single command; requires bash):
  curl -fsSL "https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/scripts/install-linux.sh" | bash -s -- --version latest --install-dir /usr/local/bin

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
  LATEST_JSON="$(mktemp)"
  LATEST_URL="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest"
  if ! curl -fsSL "${LATEST_URL}" -o "${LATEST_JSON}"; then
    rm -f "${LATEST_JSON}"
    cat >&2 <<EOF
error: could not resolve latest release from GitHub (curl failed).

Why:
  GitHub only exposes /releases/latest after at least one Release exists with assets.
  Releases are created by this repo's workflow when a maintainer pushes a version tag (e.g. v0.1.0).
  Until then, this installer cannot download binaries.

What to do:
  1) Maintainer: push a tag to trigger the release workflow, then wait for CI to finish:
       git tag v0.1.0 && git push origin v0.1.0
     See: https://github.com/${REPO_OWNER}/${REPO_NAME}/actions
  2) Everyone: open https://github.com/${REPO_OWNER}/${REPO_NAME}/releases — if empty, step (1) is still needed.
  3) After a release exists, install a specific version (one-liner):
       curl -fsSL "https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/scripts/install-linux.sh" | bash -s -- --version v0.1.0 --install-dir ${INSTALL_DIR}
  4) Or build from source (repository README).

API tried: ${LATEST_URL}
EOF
    exit 1
  fi
  VERSION="$(sed -n 's/.*"tag_name": "\([^"]*\)".*/\1/p' "${LATEST_JSON}" | head -n 1)"
  rm -f "${LATEST_JSON}"
fi

if [[ -z "${VERSION}" ]]; then
  echo "failed to resolve release version (empty tag_name)" >&2
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

install_one() {
  local src="$1"
  local dest_name="$2"
  if [[ ! -w "${INSTALL_DIR}" ]]; then
    sudo install -m 0755 "${src}" "${INSTALL_DIR}/${dest_name}"
  else
    install -m 0755 "${src}" "${INSTALL_DIR}/${dest_name}"
  fi
}

install_one "${TMP_DIR}/maverick-edge" "maverick-edge"

if [[ -f "${TMP_DIR}/maverick-edge-tui" ]]; then
  chmod +x "${TMP_DIR}/maverick-edge-tui"
  install_one "${TMP_DIR}/maverick-edge-tui" "maverick-edge-tui"
  echo "Installed: ${INSTALL_DIR}/maverick-edge and ${INSTALL_DIR}/maverick-edge-tui"
else
  echo "Installed: ${INSTALL_DIR}/maverick-edge (no maverick-edge-tui in this tarball)"
fi

echo "Run smoke checks:"
echo "  maverick-edge --help"
echo "  maverick-edge status"
