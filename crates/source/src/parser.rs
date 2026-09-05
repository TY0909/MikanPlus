//! 蜜柑计划 HTML 解析(结构集中在顶部常量,站点改版时只改这里)。
//!
//! 已验证的站点结构(mikanani.me, 2026-08):
//! - 列表页:
//!   `div.sk-bangumi[data-dayofweek]`(0-6=周日..周六, 7=剧场版)分组
//!   → `ul.an-ul > li` 卡片
//!     - `span.js-expand_bangumi[data-src]` 封面 URL
//!     - `[data-bangumiid]` 番剧 ID
//!     - `a.an-text[title]` 名称
//!     - `div.date-text` 更新日期
//! - 详情页 `/Home/Bangumi/<id>`:
//!   - `p.bangumi-info` 元数据(放送日期/开始/官网)
//!   - 简介:含 `intro` 的 class
//!   - 字幕组内容区 `div[id=<sid>]`:
//!     - `a[href="/Home/PublishGroup/..."]` 字幕组名
//!     - `a.mikan-rss[href]` RSS 订阅链接
//!     - 内含 `table.episode-table > tbody > tr` 剧集行
//!       - `input.js-episode-select[data-magnet]` 磁力
//!       - `a.magnet-link-wrap` 标题
//!       - 第 3/4 个 td:大小 / 更新时间

use scraper::{ElementRef, Html, Selector};

use domain::{
    BangumiGroup, BangumiItem, BangumiMeta, Episode, SearchEpisode, SearchResults, SubtitleGroup,
};

// ── 选择器(集中定义) ──────────────────────────────

/// 列表页:按天分组容器
const SEL_GROUP: &str = "div.sk-bangumi";
/// 列表页:分组标题行
const SEL_GROUP_TITLE: &str = "div.row";
/// 列表页:卡片
const SEL_CARD: &str = "ul.an-ul > li";
/// 列表页:封面元素(带 data-src 懒加载)
const SEL_CARD_IMG: &str = "span.js-expand_bangumi";
/// 列表页:名称链接
const SEL_CARD_NAME: &str = "a.an-text";
/// 列表页:更新日期
const SEL_CARD_DATE: &str = "div.date-text";

/// 详情页:元信息行
const SEL_META: &str = "p.bangumi-info";
/// 详情页:简介
const SEL_INTRO: &str = "[class*='intro']";
/// 详情页:字幕组名链接
const SEL_SUBGROUP_NAME: &str = "a[href^='/Home/PublishGroup/']";
/// 详情页:RSS 订阅链接
const SEL_SUBGROUP_RSS: &str = "a.mikan-rss";
/// 详情页:剧集表(位于订阅 popover 容器内,是服务端渲染的真实数据源)
const SEL_EPISODE_TABLE: &str = "table.table-striped";
/// 详情页:剧集行
const SEL_EPISODE_ROW: &str = "tbody tr";
/// 详情页:磁力输入
const SEL_EPISODE_MAGNET: &str = "input.js-episode-select";
/// 详情页:剧集标题链接
const SEL_EPISODE_TITLE: &str = "a.magnet-link-wrap";

/// 蜜柑的 dayofweek → 应用内的 day 字段(英文 key,见 home_view)
/// 蜜柑:0=周日 1=周一 … 6=周六 7=剧场版
fn mikan_day_key(mikan_day: &str) -> Option<&'static str> {
    match mikan_day {
        "0" => Some("sunday"),
        "1" => Some("monday"),
        "2" => Some("tuesday"),
        "3" => Some("wednesday"),
        "4" => Some("thursday"),
        "5" => Some("friday"),
        "6" => Some("saturday"),
        "7" => Some("movie"),
        _ => None,
    }
}

