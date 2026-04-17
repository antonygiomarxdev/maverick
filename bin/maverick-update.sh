#!/bin/bash
# maverick-update.sh — Atomic update for maverick-edge
set -euo pipefail

LOG="systemd-cat -t maverick-update"
BINARY_PATH="/usr/local/bin/maverick-edge"
BACKUP_DIR="/var/lib/maverick/backups"
DOWNLOAD_DIR="/var/lib/maverick/downloads"
CONFIG_DIR="/etc/maverick"

log() { echo "$@" | $LOG; }
log "Starting update check"

# Load config
if [ -f "$CONFIG_DIR/maverick.toml" ]; then
  UPDATE_MODE=$(grep -A1 '^\[update\]' "$CONFIG_DIR/maverick.toml" 2>/dev/null | grep 'mode' | cut -d'"' -f2 || echo "release")
  RELEASE_URL=$(grep 'release_url' "$CONFIG_DIR/maverick.toml" | cut -d'"' -f2 || echo "")
  CHECK_INTERVAL=$(grep 'check_interval' "$CONFIG_DIR/maverick.toml" | awk -F'= ' '{print $2}' || echo "3600")
else
  UPDATE_MODE="release"
  RELEASE_URL=""
  CHECK_INTERVAL="3600"
fi

# Get current version
CURRENT_VERSION=$(/usr/local/bin/maverick-edge --version 2>/dev/null | awk '{print $2}' || echo "unknown")

log "Current version: $CURRENT_VERSION, mode: $UPDATE_MODE"

# Stop maverick-edge service
systemctl stop maverick-edge.service

# For release mode: download and replace binary
if [ "$UPDATE_MODE" = "release" ] && [ -n "$RELEASE_URL" ]; then
  ARCH=$(uname -m)
  VERSION_URL="$RELEASE_URL/$ARCH/version.txt"
  BINARY_URL="$RELEASE_URL/$ARCH/maverick-edge-$CURRENT_VERSION"

  # Check version
  NEW_VERSION=$(curl -sf "$VERSION_URL" || echo "")
  if [ -n "$NEW_VERSION" ] && [ "$NEW_VERSION" != "$CURRENT_VERSION" ]; then
    log "New version available: $NEW_VERSION"
    # Download new binary
    mkdir -p "$DOWNLOAD_DIR"
    curl -sf "$RELEASE_URL/$ARCH/maverick-edge-$NEW_VERSION" -o "$DOWNLOAD_DIR/maverick-edge-$NEW_VERSION"
    # Backup current
    mkdir -p "$BACKUP_DIR"
    cp "$BINARY_PATH" "$BACKUP_DIR/maverick-edge-$CURRENT_VERSION-$(date +%s)"
    # Atomic replace
    mv "$DOWNLOAD_DIR/maverick-edge-$NEW_VERSION" "$BINARY_PATH.new"
    mv "$BINARY_PATH.new" "$BINARY_PATH"
    chmod 755 "$BINARY_PATH"
    log "Binary updated to $NEW_VERSION"
  else
    log "No new version available"
  fi
fi

# For dev mode: git pull + build
if [ "$UPDATE_MODE" = "dev" ]; then
  if command -v git >/dev/null && command -v cargo >/dev/null; then
    cd /opt/maverick 2>/dev/null || cd /root/maverick 2>/dev/null || true
    if [ -d .git ]; then
      git pull
      cargo build --release --manifest-path Cargo.toml
      mkdir -p "$BACKUP_DIR"
      cp "$BINARY_PATH" "$BACKUP_DIR/maverick-edge-dev-$(date +%s)"
      cp target/release/maverick-edge "$BINARY_PATH"
      chmod 755 "$BINARY_PATH"
      log "Dev build updated"
    fi
  fi
fi

# Start maverick-edge service
systemctl start maverick-edge.service
log "Update complete"
