use std::path::{Path, PathBuf};

const FOLDER_ART_NAMES: &[&str] = &[
    "cover.jpg",
    "cover.jpeg",
    "cover.png",
    "folder.jpg",
    "folder.jpeg",
    "folder.png",
    "front.jpg",
    "front.png",
];

/// Writes embedded art bytes into the artwork cache; returns the cached path.
pub fn cache_embedded(
    cache_dir: &Path,
    album_id: i64,
    bytes: &[u8],
    ext: &str,
) -> std::io::Result<PathBuf> {
    let dir = cache_dir.join("artwork");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("album_{album_id}.{ext}"));
    std::fs::write(&path, bytes)?;
    Ok(path)
}

/// Looks for conventional artwork files next to the audio file.
pub fn find_folder_art(track_path: &Path) -> Option<PathBuf> {
    let dir = track_path.parent()?;
    FOLDER_ART_NAMES.iter().find_map(|name| {
        let candidate = dir.join(name);
        candidate.is_file().then_some(candidate)
    })
}
