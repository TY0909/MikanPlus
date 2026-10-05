//! Project the session's live data into a UI snapshot ([`TaskView`]).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use librqbit::{Session, TorrentStatsState};

use crate::data::meta;
use crate::data::model::{DownloadError, DownloadEvent, TaskState, TaskView};
use crate::engine::manager::EngineState;
use crate::support::files::{is_safe_existing_path, is_video_file, safe_task_file_path};

/// Produce the task snapshot (once per second).
pub(crate) fn update_snapshot(
    session: &Arc<Session>,
    meta_dir: &Path,
    state: &EngineState,
    fresh_task_ids: &mut HashSet<String>,
    startup_filter_pending: &mut bool,
) {
    let metas = meta::read_all(meta_dir);
    // In-progress add requests are shown first in a "fetching info…" state (immediate feedback
    // after clicking download), then overwritten by the real task (same info_hash)
    let mut views: HashMap<String, TaskView> = state
        .pending_snapshot()
        .into_iter()
        .map(|(hash, pending)| {
            (
                hash.clone(),
                TaskView {
                    id: hash,
                    title: pending.title,
                    progress: 0.0,
                    download_rate: 0,
                    upload_rate: 0,
                    downloaded: 0,
                    total: 0,
                    state: TaskState::Initializing,
                    peers: 0,
                    video_files: Vec::new(),
                    output_dir: Some(pending.output_dir),
                },
            )
        })
        .collect();

    let session_views = session.with_torrents(|it| {
        let mut out = Vec::new();
        for (_, handle) in it {
            let hash = handle.info_hash().as_string().to_lowercase();
            let stats = handle.stats();
            let meta = metas.get(&hash);

            let (state, video_files) = match &stats.state {
                TorrentStatsState::Initializing { .. } => (TaskState::Initializing, Vec::new()),
                TorrentStatsState::Live if stats.finished => match meta {
                    Some(meta) => {
                        // Paths are resolved only from this torrent's metadata, without scanning the whole
                        // output directory, so multiple search tasks sharing a download directory never
                        // mix up files.
                        let files = handle
                            .with_metadata(|m| {
                                m.info
                                    .iter_file_details()
                                    .filter_map(|fd| {
                                        safe_task_file_path(
                                            &meta.output_dir,
                                            &fd.filename.to_pathbuf(),
                                        )
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        let expected_video_count =
                            files.iter().filter(|path| is_video_file(path)).count();
                        let output_root = meta.output_dir.canonicalize().ok();
                        let videos = files
                            .iter()
                            .filter(|path| {
                                is_video_file(path)
                                    && is_safe_existing_path(output_root.as_deref(), path)
                            })
                            .cloned()
                            .collect::<Vec<_>>();
                        let has_required_content = if expected_video_count > 0 {
                            !videos.is_empty()
                        } else {
                            files.iter().any(|path| path.is_file())
                        };
                        let state = if has_required_content {
                            TaskState::Completed
                        } else {
                            // Mark as Missing when the videos (or all files in a torrent with no videos)
                            // have been deleted externally.
                            TaskState::Missing
                        };
                        (state, videos)
                    }
                    None => (TaskState::Completed, Vec::new()),
                },
                TorrentStatsState::Live => (TaskState::Downloading, Vec::new()),
                TorrentStatsState::Paused => (TaskState::Downloading, Vec::new()),
                TorrentStatsState::Error => {
                    // Low-level error details go only to the log; the UI shows only "download error"
                    if let Some(e) = &stats.error {
                        eprintln!("下载任务出错: {e}");
                    }
                    (TaskState::Error(DownloadError::Torrent), Vec::new())
                }
            };

            out.push(TaskView {
                id: hash.clone(),
                title: meta
                    .map(|m| m.title.clone())
                    .unwrap_or_else(|| handle.name().unwrap_or_default()),
                progress: if stats.total_bytes > 0 {
                    stats.progress_bytes as f64 / stats.total_bytes as f64
                } else {
                    0.0
                },
                download_rate: stats
                    .live
                    .as_ref()
                    .map(|l| (l.download_speed.mbps * 1_000_000.0 / 8.0) as u64)
                    .unwrap_or(0),
                upload_rate: stats
                    .live
                    .as_ref()
                    .map(|l| (l.upload_speed.mbps * 1_000_000.0 / 8.0) as u64)
                    .unwrap_or(0),
                downloaded: stats.progress_bytes,
                total: stats.total_bytes,
                // Active peer count (diagnoses download speed: 0 = no seeders found, non-zero = seeders have low bandwidth)
                peers: stats
                    .live
                    .as_ref()
                    .map(|l| l.snapshot.peer_stats.live as usize)
                    .unwrap_or(0),
                state,
                video_files,
                output_dir: meta.map(|m| m.output_dir.clone()),
            });
        }
        out
    });
    for view in session_views {
        views.insert(view.id.clone(), view);
    }
    let mut views: Vec<TaskView> = views.into_values().collect();
    // HashMap's random iteration order must not become a source of snapshot changes, or it
    // would make the UI redraw pointlessly every second.
    views.sort_unstable_by(|left, right| left.id.cmp(&right.id));

    for title in completed_download_titles(&views, fresh_task_ids) {
        state.push_event(DownloadEvent::DownloadCompleted { title });
    }
    if *startup_filter_pending {
        filter_startup_views(&mut views, fresh_task_ids);
        *startup_filter_pending = false;
    }
    // Tasks that failed to add, were cancelled, or were cleaned up during metadata fetching
    // must not remain in the fresh-task set forever.
    fresh_task_ids.retain(|id| views.iter().any(|view| view.id == *id));

    // Increment the version only when the content changes, avoiding pointless redraws when idle
    state.set_snapshot(views);
}

/// Filter out the first unstable snapshot produced when librqbit restores tasks at startup.
///
/// Historical tasks may briefly appear as Initializing/Downloading before restoration
/// finishes and turn into Completed on the next snapshot. The first snapshot hides these
/// historical tasks so the download monitor page does not show momentary phantom tasks;
/// tasks newly added during this run must be kept, otherwise the user would see no
/// immediate feedback right after clicking download.
fn filter_startup_views(views: &mut Vec<TaskView>, fresh_task_ids: &HashSet<String>) {
    views.retain(|view| fresh_task_ids.contains(&view.id) || !view.state.is_active());
}

/// Find tasks that actually completed during this run.
///
/// Only tasks newly added during this run trigger a notification.
///
/// When restoring historical tasks, librqbit may report initializing/downloading first
/// and completed later, so a state transition in the snapshot cannot be used to detect a
/// "new completion". The fresh set is the only reliable source of new tasks; a task is
/// removed from it as soon as it completes to avoid duplicate notifications.
fn completed_download_titles(
    views: &[TaskView],
    fresh_task_ids: &mut HashSet<String>,
) -> Vec<String> {
    views
        .iter()
        .filter_map(|view| {
            if !matches!(view.state, TaskState::Completed) || !fresh_task_ids.remove(&view.id) {
                return None;
            }
            Some(view.title.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{completed_download_titles, filter_startup_views};
    use crate::data::model::{TaskState, TaskView};
    use std::collections::HashSet;

    fn task(id: &str, title: &str, state: TaskState) -> TaskView {
        TaskView {
            id: id.to_string(),
            title: title.to_string(),
            progress: 1.0,
            download_rate: 0,
            upload_rate: 0,
            downloaded: 0,
            total: 0,
            state,
            peers: 0,
            video_files: Vec::new(),
            output_dir: None,
        }
    }

    #[test]
    fn restored_completed_task_does_not_emit_completion_notification() {
        let views = vec![task("restored", "历史任务", TaskState::Completed)];
        let mut fresh = HashSet::new();

        assert!(completed_download_titles(&views, &mut fresh).is_empty());
    }

    #[test]
    fn restored_task_transition_does_not_emit_completion_notification() {
        let active_views = vec![task("restored", "恢复后完成任务", TaskState::Downloading)];
        let completed_views = vec![task("restored", "恢复后完成任务", TaskState::Completed)];
        let mut fresh = HashSet::new();

        assert!(completed_download_titles(&active_views, &mut fresh).is_empty());
        assert!(completed_download_titles(&completed_views, &mut fresh).is_empty());
    }

    #[test]
    fn newly_added_task_emits_completion_notification() {
        let views = vec![task("active", "新完成任务", TaskState::Completed)];
        let mut fresh = HashSet::from([String::from("active")]);

        assert_eq!(
            completed_download_titles(&views, &mut fresh),
            vec![String::from("新完成任务")]
        );
        assert!(fresh.is_empty());
    }

    #[test]
    fn newly_added_task_emits_even_if_first_observation_is_completed() {
        let views = vec![task("new", "快速完成任务", TaskState::Completed)];
        let mut fresh = HashSet::from([String::from("new")]);

        assert_eq!(
            completed_download_titles(&views, &mut fresh),
            vec![String::from("快速完成任务")]
        );
    }

    #[test]
    fn startup_filter_hides_restored_active_snapshot_but_keeps_new_task() {
        let mut views = vec![
            task(
                "restoring-initializing",
                "历史获取任务",
                TaskState::Initializing,
            ),
            task(
                "restoring-downloading",
                "历史下载任务",
                TaskState::Downloading,
            ),
            task("restored-completed", "历史完成任务", TaskState::Completed),
            task("new", "新下载任务", TaskState::Downloading),
        ];
        let fresh = HashSet::from([String::from("new")]);

        filter_startup_views(&mut views, &fresh);

        assert_eq!(
            views
                .iter()
                .map(|view| view.id.as_str())
                .collect::<Vec<_>>(),
            vec!["restored-completed", "new"]
        );
    }
}
