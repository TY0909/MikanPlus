//! DownloadManager integration test: add task → appears in snapshot → cancel → disappears from snapshot.
//!
//! Uses a real magnet link (Grand Blue 3 episode 5, info_hash taken from a leftover mikan_session file)
//! and does not depend on network connectivity: it passes as long as the task enters the snapshot,
//! whatever its state (Initializing, Downloading, or Error).

use std::time::Duration;

use downloader::{DownloadCmd, DownloadManager};

/// info_hash (hex) of Grand Blue 3 - 05
const INFO_HASH_HEX: &str = "c06e0fa66e76e5f30d10e4b00eaa2472b6d62a37";

/// Converts a lowercase hex string into its raw bytes.
fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn add_and_cancel_task() {
    // Isolated data directory: avoids sharing the DHT port and task state with parallel tests or real data.
    let base = std::env::temp_dir().join(format!("mikan_dl_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mgr = DownloadManager::start_with_base(base.clone());

    // Build the magnet link (with trackers, to avoid relying on the dead trackers baked into Mikan magnet links).
    let id20 = librqbit::dht::Id20::from_bytes(&hex_to_bytes(INFO_HASH_HEX)).unwrap();
    let magnet = librqbit::Magnet::from_id20(
        id20,
        downloader::TRACKERS.iter().map(|s| s.to_string()).collect(),
        None,
    )
    .to_string();

    // Add it to a temporary output directory.
    let out_dir = std::env::temp_dir().join("mikan_dl_test");
    let _ = mgr.send(DownloadCmd::Add {
        magnet,
        title: "[测试] 碧蓝之海 3 - 05".into(),
        output_dir: out_dir,
    });

    // Wait for the task to appear in the snapshot (up to 20 seconds; metadata fetching and initialization are included).
    let mut found = false;
    for _ in 0..40 {
        std::thread::sleep(Duration::from_millis(500));
        let snap = mgr.snapshot();
        if !snap.is_empty() {
            println!(
                "快照: {:?}",
                snap.iter()
                    .map(|t| (t.id.clone(), format!("{:?}", t.state)))
                    .collect::<Vec<_>>()
            );
        }
        if let Some(task) = snap.iter().find(|t| t.id == INFO_HASH_HEX) {
            println!(
                "任务已出现: state={:?} progress={:.2} title={}",
                task.state, task.progress, task.title
            );
            found = true;
            break;
        }
    }
    assert!(found, "添加任务后快照中应出现该任务");

    // Cancel the task.
    let _ = mgr.send(DownloadCmd::Cancel {
        id: INFO_HASH_HEX.to_string(),
    });

    // Wait for the task to disappear from the snapshot.
    let mut removed = false;
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(500));
        if !mgr.snapshot().iter().any(|t| t.id == INFO_HASH_HEX) {
            removed = true;
            break;
        }
    }
    assert!(removed, "取消任务后快照中应不再有该任务");
}
