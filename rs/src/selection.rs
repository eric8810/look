//! 文本拖选与复制(DECISIONS D11,GAP G10)。
//!
//! 坐标模型:**内容坐标** = (doc.lines 行索引, 行内字符列)。
//! 屏幕坐标 → 内容坐标的换算在 termio.rs(视口 top + body 起始行列)。
//! 滚动期间选区基于内容坐标,保持稳定;resize/热重载会重建 lines,调用方应清除选区。
//!
//! 行为对齐 vue-tui selection:
//!   Down(左键) 定锚 → Drag 移动焦点(拖到视口边缘自动滚动)→ Up 自动复制(autoCopy);
//!   Shift+点击 扩展选区;`y`/Enter 手动复制;Esc 清除选择(无选择时才退出)。

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelPoint {
    /// doc.lines 行索引。
    pub line: usize,
    /// 行内字符列(0 起;与 ansi_lines 一致,1 字符 = 1 cell 近似)。
    pub col: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub anchor: SelPoint,
    pub focus: SelPoint,
}

impl Selection {
    pub fn new(p: SelPoint) -> Self {
        Selection { anchor: p, focus: p }
    }

    /// 规整为 (start, end),按 (line, col) 字典序。
    pub fn normalized(&self) -> (SelPoint, SelPoint) {
        if (self.focus.line, self.focus.col) >= (self.anchor.line, self.anchor.col) {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.focus
    }
}

/// 选区覆盖第 line_idx 行时的列范围,返回 (x0, x1):
/// x1 = None 表示选到行尾(含 ansi_lines 的补位空格)。
pub fn line_range(sel: &Selection, line_idx: usize) -> Option<(usize, Option<usize>)> {
    let (start, end) = sel.normalized();
    if line_idx < start.line || line_idx > end.line {
        return None;
    }
    let x0 = if line_idx == start.line { start.col } else { 0 };
    let x1 = if line_idx == end.line {
        Some(end.col)
    } else {
        None
    };
    Some((x0, x1))
}

/// 提取选区文本:每行截取后 trim_end(去掉排版补位空格),行间以 `\n` 连接。
pub fn text(lines: &[Line<'_>], sel: &Selection) -> String {
    let (start, end) = sel.normalized();
    let mut out: Vec<String> = Vec::new();
    for li in start.line..=end.line {
        let Some(line) = lines.get(li) else {
            break;
        };
        let s: String = line.spans.iter().map(|sp| sp.content.as_ref()).collect();
        let len = s.chars().count();
        let x0 = if li == start.line { start.col.min(len) } else { 0 };
        let x1 = if li == end.line { end.col.min(len) } else { len };
        let seg: String = s
            .chars()
            .skip(x0)
            .take(x1.saturating_sub(x0))
            .collect();
        out.push(seg.trim_end().to_string());
    }
    out.join("\n")
}

/// 对一行应用选区反显:列范围 [x0, x1)(x1 = None 到行尾)内的 span 片段加 REVERSED。
/// 覆盖的 span 会按列边界拆分为至多三段;返回新的 owned Line。
pub fn highlight_line(line: &Line<'_>, x0: usize, x1: Option<usize>) -> Line<'static> {
    let reversed = Style::default().add_modifier(Modifier::REVERSED);
    let x1v = x1.unwrap_or(usize::MAX);
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut col = 0usize;

    for span in &line.spans {
        let text: &str = span.content.as_ref();
        let n = text.chars().count();
        let span_end = col + n;

        if n == 0 || span_end <= x0 || col >= x1v {
            // 转 owned(content.to_string)以满足 'static 生命周期
            out.push(Span::styled(span.content.to_string(), span.style));
            col = span_end;
            continue;
        }

        // span 与选区相交:拆 前段(原样式) / 中段(反显) / 后段(原样式)
        let lead = x0.saturating_sub(col).min(n); // 前缀字符数
        let mid_end = x1v.saturating_sub(col).min(n); // 中段结束(相对 span 起点)
        let chars: Vec<char> = text.chars().collect();
        if lead > 0 {
            out.push(Span::styled(
                chars[..lead].iter().collect::<String>(),
                span.style,
            ));
        }
        if mid_end > lead {
            out.push(Span::styled(
                chars[lead..mid_end].iter().collect::<String>(),
                span.style.patch(reversed),
            ));
        }
        if mid_end < n {
            out.push(Span::styled(
                chars[mid_end..].iter().collect::<String>(),
                span.style,
            ));
        }
        col = span_end;
    }

    Line::default().spans(out).style(line.style)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::text::Span;

    fn pt(line: usize, col: usize) -> SelPoint {
        SelPoint { line, col }
    }

    fn mk_line(t: &str) -> Line<'static> {
        Line::default().spans(vec![Span::raw(t.to_string())])
    }

    #[test]
    fn normalize_and_empty() {
        let s = Selection {
            anchor: pt(3, 5),
            focus: pt(1, 2),
        };
        let (start, end) = s.normalized();
        assert_eq!((start.line, start.col), (1, 2));
        assert_eq!((end.line, end.col), (3, 5));
        assert!(!s.is_empty());
        assert!(Selection::new(pt(2, 2)).is_empty());
    }

    #[test]
    fn text_single_line() {
        let lines = vec![mk_line("hello world")];
        let sel = Selection {
            anchor: pt(0, 0),
            focus: pt(0, 5),
        };
        assert_eq!(text(&lines, &sel), "hello");
    }

    #[test]
    fn text_multiline_trims_padding() {
        let lines = vec![mk_line("abc   "), mk_line("def   "), mk_line("ghi   ")];
        let sel = Selection {
            anchor: pt(0, 1),
            focus: pt(2, 2),
        };
        // 行尾补位空格被 trim
        assert_eq!(text(&lines, &sel), "bc\ndef\ngh");
    }

    #[test]
    fn line_range_bounds() {
        let sel = Selection {
            anchor: pt(1, 4),
            focus: pt(3, 2),
        };
        assert_eq!(line_range(&sel, 0), None);
        assert_eq!(line_range(&sel, 1), Some((4, None)));
        assert_eq!(line_range(&sel, 2), Some((0, None)));
        assert_eq!(line_range(&sel, 3), Some((0, Some(2))));
        assert_eq!(line_range(&sel, 4), None);
        // 同一行
        let same = Selection {
            anchor: pt(2, 1),
            focus: pt(2, 5),
        };
        assert_eq!(line_range(&same, 2), Some((1, Some(5))));
    }

    #[test]
    fn highlight_splits_span() {
        let line = mk_line("abcdefgh");
        let out = highlight_line(&line, 2, Some(5));
        assert_eq!(out.spans.len(), 3);
        assert_eq!(out.spans[0].content, "ab");
        assert_eq!(out.spans[1].content, "cde");
        assert!(out.spans[1]
            .style
            .add_modifier
            .contains(Modifier::REVERSED));
        assert!(!out.spans[0]
            .style
            .add_modifier
            .contains(Modifier::REVERSED));
        assert_eq!(out.spans[2].content, "fgh");
    }

    #[test]
    fn highlight_to_end_of_line() {
        let line = mk_line("abcd");
        let out = highlight_line(&line, 2, None);
        assert_eq!(out.spans.len(), 2);
        assert_eq!(out.spans[0].content, "ab");
        assert_eq!(out.spans[1].content, "cd");
        assert!(out.spans[1]
            .style
            .add_modifier
            .contains(Modifier::REVERSED));
    }
}
