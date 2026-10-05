//! On-disk cache: cover images only.
//!
//! Live data (home / detail / search) is API-backed and cached in memory for the
//! lifetime of the app (see `AppData`); only images are persisted, since they are
//! large and effectively immutable.
//!
//! Directory layout (see paths.rs):
//! ```text
//! <cache_dir>/
//! └── images/<resource-key-sha256>  # cover images, cached permanently, 512MB total cap (LRU)
//! ```

use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use sha2::{Digest, Sha256};

/// Soft cap on total image-cache size (bytes)
const IMAGE_CACHE_LIMIT: u64 = 512 * 1024 * 1024;
/// Target ratio to prune down to once the cap is exceeded (80%)
const IMAGE_CACHE_TARGET_RATIO: f64 = 0.8;

fn ensure_dir(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
}

/// URL → cache filename (SHA-256 hex)
fn url_hash(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Image resource cache key: independent of the source host.
///
/// The same path on mikanani.me and the alternate domain mikanime.tv is the same
/// resource, so switching source domains should not trigger a re-download or
/// create duplicate cache entries. The key is derived from "path + query string"
/// (the query string changes what the server returns, e.g. size cropping, so it is kept).
fn image_key(url: &str) -> String {
    let path = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .and_then(|rest| rest.split_once('/'))
        .map(|(_, path)| format!("/{path}"))
        .unwrap_or_else(|| url.to_string());
    url_hash(&path)
}

// ── Image cache (permanent) ─────────────────────

/// Image cache file path; returns None when not cached or the file is damaged (empty)
pub fn cached_image(url: &str) -> Option<PathBuf> {
    let path = image_path(url);
    let ok = std::fs::metadata(&path).is_ok_and(|m| m.len() > 0);
    if !ok {
        return None;
    }
    // Touch mtime on read so LRU evicts by most-recently-used
    let _ = filetime_touch(&path);
    Some(path)
}

/// Update the file mtime (preserves LRU semantics on read hits); failures are
/// silently ignored
fn filetime_touch(path: &Path) -> std::io::Result<()> {
    let now = filetime::FileTime::now();
    filetime::set_file_mtime(path, now)
}

fn image_path(url: &str) -> PathBuf {
    crate::paths::app_cache_dir()
        .join("images")
        .join(image_key(url))
}

/// Save an image to the cache (atomic: temp file + rename, preventing a partial
/// file from being hit)
pub fn store_image(url: &str, bytes: &[u8]) -> Option<PathBuf> {
    let path = image_path(url);
    if let Some(dir) = path.parent() {
        ensure_dir(dir);
    }
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, bytes).is_ok() && std::fs::rename(&tmp, &path).is_ok() {
        Some(path)
    } else {
        let _ = std::fs::remove_file(&tmp);
        None
    }
}

/// Image-cache size management: when the total exceeds the soft cap, delete the
/// oldest by modification time until usage falls back to 80% of the cap (an LRU
/// approximation: images are only written once on first save, so mtime ≈ first
/// access). Called once at startup (background thread).
pub fn enforce_image_cache_limit() {
    let images = crate::paths::app_cache_dir().join("images");
    let Ok(entries) = std::fs::read_dir(&images) else {
        return;
    };

    // (mtime, path, size)
    let mut files: Vec<(SystemTime, PathBuf, u64)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            Some((
                meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                e.path(),
                meta.len(),
            ))
        })
        .collect();
    let total: u64 = files.iter().map(|(_, _, s)| *s).sum();
    if total <= IMAGE_CACHE_LIMIT {
        return;
    }

    let target = (IMAGE_CACHE_LIMIT as f64 * IMAGE_CACHE_TARGET_RATIO) as u64;
    // Oldest first
    files.sort_by_key(|(mtime, _, _)| *mtime);
    let mut freed = 0u64;
    for (_, path, size) in files {
        if total - freed <= target {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            freed += size;
        }
    }
    eprintln!(
        "图片缓存清理: 释放 {} MB(上限 {} MB)",
        freed / 1024 / 1024,
        IMAGE_CACHE_LIMIT / 1024 / 1024
    );
}

/// Clear the entire cache (triggerable from the settings page)
pub fn clear_all() {
    let _ = std::fs::remove_dir_all(crate::paths::app_cache_dir());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_stable_and_distinct() {
        let a = url_hash("https://x/1.jpg");
        let b = url_hash("https://x/1.jpg");
        let c = url_hash("https://x/2.jpg");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn image_key_is_host_independent() {
        // The same path shares one cache key across source domains
        assert_eq!(
            image_key("https://mikanani.me/images/a.jpg?width=400&height=560"),
            image_key("https://mikanime.tv/images/a.jpg?width=400&height=560"),
        );
        assert_eq!(
            image_key("https://mikanani.me/images/a.jpg"),
            image_key("https://mikanime.tv/images/a.jpg"),
        );
        // Query strings change the returned content, so they must be distinguished (size cropping)
        assert_ne!(
            image_key("https://mikanani.me/images/a.jpg?width=400&height=400"),
            image_key("https://mikanani.me/images/a.jpg?width=400&height=560"),
        );
        // Different paths are distinguished
        assert_ne!(
            image_key("https://mikanani.me/images/a.jpg"),
            image_key("https://mikanani.me/images/b.jpg"),
        );
        // Relative / host-less URLs are hashed as-is
        assert_eq!(image_key("/images/a.jpg"), image_key("/images/a.jpg"),);
    }

    #[test]
    fn image_cache_roundtrip() {
        let url = "https://example.com/cache-test.jpg";
        let _ = std::fs::remove_file(image_path(url));
        assert!(cached_image(url).is_none());
        let path = store_image(url, b"fake-image-bytes").expect("写入成功");
        assert_eq!(path, image_path(url));
        assert!(cached_image(url).is_some());
        let _ = std::fs::remove_file(path);
    }
}
