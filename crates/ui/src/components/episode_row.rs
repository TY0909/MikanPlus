//! Episode row: shared text measurement, title cell, and right-hand action button.
//!
//! Shared by the bangumi detail page, subgroup detail page, search result page, and subscription page.
//!
//! Composition: text width estimation and truncation in [`text`], title cell and hover tooltip in [`title`],
//! right-hand action button (three download states) in [`action`].

mod action;
mod text;
mod title;

pub use action::action_button;
pub use text::{exceeds_lines, truncate_title};
pub use title::{title_cell, title_tooltip};

use std::rc::Rc;

use gpui_kit::{App, Window};

/// Callback that opens the download collection page.
pub type OpenCollectionCallback = Rc<dyn Fn(String, &mut Window, &mut App)>;
