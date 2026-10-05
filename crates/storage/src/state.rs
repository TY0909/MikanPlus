//! The persisted application state (`state.json`) and its owner.
//!
//! [`State`] owns the whole document in memory (loaded once), exposes strongly typed accessors
//! over it, and persists on every mutation. The key layout is a user-visible persistence
//! contract, so any change needs a matching migration (see `crate::migrate`).
//!
//! The application owns the single [`State`] instance (see `MikanPlus`) and passes it to the
//! components that need it, rather than reaching for a global.

mod store;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use domain::{BangumiId, SubgroupId, SubgroupRef, Subscription};

use store::{persist, read_state, state_path};

/// The owner of the persisted application state (`state.json`).
pub struct State {
    doc: RefCell<Map<String, Value>>,
    /// Whether the document loaded successfully. When the file was damaged at load it is backed up
    /// and writes are refused, so a settings change never overwrites (damaged) user state.
    writable: bool,
}

impl State {
    /// Loads the persisted state from disk (once).
    pub fn load() -> State {
        match read_state(&state_path()) {
            Some(Value::Object(doc)) => State {
                doc: RefCell::new(doc),
                writable: true,
            },
            _ => State {
                doc: RefCell::new(Map::new()),
                writable: false,
            },
        }
    }

    /// Reads a raw field (the document is already parsed in memory).
    fn get(&self, key: &str) -> Option<Value> {
        self.doc.borrow().get(key).cloned()
    }

    /// Writes a raw field and persists the whole document.
    fn set(&self, key: &str, value: Value) {
        if !self.writable {
            eprintln!("状态文件损坏,已拒绝写入以保护原文件");
            return;
        }
        let mut doc = self.doc.borrow_mut();
        doc.insert(key.to_string(), value);
        persist(&doc);
    }

    /// The subscription records.
    pub fn subscriptions(&self) -> Vec<Subscription> {
        self.get("subscriptions")
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default()
    }

    /// Saves the subscription records.
    pub fn set_subscriptions(&self, subscriptions: &[Subscription]) {
        self.set(
            "subscriptions",
            serde_json::to_value(subscriptions).unwrap_or_default(),
        );
    }

    /// The saved theme mode (light / dark / none).
    pub fn theme_mode(&self) -> Option<String> {
        self.get("theme")?.as_str().map(str::to_string)
    }

    /// Saves the theme mode.
    pub fn set_theme_mode(&self, mode: &str) {
        self.set("theme", Value::String(mode.into()));
    }

    /// The download directory (defaults to `~/Videos`, user-modifiable).
    pub fn download_dir(&self) -> PathBuf {
        self.get("download_dir")
            .and_then(|value| value.as_str().map(PathBuf::from))
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(crate::paths::video_dir)
    }

    /// Saves the download directory.
    pub fn set_download_dir(&self, dir: &Path) {
        self.set(
            "download_dir",
            Value::String(dir.to_string_lossy().into_owned()),
        );
    }

    /// The "enable backup domain" switch (off by default, using the primary site mikanani.me).
    pub fn use_backup_domain(&self) -> bool {
        self.get("use_backup_domain")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    /// Saves the "enable backup domain" switch.
    pub fn set_use_backup_domain(&self, enabled: bool) {
        self.set("use_backup_domain", Value::Bool(enabled));
    }

    /// Whether unsubscribing defaults to removing the download directory and files (off by
    /// default, preserving the user's downloads).
    pub fn remove_downloads_on_unsubscribe(&self) -> bool {
        self.get("remove_downloads_on_unsubscribe")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    /// Saves the default choice of "remove download directory and files" in the unsubscribe
    /// confirmation dialog.
    pub fn set_remove_downloads_on_unsubscribe(&self, enabled: bool) {
        self.set("remove_downloads_on_unsubscribe", Value::Bool(enabled));
    }

    /// Episode-filter keywords on the subscription detail page (JSON keys use
    /// "bangumi id:subtitle group id").
    pub fn subgroup_keywords(&self) -> HashMap<SubgroupRef, String> {
        parse_subgroup_keywords(self.get("subgroup_keywords"))
    }

    /// Saves the episode-filter keywords.
    pub fn set_subgroup_keywords(&self, keywords: &HashMap<SubgroupRef, String>) {
        let map: Map<String, Value> = keywords
            .iter()
            .map(|(key, keyword)| {
                (
                    format!("{}:{}", key.bangumi.get(), key.subgroup.get()),
                    Value::String(keyword.clone()),
                )
            })
            .collect();
        self.set("subgroup_keywords", Value::Object(map));
    }
}

/// Parses the filter-keyword JSON (a standalone pure function, easy to test).
fn parse_subgroup_keywords(value: Option<Value>) -> HashMap<SubgroupRef, String> {
    value
        .and_then(|v| v.as_object().cloned())
        .map(|obj| {
            obj.into_iter()
                .filter_map(|(key, value)| {
                    let (bangumi, subgroup) = key.split_once(':')?;
                    let bangumi: u32 = bangumi.parse().ok()?;
                    let subgroup: u32 = subgroup.parse().ok()?;
                    let keyword = value.as_str()?;
                    if keyword.is_empty() {
                        return None;
                    }
                    Some((
                        SubgroupRef::new(BangumiId::from(bangumi), SubgroupId::from(subgroup)),
                        keyword.to_string(),
                    ))
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
        assert_eq!(
            map.get(&SubgroupRef::new(BangumiId::from(101), SubgroupId::from(5)))
                .map(String::as_str),
            Some("简日")
        );
        assert_eq!(
            map.get(&SubgroupRef::new(BangumiId::from(202), SubgroupId::from(7)))
                .map(String::as_str),
            Some("1080")
        );
        // Empty keywords are dropped
        assert!(!map.contains_key(&SubgroupRef::new(BangumiId::from(303), SubgroupId::from(9))));
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
