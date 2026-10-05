//! Reading and writing of task business metadata (`torrents/meta/<hash>.json`).
//!
//! This runs in parallel with librqbit's own persistence (`torrents/session/`): the
//! former stores business information (episode title, output directory), while the
//! latter stores torrent state and the downloaded bitmap.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Task business metadata (persisted to `torrents/meta/<hash>.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TaskMeta {
    pub(crate) title: String,
    pub(crate) output_dir: PathBuf,
}

/// Read all task metadata in the directory: hash → meta.
pub(crate) fn read_all(meta_dir: &Path) -> HashMap<String, TaskMeta> {
    let mut metas = HashMap::new();
    let Ok(entries) = std::fs::read_dir(meta_dir) else {
        return metas;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json")
            && let Some(hash) = path.file_stem().and_then(|stem| stem.to_str())
            && let Some(meta) = read_file(&path)
        {
            metas.insert(hash.to_string(), meta);
        }
    }
    metas
}

/// Read the task metadata for a given output directory.
pub(crate) fn read_in_dir(meta_dir: &Path, dir: &Path) -> Vec<(String, TaskMeta)> {
    read_all(meta_dir)
        .into_iter()
        .filter(|(_, meta)| meta.output_dir == dir)
        .collect()
}

/// Read a single task's metadata.
pub(crate) fn read_one(meta_dir: &Path, hash: &str) -> Option<TaskMeta> {
    read_file(&meta_dir.join(format!("{hash}.json")))
}

/// Write task metadata.
pub(crate) fn write(meta_dir: &Path, hash: &str, meta: &TaskMeta) {
    let Ok(value) = serde_json::to_vec_pretty(meta) else {
        return;
    };
    if let Err(e) = std::fs::write(meta_dir.join(format!("{hash}.json")), value) {
        eprintln!("保存任务元信息失败: {e}");
    }
}

/// Delete task metadata.
pub(crate) fn remove(meta_dir: &Path, hash: &str) {
    let _ = std::fs::remove_file(meta_dir.join(format!("{hash}.json")));
}

/// Read and deserialize a metadata file, returning `None` when it is missing or invalid.
fn read_file(path: &Path) -> Option<TaskMeta> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}
