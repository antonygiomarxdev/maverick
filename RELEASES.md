# Maverick v1.0.1 Release Notes

## What's New

### Auto-Update Mechanism
- New `maverick-update.sh` script for automatic binary updates
- Systemd timer (`maverick-update.timer`) checks for updates hourly
- Support for both release (GitHub Downloads) and dev (git pull) modes
- Atomic update with rollback backup capability

### Configuration
- `maverick.toml` configuration file with update settings
- Update check interval configurable (default: 1 hour)

## Known Limitations

### SPI/Gateway Hardware Support
**Pre-built release binaries do NOT include SPI support** (libloragw).

The release binaries (`maverick-x86_64.tar.gz`, etc.) are compiled **without** the `spi` feature because:
1. libloragw (sx1302_hal) is not vendored in the repository
2. Cross-compiling C libraries for ARM requires additional setup

**Workaround for SPI hardware (RAK Pi, etc.):**
```bash
# Clone and build with SPI
git clone https://github.com/antonygiomarxdev/maverick.git
cd maverick
git checkout v1.0.1
cargo build --release --features spi -p maverick-runtime-edge
sudo cp target/release/maverick-edge /usr/local/bin/
```

SPI support will be added to release binaries in a future version.

### What's Included in Release Binaries
- `maverick-edge` - Main LNS binary (UDP backend)
- `maverick-edge-tui` - TUI extension
- `install-linux.sh` - Installation script
- `version.txt` - Version tag

### What's NOT Included
- SPI concentrator support (requires local build with `--features spi`)
- Class A Downlink TX (deferred to v1.1)
- SQLCipher encryption (deferred to v1.1)

## Installation

```bash
# Download latest release for your architecture
curl -LO https://github.com/antonygiomarxdev/maverick/releases/latest/download/maverick-ARCH.tar.gz
tar -xzf maverick-ARCH.tar.gz
sudo ./install-linux.sh
```

## Upgrading from Previous Versions

```bash
# Automatic (if update timer is enabled)
sudo systemctl start maverick-update.service

# Manual
sudo systemctl stop maverick-edge
# Replace binary
sudo systemctl start maverick-edge
```

## Security

See [SECURITY.md](SECURITY.md) for vulnerability reporting guidelines.

## Support

- GitHub Issues: https://github.com/antonygiomarxdev/maverick/issues
- Discussions: https://github.com/antonygiomarxdev/maverick/discussions
