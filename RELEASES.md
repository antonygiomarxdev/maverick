# Maverick v1.0.2 Release Notes

## What's New in v1.0.2

### SPI Support in Release Binaries
**Release binaries now include SPI support** for ARM targets (aarch64, armv7).

The CI now cross-compiles `libloragw` (sx1302_hal) for ARM targets, so pre-built binaries work directly on SPI hardware devices (RAK Pi, etc.).

### Auto-Update Now Works for SPI Devices
With SPI support in release binaries, the auto-update mechanism now works for all supported architectures:
- `x86_64` - UDP only (native build)
- `aarch64` - SPI + UDP (cross-compiled with libloragw)
- `armv7` - SPI + UDP (cross-compiled with libloragw)

### Installation
```bash
# Download latest release for your architecture
curl -LO https://github.com/antonygiomarxdev/maverick/releases/latest/download/maverick-ARCH.tar.gz
tar -xzf maverick-ARCH.tar.gz
sudo ./install-linux.sh
```

### Architecture Support

| Architecture | Use Case | SPI Support |
|-------------|----------|-------------|
| `x86_64` | Development, UDP testing | ❌ |
| `aarch64` | RAK Pi 4, modern ARM gateways | ✅ |
| `armv7` | RAK Pi 3, older ARM gateways | ✅ |

## v1.0.1 → v1.0.2 Changes

- ✅ CI now builds ARM targets with `--features spi`
- ✅ `build.rs` properly handles cross-compilation sysroot
- ✅ Auto-update works for SPI hardware devices

## Known Limitations

### Class A Downlink TX
- Downlink transmission (SPI TX) deferred to v1.1
- RX1/RX2 receive window handling requires additional work

### SQLCipher Encryption
- Session key encryption deferred to v1.1
- Keys stored as plaintext BLOB

## Security

See [SECURITY.md](SECURITY.md) for vulnerability reporting guidelines.

## Support

- GitHub Issues: https://github.com/antonygiomarxdev/maverick/issues
- Discussions: https://github.com/antonygiomarxdev/maverick/discussions
