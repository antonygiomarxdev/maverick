//! Download logic for release updates

use super::UpdateError;

/// Download a file from URL to destination path
pub fn download_file(url: &str, dest: &std::path::Path, insecure: bool) -> Result<(), UpdateError> {
    let mut cmd = std::process::Command::new("curl");

    if insecure {
        cmd.args(["-s", "-L", "-o", dest.to_str().unwrap()]);
    } else {
        cmd.args([
            "-s",
            "-L",
            "-f",
            "--cacert",
            "/etc/ssl/certs/ca-certificates.crt",
            "-o",
            dest.to_str().unwrap(),
        ]);
    }
    cmd.arg(url);

    let output = cmd
        .output()
        .map_err(|e| UpdateError::Download(format!("curl failed: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(UpdateError::Download(format!(
            "Download failed ({}): {}",
            output.status.code().unwrap_or(-1),
            stderr
        )));
    }

    let metadata = std::fs::metadata(dest)
        .map_err(|e| UpdateError::Download(format!("dest metadata failed: {}", e)))?;

    if metadata.len() == 0 {
        return Err(UpdateError::Download(
            "Downloaded file is empty".to_string(),
        ));
    }

    Ok(())
}
