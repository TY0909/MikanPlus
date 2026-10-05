//! Handle download commands: add / cancel / unsubscribe check and cleanup.
//!
//! Each command is a transformation of the session and metadata data; commands are kept
//! from running concurrently by actor ordering (see [`crate::engine::runtime`]).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use librqbit::{AddTorrent, AddTorrentOptions, AddTorrentResponse, Magnet, Session};

use crate::data::meta::{self, TaskMeta};
use crate::data::model::{DownloadCmd, DownloadError, DownloadEvent, METADATA_TIMEOUT, TRACKERS};
use crate::engine::manager::EngineState;
use crate::support::magnet::magnet_info_hash;

/// Handle one download command.
pub(crate) async fn handle_cmd(
    session: &Arc<Session>,
    meta_dir: &Path,
    state: &EngineState,
    cmd: DownloadCmd,
) {
    match cmd {
        DownloadCmd::Add {
            magnet,
            title,
            output_dir,
        } => {
            add(session, meta_dir, state, magnet, title, output_dir).await;
        }
        DownloadCmd::Cancel { id } => {
            cancel(session, meta_dir, state, &id).await;
        }
        DownloadCmd::CheckUnsubscribeDir {
            dir,
            bangumi_name,
            group_name,
        } => {
            let active = active_titles_for_dir(session, meta_dir, state, &dir);
            if active.is_empty() {
                state.push_event(DownloadEvent::UnsubscribeCheckReady { dir });
            } else {
                let count = active.len();
                eprintln!("退订「{bangumi_name} - {group_name}」被阻断:{count} 个剧集正在下载");
                state.push_event(DownloadEvent::UnsubscribeBlocked {
                    dir,
                    titles: active,
                });
            }
        }
        DownloadCmd::Unsubscribe {
            dir,
            bangumi_name,
            group_name,
            remove_downloads,
        } => {
            unsubscribe(
                session,
                meta_dir,
                state,
                dir,
                &bangumi_name,
                &group_name,
                remove_downloads,
            )
            .await;
        }
    }
}

/// Add a task: merge trackers, take over an existing task, fetch metadata with a timeout,
/// and write metadata.
async fn add(
    session: &Arc<Session>,
    meta_dir: &Path,
    state: &EngineState,
    magnet: String,
    title: String,
    output_dir: PathBuf,
) {
    // Merge the trackers carried by the magnet with the public list into a unified,
    // deduplicated list. librqbit only uses the `tr` parameters embedded in the magnet URL
    // for magnets, so those trackers are injected here.
    let hash = magnet_info_hash(&magnet);
    let mut parsed = match Magnet::parse(&magnet) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("磁力解析失败: {e}");
            state.push_event(DownloadEvent::AddFailed {
                title,
                error: DownloadError::MagnetParse,
            });
            return;
        }
    };
    let mut seen: HashSet<String> = parsed.trackers.iter().cloned().collect();
    for tracker in TRACKERS {
        if seen.insert(tracker.to_string()) {
            parsed.trackers.push(tracker.to_string());
        }
    }
    eprintln!("「{title}」tracker: 共 {} 个", parsed.trackers.len());
    let magnet = parsed.to_string();

    let add_opts = AddTorrentOptions {
        output_folder: Some(output_dir.to_string_lossy().into_owned()),
        overwrite: true,
        ..Default::default()
    };
    // A task with the same name already in the session (e.g. re-downloading after its files
    // were deleted externally): remove the old task and its metadata first, then add again.
    if let Some(hash) = &hash {
        let existing: Vec<_> = session.with_torrents(|it| {
            it.filter_map(|(tid, handle)| {
                (handle.info_hash().as_string().to_lowercase() == *hash).then_some(tid)
            })
            .collect()
        });
        if !existing.is_empty() {
            for tid in existing {
                let _ = session.delete(tid.into(), true).await;
            }
            meta::remove(meta_dir, hash);
            // Clean up the directory if empty (files of other episodes in the same directory are kept)
            let _ = std::fs::remove_dir(&output_dir);
            eprintln!("「{title}」重新下载:已移除旧任务");
        }
    }

    let fut = session.add_torrent(AddTorrent::Url(magnet.into()), Some(add_opts));
    let response = match tokio::time::timeout(METADATA_TIMEOUT, fut).await {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => {
            if let Some(hash) = &hash {
                state.pending_remove(hash);
            }
            eprintln!("添加任务失败: {e:#}");
            state.push_event(DownloadEvent::AddFailed {
                title,
                error: DownloadError::AddFailed,
            });
            return;
        }
        Err(_) => {
            if let Some(hash) = &hash {
                state.pending_remove(hash);
            }
            eprintln!("获取资源信息超时({METADATA_TIMEOUT:?}),该资源可能暂无可用来源");
            state.push_event(DownloadEvent::AddFailed {
                title,
                error: DownloadError::MetadataTimeout,
            });
            return;
        }
    };

    match response {
        AddTorrentResponse::Added(_, handle) => {
            let hash = handle.info_hash().as_string().to_lowercase();
            state.pending_remove(&hash);
            // Cancelled during metadata fetching: delete the task immediately, leaving no artifacts
            if state.cancelled_take(&hash) {
                let id = handle.id();
                let _ = session.delete(id.into(), true).await;
                // Clean up the leftover empty directory (removed only when empty; files of other
                // episodes in the same directory are kept)
                let _ = std::fs::remove_dir(&output_dir);
                eprintln!("「{title}」在获取信息期间被取消,已清理");
                return;
            }
            let meta = TaskMeta { title, output_dir };
            meta::write(meta_dir, &hash, &meta);
            eprintln!("「{}」已开始下载", meta.title);
        }
        AddTorrentResponse::AlreadyManaged(_, _) => {
            // Concurrent duplicate add (e.g. a rapid double-click): the task is already in the
            // session; the first add wins
            if let Some(hash) = &hash {
                state.pending_remove(hash);
            }
            eprintln!("「{title}」已在会话中,忽略重复添加");
        }
        AddTorrentResponse::ListOnly(_) => {
            // This app never produces list-only responses; this is only defensive cleanup
            if let Some(hash) = &hash {
                state.pending_remove(hash);
            }
        }
    }
}

