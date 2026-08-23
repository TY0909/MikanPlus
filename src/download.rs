//! 下载管理模块(librqbit 内嵌)。
//!
//! 线程模型:
//! - 后台线程运行 tokio runtime,持有 [`librqbit::Session`]
//! - UI 通过命令通道(`Add` / `Cancel`)与后台通信,不阻塞
//! - 后台每秒生成任务快照,UI 轮询读取(沿用 500ms 轮询机制)
//!
//! 持久化:
//! - librqbit `SessionPersistenceConfig::Json` 自动保存/恢复任务
//!   (含 torrent 状态、已下载 piece 位图、输出目录),重启后续传
//! - 业务元信息(集标题等)以 `<info_hash>.json` 存于同一目录
//!
//! Tracker 策略:添加任务时将蜜柑磁力携带的 tracker 与公共列表
//! (见 [`TRACKERS`])合并为一份统一列表(去重),不做来源区分。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, Session, SessionOptions,
    SessionPersistenceConfig, TorrentStatsState,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::paths;

/// Tracker 列表(统一,不区分协议与优先级)。
/// 来源:qBittorrent 默认列表 + ngosang/trackerslist 存活项 + ACG 生态。
/// 注意:添加任务时把蜜柑磁力携带的 tracker 与本列表合并(去重),
/// 最终统一生效,不区分来源。
pub const TRACKERS: &[&str] = &[
    "http://tracker.opentrackr.org:1337/announce",
    "https://tracker.gbitt.info:443/announce",
    "udp://tracker.opentrackr.org:1337/announce",
    "udp://tracker.openbittorrent.com:6969/announce",
    "http://tracker.openbittorrent.com:80/announce",
    "https://open.acgnxtracker.com/announce",
    "udp://open.stealth.si:80/announce",
    "https://tr.bangumi.moe:9696/announce",
    "udp://tracker.torrent.eu.org:451/announce",
    "http://tracker.bt4g.com:2095/announce",
    "https://tracker.zhuqiy.com:443/announce",
    "http://share.camoe.cn:8080/announce",
    "http://t.nyaatracker.com/announce",
    "https://tracker.pmman.tech:443/announce",
    "https://tracker.nekomi.cn:443/announce",
    "http://opentracker.acgnx.se/announce",
    "udp://exodus.desync.com:6969/announce",
    "udp://open.demonii.com:1337/announce",
    "udp://explodie.org:6969/announce",
    "udp://zer0day.ch:1337/announce",
    "udp://tracker-udp.gbitt.info:80/announce",
    "udp://tracker.tiny-vps.com:6969/announce",
    "udp://opentracker.i2p.rocks:6969/announce",
    "udp://tracker.moeking.me:6969/announce",
    "udp://tracker.cyberia.is:6969/announce",
    "udp://tracker.leechers-paradise.org:6969/announce",
    "udp://tracker.internetwarriors.net:1337/announce",
];

/// magnet 添加(metadata 获取)超时
const METADATA_TIMEOUT: Duration = Duration::from_secs(60);

/// UI 可见的任务状态
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    /// 获取 metadata / 初始校验中
    Initializing,
    /// 下载中
    Downloading,
    /// 下载完成
    Completed,
    /// 任务已完成但落盘文件被外部删除(UI 回到「下载」态)
    Missing,
    /// 出错(无自动重试,等待用户操作)
    Error(String),
}

/// 任务快照(UI 轮询读取)
#[derive(Debug, Clone, PartialEq)]
pub struct TaskView {
    /// info_hash(十六进制,任务唯一标识)
    pub id: String,
    /// 显示名(集标题)
    pub title: String,
    /// 0.0 ~ 1.0
    pub progress: f64,
    /// 下载速度 B/s
    pub download_rate: u64,
    /// 上传速度 B/s
    pub upload_rate: u64,
    pub downloaded: u64,
    pub total: u64,
    pub state: TaskState,
    /// 当前活跃(已连接)的 peer 数
    pub peers: usize,
    /// 完成后的文件路径(「打开」用)
    pub output_file: Option<PathBuf>,
    /// 任务输出目录(退订清理时按目录定位任务)
    pub output_dir: Option<PathBuf>,
}

