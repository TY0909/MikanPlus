# Development Guide

本文档面向 MikanPlus 开发者，说明本地开发、workspace 边界、资源管理和质量门禁。

## 环境要求

- Rust 1.88 或更高版本
- Cargo
- GPUI Kit 支持的桌面环境

## 常用命令

```bash
cargo run -p app
cargo check --workspace --all-targets
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- --deny warnings
cargo test --workspace --all-features
cargo build --release --locked -p app
```

如果系统临时目录空间不足，可将 `TMPDIR` 指向项目的 `target/tmp`：

```bash
TMPDIR=/home/ty/Projects/MikanPlus/target/tmp cargo test --workspace --all-features --locked
```

## Workspace

| Crate | 职责 |
| --- | --- |
| `domain` | 与框架无关的领域模型和导航状态 |
| `storage` | 状态持久化、缓存、迁移和平台路径 |
| `source` | 蜜柑 HTTP 访问与 HTML 解析 |
| `downloader` | BT 下载生命周期与任务快照 |
| `ui` | GPUI Kit 页面、组件和主题表现 |
| `app` | 窗口、菜单、快捷键及服务组合根 |

详细的依赖方向和设计约束见 [`architecture.md`](architecture.md)。

## GPUI Kit 约定

- GPUI 生态统一通过 `gpui-kit` 使用，不直接依赖 `gpui`、`gpui-component` 或 `gpui-base`。
- 创建任何组件视图前调用 `gpui_kit::init(cx)`。
- 每个窗口的第一层视图使用 `Root`。
- 应用 shell 只负责服务组合和全局生命周期管理。
- 重复 UI 元素使用稳定的领域 ID。
- 渲染代码优先使用主题提供的语义颜色。

## 资源管理

应用使用组合式 AssetSource：

1. 优先加载项目 `assets/` 中的资源。
2. 项目中不存在时回退到 `gpui_kit::assets::Assets`。

项目只保存自身资源和 GPUI Kit 未提供的资源：

- `mikan-pic.png`：应用 Logo
- `mikan_icon.ico`：Windows 可执行文件图标
- `screenshot_1.png`：README 截图
- `assets/icons/`：GPUI Kit 0.6 未提供、但应用仍在使用的图标

不要把 GPUI Kit 已内置的图标复制到项目中。本地同名资源会覆盖 GPUI Kit 资源，只应在确实需要定制视觉时添加。

## 质量门禁

提交前应确保以下命令全部通过：

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- --deny warnings
cargo test --workspace --all-features
```
