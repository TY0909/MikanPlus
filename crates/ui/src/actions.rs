//! Global action definitions.
//!
//! Keeps only the standard macOS application menu actions (About/Settings/Hide/Quit/Close/
//! Minimize/Zoom/Fullscreen, plus the Edit menu's system actions). The namespace is `mikan`,
//! handled by the global listeners registered by `MikanPlus`.

use gpui_kit::actions;

actions!(
    mikan,
    [
        // ---- Navigation / overlays ----
        /// ⌘, Settings
        GoSettings,
        /// Esc closes the filter modal
        CloseFilterModal,
        // ---- Window / application ----
        /// About
        AboutMikan,
        /// ⌘H hide the app
        HideApp,
        /// ⌥⌘H hide others
        HideOthers,
        /// Show all (unhide other apps)
        ShowAllApps,
        /// ⌘Q quit
        QuitApp,
        /// ⌘W close window
        CloseWindow,
        /// ⌃⌘F toggle fullscreen
        ToggleFullscreen,
        /// ⌘M minimize
        MinimizeWindow,
        /// Zoom window (green traffic light / menu)
        ZoomWindow,
    ]
);
