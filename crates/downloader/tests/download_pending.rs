//! 回归测试:metadata 获取耗时期间(任务尚未注册到 librqbit),
//! 快照循环必须继续运行并展示「获取信息…」状态(不阻塞)。

use std::time::Duration;

use downloader::{DownloadCmd, DownloadManager, TaskState};

/// 随机 info_hash(不存在对应资源,metadata 将永远获取不到)
const RANDOM_HASH: &str = "0000000000000000000000000000000000000001";

#[test]
fn pending_state_shows_during_slow_metadata() {
    // 隔离数据目录:避免与并行测试/真实数据共享 DHT 端口与任务状态
    let base = std::env::temp_dir().join(format!("mikan_pending_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mgr = DownloadManager::start_with_base(base.clone());

    let id20 = librqbit::dht::Id20::from_bytes(&[0u8; 20]).unwrap();
    let mut magnet = librqbit::Magnet::from_id20(id20, Vec::new(), None);
    magnet.name = Some("[测试] 无源任务".into());
    // 覆盖 info_hash 为固定测试值,便于断言
    let magnet = magnet
        .to_string()
        .replace("0000000000000000000000000000000000000000", RANDOM_HASH);

    let _ = mgr.send(DownloadCmd::Add {
        magnet,
        title: "[测试] 无源任务".into(),
        output_dir: std::env::temp_dir().join("mikan_pending_test"),
    });

    // metadata 永远拿不到(add_torrent 会挂到 60s 超时),
    // 但快照循环不受影响:1 秒内应出现 Initializing 状态
    let mut appeared = false;
    for i in 0..10 {
        std::thread::sleep(Duration::from_millis(500));
        let events = downloader::take_events();
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

    // 取消:状态应消失
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
