//! Model badge with plain context, metrics, and a secondary information row.

use crate::git::{self, GitInfo};
use crate::input::StatusInput;
use crate::render::{
    compact_model, display_width, fmt_countdown_at, fmt_duration, now_unix, shorten,
};
use crate::theme::{self, ribbon as palette, RESET};
use palette::Color;

struct Part {
    text: String,
    color: String,
}

fn part(text: impl Into<String>, color: Color) -> Part {
    ansi_part(text, &palette::foreground(color))
}

fn ansi_part(text: impl Into<String>, color: &str) -> Part {
    Part {
        text: text.into(),
        color: color.to_string(),
    }
}

struct Segment {
    parts: Vec<Part>,
    background: Option<Color>,
    compact: Option<Vec<Part>>,
    separator_after: Option<&'static str>,
    hyperlink: Option<String>,
    priority: u8,
}

impl Segment {
    fn new(parts: Vec<Part>, background: Option<Color>, priority: u8) -> Self {
        Self {
            parts,
            background,
            compact: None,
            separator_after: None,
            hyperlink: None,
            priority,
        }
    }
    fn text(&self) -> String {
        self.parts.iter().map(|p| p.text.as_str()).collect()
    }
    fn width(&self) -> usize {
        display_width(&self.text()) + if self.background.is_some() { 3 } else { 0 }
    }
}

fn separator(left: &Segment, right: &Segment, ascii: bool) -> &'static str {
    if let Some(separator) = left.separator_after {
        return separator;
    }
    match (left.background, right.background) {
        (Some(_), Some(_)) => "",
        (None, None) if !ascii => " │ ",
        (None, None) => " | ",
        _ => " ",
    }
}

fn width(segments: &[Segment]) -> usize {
    segments.iter().map(Segment::width).sum::<usize>()
        + segments
            .windows(2)
            .map(|pair| display_width(separator(&pair[0], &pair[1], false)))
            .sum::<usize>()
}

fn paint(segments: &[Segment], ascii: bool) -> String {
    let mut output = String::new();
    for (index, segment) in segments.iter().enumerate() {
        if index > 0 {
            output.push_str(theme::SEP);
            output.push_str(separator(&segments[index - 1], segment, ascii));
            output.push_str(RESET);
        }
        if let Some(bg) = segment.background {
            output.push_str(&palette::background(bg));
            output.push(' ');
        }
        if let Some(url) = &segment.hyperlink {
            output.push_str(&format!("\x1b]8;;{url}\x07"));
        }
        for p in &segment.parts {
            // DIM is a classic style, not a foreground color. Clear it before
            // every field so neutral labels cannot dim the following accent.
            // Preserve the model background while resetting only text styles.
            output.push_str("\x1b[22;39m");
            output.push_str(&p.color);
            output.push_str(&p.text);
        }
        if segment.hyperlink.is_some() {
            output.push_str("\x1b]8;;\x07");
        }
        if let Some(bg) = segment.background {
            output.push(' ');
            output.push_str(RESET);
            output.push_str(&palette::foreground(bg));
            if let Some(next_bg) = segments.get(index + 1).and_then(|s| s.background) {
                output.push_str(&palette::background(next_bg));
            }
            output.push(if ascii { '>' } else { '▶' });
        }
        output.push_str(RESET);
    }
    if output.is_empty() {
        output.push_str(RESET);
    }
    output
}

fn fit(mut segments: Vec<Segment>, cols: usize, ascii: bool) -> String {
    while width(&segments) > cols {
        if let Some(segment) = segments.iter_mut().find(|s| s.compact.is_some()) {
            segment.parts = segment.compact.take().unwrap_or_default();
        } else if segments.len() > 1 {
            let index = segments
                .iter()
                .enumerate()
                .min_by_key(|(_, s)| s.priority)
                .map(|(i, _)| i)
                .unwrap_or(0);
            segments.remove(index);
        } else {
            return format!(
                "{}{}{RESET}",
                theme::DIM,
                shorten(
                    &segments.first().map(Segment::text).unwrap_or_default(),
                    cols
                )
            );
        }
    }
    paint(&segments, ascii)
}

fn clean(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}

