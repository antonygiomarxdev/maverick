---
phase: 11
plan: A
subsystem: update
wave: 1
depends_on: []
type: execute
files_modified:
  - crates/maverick-runtime-edge/src/main.rs (update CLI subcommands)
  - crates/maverick-runtime-edge/src/update.rs (new file)
  - etc/maverick-update.service (new file)
  - etc/maverick-update.timer (new file)
  - bin/maverick-update.sh (new file, shell script)
autonomous: true
requirements: []
---

<objective>
Create the update script (bin/maverick-update.sh) and systemd service/timer files for the auto-update mechanism. This establishes the foundation for atomic updates via systemd timer + shell script.

**Out of scope:** Version checking logic, download logic (Plan B), CLI integration (Plan C)
</objective>

<read_first>
- .planning/phases/11-auto-update-mechanism-for-arm-gateways/11-CONTEXT.md (update mechanism decisions)
- .planning/phases/04-process-supervision/04-CONTEXT.md (systemd service patterns)
</read_first>

<action>
Create `/usr/local/bin/maverick-update.sh` — Bash update script with:

```
#!/bin/bash
# maverick-update.sh — Atomic update for maverick-edge
set -euo pipefail

LOG="systemd-cat -t maverick-update"
BINARY_PATH="/usr/local/bin/maverick-edge"
BACKUP_DIR="/var/lib/maverick/backups"
DOWNLOAD_DIR="/var/lib/maverick/downloads"
CONFIG_DIR="/etc/maverick"

$LOG "Starting update check"

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

$LOG "Current version: $CURRENT_VERSION, mode: $UPDATE_MODE"

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
    $LOG "New version available: $NEW_VERSION"
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
    $LOG "Binary updated to $NEW_VERSION"
  else
    $LOG "No new version available"
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
      $LOG "Dev build updated"
    fi
  fi
fi

# Start maverick-edge service
systemctl start maverick-edge.service
$LOG "Update complete"
```

Create `/etc/systemd/system/maverick-update.service`:
```
[Unit]
Description=Maverick Update Service
After=network.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/maverick-update.sh
StandardOutput=journal
StandardError=journal
```

Create `/etc/systemd/system/maverick-update.timer`:
```
[Unit]
Description=Maverick Update Timer
Requires=maverick-update.service

[Timer]
OnBootSec=5min
OnUnitActiveSec=1h
AccuracySec=1min

[Install]
WantedBy=timers.target
```

Create `/etc/maverick/maverick.toml` update section default:
```
[update]
mode = "release"           # "release" or "dev"
release_url = ""            # e.g., "https://github.com/yourorg/maverick/releases"
check_interval = 3600       # seconds between update checks
download_dir = "/var/lib/maverick/downloads"
backup_dir = "/var/lib/maverick/backups"
insecure = false            # allow HTTP (not recommended)
```

Create directory structure:
```
mkdir -p /var/lib/maverick/downloads
mkdir -p /var/lib/maverick/backups
mkdir -p /etc/maverick
```

Install:
```
chmod +x /usr/local/bin/maverick-update.sh
systemctl daemon-reload
systemctl enable maverick-update.timer
systemctl start maverick-update.timer
```
</action>

<acceptance_criteria>
- `/usr/local/bin/maverick-update.sh` exists and is executable
- `/etc/systemd/system/maverick-update.service` exists
- `/etc/systemd/system/maverick-update.timer` exists and is enabled
- Timer fires at least once after boot (OnBootSec=5min)
- Service is Type=oneshot (exits after completion)
- Update script logs to journald via `systemd-cat -t maverick-update`
</acceptance_criteria>

<verification>
1. Check files exist:
   - `test -x /usr/local/bin/maverick-update.sh && echo "script exists"`
   - `test -f /etc/systemd/system/maverick-update.service && echo "service exists"`
   - `test -f /etc/systemd/system/maverick-update.timer && echo "timer exists"`
2. Check timer is enabled: `systemctl is-enabled maverick-update.timer`
3. Check timer is active: `systemctl is-active maverick-update.timer`
4. View recent updates: `journalctl -u maverick-update -n 20 --no-pager`
5. Manual trigger: `systemctl start maverick-update.service && journalctl -u maverick-update -n 5 --no-pager`
</verification>

<success_criteria>
- maverick-update.timer is enabled and active
- Update script can be run manually without errors
- Service exits cleanly after update check (Type=oneshot)
- Logs appear in journald under `maverick-update` identifier
</success_criteria>

---
*Plan: 11-A*
*Created: 2026-04-17*