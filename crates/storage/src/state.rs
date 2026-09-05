use domain::Subscription;

/// 持久化文件路径(标准数据目录,并迁移旧版相对路径文件)
fn state_path() -> std::path::PathBuf {
    let dir = crate::paths::app_data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let new = dir.join("state.json");
    // 迁移旧版(运行目录下的 mikan_state.json):copy 到 .tmp 后 rename,保证原子性
    let old = std::path::PathBuf::from("mikan_state.json");
    if old.exists() && !new.exists() {
        let tmp = new.with_extension("json.tmp");
        if std::fs::copy(&old, &tmp).is_ok() {
            let _ = std::fs::rename(&tmp, &new);
        }
    }
    new
}

/// 全文件读-改-写的互斥锁(防止并发写者交错丢字段)
static STATE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 原子写入:先写 `state.json.tmp` 再 rename 替换。
/// 进程中途崩溃不会留下半截文件。
fn atomic_write(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

/// 读取状态文件。损坏或根节点类型错误时备份并拒绝自动写回，
/// 避免一次设置修改覆盖全部用户状态。
fn read_state(path: &std::path::Path) -> Option<serde_json::Value> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Some(serde_json::json!({}));
        }
        Err(error) => {
            eprintln!("读取状态文件失败: {error}");
            return None;
        }
    };
    if text.trim().is_empty() {
        return Some(serde_json::json!({}));
    }

    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(state @ serde_json::Value::Object(_)) => Some(state),
        Ok(_) => {
            let bak = path.with_extension("json.bak");
            let _ = std::fs::write(&bak, &text);
            eprintln!("state.json 根节点不是对象,已备份到 {bak:?}");
            None
        }
        Err(error) => {
            let bak = path.with_extension("json.bak");
            let _ = std::fs::write(&bak, &text);
            eprintln!("state.json 解析失败,已备份到 {bak:?}: {error}");
            None
        }
    }
}

/// 读写合并的状态文件(原子写 + 互斥)
fn write_state(update: impl FnOnce(&mut serde_json::Value)) {
    let _guard = STATE_LOCK.lock().unwrap();
    let path = state_path();
    let Some(mut state) = read_state(&path) else {
        return;
    };
    update(&mut state);
    if let Ok(text) = serde_json::to_string_pretty(&state)
        && let Err(e) = atomic_write(&path, &text)
    {
        eprintln!("写入状态文件失败: {e}");
    }
}

/// 读取状态文件(带互斥,损坏时备份)
fn read_json_field(key: &str) -> Option<serde_json::Value> {
    let _guard = STATE_LOCK.lock().unwrap();
    read_state(&state_path())?.get(key).cloned()
}

/// 保存订阅记录
pub fn save_subscriptions(subscriptions: &[Subscription]) {
    write_state(|state| {
        state["subscriptions"] = serde_json::to_value(subscriptions).unwrap_or_default();
    });
}

/// 加载订阅记录
pub fn load_subscriptions() -> Vec<Subscription> {
    read_json_field("subscriptions")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

/// 保存主题模式
pub fn save_theme_mode(mode: &str) {
    write_state(|state| {
        state["theme"] = serde_json::Value::String(mode.into());
    });
}

/// 读取已保存的主题模式(light / dark / 无)
pub fn load_theme_mode() -> Option<String> {
    read_json_field("theme")?.as_str().map(|s| s.to_string())
}

/// 写入任意 JSON 字段(合并到状态文件)
pub fn save_json_field(key: &str, value: &serde_json::Value) {
    write_state(|state| {
        state[key] = value.clone();
    });
}

/// 读取任意 JSON 字段
pub fn load_json_field(key: &str) -> Option<serde_json::Value> {
    read_json_field(key)
}

/// 下载目录的内存缓存(读取时惰性填充,保存时失效)。
/// 避免 render 每帧读盘解析 state.json。
static DOWNLOAD_DIR_CACHE: std::sync::OnceLock<std::sync::Mutex<Option<std::path::PathBuf>>> =
    std::sync::OnceLock::new();

fn download_dir_cache() -> &'static std::sync::Mutex<Option<std::path::PathBuf>> {
    DOWNLOAD_DIR_CACHE.get_or_init(|| std::sync::Mutex::new(None))
}

