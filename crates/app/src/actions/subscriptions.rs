//! Subscriptions and unsubscriptions: receipts from the download thread are also
//! settled into subscription data here.

use crate::data::model::{PendingUnsub, UnsubscribeConfirmation, UnsubscribeWarning};
use crate::prelude::*;
use crate::shell::state::MikanPlus;
use domain::{BangumiId, SubgroupId, SubgroupRef};

impl MikanPlus {
    /// Toggle a single subtitle group's subscription (subscriptions are per subtitle group).
    pub(crate) fn toggle_subscription(
        &mut self,
        bangumi_id: BangumiId,
        subgroup_id: SubgroupId,
        bangumi_name: Option<&str>,
        group_name: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        if self.data.is_subscribed(bangumi_id, subgroup_id) {
            self.request_unsubscribe(bangumi_id, subgroup_id, cx);
            return;
        }
        // Cover: prefer a basic entry locatable by name, then fall back to search results for that id
        let cover = self.data.cover_of(bangumi_id);
        self.data.subscriptions.push(Subscription {
            bangumi_id,
            subgroup_id,
            bangumi_name: bangumi_name.unwrap_or("未知").to_string(),
            group_name: group_name.unwrap_or("未知").to_string(),
            cover_url: cover,
        });
        self.state.set_subscriptions(&self.data.subscriptions);
    }

    /// Open the unsubscribe confirmation window; do not change the subscription or downloads before the user confirms.
    pub(crate) fn request_unsubscribe(
        &mut self,
        bangumi_id: BangumiId,
        subgroup_id: SubgroupId,
        cx: &mut Context<Self>,
    ) {
        let Some(sub) = self.data.subscription(bangumi_id, subgroup_id).cloned() else {
            return;
        };
        let dir = storage::paths::subgroup_download_dir(
            &self.state.download_dir(),
            &sub.bangumi_name,
            &sub.group_name,
        );
        self.overlays.checking = Some(UnsubscribeConfirmation {
            bangumi_id,
            subgroup_id,
            bangumi_name: sub.bangumi_name.clone(),
            group_name: sub.group_name.clone(),
            remove_downloads: self.state.remove_downloads_on_unsubscribe(),
        });
        if let Err(e) = self.downloader.send(DownloadCmd::CheckUnsubscribeDir {
            dir: dir.clone(),
            bangumi_name: sub.bangumi_name.clone(),
            group_name: sub.group_name.clone(),
        }) {
            // When the download engine is unavailable, fall back to checking the current snapshot; active tasks still block the unsubscribe with priority.
            eprintln!("退订目录检查命令发送失败: {e}");
            let confirmation = self.overlays.checking.take();
            let active = self.active_titles_in(&dir);
            if let Some(confirmation) = confirmation {
                if active.is_empty() {
                    self.overlays.confirmation = Some(confirmation);
                } else {
                    self.overlays.warning = Some(UnsubscribeWarning {
                        bangumi_name: confirmation.bangumi_name,
                        group_name: confirmation.group_name,
                        active_titles: active,
                    });
                }
            }
            cx.notify();
        }
    }

    /// Apply the unsubscribe confirmation result.
    pub(crate) fn confirm_unsubscribe(&mut self, remove_downloads: bool, cx: &mut Context<Self>) {
        let Some(confirmation) = self.overlays.confirmation.take() else {
            return;
        };
        // The download actor still atomically rechecks active tasks for the final confirmation: even if the
        // user starts a download while the window is open, the "no unsubscribe while downloading" rule cannot be bypassed.
        self.unsubscribe(
            confirmation.bangumi_id,
            confirmation.subgroup_id,
            remove_downloads,
            cx,
        );
        // The confirmation window must close immediately after the confirm click; if cleanup runs asynchronously, the subscription state updates when the receipt arrives.
        cx.notify();
    }

