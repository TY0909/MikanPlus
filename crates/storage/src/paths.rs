//! 跨平台应用目录。
//!
//! 遵循各平台惯例:
//! - macOS: `~/Library/Application Support/<App>`(数据)、`~/Library/Caches/<App>`(缓存)
//! - Linux: `$XDG_DATA_HOME/<app>` / `$XDG_CACHE_HOME/<app>`(默认 `~/.local/share` / `~/.cache`)
//! - Windows: `%APPDATA%\<App>`(数据)、`%LOCALAPPDATA%\<App>`(缓存)

use std::path::{Path, PathBuf};

/// 用户数据目录(订阅记录、设置——不可丢失)
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

/// 缓存目录(图片、列表/详情 JSON——可重新获取,允许被系统清理)
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

/// 默认下载目录(三平台均为 `~/Videos`;Linux 尊重 xdg-user-dirs 的重定向)。
pub fn video_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        // 读取 ~/.config/user-dirs.dirs 中的 XDG_VIDEOS_DIR,失败则回退 ~/Videos
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

/// librqbit 默认的 DHT 持久化路径(第三方默认,待迁入我们的数据目录)。
/// 对应 `directories` crate 的 cache_dir + "com.rqbit.dht/dht.json"。
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

/// 用系统默认程序打开文件或目录。
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

/// 用系统默认浏览器打开 HTTP(S) URL。
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

/// 清洗文件/目录名(跨平台):替换 Windows 非法字符 `\ / : * ? " < > |`,
/// 压缩空白、去掉首尾空格与点;空结果回退「未命名」。
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

/// 字幕组级别的下载目录:`<下载目录>/<番剧名称> - <字幕组名称>`。
///
/// 目录名包含字幕组信息,防止不同字幕组的文件落入同一文件夹;
/// 取消订阅时按此路径整体移除。
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
