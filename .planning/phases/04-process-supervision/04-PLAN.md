---
phase: 4
name: Process Supervision
wave: 1
depends_on: []
autonomous: true
requirements_addressed:
  - RELI-03
  - RELI-04
  - SEC-02
files_modified:
  - deploy/systemd/maverick-edge.service
  - crates/maverick-runtime-edge/src/main.rs
  - crates/maverick-runtime-edge/src/watchdog.rs
  - crates/maverick-adapter-persistence-sqlite/src/lib.rs
  - crates/maverick-adapter-persistence-sqlite/src/schema.rs
---

## Plan: Process Supervision Implementation

### Objective

Implement self-healing process supervision via systemd with watchdog support, and encrypt SQLite session keys so they cannot be read as plaintext by unprivileged users.

### Success Criteria

1. **RELI-03**: After SIGKILL, maverick-edge automatically restarted by systemd within 2 seconds
2. **RELI-04**: Hung process detected by systemd WatchdogSec and restarted
3. **SEC-02**: NwkSKey and AppSKey not readable as plaintext from SQLite schema

### Tasks

#### Task 1: Create systemd Unit File

<read_first>
- deploy/systemd/ (check if directory exists)
</read_first>

<action>
Create `deploy/systemd/maverick-edge.service`:

```ini
[Unit]
Description=Maverick Edge LoRaWAN LNS
After=network.target
Wants=network.target

[Service]
Type=notify
ExecStart=/usr/bin/maverick-edge run
Restart=always
RestartSec=2s
WatchdogSec=30s
User=maverick
Group=maverick
Environment=RUST_LOG=info
EnvironmentFile=-/etc/maverick/edge.env

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/maverick /etc/maverick
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

Create `deploy/systemd/` directory structure and install target:
```bash
mkdir -p deploy/systemd
```

The Type=notify requires our process to send sd_notify ready信号.
</action>

<acceptance_criteria>
- systemd unit file exists at `deploy/systemd/maverick-edge.service`
- Contains `Restart=always`, `RestartSec=2s`, `WatchdogSec=30s`
- Contains security hardening options (ProtectSystem, NoNewPrivileges)
- User/Group set to maverick:maverick
</acceptance_criteria>

---

#### Task 2: Implement Watchdog via sd_notify

<read_first>
- crates/maverick-runtime-edge/src/main.rs
</read_first>

<action>
Create `crates/maverick-runtime-edge/src/watchdog.rs`:

```rust
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::time::Duration;

/// Send watchdog ping via sd_notify protocol.
/// Must be called at interval < WatchdogSec/2 to keep systemd happy.
pub fn send_watchdog_ping() -> Result<(), std::io::Error> {
    let pid = std::process::id();
    let watchdog_us = 15_000_000u64; // 15 seconds in microseconds

    let notify_socket = std::env::var("NOTIFY_SOCKET")?;

    let msg: OsString = format!("WATCHDOG=1\nPID={}\nMONOTONIC_USEC={}\n", pid, watchdog_us).into();
    let bytes: Vec<u8> = msg.into_bytes();

    std::os::unix::net::UnixDatagram::unconnected()?
        .send_to(bytes.as_slice(), &notify_socket)?;

    Ok(())
}

/// Signal systemd that startup is complete (READY=1).
pub fn send_ready() -> Result<(), std::io::Error> {
    let notify_socket = std::env::var("NOTIFY_SOCKET")?;
    let msg: OsString = "READY=1\nSTATUS=Running\n".into();
    let bytes: Vec<u8> = msg.into_bytes();

    std::os::unix::net::UnixDatagram::unconnected()?
        .send_to(bytes.as_slice(), &notify_socket)?;

    Ok(())
}

