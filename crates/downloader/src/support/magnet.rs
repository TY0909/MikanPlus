//! Magnet link parsing.

/// Extract the info_hash from a magnet link (btih, lowercase hex).
/// Example: `magnet:?xt=urn:btih:abc123...` → `abc123...`
pub fn magnet_info_hash(magnet: &str) -> Option<String> {
    let lower = magnet.to_lowercase();
    // "xt=urn:btih:" is 12 characters
    let idx = lower.find("xt=urn:btih:")?;
    let rest = &lower[idx + 12..];
    let hex: String = rest.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
    (hex.len() == 40).then_some(hex)
}

#[cfg(test)]
mod tests {
    use super::magnet_info_hash;

    #[test]
    fn extracts_btih_hex() {
        let magnet = "magnet:?xt=urn:btih:c06e0fa66e76e5f30d10e4b00eaa2472b6d62a37&tr=http%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce&dn=test";
        assert_eq!(
            magnet_info_hash(magnet).as_deref(),
            Some("c06e0fa66e76e5f30d10e4b00eaa2472b6d62a37")
        );
    }

    #[test]
    fn rejects_non_40hex() {
        assert_eq!(magnet_info_hash("magnet:?xt=urn:btih:abc"), None);
        assert_eq!(magnet_info_hash("no-magnet"), None);
    }
}
