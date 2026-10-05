// Windows release builds use the GUI subsystem: double-clicking the app no
// longer opens a terminal window. Debug builds keep the console so developers
// can read logs.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod actions;
mod data;
mod load;
mod navigation;
mod platform;
mod prelude;
mod shell;

use gpui_kit::{App, Bounds, WindowBounds, WindowOptions, prelude::*};
use platform::assets::Assets;
use platform::keys::{register_app_actions, register_keybindings};
use shell::state::MikanPlus;
use std::rc::Rc;
use ui::theme::app_theme;

#[cfg(target_os = "macos")]
use crate::platform::menu::build_menus;

fn open_main_window(cx: &mut App) {
    // The application owns the persisted state; it is loaded here and handed to the view.
    let state = Rc::new(storage::State::load());
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Maximized(Bounds::default())),
            titlebar: Some(gpui_kit::TitlebarOptions {
                title: Some("MikanPlus".into()),
                ..Default::default()
            }),
            // The toolbar's left search area, center navigation, and right
            // action area all need stable minimum space. Below this size the
            // toolbar regions overlap even though the content area can scale
            // responsively.
            window_min_size: Some(gpui_kit::Size {
                width: gpui_kit::px(1200.),
                height: gpui_kit::px(720.),
            }),
            ..Default::default()
        },
        |window, cx| {
            // Apply the persisted theme mode before the first paint.
            let mode = match state.theme_mode().as_deref() {
                Some("dark") => gpui_kit::component::theme::ThemeMode::Dark,
                _ => gpui_kit::component::theme::ThemeMode::Light,
            };
            app_theme::apply_mode(mode, None, cx);
            let mikan = MikanPlus::new(window, cx, state);
            cx.new(|cx| gpui_kit::component::Root::new(mikan, window, cx))
        },
    )
    .ok();
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets::locate());

    // macOS convention: after ⌘W closes the last window the app stays in the
    // Dock; clicking the Dock icon rebuilds the window (downloads keep running
    // in the background).
    app.on_reopen(|cx: &mut App| {
        if cx.windows().is_empty() {
            open_main_window(cx);
        } else {
            cx.activate(true);
        }
    });

    app.run(|cx: &mut App| {
        gpui_kit::init(cx);

        app_theme::install(cx);

        register_keybindings(cx);

        register_app_actions(cx);

        // Application menu bar (only macOS has a native menu bar)
        #[cfg(target_os = "macos")]
        cx.set_menus(build_menus(false));

        open_main_window(cx);
    });
}