pub fn render(data: &StatusInput, cols: usize, ascii: bool, one_line: bool) -> String {
    let now = now_unix();
    let first = first_row(data, cols, ascii, now);
    if one_line {
        first
    } else {
        let branch = git::status(&data.workspace.current_dir, &data.session_id);
        let repository_url = git::repository_url(&data.workspace.current_dir, &data.session_id);
        format!(
            "{first}\n{}",
            second_row(
                data,
                cols,
                ascii,
                now,
                branch.as_ref(),
                repository_url.as_deref()
            )
        )
    }
}

fn first_row(data: &StatusInput, cols: usize, ascii: bool, now: i64) -> String {
    let mut model = clean(&compact_model(&data.model.display_name));
    if let Some(effort) = data.effort.as_ref().filter(|e| !e.level.is_empty()) {
        model.push_str(" · ");
        model.push_str(&clean(&effort.level));
    }
    if data.fast_mode {
        model.push_str(" · fast");
    }
    let mut segments = vec![Segment::new(
        vec![part(model, palette::MODEL_TEXT)],
        Some(palette::MODEL_BG),
        100,
    )];
    if let Some(percent) = data
        .context_window
        .used_percentage
        .filter(|p| p.is_finite())
    {
        let percent = percent.clamp(0.0, 100.0);
        let core = || {
            vec![
                ansi_part("ctx ", theme::DIM),
                part(format!("{percent:.0}%"), palette::context_color(percent)),
            ]
        };
        let mut parts = core();
        let filled = ((percent / 100.0 * 6.0).round() as usize).min(6);
        let tick = if ascii { "-" } else { "━" };
        parts.push(ansi_part(" ", theme::DIM));
        parts.push(part(tick.repeat(filled), palette::context_color(percent)));
        parts.push(part(tick.repeat(6 - filled), palette::TRACK));
        let mut context = Segment::new(parts, None, 60);
        context.compact = Some(core());
        context.separator_after = Some(" · ");
        segments.push(context);
    }
    if let Some(limits) = &data.rate_limits {
        let mut parts = Vec::new();
        for (label, window) in [
            ("5h", limits.five_hour.as_ref()),
            ("7d", limits.seven_day.as_ref()),
            ("$", limits.spend_limit.as_ref()),
        ] {
            let Some(window) = window else { continue };
            let Some(percent) = window.used_percentage.filter(|p| p.is_finite()) else {
                continue;
            };
            let percent = if label == "$" {
                percent.max(0.0)
            } else {
                percent.clamp(0.0, 100.0)
            };
            if !parts.is_empty() {
                parts.push(ansi_part(" · ", theme::SEP));
            }
            parts.push(ansi_part(format!("{label} "), theme::WHITE));
            parts.push(part(
                format!("{percent:.0}%"),
                palette::quota_color(percent),
            ));
            if let Some(reset) = window.resets_at {
                let time = fmt_countdown_at(reset, now);
                parts.push(ansi_part(format!(" {time}"), theme::DIM));
            }
        }
        if !parts.is_empty() {
            segments.push(Segment::new(parts, None, 95));
        }
    }
    if let Some(cost) = data.cost.total_cost_usd.filter(|c| c.is_finite()) {
        segments.push(Segment::new(
            vec![ansi_part(format!("${cost:.2}"), theme::COST)],
            None,
            30,
        ));
    }
    fit(segments, cols, ascii)
}

fn cache_segment(data: &StatusInput, now: i64) -> Option<Segment> {
    let (percent, tail) = if let Some(cache) = &data.prompt_cache {
        if !cache.caching_observed {
            return None;
        }
        let percent = (cache.hit_ratio.filter(|p| p.is_finite())? * 100.0).clamp(0.0, 100.0);
        let tail = if !cache.warm || cache.expires_at.is_some_and(|expiry| expiry <= now) {
            Some(part("(cold)", palette::COLD))
        } else {
            cache
                .expires_at
                .map(|expiry| ansi_part(format!("({})", fmt_countdown_at(expiry, now)), theme::DIM))
        };
        (percent, tail)
    } else {
        let usage = data.context_window.current_usage.as_ref()?;
        let total = u128::from(usage.input_tokens)
            + u128::from(usage.cache_creation_input_tokens)
            + u128::from(usage.cache_read_input_tokens);
        let percent = (u128::from(usage.cache_read_input_tokens) * 100).checked_div(total)?;
        (percent as f64, None)
    };
    let mut parts = vec![
        ansi_part("Cache: ", theme::DIM),
        ansi_part(format!("{percent:.0}%"), theme::CACHE),
    ];
    if let Some(tail) = tail {
        parts.push(tail);
    }
    Some(Segment::new(parts, None, 100))
}

