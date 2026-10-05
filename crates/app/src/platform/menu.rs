//! Application menu bar.
//!
//! Keeps only the macOS standard menu structure: App / File / Edit / View / Window.
//! Menu item keyboard shortcuts are resolved automatically by the keymap (see the bindings
//! in `main.rs`).

use gpui_kit::{Menu, MenuItem, OsAction, SystemMenuType};

use ui::actions::{
    AboutMikan, CloseWindow, GoSettings, HideApp, HideOthers, MinimizeWindow, QuitApp, ShowAllApps,
    ToggleFullscreen, ZoomWindow,
};

/// Build the complete application menu bar.
///
/// `fullscreen` indicates whether the window is currently fullscreen, used to switch the label
/// in the "View" menu (Enter Full Screen / Exit Full Screen), following the macOS convention of
/// dynamic menu titles.
pub fn build_menus(fullscreen: bool) -> Vec<Menu> {
    vec![
        // ---- App menu ----
        Menu {
            name: "MikanPlus".into(),
            disabled: false,
            items: vec![
                MenuItem::action("关于 MikanPlus", AboutMikan),
                MenuItem::separator(),
                MenuItem::action("设置…", GoSettings),
                MenuItem::separator(),
                MenuItem::os_submenu("服务", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("隐藏 MikanPlus", HideApp),
                MenuItem::action("隐藏其他", HideOthers),
                MenuItem::action("全部显示", ShowAllApps),
                MenuItem::separator(),
                MenuItem::action("退出 MikanPlus", QuitApp),
            ],
        },
        // ---- File menu ----
        Menu {
            name: "文件".into(),
            disabled: false,
            items: vec![MenuItem::action("关闭窗口", CloseWindow)],
        },
        // ---- Edit menu ----
        // Dispatch the editing actions natively supported by gpui-component input fields
        // (Cut/Copy/Paste/Select All, acting on the currently focused input, with built-in
        // system clipboard and shortcut handling)
        Menu {
            name: "编辑".into(),
            disabled: false,
            items: vec![
                MenuItem::os_action("剪切", gpui_kit::component::input::Cut, OsAction::Cut),
                MenuItem::os_action("拷贝", gpui_kit::component::input::Copy, OsAction::Copy),
                MenuItem::os_action("粘贴", gpui_kit::component::input::Paste, OsAction::Paste),
                MenuItem::separator(),
                MenuItem::os_action(
                    "全选",
                    gpui_kit::component::input::SelectAll,
                    OsAction::SelectAll,
                ),
            ],
        },
        // ---- View menu ----
        Menu {
            name: "视图".into(),
            disabled: false,
            items: vec![MenuItem::action(
                if fullscreen {
                    "退出全屏"
                } else {
                    "进入全屏"
                },
                ToggleFullscreen,
            )],
        },
        // ---- Window menu ----
        Menu {
            name: "窗口".into(),
            disabled: false,
            items: vec![
                MenuItem::action("最小化", MinimizeWindow),
                MenuItem::action("缩放", ZoomWindow),
            ],
        },
    ]
}
