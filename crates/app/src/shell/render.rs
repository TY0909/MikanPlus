//! Root view rendering for `MikanPlus`: dispatches to page components by the current page and layers global modals on top.

use gpui_kit::component::theme::Theme;

use crate::load::episodes::episodes_view;
use crate::prelude::*;
use crate::shell::state::MikanPlus;
use domain::{BangumiId, SubgroupId, SubgroupRef};

use crate::shell::views::{
    error_view, loading_view, render_filter_modal, render_unsubscribe_confirmation,
    render_unsubscribe_warning, scroll_page,
};

impl Render for MikanPlus {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_toolbar(cx);
        // Settle results posted by background threads (idempotent)
        self.settle_updates();

        let theme = cx.theme().clone();
        let scroll_handle = {
            let key = self.scroll_key();
            self.scroll_handle(key)
        };
        let content = self.render_page(&theme, &scroll_handle, cx);

        // Floating modal layers (overlaid on the content)
        let filter_modal = self.filter_modal(&theme, cx);
        let warning_modal = self.warning_modal(&theme, cx);
        let confirmation_modal = self.confirmation_modal(&theme, cx);

        // ---- Layout ----
        //
        // Important: in gpui 0.2.2 (the crates.io release), flex main-axis
        // grow and cross-axis stretch are unreliable in the vertical direction
        // (height degenerates to 0 or content height), so everything here uses
        // "absolute positioning + explicit size": the scroll container's height
        // is fully determined, so content can overflow and scroll.
        gpui_kit::div()
            .id("mikan-root")
            .size_full()
            .relative()
            .bg(theme.background)
            .child(
                // Toolbar: fixed at the top
                gpui_kit::div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(48.))
                    .child(self.toolbar.clone()),
            )
            .child(
                // Content mount point: below the toolbar, with a determined height (viewport - 48)
                gpui_kit::div()
                    .absolute()
                    .top(px(48.))
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .flex()
                    .flex_row()
                    .justify_center()
                    .child(content),
            )
            // Filter window (centered floating, overlaid on the content layer)
            .when_some(filter_modal, |this, modal| this.child(modal))
            // Unsubscribe warning window (centered floating, overlaid on the content layer)
            .when_some(warning_modal, |this, modal| this.child(modal))
            // Unsubscribe confirmation window (centered floating, overlaid on the content layer)
            .when_some(confirmation_modal, |this, modal| this.child(modal))
            // In-app notification layer (pops from the top-right; Root doesn't render it automatically, so mount it manually)
            .when_some(
                gpui_kit::component::Root::render_notification_layer(window, cx),
                |this, layer| this.child(layer),
            )
    }
}

impl MikanPlus {
    /// Push the current navigation / download state into the resident toolbar.
    fn sync_toolbar(&self, cx: &mut Context<Self>) {
        let can_back = self.can_go_back();
        let title = self.page_title();
        let current_section = self.current_section();
        let download_count = self
            .downloader
            .snapshot()
            .iter()
            .filter(|task| task.state.is_active())
            .count();
        self.toolbar.update(cx, |toolbar, cx| {
            let changed = toolbar.can_go_back != can_back
                || toolbar.title != title
                || toolbar.current_section != current_section
                || toolbar.download_count != download_count;
            if changed {
                toolbar.can_go_back = can_back;
                toolbar.title = title.clone();
                toolbar.current_section = current_section;
                toolbar.download_count = download_count;
                cx.notify();
            }
        });
    }

    /// Dispatch to the component for the current page.
    fn render_page(
        &mut self,
        theme: &Theme,
        scroll_handle: &ScrollHandle,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        match self.page.clone() {
            Page::Home(filter) => self.home_page(filter, theme, scroll_handle, cx),
            Page::Subscription => self.subscription_page(scroll_handle, cx),
            Page::Settings => self.settings_page(cx),
            Page::DownloadObserver => self.download_observer_page(scroll_handle),
            Page::DownloadCollection(task_id) => {
                self.download_collection_page(&task_id, theme, scroll_handle)
            }
            Page::BangumiDetail { bangumi_id, name } => {
                self.bangumi_detail_page(bangumi_id, name, theme, scroll_handle, cx)
            }
            Page::SubGroupDetail(key) => self.subgroup_detail_page(key, theme, scroll_handle, cx),
            Page::SearchResult(query) => self.search_result_page(query, theme, scroll_handle, cx),
        }
    }