fn second_row(
    data: &StatusInput,
    cols: usize,
    ascii: bool,
    now: i64,
    branch: Option<&GitInfo>,
    repository_url: Option<&str>,
) -> String {
    let mut segments = Vec::new();
    if let Some(cache) = cache_segment(data, now) {
        segments.push(cache);
    }
    let mut metadata = Vec::new();
    if !data.version.is_empty() {
        metadata.push(ansi_part(
            format!("v{}", clean(&data.version)),
            theme::VERSION,
        ));
    }
    if let Some(duration) = data.cost.total_duration_ms {
        if !metadata.is_empty() {
            metadata.push(ansi_part(" · ", theme::SEP));
        }
        metadata.push(ansi_part(fmt_duration(duration), theme::WHITE));
    }
    if !metadata.is_empty() {
        let mut segment = Segment::new(metadata, None, 10);
        if !data.version.is_empty() {
            segment.compact = data
                .cost
                .total_duration_ms
                .map(|duration| vec![ansi_part(fmt_duration(duration), theme::WHITE)]);
        }
        segments.push(segment);
    }
    if let Some(branch) = branch {
        let mut parts = vec![part(shorten(&clean(&branch.branch), 24), palette::BRANCH)];
        if branch.dirty {
            parts.push(part("*", palette::RED));
        }
        segments.push(Segment::new(parts, None, 30));
    }
    let path = clean(&data.workspace.current_dir).replace('\\', "/");
    if path.is_empty() {
        return fit(segments, cols, ascii);
    }
    let cache_width = segments
        .iter()
        .find(|s| s.priority == 100)
        .map_or(0, |s| s.width() + 3);
    let leaf = path.rsplit('/').find(|s| !s.is_empty()).unwrap_or(&path);
    let minimum_path = (display_width(leaf) + 2).min(cols.saturating_sub(cache_width));
    let mut path_segment = Segment::new(Vec::new(), None, 90);
    path_segment.hyperlink = repository_url.map(str::to_owned);
    loop {
        let sep = segments
            .last()
            .map_or(0, |s| display_width(separator(s, &path_segment, ascii)));
        if width(&segments) + sep + minimum_path <= cols {
            break;
        }
        if let Some(segment) = segments.iter_mut().find(|s| s.compact.is_some()) {
            segment.parts = segment.compact.take().unwrap_or_default();
        } else if let Some(index) = segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.priority < 100)
            .min_by_key(|(_, s)| s.priority)
            .map(|(i, _)| i)
        {
            segments.remove(index);
        } else {
            break;
        }
    }
    let sep = segments
        .last()
        .map_or(0, |s| display_width(separator(s, &path_segment, ascii)));
    let available = cols.saturating_sub(width(&segments) + sep);
    if available > 0 {
        path_segment.parts = path_parts(&path, available, ascii);
        segments.push(path_segment);
    }
    fit(segments, cols, ascii)
}

fn path_parts(path: &str, cols: usize, ascii: bool) -> Vec<Part> {
    let mut parts = Vec::new();
    let marker = if cols > 2 {
        if ascii {
            "> "
        } else {
            "› "
        }
    } else {
        ""
    };
    if !marker.is_empty() {
        parts.push(part(marker, palette::PATH));
    }
    let path = fit_path(path, cols - display_width(marker));
    let name_start = path.trim_end_matches('/').rfind('/').map_or(0, |i| i + 1);
    if name_start > 0 {
        parts.push(ansi_part(&path[..name_start], theme::DIM));
    }
    parts.push(part(&path[name_start..], palette::PATH));
    parts
}

