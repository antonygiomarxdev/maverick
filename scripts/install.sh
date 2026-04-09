#!/usr/bin/env sh
# Maverick LNS — one-line installer
#
# Usage:
#   curl -sSf https://raw.githubusercontent.com/antonygiomarxdev/maverick/main/scripts/install.sh | sh
#
# Variables you can override:
#   MAVERICK_VERSION  — specific version tag, e.g. "v0.1.0" (default: latest)
#   MAVERICK_INSTALL_DIR — binary destination (default: /usr/local/bin)
#   MAVERICK_DATA_DIR — data directory (default: /var/lib/maverick)
#   MAVERICK_DB_PATH  — database file path (default: $MAVERICK_DATA_DIR/maverick.db)
#
# Supported platforms:
#   linux/amd64   (VPS, x86 gateways)
#   linux/arm64   (Raspberry Pi 4+, modern gateways)
#   linux/armv7   (Raspberry Pi 3, older gateways)

set -eu

REPO="antonygiomarxdev/maverick"
INSTALL_DIR="${MAVERICK_INSTALL_DIR:-/usr/local/bin}"
DATA_DIR="${MAVERICK_DATA_DIR:-/var/lib/maverick}"
DB_PATH="${MAVERICK_DB_PATH:-$DATA_DIR/maverick.db}"
SERVICE_USER="maverick"
SERVICE_FILE="/etc/systemd/system/maverick.service"

# ── Helpers ──────────────────────────────────────────────────────────────────

info()    { printf '\033[0;34m[maverick]\033[0m %s\n' "$*"; }
success() { printf '\033[0;32m[maverick]\033[0m %s\n' "$*"; }
error()   { printf '\033[0;31m[maverick] error:\033[0m %s\n' "$*" >&2; exit 1; }

need_cmd() { command -v "$1" >/dev/null 2>&1 || error "required command not found: $1"; }
need_root() { [ "$(id -u)" -eq 0 ] || error "this installer must be run as root (try: sudo sh)"; }

# ── Detect architecture ───────────────────────────────────────────────────────

detect_arch() {
    arch="$(uname -m)"
    case "$arch" in
        x86_64)           echo "x86_64-unknown-linux-gnu" ;;
        aarch64|arm64)    echo "aarch64-unknown-linux-gnu" ;;
        armv7l|armv7)     echo "armv7-unknown-linux-gnueabihf" ;;
        *)                error "unsupported architecture: $arch" ;;
    esac
}

# ── Resolve version ───────────────────────────────────────────────────────────

resolve_version() {
    if [ -n "${MAVERICK_VERSION:-}" ]; then
        echo "$MAVERICK_VERSION"
        return
    fi
    need_cmd curl
    version=$(curl -sSf "https://api.github.com/repos/$REPO/releases/latest" \
        | grep '"tag_name"' \
        | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
    [ -n "$version" ] || error "could not resolve latest version from GitHub"
    echo "$version"
}

# ── Download ──────────────────────────────────────────────────────────────────

download_binary() {
    target="$1"
    version="$2"

    url="https://github.com/$REPO/releases/download/$version/maverick-${target}.tar.gz"
    tmp="$(mktemp -d)"

    info "downloading maverick $version for $target ..."
    curl -sSfL "$url" -o "$tmp/maverick.tar.gz" \
        || error "download failed: $url"

    tar -xzf "$tmp/maverick.tar.gz" -C "$tmp"
    mv "$tmp/maverick" "$INSTALL_DIR/maverick"
    chmod +x "$INSTALL_DIR/maverick"
    rm -rf "$tmp"

    success "installed binary → $INSTALL_DIR/maverick"
}

# ── System user ───────────────────────────────────────────────────────────────

create_user() {
    if ! id "$SERVICE_USER" >/dev/null 2>&1; then
        info "creating system user: $SERVICE_USER"
        useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER"
    fi
}

# ── Data directory ────────────────────────────────────────────────────────────

setup_data_dir() {
    mkdir -p "$DATA_DIR"
    chown "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"
    chmod 750 "$DATA_DIR"
    info "data directory: $DATA_DIR"
}

# ── systemd service ───────────────────────────────────────────────────────────

install_service() {
    cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=Maverick LoRaWAN Network Server
Documentation=https://github.com/$REPO
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
ExecStart=$INSTALL_DIR/maverick
Restart=on-failure
RestartSec=5s

# Directories
WorkingDirectory=$DATA_DIR
StateDirectory=maverick
LogsDirectory=maverick

# Environment
Environment="MAVERICK_DB_PATH=$DB_PATH"
Environment="MAVERICK_HTTP_BIND_ADDR=0.0.0.0:8080"
Environment="MAVERICK_UDP_BIND_ADDR=0.0.0.0:1700"
Environment="MAVERICK_LOG_FILTER=maverick_core=info"
Environment="MAVERICK_STORAGE_PROFILE=auto"

# Hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=$DATA_DIR
AmbientCapabilities=CAP_NET_BIND_SERVICE

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable maverick
    success "systemd service installed: $SERVICE_FILE"
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
    info "Maverick LNS installer"

    need_root
    need_cmd curl
    need_cmd tar
    need_cmd useradd
    need_cmd systemctl

    target="$(detect_arch)"
    version="$(resolve_version)"

    info "platform : $target"
    info "version  : $version"

    download_binary "$target" "$version"
    create_user
    setup_data_dir
    install_service

    success ""
    success "Maverick $version installed successfully!"
    success ""
    success "  Start now:      systemctl start maverick"
    success "  Enable on boot: systemctl enable maverick"
    success "  View logs:      journalctl -u maverick -f"
    success "  HTTP API:       http://localhost:8080/api/v1/health"
    success "  UDP ingester:   0.0.0.0:1700"
    success ""
    success "  Config via env in: $SERVICE_FILE"
}

main "$@"
