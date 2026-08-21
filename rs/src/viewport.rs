//! 自定义 ratatui widget：虚拟视口，渲染 lines[top .. top+h]。

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Widget;

pub struct Viewport<'a> {
    pub lines: &'a [Line<'a>],
    pub top: usize,
}

impl Widget for Viewport<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for y in 0..area.height {
            let li = self.top + y as usize;
            let row = area.y + y;
            match self.lines.get(li) {
                Some(line) => {
                    buf.set_line(area.x, row, line, area.width);
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