fn fit_path(path: &str, cols: usize) -> String {
    if display_width(path) <= cols {
        return path.to_string();
    }
    if cols == 0 {
        return String::new();
    }
    let mut suffix = Vec::new();
    let mut cells = 1;
    for c in path.chars().rev() {
        let width = display_width(&c.to_string());
        if cells + width > cols {
            break;
        }
        suffix.push(c);
        cells += width;
    }
    format!("…{}", suffix.iter().rev().collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> StatusInput {
        serde_json::from_value(serde_json::json!({
            "model":{"display_name":"Opus 5"}, "effort":{"level":"high"},
            "context_window":{"used_percentage":32},
            "version":"2.1.257", "cost":{"total_cost_usd":12.84,"total_duration_ms":6120000},
            "workspace":{"current_dir":"D:\\Github\\claude-code-plugin-hud"},
            "rate_limits":{"five_hour":{"used_percentage":24,"resets_at":14680},"seven_day":{"used_percentage":38,"resets_at":218800}},
            "prompt_cache":{"warm":true,"caching_observed":true,"hit_ratio":0.91,"expires_at":12820}
        })).unwrap()
    }

    fn plain(text: &str) -> String {
        let mut chars = text.chars();
        let mut output = String::new();
        while let Some(c) = chars.next() {
            if c != '\x1b' {
                output.push(c);
                continue;
            }
            let end = match chars.next() {
                Some('[') => 'm',
                Some(']') => '\x07',
                _ => continue,
            };
            for c in chars.by_ref() {
                if c == end {
                    break;
                }
            }
        }
        output
    }

    #[test]
    fn selected_layout_keeps_cache_on_second_track_and_path_last() {
        let data = sample();
        let branch = GitInfo {
            branch: "main".into(),
            dirty: true,
        };
        let first = first_row(&data, 116, false, 10000);
        let second = second_row(&data, 116, false, 10000, Some(&branch), None);
        assert_eq!(
            plain(&first),
            " Opus 5 · high ▶ ctx 32% ━━━━━━ · 5h 24% 1h18m · 7d 38% 2d10h │ $12.84"
        );
        assert_eq!(
            plain(&second),
            "Cache: 91%(47m) │ v2.1.257 · 1h42m │ main* │ › D:/Github/claude-code-plugin-hud"
        );
        assert!(!first[first.find("ctx").unwrap()..].contains("\x1b[48;"));
        assert_eq!(first.matches('▶').count(), 1);
        assert!(!second.contains("\x1b[48;"));
        assert!(!second.contains('▶'));
    }

    #[test]
    fn unchanged_input_advances_all_countdowns_and_expires_stale_warm_cache() {
        let mut data = sample();
        data.rate_limits
            .as_mut()
            .unwrap()
            .five_hour
            .as_mut()
            .unwrap()
            .resets_at = Some(13599);
        data.rate_limits
            .as_mut()
            .unwrap()
            .seven_day
            .as_mut()
            .unwrap()
            .resets_at = Some(13659);
        data.prompt_cache.as_mut().unwrap().expires_at = Some(13599);
        for cols in [76, 116] {
            for (now, five, seven, cache) in [
                (10000, "59m", "1h0m", "59m"),
                (10060, "58m", "59m", "58m"),
                (14000, "now", "now", "cold"),
            ] {
                let first = plain(&first_row(&data, cols, false, now));
                let second = plain(&second_row(&data, cols, false, now, None, None));
                assert!(first.contains(&format!("5h 24% {five}")), "{first}");
                assert!(first.contains(&format!("7d 38% {seven}")), "{first}");
                assert!(second.contains(&format!("Cache: 91%({cache})")), "{second}");
            }
        }
    }

    #[test]
    fn both_tracks_fit_unicode_and_narrow_widths() {
        let mut data = sample();
        data.model.display_name = "Opus 开发🧪超长名称".repeat(8);
        data.version = "2.1.257测试".repeat(8);
        data.workspace.current_dir = "D:\\团队🧪\\长路径\\claude-code-plugin-hud".into();
        let branch = GitInfo {
            branch: "feature/开发🧪分支".repeat(8),
            dirty: true,
        };
        for cols in 0..180 {
            for ascii in [false, true] {
                for row in [
                    first_row(&data, cols, ascii, 10000),
                    second_row(&data, cols, ascii, 10000, Some(&branch), None),
                ] {
                    assert!(row.ends_with(RESET));
                    assert!(
                        display_width(&plain(&row)) <= cols,
                        "{cols}: {}",
                        plain(&row)
                    );
                    assert!(!plain(&row).contains(['\n', '\r', '\\']));
                    if ascii {
                        assert!(!row.contains(['▶', '│', '━', '›']));
                    }
                }
            }
        }
    }

    #[test]
    fn metadata_normalizes_full_paths_and_handles_missing_fields() {
        let mut data = StatusInput::default();
        for path in [
            "D:\\Github/团队\\project",
            "\\\\server\\share/mixed\\项目",
            "/home/u/project",
            "",
        ] {
            data.workspace.current_dir = path.into();
            assert_eq!(
                plain(&second_row(&data, 116, false, 0, None, None)),
                if path.is_empty() {
                    String::new()
                } else {
                    format!("› {}", path.replace('\\', "/"))
                }
            );
        }
        data = sample();
        assert!(plain(&second_row(&data, 76, false, 10000, None, None))
            .ends_with("/claude-code-plugin-hud"));
        data.prompt_cache = None;
        let row = second_row(&data, 116, false, 10000, None, None);
        assert!(!row.contains("\x1b[48;"));
        assert!(!row.contains("Cache:"));
    }

    #[test]
    fn cache_optional_legacy_and_large_counters_are_safe() {
        let mut data = sample();
        data.prompt_cache.as_mut().unwrap().caching_observed = false;
        assert!(cache_segment(&data, 10000).is_none());
        data.prompt_cache.as_mut().unwrap().caching_observed = true;
        data.prompt_cache.as_mut().unwrap().expires_at = None;
        assert_eq!(cache_segment(&data, 10000).unwrap().text(), "Cache: 91%");
        data.prompt_cache.as_mut().unwrap().warm = false;
        assert_eq!(
            cache_segment(&data, 10000).unwrap().text(),
            "Cache: 91%(cold)"
        );
        data.prompt_cache = None;
        data.context_window.current_usage = Some(crate::input::CurrentUsage {
            input_tokens: u64::MAX,
            cache_creation_input_tokens: u64::MAX,
            cache_read_input_tokens: u64::MAX,
        });
        assert_eq!(cache_segment(&data, 0).unwrap().text(), "Cache: 33%");
        data.context_window.current_usage = Some(crate::input::CurrentUsage::default());
        assert!(cache_segment(&data, 0).is_none());
    }

    #[test]
    fn percentages_are_validated_without_capping_gateway_spend() {
        let mut data = sample();
        data.rate_limits.as_mut().unwrap().spend_limit = Some(crate::input::RateWindow {
            used_percentage: Some(135.0),
            resets_at: Some(10000),
        });
        assert!(plain(&first_row(&data, 160, false, 10000)).contains("$ 135% now"));
        data.context_window.used_percentage = Some(f64::NAN);
        data.prompt_cache.as_mut().unwrap().hit_ratio = Some(f64::NAN);
        data.rate_limits
            .as_mut()
            .unwrap()
            .five_hour
            .as_mut()
            .unwrap()
            .used_percentage = Some(f64::INFINITY);
        assert!(!first_row(&data, 160, false, 10000).contains("NaN"));
        assert!(cache_segment(&data, 10000).is_none());
        assert!(!plain(&first_row(&data, 160, false, 10000)).contains("5h"));
    }

    #[test]
    fn render_keeps_one_line_override_and_two_line_default() {
        let mut data = sample();
        data.session_id = "studio-height-test".into();
        assert_eq!(render(&data, 116, false, true).lines().count(), 1);
        assert_eq!(render(&data, 116, false, false).lines().count(), 2);
    }

    #[test]
    fn path_hyperlink_is_zero_width_balanced_and_does_not_cover_other_fields() {
        let data = sample();
        let url = "https://github.com/sj817/claude-code-plugin-hud";
        let open = format!("\x1b]8;;{url}\x07");
        let close = "\x1b]8;;\x07";
        for cols in 0..180 {
            for ascii in [false, true] {
                let row = second_row(&data, cols, ascii, 10000, None, Some(url));
                let unlinked = second_row(&data, cols, ascii, 10000, None, None);
                assert_eq!(plain(&row), plain(&unlinked));
                assert!(display_width(&plain(&row)) <= cols);
                assert_eq!(row.matches(&open).count(), row.matches(close).count());
                if let Some((_, linked)) = row.split_once(&open) {
                    let (linked, after) = linked.split_once(close).unwrap();
                    assert!(!plain(linked).contains(['│', '|']));
                    assert!(plain(&row).ends_with(&plain(linked)));
                    assert!(plain(after).is_empty());
                }
                if cols >= 76 {
                    assert!(row.contains(&open));
                }
            }
        }
    }
}
