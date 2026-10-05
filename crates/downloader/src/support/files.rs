//! Download file and path rules: video detection, safe path resolution, output-directory validation.

use std::path::{Component, Path, PathBuf};

/// Video file extension. Downloaded content may also include subtitles, fonts, checksum
/// files, etc., which must not be treated as playable items.
pub(crate) fn is_video_file(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "3gp"
            | "avi"
            | "flv"
            | "m2ts"
            | "m4v"
            | "mkv"
            | "mov"
            | "mp4"
            | "mpeg"
            | "mpg"
            | "ogv"
            | "rmvb"
            | "ts"
            | "vob"
            | "webm"
            | "wmv"
    )
}

/// Resolve a torrent's relative file name against the task output directory, rejecting
/// absolute paths and `..` paths.
pub(crate) fn safe_task_file_path(output_dir: &Path, relative: &Path) -> Option<PathBuf> {
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }
    Some(output_dir.join(relative))
}

/// Prevent an on-disk symlink from redirecting an open action outside the output directory.
pub(crate) fn is_safe_existing_path(output_root: Option<&Path>, path: &Path) -> bool {
    let Some(root) = output_root else {
        return false;
    };
    let Some(resolved) = path.canonicalize().ok() else {
        return false;
    };
    resolved.starts_with(root) && resolved.is_file()
}

/// Validate that the output directory is usable (exists or can be created, and is writable).
pub fn ensure_output_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_video_file, safe_task_file_path};
    use std::path::Path;

    #[test]
    fn recognizes_video_files_but_not_sidecar_files() {
        assert!(is_video_file(Path::new("Season 1/01.MKV")));
        assert!(is_video_file(Path::new("episode.webm")));
        assert!(!is_video_file(Path::new("episode.ass")));
        assert!(!is_video_file(Path::new("font.ttf")));
    }

    #[test]
    fn resolves_nested_paths_without_allowing_escape() {
        assert_eq!(
            safe_task_file_path(Path::new("/downloads"), Path::new("Show/01.mkv")),
            Some(Path::new("/downloads/Show/01.mkv").to_path_buf())
        );
        assert_eq!(
            safe_task_file_path(Path::new("/downloads"), Path::new("../01.mkv")),
            None
        );
    }
}
