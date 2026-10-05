//! End-to-end network smoke test: real requests to the Mikan API/RSS → list → detail → subgroup episodes → cover cache.
//! Requires network access; on failure it prints the specific error for diagnostics (a restricted network does not fail the run).

#[test]
fn end_to_end_smoke() {
    let network = source::Network::new();
    // 1. Home list (JSON API).
    let groups = match source::api::fetch_home(&network) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("LIST_FETCH_FAIL: {e}");
            return;
        }
    };
    let total: usize = groups.iter().map(|g| g.items.len()).sum();
    println!("首页: {} 分组 / {total} 番剧", groups.len());
    assert!(total > 0, "首页应返回番剧");

    // 2. Download the first cover and verify caching.
    let Some(item) = groups
        .iter()
        .flat_map(|g| g.items.iter())
        .find(|i| i.cover_url.is_some())
    else {
        eprintln!("NO_COVER");
        return;
    };
    let url = item.cover_url.clone().unwrap();
    println!("封面: {url}");
    match network.fetch_bytes(&url) {
        Ok(bytes) => {
            println!("图片: {} bytes", bytes.len());
            assert!(bytes.len() > 1000, "图片数据量异常");
            let path = storage::cache::store_image(&url, &bytes).expect("写缓存");
            assert!(path.exists());
            assert!(storage::cache::cached_image(&url).is_some());
            println!("缓存: {path:?}");
        }
        Err(e) => eprintln!("IMAGE_FETCH_FAIL: {e}"),
    }

    // 3. Detail (JSON API) → subtitle-group summary (episodes not included).
    // A new show may have no subtitle groups yet, so scan forward for the first show that has one.
    let mut found: Option<(domain::BangumiId, domain::BangumiItem)> = None;
    for candidate in groups.iter().flat_map(|g| g.items.iter()) {
        let bid = candidate.bangumi_id;
        match source::api::fetch_bangumi(&network, bid) {
            Ok(detail) if !detail.subtitle_groups.is_empty() => {
                println!(
                    "详情: {} / {} 个字幕组",
                    detail.name,
                    detail.subtitle_groups.len()
                );
                found = Some((bid, detail));
                break;
            }
            Ok(_) => continue,
            Err(e) => {
                eprintln!("DETAIL_FETCH_FAIL: {e}");
                break;
            }
        }
    }
    let Some((bid, detail)) = found else {
        eprintln!("NO_SUBGROUP");
        return;
    };

    // 4. Lazily load the episodes of the first subtitle group (RSS).
    let Some((group, sid)) = detail
        .subtitle_groups
        .iter()
        .map(|g| (g, g.subgroup_id))
        .find(|(_, sid)| sid.is_known())
    else {
        eprintln!("NO_SUBGROUP_ID");
        return;
    };
    match source::rss::fetch_subgroup_episodes(&network, bid, sid) {
        Ok(episodes) => {
            println!("字幕组「{}」: {} 集", group.name, episodes.len());
            if episodes.is_empty() {
                eprintln!("EMPTY_SUBGROUP: 该字幕组暂无剧集");
                return;
            }
            assert!(
                episodes.iter().all(|e| e
                    .magnet_link
                    .as_deref()
                    .unwrap_or_default()
                    .starts_with("magnet:")),
                "剧集应带磁力链接"
            );
        }
        Err(e) => eprintln!("RSS_FETCH_FAIL: {e}"),
    }
}
