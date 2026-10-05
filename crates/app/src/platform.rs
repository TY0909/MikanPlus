//! Platform integration: assets, menus, keyboard shortcuts, and notifications.

pub(crate) mod assets;
pub(crate) mod keys;
pub(crate) mod notifications;

#[cfg(target_os = "macos")]
pub(crate) mod menu;
