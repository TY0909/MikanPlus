//! 应用菜单栏。
//!
//! 只保留 macOS 标准菜单结构:App / 文件 / 编辑 / 视图 / 窗口。
//! 菜单项的键盘快捷键由 keymap 自动解析(见 `main.rs` 中的绑定)。

use gpui_kit::{Menu, MenuItem, OsAction, SystemMenuType};

use ui::actions::{
    AboutMikan, CloseWindow, GoSettings, HideApp, HideOthers, MinimizeWindow, QuitApp, ShowAllApps,
    ToggleFullscreen, ZoomWindow,
};

/// 构建完整的应用菜单栏。
///
/// `fullscreen` 表示窗口当前是否处于全屏,用于切换「视图」菜单中的文案
/// (进入全屏 / 退出全屏),遵循 macOS 动态菜单标题的惯例。
pub fn build_menus(fullscreen: bool) -> Vec<Menu> {
    vec![
        // ---- App 菜单 ----
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
        // ---- 文件菜单 ----
        Menu {
            name: "文件".into(),
            disabled: false,
            items: vec![MenuItem::action("关闭窗口", CloseWindow)],
        },
        // ---- 编辑菜单 ----
        // 直接派发 gpui-component 输入框原生支持的编辑动作
        // (剪切/拷贝/粘贴/全选,作用于当前聚焦的输入框,
        // 自带系统剪贴板与快捷键处理)
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
        // ---- 视图菜单 ----
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
        // ---- 窗口菜单 ----
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