/// Signal systemd that we're stopping gracefully (STOPPING=1).
pub fn send_stopping() -> Result<(), std::io::Error> {
    let notify_socket = std::env::var("NOTIFY_SOCKET")?;
    let msg: OsString = "STOPPING=1\n".into();
    let bytes: Vec<u8> = msg.into_bytes();

    std::os::unix::net::UnixDatagram::unconnected()?
        .send_to(bytes.as_slice(), &notify_socket)?;

    Ok(())
}
```

Integrate into main.rs:
- On startup: call `send_ready()`
- In ingest loop: spawn watchdog task that pings every 15 seconds
- On shutdown: call `send_stopping()`
</action>

<acceptance_criteria>
- `watchdog.rs` module exists with `send_watchdog_ping()`, `send_ready()`, `send_stopping()`
- main.rs calls `send_ready()` on startup
- Watchdog ping task runs every 15 seconds
- `send_stopping()` called on graceful shutdown
</acceptance_criteria>

---

#### Task 3: SQLite Encryption for Session Keys (SEC-02)

<read_first>
- crates/maverick-adapter-persistence-sqlite/src/lib.rs
- crates/maverick-adapter-persistence-sqlite/src/schema.rs
</read_first>

<action>
The SEC-02 requirement states: "NwkSKey and AppSKey stored in SQLite with SQLite-level encryption or access controls (not plaintext in schema)"

Options:
1. **SQLCipher** (recommended): Use `rusqlite` with `bundled` feature plus SQLCipher compile-time option
2. **SQLite user authentication**: pragma user_authentication
3. **Application-level encryption**: Encrypt/decrypt in Rust code before storage

Given embedded Linux constraints, implement SQLCipher via rusqlite_bundled:

1. Update `Cargo.toml` for maverick-adapter-persistence-sqlite:
```toml
[dependencies]
rusqlite = { version = "0.33", features = ["bundled"], default-features = false }
```

2. Key derivation:
- Derive encryption key from device-specific secret (stored in /etc/maverick/secret)
- Use PBKDF2 or argon2 to derive SQLCipher key
- Pass key via `PRAGMA key` before opening connection

3. Add to SqlitePersistence initialization:
```rust
impl SqlitePersistence {
    pub fn with_key(&self, key: &str) -> AppResult<()> {
        let this = self.clone();
        this.run_blocking(move |p| {
            p.run_with_busy_retry(|conn| {
                // SQLCipher key derivation (requires sqlcipher build)
                let sql = format!("PRAGMA key = \"x'{}'\";", hex::encode(key));
                conn.execute_batch(&sql)?;
                Ok(())
            })
        }).await
    }
}
```

Note: SEC-02 requires that keys are NOT plaintext in schema. With SQLCipher, the keys are encrypted at rest. Even if someone copies the DB file, they cannot read the keys without the encryption key.
</action>

<acceptance_criteria>
- rusqlite configured with bundled SQLite
- SQLCipher key derivation implemented
- PRAGMA key called on new connections
- NwkSKey and AppSKey stored as encrypted BLOB (not readable as plaintext)
- Encryption key stored in /etc/maverick/secret (permissions 0600)
</acceptance_criteria>

---

#### Task 4: Integration Test for Watchdog

<read_first>
- crates/maverick-integration-tests/tests/
</read_first>

<action>
Create `crates/maverick-integration-tests/tests/watchdog.rs`:

```rust
#[test]
fn test_watchdog_ping_succeeds_when_socket_set() {
    std::env::set_var("NOTIFY_SOCKET", "/run/test.sock");
    // Create mock socket
    let _ = std::fs::create_dir_all("/run");
    let socket = std::os::unix::net::UnixDatagram::bind("/run/test.sock").unwrap();

    let handle = std::thread::spawn(move || {
        let mut buf = [0u8; 256];
        let (size, _) = socket.recv_from(&mut buf).unwrap();
        let msg = String::from_utf8_lossy(&buf[..size]);
        assert!(msg.contains("WATCHDOG=1"));
    });

    send_watchdog_ping().unwrap();
    handle.join().unwrap();
}

#[test]
fn test_ready_signal() {
    std::env::set_var("NOTIFY_SOCKET", "/run/test2.sock");
    let socket = std::os::unix::net::UnixDatagram::bind("/run/test2.sock").unwrap();

    let handle = std::thread::spawn(move || {
        let mut buf = [0u8; 256];
        let (size, _) = socket.recv_from(&mut buf).unwrap();
        let msg = String::from_utf8_lossy(&buf[..size]);
        assert!(msg.contains("READY=1"));
    });

    send_ready().unwrap();
    handle.join().unwrap();
}
```
</action>

<acceptance_criteria>
- cargo test -p maverick-integration-tests -- watchdog passes
- Integration tests verify sd_notify protocol works correctly
</acceptance_criteria>

---

### Verification

Run:
```bash
cargo test -p maverick-integration-tests -- watchdog
cargo test -p maverick-runtime-edge
```

Manual verification:
1. Install systemd unit: `sudo cp deploy/systemd/maverick-edge.service /etc/systemd/system/`
2. Start service: `sudo systemctl start maverick-edge`
3. Kill with SIGKILL: `sudo kill -9 $(pidof maverick-edge)`
4. Verify restart: `sudo systemctl status maverick-edge` (should show Running)
5. Verify keys encrypted: `sqlite3 /var/lib/maverick/maverick.db "SELECT nwk_s_key FROM sessions LIMIT 1"` (should not show plaintext)
