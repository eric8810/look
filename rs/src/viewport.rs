//! 自定义 ratatui widget：虚拟视口，渲染 lines[top .. top+h]。
//!
//! 有选区时(DECISIONS D11),选区覆盖的行按列范围加 REVERSED 反显。

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Widget;

use crate::selection;

pub struct Viewport<'a> {
    pub lines: &'a [Line<'a>],
    pub top: usize,
    /// 文本选区(内容坐标);None = 无选区。
    pub selection: Option<selection::Selection>,
}

impl Widget for Viewport<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for y in 0..area.height {
            let li = self.top + y as usize;
            let row = area.y + y;
            match self.lines.get(li) {
                Some(line) => {
                    // 选区覆盖此行 → 反显高亮后渲染
                    let highlighted = self
                        .selection
                        .and_then(|sel| selection::line_range(&sel, li))
                        .map(|(x0, x1)| selection::highlight_line(line, x0, x1));
                    match highlighted {
                        Some(hl) => {
                            buf.set_line(area.x, row, &hl, area.width);
                        }
                        None => {
                            buf.set_line(area.x, row, line, area.width);
                        }
                    }
                }
                None => {
                    // 超出文档末尾：填空格
                    let spaces = " ".repeat(area.width as usize);
                    buf.set_string(area.x, row, &spaces, Style::default());
                }
            }
        }
    }
}
