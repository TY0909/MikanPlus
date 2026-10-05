//! Registration of keyboard shortcuts and application actions. Shortcuts target macOS only;
//! Esc to close overlays is kept cross-platform.

use domain::navigation::Page;

#[cfg(target_os = "macos")]
use crate::platform::menu::build_menus;
use crate::shell::state::MikanPlus;
use gpui_kit::{App, Context, Window};
use ui::actions::*;

/// Register application-level keyboard shortcuts.
///
/// Shortcuts target macOS only: Windows/Linux provide no application-level shortcuts (on Linux
/// they're left to the window manager / display manager, and Windows users rarely use them).
/// "Esc closes overlays" is a cross-platform interaction and isn't disabled along with the
/// platform shortcuts.
pub(crate) fn register_keybindings(cx: &mut App) {
    use gpui_kit::KeyBinding;

    // Cross-platform: Esc closes the filter window / unsubscribe modals
    cx.bind_keys([KeyBinding::new("escape", CloseFilterModal, None)]);

    // macOS: standard menu shortcuts (App menu / File / View / Window)
    #[cfg(target_os = "macos")]
    {
        cx.bind_keys([
            KeyBinding::new("cmd-,", GoSettings, None),
            KeyBinding::new("cmd-h", HideApp, None),
            KeyBinding::new("cmd-alt-h", HideOthers, None),
            KeyBinding::new("cmd-q", QuitApp, None),
            KeyBinding::new("cmd-w", CloseWindow, None),
            KeyBinding::new("cmd-m", MinimizeWindow, None),
            KeyBinding::new("cmd-ctrl-f", ToggleFullscreen, None),
        ]);
    }
}

/// Register global listeners for menu actions.
///
/// On macOS, a menu item's enabled state is determined by `App::is_action_available`: it's
/// available only when the action appears on the current focus dispatch path or a global listener
/// exists. The app has no default focused element, so menu actions must be registered here,
/// otherwise every menu item (and its shortcut) is greyed out.
///
/// Menu actions are dispatched inside a window `update`, where the same window can't be `update`d
/// again (the window is already taken out and would fail silently), so operations touching a
/// window/view are deferred with `cx.defer` until the current update finishes.
///
/// Menu actions exist only on macOS (gpui renders a native menu bar only on macOS; Windows/Linux
/// have neither a menu nor shortcuts), so this part is registered only on macOS, avoiding a
/// dependency on unimplemented platform APIs.
pub(crate) fn register_app_actions(cx: &mut App) {
    // Cross-platform: Esc closes overlays (a general interaction, independent of the platform menu)
    cx.on_action(|_: &CloseFilterModal, cx: &mut App| {
        cx.defer(|cx| {
            let _ = with_mikan(cx, |this, _window, cx| {
                this.close_filter(cx);
                this.close_warning(cx);
                this.close_confirmation(cx);
            });
        });
    });

    // macOS: native menu bar actions
    #[cfg(target_os = "macos")]
    {
        cx.on_action(|_: &QuitApp, cx: &mut App| cx.quit());
        cx.on_action(|_: &HideApp, cx: &mut App| cx.hide());
        cx.on_action(|_: &HideOthers, cx: &mut App| cx.hide_other_apps());
        cx.on_action(|_: &ShowAllApps, cx: &mut App| cx.unhide_other_apps());

        cx.on_action(|_: &CloseWindow, cx: &mut App| {
            cx.defer(|cx| with_active_window(cx, |window| window.remove_window()));
        });
        cx.on_action(|_: &MinimizeWindow, cx: &mut App| {
            cx.defer(|cx| with_active_window(cx, |window| window.minimize_window()));
        });
        cx.on_action(|_: &ZoomWindow, cx: &mut App| {
            cx.defer(|cx| with_active_window(cx, |window| window.zoom_window()));
        });
        cx.on_action(|_: &ToggleFullscreen, cx: &mut App| {
            cx.defer(|cx| with_active_window(cx, |window| window.toggle_fullscreen()));
        });

        cx.on_action(|_: &GoSettings, cx: &mut App| {
            cx.defer(|cx| {
                let _ = with_mikan(cx, |this, _window, cx| this.navigate_to(Page::Settings, cx));
            });
        });
        cx.on_action(|_: &AboutMikan, cx: &mut App| {
            // Open the settings page (which includes the about information)
            cx.defer(|cx| {
                let _ = with_mikan(cx, |this, _window, cx| this.navigate_to(Page::Settings, cx));
            });
        });
    }
}

/// Run an operation on the active window (ignored when there is none). Only used by macOS menu actions.
#[cfg(target_os = "macos")]
fn with_active_window(cx: &mut App, f: impl FnOnce(&mut Window)) {
    if let Some(handle) = cx.active_window() {
        let _ = handle.update(cx, |_, window, _| f(window));
    }
}

/// Run an operation on the `MikanPlus` view of the active window (ignored when there's no window or the view type doesn't match).
fn with_mikan<R>(
    cx: &mut App,
    f: impl FnOnce(&mut MikanPlus, &mut Window, &mut Context<MikanPlus>) -> R,
) -> Option<R> {
    let handle = cx.active_window()?;
    handle
        .update(cx, |root_view, window, cx| {
            let root = root_view.downcast::<gpui_kit::component::Root>().ok()?;
            let view = root.read(cx).view().clone().downcast::<MikanPlus>().ok()?;
            Some(view.update(cx, |this, cx| f(this, window, cx)))
        })
        .ok()
        .flatten()
}

/// Fullscreen state reflected in the macOS menu bar (owned by the view that owns the window).
#[cfg(target_os = "macos")]
impl MikanPlus {
    /// Rebuild the menu bar when the window's fullscreen state changes, switching the "View"
    /// item between "Enter Full Screen" / "Exit Full Screen".
    ///
    /// gpui has no fullscreen-state-change callback, so the polling loop calls this every 500ms;
    /// the menu is rebuilt only when the state actually changed. The last reflected state lives on
    /// the view (see [`MikanPlus::fullscreen`]), not in a global.
    pub(crate) fn sync_fullscreen_menu(&mut self, window: &mut Window, cx: &mut App) {
        let fullscreen = window.is_fullscreen();
        if self.fullscreen != fullscreen {
            self.fullscreen = fullscreen;
            cx.set_menus(build_menus(fullscreen));
        }
    }
}
