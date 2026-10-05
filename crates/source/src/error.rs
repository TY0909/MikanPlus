//! Data-source errors (network fetching / decoding / cache writes).
//!
//! User-facing text is always obtained through [`SourceError::user_message`] and
//! [`SourceError::user_hint`]; underlying error details are never exposed.

use thiserror::Error;

/// A data-source error. Variants are split so the UI can show a targeted message.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SourceError {
    /// Requested too frequently (within the backoff window)
    #[error("请求过于频繁")]
    Throttled,
    /// Could not establish a network connection (connection refused, DNS failure, etc.)
    #[error("无法连接到服务器")]
    Network,
    /// Connection established, but reading the response body was interrupted
    /// (download timeout, connection reset)
    #[error("加载中断")]
    Interrupted,
    /// Server returned an error status code (4xx / 5xx)
    #[error("服务器返回错误状态码 {0}")]
    Server(u16),
    /// Response body failed to decode (unexpected encoding)
    #[error("响应内容解码失败")]
    Decode,
    /// Image exceeds the size limit
    #[error("图片大小超过限制")]
    ImageTooLarge,
    /// Failed to write to the local cache
    #[error("写入本地缓存失败")]
    Cache,
}

impl SourceError {
    /// Short user-facing description (no technical details)
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

    /// What the user should check / do next
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
