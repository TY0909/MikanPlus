//! 端到端网络冒烟:真实请求蜜柑 → 解析列表 → 下载一张封面 → 验证缓存。
//! 需要网络;失败时打印具体错误用于诊断。

#[test]
fn end_to_end_smoke() {
    // 1. 抓列表
    let html = match source::network::fetch_html(source::network::BASE_URL) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("LIST_FETCH_FAIL: {e}");
            return;
        }
    };
    let groups = source::parser::parse_bangumi_list(&html);
    let total: usize = groups.iter().map(|g| g.items.len()).sum();
    println!("列表: {} 分组 / {total} 番剧", groups.len());
    assert!(total > 0);

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
}
