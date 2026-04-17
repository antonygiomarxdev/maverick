use std::os::unix::net::UnixDatagram;

fn cleanup_notify_socket() {
    std::env::remove_var("NOTIFY_SOCKET");
}

#[test]
#[ignore = "requires systemd socket activation - flaky in CI"]
fn watchdog_ping_succeeds_when_socket_set() {
    let original = std::env::var("NOTIFY_SOCKET").ok();
    cleanup_notify_socket();

    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("test_watchdog.sock");

    let socket = UnixDatagram::bind(&socket_path).unwrap();
    std::env::set_var("NOTIFY_SOCKET", socket_path.to_str().unwrap());
    let socket_clone = socket.try_clone().unwrap();
    drop(socket);

    std::thread::spawn(move || {
        let mut buf = [0u8; 256];
        socket_clone.recv_from(&mut buf).ok();
    });

    std::thread::sleep(std::time::Duration::from_millis(10));
    let result = maverick_runtime_edge::watchdog::send_watchdog_ping();
    assert!(
        result.is_ok(),
        "watchdog ping should succeed when socket is set"
    );

    cleanup_notify_socket();
    if let Some(v) = original {
        std::env::set_var("NOTIFY_SOCKET", v);
    }
}

#[test]
#[ignore = "requires systemd socket activation - flaky in CI"]
fn ready_signal_succeeds_when_socket_set() {
    let original = std::env::var("NOTIFY_SOCKET").ok();
    cleanup_notify_socket();

    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("test_ready.sock");

    let socket = UnixDatagram::bind(&socket_path).unwrap();
    std::env::set_var("NOTIFY_SOCKET", socket_path.to_str().unwrap());
    let socket_clone = socket.try_clone().unwrap();
    drop(socket);

    std::thread::spawn(move || {
        let mut buf = [0u8; 256];
        socket_clone.recv_from(&mut buf).ok();
    });

    std::thread::sleep(std::time::Duration::from_millis(10));
    let result = maverick_runtime_edge::watchdog::send_ready();
    assert!(
        result.is_ok(),
        "ready signal should succeed when socket is set"
    );

    cleanup_notify_socket();
    if let Some(v) = original {
        std::env::set_var("NOTIFY_SOCKET", v);
    }
}

#[test]
#[ignore = "requires systemd socket activation - flaky in CI"]
fn stopping_signal_succeeds_when_socket_set() {
    let original = std::env::var("NOTIFY_SOCKET").ok();
    cleanup_notify_socket();

    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("test_stopping.sock");

    let socket = UnixDatagram::bind(&socket_path).unwrap();
    std::env::set_var("NOTIFY_SOCKET", socket_path.to_str().unwrap());
    let socket_clone = socket.try_clone().unwrap();
    drop(socket);

    std::thread::spawn(move || {
        let mut buf = [0u8; 256];
        socket_clone.recv_from(&mut buf).ok();
    });

    std::thread::sleep(std::time::Duration::from_millis(10));
    let result = maverick_runtime_edge::watchdog::send_stopping();
    assert!(
        result.is_ok(),
        "stopping signal should succeed when socket is set"
    );

    cleanup_notify_socket();
    if let Some(v) = original {
        std::env::set_var("NOTIFY_SOCKET", v);
    }
}

#[test]
fn watchdog_fails_gracefully_when_socket_not_set() {
    let original = std::env::var("NOTIFY_SOCKET").ok();
    cleanup_notify_socket();
    let result = maverick_runtime_edge::watchdog::send_watchdog_ping();
    assert!(result.is_err(), "expected error when NOTIFY_SOCKET not set");
    if let Some(v) = original {
        std::env::set_var("NOTIFY_SOCKET", v);
    }
}

#[test]
fn ready_fails_gracefully_when_socket_not_set() {
    let original = std::env::var("NOTIFY_SOCKET").ok();
    cleanup_notify_socket();
    let result = maverick_runtime_edge::watchdog::send_ready();
    assert!(result.is_err(), "expected error when NOTIFY_SOCKET not set");
    if let Some(v) = original {
        std::env::set_var("NOTIFY_SOCKET", v);
    }
}

#[test]
fn stopping_fails_gracefully_when_socket_not_set() {
    let original = std::env::var("NOTIFY_SOCKET").ok();
    cleanup_notify_socket();
    let result = maverick_runtime_edge::watchdog::send_stopping();
    assert!(result.is_err(), "expected error when NOTIFY_SOCKET not set");
    if let Some(v) = original {
        std::env::set_var("NOTIFY_SOCKET", v);
    }
}
