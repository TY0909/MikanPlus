// Windows 发布版走 GUI 子系统:双击启动时不再弹出终端窗口。
// debug 构建保留控制台,便于开发时查看日志。
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod menu;

use std::borrow::Cow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::StyledExt;
use gpui_kit::component::WindowExt;
use gpui_kit::{
    App, AssetSource, Bounds, Context, Entity, ScrollHandle, SharedString, Window, WindowBounds,
    WindowOptions, prelude::*, px,
};

use crate::menu::build_menus;
use domain::navigation::{HomeFilter, Page, TopSection};
use domain::{BangumiGroup, BangumiItem, SearchResults, Subscription};
use downloader::{DownloadCmd, DownloadManager};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use source::SourceError;
use storage::{
    load_subgroup_keywords, load_subscriptions, save_subgroup_keywords, save_subscriptions,
};
use ui::actions::*;
use ui::app_theme;
use ui::home_view::{FilterChangeCallback, HomeView, today_weekday};
use ui::subgroup_detail_page::OpenFilterCallback;
use ui::subscription_page::{SubCardClickCallback, UnsubscribeCallback};
use ui::toolbar::{ActionCallback, NavigateCallback, SearchCallback, Toolbar};
use ui::{BangumiDetailPage, SearchResultPage, SettingsPage, SubGroupDetailPage, SubscriptionPage};

pub type GoBackCallback = Rc<dyn Fn(&mut Window, &mut App)>;
pub type CardClickCallback = Rc<dyn Fn(&str, Option<u32>, &mut Window, &mut App)>;
pub type ToggleSubscribeCallback =
    Rc<dyn Fn(u32, u32, Option<&str>, Option<&str>, &mut Window, &mut App)>;
/// 检查某个 (番剧, 字幕组) 是否已订阅
pub type SubscribedChecker = Rc<dyn Fn(u32, u32) -> bool>;

/// 退订被拦截时的警告窗口内容(该订阅有正在下载的剧集)
#[derive(Clone)]
struct UnsubscribeWarning {
    bangumi_name: String,
    group_name: String,
    /// 正在下载的剧集标题
    active_titles: Vec<String>,
}

/// 注册应用级键盘快捷键。
fn register_keybindings(cx: &mut App) {
    use gpui_kit::KeyBinding;
    cx.bind_keys([
        KeyBinding::new("cmd-1", GoHome, None),
        KeyBinding::new("cmd-2", GoSubscription, None),
        KeyBinding::new("cmd-4", GoMonday, None),
        KeyBinding::new("cmd-5", GoTuesday, None),
        KeyBinding::new("cmd-6", GoWednesday, None),
        KeyBinding::new("cmd-7", GoThursday, None),
        KeyBinding::new("cmd-8", GoFriday, None),
        KeyBinding::new("cmd-9", GoSaturday, None),
        KeyBinding::new("cmd-0", GoSunday, None),
        KeyBinding::new("cmd-shift-m", GoMovies, None),
        KeyBinding::new("cmd-,", GoSettings, None),
        KeyBinding::new("cmd-[", GoBack, None),
        KeyBinding::new("cmd-f", FocusSearch, None),
        KeyBinding::new("cmd-shift-l", ToggleTheme, None),
        KeyBinding::new("escape", CloseFilterModal, None),
        KeyBinding::new("cmd-q", QuitApp, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-m", MinimizeWindow, None),
        KeyBinding::new("cmd-ctrl-f", ToggleFullscreen, None),
        KeyBinding::new("cmd-h", HideApp, None),
        KeyBinding::new("cmd-alt-h", HideOthers, None),
    ]);
}

/// 列表页加载状态
#[derive(Clone, PartialEq)]
enum ListState {
    /// 尚未加载(初始态,允许启动加载)
    Idle,
    Loading,
    Ready,
    Error(SourceError),
}

/// 后台加载结果(由独立线程写入,主线程轮询消费)
static LIST_RESULT: Mutex<Option<Result<Vec<BangumiGroup>, SourceError>>> = Mutex::new(None);
/// 详情加载结果:bid → 结果(使用 OnceLock 惰性初始化)
static DETAIL_RESULT: OnceLock<Mutex<HashMap<u32, Result<BangumiItem, SourceError>>>> =
    OnceLock::new();
/// 搜索加载结果:query → 结果
static SEARCH_RESULT: OnceLock<Mutex<HashMap<String, Result<SearchResults, SourceError>>>> =
    OnceLock::new();
/// 后台加载完成信号:任何加载完成时递增,供轮询循环刷新 UI
static LOAD_VERSION: AtomicU64 = AtomicU64::new(0);

fn detail_result() -> &'static Mutex<HashMap<u32, Result<BangumiItem, SourceError>>> {
    DETAIL_RESULT.get_or_init(|| Mutex::new(HashMap::new()))
}

fn search_result() -> &'static Mutex<HashMap<String, Result<SearchResults, SourceError>>> {
    SEARCH_RESULT.get_or_init(|| Mutex::new(HashMap::new()))
}

struct MikanPlus {
    current_page: Page,
    history: Vec<Page>,
    bangumi_groups: Vec<BangumiGroup>,
    list_state: ListState,
    window_handle: gpui_kit::AnyWindowHandle,
    /// 已加载的番剧详情:name → 完整数据(含字幕组/剧集)
    details: HashMap<String, BangumiItem>,
    /// details 的插入顺序(LRU 近似淘汰用)
    detail_order: VecDeque<String>,
    /// 详情加载中:name
    detail_loading: HashSet<String>,
    /// 详情加载失败:name → 错误
    detail_error: HashMap<String, SourceError>,
    /// 详情加载的 bid → name 映射(消费结果时不再依赖列表查找)
    detail_names: HashMap<u32, String>,
    /// 在线搜索结果:query → 结果(番剧卡片 + 剧集列表)
    search_results: HashMap<String, SearchResults>,
    /// search_results 的插入顺序(LRU 近似淘汰用)
    search_order: VecDeque<String>,
    /// 搜索加载中:query
    search_loading: HashSet<String>,
    /// 搜索失败:query → 错误
    search_error: HashMap<String, SourceError>,
    /// 搜索结果分页页码:query → 页码(0 起,切换页面后保留)
    search_page: HashMap<String, usize>,
    subscriptions: Vec<Subscription>,
    toolbar: Entity<Toolbar>,
    settings: Entity<SettingsPage>,
    /// 各页面的滚动位置:页面键 → ScrollHandle(应用持有,页面切换不销毁)
    scroll_handles: HashMap<String, ScrollHandle>,
    /// scroll_handles 的插入顺序(LRU 近似淘汰用)
    scroll_order: VecDeque<String>,
    /// 订阅详情页的剧集筛选关键词:(番剧id, 字幕组id) → 关键词(空 = 不过滤)
    subgroup_keywords: HashMap<(u32, u32), String>,
    /// 当前打开的筛选窗口对应的订阅条目(None = 未打开)
    filter_modal: Option<(u32, u32)>,
    /// 退订被拦截时的警告窗口内容(None = 未打开)
    unsubscribe_warning: Option<UnsubscribeWarning>,
    /// 待处理的退订:(番剧id, 字幕组id, 番剧名, 字幕组名)——等待下载线程回执
    pending_unsub: Option<(u32, u32, String, String)>,
    /// 筛选窗口输入框状态(常驻,打开时预填当前关键词)
    filter_input: Entity<InputState>,
    downloader: std::sync::Arc<DownloadManager>,
    on_card_click: CardClickCallback,
    on_toggle_subscribe: ToggleSubscribeCallback,
}

/// 有界缓存的容量上限(详情/搜索结果/滚动位置)
const BOUNDED_CACHE_LIMIT: usize = 64;
/// 导航历史上限(返回栈)
const HISTORY_LIMIT: usize = 100;

