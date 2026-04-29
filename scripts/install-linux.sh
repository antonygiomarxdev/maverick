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

# Installer / onboarding
INSTALLER_SCRIPT_VERSION="2026.04.13.1"
NON_INTERACTIVE=0
ASSUME_YES=0
SKIP_ONBOARDING=0
# If set, skip GitHub download and install binaries from this directory (developer / hardware preview).
LOCAL_DIST_DIR=""

MAVERICK_ETC="/etc/maverick"
RUNTIME_ENV_PATH="${MAVERICK_ETC}/runtime.env"
SETUP_JSON_PATH="${MAVERICK_ETC}/setup.json"

# Defaults (also used with --non-interactive when env is unset)
DEFAULT_DATA_DIR="${MAVERICK_DATA_DIR:-/var/lib/maverick}"
# Semtech GWMP UDP listen address (packet forwarder must push here). `maverick-edge probe` / `status`
# include this in `runtime_capabilities.selected_ingest.listen_bind` when the binary supports it.
DEFAULT_GWMP_BIND="${MAVERICK_GWMP_BIND:-0.0.0.0:17000}"
DEFAULT_LOOP_READ_TIMEOUT_MS="${MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS:-1000}"
DEFAULT_LOOP_MAX_MESSAGES="${MAVERICK_GWMP_LOOP_MAX_MESSAGES:-0}"
DEFAULT_INSTALL_CONSOLE="${MAVERICK_INSTALL_CONSOLE:-0}"

is_interactive_tty() {
  [[ -t 0 && -t 1 ]]
}

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

This installer will NOT perform an OS upgrade or modify base runtime libraries automatically.
The decision is always left to the operator.

Choose one path:
  1) Build from source on this host if you must stay on the current distro.
  2) Advanced only: re-run with --skip-runtime-check (install may still fail at runtime).
  3) If desired, plan an OS upgrade to a supported baseline (Raspberry Pi OS Lite Bookworm or Debian 12 minimal).
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
        python3)
          printf '%s\n' python3-minimal
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
        python3)
          printf '%s\n' python3
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

ui_has_gum() {
  command -v gum >/dev/null 2>&1
}

ui_has_whiptail() {
  command -v whiptail >/dev/null 2>&1
}

ui_has_dialog() {
  command -v dialog >/dev/null 2>&1
}

validate_safe_path() {
  local p="$1"
  if [[ -z "${p}" || "${p}" == *$'\n'* || "${p}" == *$'\r'* ]]; then
    echo "invalid path (empty or contains newline characters)" >&2
    return 1
  fi
  return 0
}

prompt_string() {
  local label="$1"
  local default="$2"
  if [[ "${NON_INTERACTIVE}" -eq 1 ]]; then
    printf '%s\n' "${default}"
    return
  fi
  if [[ "${ASSUME_YES}" -eq 1 ]]; then
    printf '%s\n' "${default}"
    return
  fi
  local out=""
  if ui_has_gum; then
    out="$(gum input --prompt "${label} " --value "${default}" --placeholder "${default}" 2>/dev/null || true)"
  elif ui_has_whiptail; then
    out="$(whiptail --inputbox "${label}" 12 70 "${default}" 3>&1 1>&2 2>&3 || true)"
  elif ui_has_dialog; then
    out="$(dialog --stdout --inputbox "${label}" 12 70 "${default}" 2>&1 || true)"
  else
    read -r -p "${label} [${default}]: " out || true
    out="${out:-$default}"
  fi
  if [[ -z "${out}" ]]; then
    out="${default}"
  fi
  printf '%s\n' "${out}"
}

