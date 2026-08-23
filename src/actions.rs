//! 全局动作定义。
//!
//! 所有可通过菜单栏、键盘快捷键触发的动作都定义在这里。
//! 命名空间为 `mikan`,绑定快捷键后由 `MikanPlus` 根元素统一处理。

use gpui::actions;

actions!(
    mikan,
    [
        // ---- 导航 ----
        /// ⌘1 今日更新
        GoHome,
        /// ⌘2 我的订阅
        GoSubscription,
        /// ⌘4 星期一
        GoMonday,
        /// ⌘5 星期二
        GoTuesday,
        /// ⌘6 星期三
        GoWednesday,
        /// ⌘7 星期四
        GoThursday,
        /// ⌘8 星期五
        GoFriday,
        /// ⌘9 星期六
        GoSaturday,
        /// ⌘0 星期日
        GoSunday,
        /// ⌘⇧M 剧场版
        GoMovies,
        /// ⌘, 设置
        GoSettings,
        /// ⌘[ 返回
        GoBack,
        /// ⌘F 聚焦搜索
        FocusSearch,
        /// ⌘⇧L 切换主题
        ToggleTheme,
        /// Esc 关闭筛选窗口
        CloseFilterModal,
        // ---- 窗口 / 应用 ----
        /// 关于
        AboutMikan,
        /// ⌘H 隐藏应用
        HideApp,
        /// ⌥⌘H 隐藏其他
        HideOthers,
        /// ⌘Q 退出
        QuitApp,
        /// ⌘W 关闭窗口
        CloseWindow,
        /// ⌘⌃F 全屏
        ToggleFullscreen,
        /// ⌘M 最小化
        MinimizeWindow,
        /// 缩放窗口(绿色交通灯 / 菜单)
        ZoomWindow,
        // ---- 外观 ----
        /// 浅色模式
        LightMode,
        /// 深色模式
        DarkMode,
        // ---- 帮助 ----
        /// 打开蜜柑计划官网
        OpenMikanWebsite,
    ]
);