/// 带容量上限的插入(LRU 近似:按插入顺序淘汰最旧)
fn bounded_insert<K: Clone + std::hash::Hash + Eq, V>(
    map: &mut HashMap<K, V>,
    order: &mut VecDeque<K>,
    key: K,
    value: V,
    limit: usize,
) {
    if !map.contains_key(&key) {
        order.push_back(key.clone());
        while order.len() > limit {
            if let Some(oldest) = order.pop_front() {
                map.remove(&oldest);
            }
        }
    }
    map.insert(key, value);
}

impl MikanPlus {
    fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx: &mut Context<Self>| {
            let entity = cx.entity().clone();

            // 恢复上次选择的数据源(备用域名开关),须在任何网络请求前生效
            source::network::set_backup_domain(storage::load_use_backup_domain());

            // 启动时的一次性迁移(旧缓存 v2→v3、DHT 迁入数据目录),幂等。
            // 后台线程执行:涉及目录删除,不阻塞首帧
            std::thread::spawn(|| {
                storage::migrate::run_all();
            });

            // 图片缓存容量治理(LRU,后台线程,不阻塞 UI)
            std::thread::spawn(|| {
                storage::cache::enforce_image_cache_limit();
            });

            // 轮询封面图片缓存、后台加载与下载快照版本:变化时刷新界面;
            // 同时消费下载后台事件(失败通知)。
            cx.spawn(
                move |this: gpui_kit::WeakEntity<MikanPlus>, cx: &mut gpui_kit::AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        let mut last_img = source::network::image_version();
                        let mut last_load = LOAD_VERSION.load(Ordering::Relaxed);
                        let mut last_dl = downloader::snapshot_version();
                        loop {
                            cx.background_executor()
                                .timer(std::time::Duration::from_millis(500))
                                .await;
                            let Some(entity) = this.upgrade() else {
                                break;
                            };
                            let iv = source::network::image_version();
                            let lv = LOAD_VERSION.load(Ordering::Relaxed);
                            let dv = downloader::snapshot_version();
                            if iv != last_img || lv != last_load || dv != last_dl {
                                last_img = iv;
                                last_load = lv;
                                last_dl = dv;
                                entity.update(&mut cx, |_, cx| cx.notify());
                            }
                            // 消费下载后台事件 → 应用内通知 / 退订回执
                            let events = downloader::take_events();
                            if !events.is_empty() {
                                entity.update(&mut cx, |mp, cx| {
                                            for e in events {
                                                match e {
                                                    downloader::DownloadEvent::AddFailed {
                                                        title,
                                                        error,
                                                    } => {
                                                        let _ = mp.window_handle.update(
                                                            cx,
                                                            |_, window, cx| {
                                                                window.push_notification(
                                                                    gpui_kit::component::notification::Notification::error(
                                                                        format!(
                                                                            "「{title}」下载失败:{} —— {}",
                                                                            error.user_message(),
                                                                            error.user_hint()
                                                                        ),
                                                                    ),
                                                                    cx,
                                                                );
                                                            },
                                                        );
                                                    }
                                                    downloader::DownloadEvent::EngineFailed {
                                                        error,
                                                    } => {
                                                        let _ = mp.window_handle.update(
                                                            cx,
                                                            |_, window, cx| {
                                                                window.push_notification(
                                                                    gpui_kit::component::notification::Notification::error(
                                                                        format!(
                                                                            "{} —— {}",
                                                                            error.user_message(),
                                                                            error.user_hint()
                                                                        ),
                                                                    ),
                                                                    cx,
                                                                );
                                                            },
                                                        );
                                                    }
                                                    downloader::DownloadEvent::UnsubscribeBlocked {
                                                        dir,
                                                        titles,
                                                    } => mp.handle_unsubscribe_blocked(&dir, titles),
                                                    downloader::DownloadEvent::UnsubscribeDone {
                                                        dir,
                                                    } => {
                                                        mp.handle_unsubscribe_done(&dir);
                                                        cx.notify();
                                                    }
                                                }
                                    }
                                });
                            }
                        }
                    }
                },
            )
            .detach();

            // 卡片点击 → 番剧详情
            let on_card_click: CardClickCallback = {
                let entity = entity.clone();
                Rc::new(move |name, _id, _window, app: &mut App| {
                    entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                        mp.navigate_to(Page::BangumiDetail(name.to_string()), cx);
                    });
                })
            };

            // 详情页 / 字幕组页「订阅」→ 切换单个字幕组的订阅(以字幕组为单位)
            let on_toggle_subscribe: ToggleSubscribeCallback = {
                let entity = entity.clone();
                Rc::new(
                    move |bangumi_id,
                          subgroup_id,
                          bangumi_name,
                          group_name,
                          _window,
                          app: &mut App| {
                        entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                            mp.toggle_subscription(
                                bangumi_id,
                                subgroup_id,
                                bangumi_name,
                                group_name,
                                cx,
                            );
                            cx.notify();
                        });
                    },
                )
            };

            // 工具栏回调
            let on_search: SearchCallback = {
                let entity = entity.clone();
                Rc::new(move |query, _window, app: &mut App| {
                    entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                        mp.navigate_to(Page::SearchResult(query), cx);
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
                Rc::new(move |window, app: &mut App| {
                    let dark = gpui_kit::component::theme::Theme::global(app).mode
                        == gpui_kit::component::theme::ThemeMode::Dark;
                    let mode = if dark {
                        gpui_kit::component::theme::ThemeMode::Light
                    } else {
                        gpui_kit::component::theme::ThemeMode::Dark
                    };
                    app_theme::set_mode(mode, Some(window), app);
                })
            };

            // 工具栏分段导航:首页 / 订阅 / 设置
            let on_navigate: NavigateCallback = {
                let entity = entity.clone();
                Rc::new(move |section: TopSection, _window, app: &mut App| {
                    let target = match section {
                        TopSection::Home => Page::Home(HomeFilter::Today),
                        TopSection::Subscription => Page::Subscription,
                        TopSection::Settings => Page::Settings,
                    };
                    entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                        mp.navigate_to(target, cx);
                    });
                })
            };

            let toolbar = cx.new(|cx| {
                Toolbar::new(
                    window,
                    on_search,
                    on_go_back,
                    on_toggle_theme,
                    on_navigate,
                    cx,
                )
            });

            // 设置页常驻(下载目录编辑状态不随页面切换丢失)
            let settings = cx.new(|cx| SettingsPage::new(window, cx));

            // 筛选窗口输入框(常驻;回车应用关键词)
            let filter_input = cx.new(|cx| {
                InputState::new(window, cx).placeholder("输入标题必须包含的关键词…")
            });
            cx.subscribe(
                &filter_input,
                move |this: &mut MikanPlus,
                      _input: Entity<InputState>,
                      event: &InputEvent,
                      cx: &mut Context<MikanPlus>| {
                    if let InputEvent::PressEnter { .. } = event
                        && this.filter_modal.is_some() {
                            this.apply_filter_from_input(cx);
                        }
                },
            )
            .detach();

            let mut this = Self {
                current_page: Page::Home(HomeFilter::Today),
                history: Vec::new(),
                bangumi_groups: Vec::new(),
                list_state: ListState::Idle,
                window_handle: window.window_handle(),
                details: HashMap::new(),
                detail_order: VecDeque::new(),
                detail_loading: HashSet::new(),
                detail_error: HashMap::new(),
                detail_names: HashMap::new(),
                search_results: HashMap::new(),
                search_order: VecDeque::new(),
                search_loading: HashSet::new(),
                search_error: HashMap::new(),
                search_page: HashMap::new(),
                subscriptions: load_subscriptions(),
                toolbar,
                settings,
                scroll_handles: HashMap::new(),
                scroll_order: VecDeque::new(),
                subgroup_keywords: load_subgroup_keywords(),
                filter_modal: None,
                unsubscribe_warning: None,
                pending_unsub: None,
                filter_input,
                downloader: DownloadManager::start(),
                on_card_click,
                on_toggle_subscribe,
            };
            // 启动列表加载(缓存命中则零请求)
            this.start_list_load(cx);
            this
        })
    }

    fn navigate_to(&mut self, page: Page, cx: &mut Context<Self>) {
        if self.current_page != page {
            self.history.push(self.current_page.clone());
            // 返回栈有界:超出上限时丢最旧
            if self.history.len() > HISTORY_LIMIT {
                self.history.remove(0);
            }
        }
        self.current_page = page.clone();
        // 页面切换时关闭筛选窗口(它与特定订阅条目绑定)
        self.filter_modal = None;
        // 关闭退订警告窗口(与旧页面上下文无关)
        self.unsubscribe_warning = None;
        // 详情页:触发懒加载(未加载时后台获取)
        if let Page::BangumiDetail(name) = &page {
            self.ensure_detail(name.clone(), cx);
        }
        // 搜索页:触发在线搜索(缓存命中则零请求)
        if let Page::SearchResult(q) = &page {
            self.ensure_search(q.clone(), cx);
        }
        cx.notify();
    }

    /// 加载番剧列表:优先磁盘缓存(30 分钟 TTL),未命中才在后台线程请求网络。
    /// 后台线程完成后写入全局结果并递增版本号,由轮询循环刷新 UI。
    fn start_list_load(&mut self, cx: &mut Context<Self>) {
        if self.list_state == ListState::Loading {
            return;
        }
        // 磁盘缓存
        if let Some(v) = storage::cache::cached_list()
            && let Ok(groups) = serde_json::from_value::<Vec<BangumiGroup>>(v)
        {
            self.bangumi_groups = groups;
            self.list_state = ListState::Ready;
            cx.notify();
            return;
        }
        // 用户主动加载/重试:清除该页面的退避状态,立即放行
        let base = source::network::base_url();
        source::network::reset_backoff(base);
        self.list_state = ListState::Loading;
        std::thread::spawn(move || {
            let result = source::network::fetch_html(base)
                .map(|html| source::parser::parse_bangumi_list(&html));
            if let Ok(groups) = &result {
                // 写缓存,减少后续访问
                if let Ok(v) = serde_json::to_value(groups) {
                    storage::cache::store_list(&v);
                }
            }
            *LIST_RESULT.lock().unwrap() = Some(result);
            LOAD_VERSION.fetch_add(1, Ordering::Relaxed);
        });
        cx.notify();
    }

    /// 消费后台线程完成的列表加载结果(render 时调用,幂等)
    fn consume_list_result(&mut self) {
        let result = LIST_RESULT.lock().unwrap().take();
        let Some(result) = result else {
            return;
        };
        match result {
            Ok(groups) => {
                self.bangumi_groups = groups;
                self.list_state = ListState::Ready;
            }
            Err(e) => {
                self.list_state = ListState::Error(e);
            }
        }
    }

    /// 按名称查找列表中的基础条目(本地列表优先,回退在线搜索结果)
    fn find_list_item(&self, name: &str) -> Option<BangumiItem> {
        self.bangumi_groups
            .iter()
            .flat_map(|g| g.items.iter())
            .find(|i| i.name == name)
            .cloned()
            .or_else(|| {
                self.search_results
                    .values()
                    .flat_map(|r| r.items.iter())
                    .find(|i| i.name == name)
                    .cloned()
            })
    }

    /// 按 ID 查找列表中的基础条目
    fn find_list_item_by_id(&self, bid: u32) -> Option<BangumiItem> {
        self.bangumi_groups
            .iter()
            .flat_map(|g| g.items.iter())
            .find(|i| i.bangumi_id == Some(bid))
            .cloned()
    }

    /// 确保详情已加载(lazy):缓存命中直接使用;否则后台线程请求一次。
    /// 基础条目查找:列表/搜索结果优先,订阅记录兑底(已下档番剧也能进入详情)。
    fn ensure_detail(&mut self, name: String, cx: &mut Context<Self>) {
        if self.details.contains_key(&name) || self.detail_loading.contains(&name) {
            return;
        }
        let base = self.find_list_item(&name).or_else(|| {
            self.subscriptions
                .iter()
                .find(|s| s.bangumi_name == name)
                .map(|s| BangumiItem {
                    name: name.clone(),
                    bangumi_id: Some(s.bangumi_id),
                    cover_url: s.cover_url.clone(),
                    ..Default::default()
                })
        });
        let Some(base) = base else {
            return;
        };
        let Some(bid) = base.bangumi_id else {
            return;
        };
        // 记录 bid → name 映射(消费结果时不再依赖列表查找)
        self.detail_names.insert(bid, name.clone());
        // 磁盘缓存(24 小时 TTL)
        if let Some(v) = storage::cache::cached_detail(bid)
            && let Ok(item) = serde_json::from_value::<BangumiItem>(v)
        {
            bounded_insert(
                &mut self.details,
                &mut self.detail_order,
                name.clone(),
                item,
                BOUNDED_CACHE_LIMIT,
            );
            cx.notify();
            return;
        }
        // 用户主动进入/重试:清除该详情页的退避状态,立即放行
        let url = format!("{}/Home/Bangumi/{bid}", source::network::base_url());
        source::network::reset_backoff(&url);
        self.detail_loading.insert(name.clone());
        cx.notify();
        std::thread::spawn(move || {
            let result = source::network::fetch_html(&url)
                .map(|html| source::parser::parse_bangumi_detail(&html))
                .map(|(meta, groups)| {
                    let mut item = base;
                    item.meta = Some(meta);
                    item.subtitle_groups = groups;
                    // 写缓存
                    if let Ok(v) = serde_json::to_value(&item) {
                        storage::cache::store_detail(bid, &v);
                    }
                    item
                });
            detail_result().lock().unwrap().insert(bid, result);
            LOAD_VERSION.fetch_add(1, Ordering::Relaxed);
        });
    }

    /// 消费后台线程完成的详情加载结果(render 时调用,幂等)
    fn consume_detail_results(&mut self) {
        let finished: Vec<(u32, Result<BangumiItem, SourceError>)> =
            detail_result().lock().unwrap().drain().collect();
        if finished.is_empty() {
            return;
        }
        for (bid, result) in finished {
            // 名称来源:加载时记录的映射 → 列表 → 搜索结果(与发起侧一致)
            let name = self.detail_names.remove(&bid).or_else(|| {
                self.find_list_item_by_id(bid).map(|i| i.name).or_else(|| {
                    self.search_results
                        .values()
                        .flat_map(|r| r.items.iter())
                        .find(|i| i.bangumi_id == Some(bid))
                        .map(|i| i.name.clone())
                })
            });
            let Some(name) = name else {
                // 防御:极端情况下查不到名称也不丢弃加载状态(避免永久转圈)
                continue;
            };
            self.detail_loading.remove(&name);
            match result {
                Ok(item) => {
                    bounded_insert(
                        &mut self.details,
                        &mut self.detail_order,
                        name.clone(),
                        item,
                        BOUNDED_CACHE_LIMIT,
                    );
                }
                Err(e) => {
                    self.detail_error.insert(name.clone(), e);
                }
            }
        }
    }

    /// 按 bid 定位详情名称(详情缓存 → 加载映射 → 列表 → 订阅记录)
    fn detail_name_by_bid(&self, bid: u32) -> Option<String> {
        self.details
            .values()
            .find(|i| i.bangumi_id == Some(bid))
            .map(|i| i.name.clone())
            .or_else(|| self.detail_names.get(&bid).cloned())
            .or_else(|| self.find_list_item_by_id(bid).map(|i| i.name.clone()))
            .or_else(|| {
                self.subscriptions
                    .iter()
                    .find(|s| s.bangumi_id == bid)
                    .map(|s| s.bangumi_name.clone())
            })
    }

    /// 确保在线搜索已加载:磁盘缓存命中直接用;否则后台线程请求一次。
    fn ensure_search(&mut self, query: String, cx: &mut Context<Self>) {
        if self.search_results.contains_key(&query) || self.search_loading.contains(&query) {
            return;
        }
        // 磁盘缓存(10 分钟 TTL,见 cache.rs)
        if let Some(v) = storage::cache::cached_search(&query)
            && let Ok(results) = serde_json::from_value::<SearchResults>(v)
        {
            bounded_insert(
                &mut self.search_results,
                &mut self.search_order,
                query,
                results,
                BOUNDED_CACHE_LIMIT,
            );
            cx.notify();
            return;
        }
        // 用户主动搜索/重试:清除退避,立即放行
        source::network::reset_backoff(&source::network::search_url(&query));
        self.search_loading.insert(query.clone());
        cx.notify();
        std::thread::spawn(move || {
            let url = source::network::search_url(&query);
            let result = source::network::fetch_html(&url)
                .map(|html| source::parser::parse_search_results(&html))
                .inspect(|results| {
                    // 写缓存,减少后续访问
                    if let Ok(v) = serde_json::to_value(results) {
                        storage::cache::store_search(&query, &v);
                    }
                });
            search_result().lock().unwrap().insert(query, result);
            LOAD_VERSION.fetch_add(1, Ordering::Relaxed);
        });
    }

    /// 消费后台线程完成的搜索结果(render 时调用,幂等)
    fn consume_search_results(&mut self) {
        let finished: Vec<(String, Result<SearchResults, SourceError>)> =
            search_result().lock().unwrap().drain().collect();
        for (query, result) in finished {
            self.search_loading.remove(&query);
            match result {
                Ok(results) => {
                    bounded_insert(
                        &mut self.search_results,
                        &mut self.search_order,
                        query.clone(),
                        results,
                        BOUNDED_CACHE_LIMIT,
                    );
                    // 淘汰的查询同步清理其分页页码,避免残留
                    let alive: HashSet<&String> = self.search_results.keys().collect();
                    self.search_page.retain(|q, _| alive.contains(q));
                }
                Err(e) => {
                    self.search_error.insert(query.clone(), e);
                }
            }
        }
    }

    fn go_back(&mut self, cx: &mut Context<Self>) {
        if let Some(prev) = self.history.pop() {
            self.current_page = prev;
            cx.notify();
        }
    }

    fn is_pair_subscribed(&self, bangumi_id: u32, subgroup_id: u32) -> bool {
        self.subscriptions
            .iter()
            .any(|s| s.bangumi_id == bangumi_id && s.subgroup_id == subgroup_id)
    }

    /// 切换单个字幕组的订阅(订阅以字幕组为单位)
    fn toggle_subscription(
        &mut self,
        bangumi_id: u32,
        subgroup_id: u32,
        bangumi_name: Option<&str>,
        group_name: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        if self.is_pair_subscribed(bangumi_id, subgroup_id) {
            self.unsubscribe(bangumi_id, subgroup_id, cx);
        } else {
            // 封面来源:列表优先,搜索结果兑底(仅存在于搜索结果的番剧也能带封面订阅)
            let cover = self
                .find_list_item(bangumi_name.unwrap_or(""))
                .and_then(|i| i.cover_url.clone())
                .or_else(|| {
                    self.search_results
                        .values()
                        .flat_map(|r| r.items.iter())
                        .find(|i| i.bangumi_id == Some(bangumi_id))
                        .and_then(|i| i.cover_url.clone())
                });
            self.subscriptions.push(Subscription {
                bangumi_id,
                subgroup_id,
                bangumi_name: bangumi_name.unwrap_or("未知").to_string(),
                group_name: group_name.unwrap_or("未知").to_string(),
                cover_url: cover,
            });
            save_subscriptions(&self.subscriptions);
        }
    }

    /// 取消单个字幕组的订阅。
    ///
    /// 「检查进行中任务 → 取消残留 → 删除目录」整体下沉到下载线程原子执行
    /// (通过 [`DownloadCmd::CleanupDir`]),避免 UI 线程删目录卡顿与快照竞态;
    /// 结果经 [`downloader::DownloadEvent`] 回执:
    /// - 有进行中任务 → 阻断,弹出警告窗口
    /// - 清理完成 → 移除订阅记录与筛选关键词
    fn unsubscribe(&mut self, bangumi_id: u32, subgroup_id: u32, cx: &mut Context<Self>) {
        let sub = self
            .subscriptions
            .iter()
            .find(|s| s.bangumi_id == bangumi_id && s.subgroup_id == subgroup_id)
            .cloned();
        let Some(sub) = sub else {
            return;
        };
        let dir = storage::paths::subgroup_download_dir(
            &storage::load_download_dir(),
            &sub.bangumi_name,
            &sub.group_name,
        );

        // 登记待处理退订,等待下载线程回执
        self.pending_unsub = Some((
            bangumi_id,
            subgroup_id,
            sub.bangumi_name.clone(),
            sub.group_name.clone(),
        ));
        if let Err(e) = self.downloader.send(DownloadCmd::CleanupDir {
            dir: dir.clone(),
            bangumi_name: sub.bangumi_name.clone(),
            group_name: sub.group_name.clone(),
        }) {
            // 引擎不可用:直接本地完成退订(引擎已死,不存在并发写)
            eprintln!("退订清理命令发送失败: {e}");
            self.pending_unsub = None;
            let _ = std::fs::remove_dir_all(&dir);
            self.finish_unsubscribe(bangumi_id, subgroup_id);
            cx.notify();
        }
    }

    /// 下载线程回执:退订被阻断(该目录有进行中的任务)→ 弹出警告窗口
    fn handle_unsubscribe_blocked(&mut self, dir: &std::path::Path, titles: Vec<String>) {
        let Some((_, _, bangumi_name, group_name)) = &self.pending_unsub else {
            return;
        };
        let expected = storage::paths::subgroup_download_dir(
            &storage::load_download_dir(),
            bangumi_name,
            group_name,
        );
        if expected != dir {
            return;
        }
        self.unsubscribe_warning = Some(UnsubscribeWarning {
            bangumi_name: bangumi_name.clone(),
            group_name: group_name.clone(),
            active_titles: titles,
        });
        self.pending_unsub = None;
    }

    /// 下载线程回执:目录清理完成 → 移除订阅记录与筛选关键词
    fn handle_unsubscribe_done(&mut self, dir: &std::path::Path) {
        let Some((bangumi_id, subgroup_id, bangumi_name, group_name)) = &self.pending_unsub else {
            return;
        };
        let expected = storage::paths::subgroup_download_dir(
            &storage::load_download_dir(),
            bangumi_name,
            group_name,
        );
        if expected != dir {
            return;
        }
        let (bangumi_id, subgroup_id) = (*bangumi_id, *subgroup_id);
        self.pending_unsub = None;
        self.finish_unsubscribe(bangumi_id, subgroup_id);
    }

    /// 移除订阅记录、筛选关键词并持久化(目录清理完成后的收尾)
    fn finish_unsubscribe(&mut self, bangumi_id: u32, subgroup_id: u32) {
        self.subscriptions
            .retain(|s| !(s.bangumi_id == bangumi_id && s.subgroup_id == subgroup_id));
        // 退订时一并清除该条目的筛选关键词(持久化数据不留残留)
        self.subgroup_keywords.remove(&(bangumi_id, subgroup_id));
        save_subgroup_keywords(&self.subgroup_keywords);
        save_subscriptions(&self.subscriptions);
    }

    /// 工具栏标题(面包屑)
    fn page_title(&self) -> String {
        match &self.current_page {
            Page::Home(filter) => filter.label().to_string(),
            Page::Subscription => "我的订阅".to_string(),
            Page::Settings => "设置".to_string(),
            Page::BangumiDetail(name) => name.clone(),
            Page::SubGroupDetail(_, _) => "字幕组".to_string(),
            Page::SearchResult(q) => format!("搜索「{q}」"),
        }
    }

    /// 页面的滚动状态键(不同页面/筛选各自独立保留滚动位置)
    fn page_scroll_key(&self, page: &Page) -> String {
        match page {
            Page::Home(filter) => format!("home:{filter:?}"),
            Page::Subscription => "subscription".to_string(),
            Page::Settings => "settings".to_string(),
            Page::BangumiDetail(name) => format!("detail:{name}"),
            Page::SubGroupDetail(bid, sid) => format!("subgroup:{bid}:{sid}"),
            Page::SearchResult(q) => format!("search:{q}"),
        }
    }

    /// 获取(或创建)当前页面的滚动句柄,页面切换后重新进入时恢复原位置。
    /// 有界:超过上限时淘汰最旧的(与详情/搜索缓存一致)。
    fn scroll_handle(&mut self, key: &str) -> ScrollHandle {
        let existing = self.scroll_handles.get(key).cloned();
        if let Some(h) = existing {
            return h;
        }
        bounded_insert(
            &mut self.scroll_handles,
            &mut self.scroll_order,
            key.to_string(),
            ScrollHandle::new(),
            BOUNDED_CACHE_LIMIT,
        );
        self.scroll_handles.get(key).cloned().unwrap()
    }

    /// 打开剧集筛选窗口(预填当前关键词)
    fn open_filter_modal(&mut self, key: (u32, u32), cx: &mut Context<Self>) {
        let keyword = self
            .subgroup_keywords
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let handle = self.window_handle;
        let input = self.filter_input.clone();
        let _ = handle.update(cx, move |_, window, cx: &mut App| {
            input.update(cx, |state, cx| {
                state.set_value(keyword, window, cx);
            });
        });
        self.filter_modal = Some(key);
        cx.notify();
    }

    /// 读取输入框内容并应用为当前订阅条目的筛选关键词(空 = 清除),然后关闭窗口
    fn apply_filter_from_input(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.filter_modal else {
            return;
        };
        let text = self.filter_input.read(cx).text().to_string();
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            self.subgroup_keywords.remove(&key);
        } else {
            self.subgroup_keywords.insert(key, trimmed);
        }
        save_subgroup_keywords(&self.subgroup_keywords);
        self.filter_modal = None;
        cx.notify();
    }

    /// 清除当前订阅条目的筛选关键词并关闭窗口
    fn clear_filter_keyword(&mut self, cx: &mut Context<Self>) {
        if let Some(key) = self.filter_modal {
            self.subgroup_keywords.remove(&key);
        }
        save_subgroup_keywords(&self.subgroup_keywords);
        self.filter_modal = None;
        cx.notify();
    }

    /// 关闭筛选窗口(不改变关键词)
    fn close_filter_modal(&mut self, cx: &mut Context<Self>) {
        self.filter_modal = None;
        cx.notify();
    }

    /// 关闭退订警告窗口
    fn close_unsubscribe_warning(&mut self, cx: &mut Context<Self>) {
        self.unsubscribe_warning = None;
        cx.notify();
    }
}