/// 解析列表页 → 按天分组的番剧列表
pub fn parse_bangumi_list(html: &str) -> Vec<BangumiGroup> {
    let doc = Html::parse_document(html);
    let group_sel = Selector::parse(SEL_GROUP).unwrap();
    let title_sel = Selector::parse(SEL_GROUP_TITLE).unwrap();
    let card_sel = Selector::parse(SEL_CARD).unwrap();
    let img_sel = Selector::parse(SEL_CARD_IMG).unwrap();
    let name_sel = Selector::parse(SEL_CARD_NAME).unwrap();
    let date_sel = Selector::parse(SEL_CARD_DATE).unwrap();

    let mut groups: Vec<BangumiGroup> = Vec::new();
    for group_el in doc.select(&group_sel) {
        let Some(day_attr) = group_el.value().attr("data-dayofweek") else {
            continue;
        };
        let day = day_attr.to_string();
        let Some(day_key) = mikan_day_key(&day) else {
            continue;
        };
        let day = day_key.to_string();

        // 标题行(星期日/剧场版 …)
        let title = group_el
            .select(&title_sel)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default()
            .trim()
            .to_string();

        let mut items = Vec::new();
        for card in group_el.select(&card_sel) {
            let Some(bid_str) = card
                .select(&img_sel)
                .next()
                .and_then(|e| e.value().attr("data-bangumiid"))
            else {
                continue;
            };
            let Some(bid) = bid_str.parse::<u32>().ok() else {
                continue;
            };
            let cover_url = card
                .select(&img_sel)
                .next()
                .and_then(|e| e.value().attr("data-src"))
                .map(crate::network::site_url);
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
            let date = card
                .select(&date_sel)
                .next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default()
                .trim()
                .to_string();

            items.push(BangumiItem {
                name,
                bangumi_id: Some(bid),
                cover_url,
                detail_url: Some(format!("{}/Home/Bangumi/{bid}", crate::network::BASE_URL)),
                meta: None,
                subtitle_groups: Vec::new(),
                #[allow(clippy::redundant_field_names)]
                update_date: if date.is_empty() { None } else { Some(date) },
            });
        }
        if !items.is_empty() {
            groups.push(BangumiGroup {
                day: day.clone(),
                title,
                items,
            });
        }
    }
    groups
}

/// 解析详情页 → 元数据 + 字幕组(含剧集)。
///
/// 注意:蜜柑页面的字幕组区块 `div[id]` 与剧集表 `table.table-striped`
/// 因 HTML 结构问题被解析器分置两处,两者按文档顺序一一对应,
/// 因此这里分开收集后按索引配对。
pub fn parse_bangumi_detail(html: &str) -> (BangumiMeta, Vec<SubtitleGroup>) {
    let doc = Html::parse_document(html);
    let meta_sel = Selector::parse(SEL_META).unwrap();
    let intro_sel = Selector::parse(SEL_INTRO).unwrap();
    let pair_sel = Selector::parse("div.subgroup-text[id], div.episode-table").unwrap();
    let name_sel = Selector::parse(SEL_SUBGROUP_NAME).unwrap();
    let rss_sel = Selector::parse(SEL_SUBGROUP_RSS).unwrap();
    let table_sel = Selector::parse(SEL_EPISODE_TABLE).unwrap();
    let row_sel = Selector::parse(SEL_EPISODE_ROW).unwrap();
    let magnet_sel = Selector::parse(SEL_EPISODE_MAGNET).unwrap();
    let title_sel = Selector::parse(SEL_EPISODE_TITLE).unwrap();
    let a_sel = Selector::parse("a").unwrap();
    let td_sel = Selector::parse("td").unwrap();

    // 元数据:p.bangumi-info 逐行解析
    let mut meta = BangumiMeta::default();
    for p in doc.select(&meta_sel) {
        let text = p.text().collect::<String>();
        let text = text.trim();
        if text.starts_with("放送日期") {
            meta.broadcast_day = Some(text.trim_start_matches("放送日期：").trim().to_string());
        } else if text.starts_with("放送开始") {
            meta.broadcast_start = Some(text.trim_start_matches("放送开始：").trim().to_string());
        } else if text.starts_with("官方网站") {
            meta.official_site = p
                .select(&a_sel)
                .next()
                .map(|a| a.value().attr("href").unwrap_or_default().to_string());
        } else if text.starts_with("Bangumi番组计划") {
            meta.bangumi_link = p
                .select(&a_sel)
                .next()
                .map(|a| a.value().attr("href").unwrap_or_default().to_string());
        }
    }

    // 简介(去掉开头的「概况介绍」标题)
    meta.summary = doc
        .select(&intro_sel)
        .next()
        .map(|e| e.text().collect::<String>())
        .map(|s| s.trim().to_string())
        .map(|s| {
            s.strip_prefix("概况介绍")
                .map(|rest| rest.trim().to_string())
                .unwrap_or(s)
        })
        .filter(|s| !s.is_empty());

    // 字幕组与剧集表的配对:按文档顺序共同遍历「subgroup-text 区块」与
    // 「episode-table 容器」,一个剧集表归属其之前最近的字幕组区块。
    // 与原先「全页收集后按索引 zip」相比:页面上出现无关表格/数字 id 的
    // div 时不会错位、不会产生幽灵分组(区块选择器要求 subgroup-text 签名)。
    let mut groups: Vec<SubtitleGroup> = Vec::new();
    for el in doc.select(&pair_sel) {
        let is_block = el.value().classes().any(|c| c == "subgroup-text");
        if is_block {
            let Some(id_attr) = el.value().attr("id") else {
                continue;
            };
            let Ok(sid) = id_attr.parse::<u32>() else {
                continue; // 非数字 id 的区块(异常结构)跳过
            };
            let name = el
                .select(&name_sel)
                .next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default()
                .trim()
                .to_string();
            let subscription_url = el
                .select(&rss_sel)
                .next()
                .and_then(|e| e.value().attr("href"))
                .map(crate::network::site_url);
            groups.push(SubtitleGroup {
                name,
                subgroup_id: Some(sid),
                subscription_url,
                episodes: Vec::new(),
            });
        } else {
            // episode-table 容器:解析其中的首个剧集表
            let mut episodes = Vec::new();
            if let Some(table) = el.select(&table_sel).next() {
                for row in table.select(&row_sel) {
                    let magnet = row
                        .select(&magnet_sel)
                        .next()
                        .and_then(|e| e.value().attr("data-magnet"))
                        .map(|s| s.to_string());
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
                    // 大小/时间:第 3、4 个 td
                    let tds: Vec<ElementRef> = row.select(&td_sel).collect();
                    let size = tds.get(2).map(|td| td.text().collect::<String>());
                    let date = tds.get(3).map(|td| td.text().collect::<String>());
                    episodes.push(Episode {
                        title,
                        magnet_link: magnet,
                        size: size.map(|s| s.trim().to_string()),
                        publish_date: date.map(|s| s.trim().to_string()),
                    });
                }
            }
            if let Some(last) = groups.last_mut() {
                last.episodes = episodes;
            }
        }
    }

    (meta, groups)
}

