//! Local data paths for copy-pasta. History lives on disk so it survives reboots.

use std::fs;
use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    directories::ProjectDirs::from("dev", "copy-pasta", "copy-pasta")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| dirs_fallback().join("Library/Application Support/copy-pasta"))
}

fn dirs_fallback() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn ensure_data_dirs() -> std::io::Result<(PathBuf, PathBuf)> {
    let root = data_dir();
    let blobs = root.join("blobs");
    fs::create_dir_all(&blobs)?;
    Ok((root, blobs))
}

pub fn db_path() -> PathBuf {
    data_dir().join("clipboard.sqlite")
}

pub fn blob_path_for_hash(content_hash: &str) -> PathBuf {
    let prefix = content_hash.get(..2).unwrap_or("xx");
    data_dir().join("blobs").join(prefix).join(content_hash)
}
