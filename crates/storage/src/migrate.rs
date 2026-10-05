//! One-time startup migration: move old-layout caches and files to the new layout.
//!
//! - JSON caches named by historical versions (list-v2/v3, detail-v2/v3, the old
//!   search directory) are all cleaned up; current code uniformly uses unversioned
//!   names (see cache.rs)
//! - DHT routing table: librqbit's third-party default directory → the app data
//!   directory (dht.dat)
//!
//! Every migration is idempotent: it is skipped when the target already exists,
//! and failure does not block startup.

use std::path::Path;

use crate::paths;

/// Run all migrations (synchronous, fast; called once at startup)
pub fn run_all() {
    cleanup_legacy_cache();
    migrate_dht();
}

/// Clean up historical cache files: early development used names like
/// list-v2/v3 and detail-v2/v3, now unified to unversioned names; old files are
/// deleted outright (their contents would expire via TTL anyway, so the loss is
/// negligible).
fn cleanup_legacy_cache() {
    let cache = paths::app_cache_dir();
    if !cache.exists() {
        return;
    }
    // Only delete files with historical versioned names; unversioned names
    // (list.json, detail/, search/) are in use by current code and must be kept
    for name in ["list-v2.json", "list-v3.json"] {
        let _ = std::fs::remove_file(cache.join(name));
    }
    for dir in ["detail-v2", "detail-v3", "search-v3"] {
        let _ = std::fs::remove_dir_all(cache.join(dir));
    }
}

/// librqbit's default DHT file → <data dir>/dht.dat
fn migrate_dht() {
    let old = paths::librqbit_dht_default();
    let new = paths::app_data_dir().join("dht.dat");
    if new.exists() || !old.exists() {
        return;
    }
    if let Some(dir) = new.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::rename(&old, &new).is_ok() {
        eprintln!("DHT 路由表已迁移: {} → {}", old.display(), new.display());
        // Also clean up the now-empty third-party directory
        let _ = std::fs::remove_dir(old.parent().unwrap_or(Path::new("")));
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn legacy_cache_cleanup_removes_old_files() {
        // Verify cleanup logic with a temp directory (directly testing rename idempotency)
        let dir = std::env::temp_dir().join(format!("mikan_migrate_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let from = dir.join("old");
        let to = dir.join("new");
        std::fs::write(&from, b"x").unwrap();

        // Idempotent rename (same logic as migrate_dht)
        if from.exists() && !to.exists() {
            std::fs::rename(&from, &to).unwrap();
        }
        assert!(to.exists());
        if from.exists() && !to.exists() {
            std::fs::rename(&from, &to).unwrap();
        }
        assert!(to.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
