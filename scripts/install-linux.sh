#!/usr/bin/env bash
set -euo pipefail

REPO_OWNER="antonygiomarxdev"
REPO_NAME="maverick"
INSTALL_DIR="/usr/local/bin"
VERSION="latest"
AUTO_INSTALL_DEPS=1
PACKAGE_MANAGER=""
SKIP_RUNTIME_CHECK=0
REQUIRED_GLIBC="2.34"

version_lt() {
  local lhs="$1"
  local rhs="$2"
  local i
  local lhs_parts=()
  local rhs_parts=()

  IFS='.' read -r -a lhs_parts <<< "${lhs}"
  IFS='.' read -r -a rhs_parts <<< "${rhs}"

  for ((i=0; i<${#lhs_parts[@]} || i<${#rhs_parts[@]}; i++)); do
    local lhs_part="${lhs_parts[i]:-0}"
    local rhs_part="${rhs_parts[i]:-0}"

    if ((10#${lhs_part} < 10#${rhs_part})); then
      return 0
    fi
    if ((10#${lhs_part} > 10#${rhs_part})); then
      return 1
    fi
  done

  return 1
}

detect_glibc_version() {
  local output=""

  if command -v getconf >/dev/null 2>&1; then
    output="$(getconf GNU_LIBC_VERSION 2>/dev/null || true)"
    if [[ -n "${output}" ]]; then
      awk '{ print $2; exit }' <<< "${output}"
      return 0
    fi
  fi

  if command -v ldd >/dev/null 2>&1; then
    output="$(ldd --version 2>/dev/null | head -n 1 || true)"
    if [[ -n "${output}" ]]; then
      grep -oE '[0-9]+\.[0-9]+' <<< "${output}" | head -n 1
      return 0
    fi
  fi

  return 1
}

ensure_runtime_compatibility() {
  if [[ "${SKIP_RUNTIME_CHECK}" -eq 1 ]]; then
    echo "Skipping runtime compatibility checks (--skip-runtime-check)."
    return
  fi

  local glibc_version=""
  glibc_version="$(detect_glibc_version || true)"

  if [[ -z "${glibc_version}" ]]; then
    echo "warning: could not detect glibc version; continuing install" >&2
    return
  fi

  if version_lt "${glibc_version}" "${REQUIRED_GLIBC}"; then
    cat >&2 <<EOF
error: host glibc ${glibc_version} is older than required ${REQUIRED_GLIBC} for published binaries.

The installer can auto-install common tools (curl/tar/coreutils), but glibc is part of the base OS
and cannot be safely upgraded in-place by this installer.

Recommended paths:
  1) Upgrade OS to a supported baseline (Raspberry Pi OS Lite Bookworm or Debian 12 minimal).
  2) Build from source on this host if you must stay on the current distro.
  3) Advanced only: re-run with --skip-runtime-check (install may still fail at runtime).
EOF
    exit 1
  fi
}

is_root() {
  [[ "${EUID:-$(id -u)}" -eq 0 ]]
}

run_privileged() {
  if is_root; then
    "$@"
  elif command -v sudo >/dev/null 2>&1; then
    sudo "$@"
  else
    echo "this operation requires root privileges; rerun as root or install sudo" >&2
    exit 1
  fi
}

detect_package_manager() {
  if [[ -n "${PACKAGE_MANAGER}" ]]; then
    return
  fi

  local manager
  for manager in apt-get dnf yum apk pacman zypper; do
    if command -v "${manager}" >/dev/null 2>&1; then
      PACKAGE_MANAGER="${manager}"
      return
    fi
  done
}

install_packages() {
  detect_package_manager

  if [[ -z "${PACKAGE_MANAGER}" ]]; then
    echo "could not find a supported package manager to install prerequisites" >&2
    exit 1
  fi

  echo "Installing missing prerequisites with ${PACKAGE_MANAGER}: $*"

  case "${PACKAGE_MANAGER}" in
    apt-get)
      run_privileged apt-get update -q
      run_privileged apt-get install -y --no-install-recommends "$@"
      ;;
    dnf)
      run_privileged dnf install -y "$@"
      ;;
    yum)
      run_privileged yum install -y "$@"
      ;;
    apk)
      run_privileged apk add --no-cache "$@"
      ;;
    pacman)
      run_privileged pacman -Sy --noconfirm "$@"
      ;;
    zypper)
      run_privileged zypper --non-interactive install --no-recommends "$@"
      ;;
    *)
      echo "unsupported package manager: ${PACKAGE_MANAGER}" >&2
      exit 1
      ;;
  esac
}

packages_for_command() {
  local cmd="$1"

  case "${PACKAGE_MANAGER}" in
    apt-get|dnf|yum|zypper|pacman)
      case "${cmd}" in
        curl)
          printf '%s\n' curl ca-certificates
          ;;
        tar)
          printf '%s\n' tar
          ;;
        sha256sum|install)
          printf '%s\n' coreutils
          ;;
        sudo)
          printf '%s\n' sudo
          ;;
      esac
      ;;
    apk)
      case "${cmd}" in
        curl)
          printf '%s\n' curl ca-certificates
          ;;
        tar)
          printf '%s\n' tar
          ;;
        sha256sum|install)
          printf '%s\n' coreutils
          ;;
        sudo)
          printf '%s\n' sudo
          ;;
      esac
      ;;
  esac
}

