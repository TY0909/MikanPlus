//! Construction and startup for `MikanPlus`: state initialization, one-time migration,
//! and restoring subscriptions and download tasks.

use std::sync::Arc;

use crate::data::model::{AppData, CACHE_LIMIT, Cache, Overlays, load_version};
use crate::prelude::*;
use crate::shell::state::MikanPlus;

impl MikanPlus {
    pub(crate) fn new(
        window: &mut Window,
        cx: &mut App,
        state: Rc<storage::State>,
    ) -> Entity<Self> {
        cx.new(|cx: &mut Context<Self>| {
            let entity = cx.entity().clone();
            // The download engine is created up front so the polling loop can share its handle.
            let downloader = DownloadManager::start();
            let network = std::sync::Arc::new(source::Network::new());
            spawn_startup_tasks(&state, &network);
            spawn_poll_loop(&downloader, &network, cx);

            let shell = shell_callbacks(&entity);
            let toolbar = build_toolbar(&entity, &state, cx);
            let settings = build_settings(&entity, &state, &network, window, cx);
            let filter_input = build_filter_input(window, cx);
            let search_input = build_search_input(window, cx);

            let mut this = Self {
                page: Page::Home(HomeFilter::Today),
                section: Section::Home,
                stacks: HashMap::from([(Section::Home, vec![Page::Home(HomeFilter::Today)])]),
                window_handle: window.window_handle(),
                data: AppData::new(&state),
                overlays: Overlays::default(),
                scroll: Cache::new(CACHE_LIMIT),
                toolbar,
                settings,
                filter_input,
                search_input,
                downloader,
                network,
                state,
                on_card_click: shell.card_click,
                on_open_collection: shell.open_collection,
                on_toggle_subscribe: shell.toggle_subscribe,
                #[cfg(target_os = "macos")]
                fullscreen: false,
            };
            // Start the home load (zero requests on a cache hit)
            this.load_home(cx);
            this
        })
    }
}

/// One-time startup tasks: restore the data source, then run the cache migration and image-cache
/// governance on background threads so they don't block the first frame.
fn spawn_startup_tasks(state: &Rc<storage::State>, network: &Arc<source::Network>) {
    // Restore the last-selected data source (backup domain switch); must take effect before any network request
    network.set_backup_domain(state.use_backup_domain());

    // One-time startup migration (old cache v2→v3, DHT moved into the data directory), idempotent.
    // Runs on a background thread: it deletes directories, so it must not block the first frame.
    std::thread::spawn(|| {
        storage::migrate::run_all();
    });

    // Image cache capacity governance (LRU, background thread, doesn't block the UI)
    std::thread::spawn(|| {
        storage::cache::enforce_image_cache_limit();
    });
}

/// The polling loop that reacts to background changes: refresh the UI when the image cache /
/// background loads / download snapshot change, and settle download events.
fn spawn_poll_loop(
    downloader: &Arc<DownloadManager>,
    network: &Arc<source::Network>,
    cx: &mut Context<MikanPlus>,
) {
    // Poll cover-image cache, background-load, and download-snapshot versions: refresh the UI on change;
    // also consume download background events (failure notifications / unsubscribe receipts).
    let downloader = downloader.clone();
    let network = network.clone();
    cx.spawn(
        move |this: gpui_kit::WeakEntity<MikanPlus>, cx: &mut gpui_kit::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let mut last_img = network.image_version();
                let mut last_load = load_version();
                let mut last_dl = downloader.version();
                loop {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(500))
                        .await;
                    let Some(entity) = this.upgrade() else {
                        break;
                    };
                    // Sync menu labels on fullscreen state change ("Enter Full Screen" / "Exit Full Screen")
                    #[cfg(target_os = "macos")]
                    entity.update(&mut cx, |mp, cx| {
                        // Copy the (Copy) handle out first so it doesn't borrow `mp`, which the method needs as `&mut self`.
                        let handle = mp.window_handle;
                        let _ =
                            handle.update(cx, |_, window, cx| mp.sync_fullscreen_menu(window, cx));
                    });
                    let iv = network.image_version();
                    let lv = load_version();
                    let dv = downloader.version();
                    if iv != last_img || lv != last_load || dv != last_dl {
                        last_img = iv;
                        last_load = lv;
                        last_dl = dv;
                        entity.update(&mut cx, |_, cx| cx.notify());
                    }
                    // Consume download background events → in-app notifications / unsubscribe receipts
                    let events = downloader.take_events();
                    if !events.is_empty() {
                        entity.update(&mut cx, |mp, cx| {
                            for event in events {
                                handle_download_event(mp, cx, event);
                            }
                        });
                    }
                }
            }
        },
    )
    .detach();
}

