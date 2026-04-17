use std::ffi::OsString;

pub fn send_watchdog_ping() -> Result<(), std::io::Error> {
    let pid = std::process::id();
    let watchdog_us = 15_000_000u64;

    let notify_socket = std::env::var("NOTIFY_SOCKET")
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e))?;

    let msg: OsString = format!("WATCHDOG=1\nPID={}\nMONOTONIC_USEC={}\n", pid, watchdog_us).into();
    let bytes: Vec<u8> = msg.into_encoded_bytes();

    std::os::unix::net::UnixDatagram::unbound()?.send_to(bytes.as_slice(), &notify_socket)?;

    Ok(())
}

pub fn send_ready() -> Result<(), std::io::Error> {
    let notify_socket = std::env::var("NOTIFY_SOCKET")
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e))?;
    let msg: OsString = "READY=1\nSTATUS=Running\n".into();
    let bytes: Vec<u8> = msg.into_encoded_bytes();

    std::os::unix::net::UnixDatagram::unbound()?.send_to(bytes.as_slice(), &notify_socket)?;

    Ok(())
}

pub fn send_stopping() -> Result<(), std::io::Error> {
    let notify_socket = std::env::var("NOTIFY_SOCKET")
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e))?;
    let msg: OsString = "STOPPING=1\n".into();
    let bytes: Vec<u8> = msg.into_encoded_bytes();

    std::os::unix::net::UnixDatagram::unbound()?.send_to(bytes.as_slice(), &notify_socket)?;

    Ok(())
}