/// 下载目录(默认 `~/Videos`,用户可修改)
pub fn load_download_dir() -> std::path::PathBuf {
    let mut cache = download_dir_cache().lock().unwrap();
    if let Some(dir) = cache.as_ref() {
        return dir.clone();
    }
    let dir = load_json_field("download_dir")
        .and_then(|v| v.as_str().map(std::path::PathBuf::from))
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(crate::paths::video_dir);
    *cache = Some(dir.clone());
    dir
}

pub fn save_download_dir(dir: &std::path::Path) {
    save_json_field(
        "download_dir",
        &serde_json::Value::String(dir.to_string_lossy().into_owned()),
    );
    *download_dir_cache().lock().unwrap() = Some(dir.to_path_buf());
}

/// 订阅详情页的剧集筛选关键词(JSON key 使用 "番剧id:字幕组id")
pub fn save_subgroup_keywords(keywords: &std::collections::HashMap<(u32, u32), String>) {
    let map: serde_json::Map<String, serde_json::Value> = keywords
        .iter()
        .map(|((bangumi_id, subgroup_id), keyword)| {
            (
                format!("{bangumi_id}:{subgroup_id}"),
                serde_json::Value::String(keyword.clone()),
            )
        })
        .collect();
    save_json_field("subgroup_keywords", &serde_json::Value::Object(map));
}

/// 读取剧集筛选关键词
pub fn load_subgroup_keywords() -> std::collections::HashMap<(u32, u32), String> {
    parse_subgroup_keywords(load_json_field("subgroup_keywords"))
}

/// 解析筛选关键词 JSON(独立纯函数,便于测试)
fn parse_subgroup_keywords(
    value: Option<serde_json::Value>,
) -> std::collections::HashMap<(u32, u32), String> {
    value
        .and_then(|v| v.as_object().cloned())
        .map(|obj| {
            obj.into_iter()
                .filter_map(|(key, value)| {
                    let (bangumi, subgroup) = key.split_once(':')?;
                    let bangumi_id: u32 = bangumi.parse().ok()?;
                    let subgroup_id: u32 = subgroup.parse().ok()?;
                    let keyword = value.as_str()?;
                    if keyword.is_empty() {
                        return None;
                    }
                    Some(((bangumi_id, subgroup_id), keyword.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_keywords_roundtrip() {
        let value = serde_json::json!({
            "101:5": "简日",
            "202:7": "1080",
            "303:9": ""
        });
        let map = parse_subgroup_keywords(Some(value));
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&(101, 5)).map(String::as_str), Some("简日"));
        assert_eq!(map.get(&(202, 7)).map(String::as_str), Some("1080"));
        // 空关键词被丢弃
        assert!(!map.contains_key(&(303, 9)));
    }

    #[test]
    fn damaged_state_is_backed_up_and_rejected() {
        let path =
            std::env::temp_dir().join(format!("mikan-state-damaged-{}.json", std::process::id()));
        let backup = path.with_extension("json.bak");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&backup);
        std::fs::write(&path, "{not-json").expect("write damaged state fixture");

        assert!(read_state(&path).is_none());
        assert_eq!(
            std::fs::read_to_string(&backup).expect("damaged state backup"),
            "{not-json"
        );

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(backup);
    }

    #[test]
    fn non_object_state_is_rejected() {
        let path =
            std::env::temp_dir().join(format!("mikan-state-array-{}.json", std::process::id()));
        let backup = path.with_extension("json.bak");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&backup);
        std::fs::write(&path, "[]").expect("write non-object state fixture");

        assert!(read_state(&path).is_none());
        assert!(backup.exists());

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(backup);
    }

    #[test]
    fn parse_keywords_invalid_entries_ignored() {
        let value = serde_json::json!({
            "bad-key": "x",
            "1:not-a-number": "y",
            "2:3": 42
        });
        assert!(parse_subgroup_keywords(Some(value)).is_empty());
        assert!(parse_subgroup_keywords(None).is_empty());
    }
}