/// Settle one background download event into a UI notification or an unsubscribe receipt.
fn handle_download_event(
    this: &mut MikanPlus,
    cx: &mut Context<MikanPlus>,
    event: downloader::DownloadEvent,
) {
    match event {
        downloader::DownloadEvent::AddFailed { title, error } => {
            this.notify_error(
                format!(
                    "「{title}」下载失败:{} —— {}",
                    error.user_message(),
                    error.user_hint()
                ),
                cx,
            );
        }
        downloader::DownloadEvent::EngineFailed { error } => {
            this.notify_error(
                format!("{} —— {}", error.user_message(), error.user_hint()),
                cx,
            );
        }
        downloader::DownloadEvent::DownloadCompleted { title } => {
            this.notify_success(format!("「{title}」已下载完成"), cx);
        }
        downloader::DownloadEvent::UnsubscribeBlocked { dir, titles } => {
            this.on_unsubscribe_blocked(&dir, titles);
            cx.notify();
        }
        downloader::DownloadEvent::UnsubscribeCheckReady { dir } => {
            this.on_unsubscribe_check_ready(&dir);
            cx.notify();
        }
        downloader::DownloadEvent::UnsubscribeDone { dir } => {
            this.on_unsubscribe_done(&dir);
            cx.notify();
        }
    }
}

/// Shell-level callbacks: navigation targets that no single component owns.
struct ShellCallbacks {
    card_click: CardClickCallback,
    open_collection: ui::components::episode_row::OpenCollectionCallback,
    toggle_subscribe: ToggleSubscribeCallback,
}

/// Build the shell-level callbacks (card click, collection open, subscription toggle).
fn shell_callbacks(entity: &Entity<MikanPlus>) -> ShellCallbacks {
    // Card click → bangumi detail
    let card_click: CardClickCallback = {
        let entity = entity.clone();
        Rc::new(move |name, bangumi_id, _window, app: &mut App| {
            entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                mp.navigate_to(
                    Page::BangumiDetail {
                        bangumi_id,
                        name: name.to_string(),
                    },
                    cx,
                );
            });
        })
    };

    // Completed multi-video download task → collection page
    let open_collection: ui::components::episode_row::OpenCollectionCallback = {
        let entity = entity.clone();
        Rc::new(move |task_id, _window, app: &mut App| {
            entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                mp.navigate_to(Page::DownloadCollection(task_id), cx);
            });
        })
    };

    // "Subscribe" on the detail / subtitle-group page → toggle a single subtitle group's subscription
    let toggle_subscribe: ToggleSubscribeCallback = {
        let entity = entity.clone();
        Rc::new(
            move |bangumi_id, subgroup_id, bangumi_name, group_name, _window, app: &mut App| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.toggle_subscription(bangumi_id, subgroup_id, bangumi_name, group_name, cx);
                    cx.notify();
                });
            },
        )
    };

    ShellCallbacks {
        card_click,
        open_collection,
        toggle_subscribe,
    }
}

/// Build the toolbar and its navigation / action callbacks.
fn build_toolbar(
    entity: &Entity<MikanPlus>,
    state: &Rc<storage::State>,
    cx: &mut Context<MikanPlus>,
) -> Entity<Toolbar> {
    // The top bar only opens the search page; actual keyword input happens on the search page
    let on_open_search: ActionCallback = {
        let entity = entity.clone();
        Rc::new(move |window, app: &mut App| {
            let window_handle = window.window_handle();
            let input = entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                mp.navigate_to(Page::SearchResult(String::new()), cx);
                mp.search_input.clone()
            });
            let _ = window_handle.update(app, move |_, window, cx| {
                input.update(cx, |state, cx| {
                    state.set_value(String::new(), window, cx);
                    state.focus(window, cx);
                });
            });
        })
    };

    let on_go_back: ActionCallback = {
        let entity = entity.clone();
        Rc::new(move |_window, app: &mut App| {
            entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                mp.go_back(cx);
            });
        })
    };

    let on_toggle_theme: ActionCallback = {
        let state = state.clone();
        Rc::new(move |window, app: &mut App| {
            let dark = gpui_kit::component::theme::Theme::global(app).mode
                == gpui_kit::component::theme::ThemeMode::Dark;
            let mode = if dark {
                gpui_kit::component::theme::ThemeMode::Light
            } else {
                gpui_kit::component::theme::ThemeMode::Dark
            };
            state.set_theme_mode(mode.name());
            app_theme::apply_mode(mode, Some(window), app);
        })
    };

    // Toolbar segmented navigation: switch to a section, restoring its own position
    let on_navigate: NavigateCallback = {
        let entity = entity.clone();
        Rc::new(move |section: Section, _window, app: &mut App| {
            entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                mp.switch_section(section, cx);
            });
        })
    };

    let on_open_downloads: ActionCallback = {
        let entity = entity.clone();
        Rc::new(move |_window, app: &mut App| {
            entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                mp.switch_section(Section::Download, cx);
            });
        })
    };

    cx.new(|cx| {
        Toolbar::new(
            on_open_search,
            on_go_back,
            on_toggle_theme,
            on_navigate,
            on_open_downloads,
            cx,
        )
    })
}

