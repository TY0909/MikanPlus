//! 蜜柑计划网络层。
//!
//! 访问策略(核心诉求:尽可能少地访问服务器):
//! - **节流**:全局请求间最小间隔,避免突发流量
//! - **退避**:失败后一段时间内不再请求同一资源(指数级)
//! - **并发克制**:最多 2 个在途请求
//! - **去重**:同一 URL 同时只允许一个下载任务(图片管理器保证)
//! - 缓存命中(见 cache.rs)时根本不会走到这里

use std::{
    collections::HashMap,
    io::Read,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

/// 站点根地址
pub const BASE_URL: &str = "https://mikanani.me";

/// 请求间最小间隔
const MIN_INTERVAL: Duration = Duration::from_millis(300);
/// 失败后的基础退避时间
const BACKOFF_BASE: Duration = Duration::from_secs(30);
/// 最大并发请求数
const MAX_CONCURRENCY: usize = 2;

/// 用户代理:表明身份,遵守礼仪
const USER_AGENT: &str = "MikanPlus/0.1 (+https://mikanani.me)";

struct NetState {
    /// 上次请求时间(节流)
    last_request: Option<Instant>,
    /// 正在进行的请求数
    inflight: usize,
    /// 退避表:url → 允许再次请求的最早时间
    backoff: HashMap<String, Instant>,
    /// 连续失败次数(全局退避放大)
    fail_streak: u32,
}

static STATE: Mutex<Option<NetState>> = Mutex::new(None);

/// 图片下载任务版本号:图片缓存变化时递增,供 UI 轮询刷新
static IMAGE_VERSION: AtomicU64 = AtomicU64::new(0);

pub fn image_version() -> u64 {
    IMAGE_VERSION.load(Ordering::Relaxed)
}

pub fn bump_image_version() {
    IMAGE_VERSION.fetch_add(1, Ordering::Relaxed);
}

/// 图片状态:同一 URL 同时只允许一个下载任务(去重)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImgStatus {
    /// 尚未下载
    Pending,
    /// 下载中
    Loading,
    /// 已下载(缓存文件已就绪)
    Loaded,
    /// 下载失败(冷却后允许重试)
    Error,
}

/// 图片任务表:url → 状态
static IMAGES: Mutex<Option<HashMap<String, ImgStatus>>> = Mutex::new(None);

/// 图片失败时间表:url → 失败时刻(冷却期后允许重试)
static IMAGE_ERRORS: Mutex<Option<HashMap<String, Instant>>> = Mutex::new(None);

/// 图片失败后的重试冷却期
const IMAGE_RETRY_COOLDOWN: Duration = Duration::from_secs(60);

/// 查询图片状态
pub fn image_status(url: &str) -> ImgStatus {
    let mut guard = IMAGES.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.get(url).copied().unwrap_or(ImgStatus::Pending)
}