prompt_confirm() {
  local text="$1"
  local default_yes="${2:-1}"
  if [[ "${NON_INTERACTIVE}" -eq 1 ]]; then
    [[ "${default_yes}" -eq 1 ]]
    return
  fi
  if [[ "${ASSUME_YES}" -eq 1 ]]; then
    return 0
  fi
  if ui_has_gum; then
    if [[ "${default_yes}" -eq 1 ]]; then
      gum confirm --default=true "${text}"
    else
      gum confirm --default=false "${text}"
    fi
    return
  fi
  local default_hint="N"
  [[ "${default_yes}" -eq 1 ]] && default_hint="Y"
  local answer=""
  read -r -p "${text} [${default_hint} y/N]: " answer || true
  answer="${answer:-}"
  if [[ -z "${answer}" ]]; then
    [[ "${default_yes}" -eq 1 ]]
    return
  fi
  case "${answer}" in
    [Yy]|[Yy][Ee][Ss]) return 0 ;;
    *) return 1 ;;
  esac
}

prompt_extensions_menu() {
  # Sets global: SEL_CONSOLE SEL_HTTP SEL_MQTT (0/1 for console; http/mqtt are always coming_soon)
  SEL_CONSOLE=1
  SEL_HTTP=0
  SEL_MQTT=0
  if [[ "${NON_INTERACTIVE}" -eq 1 ]]; then
    SEL_CONSOLE="${DEFAULT_INSTALL_CONSOLE}"
    return
  fi
  if [[ "${ASSUME_YES}" -eq 1 ]]; then
    return
  fi
  echo ""
  echo "Step 4/4 — Extensions"
  echo "  [x] Maverick console (terminal UX) — available"
  echo "  [ ] HTTP bridge — coming soon"
  echo "  [ ] MQTT bridge — coming soon"
  echo ""
  if prompt_confirm "Enable Maverick console (recommended)" 1; then
    SEL_CONSOLE=1
  else
    SEL_CONSOLE=0
  fi
}

ensure_maverick_etc() {
  if [[ ! -d "${MAVERICK_ETC}" ]]; then
    run_privileged install -d -m 0755 "${MAVERICK_ETC}"
  fi
}

# Data dir is often created with sudo while the operator runs maverick-edge as $SUDO_USER (e.g. pi).
# Without chown, SQLite cannot create maverick.db under /var/lib/maverick.
chown_data_dir_for_operator() {
  local data_dir="$1"
  if [[ -z "${SUDO_USER:-}" ]]; then
    return 0
  fi
  if ! id "${SUDO_USER}" >/dev/null 2>&1; then
    return 0
  fi
  run_privileged chown -R "${SUDO_USER}:${SUDO_USER}" "${data_dir}"
}

write_runtime_env() {
  local data_dir="$1"
  local gwmp="$2"
  local rt_ms="$3"
  local max_msg="$4"
  local tmp
  tmp="$(mktemp)"
  {
    echo "# Generated by install-linux.sh ${INSTALLER_SCRIPT_VERSION}"
    echo "MAVERICK_DATA_DIR=${data_dir}"
    echo "MAVERICK_GWMP_BIND=${gwmp}"
    echo "MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS=${rt_ms}"
    echo "MAVERICK_GWMP_LOOP_MAX_MESSAGES=${max_msg}"
  } >"${tmp}"
  run_privileged install -m 0644 "${tmp}" "${RUNTIME_ENV_PATH}"
  rm -f "${tmp}"
}

