use std::path::Path;
use std::process::Command;

/// Whether we can display inline media in this terminal session.
///
/// Current implementation detects Kitty protocol capability:
/// - running inside Kitty (`TERM=xterm-kitty`), or
/// - Kitty integration vars exported (`KITTY_WINDOW_ID` / `KITTY_INSTALLATION_DIR`)
pub fn kitty_media_supported() -> bool {
    let term = std::env::var("TERM").unwrap_or_default();
    if term == "xterm-kitty" {
        return true;
    }

    std::env::var("KITTY_WINDOW_ID").is_ok() || std::env::var("KITTY_INSTALLATION_DIR").is_ok()
}

/// Render an image path inline using Kitty's `icat` kitten.
///
/// This is best-effort: failures are returned as strings so caller can show a status message.
pub fn show_image_in_kitty(path: &str) -> Result<(), String> {
    if !kitty_media_supported() {
        return Err("Kitty inline media is not supported in this terminal session".to_string());
    }

    if !Path::new(path).exists() {
        return Err(format!("Media file not found: {}", path));
    }

    let status = Command::new("kitty")
        .arg("+kitten")
        .arg("icat")
        .arg("--stdin=no")
        .arg("--transfer-mode=file")
        .arg(path)
        .status()
        .map_err(|e| format!("Failed to launch kitty icat: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("kitty icat exited with status: {}", status))
    }
}