/// UI → 后台 命令
pub enum DownloadCmd {
    /// 添加下载任务
    Add {
        magnet: String,
        title: String,
        output_dir: PathBuf,
    },
    /// 取消任务(删除已下载文件)
    Cancel { id: String },
    /// 清理订阅目录(退订):检查进行中任务 → 有则回执阻断,无则取消残留任务并删除目录
    CleanupDir {
        dir: PathBuf,
        bangumi_name: String,
        group_name: String,
    },
}

/// 任务业务元信息(持久化到 `torrents/meta/<hash>.json`)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskMeta {
    title: String,
    output_dir: PathBuf,
}

/// 进行中的添加请求(metadata 获取期间,任务尚未在 librqbit 中注册)。
/// 让 UI 在点击下载后立即获得「获取信息…」反馈。
#[derive(Clone)]
struct PendingAdd {
    title: String,
    output_dir: PathBuf,
}

static PENDING: Mutex<Option<HashMap<String, PendingAdd>>> = Mutex::new(None);

fn pending_insert(hash: String, title: String, output_dir: PathBuf) {
    let mut guard = PENDING.lock().unwrap();
    guard
        .get_or_insert_with(HashMap::new)
        .insert(hash, PendingAdd { title, output_dir });
}

fn pending_remove(hash: &str) {
    let mut guard = PENDING.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        map.remove(hash);
    }
}

/// 已取消的 info_hash(metadata 获取中被取消的任务,完成后立即清理)
static CANCELLED: Mutex<Option<std::collections::HashSet<String>>> = Mutex::new(None);

fn cancelled_insert(hash: &str) {
    let mut guard = CANCELLED.lock().unwrap();
    guard
        .get_or_insert_with(std::collections::HashSet::new)
        .insert(hash.to_string());
}

fn cancelled_take(hash: &str) -> bool {
    let mut guard = CANCELLED.lock().unwrap();
    guard.as_mut().is_some_and(|set| set.remove(hash))
}

/// 快照版本号:每次快照更新时递增,供 UI 轮询触发重绘
static SNAPSHOT_VERSION: AtomicU64 = AtomicU64::new(0);

/// UI 轮询读取:快照版本(变化即需重绘)
pub fn snapshot_version() -> u64 {
    SNAPSHOT_VERSION.load(Ordering::Relaxed)
}

/// 后台事件(UI 通知用):添加失败等
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    /// 添加任务失败(metadata 获取失败/超时等)
    AddFailed { title: String, error: String },
    /// 下载引擎启动失败(后台线程退出,所有下载功能不可用)
    EngineFailed { error: String },
    /// 退订目录清理被阻断:该目录仍有进行中的任务
    UnsubscribeBlocked { dir: PathBuf, titles: Vec<String> },
    /// 退订目录清理完成(目录已删除,UI 可移除订阅记录)
    UnsubscribeDone { dir: PathBuf },
}

static EVENTS: Mutex<Vec<DownloadEvent>> = Mutex::new(Vec::new());

/// 取出积压的后台事件(UI 轮询消费,用于应用内通知)
pub fn take_events() -> Vec<DownloadEvent> {
    std::mem::take(&mut *EVENTS.lock().unwrap())
}

fn push_event(event: DownloadEvent) {
    EVENTS.lock().unwrap().push(event);
}

/// 下载管理器(线程安全的句柄,UI 持有)
pub struct DownloadManager {
    cmd_tx: UnboundedSender<DownloadCmd>,
    snapshot: Arc<Mutex<Vec<TaskView>>>,
}

impl DownloadManager {
    /// 启动下载后台(独立线程 + tokio runtime)。应用生命周期内调用一次。
    pub fn start() -> Arc<Self> {
        Self::start_with_base(paths::app_data_dir())
    }

