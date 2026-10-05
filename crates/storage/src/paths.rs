//! Cross-platform application directories.
//!
//! Follows each platform's conventions:
//! - macOS: `~/Library/Application Support/<App>` (data), `~/Library/Caches/<App>` (cache)
//! - Linux: `$XDG_DATA_HOME/<app>` / `$XDG_CACHE_HOME/<app>` (defaults `~/.local/share` / `~/.cache`)
//! - Windows: `%APPDATA%\<App>` (data), `%LOCALAPPDATA%\<App>` (cache)

use std::path::{Path, PathBuf};

/// User data directory (subscriptions, settings — must not be lost)
pub fn app_data_dir() -> PathBuf {
    let name = "MikanPlus";
    #[cfg(target_os = "macos")]
    {
        home()
            .join("Library")
            .join("Application Support")
            .join(name)
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".local").join("share"))
            .join(name.to_lowercase())
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(home)
            .join(name)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        home().join(format!(".{name}"))
    }
}

/// Cache directory (images, list/detail JSON — re-fetchable, may be cleared by the system)
pub fn app_cache_dir() -> PathBuf {
    let name = "MikanPlus";
    #[cfg(target_os = "macos")]
    {
        home().join("Library").join("Caches").join(name)
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".cache"))
            .join(name.to_lowercase())
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(home)
            .join(name)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        home().join(format!(".cache/{name}"))
    }
}

/// Default download directory (`~/Videos` on all three platforms; Linux honors
/// xdg-user-dirs redirection).
pub fn video_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        // Read XDG_VIDEOS_DIR from ~/.config/user-dirs.dirs; fall back to ~/Videos on failure
        let cfg = home().join(".config").join("user-dirs.dirs");
        if let Ok(text) = std::fs::read_to_string(cfg) {
            for line in text.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("XDG_VIDEOS_DIR=") {
                    let v = rest.trim().trim_matches('"');
                    if let Some(p) = v.strip_prefix("$HOME/") {
                        return home().join(p);
                    }
                    if v.starts_with('/') {
                        return PathBuf::from(v);
                    }
                    break;
                }
            }
        }
        home().join("Videos")
    }
    #[cfg(not(target_os = "linux"))]
    {
        home().join("Videos")
    }
}

/// librqbit's default DHT persistence path (third-party default, to be migrated
/// into our data directory). Corresponds to the `directories` crate's
/// cache_dir + "com.rqbit.dht/dht.json".
pub fn librqbit_dht_default() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home()
            .join("Library")
            .join("Caches")
            .join("com.rqbit.dht")
            .join("dht.json")
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".cache"))
            .join("com.rqbit.dht")
            .join("dht.json")
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(home)
            .join("com.rqbit.dht")
            .join("dht.json")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        home().join(".cache").join("com.rqbit.dht").join("dht.json")
    }
}

/// Open a file or directory with the system default application.
pub fn open_path(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(path);
        c
    };
    #[cfg(target_os = "linux")]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("explorer");
        c.arg(path);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path);
        c
    };
    cmd.spawn().map(|_| ())
}

/// Open an HTTP(S) URL in the system default browser.
pub fn open_url(url: &str) -> std::io::Result<()> {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    if !lower.starts_with("https://") && !lower.starts_with("http://") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "仅支持 HTTP(S) URL",
        ));
    }
    open_path(std::path::Path::new(url))
}

/// Sanitize a file/directory name (cross-platform): replace the Windows-illegal
/// characters `\ / : * ? " < > |`, collapse whitespace, and trim leading/trailing
/// spaces and dots; an empty result falls back to the literal `"未命名"`.
pub fn sanitize_file_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c => c,
        })
        .collect();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let s = s.trim_matches([' ', '.']).to_string();
    if s.is_empty() {
        "未命名".to_string()
    } else {
        s
    }
}

/// Subgroup-level download directory: `<download dir>/<bangumi name> - <subgroup name>`.
///
/// The directory name includes the subgroup so files from different subgroups do
/// not land in the same folder; on unsubscribe the whole path is removed.
pub fn subgroup_download_dir(base_dir: &Path, bangumi_name: &str, group_name: &str) -> PathBuf {
    base_dir.join(format!(
        "{} - {}",
        sanitize_file_name(bangumi_name),
        sanitize_file_name(group_name)
    ))
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirs_are_absolute() {
        assert!(app_data_dir().is_absolute());
        assert!(app_cache_dir().is_absolute());
        assert_ne!(app_data_dir(), app_cache_dir());
    }
}