write_setup_json_file() {
  local data_dir="$1"
  local gwmp="$2"
  local rt_ms="$3"
  local max_msg="$4"
  local console_state="$5"
  local completed_iso="$6"
  ensure_cmd python3
  local tmp
  tmp="$(mktemp)"
  MAVERICK_DATA_DIR_JSON="${data_dir}" \
    MAVERICK_GWMP_JSON="${gwmp}" \
    MAVERICK_RT_JSON="${rt_ms}" \
    MAVERICK_MAX_JSON="${max_msg}" \
    MAVERICK_CONSOLE_JSON="${console_state}" \
    MAVERICK_COMPLETED_JSON="${completed_iso}" \
    INSTALLER_VER_JSON="${INSTALLER_SCRIPT_VERSION}" \
    OUT_JSON="${tmp}" \
    python3 <<'PY'
import json, os

path = os.environ["OUT_JSON"]
completed = os.environ["MAVERICK_COMPLETED_JSON"]
data = {
  "schema_version": 1,
  "completed_at": completed if completed else None,
  "installer_version": os.environ["INSTALLER_VER_JSON"],
  "selected_extensions": {
    "console": os.environ["MAVERICK_CONSOLE_JSON"],
    "http": "coming_soon",
    "mqtt": "coming_soon",
  },
  "runtime": {
    "data_dir": os.environ["MAVERICK_DATA_DIR_JSON"],
    "gwmp_bind": os.environ["MAVERICK_GWMP_JSON"],
    "loop_read_timeout_ms": int(os.environ["MAVERICK_RT_JSON"]),
    "loop_max_messages": int(os.environ["MAVERICK_MAX_JSON"]),
  },
}
with open(path, "w", encoding="utf-8") as f:
    json.dump(data, f, indent=2)
    f.write("\n")
PY
  run_privileged install -m 0644 "${tmp}" "${SETUP_JSON_PATH}"
  rm -f "${tmp}"
}

write_user_console_config() {
  local data_dir="$1"
  local gwmp="$2"
  local rt_ms="$3"
  local max_msg="$4"
  local enable_console="$5"
  local home_dir
  home_dir="$(getent passwd "${SUDO_USER:-$USER}" 2>/dev/null | cut -d: -f6 || true)"
  if [[ -z "${home_dir}" ]]; then
    home_dir="${HOME:-}"
  fi
  if [[ -z "${home_dir}" || "${home_dir}" == "/" ]]; then
    echo "warning: could not resolve home directory; skipping ~/.config/maverick sync" >&2
    return
  fi
  local cfg_dir="${home_dir}/.config/maverick"
  local cfg_json="${cfg_dir}/tui-config.json"
  mkdir -p "${cfg_dir}" 2>/dev/null || run_privileged mkdir -p "${cfg_dir}"
  local tmp
  tmp="$(mktemp)"
  CFG_DATA_DIR="${data_dir}" CFG_GWMP="${gwmp}" CFG_RT="${rt_ms}" CFG_MAX="${max_msg}" \
    CFG_ENABLE_CONSOLE="${enable_console}" \
    python3 <<'PY' >"${tmp}"
import json, os

ext = ["console"] if os.environ.get("CFG_ENABLE_CONSOLE", "0") == "1" else []

cfg = {
  "data_dir": os.environ["CFG_DATA_DIR"],
  "gwmp_bind": os.environ["CFG_GWMP"],
  "loop_read_timeout_ms": int(os.environ["CFG_RT"]),
  "loop_max_messages": int(os.environ["CFG_MAX"]),
  "enabled_extensions": ext,
}
print(json.dumps(cfg, indent=2))
PY
  if [[ -w "${cfg_dir}" ]] 2>/dev/null; then
    install -m 0644 "${tmp}" "${cfg_json}"
  else
    run_privileged install -m 0644 "${tmp}" "${cfg_json}"
    if [[ -n "${SUDO_USER:-}" ]]; then
      run_privileged chown "${SUDO_USER}:${SUDO_USER}" "${cfg_json}" 2>/dev/null || true
    fi
  fi
  rm -f "${tmp}"

  local console_toml="${cfg_dir}/console.toml"
  if [[ ! -f "${console_toml}" ]]; then
    tmp="$(mktemp)"
    {
      echo 'schema_version = 1'
      echo 'theme = "auto"'
    } >"${tmp}"
    if [[ -w "${cfg_dir}" ]] 2>/dev/null; then
      install -m 0644 "${tmp}" "${console_toml}"
    else
      run_privileged install -m 0644 "${tmp}" "${console_toml}"
      if [[ -n "${SUDO_USER:-}" ]]; then
        run_privileged chown "${SUDO_USER}:${SUDO_USER}" "${console_toml}" 2>/dev/null || true
      fi
    fi
    rm -f "${tmp}"
  fi
}