    /// 启动下载后台,数据根目录可指定(测试隔离用)。
    pub fn start_with_base(base: PathBuf) -> Arc<Self> {
        let (cmd_tx, cmd_rx) = unbounded_channel::<DownloadCmd>();
        let snapshot: Arc<Mutex<Vec<TaskView>>> = Arc::new(Mutex::new(Vec::new()));
        let snap = snapshot.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("构建下载 tokio runtime 失败");
            rt.block_on(run_loop(cmd_rx, snap, base));
        });
        Arc::new(Self { cmd_tx, snapshot })
    }

    /// 发送命令(线程安全,不阻塞)。接收端退出时返回 Err。
    pub fn send(&self, cmd: DownloadCmd) -> Result<(), String> {
        self.cmd_tx
            .send(cmd)
            .map_err(|e| format!("下载引擎不可用: {e}"))
    }

    /// 读取任务快照(UI 轮询)
    pub fn snapshot(&self) -> Vec<TaskView> {
        self.snapshot.lock().unwrap().clone()
    }
}

/// 后台主循环:session 生命周期 + 命令处理 + 周期快照
async fn run_loop(
    mut cmd_rx: UnboundedReceiver<DownloadCmd>,
    snapshot: Arc<Mutex<Vec<TaskView>>>,
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
        Ok(s) => s,
        Err(e) => {
            let msg = format!("下载引擎启动失败: {e:#}");
            eprintln!("{msg}");
            // 通知 UI:下载功能不可用(否则所有下载点击都会静默失效)
            push_event(DownloadEvent::EngineFailed { error: msg });
            return;
        }
    };
    eprintln!("下载引擎就绪(已恢复持久化任务)");

    // 孤儿清理:meta 有而 session 无的任务元信息(任务已不存在)删除
    cleanup_orphan_meta(&persist_dir, &meta_dir);

    // 周期快照
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(cmd) => {
                        // 命令在独立 task 中执行,不阻塞快照循环:
                        // Add 的 metadata 获取可能耗时数十秒,期间快照仍需更新
                        let session = session.clone();
                        let meta_dir = meta_dir.clone();
                        tokio::spawn(async move {
                            handle_cmd(&session, &meta_dir, cmd).await;
                        });
                    }
                    None => break,
                }
            }
            _ = tick.tick() => {
                update_snapshot(&session, &meta_dir, &snapshot);
            }
        }
    }
    session.stop().await;
}

