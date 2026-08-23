//! 应用菜单栏。
//!
//! 遵循 macOS 人机界面指南:包含 App / 文件 / 编辑 / 视图 / 前往 / 窗口 / 帮助
//! 标准菜单结构。菜单项的键盘快捷键由 keymap 自动解析(见 `main.rs` 中的绑定)。

use gpui::{Menu, MenuItem, OsAction};

use crate::actions::{
    AboutMikan, CloseWindow, DarkMode, GoBack, GoFriday, GoHome, GoMonday, GoMovies, GoSaturday,
    GoSettings, GoSubscription, GoSunday, GoThursday, GoTuesday, GoWednesday, HideApp, HideOthers,
    LightMode, MinimizeWindow, OpenMikanWebsite, QuitApp, ToggleFullscreen, ToggleTheme,
    ZoomWindow,
};

/// 构建完整的应用菜单栏。
pub fn build_menus() -> Vec<Menu> {
    vec![
        // ---- App 菜单 ----
        Menu {
            name: "MikanPlus".into(),
            items: vec![
                MenuItem::action("关于 MikanPlus", AboutMikan),
                MenuItem::separator(),
                MenuItem::action("设置…", GoSettings),
                MenuItem::separator(),
                MenuItem::action("隐藏 MikanPlus", HideApp),
                MenuItem::action("隐藏其他", HideOthers),
                MenuItem::separator(),
                MenuItem::action("退出 MikanPlus", QuitApp),
            ],
        },
        // ---- 文件菜单 ----
        Menu {
            name: "文件".into(),
            items: vec![MenuItem::action("关闭窗口", CloseWindow)],
        },
        // ---- 编辑菜单 ----
        // 直接派发 gpui-component 输入框原生支持的编辑动作
        // (剪切/拷贝/粘贴/撤销/重做/全选,作用于当前聚焦的输入框,
        // 自带系统剪贴板与快捷键处理)
        Menu {
            name: "编辑".into(),
            items: vec![
                MenuItem::os_action("剪切", gpui_component::input::Cut, OsAction::Cut),
                MenuItem::os_action("拷贝", gpui_component::input::Copy, OsAction::Copy),
                MenuItem::os_action("粘贴", gpui_component::input::Paste, OsAction::Paste),
                MenuItem::separator(),
                MenuItem::os_action(
                    "全选",
                    gpui_component::input::SelectAll,
                    OsAction::SelectAll,
                ),
                MenuItem::action("撤销", gpui_component::input::Undo),
                MenuItem::action("重做", gpui_component::input::Redo),
            ],
        },
        // ---- 视图菜单 ----
        Menu {
            name: "视图".into(),
            items: vec![
                MenuItem::action("浅色模式", LightMode),
                MenuItem::action("深色模式", DarkMode),
                MenuItem::action("切换主题", ToggleTheme),
                MenuItem::separator(),
                MenuItem::action("进入全屏", ToggleFullscreen),
            ],
        },
        // ---- 前往菜单 ----
        Menu {
            name: "前往".into(),
            items: vec![
                MenuItem::action("返回", GoBack),
                MenuItem::separator(),
                MenuItem::action("今日更新", GoHome),
                MenuItem::action("我的订阅", GoSubscription),
                MenuItem::separator(),
                MenuItem::action("星期一", GoMonday),
                MenuItem::action("星期二", GoTuesday),
                MenuItem::action("星期三", GoWednesday),
                MenuItem::action("星期四", GoThursday),
                MenuItem::action("星期五", GoFriday),
                MenuItem::action("星期六", GoSaturday),
                MenuItem::action("星期日", GoSunday),
                MenuItem::separator(),
                MenuItem::action("剧场版", GoMovies),
            ],
        },
        // ---- 窗口菜单 ----
        Menu {
            name: "窗口".into(),
            items: vec![
                MenuItem::action("最小化", MinimizeWindow),
                MenuItem::action("缩放", ZoomWindow),
            ],
        },
        // ---- 帮助菜单 ----
        Menu {
            name: "帮助".into(),
            items: vec![MenuItem::action("蜜柑计划官网", OpenMikanWebsite)],
        },
    ]
}
