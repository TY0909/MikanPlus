// Embed the application icon into the Windows executable (shown in Explorer / the taskbar).
// Only takes effect on Windows; when the icon file is missing it is skipped silently, so it does not affect development builds.
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