impl Render for MikanPlus {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let today = today_weekday();

        // 工具栏状态
        let can_back = !self.history.is_empty();
        let title = self.page_title();
        let current_section = TopSection::from_page(&self.current_page);
        self.toolbar.update(cx, |toolbar, cx| {
            toolbar.can_go_back = can_back;
            toolbar.title = title.clone();
            toolbar.current_section = current_section;
            cx.notify();
        });

        let theme = cx.theme().clone();

        // 消费后台线程完成的结果(幂等)
        self.consume_list_result();
        self.consume_detail_results();
        self.consume_search_results();

        // 当前页面的滚动句柄(应用持有,页面切换后恢复原位置)
        let scroll_key = self.page_scroll_key(&self.current_page);
        let scroll_handle = self.scroll_handle(&scroll_key);

        // 首页过滤条回调(星期/今日/剧场版切换)
        let on_filter_change: FilterChangeCallback = {
            let entity = cx.entity().clone();
            Rc::new(move |filter: HomeFilter, _window, app: &mut App| {
                entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                    mp.navigate_to(Page::Home(filter), cx);
                });
            })
        };

        // 订阅状态检查(以字幕组为单位)
        let subscribed_check: SubscribedChecker = {
            let subs = self.subscriptions.clone();
            Rc::new(move |bid: u32, sid: u32| {
                subs.iter()
                    .any(|s| s.bangumi_id == bid && s.subgroup_id == sid)
            })
        };

        // 内容区
        let content: gpui_kit::AnyElement = match self.current_page.clone() {
            Page::Home(filter) => {
                // 列表加载状态:加载中 / 失败(重试)/ 就绪
                if self.list_state == ListState::Loading || self.list_state == ListState::Idle {
                    loading_view(&theme).into_any_element()
                } else if let ListState::Error(msg) = &self.list_state {
                    let entity = cx.entity().clone();
                    let retry = Rc::new(move |_window: &mut Window, app: &mut App| {
                        entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                            mp.start_list_load(cx);
                        });
                    });
                    error_view(msg, retry, &theme).into_any_element()
                } else {
                    let home = HomeView {
                        groups: self.bangumi_groups.clone(),
                        filter,
                        today_weekday: today,
                        on_card_click: self.on_card_click.clone(),
                        on_filter_change: on_filter_change.clone(),
                    };
                    scroll_page(home, &scroll_handle)
                }
            }
            Page::Subscription => {
                let on_sub_click: SubCardClickCallback = {
                    let entity = cx.entity().clone();
                    Rc::new(move |bid, sid, _window, app: &mut App| {
                        entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                            mp.navigate_to(Page::SubGroupDetail(bid, sid), cx);
                        });
                    })
                };
                let on_unsubscribe: UnsubscribeCallback = {
                    let entity = cx.entity().clone();
                    Rc::new(move |bid, sid, _window, app: &mut App| {
                        entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                            mp.unsubscribe(bid, sid, cx);
                            cx.notify();
                        });
                    })
                };
                let on_go_home: GoBackCallback = {
                    let entity = cx.entity().clone();
                    Rc::new(move |_window, app: &mut App| {
                        entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                            mp.navigate_to(Page::Home(HomeFilter::Today), cx);
                        });
                    })
                };
                let page = SubscriptionPage {
                    subscriptions: self.subscriptions.clone(),
                    scroll_handle: scroll_handle.clone(),
                    on_sub_click,
                    on_unsubscribe,
                    on_go_home,
                };
                page.into_any_element()
            }
            Page::Settings => self.settings.clone().into_any_element(),
            Page::BangumiDetail(name) => {
                // 兜底触发加载(首次渲染且未走 navigate 时)
                self.ensure_detail(name.clone(), cx);
                let check = subscribed_check.clone();
                if let Some(item) = self.details.get(&name).cloned() {
                    let detail = BangumiDetailPage {
                        item: Some(item),
                        is_subscribed: check,
                        on_toggle_subscribe: self.on_toggle_subscribe.clone(),
                        scroll_handle: scroll_handle.clone(),
                        downloader: self.downloader.clone(),
                    };
                    detail.into_any_element()
                } else if let Some(err) = self.detail_error.get(&name) {
                    let entity = cx.entity().clone();
                    let name = name.clone();
                    let retry = Rc::new(move |_window: &mut Window, app: &mut App| {
                        entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                            mp.detail_error.remove(&name);
                            mp.ensure_detail(name.clone(), cx);
                        });
                    });
                    error_view(err, retry, &theme).into_any_element()
                } else {
                    loading_view(&theme).into_any_element()
                }
            }
            Page::SubGroupDetail(bid, sid) => {
                // 剧集筛选关键词(按订阅条目记忆)与打开筛选窗口回调
                let filter_key = (bid, sid);
                let keyword = self
                    .subgroup_keywords
                    .get(&filter_key)
                    .cloned()
                    .unwrap_or_default();
                let on_open_filter: OpenFilterCallback = {
                    let entity = cx.entity().clone();
                    Rc::new(move |_window, app| {
                        entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                            mp.open_filter_modal(filter_key, cx);
                        });
                    })
                };
                let item = self
                    .details
                    .values()
                    .find(|i| i.bangumi_id == Some(bid))
                    .cloned();
                let group = item
                    .as_ref()
                    .and_then(|i| {
                        i.subtitle_groups
                            .iter()
                            .find(|sg| sg.subgroup_id == Some(sid))
                    })
                    .cloned();
                match group {
                    Some(group) => {
                        let bangumi_name = item.map(|i| i.name).unwrap_or_default();
                        let page = SubGroupDetailPage {
                            bangumi_name,
                            group,
                            scroll_handle: scroll_handle.clone(),
                            downloader: self.downloader.clone(),
                            keyword: keyword.clone(),
                            on_open_filter: on_open_filter.clone(),
                        };
                        page.into_any_element()
                    }
                    None => {
                        // 详情未加载:按 bid 定位名称并触发懒加载
                        // (列表、加载映射、订阅记录兑底——已下档番剧也能进入)
                        let name = self.detail_name_by_bid(bid);
                        if let Some(name) = &name {
                            self.ensure_detail(name.clone(), cx);
                        }
                        // 重新查找(可能刚加载完成)
                        let item = self
                            .details
                            .values()
                            .find(|i| i.bangumi_id == Some(bid))
                            .cloned();
                        let group = item
                            .as_ref()
                            .and_then(|i| {
                                i.subtitle_groups
                                    .iter()
                                    .find(|sg| sg.subgroup_id == Some(sid))
                            })
                            .cloned();
                        if let Some(group) = group {
                            let page = SubGroupDetailPage {
                                bangumi_name: item.map(|i| i.name).unwrap_or_default(),
                                group,
                                scroll_handle: scroll_handle.clone(),
                                downloader: self.downloader.clone(),
                                keyword: keyword.clone(),
                                on_open_filter: on_open_filter.clone(),
                            };
                            page.into_any_element()
                        } else if let Some(name) = name
                            && let Some(err) = self.detail_error.get(&name).cloned()
                        {
                            // 该番剧加载失败:错误按名称归属,重试只重试当前番剧
                            let entity = cx.entity().clone();
                            let retry = Rc::new(move |_window: &mut Window, app: &mut App| {
                                entity.update(
                                    app,
                                    |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                                        mp.detail_error.remove(&name);
                                        mp.ensure_detail(name.clone(), cx);
                                    },
                                );
                            });
                            error_view(&err, retry, &theme).into_any_element()
                        } else {
                            loading_view(&theme).into_any_element()
                        }
                    }
                }
            }
            Page::SearchResult(query) => {
                // 兜底触发在线搜索(首次渲染且未走 navigate 时)
                self.ensure_search(query.clone(), cx);
                if let Some(results) = self.search_results.get(&query).cloned() {
                    let page = self.search_page.get(&query).copied().unwrap_or(0);
                    let on_page_change: ui::search_result_page::SearchPageChangeCallback = {
                        let entity = cx.entity().clone();
                        let query = query.clone();
                        std::rc::Rc::new(move |p, _window, app| {
                            entity.update(
                                app,
                                |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                                    mp.search_page.insert(query.clone(), p);
                                    cx.notify();
                                },
                            );
                        })
                    };
                    let page = SearchResultPage {
                        query: query.clone(),
                        results: results.clone(),
                        on_card_click: self.on_card_click.clone(),
                        downloader: self.downloader.clone(),
                        page,
                        on_page_change,
                    };
                    scroll_page(page, &scroll_handle)
                } else if let Some(err) = self.search_error.get(&query) {
                    let entity = cx.entity().clone();
                    let query = query.clone();
                    let retry = Rc::new(move |_window: &mut Window, app: &mut App| {
                        entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                            mp.search_error.remove(&query);
                            mp.ensure_search(query.clone(), cx);
                        });
                    });
                    error_view(err, retry, &theme).into_any_element()
                } else {
                    loading_view(&theme).into_any_element()
                }
            }
        };

        // 筛选窗口(居中浮动):遮罩 + 卡片,挂载在内容层之上
        let filter_modal_view: Option<gpui_kit::AnyElement> = {
            let filter_input = self.filter_input.clone();
            let keywords = self.subgroup_keywords.clone();
            let entity = cx.entity().clone();
            self.filter_modal.map(|key| {
                let keyword = keywords.get(&key).cloned().unwrap_or_default();
                let on_clear: GoBackCallback = {
                    let entity = entity.clone();
                    Rc::new(move |_window, app| {
                        entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                            mp.clear_filter_keyword(cx);
                        });
                    })
                };
                let on_cancel: GoBackCallback = {
                    let entity = entity.clone();
                    Rc::new(move |_window, app| {
                        entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                            mp.close_filter_modal(cx);
                        });
                    })
                };
                let on_confirm: GoBackCallback = {
                    let entity = entity.clone();
                    Rc::new(move |_window, app| {
                        entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                            mp.apply_filter_from_input(cx);
                        });
                    })
                };
                render_filter_modal(
                    &theme,
                    filter_input.clone(),
                    &keyword,
                    on_clear,
                    on_cancel,
                    on_confirm,
                )
                .into_any_element()
            })
        };

        // 退订拦截警告窗口(居中浮动):告知用户有正在下载的剧集
        let warning_modal_view: Option<gpui_kit::AnyElement> = {
            let entity = cx.entity().clone();
            self.unsubscribe_warning.clone().map(|w| {
                let on_close: GoBackCallback = {
                    let entity = entity.clone();
                    Rc::new(move |_window, app| {
                        entity.update(app, |mp: &mut MikanPlus, cx: &mut Context<MikanPlus>| {
                            mp.close_unsubscribe_warning(cx);
                        });
                    })
                };
                render_unsubscribe_warning(&theme, &w, on_close).into_any_element()
            })
        };

        // ---- 布局 ----
        //
        // 重要:gpui 0.2.2(crates.io 发布版)中,flex 的
        // 主轴 grow 与交叉轴 stretch 在垂直方向上不可靠(高度会
        // 退化为 0 或内容高度),因此这里全部使用「绝对定位 + 显式
        // 尺寸」:滚动容器高度完全确定,内容才能溢出并滚动。
        gpui_kit::div()
            .id("mikan-root")
            .size_full()
            .relative()
            .bg(theme.background)
            .child(
                // 工具栏:固定在顶部
                gpui_kit::div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(48.))
                    .child(self.toolbar.clone()),
            )
            .child(
                // 内容挂载点:位于工具栏下方,高度确定(视口-48)
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
            // 筛选窗口(居中浮动,覆盖在内容层之上)
            .when_some(filter_modal_view, |this, modal| this.child(modal))
            // 退订警告窗口(居中浮动,覆盖在内容层之上)
            .when_some(warning_modal_view, |this, modal| this.child(modal))
            // ---- 动作处理 ----
            .on_action(cx.listener(|this, _: &GoHome, _window, cx| {
                this.navigate_to(Page::Home(HomeFilter::Today), cx);
            }))
            .on_action(cx.listener(|this, _: &GoSubscription, _window, cx| {
                this.navigate_to(Page::Subscription, cx);
            }))
            .on_action(cx.listener(|this, _: &GoMonday, _window, cx| {
                this.navigate_to(Page::Home(HomeFilter::Weekday(0)), cx);
            }))
            .on_action(cx.listener(|this, _: &GoTuesday, _window, cx| {
                this.navigate_to(Page::Home(HomeFilter::Weekday(1)), cx);
            }))
            .on_action(cx.listener(|this, _: &GoWednesday, _window, cx| {
                this.navigate_to(Page::Home(HomeFilter::Weekday(2)), cx);
            }))
            .on_action(cx.listener(|this, _: &GoThursday, _window, cx| {
                this.navigate_to(Page::Home(HomeFilter::Weekday(3)), cx);
            }))
            .on_action(cx.listener(|this, _: &GoFriday, _window, cx| {
                this.navigate_to(Page::Home(HomeFilter::Weekday(4)), cx);
            }))
            .on_action(cx.listener(|this, _: &GoSaturday, _window, cx| {
                this.navigate_to(Page::Home(HomeFilter::Weekday(5)), cx);
            }))
            .on_action(cx.listener(|this, _: &GoSunday, _window, cx| {
                this.navigate_to(Page::Home(HomeFilter::Weekday(6)), cx);
            }))
            .on_action(cx.listener(|this, _: &GoMovies, _window, cx| {
                this.navigate_to(Page::Home(HomeFilter::Movies), cx);
            }))
            .on_action(cx.listener(|this, _: &GoSettings, _window, cx| {
                this.navigate_to(Page::Settings, cx);
            }))
            .on_action(cx.listener(|this, _: &GoBack, _window, cx| {
                this.go_back(cx);
            }))
            .on_action(cx.listener(|this, _: &FocusSearch, window, cx| {
                this.toolbar.update(cx, |toolbar, cx| {
                    toolbar.focus_search(window, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &CloseFilterModal, _window, cx| {
                this.close_filter_modal(cx);
                this.close_unsubscribe_warning(cx);
            }))
            .on_action(cx.listener(|_this, _: &ToggleTheme, window, cx| {
                let dark = gpui_kit::component::theme::Theme::global(cx).mode
                    == gpui_kit::component::theme::ThemeMode::Dark;
                let mode = if dark {
                    gpui_kit::component::theme::ThemeMode::Light
                } else {
                    gpui_kit::component::theme::ThemeMode::Dark
                };
                app_theme::set_mode(mode, Some(window), cx);
            }))
            .on_action(cx.listener(|_this, _: &LightMode, window, cx| {
                app_theme::set_mode(
                    gpui_kit::component::theme::ThemeMode::Light,
                    Some(window),
                    cx,
                );
            }))
            .on_action(cx.listener(|_this, _: &DarkMode, window, cx| {
                app_theme::set_mode(
                    gpui_kit::component::theme::ThemeMode::Dark,
                    Some(window),
                    cx,
                );
            }))
            .on_action(cx.listener(|_this, _: &AboutMikan, _window, cx| {
                // 打开设置页(包含关于信息)
                let entity = cx.entity().clone();
                entity.update(cx, |mp: &mut MikanPlus, cx| {
                    mp.navigate_to(Page::Settings, cx);
                });
            }))
            .on_action(cx.listener(|_this, _: &HideApp, _window, cx| {
                cx.hide();
            }))
            .on_action(cx.listener(|_this, _: &HideOthers, _window, cx| {
                cx.hide_other_apps();
            }))
            .on_action(cx.listener(|_this, _: &QuitApp, _window, cx| {
                cx.quit();
            }))
            .on_action(cx.listener(|_this, _: &CloseWindow, window, _cx| {
                window.remove_window();
            }))
            .on_action(cx.listener(|_this, _: &ZoomWindow, window, _cx| {
                window.zoom_window();
            }))
            .on_action(cx.listener(|_this, _: &MinimizeWindow, window, _cx| {
                window.minimize_window();
            }))
            .on_action(cx.listener(|_this, _: &ToggleFullscreen, window, _cx| {
                window.toggle_fullscreen();
            }))
            .on_action(cx.listener(|_this, _: &OpenMikanWebsite, _window, _cx| {
                let _ = storage::paths::open_url(source::network::base_url());
            }))
            // 应用内通知层(右上角弹出,Root 不会自动渲染,需手动挂载)
            .when_some(
                gpui_kit::component::Root::render_notification_layer(window, cx),
                |this, layer| this.child(layer),
            )
    }
}

/// 将页面包一层滚动容器。
///
/// 滚动容器占满挂载点宽度(滚动条贴窗口右缘),内容自适应并居中,
/// 上限由 `layout::MAX_PAGE_W` 统一管理。滚动位置由调用方持有的
/// `ScrollHandle` 维持。高度依赖内容挂载点(绝对定位,尺寸确定)的
/// `h_full()`,不要依赖 flex 的 grow/stretch——gpui 0.2.2 中它们在
/// 垂直方向不可靠,会导致滚动容器高度退化为内容高度而无法滚动。
fn scroll_page(page: impl IntoElement, handle: &ScrollHandle) -> gpui_kit::AnyElement {
    ui::layout::page_scroll(ui::layout::MAX_PAGE_W, handle, page).into_any_element()
}

/// 居中浮动警告窗口:告知用户该订阅有正在下载的剧集,退订被拒绝。
///
/// 点击遮罩或「知道了」关闭;Esc(CloseFilterModal)由调用方全局处理。
fn render_unsubscribe_warning(
    theme: &gpui_kit::component::theme::Theme,
    warning: &UnsubscribeWarning,
    on_close: GoBackCallback,
) -> impl IntoElement {
    // 列表最多展示 5 条,其余折叠为「…等 N 个」
    const MAX_LISTED: usize = 5;
    let total = warning.active_titles.len();
    let listed: Vec<String> = warning
        .active_titles
        .iter()
        .take(MAX_LISTED)
        .cloned()
        .collect();
    let more = total.saturating_sub(MAX_LISTED);

    gpui_kit::div()
        .id("unsubscribe-warning-modal")
        .absolute()
        .inset_0()
        .bg(gpui_kit::hsla(0., 0., 0., 0.4))
        .flex()
        .items_center()
        .justify_center()
        .on_click({
            let on_close = on_close.clone();
            move |_, window, app| on_close(window, app)
        })
        .child(
            gpui_kit::div()
                .id("unsubscribe-warning-card")
                .w(px(440.))
                .rounded(px(12.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.background)
                .shadow_lg()
                .p(px(20.))
                .flex()
                .flex_col()
                .on_click(move |_, _, app: &mut App| {
                    app.stop_propagation();
                })
                .child(
                    gpui_kit::div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(ui::icons::icon("info", 16.).text_color(theme.warning))
                        .child(
                            gpui_kit::div()
                                .text_base()
                                .font_semibold()
                                .text_color(theme.foreground)
                                .child("无法取消订阅"),
                        ),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(8.))
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(format!(
                            "「{} - {}」有 {total} 个剧集正在下载,请先等待下载完成或取消下载后再退订。",
                            warning.bangumi_name, warning.group_name
                        )),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(12.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .px(px(12.))
                        .py(px(8.))
                        .flex()
                        .flex_col()
                        .children(listed.iter().map(|t| {
                            gpui_kit::div()
                                .w_full()
                                .py(px(2.))
                                .text_sm()
                                .text_color(theme.foreground)
                                .truncate()
                                .child(t.clone())
                        }))
                        .when(more > 0, |this| {
                            this.child(
                                gpui_kit::div()
                                    .py(px(2.))
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(format!("…等 {more} 个")),
                            )
                        }),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(18.))
                        .flex()
                        .justify_end()
                        .child(
                            gpui_kit::div()
                                .id("unsubscribe-warning-ok")
                                .px(px(14.))
                                .py(px(6.))
                                .rounded(px(6.))
                                .text_sm()
                                .font_semibold()
                                .bg(theme.primary)
                                .text_color(theme.primary_foreground)
                                .cursor_pointer()
                                .hover(|style| style.bg(theme.primary_hover))
                                .on_click({
                                    let on_close = on_close.clone();
                                    move |_, window, app| on_close(window, app)
                                })
                                .child("知道了"),
                        ),
                ),
        )
}

/// 居中浮动筛选窗口:半透明遮罩 + 居中卡片(标题 / 说明 / 输入框 / 按钮行)。
///
/// 点击遮罩取消;卡片内点击不冒泡;Esc(CloseFilterModal)与回车(Input PressEnter)
/// 由调用方全局处理。
fn render_filter_modal(
    theme: &gpui_kit::component::theme::Theme,
    input: Entity<InputState>,
    keyword: &str,
    on_clear: GoBackCallback,
    on_cancel: GoBackCallback,
    on_confirm: GoBackCallback,
) -> impl IntoElement {
    let has_keyword = !keyword.is_empty();

    let btn = |label: &str, primary: bool, cb: GoBackCallback| {
        gpui_kit::div()
            .px(px(14.))
            .py(px(6.))
            .rounded(px(6.))
            .text_sm()
            .font_semibold()
            .cursor_pointer()
            .id(gpui_kit::SharedString::from(format!("filter-btn-{label}")))
            .when(primary, |this| {
                this.bg(theme.primary)
                    .text_color(theme.primary_foreground)
                    .hover(|style| style.bg(theme.primary_hover))
            })
            .when(!primary, |this| {
                this.border_1()
                    .border_color(theme.border)
                    .text_color(theme.foreground)
                    .hover(|style| style.bg(theme.list_hover))
            })
            .on_click(move |_, window, app| cb(window, app))
            .child(label.to_string())
    };

    gpui_kit::div()
        .id("filter-modal")
        .absolute()
        .inset_0()
        .bg(gpui_kit::hsla(0., 0., 0., 0.4))
        .flex()
        .items_center()
        .justify_center()
        .on_click({
            let on_cancel = on_cancel.clone();
            move |_, window, app| on_cancel(window, app)
        })
        .child(
            gpui_kit::div()
                .id("filter-card")
                .w(px(380.))
                .rounded(px(12.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.background)
                .shadow_lg()
                .p(px(20.))
                .flex()
                .flex_col()
                .on_click(move |_, _, app: &mut App| {
                    app.stop_propagation();
                })
                .child(
                    gpui_kit::div()
                        .text_base()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .child("筛选剧集"),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(6.))
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("仅显示标题包含以下关键词的剧集,留空则显示全部"),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(14.))
                        .child(Input::new(&input).w_full()),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(18.))
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .when(has_keyword, |this| {
                            this.child(btn("清除筛选", false, on_clear))
                        })
                        .child(btn("取消", false, on_cancel.clone()))
                        .child(btn("确定", true, on_confirm)),
                ),
        )
}

/// 加载中视图:缓慢旋转的加载图标 + 提示文字。
///
/// gpui-component 的 Spinner 转速固定为 0.8s/圈且无法配置,这里照其内部实现
/// 自绘一个 1.6s/圈的版本(仅调整周期,其余行为一致),让等待过程更舒缓。
fn loading_view(theme: &gpui_kit::component::theme::Theme) -> gpui_kit::Div {
    use gpui_kit::component::{Icon, IconName, Sizable};
    use gpui_kit::{Animation, AnimationExt, Transformation, percentage};
    gpui_kit::div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .flex_col()
        .child(
            // 圆形容器 + 缓慢旋转的加载图标:比裸 spinner 更有层次
            gpui_kit::div()
                .size(px(64.))
                .rounded_full()
                .border_1()
                .border_color(theme.border)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Icon::new(IconName::Loader)
                        .with_size(px(30.))
                        .text_color(theme.primary)
                        .with_animation(
                            "loading-spin",
                            Animation::new(std::time::Duration::from_secs_f64(1.6)).repeat(),
                            |this, delta| this.transform(Transformation::rotate(percentage(delta))),
                        ),
                ),
        )
        .child(
            gpui_kit::div()
                .mt(px(16.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(3.))
                .child(
                    gpui_kit::div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("正在加载…"),
                ),
        )
}

/// 加载失败视图:错误信息 + 重试按钮
fn error_view(
    err: &SourceError,
    retry: GoBackCallback,
    theme: &gpui_kit::component::theme::Theme,
) -> gpui_kit::Div {
    // 只展示用户能理解的信息,不暴露底层错误细节
    let message = err.user_message().to_string();
    let hint = err.user_hint().to_string();
    gpui_kit::div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(
            gpui_kit::div()
                .text_sm()
                .text_color(theme.danger)
                .child(message),
        )
        .when(!hint.is_empty(), |this| {
            this.child(
                gpui_kit::div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(hint),
            )
        })
        .child(
            gpui_kit::div()
                .px(px(14.))
                .py(px(6.))
                .rounded(px(6.))
                .bg(theme.primary)
                .text_sm()
                .text_color(theme.primary_foreground)
                .cursor_pointer()
                .id("retry-load")
                .hover(|style| style.bg(theme.primary_hover))
                .on_click(move |_, window, app| retry(window, app))
                .child("重试"),
        )
}

/// 创建主窗口(注册 on_reopen 重建用)
fn open_main_window(cx: &mut App) {
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Maximized(Bounds::default())),
            titlebar: Some(gpui_kit::TitlebarOptions {
                title: Some("MikanPlus".into()),
                ..Default::default()
            }),
            window_min_size: Some(gpui_kit::Size {
                width: gpui_kit::px(960.),
                height: gpui_kit::px(640.),
            }),
            ..Default::default()
        },
        |window, cx| {
            let mikan = MikanPlus::new(window, cx);
            cx.new(|cx| gpui_kit::component::Root::new(mikan, window, cx))
        },
    )
    .ok();
}

