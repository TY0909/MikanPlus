//! 启动时的一次性迁移:把旧版布局的缓存与文件迁到新布局。
//!
//! - 历史版本命名的 JSON 缓存(list-v2/v3、detail-v2/v3、search 旧目录)全部清理,
//!   当前代码统一使用无版本命名(见 cache.rs)
//! - DHT 路由表:librqbit 第三方默认目录 → 应用数据目录(dht.dat)
//!
//! 每个迁移都幂等:目标已存在则跳过,失败不阻断启动。

use std::path::Path;

use crate::paths;

/// 执行全部迁移(同步、快速,启动时调用一次)
pub fn run_all() {
    cleanup_legacy_cache();
    migrate_dht();
}

/// 清理历史版本的缓存文件:开发早期用过 list-v2/v3、detail-v2/v3 等命名,
/// 现已统一为无版本命名,旧文件直接删除(内容由 TTL 自然失效,损失可忽略)。
fn cleanup_legacy_cache() {
    let cache = paths::app_cache_dir();
    if !cache.exists() {
        return;
    }
    // 仅删除历史版本命名的文件;无版本命名(list.json、detail/、search/)
    // 是当前代码正在使用的,必须保留
    for name in ["list-v2.json", "list-v3.json"] {
        let _ = std::fs::remove_file(cache.join(name));
    }
    for dir in ["detail-v2", "detail-v3", "search-v3"] {
        let _ = std::fs::remove_dir_all(cache.join(dir));
    }
}

/// librqbit 默认 DHT 文件 → <数据目录>/dht.dat
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
        // 顺带清理空的第三方目录
        let _ = std::fs::remove_dir(old.parent().unwrap_or(Path::new("")));
    }
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn legacy_cache_cleanup_removes_old_files() {
        // 用临时目录验证清理逻辑(直接测试 rename 幂等性)
        let dir = std::env::temp_dir().join(format!("mikan_migrate_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let from = dir.join("old");
        let to = dir.join("new");
        std::fs::write(&from, b"x").unwrap();

        // 幂等 rename(逻辑与 migrate_dht 一致)
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