    /// Home page: the weekday / today / movie list.
    fn home_page(
        &self,
        filter: HomeFilter,
        theme: &Theme,
        scroll_handle: &ScrollHandle,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let groups = self.data.home.ready().cloned();
        let failed = self.data.home.failed().cloned();
        if let Some(groups) = groups {
            // Home filter bar callback (weekday / today / movie switching)
            let on_filter_change: FilterChangeCallback = {
                let entity = cx.entity().clone();
                Rc::new(move |filter: HomeFilter, _window, app: &mut App| {
                    entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                        mp.navigate_to(Page::Home(filter), cx);
                    });
                })
            };
            let home = HomeView {
                groups,
                filter,
                today_weekday: today_weekday(),
                network: self.network.clone(),
                on_card_click: self.on_card_click.clone(),
                on_filter_change,
            };
            scroll_page(home, scroll_handle)
        } else if let Some(error) = failed {
            self.retry_error_view(&error, theme, cx, |mp, cx| mp.load_home(cx))
        } else {
            loading_view(theme).into_any_element()
        }
    }

    /// Subscription page (subscribed bangumi + subtitle groups).
    fn subscription_page(
        &self,
        scroll_handle: &ScrollHandle,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let on_sub_click: SubCardClickCallback = {
            let entity = cx.entity().clone();
            Rc::new(move |bangumi_id, subgroup_id, _window, app| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.navigate_to(
                        Page::SubGroupDetail(SubgroupRef::new(bangumi_id, subgroup_id)),
                        cx,
                    );
                });
            })
        };
        let on_unsubscribe: UnsubscribeCallback = {
            let entity = cx.entity().clone();
            Rc::new(move |bangumi_id, subgroup_id, _window, app| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.request_unsubscribe(bangumi_id, subgroup_id, cx);
                });
            })
        };
        let on_go_home: GoBackCallback = {
            let entity = cx.entity().clone();
            Rc::new(move |_window, app| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.navigate_to(Page::Home(HomeFilter::Today), cx);
                });
            })
        };
        let page = SubscriptionPage {
            subscriptions: self.data.resolved_subscriptions(),
            scroll_handle: scroll_handle.clone(),
            network: self.network.clone(),
            on_sub_click,
            on_unsubscribe,
            on_go_home,
        };
        page.into_any_element()
    }

    /// Settings page: push the current values into the resident page (this model owns the state).
    fn settings_page(&mut self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let download_dir = self.state.download_dir();
        let use_backup_domain = self.state.use_backup_domain();
        let remove_downloads_on_unsubscribe = self.state.remove_downloads_on_unsubscribe();
        self.settings.update(cx, |settings, cx| {
            let changed = settings.download_dir != download_dir
                || settings.use_backup_domain != use_backup_domain
                || settings.remove_downloads_on_unsubscribe != remove_downloads_on_unsubscribe;
            if changed {
                settings.download_dir = download_dir;
                settings.use_backup_domain = use_backup_domain;
                settings.remove_downloads_on_unsubscribe = remove_downloads_on_unsubscribe;
                cx.notify();
            }
        });
        self.settings.clone().into_any_element()
    }

    /// Download observer page (in-progress / completed tasks).
    fn download_observer_page(&self, scroll_handle: &ScrollHandle) -> gpui_kit::AnyElement {
        let page = DownloadObserverPage {
            tasks: self.downloader.snapshot(),
            downloader: self.downloader.clone(),
        };
        scroll_page(page, scroll_handle)
    }

    /// Download collection page (file list of a completed multi-file task).
    fn download_collection_page(
        &self,
        task_id: &str,
        theme: &Theme,
        scroll_handle: &ScrollHandle,
    ) -> gpui_kit::AnyElement {
        let task = self
            .downloader
            .snapshot()
            .into_iter()
            .find(|task| task.id == task_id);
        if let Some(task) = task {
            let page = DownloadCollectionPage {
                title: task.title,
                files: task.video_files,
                output_dir: task.output_dir,
            };
            scroll_page(page, scroll_handle)
        } else {
            gpui_kit::div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child("下载任务不存在或已被删除")
                .into_any_element()
        }
    }

    /// Bangumi detail page (hero header + subtitle-group list).
    fn bangumi_detail_page(
        &mut self,
        bangumi_id: BangumiId,
        name: String,
        theme: &Theme,
        scroll_handle: &ScrollHandle,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let loaded = self
            .data
            .details
            .get(&bangumi_id)
            .and_then(|s| s.ready())
            .cloned();
        let failed = self
            .data
            .details
            .get(&bangumi_id)
            .and_then(|s| s.failed())
            .cloned();
        // Fallback load trigger (first render before navigate); keep the error visible when already in a failed state
        if loaded.is_none() {
            self.load_detail(bangumi_id, name.clone(), cx);
        }
        if let Some(item) = loaded {
            let bid = item.bangumi_id;
            // Subtitle groups expanded for this bangumi + each one's episode state (mapped to a UI view)
            let expanded_groups: HashSet<SubgroupId> = self
                .data
                .expanded
                .iter()
                .filter(|key| key.bangumi == bid)
                .map(|key| key.subgroup)
                .collect();
            let group_episodes: HashMap<SubgroupId, GroupEpisodesState> = self
                .data
                .episodes
                .iter()
                .filter(|(key, _)| key.bangumi == bid)
                .map(|(key, state)| (key.subgroup, episodes_view(state)))
                .collect();
            let on_toggle_group: ui::ToggleGroupCallback = {
                let entity = cx.entity().clone();
                Rc::new(move |bangumi_id, subgroup_id, _window, app| {
                    entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                        mp.toggle_group(SubgroupRef::new(bangumi_id, subgroup_id), cx);
                    });
                })
            };
            let on_reload_group: ui::ToggleGroupCallback = {
                let entity = cx.entity().clone();
                Rc::new(move |bangumi_id, subgroup_id, _window, app| {
                    entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                        mp.reload_episodes(SubgroupRef::new(bangumi_id, subgroup_id), cx);
                    });
                })
            };
            // Subscription check (per subtitle group)
            let is_subscribed: SubscribedChecker = {
                let subs = self.data.subscriptions.clone();
                Rc::new(move |bangumi_id: BangumiId, subgroup_id: SubgroupId| {
                    subs.iter()
                        .any(|s| s.bangumi_id == bangumi_id && s.subgroup_id == subgroup_id)
                })
            };
            let detail = BangumiDetailPage {
                item,
                network: self.network.clone(),
                is_subscribed,
                on_toggle_subscribe: self.on_toggle_subscribe.clone(),
                scroll_handle: scroll_handle.clone(),
                downloader: self.downloader.clone(),
                on_open_collection: self.on_open_collection.clone(),
                download_base: self.state.download_dir(),
                expanded_groups,
                group_episodes,
                on_toggle_group,
                on_reload_group,
            };
            detail.into_any_element()
        } else if let Some(error) = failed {
            self.retry_error_view(&error, theme, cx, move |mp, cx| {
                mp.retry_detail(bangumi_id, name.clone(), cx)
            })
        } else {
            loading_view(theme).into_any_element()
        }
    }

    /// Subtitle-group detail page (episode list of one offering, with its own filter).
    fn subgroup_detail_page(
        &mut self,
        key: SubgroupRef,
        theme: &Theme,
        scroll_handle: &ScrollHandle,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        // Lazy episode loading: fetch on demand on page entry (reused on a hit)
        self.load_episodes(key, cx);
        let episodes_state = self
            .data
            .episodes
            .get(&key)
            .map(episodes_view)
            .unwrap_or(GroupEpisodesState::Loading);
        let on_reload_episodes: ReloadEpisodesCallback = {
            let entity = cx.entity().clone();
            Rc::new(move |_window, app| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.reload_episodes(key, cx);
                });
            })
        };
        // Episode filter keyword (remembered per subscription entry) and the open-filter callback
        let keyword = self.data.keywords.get(&key).cloned().unwrap_or_default();
        let on_open_filter: OpenFilterCallback = {
            let entity = cx.entity().clone();
            Rc::new(move |_window, app| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.open_filter(key, cx);
                });
            })
        };
        // Locate the name by bangumi id while the detail isn't ready yet; capture the failed state before
        // triggering a reload, so the "error view" isn't overwritten by the Loading the reload writes back.
        let name = self.data.name_for(key.bangumi);
        let failed = self
            .data
            .details
            .get(&key.bangumi)
            .and_then(|s| s.failed())
            .cloned();
        let has_group = self
            .data
            .detail_with_id(key.bangumi)
            .as_ref()
            .is_some_and(|item| {
                item.subtitle_groups
                    .iter()
                    .any(|g| g.subgroup_id == key.subgroup)
            });
        if !has_group && let Some(name) = &name {
            self.load_detail(key.bangumi, name.clone(), cx);
        }
        let item = self.data.detail_with_id(key.bangumi);
        let group = item
            .as_ref()
            .and_then(|item| {
                item.subtitle_groups
                    .iter()
                    .find(|g| g.subgroup_id == key.subgroup)
            })
            .cloned();
        if let Some(group) = group {
            let page = SubGroupDetailPage {
                bangumi_name: item.map(|item| item.name).unwrap_or_default(),
                group,
                scroll_handle: scroll_handle.clone(),
                downloader: self.downloader.clone(),
                on_open_collection: self.on_open_collection.clone(),
                download_base: self.state.download_dir(),
                keyword: keyword.clone(),
                on_open_filter: on_open_filter.clone(),
                episodes_state: episodes_state.clone(),
                on_reload_episodes: on_reload_episodes.clone(),
            };
            page.into_any_element()
        } else if let Some(error) = failed
            && let Some(name) = name
        {
            // This bangumi failed to load: the error is attributed by name, and retry only retries the current bangumi
            self.retry_error_view(&error, theme, cx, move |mp, cx| {
                mp.retry_detail(key.bangumi, name.clone(), cx)
            })
        } else {
            loading_view(theme).into_any_element()
        }
    }

    /// Search result page (online search results + pagination).
    fn search_result_page(
        &mut self,
        query: String,
        theme: &Theme,
        scroll_handle: &ScrollHandle,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let loaded = self
            .data
            .searches
            .get(&query)
            .and_then(|search| search.results.ready())
            .cloned();
        let page_index = self.data.searches.get(&query).map(|s| s.page).unwrap_or(0);
        let failed = self
            .data
            .searches
            .get(&query)
            .and_then(|search| search.results.failed())
            .cloned();
        // Fallback online-search trigger (first render before navigate); an empty query only shows the search UI.
        // Capture the failed state before triggering a reload, so the error view isn't overwritten by Loading.
        if !query.trim().is_empty() && loaded.is_none() {
            self.load_search(query.clone(), cx);
        }
        let on_search: ui::pages::search_result_page::SearchCallback = {
            let entity = cx.entity().clone();
            Rc::new(move |query, _window, app: &mut App| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.navigate_to(Page::SearchResult(query), cx);
                });
            })
        };
        if let Some(results) = loaded {
            let on_page_change: ui::pages::search_result_page::SearchPageChangeCallback = {
                let entity = cx.entity().clone();
                let query = query.clone();
                Rc::new(move |page, _window, app| {
                    entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                        if let Some(search) = mp.data.searches.get_mut(&query) {
                            search.page = page;
                        }
                        cx.notify();
                    });
                })
            };
            let page = SearchResultPage {
                query: query.clone(),
                input_state: self.search_input.clone(),
                on_search: on_search.clone(),
                results,
                network: self.network.clone(),
                on_card_click: self.on_card_click.clone(),
                on_open_collection: self.on_open_collection.clone(),
                downloader: self.downloader.clone(),
                download_dir: self.state.download_dir(),
                page: page_index,
                on_page_change,
            };
            scroll_page(page, scroll_handle)
        } else if query.trim().is_empty() {
            let page = SearchResultPage {
                query,
                input_state: self.search_input.clone(),
                on_search,
                results: SearchResults::default(),
                network: self.network.clone(),
                on_card_click: self.on_card_click.clone(),
                on_open_collection: self.on_open_collection.clone(),
                downloader: self.downloader.clone(),
                download_dir: self.state.download_dir(),
                page: 0,
                on_page_change: Rc::new(|_, _, _| {}),
            };
            scroll_page(page, scroll_handle)
        } else if let Some(error) = failed {
            self.retry_error_view(&error, theme, cx, move |mp, cx| {
                mp.load_search(query.clone(), cx)
            })
        } else {
            loading_view(theme).into_any_element()
        }
    }

    /// A load-failure view with a retry action that runs `retry` on this model.
    fn retry_error_view(
        &self,
        error: &SourceError,
        theme: &Theme,
        cx: &mut Context<Self>,
        retry: impl Fn(&mut MikanPlus, &mut Context<MikanPlus>) + 'static,
    ) -> gpui_kit::AnyElement {
        let entity = cx.entity().clone();
        let on_retry: GoBackCallback = Rc::new(move |_window, app: &mut App| {
            entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                retry(mp, cx)
            });
        });
        error_view(error, on_retry, theme).into_any_element()
    }

    /// Floating filter-window layer (only when a filter window is open).
    fn filter_modal(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<gpui_kit::AnyElement> {
        let key = self.overlays.filter?;
        let keyword = self.data.keywords.get(&key).cloned().unwrap_or_default();
        let on_clear: GoBackCallback = {
            let entity = cx.entity().clone();
            Rc::new(move |_window, app| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.clear_filter(cx);
                });
            })
        };
        let on_cancel: GoBackCallback = {
            let entity = cx.entity().clone();
            Rc::new(move |_window, app| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.close_filter(cx);
                });
            })
        };
        let on_confirm: GoBackCallback = {
            let entity = cx.entity().clone();
            Rc::new(move |_window, app| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.apply_filter(cx);
                });
            })
        };
        Some(
            render_filter_modal(
                theme,
                self.filter_input.clone(),
                &keyword,
                on_clear,
                on_cancel,
                on_confirm,
            )
            .into_any_element(),
        )
    }

    /// Floating unsubscribe-blocked warning layer.
    fn warning_modal(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<gpui_kit::AnyElement> {
        let warning = self.overlays.warning.clone()?;
        let on_close: GoBackCallback = {
            let entity = cx.entity().clone();
            Rc::new(move |_window, app| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.close_warning(cx);
                });
            })
        };
        Some(render_unsubscribe_warning(theme, &warning, on_close).into_any_element())
    }

    /// Floating unsubscribe-confirmation layer.
    fn confirmation_modal(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<gpui_kit::AnyElement> {
        let confirmation = self.overlays.confirmation.clone()?;
        let on_cancel: GoBackCallback = {
            let entity = cx.entity().clone();
            Rc::new(move |_window, app| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.close_confirmation(cx);
                });
            })
        };
        let on_confirm: UnsubscribeConfirmCallback = {
            let entity = cx.entity().clone();
            Rc::new(move |remove_downloads, _window, app| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.confirm_unsubscribe(remove_downloads, cx);
                });
            })
        };
        let on_toggle_remove: UnsubscribeConfirmCallback = {
            let entity = cx.entity().clone();
            Rc::new(move |remove_downloads, _window, app| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.set_remove_downloads(remove_downloads, cx);
                });
            })
        };
        Some(
            render_unsubscribe_confirmation(
                theme,
                &confirmation,
                on_cancel,
                on_confirm,
                on_toggle_remove,
            )
            .into_any_element(),
        )
    }
}