ensure_cmd() {
  local cmd="$1"
  if command -v "${cmd}" >/dev/null 2>&1; then
    return
  fi

  if [[ "${AUTO_INSTALL_DEPS}" -ne 1 ]]; then
    echo "missing required command: ${cmd}" >&2
    exit 1
  fi

  detect_package_manager
  local packages
  packages="$(packages_for_command "${cmd}")"
  if [[ -z "${packages}" ]]; then
    echo "missing required command: ${cmd}; automatic installation is not configured for ${PACKAGE_MANAGER:-this system}" >&2
    exit 1
  fi

  mapfile -t package_list <<< "${packages}"
  install_packages "${package_list[@]}"

  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "failed to install required command: ${cmd}" >&2
    exit 1
  fi
}

validate_binary() {
  local binary_path="$1"
  local label="$2"
  local output

  if ! output="$("${binary_path}" --help 2>&1)"; then
    echo "installed ${label} failed the --help smoke check" >&2
    if [[ -n "${output}" ]]; then
      echo "runtime output:" >&2
      echo "${output}" >&2
    fi

    if grep -Eq 'GLIBC_[0-9]|version `GLIBC|not found' <<< "${output}"; then
      cat >&2 <<EOF
hint: this usually means the release binary requires a newer glibc/runtime loader than the host distro.
      verify your distro baseline (Tier 1 edge: Raspberry Pi OS Lite Bookworm / Debian 12 minimal).
EOF
    fi

    exit 1
  fi
}

usage() {
  cat <<EOF
install-linux.sh - install maverick-edge (and maverick-edge-tui if present) on Linux

Usage:
  $0 [--version <tag|latest>] [--install-dir <path>] [--no-install-deps] [--skip-runtime-check]

One-liner (download and run in a single command; requires bash):
  curl -fsSL "https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/scripts/install-linux.sh" | bash -s -- --version latest --install-dir /usr/local/bin

Examples:
  $0 --version latest
  $0 --version v0.1.0 --install-dir /usr/local/bin
  $0 --version v0.1.0 --no-install-deps
  $0 --version v0.1.0 --skip-runtime-check
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
    --no-install-deps)
      AUTO_INSTALL_DEPS=0
      shift
      ;;
    --skip-runtime-check)
      SKIP_RUNTIME_CHECK=1
      shift
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

detect_package_manager

ensure_cmd curl
ensure_cmd tar
ensure_cmd sha256sum
ensure_cmd install

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

  ensure_runtime_compatibility

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

expected_hash="$(awk 'NF { print $1; exit }' "${TMP_DIR}/${SHA_FILE}")"
if [[ -z "${expected_hash}" ]]; then
  echo "failed to parse expected checksum from ${SHA_FILE}" >&2
  exit 1
fi

actual_hash="$(sha256sum "${TMP_DIR}/${ASSET}" | awk '{ print $1 }')"
if [[ "${actual_hash}" != "${expected_hash}" ]]; then
  echo "sha256sum mismatch for ${ASSET}" >&2
  echo "expected: ${expected_hash}" >&2
  echo "actual:   ${actual_hash}" >&2
  exit 1
fi

tar -xzf "${TMP_DIR}/${ASSET}" -C "${TMP_DIR}"
chmod +x "${TMP_DIR}/maverick-edge"

install_one() {
  local src="$1"
  local dest_name="$2"
  if [[ ! -w "${INSTALL_DIR}" ]]; then
    run_privileged install -d "${INSTALL_DIR}"
    run_privileged install -m 0755 "${src}" "${INSTALL_DIR}/${dest_name}"
  else
    install -d "${INSTALL_DIR}"
    install -m 0755 "${src}" "${INSTALL_DIR}/${dest_name}"
  fi
}

install_one "${TMP_DIR}/maverick-edge" "maverick-edge"
validate_binary "${INSTALL_DIR}/maverick-edge" "maverick-edge"

if [[ -f "${TMP_DIR}/maverick-edge-tui" ]]; then
  chmod +x "${TMP_DIR}/maverick-edge-tui"
  install_one "${TMP_DIR}/maverick-edge-tui" "maverick-edge-tui"
  validate_binary "${INSTALL_DIR}/maverick-edge-tui" "maverick-edge-tui"
  echo "Installed: ${INSTALL_DIR}/maverick-edge and ${INSTALL_DIR}/maverick-edge-tui"
else
  echo "Installed: ${INSTALL_DIR}/maverick-edge (no maverick-edge-tui in this tarball)"
fi

if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
  echo "Note: ${INSTALL_DIR} is not currently in PATH. Add it to your shell profile before using maverick-edge directly."
fi

echo "Run smoke checks:"
echo "  maverick-edge --help"
echo "  maverick-edge status"
