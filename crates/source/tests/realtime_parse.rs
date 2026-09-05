//! 真实页面解析验证:读取抓取的蜜柑页面,验证解析器输出。
//!
//! 需要先抓取页面(文件存在才运行):
//! ```text
//! curl -s -A "Mozilla/5.0" https://mikanani.me/ -o /tmp/mikan_home.html
//! curl -s -A "Mozilla/5.0" https://mikanani.me/Home/Bangumi/3883 -o /tmp/mikan_detail.html
//! ```

use source::parser::{parse_bangumi_detail, parse_bangumi_list};

#[test]
fn real_home_page_parses() {
    let html = match std::fs::read_to_string("/tmp/mikan_home.html") {
        Ok(h) => h,
        Err(_) => {
            eprintln!("跳过:缺少 /tmp/mikan_home.html");
            return;
        }
    };
    let groups = parse_bangumi_list(&html);
    assert!(!groups.is_empty(), "应解析出分组");
    let total: usize = groups.iter().map(|g| g.items.len()).sum();
    assert!(total > 50, "应解析出大量番剧,实际 {total}");
    // 抽查第一个条目
    let item = &groups[0].items[0];
    assert!(!item.name.is_empty());
    assert!(item.bangumi_id.is_some());
    assert!(
        item.cover_url
            .as_deref()
            .unwrap_or_default()
            .starts_with("http"),
        "封面应为完整 URL"
    );
    println!(
        "列表: {} 个分组, {} 个番剧; 样例: {} (id={})",
        groups.len(),
        total,
        item.name,
        item.bangumi_id.unwrap()
    );
}

#[test]
fn real_detail_page_parses() {
    let html = match std::fs::read_to_string("/tmp/mikan_detail.html") {
        Ok(h) => h,
        Err(_) => {
            eprintln!("跳过:缺少 /tmp/mikan_detail.html");
            return;
        }
    };
    let (meta, groups) = parse_bangumi_detail(&html);
    assert!(!groups.is_empty(), "应解析出字幕组");
    let total_eps: usize = groups.iter().map(|g| g.episodes.len()).sum();
    assert!(total_eps > 0, "应解析出剧集,实际 {total_eps}");
    // 抽查第一集
    let ep = &groups[0].episodes[0];
    assert!(!ep.title.is_empty());
    assert!(
        ep.magnet_link
            .as_deref()
            .unwrap_or_default()
            .starts_with("magnet:"),
        "剧集应有磁力链接"
    );
    println!(
        "详情: {} 个字幕组, {} 集; 字幕组: {}; 简介: {}",
        groups.len(),
        total_eps,
        groups[0].name,
        meta.summary
            .as_deref()
            .unwrap_or("无")
            .chars()
            .take(30)
            .collect::<String>()
    );
}
