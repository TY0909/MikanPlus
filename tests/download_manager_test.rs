//! DownloadManager 集成测试:添加任务 → 快照出现 → 取消 → 快照消失。
//!
//! 使用真实磁力(碧蓝之海 3 第 5 集,info_hash 来自 mikan_session 遗留文件),
//! 不依赖网络连通性:只要任务进入快照(Initializing/Downloading/Error 均可)即通过。

use std::time::Duration;

use mikan_plus::download::{DownloadCmd, DownloadManager};

/// 碧蓝之海 3 - 05 的 info_hash(hex)
const INFO_HASH_HEX: &str = "c06e0fa66e76e5f30d10e4b00eaa2472b6d62a37";

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn add_and_cancel_task() {
    // 隔离数据目录:避免与并行测试/真实数据共享 DHT 端口与任务状态
    let base = std::env::temp_dir().join(format!("mikan_dl_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mgr = DownloadManager::start_with_base(base.clone());

    // 构造磁力(带 tracker,避免依赖蜜柑磁力的失效 tracker)
    let id20 = librqbit::dht::Id20::from_bytes(&hex_to_bytes(INFO_HASH_HEX)).unwrap();
    let magnet = librqbit::Magnet::from_id20(
        id20,
        mikan_plus::download::TRACKERS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        None,
    )
    .to_string();

    // 添加到临时输出目录
    let out_dir = std::env::temp_dir().join("mikan_dl_test");
    let _ = mgr.send(DownloadCmd::Add {
        magnet,
        title: "[测试] 碧蓝之海 3 - 05".into(),
        output_dir: out_dir,
    });

    // 等待任务出现在快照中(最多 20 秒;metadata 获取/初始化均计入)
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

    // 取消任务
    let _ = mgr.send(DownloadCmd::Cancel {
        id: INFO_HASH_HEX.to_string(),
    });

    // 等待任务从快照消失
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
