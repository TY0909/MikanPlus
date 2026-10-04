//! 字幕组剧集的懒加载状态(详情页与订阅详情页共享)。
//!
//! 详情数据来自 JSON API,不含剧集;剧集由用户展开字幕组时按需拉取。

use std::rc::Rc;

use gpui_kit::{App, Window};

/// 某个字幕组的剧集加载状态。
#[derive(Clone)]
pub enum GroupEpisodesState {
    /// 已发起请求,等待返回
    Loading,
    /// 已加载
    Ready(Vec<domain::Episode>),
    /// 加载失败(面向用户的简短描述)
    Failed(String),
}

/// 展开/折叠某个字幕组;参数为 `(番剧id, 字幕组id)`。
pub type ToggleGroupCallback = Rc<dyn Fn(u32, u32, &mut Window, &mut App)>;
