//! The download background main loop: session lifecycle + command handling + periodic snapshots.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc::UnboundedReceiver;

use crate::data::model::{DownloadCmd, DownloadError, DownloadEvent};
use crate::engine::commands::handle_cmd;
use crate::engine::manager::EngineState;
use crate::engine::session::{cleanup_orphan_meta, create_session, update_status};
use crate::engine::snapshot::update_snapshot;
use crate::support::magnet::magnet_info_hash;

/// The background main loop: start the session, then `select` between commands and a per-second snapshot tick.
pub(crate) async fn run_loop(
    mut cmd_rx: UnboundedReceiver<DownloadCmd>,
    state: Arc<EngineState>,
    base_dir: PathBuf,
) {
    let persist_dir = base_dir.join("torrents").join("session");
    let meta_dir = base_dir.join("torrents").join("meta");
    let dht_file = base_dir.join("dht.dat");
    if let Err(e) = std::fs::create_dir_all(&persist_dir) {
        eprintln!("创建下载状态目录失败: {e}");
    }
    if let Err(e) = std::fs::create_dir_all(&meta_dir) {
        eprintln!("创建下载元信息目录失败: {e}");
    }

    let session = match create_session(&persist_dir, &dht_file).await {
        Ok(session) => session,
        Err(e) => {
            eprintln!("下载引擎启动失败: {e:#}");
            // Notify the UI that downloads are unavailable; otherwise every download click would silently fail.
            state.push_event(DownloadEvent::EngineFailed {
                error: DownloadError::EngineInit,
            });
            return;
        }
    };
    eprintln!("下载引擎就绪(已恢复持久化任务)");
    match session.listen_addr() {
        Some(addr) => eprintln!("入站监听: {addr}"),
        None => eprintln!("入站未监听(仅主动出站)"),
    }

    // Orphan cleanup: delete metadata for tasks that exist in meta but not in session (the task no longer exists).
    cleanup_orphan_meta(&persist_dir, &meta_dir);

    let mut tick = tokio::time::interval(Duration::from_secs(1));
    // Restored historical tasks only establish a snapshot baseline and do not trigger
    // completion notifications. Only tasks newly added during this run may send completion
    // notifications, avoiding false reports when old tasks briefly transition from
    // Initializing/Downloading to Completed on restore.
    let mut fresh_task_ids = HashSet::new();
    // librqbit's first snapshot after restoring tasks may temporarily report completed
    // tasks as initializing/downloading. The first snapshot filters out these historical
    // tasks, and the next snapshot displays tasks that really are still downloading.
    let mut startup_filter_pending = true;
    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                let Some(cmd) = cmd else {
                    break;
                };
                if let DownloadCmd::Add { magnet, title, output_dir } = &cmd {
                    // Register synchronously before starting the async metadata fetch, ensuring that a
                    // cancel or directory cleanup immediately afterwards can always observe this task.
                    if let Some(hash) = magnet_info_hash(magnet) {
                        fresh_task_ids.insert(hash.clone());
                        state.pending_insert(hash, title.clone(), output_dir.clone());
                    }
                    let session = session.clone();
                    let meta_dir = meta_dir.clone();
                    let state = state.clone();
                    tokio::spawn(async move {
                        handle_cmd(&session, &meta_dir, &state, cmd).await;
                    });
                } else {
                    // Cancel and unsubscribe check/cleanup preserve actor ordering and do not run concurrently.
                    handle_cmd(&session, &meta_dir, &state, cmd).await;
                }
            }
            _ = tick.tick() => {
                update_snapshot(
                    &session,
                    &meta_dir,
                    &state,
                    &mut fresh_task_ids,
                    &mut startup_filter_pending,
                );
                update_status(&session, &state);
            },
        }
    }
    session.stop().await;
}
