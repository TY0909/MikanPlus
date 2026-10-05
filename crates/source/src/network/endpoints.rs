//! Data-source addresses and pure URL transformations (selection of the primary site / backup
//! domain, normalization, and joining).

/// Site root address (primary site by default).
pub(crate) const BASE_URL: &str = "https://mikanani.me";
/// Backup domain: directly reachable from mainland China (it serves content directly to mainland
/// IPs; overseas access is 302-redirected back to the primary site, and both domains are the same
/// origin).
pub(crate) const BACKUP_BASE_URL: &str = "https://mikanime.tv";

/// The site root for the given data-source selection.
pub(crate) fn base_url(use_backup: bool) -> &'static str {
    if use_backup {
        BACKUP_BASE_URL
    } else {
        BASE_URL
    }
}

/// Replaces any known data-source host in `url` with `base`. A fallback for absolute addresses
/// saved under an old domain in historical caches / subscriptions, so they keep working after
/// switching domains. Substitution happens only when the host is followed by an empty string or a
/// path, to avoid accidentally rewriting other domains that share the same prefix.
pub(crate) fn rewrite_host(url: &str, base: &str) -> String {
    [BASE_URL, BACKUP_BASE_URL]
        .iter()
        .find_map(|host| {
            url.strip_prefix(host)
                .filter(|rest| rest.is_empty() || rest.starts_with('/'))
                .map(|rest| format!("{base}{rest}"))
        })
        .unwrap_or_else(|| url.to_string())
}

/// The URL for the same path on the other data-source host.
///
/// Used for image-fetch fallback: the backup domain `mikanime.tv` 302-redirects images back to the
/// primary site, but its `Location` lowercases the path (`/images/Bangumi` → `/images/bangumi`),
/// and the case-sensitive primary site then returns 404; conversely, when the primary site is
/// unreachable the backup domain can connect directly. Returns `None` when the URL is not under
/// any known data-source host.
pub(crate) fn alternate_url(url: &str) -> Option<String> {
    [BASE_URL, BACKUP_BASE_URL].iter().find_map(|host| {
        url.strip_prefix(host)
            .filter(|rest| rest.is_empty() || rest.starts_with('/'))
            .map(|rest| {
                let other = if *host == BASE_URL {
                    BACKUP_BASE_URL
                } else {
                    BASE_URL
                };
                format!("{other}{rest}")
            })
    })
}

/// Joins a full site URL; only the http/https scheme is accepted (case-insensitive) to prevent
/// malformed URLs or local resources such as `file://` from a badly parsed link.
pub(crate) fn site_url(base: &str, path: &str) -> String {
    let lower = path.trim().to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        path.trim().to_string()
    } else if lower.starts_with("//") {
        format!("https:{}", path.trim())
    } else {
        format!("{base}{path}")
    }
}

/// Search-page URL (with the keyword URL-encoded).
pub(crate) fn search_url(base: &str, query: &str) -> String {
    let mut encoded = String::new();
    for b in query.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char)
            }
            _ => encoded.push_str(&format!("%{b:02X}")),
        }
    }
    format!("{base}/Home/Search?searchstr={encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_url_join() {
        assert_eq!(
            site_url(BASE_URL, "/Home/Bangumi"),
            "https://mikanani.me/Home/Bangumi"
        );
        assert_eq!(
            site_url(BASE_URL, "https://other.com/x"),
            "https://other.com/x",
            "绝对地址原样返回"
        );
    }

    #[test]
    fn host_rewrite_normalizes_both_mirrors() {
        // Primary → backup
        assert_eq!(
            rewrite_host("https://mikanani.me/images/a.jpg", BACKUP_BASE_URL),
            "https://mikanime.tv/images/a.jpg"
        );
        // Backup → primary
        assert_eq!(
            rewrite_host("https://mikanime.tv/Home/Bangumi/3883", BASE_URL),
            "https://mikanani.me/Home/Bangumi/3883"
        );
        // Unknown host / relative path is left unchanged
        assert_eq!(
            rewrite_host("https://other.com/x", BASE_URL),
            "https://other.com/x"
        );
        assert_eq!(rewrite_host("/images/a.jpg", BASE_URL), "/images/a.jpg");
        // Other domains sharing the same prefix are not rewritten
        assert_eq!(
            rewrite_host("https://mikanani.me.evil.com/x", BASE_URL),
            "https://mikanani.me.evil.com/x"
        );
    }

    #[test]
    fn alternate_url_swaps_known_hosts() {
        assert_eq!(
            alternate_url("https://mikanani.me/images/Bangumi/a.jpg?width=400&height=560"),
            Some("https://mikanime.tv/images/Bangumi/a.jpg?width=400&height=560".to_string())
        );
        assert_eq!(
            alternate_url("https://mikanime.tv/images/Bangumi/a.jpg"),
            Some("https://mikanani.me/images/Bangumi/a.jpg".to_string())
        );
        // Unknown host / same-prefix domain is not rewritten
        assert_eq!(alternate_url("https://other.com/x"), None);
        assert_eq!(alternate_url("https://mikanani.me.evil.com/x"), None);
        assert_eq!(alternate_url("/images/a.jpg"), None);
    }
}
