//! Version checking for update mechanism

use super::UpdateError;

/// Fetch remote version from version.txt URL
pub fn fetch_remote_version(version_url: &str, insecure: bool) -> Result<String, UpdateError> {
    let output = if insecure {
        std::process::Command::new("curl")
            .args(["-s", "-L", version_url])
            .output()
    } else {
        std::process::Command::new("curl")
            .args([
                "-s",
                "-L",
                "-f",
                "--cacert",
                "/etc/ssl/certs/ca-certificates.crt",
                version_url,
            ])
            .output()
    }
    .map_err(|e| UpdateError::Command(format!("curl failed: {}", e)))?;

    if !output.status.success() {
        return Err(UpdateError::Version(format!(
            "curl exited with {}",
            output.status.code().unwrap_or(-1)
        )));
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if version.is_empty() || !version.matches('.').count() >= 1 {
        return Err(UpdateError::Version(format!(
            "Invalid version string: {}",
            version
        )));
    }

    Ok(version)
}

/// Compare two version strings
/// Returns: negative if a < b, 0 if equal, positive if a > b
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |v: &str| -> Vec<u64> {
        v.split('-')
            .next()
            .unwrap_or(v)
            .split('.')
            .filter_map(|p| p.parse().ok())
            .collect()
    };

    let a_parts = parse(a);
    let b_parts = parse(b);

    for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
        match a_part.cmp(b_part) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    a_parts.len().cmp(&b_parts.len())
}
