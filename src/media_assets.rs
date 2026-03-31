use std::fs;
use std::path::{Path, PathBuf};

pub fn media_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("sat-stream").join("media")
}

pub fn ensure_media_dir() -> std::io::Result<PathBuf> {
    let dir = media_dir();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn save_png(name: &str, bytes: &[u8]) -> Result<String, String> {
    let dir = ensure_media_dir().map_err(|e| format!("Failed creating media dir: {}", e))?;
    let path = dir.join(name);
    fs::write(&path, bytes).map_err(|e| format!("Failed writing media file: {}", e))?;
    Ok(path.to_string_lossy().to_string())
}

pub fn file_exists(path: &str) -> bool {
    Path::new(path).exists()
}