install_console_symlink() {
  if [[ ! -f "${INSTALL_DIR}/maverick-edge-tui" ]]; then
    return
  fi
  if [[ -L "${INSTALL_DIR}/maverick" || ! -e "${INSTALL_DIR}/maverick" ]]; then
    run_privileged ln -sf maverick-edge-tui "${INSTALL_DIR}/maverick"
  fi
}

LNS_CONFIG_PATH="${MAVERICK_ETC}/lns-config.toml"

ensure_lns_declarative_config() {
  local edge="$1"
  local data_dir="$2"
  ensure_maverick_etc
  if [[ ! -f "${LNS_CONFIG_PATH}" ]]; then
    echo "Creating starter LNS declarative config: ${LNS_CONFIG_PATH}"
    if ! run_privileged env MAVERICK_DATA_DIR="${data_dir}" "${edge}" config init --config-path "${LNS_CONFIG_PATH}"; then
      echo "warning: maverick-edge config init failed (older binary?)" >&2
      return 0
    fi
  fi
  echo "Syncing LNS config into SQLite (MAVERICK_DATA_DIR=${data_dir})"
  if ! run_privileged env MAVERICK_DATA_DIR="${data_dir}" "${edge}" config load --config-path "${LNS_CONFIG_PATH}"; then
    echo "warning: maverick-edge config load failed" >&2
  fi
}

install_maverick_systemd_service() {
  local data_dir="$1"
  local gwmp="$2"
  local rt_ms="$3"
  local max_msg="$4"
  if ! command -v systemctl >/dev/null 2>&1; then
    echo "systemctl not found; skipping systemd unit for maverick-edge."
    return 0
  fi
  local unit_path="/etc/systemd/system/maverick-edge.service"
  local tmp
  tmp="$(mktemp)"
  {
    echo "[Unit]"
    echo "Description=Maverick Edge Runtime (GWMP ingest-loop)"
    echo "After=network.target"
    echo ""
    echo "[Service]"
    echo "Type=simple"
    printf 'Environment="MAVERICK_DATA_DIR=%s"\n' "${data_dir}"
    printf 'Environment="MAVERICK_GWMP_BIND=%s"\n' "${gwmp}"
    printf 'Environment="MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS=%s"\n' "${rt_ms}"
    printf 'Environment="MAVERICK_GWMP_LOOP_MAX_MESSAGES=%s"\n' "${max_msg}"
    echo "ExecStartPre=${INSTALL_DIR}/maverick-reset-spi.sh start"
    echo "ExecStart=${INSTALL_DIR}/maverick-edge radio ingest-loop"
    echo "Restart=on-failure"
    echo "RestartSec=5"
    echo ""
    echo "[Install]"
    echo "WantedBy=multi-user.target"
  } >"${tmp}"
  echo "Installing systemd unit: ${unit_path}"
  run_privileged install -m 0644 "${tmp}" "${unit_path}"
  rm -f "${tmp}"
  run_privileged systemctl daemon-reload
  run_privileged systemctl enable maverick-edge.service
  run_privileged systemctl restart maverick-edge.service || run_privileged systemctl start maverick-edge.service || true
}

export_edge_env() {
  export MAVERICK_DATA_DIR="$1"
  export MAVERICK_GWMP_BIND="$2"
  export MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS="$3"
  export MAVERICK_GWMP_LOOP_MAX_MESSAGES="$4"
}

