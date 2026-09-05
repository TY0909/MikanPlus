//! 磁盘缓存:图片(永久 + 容量上限)+ 数据 JSON(TTL)。
//!
//! 目录结构(见 paths.rs):
//! ```text
//! <cache_dir>/
//! ├── images/<资源键-sha256>   # 封面图片,永久缓存,总量上限 512MB(LRU)
//! ├── list.json                  # 列表页(30 分钟 TTL)
//! ├── detail/<id>.json           # 详情页(24 小时 TTL)
//! └── search/<q-hash>.json       # 搜索(10 分钟 TTL)
//! ```

use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use sha2::{Digest, Sha256};

/// 列表页缓存 TTL
pub const LIST_TTL: Duration = Duration::from_secs(30 * 60);
/// 详情页缓存 TTL
pub const DETAIL_TTL: Duration = Duration::from_secs(24 * 60 * 60);
/// 搜索缓存 TTL
pub const SEARCH_TTL: Duration = Duration::from_secs(10 * 60);

/// 图片缓存总量软上限(字节)
const IMAGE_CACHE_LIMIT: u64 = 512 * 1024 * 1024;
/// 超限后清理到的目标比例(80%)
const IMAGE_CACHE_TARGET_RATIO: f64 = 0.8;

fn ensure_dir(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
}

/// URL → 缓存文件名(SHA-256 十六进制)
fn url_hash(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 图片资源缓存键:与来源主机无关。
///
/// 同一路径在 mikanani.me 与备用域名 mikanime.tv 下是同一份资源,
/// 切换数据源域名不应导致重新下载或产生重复缓存。键由「路径 + 查询串」决定
/// (查询串会改变服务器返回的内容,如尺寸裁剪,因此保留)。
fn image_key(url: &str) -> String {
    let path = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .and_then(|rest| rest.split_once('/'))
        .map(|(_, path)| format!("/{path}"))
        .unwrap_or_else(|| url.to_string());
    url_hash(&path)
}

fn fresh(path: &Path, ttl: Duration) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| {
            SystemTime::now()
                .duration_since(t)
                .map(|age| age < ttl)
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

// ── 数据缓存(JSON) ─────────────────────────────

/// 读取未过期的 JSON 缓存
pub fn cached_json(key: &str, ttl: Duration) -> Option<serde_json::Value> {
    let path = json_path(key);
    if !fresh(&path, ttl) {
        return None;
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

/// 写入 JSON 缓存(原子:临时文件 + rename,杜绝半截文件)
pub fn store_json(key: &str, value: &serde_json::Value) {
    let path = json_path(key);
    if let Some(dir) = path.parent() {
        ensure_dir(dir);
    }
    if let Ok(text) = serde_json::to_string(value) {
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

/// 列表页缓存路径
fn json_path(key: &str) -> PathBuf {
    // key 形如 "list" / "detail/3883" / "search/abc"
    crate::paths::app_cache_dir().join(format!("{key}.json"))
}

/// 列表页缓存
pub fn cached_list() -> Option<serde_json::Value> {
    cached_json("list", LIST_TTL)
}
pub fn store_list(value: &serde_json::Value) {
    store_json("list", value);
}

/// 详情页缓存
pub fn cached_detail(id: u32) -> Option<serde_json::Value> {
    cached_json(&format!("detail/{id}"), DETAIL_TTL)
}
pub fn store_detail(id: u32, value: &serde_json::Value) {
    store_json(&format!("detail/{id}"), value);
}

/// 搜索结果缓存
pub fn cached_search(q: &str) -> Option<serde_json::Value> {
    cached_json(&format!("search/{}", url_hash(q)), SEARCH_TTL)
}
pub fn store_search(q: &str, value: &serde_json::Value) {
    store_json(&format!("search/{}", url_hash(q)), value);
}

// ── 图片缓存(永久) ─────────────────────────────

/// 图片缓存文件路径;未缓存或文件损坏(空文件)时返回 None
pub fn cached_image(url: &str) -> Option<PathBuf> {
    let path = image_path(url);
    let ok = std::fs::metadata(&path).is_ok_and(|m| m.len() > 0);
    if !ok {
        return None;
    }
    // 读取即更新 mtime,使 LRU 按最近使用淘汰
    let _ = filetime_touch(&path);
    Some(path)
}

/// 更新文件 mtime(读取命中时保持 LRU 语义);失败静默忽略
fn filetime_touch(path: &Path) -> std::io::Result<()> {
    let now = filetime::FileTime::now();
    filetime::set_file_mtime(path, now)
}

fn image_path(url: &str) -> PathBuf {
    crate::paths::app_cache_dir()
        .join("images")
        .join(image_key(url))
}

/// 保存图片到缓存(原子:临时文件 + rename,杜绝半截文件被命中)
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

/// 图片缓存容量治理:总量超过软上限时,按修改时间删除最旧的,
/// 直到回到上限的 80%(LRU 近似,图片无写入只有首次保存,mtime ≈ 首次访问)。
/// 启动时调用一次(后台线程)。
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
    // 最旧的在前
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

/// 清空全部缓存(设置页可触发)
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
        // 同一路径在不同数据源域名下共享同一缓存键
        assert_eq!(
            image_key("https://mikanani.me/images/a.jpg?width=400&height=560"),
            image_key("https://mikanime.tv/images/a.jpg?width=400&height=560"),
        );
        assert_eq!(
            image_key("https://mikanani.me/images/a.jpg"),
            image_key("https://mikanime.tv/images/a.jpg"),
        );
        // 查询串影响返回内容,应区分(尺寸裁剪)
        assert_ne!(
            image_key("https://mikanani.me/images/a.jpg?width=400&height=400"),
            image_key("https://mikanani.me/images/a.jpg?width=400&height=560"),
        );
        // 不同路径区分
        assert_ne!(
            image_key("https://mikanani.me/images/a.jpg"),
            image_key("https://mikanani.me/images/b.jpg"),
        );
        // 相对路径 / 无主机 URL 原样参与哈希
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
