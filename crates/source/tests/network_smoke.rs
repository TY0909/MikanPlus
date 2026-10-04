//! 端到端网络冒烟:真实请求蜜柑 API/RSS → 列表 → 详情 → 字幕组剧集 → 封面缓存。
//! 需要网络;失败时打印具体错误用于诊断(网络受限时不判失败)。

#[test]
fn end_to_end_smoke() {
    // 1. 首页列表(JSON API)
    let groups = match source::api::fetch_home() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("LIST_FETCH_FAIL: {e}");
            return;
        }
    };
    let total: usize = groups.iter().map(|g| g.items.len()).sum();
    println!("首页: {} 分组 / {total} 番剧", groups.len());
    assert!(total > 0, "首页应返回番剧");

    // 2. 下载第一张封面并验证缓存
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
    match source::network::fetch_bytes(&url) {
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

    // 3. 详情(JSON API) → 字幕组摘要(不含剧集)
    // 新番可能暂无字幕组,顺序往后找第一个有字幕组的番剧
    let mut found: Option<(u32, domain::BangumiItem)> = None;
    for candidate in groups.iter().flat_map(|g| g.items.iter()) {
        let Some(bid) = candidate.bangumi_id else {
            continue;
        };
        match source::api::fetch_bangumi(bid) {
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

    // 4. 懒加载第一个字幕组的剧集(RSS)
    let Some(group) = detail
        .subtitle_groups
        .iter()
        .find(|g| g.subgroup_id.is_some())
    else {
        eprintln!("NO_SUBGROUP_ID");
        return;
    };
    let sid = group.subgroup_id.unwrap();
    match source::rss::fetch_subgroup_episodes(bid, sid) {
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
