//! Mikan RSS data source (structured XML).
//!
//! Used for "all episodes of one subgroup" (detail-page lazy loading):
//! `GET /RSS/Bangumi?bangumiId=&subgroupid=`.
//! - Each `<item>`'s `<link>` last segment is the info hash; `<enclosure url>` is
//!   the direct `.torrent` link
//! - The magnet carries only the info hash; public trackers are merged uniformly
//!   by the download layer when the task is added
//!
//! Note: `?bangumiId=` (without subgroupid) returns the whole bangumi's episodes
//! but **omits the subgroup attribution**, so it is not used for the grouped
//! detail display.

use domain::{BangumiId, Episode, SubgroupId};
use roxmltree::{Document, Node};

use crate::Network;
use crate::SourceError;

/// Parse RSS text → episode list (preserving item order).
pub fn parse_rss_episodes(xml: &str) -> Vec<Episode> {
    let Ok(doc) = Document::parse(xml) else {
        return Vec::new();
    };
    doc.descendants()
        .filter(|node| node.has_tag_name("item"))
        .filter_map(episode_from_item)
        .collect()
}

/// Fetch one subgroup's episode list (called by detail-page lazy loading).
pub fn fetch_subgroup_episodes(
    network: &Network,
    bangumi_id: BangumiId,
    subgroup_id: SubgroupId,
) -> Result<Vec<Episode>, SourceError> {
    let xml = network.fetch_html(&subgroup_rss_url(network, bangumi_id, subgroup_id))?;
    Ok(parse_rss_episodes(&xml))
}

/// RSS URL for one subgroup (matches the `href` of `a.mikan-rss` on the detail page).
pub fn subgroup_rss_url(
    network: &Network,
    bangumi_id: BangumiId,
    subgroup_id: SubgroupId,
) -> String {
    format!(
        "{}/RSS/Bangumi?bangumiId={}&subgroupid={}",
        network.base_url(),
        bangumi_id.get(),
        subgroup_id.get()
    )
}

fn episode_from_item(item: Node) -> Option<Episode> {
    let title = first_text(item, "title")?;
    let hash = item_hash(item)?;
    let size = first_text(item, "description").and_then(|d| trailing_size(&d));
    let publish_date = first_text(item, "pubDate").and_then(|d| format_pub_date(&d));
    let torrent_url = item
        .descendants()
        .find(|node| node.has_tag_name("enclosure"))
        .and_then(|node| node.attribute("url"))
        .map(str::to_string);
    Some(Episode {
        title,
        magnet_link: Some(format!("magnet:?xt=urn:btih:{hash}")),
        size,
        publish_date,
        hash: Some(hash),
        torrent_url,
    })
}

