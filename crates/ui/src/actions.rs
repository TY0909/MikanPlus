//! 全局动作定义。
//!
//! 只保留 macOS 应用的标准菜单动作(关于/设置/隐藏/退出/关闭/最小化/缩放/全屏,
//! 以及编辑菜单的系统动作)。命名空间为 `mikan`,由 `MikanPlus` 注册的全局监听器处理。

use gpui_kit::actions;

actions!(
    mikan,
    [
        // ---- 导航 / 弹层 ----
        /// ⌘, 设置
        GoSettings,
        /// Esc 关闭筛选窗口
        CloseFilterModal,
        // ---- 窗口 / 应用 ----
        /// 关于
        AboutMikan,
        /// ⌘H 隐藏应用
        HideApp,
        /// ⌥⌘H 隐藏其他
        HideOthers,
        /// 全部显示(取消隐藏其他应用)
        ShowAllApps,
        /// ⌘Q 退出
        QuitApp,
        /// ⌘W 关闭窗口
        CloseWindow,
        /// ⌃⌘F 全屏
        ToggleFullscreen,
        /// ⌘M 最小化
        MinimizeWindow,
        /// 缩放窗口(绿色交通灯 / 菜单)
        ZoomWindow,
    ]
);