run_smoke_checks() {
  local edge="$1"
  echo ""
  echo "Step 3/4 — Smoke checks"
  set +e
  "${edge}" --help >/dev/null 2>&1
  local h=$?
  MAVERICK_DATA_DIR="${MAVERICK_DATA_DIR}" MAVERICK_GWMP_BIND="${MAVERICK_GWMP_BIND}" \
    MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS="${MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS}" \
    MAVERICK_GWMP_LOOP_MAX_MESSAGES="${MAVERICK_GWMP_LOOP_MAX_MESSAGES}" \
    "${edge}" status >/dev/null 2>&1
  local s=$?
  MAVERICK_DATA_DIR="${MAVERICK_DATA_DIR}" MAVERICK_GWMP_BIND="${MAVERICK_GWMP_BIND}" \
    MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS="${MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS}" \
    MAVERICK_GWMP_LOOP_MAX_MESSAGES="${MAVERICK_GWMP_LOOP_MAX_MESSAGES}" \
    "${edge}" health >/dev/null 2>&1
  local he=$?
  set -euo pipefail
  if [[ "${h}" -ne 0 ]]; then
    echo "  [FAIL] maverick-edge --help"
    return 1
  fi
  echo "  [OK] maverick-edge --help"
  if [[ "${s}" -ne 0 ]]; then
    echo "  [WARN] maverick-edge status (non-fatal for first install)"
  else
    echo "  [OK] maverick-edge status"
  fi
  if [[ "${he}" -ne 0 ]]; then
    echo "  [WARN] maverick-edge health (non-fatal for first install)"
  else
    echo "  [OK] maverick-edge health"
  fi
  return 0
}