/// Cancel a task: defer cancellation for tasks still fetching metadata, and delete
/// registered tasks and metadata.
async fn cancel(session: &Arc<Session>, meta_dir: &Path, state: &EngineState, id: &str) {
    // Read metadata first (the cancellation log needs the title)
    let meta = meta::read_one(meta_dir, id);
    // Only tasks still fetching metadata need a deferred cancellation mark; registered tasks
    // are deleted directly.
    if state.pending_remove(id) {
        state.cancelled_insert(id);
    }
    let targets: Vec<_> = session.with_torrents(|it| {
        it.filter_map(|(tid, handle)| {
            (handle.info_hash().as_string().to_lowercase() == id).then_some(tid)
        })
        .collect()
    });
    for tid in targets {
        if let Err(e) = session.delete(tid.into(), true).await {
            eprintln!("取消任务失败: {e:#}");
        }
    }
    // Delete metadata and clean up any leftover empty output directory
    meta::remove(meta_dir, id);
    if let Some(meta) = meta {
        // Only delete the directory if empty (files of other episodes of the show are kept)
        let _ = std::fs::remove_dir(&meta.output_dir);
        eprintln!("「{}」已取消,文件已删除", meta.title);
    }
}

/// Unsubscribe cleanup: check for in-progress tasks and, per the option, decide whether to
/// cancel tasks and delete the directory.
async fn unsubscribe(
    session: &Arc<Session>,
    meta_dir: &Path,
    state: &EngineState,
    dir: PathBuf,
    bangumi_name: &str,
    group_name: &str,
    remove_downloads: bool,
) {
    // Unsubscribe cleanup: the check, cancellation, and directory deletion complete atomically
    // on the same thread, without relying on the UI-side periodic snapshot (avoiding a race
    // window when unsubscribing right after starting a download).
    let active = active_titles_for_dir(session, meta_dir, state, &dir);
    if !active.is_empty() {
        // There are in-progress tasks: block the unsubscribe and let the UI show a warning dialog
        let count = active.len();
        eprintln!("退订「{bangumi_name} - {group_name}」被阻断:{count} 个剧集正在下载");
        state.push_event(DownloadEvent::UnsubscribeBlocked {
            dir,
            titles: active,
        });
        return;
    }

    if !remove_downloads {
        // The user chose to keep the downloaded content: after the check passes through the same
        // actor, allow the unsubscribe without cancelling tasks, deleting metadata, or touching
        // the directory.
        state.push_event(DownloadEvent::UnsubscribeDone { dir });
        return;
    }

    // Cancel leftover tasks (completed seeding / errored), and delete metadata
    for (hash, meta) in meta::read_in_dir(meta_dir, &dir) {
        let targets: Vec<_> = session.with_torrents(|it| {
            it.filter_map(|(tid, handle)| {
                (handle.info_hash().as_string().to_lowercase() == hash).then_some(tid)
            })
            .collect()
        });
        for tid in targets {
            let _ = session.delete(tid.into(), true).await;
        }
        meta::remove(meta_dir, &hash);
        eprintln!("退订清理:「{}」已取消", meta.title);
    }
    // Delete the entire directory (including fully downloaded video files; no task holds this
    // directory at this point)
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => {
            eprintln!("退订「{bangumi_name} - {group_name}」:下载目录已删除");
        }
        Err(e) => {
            eprintln!("退订「{bangumi_name} - {group_name}」:删除目录失败 {e}");
        }
    }
    // Allow the unsubscribe even if directory deletion fails (leftover files can be removed
    // manually), so the subscription record and download state do not get permanently stuck
    state.push_event(DownloadEvent::UnsubscribeDone { dir });
}

/// Collect the titles of tasks still in progress in the given download directory.
///
/// Checks both tasks already registered in librqbit and pending tasks still fetching
/// metadata, so a download that just started is not missed while the unsubscribe
/// confirmation dialog is open.
fn active_titles_for_dir(
    session: &Arc<Session>,
    meta_dir: &Path,
    state: &EngineState,
    dir: &Path,
) -> Vec<String> {
    let mut active: Vec<String> = meta::read_in_dir(meta_dir, dir)
        .into_iter()
        .filter(|(hash, _)| is_active_in_session(session, hash))
        .map(|(_, meta)| meta.title)
        .collect();
    active.extend(
        state
            .pending_snapshot()
            .into_values()
            .filter(|pending| pending.output_dir == dir)
            .map(|pending| pending.title),
    );
    active
}

/// Whether the session contains a task with this info_hash that has not yet completed.
fn is_active_in_session(session: &Arc<Session>, hash: &str) -> bool {
    session.with_torrents(|it| {
        let mut found = false;
        for (_, handle) in it {
            if handle.info_hash().as_string().to_lowercase() == hash && !handle.stats().finished {
                found = true;
                break;
            }
        }
        found
    })
}