/// Depth-first lookup of the first matching element's text (`<pubDate>` is nested
/// inside the namespaced `<torrent>` element).
fn first_text(node: Node, name: &str) -> Option<String> {
    node.descendants()
        .find(|child| child.is_element() && child.has_tag_name(name))
        .and_then(|child| child.text())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

/// info hash: prefer the last segment of `<link>` (episode page), falling back to
/// the `.torrent` filename from `<enclosure>`.
fn item_hash(item: Node) -> Option<String> {
    if let Some(hash) = first_text(item, "link")
        .and_then(|link| last_segment(&link).map(str::to_string))
        .and_then(|segment| is_info_hash(&segment))
    {
        return Some(hash);
    }
    item.descendants()
        .filter(|node| node.has_tag_name("enclosure"))
        .filter_map(|node| node.attribute("url"))
        .filter_map(last_segment)
        .filter_map(|name| name.strip_suffix(".torrent"))
        .find_map(is_info_hash)
}

fn last_segment(url: &str) -> Option<&str> {
    url.trim_end_matches('/').rsplit('/').next()
}

fn is_info_hash(candidate: &str) -> Option<String> {
    let candidate = candidate.trim().to_ascii_lowercase();
    (candidate.len() == 40 && candidate.bytes().all(|b| b.is_ascii_hexdigit())).then_some(candidate)
}

/// Extract the size from a trailing `[...]` in the description (Mikan's display
/// format, e.g. `674.7 MB`).
fn trailing_size(description: &str) -> Option<String> {
    let inner = description.trim_end().strip_suffix(']')?;
    let start = inner.rfind('[')?;
    let size = inner[start + 1..].trim();
    (!size.is_empty()).then(|| size.to_string())
}

/// ISO `2025-11-13T19:15:26.336282` → Mikan display format `2025/11/13 19:15`.
fn format_pub_date(iso: &str) -> Option<String> {
    let (date, time) = iso.split_once('T')?;
    let date = date.replace('-', "/");
    let hhmm: String = time.chars().take(5).collect();
    (date.len() == 10 && hhmm.len() == 5).then(|| format!("{date} {hhmm}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEED: &str = r#"<?xml version="1.0" encoding="utf-8"?><rss version="2.0"><channel><title>Mikan Project - X</title><link>http://mikanani.me/RSS/Bangumi?bangumiId=1</link><item><guid isPermaLink="false">[G] 测试 - 25</guid><link>https://mikanani.me/Home/Episode/5d9c3c435b929a2252bbe11681f09bd888a74085</link><title>[G] 测试 - 25</title><description>[G] 测试 - 25[674.7 MB]</description><torrent xmlns="https://mikanani.me/0.1/"><link>https://mikanani.me/Home/Episode/5d9c3c435b929a2252bbe11681f09bd888a74085</link><contentLength>707474240</contentLength><pubDate>2025-11-13T19:15:26.336282</pubDate></torrent><enclosure type="application/x-bittorrent" length="707474240" url="https://mikanani.me/Download/20251113/5d9c3c435b929a2252bbe11681f09bd888a74085.torrent" /></item><item><guid isPermaLink="false">[G] 测试 - 24</guid><link>https://mikanani.me/Home/Episode/A053431E73DE1081790BC12C4446CEF7A5F37A7E</link><title>[G] 测试 - 24</title><description>[G] 测试 - 24[1024.0 MB]</description></item></channel></rss>"#;

    #[test]
    fn parses_title_hash_size_date() {
        let episodes = parse_rss_episodes(FEED);
        assert_eq!(episodes.len(), 2);
        let first = &episodes[0];
        assert_eq!(first.title, "[G] 测试 - 25");
        assert_eq!(
            first.magnet_link.as_deref(),
            Some("magnet:?xt=urn:btih:5d9c3c435b929a2252bbe11681f09bd888a74085")
        );
        assert_eq!(
            first.hash.as_deref(),
            Some("5d9c3c435b929a2252bbe11681f09bd888a74085")
        );
        assert_eq!(first.size.as_deref(), Some("674.7 MB"));
        assert_eq!(first.publish_date.as_deref(), Some("2025/11/13 19:15"));
        assert_eq!(
            first.torrent_url.as_deref(),
            Some(
                "https://mikanani.me/Download/20251113/5d9c3c435b929a2252bbe11681f09bd888a74085.torrent"
            )
        );
    }

    #[test]
    fn falls_back_to_link_hash_and_lowercases_it() {
        let episodes = parse_rss_episodes(FEED);
        assert_eq!(
            episodes[1].magnet_link.as_deref(),
            Some("magnet:?xt=urn:btih:a053431e73de1081790bc12c4446cef7a5f37a7e")
        );
        assert_eq!(episodes[1].size.as_deref(), Some("1024.0 MB"));
        assert_eq!(episodes[1].publish_date, None);
    }

    #[test]
    fn invalid_xml_yields_no_episodes() {
        assert!(parse_rss_episodes("not xml <<<").is_empty());
    }

    #[test]
    fn subgroup_rss_url_matches_site() {
        assert!(
            subgroup_rss_url(
                &crate::Network::new(),
                BangumiId::from(3560),
                SubgroupId::from(370)
            )
            .ends_with("/RSS/Bangumi?bangumiId=3560&subgroupid=370")
        );
    }
}
