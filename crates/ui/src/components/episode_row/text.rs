//! Display-width estimation and truncation for episode titles (conservative estimate at the data layer).

/// Estimate whether text exceeds the capacity of the given line width / line count (conservative estimate at the data layer).
///
/// Used to decide whether to attach a tooltip to text clamped by `line_clamp`: characters are laid out
/// greedily by width (full-width ≈ font size, half-width ≈ 0.55× font size), the line width is taken at 95%,
/// leaving margin for the trailing space wasted when Latin text wraps at word boundaries.
pub fn exceeds_lines(text: &str, line_width_px: f32, font_size: f32, max_lines: usize) -> bool {
    if line_width_px <= 0.0 || max_lines == 0 {
        return false;
    }
    let effective = line_width_px * 0.95;
    let mut lines = 1usize;
    let mut used = 0.0f32;
    for ch in text.chars() {
        let w = if ch.is_ascii() {
            font_size * 0.55
        } else {
            font_size
        };
        if used + w > effective {
            lines += 1;
            used = 0.0;
            if lines > max_lines {
                return true;
            }
        }
        used += w;
    }
    false
}

/// Truncate an episode title by display width (the overflow is replaced with an ellipsis).
///
/// gpui 0.2.2's text ellipsis is unreliable in flex layouts with multiple levels of percentage widths
/// (it does not truncate when the available space is not Definite on the first measurement), so a
/// conservative truncation is performed at the data layer to keep over-long titles from overflowing into
/// the action area.
///
/// Width estimate: full-width characters (Chinese, etc.) ≈ 1 font size, half-width (ASCII) ≈ 0.55 font size.
pub fn truncate_title(text: &str, max_px: f32, font_size: f32) -> String {
    if max_px <= 0.0 {
        return text.to_string();
    }
    const ELLIPSIS: char = '…';
    let ellipsis_w = font_size; // Count the ellipsis as full-width.
    let mut used = 0.0f32;
    for (i, ch) in text.chars().enumerate() {
        let w = if ch.is_ascii() {
            font_size * 0.55
        } else {
            font_size
        };
        // Truncate when the current character plus the ellipsis does not fit.
        if used + w + ellipsis_w > max_px {
            let mut out: String = text.chars().take(i).collect();
            out.push(ELLIPSIS);
            return out;
        }
        used += w;
    }
    text.to_string()
}

#[cfg(test)]
mod truncate_tests {
    use super::truncate_title;

    fn estimated_width(text: &str, font_size: f32) -> f32 {
        text.chars()
            .map(|c| {
                if c.is_ascii() {
                    font_size * 0.55
                } else {
                    font_size
                }
            })
            .sum()
    }

    #[test]
    fn short_text_untouched() {
        assert_eq!(truncate_title("第 1 集", 200.0, 14.0), "第 1 集");
        assert_eq!(truncate_title("S01E01", 200.0, 14.0), "S01E01");
    }

    #[test]
    fn long_text_truncated_within_limit() {
        let long = "【某某字幕组】这个番剧的名字真的非常非常长以至于肯定装不下一行";
        let out = truncate_title(long, 150.0, 14.0);
        assert!(out.ends_with('…'), "应以省略号结尾: {out}");
        assert!(out.len() < long.len(), "应发生截断: {out}");
        assert!(
            estimated_width(&out, 14.0) <= 150.0 + 0.01,
            "截断结果宽度应不超过限制: {:?} vs 150",
            estimated_width(&out, 14.0)
        );
    }

    #[test]
    fn ascii_long_truncated() {
        let long = "[SomeFanSub] Some.Anime.Title.S2.E01.1080p.WEB-DL.AAC.x264-SomeFanSub";
        let out = truncate_title(long, 120.0, 14.0);
        assert!(out.ends_with('…'));
        assert!(estimated_width(&out, 14.0) <= 120.0 + 0.01);
    }

    #[test]
    fn single_char_tight_space() {
        // Only enough space for 1 full-width character plus the ellipsis.
        let out = truncate_title("一二三", 14.0 * 2.0, 14.0);
        assert_eq!(out, "一…");
    }
}
