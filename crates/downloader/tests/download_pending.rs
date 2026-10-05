//! Regression test: while metadata fetching is slow (the task is not yet registered with librqbit),
//! the snapshot loop must keep running and show the "fetching info…" state without blocking.

use std::time::Duration;

use downloader::{DownloadCmd, DownloadManager, TaskState};

/// Random info_hash (no matching resource exists, so metadata will never be fetched)
const RANDOM_HASH: &str = "0000000000000000000000000000000000000001";

#[test]
fn pending_state_shows_during_slow_metadata() {
    // Isolated data directory: avoids sharing the DHT port and task state with parallel tests or real data.
    let base = std::env::temp_dir().join(format!("mikan_pending_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mgr = DownloadManager::start_with_base(base.clone());

    let id20 = librqbit::dht::Id20::from_bytes(&[0u8; 20]).unwrap();
    let mut magnet = librqbit::Magnet::from_id20(id20, Vec::new(), None);
    magnet.name = Some("[测试] 无源任务".into());
    // Override info_hash with a fixed test value so the assertion is deterministic.
    let magnet = magnet
        .to_string()
        .replace("0000000000000000000000000000000000000000", RANDOM_HASH);

    let _ = mgr.send(DownloadCmd::Add {
        magnet,
        title: "[测试] 无源任务".into(),
        output_dir: std::env::temp_dir().join("mikan_pending_test"),
    });

    // Metadata can never be fetched (add_torrent hangs until its 60s timeout),
    // but the snapshot loop is unaffected: the Initializing state should appear within 1 second.
    let mut appeared = false;
    for i in 0..10 {
        std::thread::sleep(Duration::from_millis(500));
        let events = mgr.take_events();
        if !events.is_empty() {
            println!("事件: {:?}", events);
        }
        let snap = mgr.snapshot();
        if i < 4 || !snap.is_empty() {
            println!(
                "[{}] 快照: {:?}",
                i,
                snap.iter()
                    .map(|t| (t.id.clone(), format!("{:?}", t.state)))
                    .collect::<Vec<_>>()
            );
        }
        if let Some(task) = snap.iter().find(|t| t.id == RANDOM_HASH) {
            println!("任务已出现: state={:?}", task.state);
            assert_eq!(task.state, TaskState::Initializing);
            appeared = true;
            break;
        }
    }
    assert!(appeared, "metadata 获取期间快照中应出现「获取信息…」状态");

    // Cancel: the state should disappear.
    let _ = mgr.send(DownloadCmd::Cancel {
        id: RANDOM_HASH.to_string(),
    });
    let mut removed = false;
    for _ in 0..10 {
        std::thread::sleep(Duration::from_millis(500));
        if !mgr.snapshot().iter().any(|t| t.id == RANDOM_HASH) {
            removed = true;
            break;
        }
    }
    assert!(removed, "取消后「获取信息…」状态应消失");
}
