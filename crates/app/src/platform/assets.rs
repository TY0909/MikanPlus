//! Application assets: resource directory resolution and the `AssetSource` implementation
//! (project resources first, falling back to GPUI Kit).

use gpui_kit::{AssetSource, SharedString};
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

/// The application asset source: project resources first, falling back to GPUI Kit.
pub(crate) struct Assets {
    base: PathBuf,
}

impl Assets {
    /// Locate and wrap the application asset directory for the current launch layout.
    ///
    /// Resolution does not depend on the working directory at startup:
    /// - macOS .app bundle: `Contents/MacOS/../Resources/assets`
    /// - Linux installed / Windows portable: `assets/` next to the executable
    /// - development fallback: working directory `assets` (cargo run)
    pub(crate) fn locate() -> Self {
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            #[cfg(target_os = "macos")]
            {
                let resources = dir.join("../Resources").join("assets");
                if resources.is_dir() {
                    return Assets { base: resources };
                }
            }
            let beside_exe = dir.join("assets");
            if beside_exe.is_dir() {
                return Assets { base: beside_exe };
            }
        }
        Assets {
            base: PathBuf::from("assets"),
        }
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        match fs::read(self.base.join(path)) {
            Ok(data) => Ok(Some(Cow::Owned(data))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                gpui_kit::assets::Assets.load(path)
            }
            // io::Error → anyhow::Error (the underlying type of gpui::Result) is converted automatically by From
            Err(error) => Err(error.into()),
        }
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<SharedString>> {
        let mut assets = gpui_kit::assets::Assets.list(path)?;
        match fs::read_dir(self.base.join(path)) {
            Ok(entries) => assets.extend(
                entries
                    .filter_map(|entry| {
                        entry
                            .ok()
                            .and_then(|entry| entry.file_name().into_string().ok())
                    })
                    .map(SharedString::from),
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        assets.sort_unstable();
        assets.dedup();
        Ok(assets)
    }
}