/// 解析搜索结果页 → 番剧卡片 + 剧集列表。
///
/// 搜索页结构(mikanani.me, 2026-08):
/// - 番剧卡片:`li > a[href="/Home/Bangumi/<id>"] > span.b-lazy[data-src]` + `div.an-text[title]`
/// - 剧集表:`tr.js-search-results-row`(同详情页剧集行选择器)
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
            detail_url: Some(format!("{}/Home/Bangumi/{bid}", crate::network::BASE_URL)),
            meta: None,
            subtitle_groups: Vec::new(),
            update_date: None,
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
    fn day_mapping() {
        // 蜜柑 0=周日 → 应用 sunday
        assert_eq!(mikan_day_key("0"), Some("sunday"));
        assert_eq!(mikan_day_key("1"), Some("monday"));
        assert_eq!(mikan_day_key("6"), Some("saturday"));
        assert_eq!(mikan_day_key("7"), Some("movie"));
        assert_eq!(mikan_day_key("x"), None);
    }

    #[test]
    fn parse_minimal_list() {
        let html = r#"
        <div class="sk-bangumi" data-dayofweek="1">
          <div class="row">星期一</div>
          <ul class="list-inline an-ul">
            <li>
              <span class="js-expand_bangumi b-lazy" data-src="/images/Bangumi/202602/abc.jpg?width=400&height=400&format=webp" data-bangumiid="3883" data-bangumiindex="1"></span>
              <div class="an-info"><div class="an-info-group">
                <div class="date-text">2026/08/09 更新</div>
                <a href="/Home/Bangumi/3883" target="_blank" class="an-text" title="测试番剧">测试番剧</a>
              </div></div>
            </li>
          </ul>
        </div>"#;
        let groups = parse_bangumi_list(html);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].day, "monday");
        assert_eq!(groups[0].title, "星期一");
        assert_eq!(groups[0].items.len(), 1);
        let item = &groups[0].items[0];
        assert_eq!(item.name, "测试番剧");
        assert_eq!(item.bangumi_id, Some(3883));
        assert!(
            item.cover_url
                .as_deref()
                .unwrap()
                .contains("/images/Bangumi/202602/abc.jpg")
        );
    }

    #[test]
    fn parse_minimal_detail() {
        let html = r#"
        <p class="bangumi-info">放送日期：星期日</p>
        <p class="bangumi-info">放送开始：2/1/2026</p>
        <p class="bangumi-info">官方网站：<a class="w-other-c" href="https://example.com/site">site</a></p>
        <p class="bangumi-info">Bangumi番组计划链接：<a class="w-other-c" href="https://bgm.tv/subject/123">bgm</a></p>
        <div class="bangumi-intro">这是一个测试简介</div>
        <div class="subgroup-text" id="370">
          <a href="/Home/PublishGroup/223">LoliHouse</a>
          <a href="/RSS/Bangumi?bangumiId=3883&subgroupid=370" class="mikan-rss"></a>
        </div>
        <div class="episode-table">
          <table class="table table-striped">
            <tbody>
              <tr>
                <td><input class="js-episode-select" data-magnet="magnet:?xt=urn:btih:abc123" /></td>
                <td><a class="magnet-link-wrap" href="/Home/Episode/x">[LoliHouse] 测试 - 01</a></td>
                <td>997.4MB</td>
                <td>2026/08/09 19:06</td>
              </tr>
            </tbody>
          </table>
        </div>"#;
        let (meta, groups) = parse_bangumi_detail(html);
        assert_eq!(meta.broadcast_day.as_deref(), Some("星期日"));
        assert_eq!(meta.broadcast_start.as_deref(), Some("2/1/2026"));
        assert_eq!(
            meta.official_site.as_deref(),
            Some("https://example.com/site")
        );
        assert_eq!(
            meta.bangumi_link.as_deref(),
            Some("https://bgm.tv/subject/123")
        );
        assert_eq!(meta.summary.as_deref(), Some("这是一个测试简介"));
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "LoliHouse");
        assert_eq!(groups[0].subgroup_id, Some(370));
        assert_eq!(groups[0].episodes.len(), 1);
        let ep = &groups[0].episodes[0];
        assert_eq!(ep.title, "[LoliHouse] 测试 - 01");
        assert!(ep.magnet_link.as_deref().unwrap().starts_with("magnet:"));
        assert_eq!(ep.size.as_deref(), Some("997.4MB"));
        assert_eq!(ep.publish_date.as_deref(), Some("2026/08/09 19:06"));
    }

    #[test]
    fn stray_tables_do_not_misalign_groups() {
        // 页面上出现无关的 table(广告/隐藏模板)与数字 id 的幽灵 div 时,
        // 配对不应错位、不应产生幽灵分组。
        let html = r#"
        <div id="999">无关的锚点 div</div>
        <table class="table-striped"><tbody><tr>
          <td><a class="magnet-link-wrap">无关表格的行</a></td>
        </tr></tbody></table>
        <div class="subgroup-text" id="370">
          <a href="/Home/PublishGroup/223">LoliHouse</a>
          <a href="/RSS/Bangumi?bangumiId=3883&subgroupid=370" class="mikan-rss"></a>
        </div>
        <div class="episode-table">
          <table class="table-striped"><tbody><tr>
            <td><input class="js-episode-select" data-magnet="magnet:?xt=urn:btih:aaa" /></td>
            <td><a class="magnet-link-wrap">[LoliHouse] 真正的 - 01</a></td>
          </tr></tbody></table>
        </div>
        <div class="subgroup-text" id="371">
          <a href="/Home/PublishGroup/999">另一个组</a>
        </div>
        <div class="episode-table">
          <table class="table-striped"><tbody><tr>
            <td><input class="js-episode-select" data-magnet="magnet:?xt=urn:btih:bbb" /></td>
            <td><a class="magnet-link-wrap">[另一个组] 第 1 话</a></td>
          </tr></tbody></table>
        </div>"#;
        let (_, groups) = parse_bangumi_detail(html);
        // 幽灵 div(数字 id 但无 subgroup-text 签名)不产生分组
        assert_eq!(groups.len(), 2, "只应有 2 个真实字幕组");
        assert_eq!(groups[0].name, "LoliHouse");
        assert_eq!(groups[0].episodes.len(), 1);
        assert_eq!(groups[0].episodes[0].title, "[LoliHouse] 真正的 - 01");
        assert_eq!(groups[1].name, "另一个组");
        assert_eq!(groups[1].episodes[0].title, "[另一个组] 第 1 话");
    }

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
