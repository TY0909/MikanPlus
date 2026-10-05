//! HTTP transport: shared Agent (connection pool), proxy detection, and body decoding.

use std::io::Read;
use std::time::Duration;

use crate::SourceError;

/// Total request timeout (covers the connection and the entire response-body download; requests
/// run on background threads, so relaxing it will not block the UI).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// TCP connection timeout.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// User agent: identifies the client and follows etiquette.
const USER_AGENT: &str = "MikanPlus/0.1 (+https://mikanani.me)";

/// Builds the shared `ureq::Agent` (connection pooling + proxy from the environment / macOS system
/// settings). The agent is built lazily on the first request.
pub(crate) fn build_agent() -> ureq::Agent {
    let mut builder = ureq::AgentBuilder::new()
        .timeout(REQUEST_TIMEOUT)
        .timeout_connect(CONNECT_TIMEOUT)
        .user_agent(USER_AGENT);
    // Proxy: environment variables take priority, with the macOS system proxy as a fallback
    // (probed only once per process)
    if let Some(proxy) = env_proxy().or_else(system_proxy) {
        builder = builder.proxy(proxy);
    }
    builder.build()
}

/// Proxy from environment variables (HTTPS_PROXY / HTTP_PROXY / ALL_PROXY).
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

/// macOS system proxy (Network settings → Proxies; read via `scutil --proxy`).
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

/// Reads the `Key : value` value from scutil output.
#[cfg(target_os = "macos")]
fn line_value(text: &str, key: &str) -> Option<String> {
    text.lines()
        .find(|l| l.trim_start().starts_with(key))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Whether a scutil boolean value is 1.
#[cfg(target_os = "macos")]
fn line_bool(text: &str, key: &str) -> bool {
    line_value(text, key).is_some_and(|v| v == "1")
}

/// Maps a ureq error to a dispatchable error type.
fn map_ureq_error(e: ureq::Error) -> SourceError {
    match e {
        ureq::Error::Status(code, _) => SourceError::Server(code),
        ureq::Error::Transport(_) => SourceError::Network,
    }
}

/// Reads and decodes the response body into a UTF-8 string.
///
/// Distinguishes two kinds of failure: read errors (timeout / interrupted connection) belong to
/// the transport layer and are classified as [`Interrupted`](SourceError::Interrupted); only
/// invalid response-body bytes (or exceeding the size limit) are classified as
/// [`Decode`](SourceError::Decode).
fn read_html_body(response: ureq::Response) -> Result<String, SourceError> {
    // A Mikan search-results page returns all matching episodes at once and can reach several MB,
    // so the limit is relaxed to 32MB
    const MAX_HTML_BYTES: usize = 32 * 1024 * 1024;
    let mut buf: Vec<u8> = Vec::new();
    response
        .into_reader()
        .take((MAX_HTML_BYTES + 1) as u64)
        .read_to_end(&mut buf)
        .map_err(|_| SourceError::Interrupted)?;
    if buf.len() > MAX_HTML_BYTES {
        return Err(SourceError::Decode);
    }
    // Origin pages are UTF-8; treat as a decode failure only when the bytes themselves are invalid
    String::from_utf8(buf).map_err(|_| SourceError::Decode)
}

/// Fetches and decodes an HTML page.
pub(crate) fn get_html(agent: &ureq::Agent, url: &str) -> Result<String, SourceError> {
    agent
        .get(url)
        .call()
        .map_err(map_ureq_error)
        .and_then(read_html_body)
}

/// Size limit for binary content (images); exceeding it returns Err (no caching, no success
/// marker) to avoid silent truncation.
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

/// Downloads binary content (images).
pub(crate) fn get_bytes(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, SourceError> {
    agent.get(url).call().map_err(map_ureq_error).and_then(|r| {
        // Read one extra byte to detect truncation: exceeding the limit must be reported
        // explicitly
        let mut buf: Vec<u8> = Vec::new();
        r.into_reader()
            .take((MAX_IMAGE_BYTES + 1) as u64)
            .read_to_end(&mut buf)
            .map_err(|_| SourceError::Interrupted)?;
        if buf.len() > MAX_IMAGE_BYTES {
            return Err(SourceError::ImageTooLarge);
        }
        Ok(buf)
    })
}
