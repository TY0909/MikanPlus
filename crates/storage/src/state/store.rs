//! Read/write mechanics for the `state.json` file: path resolution, atomic writes, and damage
//! protection.
//!
//! This layer only handles the "file"; the [`State`](super::State) object owns the in-memory
//! document and decides which fields it holds.

use std::path::{Path, PathBuf};

/// Persisted file path (in the standard data directory, migrating the old relative-path file).
pub(super) fn state_path() -> PathBuf {
    let dir = crate::paths::app_data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let new = dir.join("state.json");
    // Migrate the old version (mikan_state.json under the working directory): copy to .tmp then
    // rename, to guarantee atomicity
    let old = PathBuf::from("mikan_state.json");
    if old.exists() && !new.exists() {
        let tmp = new.with_extension("json.tmp");
        if std::fs::copy(&old, &tmp).is_ok() {
            let _ = std::fs::rename(&tmp, &new);
        }
    }
    new
}

/// Atomic write: write `state.json.tmp` first, then rename over the target. A mid-process crash
/// never leaves a half-written file.
fn atomic_write(path: &Path, text: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

/// Reads the state file. On corruption or a wrong root-node type, backs the file up and refuses to
/// write it back automatically, so that a single settings change cannot overwrite all user state.
pub(super) fn read_state(path: &Path) -> Option<serde_json::Value> {
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

/// Atomically persists the whole in-memory document. Damage protection already happened at load,
/// so this only writes when the document was loaded successfully.
pub(super) fn persist(state: &serde_json::Map<String, serde_json::Value>) {
    let path = state_path();
    if let Ok(text) = serde_json::to_string_pretty(state)
        && let Err(e) = atomic_write(&path, &text)
    {
        eprintln!("写入状态文件失败: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::read_state;

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
}