    /// Unsubscribe a single subtitle group.
    ///
    /// "Check active tasks → keep or delete content per the option" is pushed down and executed
    /// atomically by the download thread (via [`DownloadCmd::Unsubscribe`]), avoiding UI-thread stalls
    /// on directory deletion and snapshot races; the result comes back as a download event receipt:
    /// block if there are active tasks, otherwise remove the subscription record once cleanup completes.
    pub(crate) fn unsubscribe(
        &mut self,
        bangumi_id: BangumiId,
        subgroup_id: SubgroupId,
        remove_downloads: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(sub) = self.data.subscription(bangumi_id, subgroup_id).cloned() else {
            return;
        };
        let dir = storage::paths::subgroup_download_dir(
            &self.state.download_dir(),
            &sub.bangumi_name,
            &sub.group_name,
        );
        // Register the pending unsubscribe and wait for the download thread's receipt
        self.overlays.pending_unsub = Some(PendingUnsub {
            bangumi_id,
            subgroup_id,
            bangumi_name: sub.bangumi_name.clone(),
            group_name: sub.group_name.clone(),
        });
        if let Err(e) = self.downloader.send(DownloadCmd::Unsubscribe {
            dir: dir.clone(),
            bangumi_name: sub.bangumi_name.clone(),
            group_name: sub.group_name.clone(),
            remove_downloads,
        }) {
            // Fall back to the current snapshot when the engine is unavailable; even if files are kept, active tasks must still block the unsubscribe.
            eprintln!("退订清理命令发送失败: {e}");
            self.overlays.pending_unsub = None;
            let active = self.active_titles_in(&dir);
            if active.is_empty() {
                if remove_downloads {
                    let _ = std::fs::remove_dir_all(&dir);
                }
                self.finish_unsubscribe(bangumi_id, subgroup_id);
            } else {
                self.overlays.warning = Some(UnsubscribeWarning {
                    bangumi_name: sub.bangumi_name,
                    group_name: sub.group_name,
                    active_titles: active,
                });
            }
            cx.notify();
        }
    }

    /// Receipt: unsubscribe blocked (the directory has active tasks) → show the warning window.
    pub(crate) fn on_unsubscribe_blocked(&mut self, dir: &std::path::Path, titles: Vec<String>) {
        // The initial check takes precedence over the unsubscribe confirmation window: show the block warning directly when there are active tasks.
        if let Some(checking) = self.overlays.checking.as_ref() {
            let expected = storage::paths::subgroup_download_dir(
                &self.state.download_dir(),
                &checking.bangumi_name,
                &checking.group_name,
            );
            if expected == dir {
                let checking = self.overlays.checking.take().unwrap();
                self.overlays.warning = Some(UnsubscribeWarning {
                    bangumi_name: checking.bangumi_name,
                    group_name: checking.group_name,
                    active_titles: titles,
                });
                self.overlays.confirmation = None;
                return;
            }
        }

        let Some(pending) = &self.overlays.pending_unsub else {
            return;
        };
        let expected = storage::paths::subgroup_download_dir(
            &self.state.download_dir(),
            &pending.bangumi_name,
            &pending.group_name,
        );
        if expected != dir {
            return;
        }
        self.overlays.warning = Some(UnsubscribeWarning {
            bangumi_name: pending.bangumi_name.clone(),
            group_name: pending.group_name.clone(),
            active_titles: titles,
        });
        self.overlays.pending_unsub = None;
    }

    /// Receipt: the initial directory check passed; only now show the unsubscribe confirmation window.
    pub(crate) fn on_unsubscribe_check_ready(&mut self, dir: &std::path::Path) {
        let Some(checking) = self.overlays.checking.as_ref() else {
            return;
        };
        let expected = storage::paths::subgroup_download_dir(
            &self.state.download_dir(),
            &checking.bangumi_name,
            &checking.group_name,
        );
        if expected == dir {
            let checking = self.overlays.checking.take().unwrap();
            self.overlays.confirmation = Some(checking);
        }
    }

    /// Receipt: directory cleanup complete → remove the subscription record and filter keyword.
    pub(crate) fn on_unsubscribe_done(&mut self, dir: &std::path::Path) {
        let Some(pending) = &self.overlays.pending_unsub else {
            return;
        };
        let expected = storage::paths::subgroup_download_dir(
            &self.state.download_dir(),
            &pending.bangumi_name,
            &pending.group_name,
        );
        if expected != dir {
            return;
        }
        let (bangumi_id, subgroup_id) = (pending.bangumi_id, pending.subgroup_id);
        self.overlays.pending_unsub = None;
        self.finish_unsubscribe(bangumi_id, subgroup_id);
    }

    /// Remove the subscription record and filter keyword and persist them (the wrap-up after directory cleanup completes).
    fn finish_unsubscribe(&mut self, bangumi_id: BangumiId, subgroup_id: SubgroupId) {
        self.data
            .subscriptions
            .retain(|s| !(s.bangumi_id == bangumi_id && s.subgroup_id == subgroup_id));
        // Also clear this entry's filter keyword when unsubscribing (no leftover in persisted data)
        self.data
            .keywords
            .remove(&SubgroupRef::new(bangumi_id, subgroup_id));
        self.state.set_subgroup_keywords(&self.data.keywords);
        self.state.set_subscriptions(&self.data.subscriptions);
    }

    /// Titles of active download tasks located in a given directory in the current snapshot (fallback when the download engine is unavailable).
    fn active_titles_in(&self, dir: &std::path::Path) -> Vec<String> {
        self.downloader
            .snapshot()
            .into_iter()
            .filter(|task| task.output_dir.as_deref() == Some(dir) && task.state.is_active())
            .map(|task| task.title)
            .collect()
    }
}
