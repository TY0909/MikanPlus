// 将应用图标嵌入 Windows 可执行文件(资源管理器 / 任务栏显示)。
// 仅 Windows 生效;图标文件缺失时静默跳过,不影响开发构建。
fn main() {
    #[cfg(target_os = "windows")]
    {
        let ico = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
            .join("mikan_icon.ico");
        if ico.exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon(&ico.to_string_lossy().replace('/', "\\"));
            let _ = res.compile();
        }
    }
}
