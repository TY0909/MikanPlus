//! Creation of the librqbit session and connection-status diagnostics.

use std::path::Path;
use std::sync::Arc;

use librqbit::{
    ConnectionOptions, DhtSessionConfig, ListenerMode, ListenerOptions, Session, SessionOptions,
    SessionPersistenceConfig,
};

use storage::paths;

use crate::data::model::{DownloadError, DownloadStatus};
use crate::engine::manager::EngineState;

/// librqbit uses a random port by default; only a fixed port lets port mappings /
/// firewall rules survive restarts. Falls back to a random port when it is occupied.
const DEFAULT_LISTEN_PORT: u16 = 6881;

/// Create a session. It tries the following in order and returns on the first success:
/// 1. fixed port + DHT; 2. random port + DHT (the fixed port is occupied);
/// 3. random port with DHT disabled (port conflicts often mean a second instance is
///    running; with DHT off, trackers alone can still sustain downloads).
pub(crate) async fn create_session(
    persist_dir: &Path,
    dht_file: &Path,
) -> Result<Arc<Session>, DownloadError> {
    use librqbit::dht::DhtPersistenceConfig;

    let proxy = socks_proxy();
    let make_opts = |port: u16, enable_dht: bool| SessionOptions {
        // Inbound listening: TCP + uTP (friendly to NAT and covers uTP-only seeders), fixed port + UPnP forwarding
        listen: Some(listener_options(port)),
        connect: proxy.as_ref().map(|url| ConnectionOptions {
            proxy_url: Some(url.clone()),
            ..Default::default()
        }),
        fastresume: true,
        persistence: Some(SessionPersistenceConfig::Json {
            folder: Some(persist_dir.to_path_buf()),
        }),
        // Persist the DHT routing table to the application data directory (expensive to rebuild, so not disposable cache)
        dht: enable_dht.then(|| DhtSessionConfig {
            persistence: Some(DhtPersistenceConfig {
                config_filename: Some(dht_file.to_path_buf()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    for (port, enable_dht) in [(DEFAULT_LISTEN_PORT, true), (0, true), (0, false)] {
        match Session::new_with_opts(paths::video_dir(), make_opts(port, enable_dht)).await {
            Ok(session) => return Ok(session),
            Err(error) => eprintln!(
                "下载引擎初始化失败(端口 {port}, DHT {}): {error:#}",
                if enable_dht { "开" } else { "关" }
            ),
        }
    }
    Err(DownloadError::EngineInit)
}

/// Aggregate connection status (diagnoses NAT / inbound reachability): listening port,
/// DHT nodes, and per-category peer counts.
pub(crate) fn update_status(session: &Arc<Session>, state: &EngineState) {
    let listen = session.listen_addr();
    let dht = session.get_dht().map(|dht| dht.stats());
    let peers = session.with_torrents(|it| {
        let mut totals = PeerTotals::default();
        for (_, handle) in it {
            if let Some(live) = handle.stats().live.as_ref() {
                let stats = &live.snapshot.peer_stats;
                totals.live += stats.live;
                totals.seen += stats.seen;
                totals.connecting += stats.connecting;
                totals.tcp += stats.live_tcp;
                totals.utp += stats.live_utp;
            }
        }
        totals
    });
    let next = DownloadStatus {
        listen_port: listen.map(|addr| addr.port()).unwrap_or(0),
        ipv6: listen.is_some_and(|addr| addr.is_ipv6()),
        dht_nodes: dht.as_ref().map(|s| s.routing_table_size).unwrap_or(0),
        dht_nodes_v6: dht.as_ref().map(|s| s.routing_table_size_v6).unwrap_or(0),
        peers_live: peers.live,
        peers_seen: peers.seen,
        peers_connecting: peers.connecting,
        peers_tcp: peers.tcp,
        peers_utp: peers.utp,
    };
    state.set_status(next);
}

/// Orphan cleanup: delete the meta file when metadata exists but the corresponding
/// task is no longer persisted. The reverse case (missing meta, session exists) is not
/// handled, because accidentally deleting resume data is unrecoverable; the UI falls
/// back to the torrent name.
pub(crate) fn cleanup_orphan_meta(session_dir: &Path, meta_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(meta_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            // The corresponding task exists in the session directory as <hash>.torrent
            let torrent = session_dir.join(format!("{stem}.torrent"));
            if !torrent.exists() && std::fs::remove_file(&path).is_ok() {
                eprintln!("已清理无效的任务记录");
            }
        }
    }
}

/// Peer counts accumulated across tasks (session-level aggregate).
#[derive(Default)]
struct PeerTotals {
    live: u32,
    seen: u32,
    connecting: u32,
    tcp: u32,
    utp: u32,
}

/// Whether IPv6 is available (determines whether dual-stack listening is possible).
fn ipv6_available() -> bool {
    std::net::TcpListener::bind((std::net::Ipv6Addr::UNSPECIFIED, 0)).is_ok()
}

/// Optional SOCKS5 proxy: BitTorrent traffic egresses through the proxy (an advanced
/// feature, in the form `socks5://[user:pass@]host:port`), configured via the
/// `MIKAN_SOCKS_PROXY` environment variable.
fn socks_proxy() -> Option<String> {
    std::env::var("MIKAN_SOCKS_PROXY")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Inbound listening: prefer dual-stack (IPv4 + IPv6), falling back to IPv4-only when
/// IPv6 is unavailable.
///
/// Dual-stack lets users with public IPv6 behind NAT / CGNAT accept inbound connections
/// (IPv6 has no NAT and is the main inbound path for mainland-China home broadband to
/// bypass CGNAT).
fn listener_options(port: u16) -> ListenerOptions {
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
    let (listen_addr, ipv4_only) = if ipv6_available() {
        (SocketAddr::from((Ipv6Addr::UNSPECIFIED, port)), false)
    } else {
        (SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)), true)
    };
    ListenerOptions {
        mode: ListenerMode::TcpAndUtp,
        listen_addr,
        enable_upnp_port_forwarding: true,
        ipv4_only,
        ..Default::default()
    }
}