/// 尝试认领下载任务:同一 URL 只有第一个调用返回 true;
/// 失败态在冷却期结束后允许重新认领(自动重试)。
pub fn claim_image(url: &str) -> bool {
    let mut guard = IMAGES.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    match map.get(url) {
        Some(ImgStatus::Pending) | None => {
            map.insert(url.to_string(), ImgStatus::Loading);
            true
        }
        Some(ImgStatus::Error) => {
            let cooldown_ok = IMAGE_ERRORS
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|m| m.get(url))
                .is_none_or(|at| Instant::now().duration_since(*at) >= IMAGE_RETRY_COOLDOWN);
            if cooldown_ok {
                map.insert(url.to_string(), ImgStatus::Loading);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

/// 记录图片下载结果
pub fn finish_image(url: &str, ok: bool) {
    let mut guard = IMAGES.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(
        url.to_string(),
        if ok {
            ImgStatus::Loaded
        } else {
            ImgStatus::Error
        },
    );
    if !ok {
        IMAGE_ERRORS
            .lock()
            .unwrap()
            .get_or_insert_with(HashMap::new)
            .insert(url.to_string(), Instant::now());
    }
    bump_image_version();
}

/// 该 URL 是否在退避期内(过期条目惰性清理,保证退避表有界)
fn in_backoff(url: &str) -> bool {
    with_state(|s| match s.backoff.get(url) {
        Some(until) if Instant::now() >= *until => {
            s.backoff.remove(url);
            false
        }
        Some(_) => true,
        None => false,
    })
}

/// 清除指定 URL 的退避状态(用户主动发起请求前调用,如点击「重试」)。
/// 退避仅用于抑制程序自动重试;用户明确的操作应立即放行。
pub fn reset_backoff(url: &str) {
    with_state(|s| {
        s.backoff.remove(url);
    });
}

/// 对全局状态执行一次操作
fn with_state<T>(f: impl FnOnce(&mut NetState) -> T) -> T {
    let mut guard = STATE.lock().unwrap();
    let state = guard.get_or_insert_with(|| NetState {
        last_request: None,
        inflight: 0,
        backoff: HashMap::new(),
        fail_streak: 0,
    });
    f(state)
}

/// 等待全局节流间隔与并发槽(阻塞当前线程,仅在后台线程调用)。
///
/// 并发检查与节流检查在同一临界区内完成,保证「最多 N 个在途请求」
/// 不变量严格成立(两段式检查存在竞态窗口)。
fn acquire_slot() {
    loop {
        let now = Instant::now();
        let ok = with_state(|s| {
            if s.inflight >= MAX_CONCURRENCY {
                return false;
            }
            match s.last_request {
                Some(last) if now.duration_since(last) < MIN_INTERVAL => false,
                _ => {
                    s.last_request = Some(now);
                    s.inflight += 1;
                    true
                }
            }
        });
        if ok {
            break;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
}

fn release_slot() {
    with_state(|s| s.inflight -= 1);
}

/// 记录失败:对该 URL 退避(时长随全局连续失败次数放大),并更新连续失败计数
fn note_failure(url: &str) {
    with_state(|s| {
        s.fail_streak += 1;
        let factor = 1u32 << s.fail_streak.min(4); // 30s → 60s → 120s → 240s → 480s
        let until = Instant::now() + BACKOFF_BASE * factor;
        s.backoff.insert(url.to_string(), until);
    });
}

fn note_success() {
    with_state(|s| s.fail_streak = 0);
}

/// 全局共享的 ureq Agent(复用连接池,避免每请求重建)
static AGENT: std::sync::OnceLock<ureq::Agent> = std::sync::OnceLock::new();

fn agent() -> ureq::Agent {
    AGENT
        .get_or_init(|| {
            let mut builder = ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(15))
                .timeout_connect(Duration::from_secs(10))
                .user_agent(USER_AGENT);
            // 代理:环境变量优先,macOS 系统代理兜底(进程内只探测一次)
            if let Some(proxy) = env_proxy().or_else(system_proxy) {
                builder = builder.proxy(proxy);
            }
            builder.build()
        })
        .clone()
}

/// 环境变量代理(HTTPS_PROXY / HTTP_PROXY / ALL_PROXY)
fn env_proxy() -> Option<ureq::Proxy> {
    for var in [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        if let Ok(v) = std::env::var(var)
            && !v.trim().is_empty()
            && let Ok(p) = ureq::Proxy::new(v.trim())
        {
            return Some(p);
        }
    }
    None
}

/// macOS 系统代理(网络设置 → 代理;通过 scutil --proxy 读取)
#[cfg(target_os = "macos")]
fn system_proxy() -> Option<ureq::Proxy> {
    let out = std::process::Command::new("scutil")
        .arg("--proxy")
        .output()
        .ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    for (enable, host_key, port_key) in [
        ("HTTPSEnable", "HTTPSProxy", "HTTPSPort"),
        ("HTTPEnable", "HTTPProxy", "HTTPPort"),
    ] {
        if !line_bool(&text, enable) {
            continue;
        }
        let Some(host) = line_value(&text, host_key) else {
            continue;
        };
        let Some(port) = line_value(&text, port_key) else {
            continue;
        };
        let url = format!("http://{host}:{port}");
        if let Ok(p) = ureq::Proxy::new(url) {
            return Some(p);
        }
    }
    None
}

#[cfg(not(target_os = "macos"))]
fn system_proxy() -> Option<ureq::Proxy> {
    None
}

/// 读取 scutil 输出中 `Key : value` 的值
#[cfg(target_os = "macos")]
fn line_value(text: &str, key: &str) -> Option<String> {
    text.lines()
        .find(|l| l.trim_start().starts_with(key))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// scutil 布尔值是否为 1
#[cfg(target_os = "macos")]
fn line_bool(text: &str, key: &str) -> bool {
    line_value(text, key).is_some_and(|v| v == "1")
}

/// 抓取 HTML 页面(带节流/退避/并发控制)。退避期内返回 Err。
pub fn fetch_html(url: &str) -> Result<String, String> {
    if in_backoff(url) {
        return Err("请求过于频繁,请稍后重试".into());
    }
    acquire_slot();
    let result = agent()
        .get(url)
        .call()
        .map_err(|e| format!("{e}"))
        .and_then(|r| r.into_string().map_err(|e| format!("{e}")));
    release_slot();
    match result {
        Ok(text) => {
            note_success();
            Ok(text)
        }
        Err(e) => {
            note_failure(url);
            Err(e)
        }
    }
}

/// 下载二进制内容(图片)。带节流/退避/并发控制;退避期内返回 Err。
/// 超过大小上限时返回 Err(不缓存、不标记成功),避免静默截断。
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

pub fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    if in_backoff(url) {
        return Err("请求过于频繁,请稍后重试".into());
    }
    acquire_slot();
    let result = agent()
        .get(url)
        .call()
        .map_err(|e| format!("{e}"))
        .and_then(|r| {
            // 多读 1 字节以检测截断:超限必须显式报错
            let mut buf: Vec<u8> = Vec::new();
            r.into_reader()
                .take((MAX_IMAGE_BYTES + 1) as u64)
                .read_to_end(&mut buf)
                .map_err(|e| format!("{e}"))?;
            if buf.len() > MAX_IMAGE_BYTES {
                return Err(format!("图片超过 {}MB 上限", MAX_IMAGE_BYTES / 1024 / 1024));
            }
            Ok(buf)
        });
    release_slot();
    match result {
        Ok(bytes) => {
            note_success();
            Ok(bytes)
        }
        Err(e) => {
            note_failure(url);
            Err(e)
        }
    }
}

/// 拼接站点完整 URL;仅接受 http/https scheme(大小写不敏感),
/// 防止解析出的坏链接拼出畸形 URL 或指向 file:// 等本地资源。
pub fn site_url(path: &str) -> String {
    let lower = path.trim().to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        path.trim().to_string()
    } else if lower.starts_with("//") {
        format!("https:{}", path.trim())
    } else {
        format!("{BASE_URL}{path}")
    }
}

/// 搜索页 URL(关键词 URL 编码)
pub fn search_url(query: &str) -> String {
    let mut encoded = String::new();
    for b in query.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char)
            }
            _ => encoded.push_str(&format!("%{b:02X}")),
        }
    }
    format!("{BASE_URL}/Home/Search?searchstr={encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_url_join() {
        assert_eq!(
            site_url("/Home/Bangumi"),
            "https://mikanani.me/Home/Bangumi"
        );
        assert_eq!(
            site_url("https://other.com/x"),
            "https://other.com/x",
            "绝对地址原样返回"
        );
    }

    #[test]
    fn image_claim_dedup() {
        let url = "https://example.com/a.jpg";
        assert!(claim_image(url), "首次认领成功");
        assert!(!claim_image(url), "第二次认领被拒绝(去重)");
        finish_image(url, true);
        assert_eq!(image_status(url), ImgStatus::Loaded);
        assert!(!claim_image(url), "已加载不再重复下载");
    }

    #[test]
    fn backoff_blocks_then_resets() {
        // 模拟一次失败:进入退避
        with_state(|s| {
            s.fail_streak = 1;
            s.backoff.insert(
                "https://example.com/page".into(),
                Instant::now() + Duration::from_secs(60),
            );
        });
        assert!(in_backoff("https://example.com/page"), "失败后应处于退避期");

        // 用户主动重试:清除退避后立即放行
        reset_backoff("https://example.com/page");
        assert!(!in_backoff("https://example.com/page"), "重置后不再退避");
        // 其他 URL 不受影响
        assert!(!in_backoff("https://example.com/other"));
    }
}