/// Build the resident settings page and its write callbacks (the app model owns the state).
fn build_settings(
    entity: &Entity<MikanPlus>,
    state: &Rc<storage::State>,
    network: &Arc<source::Network>,
    window: &mut Window,
    cx: &mut Context<MikanPlus>,
) -> Entity<SettingsPage> {
    let on_set_download_dir: ui::pages::settings_page::SetDownloadDirCallback = {
        let entity = entity.clone();
        Rc::new(move |dir: std::path::PathBuf, app: &mut App| {
            entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                mp.state.set_download_dir(&dir);
                cx.notify();
            });
        })
    };
    let on_toggle_backup: ui::pages::settings_page::ToggleSettingCallback = {
        let entity = entity.clone();
        let network = network.clone();
        Rc::new(move |enabled: bool, app: &mut App| {
            network.set_backup_domain(enabled);
            entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                mp.state.set_use_backup_domain(enabled);
                mp.reload_failed(cx);
                cx.notify();
            });
        })
    };
    let on_toggle_cleanup: ui::pages::settings_page::ToggleSettingCallback = {
        let entity = entity.clone();
        Rc::new(move |enabled: bool, app: &mut App| {
            entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                mp.state.set_remove_downloads_on_unsubscribe(enabled);
                cx.notify();
            });
        })
    };
    let on_select_theme: ui::pages::settings_page::SelectThemeCallback = {
        let entity = entity.clone();
        Rc::new(
            move |mode: gpui_kit::component::theme::ThemeMode,
                  window: &mut Window,
                  app: &mut App| {
                entity.update(app, |mp: &mut MikanPlus, _cx| {
                    mp.state.set_theme_mode(mode.name());
                });
                app_theme::apply_mode(mode, Some(window), app);
            },
        )
    };

    cx.new(|cx| {
        SettingsPage::new(
            window,
            cx,
            state.download_dir(),
            on_set_download_dir,
            on_toggle_backup,
            on_toggle_cleanup,
            on_select_theme,
        )
    })
}

/// Build the resident filter-keyword input and apply the filter on Enter.
fn build_filter_input(window: &mut Window, cx: &mut Context<MikanPlus>) -> Entity<InputState> {
    let input = cx.new(|cx| InputState::new(window, cx).placeholder("输入标题必须包含的关键词…"));
    cx.subscribe(
        &input,
        move |this: &mut MikanPlus,
              _input: Entity<InputState>,
              event: &InputEvent,
              cx: &mut Context<MikanPlus>| {
            if let InputEvent::PressEnter { .. } = event
                && this.overlays.filter.is_some()
            {
                this.apply_filter(cx);
            }
        },
    )
    .detach();
    input
}

/// Build the resident search-keyword input and submit the search on Enter.
fn build_search_input(window: &mut Window, cx: &mut Context<MikanPlus>) -> Entity<InputState> {
    let input = cx.new(|cx| InputState::new(window, cx).placeholder("输入番剧名称…"));
    cx.subscribe(
        &input,
        move |this: &mut MikanPlus,
              _input: Entity<InputState>,
              event: &InputEvent,
              cx: &mut Context<MikanPlus>| {
            if let InputEvent::PressEnter { .. } = event {
                this.submit_search(cx);
            }
        },
    )
    .detach();
    input
}