/// 处理一条下载命令
async fn handle_cmd(session: &Arc<Session>, meta_dir: &Path, cmd: DownloadCmd) {
    match cmd {
        DownloadCmd::Add {
            magnet,
            title,
            output_dir,
        } => {
            // 合并磁力携带的 tracker 与公共列表为统一列表(去重)
            // librqbit 对 magnet 只使用磁力 URL 自带的 tr 参数,所以在这里注入
            let hash = magnet_info_hash(&magnet);
            let mut m = match librqbit::Magnet::parse(&magnet) {
                Ok(m) => m,
                Err(e) => {
                    let msg = format!("磁力解析失败: {e}");
                    eprintln!("{msg}");
                    push_event(DownloadEvent::AddFailed {
                        title: title.clone(),
                        error: msg,
                    });
                    return;
                }
            };
            // 合并 tracker 为一份统一列表(去重)
            let mut seen: std::collections::HashSet<String> = m.trackers.iter().cloned().collect();
            for t in TRACKERS {
                if seen.insert(t.to_string()) {
                    m.trackers.push(t.to_string());
                }
            }
            eprintln!("「{title}」tracker: 共 {} 个", m.trackers.len());
            let magnet = m.to_string();

            // 立即登记「获取信息中」状态(metadata 获取期间 UI 有反馈)
            if let Some(hash) = hash.clone() {
                pending_insert(hash, title.clone(), output_dir.clone());
            }

            let add_opts = AddTorrentOptions {
                output_folder: Some(output_dir.to_string_lossy().into_owned()),
                overwrite: true,
                ..Default::default()
            };
            // 已在会话中的同名任务(如文件被外部删除后重新下载):
            // 先移除旧任务与其元信息,再重新添加
            if let Some(hash) = &hash {
                let existing: Vec<_> = session.with_torrents(|it| {
                    it.filter_map(|(tid, h)| {
                        (h.info_hash().as_string().to_lowercase() == *hash).then_some(tid)
                    })
                    .collect()
                });
                if !existing.is_empty() {
                    for tid in existing {
                        let _ = session.delete(tid.into(), true).await;
                    }
                    let _ = std::fs::remove_file(meta_dir.join(format!("{hash}.json")));
                    // 空目录清理(同目录其他集的文件保留)
                    let _ = std::fs::remove_dir(&output_dir);
                    eprintln!("「{title}」重新下载:已移除旧任务");
                }
            }
            let fut = session.add_torrent(AddTorrent::Url(magnet.into()), Some(add_opts));
            let resp = match tokio::time::timeout(METADATA_TIMEOUT, fut).await {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    if let Some(hash) = &hash {
                        pending_remove(hash);
                    }
                    let msg = format!("添加任务失败: {e:#}");
                    eprintln!("{msg}");
                    push_event(DownloadEvent::AddFailed {
                        title: title.clone(),
                        error: msg,
                    });
                    return;
                }
                Err(_) => {
                    if let Some(hash) = &hash {
                        pending_remove(hash);
                    }
                    let msg =
                        format!("获取资源信息超时({METADATA_TIMEOUT:?}),该资源可能暂无可用来源");
                    eprintln!("{msg}");
                    push_event(DownloadEvent::AddFailed {
                        title: title.clone(),
                        error: msg,
                    });
                    return;
                }
            };
            match resp {
                AddTorrentResponse::Added(_, handle) => {
                    let hash = handle.info_hash().as_string().to_lowercase();
                    pending_remove(&hash);
                    // metadata 获取期间被取消:立即删除任务,不留下产物
                    if cancelled_take(&hash) {
                        let id = handle.id();
                        let _ = session.delete(id.into(), true).await;
                        // 清理残留的空目录(仅当为空时移除,保留同目录其他集的文件)
                        let _ = std::fs::remove_dir(&output_dir);
                        eprintln!("「{title}」在获取信息期间被取消,已清理");
                        return;
                    }
                    let meta = TaskMeta { title, output_dir };
                    let path = meta_dir.join(format!("{hash}.json"));
                    if let Ok(v) = serde_json::to_vec_pretty(&meta)
                        && let Err(e) = std::fs::write(&path, v)
                    {
                        eprintln!("保存任务元信息失败: {e}");
                    }
                    eprintln!("「{}」已开始下载", meta.title);
                }
                AddTorrentResponse::AlreadyManaged(_, _) => {
                    // 并发重复添加(如快速双击下载):任务已在会话中,以先添加的为准
                    if let Some(hash) = &hash {
                        pending_remove(hash);
                    }
                    eprintln!("「{title}」已在会话中,忽略重复添加");
                }
                AddTorrentResponse::ListOnly(_) => {
                    // 本应用不产生 list-only 响应,仅做防御性清理
                    if let Some(hash) = &hash {
                        pending_remove(hash);
                    }
                }
            }
        }
        DownloadCmd::Cancel { id } => {
            // 先读元信息(取消日志需要标题)
            let meta_path = meta_dir.join(format!("{id}.json"));
            let meta: Option<TaskMeta> = std::fs::read_to_string(&meta_path)
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok());
            // metadata 获取中(任务未注册)的取消:移除进行中标记 + 登记取消
            pending_remove(&id);
            cancelled_insert(&id);
            let targets: Vec<_> = session.with_torrents(|it| {
                it.filter_map(|(tid, h)| {
                    (h.info_hash().as_string().to_lowercase() == id).then_some(tid)
                })
                .collect()
            });
            for tid in targets {
                if let Err(e) = session.delete(tid.into(), true).await {
                    eprintln!("取消任务失败: {e:#}");
                }
            }
            // 删除元信息,并清理可能遗留的空输出目录
            let _ = std::fs::remove_file(&meta_path);
            if let Some(meta) = meta {
                // 只删除空目录(该番剧其他集的文件保留)
                let _ = std::fs::remove_dir(&meta.output_dir);
                eprintln!("「{}」已取消,文件已删除", meta.title);
            }
        }
        DownloadCmd::CleanupDir {
            dir,
            bangumi_name,
            group_name,
        } => {
            // 退订清理:检查与取消、删目录在同一线程内原子完成,
            // 不依赖 UI 侧的周期快照(避免「刚点下载就退订」的竞态窗口)。

            // 1. 收集该目录下所有任务元信息(hash → meta)
            let mut metas: Vec<(String, TaskMeta)> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(meta_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "json")
                        && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                        && let Ok(text) = std::fs::read_to_string(&path)
                        && let Ok(meta) = serde_json::from_str::<TaskMeta>(&text)
                        && meta.output_dir == dir
                    {
                        metas.push((stem.to_string(), meta));
                    }
                }
            }

            // 2. 检查进行中的任务(session 内未完成 + PENDING 中获取信息的)
            let mut active: Vec<String> = Vec::new();
            for (hash, meta) in &metas {
                let in_session = session.with_torrents(|it| {
                    let mut found = false;
                    for (_, h) in it {
                        if h.info_hash().as_string().to_lowercase() == *hash {
                            // 完成/出错视为非活动;其余(初始化、下载、暂停)都算进行中
                            found = !h.stats().finished;
                            break;
                        }
                    }
                    found
                });
                if in_session {
                    active.push(meta.title.clone());
                }
            }
            {
                let pendings = PENDING.lock().unwrap().clone().unwrap_or_default();
                for p in pendings.values() {
                    if p.output_dir == dir {
                        active.push(p.title.clone());
                    }
                }
            }

            if !active.is_empty() {
                // 有进行中任务:阻断退订,由 UI 弹出警告窗口
                let count = active.len();
                eprintln!("退订「{bangumi_name} - {group_name}」被阻断:{count} 个剧集正在下载");
                push_event(DownloadEvent::UnsubscribeBlocked {
                    dir: dir.clone(),
                    titles: active,
                });
                return;
            }

            // 3. 取消残留任务(完成态做种 / 错误态),删除元信息
            for (hash, meta) in &metas {
                let targets: Vec<_> = session.with_torrents(|it| {
                    it.filter_map(|(tid, h)| {
                        (h.info_hash().as_string().to_lowercase() == *hash).then_some(tid)
                    })
                    .collect()
                });
                for tid in targets {
                    let _ = session.delete(tid.into(), true).await;
                }
                let _ = std::fs::remove_file(meta_dir.join(format!("{hash}.json")));
                eprintln!("退订清理:「{}」已取消", meta.title);
            }
            // 4. 删除整个目录(含已下载完成的视频文件;此时已无任何任务持有该目录)
            match std::fs::remove_dir_all(&dir) {
                Ok(()) => {
                    eprintln!("退订「{bangumi_name} - {group_name}」:下载目录已删除");
                    push_event(DownloadEvent::UnsubscribeDone { dir });
                }
                Err(e) => {
                    eprintln!("退订「{bangumi_name} - {group_name}」:删除目录失败 {e}");
                    // 目录删除失败也放行退订(文件残留可手动清理),
                    // 避免订阅记录与下载状态永久卡住
                    push_event(DownloadEvent::UnsubscribeDone { dir });
                }
            }
        }
    }
}

