#!/usr/bin/env bash
# Gate-style checks for a real RAK Pi (or similar Linux gateway) before pushing ingest/runtime changes.
#
# Usage:
#   ./scripts/e2e-rakpi-prepush.sh
#   RAKPI_SSH=pi@rak.local ./scripts/e2e-rakpi-prepush.sh
#
# Evidence: set WRITE_EVIDENCE=1 (default) to save ./dist/e2e-evidence/rakpi-e2e-<utc>.txt
set -euo pipefail

WRITE_EVIDENCE="${WRITE_EVIDENCE:-1}"
EVIDENCE_DIR="${EVIDENCE_DIR:-dist/e2e-evidence}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
EVIDENCE_FILE="${EVIDENCE_DIR}/rakpi-e2e-${STAMP}.txt"
RAKPI_SSH="${RAKPI_SSH:-}"

run_sh() {
  local script="$1"
  if [[ -n "${RAKPI_SSH}" ]]; then
    ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new "${RAKPI_SSH}" "${script}"
  else
    bash -c "${script}"
  fi
}

write_block() {
  local title="$1"
  local script="$2"
  if [[ "${WRITE_EVIDENCE}" == "1" ]]; then
    {
      echo "=== ${title} ==="
      run_sh "${script}" || true
      echo ""
    } | tee -a "${EVIDENCE_FILE}"
  else
    echo "=== ${title} ==="
    run_sh "${script}" || true
    echo ""
  fi
}

mkdir -p "${EVIDENCE_DIR}"
if [[ "${WRITE_EVIDENCE}" == "1" ]]; then
  {
    echo "Maverick RAK Pi E2E pre-push evidence"
    echo "timestamp_utc: ${STAMP}"
    echo "RAKPI_SSH: ${RAKPI_SSH:-local}"
    echo ""
  } >"${EVIDENCE_FILE}"
fi

write_block "maverick-edge version" 'command -v maverick-edge && maverick-edge --version || echo "maverick-edge missing"'
write_block "probe" 'maverick-edge probe 2>/dev/null | head -c 12000'
write_block "status" 'maverick-edge status 2>/dev/null | head -c 12000'
write_block "health" 'maverick-edge health 2>/dev/null | head -c 12000'
write_block "config validate" '(sudo -n maverick-edge config validate 2>/dev/null) || (maverick-edge config validate 2>/dev/null) || true'
write_block "systemd maverick-edge" 'systemctl is-active maverick-edge 2>/dev/null || echo "no unit or systemctl unavailable"'

echo "Done. For RF proof, run ingest-loop and confirm ingested > 0; see docs/compatibility-matrix.md."
if [[ "${WRITE_EVIDENCE}" == "1" ]]; then
  echo "Evidence: ${EVIDENCE_FILE}"
fi
