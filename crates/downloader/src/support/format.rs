//! Numeric display formatting.

/// Rate formatting: `1536` → `1.5 MB/s`, `512` → `512 B/s`
pub fn format_rate(bytes_per_sec: u64) -> String {
    if bytes_per_sec >= 1024 * 1024 {
        format!("{:.1} MB/s", bytes_per_sec as f64 / 1024.0 / 1024.0)
    } else if bytes_per_sec >= 1024 {
        format!("{:.0} KB/s", bytes_per_sec as f64 / 1024.0)
    } else {
        format!("{bytes_per_sec} B/s")
    }
}

/// Percentage formatting: `0.375` → `37%`
pub fn format_percent(progress: f64) -> String {
    format!("{:.0}%", progress * 100.0)
}