/// 解析资源目录(assets/),不依赖启动时的工作目录:
/// - macOS .app 包:`Contents/MacOS/../Resources/assets`
/// - Linux 安装版/Windows 便携版:可执行文件同级的 `assets/`
/// - 开发场景回退:当前目录 `assets`(cargo run)
fn asset_base() -> PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        #[cfg(target_os = "macos")]
        {
            let resources = dir.join("../Resources").join("assets");
            if resources.is_dir() {
                return resources;
            }
        }
        let beside_exe = dir.join("assets");
        if beside_exe.is_dir() {
            return beside_exe;
        }
    }
    PathBuf::from("assets")
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets { base: asset_base() });

    // macOS 惯例:⌘W 关闭最后窗口后应用仍驻留 Dock;
    // 点击 Dock 图标时重建窗口(下载任务在后台继续)
    app.on_reopen(|cx: &mut App| {
        if cx.windows().is_empty() {
            open_main_window(cx);
        } else {
            cx.activate(true);
        }
    });

    app.run(|cx: &mut App| {
        gpui_kit::init(cx);

        app_theme::init(cx);

        register_keybindings(cx);

        // 应用菜单栏
        cx.set_menus(build_menus());

        open_main_window(cx);
    });
}

struct Assets {
    base: PathBuf,
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        match fs::read(self.base.join(path)) {
            Ok(data) => Ok(Some(Cow::Owned(data))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                gpui_kit::assets::Assets.load(path)
            }
            // io::Error → anyhow::Error(gpui::Result 底层类型)由 From 自动转换
            Err(error) => Err(error.into()),
        }
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<SharedString>> {
        let mut assets = gpui_kit::assets::Assets.list(path)?;
        match fs::read_dir(self.base.join(path)) {
            Ok(entries) => assets.extend(
                entries
                    .filter_map(|entry| {
                        entry
                            .ok()
                            .and_then(|entry| entry.file_name().into_string().ok())
                    })
                    .map(SharedString::from),
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        assets.sort_unstable();
        assets.dedup();
        Ok(assets)
    }
}