run_onboarding_wizard() {
  local edge="${INSTALL_DIR}/maverick-edge"
  local data_dir gwmp rt_ms max_msg
  data_dir="${DEFAULT_DATA_DIR}"
  gwmp="${DEFAULT_GWMP_BIND}"
  rt_ms="${DEFAULT_LOOP_READ_TIMEOUT_MS}"
  max_msg="${DEFAULT_LOOP_MAX_MESSAGES}"

  if [[ "${SKIP_ONBOARDING}" -eq 1 ]]; then
    echo "Skipping onboarding (--skip-onboarding)."
    return
  fi

  if [[ "${NON_INTERACTIVE}" -eq 1 ]]; then
    echo "Non-interactive onboarding: applying defaults / environment."
    data_dir="${MAVERICK_DATA_DIR:-${DEFAULT_DATA_DIR}}"
    gwmp="${MAVERICK_GWMP_BIND:-${DEFAULT_GWMP_BIND}}"
    rt_ms="${MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS:-${DEFAULT_LOOP_READ_TIMEOUT_MS}}"
    max_msg="${MAVERICK_GWMP_LOOP_MAX_MESSAGES:-${DEFAULT_LOOP_MAX_MESSAGES}}"
    local console_st="skipped"
    if [[ "${DEFAULT_INSTALL_CONSOLE}" -eq 1 ]]; then
      console_st="enabled"
    fi
    ensure_maverick_etc
    validate_safe_path "${data_dir}" || exit 1
    run_privileged install -d -m 0755 "${data_dir}" 2>/dev/null || true
    chown_data_dir_for_operator "${data_dir}"
    write_runtime_env "${data_dir}" "${gwmp}" "${rt_ms}" "${max_msg}"
    local completed
    completed="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    write_setup_json_file "${data_dir}" "${gwmp}" "${rt_ms}" "${max_msg}" "${console_st}" "${completed}"
    write_user_console_config "${data_dir}" "${gwmp}" "${rt_ms}" "${max_msg}" "${DEFAULT_INSTALL_CONSOLE}"
    ensure_lns_declarative_config "${edge}" "${data_dir}"
    install_maverick_systemd_service "${data_dir}" "${gwmp}" "${rt_ms}" "${max_msg}"
    install_console_symlink
    export_edge_env "${data_dir}" "${gwmp}" "${rt_ms}" "${max_msg}"
    return
  fi

  if ! is_interactive_tty; then
    echo "No interactive TTY; running non-interactive onboarding with defaults."
    local save_ni="${NON_INTERACTIVE}"
    NON_INTERACTIVE=1
    run_onboarding_wizard
    NON_INTERACTIVE="${save_ni}"
    return
  fi

  ensure_cmd python3

  echo ""
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo " Maverick first-run setup                                    "
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

  echo ""
  echo "Step 1/4 — Data directory"
  data_dir="$(prompt_string "Local data directory for Maverick" "${data_dir}")"
  validate_safe_path "${data_dir}" || exit 1
  run_privileged install -d -m 0755 "${data_dir}"
  chown_data_dir_for_operator "${data_dir}"

  echo ""
  echo "Step 2/4 — Gateway (GWMP) bind and loop policy"
  gwmp="$(prompt_string "UDP bind address (host:port)" "${gwmp}")"
  rt_ms="$(prompt_string "Loop read timeout (ms)" "${rt_ms}")"
  max_msg="$(prompt_string "Max messages per loop iteration" "${max_msg}")"

  export_edge_env "${data_dir}" "${gwmp}" "${rt_ms}" "${max_msg}"
  run_smoke_checks "${edge}" || true

  prompt_extensions_menu
  local console_state="disabled"
  [[ "${SEL_CONSOLE:-0}" -eq 1 ]] && console_state="enabled"

  ensure_maverick_etc
  write_runtime_env "${data_dir}" "${gwmp}" "${rt_ms}" "${max_msg}"
  local completed
  completed="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  write_setup_json_file "${data_dir}" "${gwmp}" "${rt_ms}" "${max_msg}" "${console_state}" "${completed}"
  write_user_console_config "${data_dir}" "${gwmp}" "${rt_ms}" "${max_msg}" "${SEL_CONSOLE:-0}"
  install_console_symlink
  ensure_lns_declarative_config "${edge}" "${data_dir}"
  install_maverick_systemd_service "${data_dir}" "${gwmp}" "${rt_ms}" "${max_msg}"

  echo ""
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo " Setup complete"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  Config: ${RUNTIME_ENV_PATH}"
  echo "  State:  ${SETUP_JSON_PATH}"
  echo ""
  echo "Next commands:"
  echo "  MAVERICK_DATA_DIR=\"${data_dir}\" ${edge} status"
  echo "  MAVERICK_DATA_DIR=\"${data_dir}\" ${edge} health"
  if [[ "${SEL_CONSOLE:-0}" -eq 1 ]] && command -v maverick >/dev/null 2>&1; then
    echo "  maverick"
  elif [[ "${SEL_CONSOLE:-0}" -eq 1 ]]; then
    echo "  maverick-edge-tui   (or symlink: maverick)"
  fi
  echo ""
}

usage() {
  cat <<EOF
install-linux.sh - install maverick-edge (+ optional Maverick console) on Linux

Usage:
  $0 [--version <tag|latest>] [--install-dir <path>] [installer flags]

Installer / onboarding flags:
  --interactive            Interactive onboarding (default when stdin is a TTY)
  --non-interactive        No prompts; use environment defaults (see below)
  --yes                    Accept recommended defaults in interactive mode
  --skip-onboarding        Only download and install binaries
  --local-dist-dir <path>  Install maverick-edge (+ tui if present) from a local directory (no GitHub)
  --no-install-deps        Do not auto-install missing base packages (curl, etc.)
  --skip-runtime-check     Skip glibc baseline check (advanced)

Non-interactive environment (optional overrides):
  MAVERICK_DATA_DIR (default /var/lib/maverick)
  MAVERICK_GWMP_BIND (default 0.0.0.0:17000)
  MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS (default 1000)
  MAVERICK_GWMP_LOOP_MAX_MESSAGES (default 0 = unlimited ingest-loop for systemd)
  MAVERICK_INSTALL_CONSOLE  (default 0) set to 1 to enable console in setup.json

One-liner:
  curl -fsSL "https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/scripts/install-linux.sh" | bash -s -- --version latest --install-dir /usr/local/bin

Examples:
  $0 --version latest
  $0 --version v0.1.0 --install-dir /usr/local/bin --non-interactive
  $0 --version v0.1.0 --yes
  $0 --local-dist-dir ./dist-preview --non-interactive --install-dir /usr/local/bin
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
    --interactive)
      NON_INTERACTIVE=0
      shift
      ;;
    --non-interactive)
      NON_INTERACTIVE=1
      shift
      ;;
    --yes)
      ASSUME_YES=1
      shift
      ;;
    --skip-onboarding)
      SKIP_ONBOARDING=1
      shift
      ;;
    --local-dist-dir)
      LOCAL_DIST_DIR="${2:-}"
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

