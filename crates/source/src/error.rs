//! 数据源错误(网络抓取 / 解码 / 缓存写入)。
//!
//! 面向用户的展示统一通过 [`SourceError::user_message`] 与
//! [`SourceError::user_hint`] 获取,不暴露底层错误细节。

use thiserror::Error;

/// 数据源错误。区分类型,以便 UI 给出针对性的提示。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SourceError {
    /// 请求过于频繁(处于退避期内)
    #[error("请求过于频繁")]
    Throttled,
    /// 无法建立网络连接(连接被拒、DNS 失败等)
    #[error("无法连接到服务器")]
    Network,
    /// 已建立连接但响应体读取中断(下载超时、连接被重置)
    #[error("加载中断")]
    Interrupted,
    /// 服务器返回错误状态码(4xx / 5xx)
    #[error("服务器返回错误状态码 {0}")]
    Server(u16),
    /// 响应内容解码失败(非预期编码)
    #[error("响应内容解码失败")]
    Decode,
    /// 图片超过大小上限
    #[error("图片大小超过限制")]
    ImageTooLarge,
    /// 写入本地缓存失败
    #[error("写入本地缓存失败")]
    Cache,
}

impl SourceError {
    /// 面向用户的简要说明(不含技术细节)
    pub fn user_message(&self) -> &'static str {
        match self {
            SourceError::Throttled => "请求过于频繁",
            SourceError::Network => "无法连接到服务器",
            SourceError::Interrupted => "加载中断",
            SourceError::Server(_) => "源站返回了错误",
            SourceError::Decode => "无法解析返回的内容",
            SourceError::ImageTooLarge => "图片过大",
            SourceError::Cache => "本地缓存写入失败",
        }
    }

    /// 用户需要检查 / 采取的下一步
    pub fn user_hint(&self) -> &'static str {
        match self {
            SourceError::Throttled => "请稍后重试",
            SourceError::Network => "请检查网络连接或代理设置",
            SourceError::Interrupted => "源站响应缓慢或网络连接不稳定，请稍后重试",
            SourceError::Server(_) => "蜜柑计划可能暂时异常，请稍后重试",
            SourceError::Decode => "源站返回了无法识别的内容，请稍后重试",
            SourceError::ImageTooLarge => "",
            SourceError::Cache => "请检查磁盘空间与写入权限",
        }
    }
}
