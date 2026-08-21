//! 统一转换枢纽：ANSI SGR 字符串 → ratatui Vec<Line>。
//!
//! mermansi（图）、syntect（代码高亮）、termimad（markdown 正文）均输出带 ANSI
//! truecolor 的字符串。ansi-to-tui 将其解析为 ratatui Text（含 Line/Span/Style）。

use ansi_to_tui::IntoText as _;
use ratatui::text::Line;

/// ANSI 字符串 → Vec<Line>（每行按 width 截断，less -S 风格）。
pub fn to_lines(ansi: &str, width: u16) -> Vec<Line<'static>> {
    let text = match ansi.as_bytes().into_text() {
        Ok(t) => t,
        Err(_) => return vec![Line::raw(ansy_trim(ansi).to_string())],
    };
    text.lines
        .into_iter()
        .map(|l| truncate_line(l, width))
        .collect()
}

/// ANSI 字符串 → Vec<Line>（不截断，用于 mermaid 图，已由 max_width 适配）。
pub fn to_lines_untruncated(ansi: &str) -> Vec<Line<'static>> {
    match ansi.as_bytes().into_text() {
        Ok(t) => t.lines,
        Err(_) => vec![Line::raw(ansy_trim(ansi).to_string())],
    }
}

/// 按 cell 宽度截断一行（less -S 风格）。
fn truncate_line(line: Line<'static>, width: u16) -> Line<'static> {
    if width == 0 {
        return line;
    }
    let w = width as usize;
    let mut cell_count = 0usize;
    let mut kept_spans = Vec::new();

    for span in line.spans {
        let span_width = unicode_width_cell(&span.content);
        if cell_count >= w {
            break;
        }
        let remaining = w - cell_count;
        if span_width <= remaining {
            kept_spans.push(span);
            cell_count += span_width;
        } else {
            // 截断此 span 到 remaining 宽度
            let (truncated, _) = slice_to_width(&span.content, remaining);
            if !truncated.is_empty() {
                kept_spans.push(ratatui::text::Span::styled(truncated, span.style));
            }
            cell_count = w;
            break;
        }
    }

    // 补空格到 width（确保整行覆盖旧行）
    let padding = w.saturating_sub(cell_count);
    if padding > 0 {
        kept_spans.push(ratatui::text::Span::raw(" ".repeat(padding)));
    }

    Line::styled("", line.style).spans(kept_spans)
}

/// 近似 cell 宽度（ASCII=1，其它按 1 近似；宽字符留待 unicode-width 优化）。
fn unicode_width_cell(s: &str) -> usize {
    s.chars().count()
}

/// 将字符串截断到 max_width cell 宽度，返回 (截断后, 实际宽度)。
fn slice_to_width(s: &str, max_width: usize) -> (String, usize) {
    let mut out = String::new();
    let mut w = 0usize;
    for ch in s.chars() {
        if w >= max_width {
            break;
        }
        out.push(ch);
        w += 1;
    }
    (out, w)
}

fn ansy_trim(s: &str) -> &str {
    s.trim_end_matches('\n')
}