if [[ -n "${LOCAL_DIST_DIR}" ]]; then
  case "${LOCAL_DIST_DIR}" in
    "~"|~/*)
      LOCAL_DIST_DIR="${LOCAL_DIST_DIR/#\~/${HOME}}"
      ;;
  esac
  if [[ ! -d "${LOCAL_DIST_DIR}" ]]; then
    echo "error: --local-dist-dir is not a directory: ${LOCAL_DIST_DIR}" >&2
    exit 1
  fi
  if [[ ! -f "${LOCAL_DIST_DIR}/maverick-edge" ]]; then
    echo "error: missing maverick-edge in ${LOCAL_DIST_DIR}" >&2
    exit 1
  fi
fi

if [[ -z "${LOCAL_DIST_DIR}" && "${VERSION}" == "latest" ]]; then
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

if [[ -z "${LOCAL_DIST_DIR}" ]]; then
  if [[ -z "${VERSION}" ]]; then
    echo "failed to resolve release version (empty tag_name)" >&2
    exit 1
  fi
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

if [[ -n "${LOCAL_DIST_DIR}" ]]; then
  VERSION="local"
  echo "Installing maverick-edge from local dist ${LOCAL_DIST_DIR} (${TARGET})..."
  cp -a "${LOCAL_DIST_DIR}/maverick-edge" "${TMP_DIR}/maverick-edge"
  if [[ -f "${LOCAL_DIST_DIR}/maverick-edge-tui" ]]; then
    cp -a "${LOCAL_DIST_DIR}/maverick-edge-tui" "${TMP_DIR}/maverick-edge-tui"
  fi
  chmod +x "${TMP_DIR}/maverick-edge"
else
  ASSET="maverick-${TARGET}.tar.gz"
  SHA_FILE="${ASSET}.sha256"
  BASE_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${VERSION}"

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
fi

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

if [[ -f "${TMP_DIR}/maverick-reset-spi.sh" ]]; then
  install_one "${TMP_DIR}/maverick-reset-spi.sh" "maverick-reset-spi.sh"
  echo "Installed: ${INSTALL_DIR}/maverick-reset-spi.sh"
fi

if [[ -f "${TMP_DIR}/maverick-edge-tui" ]]; then
  chmod +x "${TMP_DIR}/maverick-edge-tui"
  install_one "${TMP_DIR}/maverick-edge-tui" "maverick-edge-tui"
  validate_binary "${INSTALL_DIR}/maverick-edge-tui" "maverick-edge-tui"
  install_console_symlink
  echo "Installed: ${INSTALL_DIR}/maverick-edge, ${INSTALL_DIR}/maverick-edge-tui (public name: maverick)"
else
  echo "Installed: ${INSTALL_DIR}/maverick-edge (no maverick-edge-tui in this tarball)"
fi

if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
  echo "Note: ${INSTALL_DIR} is not currently in PATH. Add it to your shell profile before using maverick-edge directly."
fi

run_onboarding_wizard

echo ""
echo "Smoke checks:"
echo "  maverick-edge --help"
echo "  maverick-edge status"
