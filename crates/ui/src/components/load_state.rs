//! Lazy-loading state for subtitle-group episodes (shared by the detail page and
//! the subscription detail page).
//!
//! The detail data comes from a JSON API and does not include episodes; episodes
//! are fetched on demand when the user expands a subtitle group.

use std::rc::Rc;

use gpui_kit::{App, Window};

use domain::{BangumiId, SubgroupId};

/// Episode loading state for a single subtitle group.
#[derive(Clone)]
pub enum GroupEpisodesState {
    /// Request sent, awaiting response.
    Loading,
    /// Loaded.
    Ready(Vec<domain::Episode>),
    /// Loading failed (short user-facing description).
    Failed(String),
}

/// Expand/collapse a subtitle group of a bangumi.
pub type ToggleGroupCallback = Rc<dyn Fn(BangumiId, SubgroupId, &mut Window, &mut App)>;
