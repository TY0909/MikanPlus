//! 蜜柑计划 HTML 解析(仅搜索页)。
//!
//! 列表与详情已改用公开 JSON API(见 [`crate::api`] 与 [`crate::rss`]),
//! 搜索没有对应 API,继续走服务端渲染的 HTML。
//!
//! 已验证的搜索页结构(mikanani.me, 2026-08):
//! - 番剧卡片:`li a[href='/Home/Bangumi/<id>']` 内 `span[data-src]` + `div.an-text`
//! - 剧集表:`tr.js-search-results-row`
//!   - `input.js-episode-select[data-magnet]` 磁力
//!   - `a.magnet-link-wrap` 标题
//!   - 第 3/4 个 td:大小 / 更新时间

use scraper::{ElementRef, Html, Selector};

use domain::{BangumiItem, SearchEpisode, SearchResults};

/// 搜索页:磁力输入(与详情页剧集行同签名)
const SEL_EPISODE_MAGNET: &str = "input.js-episode-select";
/// 搜索页:剧集标题链接
const SEL_EPISODE_TITLE: &str = "a.magnet-link-wrap";

/// 解析搜索结果页 → 番剧卡片 + 剧集列表。
///
/// 搜索页封面为 400×400 方形,统一替换为 400×560 以匹配应用的竖版比例。
pub fn parse_search_results(html: &str) -> SearchResults {
    let doc = Html::parse_document(html);
    let card_sel = Selector::parse("li a[href^='/Home/Bangumi/']").unwrap();
    let img_sel = Selector::parse("span[data-src]").unwrap();
    let name_sel = Selector::parse("div.an-text").unwrap();
    let row_sel = Selector::parse("tr.js-search-results-row").unwrap();
    let magnet_sel = Selector::parse(SEL_EPISODE_MAGNET).unwrap();
    let title_sel = Selector::parse(SEL_EPISODE_TITLE).unwrap();
    let td_sel = Selector::parse("td").unwrap();

    // 番剧卡片
    let mut items = Vec::new();
    for card in doc.select(&card_sel) {
        let href = card.value().attr("href").unwrap_or_default();
        let Some(bid_str) = href.strip_prefix("/Home/Bangumi/") else {
            continue;
        };
        let Ok(bid) = bid_str.parse::<u32>() else {
            continue;
        };
        // 封面:400×400 方形 → 400×560 竖版(与列表/详情一致)
        let cover_url = card
            .select(&img_sel)
            .next()
            .and_then(|e| e.value().attr("data-src"))
            .map(|u| u.replace("width=400&height=400", "width=400&height=560"))
            .map(|u| crate::network::site_url(&u));
        let name = card
            .select(&name_sel)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default()
            .trim()
            .to_string();
        if name.is_empty() {
            continue;
        }
        items.push(BangumiItem {
            name,
            bangumi_id: Some(bid),
            cover_url,
            detail_url: Some(format!("{}/Home/Bangumi/{bid}", crate::network::base_url())),
            ..Default::default()
        });
    }

    // 剧集结果行
    let mut episodes = Vec::new();
    for row in doc.select(&row_sel) {
        let magnet = row
            .select(&magnet_sel)
            .next()
            .and_then(|e| e.value().attr("data-magnet"))
            .unwrap_or_default()
            .to_string();
        let title = row
            .select(&title_sel)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default()
            .trim()
            .to_string();
        if title.is_empty() {
            continue;
        }
        let tds: Vec<ElementRef> = row.select(&td_sel).collect();
        let size = tds
            .get(2)
            .map(|td| td.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let date = tds
            .get(3)
            .map(|td| td.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        episodes.push(SearchEpisode {
            title,
            magnet,
            size,
            date,
        });
    }

    SearchResults { items, episodes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_search_page() {
        let html = r#"
        <ul>
            <li>
                <a href="/Home/Bangumi/4014" target="_blank">
                    <span data-src="/images/Bangumi/202607/79691e78.jpg?width=400&height=400&format=webp" class="b-lazy"></span>
                    <div class="an-info">
                        <div class="an-info-group">
                            <div class="an-text" title="碧蓝之海 第三季">碧蓝之海 第三季</div>
                        </div>
                    </div>
                </a>
            </li>
        </ul>
        <div class="episode-table">
            <table class="table table-striped">
                <tbody>
                    <tr class="js-search-results-row">
                        <td><input type="checkbox" class="js-episode-select" data-magnet="magnet:?xt=urn:btih:aaa111" /></td>
                        <td><a href="/Home/Episode/aaa111" class="magnet-link-wrap">[ANi] GRAND BLUE 碧蓝之海 3 - 06</a></td>
                        <td>342.8 MB</td>
                        <td>2026/08/20 12:00</td>
                    </tr>
                </tbody>
            </table>
        </div>"#;
        let results = parse_search_results(html);
        assert_eq!(results.items.len(), 1);
        let item = &results.items[0];
        assert_eq!(item.name, "碧蓝之海 第三季");
        assert_eq!(item.bangumi_id, Some(4014));
        // 方形封面应替换为 400×560 竖版
        assert!(
            item.cover_url
                .as_deref()
                .unwrap()
                .contains("width=400&height=560")
        );
        assert_eq!(results.episodes.len(), 1);
        let ep = &results.episodes[0];
        assert_eq!(ep.title, "[ANi] GRAND BLUE 碧蓝之海 3 - 06");
        assert!(ep.magnet.starts_with("magnet:"));
        assert_eq!(ep.size, "342.8 MB");
        assert_eq!(ep.date, "2026/08/20 12:00");
    }
}
