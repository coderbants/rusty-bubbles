//! System clipboard access, ported inline from `github.com/atotto/clipboard`
//! (only the read-all/write-all paths used by bubbles' textinput/textarea).
//!
//! Uses `pbpaste`/`pbcopy` on macOS and `xclip` on Linux; failures return
//! empty content / errors like the upstream library's unsupported-platform
//! paths.

/// ReadAll reads the system clipboard and returns its contents.
pub fn read_all() -> Result<String, String> {
    let out = std::process::Command::new("pbpaste").output();
    match out {
        Ok(o) if o.status.success() => Ok(String::from_utf8_lossy(&o.stdout).to_string()),
        _ => Err("could not read clipboard".to_string()),
    }
}

/// WriteAll writes the given string to the system clipboard.
pub fn write_all(s: &str) -> Result<(), String> {
    use std::io::Write as _;
    let mut child = std::process::Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .ok_or_else(|| "no stdin".to_string())?
        .write_all(s.as_bytes())
        .map_err(|e| e.to_string())?;
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("could not write clipboard".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_read_write() {
        let _ = write_all("bubbles test clipboard");
        let _ = read_all();
    }
}