/// 创建 session:DHT 初始化失败(如端口被占用,常见于双开)时
/// 降级为禁用 DHT 重试一次,保证 tracker 下载路径可用。
async fn create_session(persist_dir: &Path, dht_file: &Path) -> anyhow::Result<Arc<Session>> {
    use librqbit::dht::DhtPersistenceConfig;
    use librqbit::{DhtSessionConfig, ListenerMode, ListenerOptions};
    use std::net::Ipv4Addr;

    let make_opts = |disable_dht: bool| SessionOptions {
        // 入站监听:TCP + uTP(对 NAT 友好、覆盖只用 uTP 的做种者),随机端口 + UPnP 转发
        listen: Some(ListenerOptions {
            mode: ListenerMode::TcpAndUtp,
            listen_addr: (Ipv4Addr::UNSPECIFIED, 0).into(),
            enable_upnp_port_forwarding: true,
            ..Default::default()
        }),
        fastresume: true,
        persistence: Some(SessionPersistenceConfig::Json {
            folder: Some(persist_dir.to_path_buf()),
        }),
        // DHT 路由表持久化到应用数据目录(重建成本高,不属于可清理缓存)
        dht: (!disable_dht).then(|| DhtSessionConfig {
            persistence: Some(DhtPersistenceConfig {
                config_filename: Some(dht_file.to_path_buf()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    match Session::new_with_opts(paths::video_dir(), make_opts(false)).await {
        Ok(s) => Ok(s),
        Err(e) => {
            eprintln!("下载引擎初始化失败,尝试禁用 DHT 降级: {e:#}");
            Session::new_with_opts(paths::video_dir(), make_opts(true))
                .await
                .map_err(|e2| anyhow::anyhow!("{e2:#}(降级后仍失败)"))
        }
    }
}

/// 孤儿清理:元信息存在但对应任务已不在持久化中的,删除 meta 文件。
/// 反向(missing meta、session 存在)不处理——续传数据误删不可恢复,
/// UI 用 torrent 名兜底。
fn cleanup_orphan_meta(session_dir: &Path, meta_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(meta_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            // session 目录中对应任务以 <hash>.torrent 存在
            let torrent = session_dir.join(format!("{stem}.torrent"));
            if !torrent.exists() && std::fs::remove_file(&path).is_ok() {
                eprintln!("已清理无效的任务记录");
            }
        }
    }
}

/// 生成任务快照(每秒)
fn update_snapshot(session: &Arc<Session>, meta_dir: &Path, snapshot: &Arc<Mutex<Vec<TaskView>>>) {
    // 读取业务元信息
    let mut metas: HashMap<String, TaskMeta> = HashMap::new();
    if let Ok(entries) = std::fs::read_dir(meta_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && let Ok(text) = std::fs::read_to_string(&path)
                && let Ok(meta) = serde_json::from_str::<TaskMeta>(&text)
            {
                metas.insert(stem.to_string(), meta);
            }
        }
    }
    // 进行中的添加请求:先以「获取信息…」状态展示(点击下载后的即时反馈),
    // 随后被真实任务(同 info_hash)覆盖
    let mut views: std::collections::HashMap<String, TaskView> = {
        let pendings = PENDING.lock().unwrap().clone().unwrap_or_default();
        pendings
            .into_iter()
            .map(|(hash, p)| {
                (
                    hash.clone(),
                    TaskView {
                        id: hash,
                        title: p.title,
                        progress: 0.0,
                        download_rate: 0,
                        upload_rate: 0,
                        downloaded: 0,
                        total: 0,
                        state: TaskState::Initializing,
                        peers: 0,
                        output_file: None,
                        output_dir: Some(p.output_dir),
                    },
                )
            })
            .collect()
    };
    let session_views = session.with_torrents(|it| {
        let mut out = Vec::new();
        for (_, h) in it {
            let hash = h.info_hash().as_string().to_lowercase();
            let stats = h.stats();
            let meta = metas.get(&hash);

            let (state, output_file) = match &stats.state {
                TorrentStatsState::Initializing { .. } => (TaskState::Initializing, None),
                TorrentStatsState::Live if stats.finished => {
                    // 完成:尝试获取落盘文件路径(一般单文件,取第一个)。
                    // 种子内部文件名来自远端元数据,必须清洗并校验路径
                    // 仍在输出目录内(防路径逃逸)。
                    let file = h
                        .with_metadata(|m| {
                            let mut it = m.info.iter_file_details();
                            it.next().map(|fd| fd.filename.to_pathbuf())
                        })
                        .unwrap_or_default()
                        .unwrap_or_default();
                    let out = meta.and_then(|m| {
                        let safe = crate::paths::sanitize_file_name(&file.to_string_lossy());
                        if safe.is_empty() {
                            return None;
                        }
                        let p = m.output_dir.join(safe);
                        p.starts_with(&m.output_dir).then_some(p)
                    });
                    // 文件被外部删除时标记 Missing,UI 回到「下载」态
                    let state = match &out {
                        Some(p) if !p.exists() => TaskState::Missing,
                        _ => TaskState::Completed,
                    };
                    (state, out)
                }
                TorrentStatsState::Live => (TaskState::Downloading, None),
                TorrentStatsState::Paused => (TaskState::Downloading, None),
                TorrentStatsState::Error => (
                    TaskState::Error(stats.error.clone().unwrap_or_else(|| "未知错误".into())),
                    None,
                ),
            };

            out.push(TaskView {
                id: hash.clone(),
                title: meta
                    .map(|m| m.title.clone())
                    .unwrap_or_else(|| h.name().unwrap_or_default()),
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
                // 活跃 peer 数(诊断下载速度:0 = 没找到做种者,非 0 = 做种者带宽低)
                peers: stats
                    .live
                    .as_ref()
                    .map(|l| l.snapshot.peer_stats.live as usize)
                    .unwrap_or(0),
                state,
                output_file,
                output_dir: meta.map(|m| m.output_dir.clone()),
            });
        }
        out
    });
    for v in session_views {
        views.insert(v.id.clone(), v);
    }
    let views: Vec<TaskView> = views.into_values().collect();

    // 仅在内容变化时递增版本,避免无任务时空转重绘
    let mut guard = snapshot.lock().unwrap();
    if *guard != views {
        *guard = views;
        SNAPSHOT_VERSION.fetch_add(1, Ordering::Relaxed);
    }
}

/// 校验输出目录是否可用(存在或可创建、可写)
pub fn ensure_output_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    Ok(())
}

/// 从磁力链接提取 info_hash(btih,hex 小写)。
/// 例: `magnet:?xt=urn:btih:abc123...` → `abc123...`
pub fn magnet_info_hash(magnet: &str) -> Option<String> {
    let lower = magnet.to_lowercase();
    // "xt=urn:btih:" 共 12 个字符
    let idx = lower.find("xt=urn:btih:")?;
    let rest = &lower[idx + 12..];
    let hex: String = rest.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
    (hex.len() == 40).then_some(hex)
}

/// 速度格式化: `1536` → `1.5 MB/s`, `512` → `512 B/s`
pub fn format_rate(bytes_per_sec: u64) -> String {
    if bytes_per_sec >= 1024 * 1024 {
        format!("{:.1} MB/s", bytes_per_sec as f64 / 1024.0 / 1024.0)
    } else if bytes_per_sec >= 1024 {
        format!("{:.0} KB/s", bytes_per_sec as f64 / 1024.0)
    } else {
        format!("{bytes_per_sec} B/s")
    }
}

/// 百分比格式化: `0.375` → `37%`
pub fn format_percent(progress: f64) -> String {
    format!("{:.0}%", progress * 100.0)
}

#[cfg(test)]
mod tests {
    use super::magnet_info_hash;

    #[test]
    fn extracts_btih_hex() {
        let magnet = "magnet:?xt=urn:btih:c06e0fa66e76e5f30d10e4b00eaa2472b6d62a37&tr=http%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce&dn=test";
        assert_eq!(
            magnet_info_hash(magnet).as_deref(),
            Some("c06e0fa66e76e5f30d10e4b00eaa2472b6d62a37")
        );
    }

    #[test]
    fn rejects_non_40hex() {
        assert_eq!(magnet_info_hash("magnet:?xt=urn:btih:abc"), None);
        assert_eq!(magnet_info_hash("no-magnet"), None);
    }
}
