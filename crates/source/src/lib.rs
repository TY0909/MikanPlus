//! Mikan HTTP access and HTML parsing adapters.

pub mod api;
pub mod error;
pub mod parser;
pub mod rss;

mod network;

pub use error::SourceError;
pub use network::{ImgStatus, Network};
