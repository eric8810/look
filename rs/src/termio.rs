//! crossterm 装配 + TUI 事件循环。
//!
//! 职责：raw mode / alt-screen / 事件读取 / 键位映射 / resize / cleanup。

use std::io::{self, IsTerminal, Stdout};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

use crate::ansi_lines;
use crate::doc::Doc;
use crate::highlight::Highlighter;
use crate::lang::Mode;
use crate::markdown;
use crate::mermaid;
use crate::viewport::Viewport;

type Term = Terminal<CrosstermBackend<Stdout>>;

const FOOTER: &str = " q quit  ↑↓/jk scroll  space/pgdn  g/G top/bottom  Ctrl+C quit";

/// 判定 stdout 是否为 TTY。
pub fn is_tty() -> bool {
    io::stdout().is_terminal()
}

/// 运行 TUI 循环，返回退出码。
pub fn run(
    content: &str,
    mode: Mode,
    syntax_token: Option<&str>,
    file_name: &str,
) -> i32 {
    // 强制启用 ANSI 颜色输出（忽略 NO_COLOR 环境变量）。
    // 我们是 TUI 应用，明确需要 truecolor 输出；NO_COLOR 会令 crossterm 抑制所有颜色。
    crossterm::style::Colored::set_ansi_color_disabled(false);

    let mut stdout = io::stdout();
    let _ = execute!(stdout, EnterAlternateScreen);
    let _ = enable_raw_mode();

    let mut terminal = match Terminal::new(CrosstermBackend::new(stdout)) {
        Ok(t) => t,
        Err(_) => return 1,
    };

    let highlighter = Highlighter::new();
    let skin = termimad::MadSkin::default();

    let (w, _h) = current_size(&terminal);
    let lines = build_lines(content, mode, syntax_token, w, &highlighter, &skin);
    let mut doc = Doc::new(lines, mode, w);

    let exit_code = event_loop(
        &mut terminal,
        &mut doc,
        content,
        mode,
        syntax_token,
        file_name,
        &highlighter,
        &skin,
    );

    // Cleanup
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);

    exit_code
}

fn current_size(terminal: &Term) -> (u16, u16) {
    terminal
        .size()
        .map(|r| (r.width, r.height))
        .unwrap_or((80, 24))
}

#[allow(clippy::too_many_arguments)]
fn event_loop(
    terminal: &mut Term,
    doc: &mut Doc,
    content: &str,
    mode: Mode,
    syntax_token: Option<&str>,
    file_name: &str,
    hl: &Highlighter,
    skin: &termimad::MadSkin,
) -> i32 {
    loop {
        let _ = terminal.draw(|f| render_frame(f, doc, file_name));

        match event::read() {
            Ok(Event::Key(k)) => {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                match map_key(k) {
                    Action::Quit(code) => return code,
                    Action::Scroll(delta) => {
                        let body_h = body_height(terminal);
                        doc.scroll(delta, body_h);
                    }
                    Action::Page(delta) => {
                        let body_h = body_height(terminal);
                        let page = body_h as isize;
                        doc.scroll(delta * page, body_h);
                    }
                    Action::Top => doc.set_top(0, body_height(terminal)),
                    Action::Bottom => doc.set_top(usize::MAX, body_height(terminal)),
                    Action::None => {}
                }
            }
            Ok(Event::Resize(w, h)) => {
                let body_h = (h as usize).saturating_sub(2);
                let new_lines = build_lines(content, mode, syntax_token, w, hl, skin);
                doc.replace_lines(new_lines, w, body_h);
            }
            Ok(_) => {}
            Err(_) => return 1,
        }
    }
}

fn body_height(terminal: &Term) -> usize {
    terminal
        .size()
        .map(|r| r.height as usize)
        .unwrap_or(24)
        .saturating_sub(2)
}

enum Action {
    Quit(i32),
    Scroll(isize),
    Page(isize),
    Top,
    Bottom,
    None,
}

fn map_key(k: KeyEvent) -> Action {
    // Ctrl+C → 退出码 130
    if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Quit(130);
    }
    if k.code == KeyCode::Char('\u{0003}') {
        return Action::Quit(130);
    }

    match k.code {
        KeyCode::Char('q') => Action::Quit(0),
        KeyCode::Esc => Action::Quit(0),
        KeyCode::Char('j') => Action::Scroll(1),
        KeyCode::Down => Action::Scroll(1),
        KeyCode::Char('k') => Action::Scroll(-1),
        KeyCode::Up => Action::Scroll(-1),
        KeyCode::Char(' ') => Action::Page(1),
        KeyCode::PageDown => Action::Page(1),
        KeyCode::PageUp => Action::Page(-1),
        KeyCode::Home => Action::Top,
        KeyCode::Char('g') => Action::Top,
        KeyCode::End => Action::Bottom,
        KeyCode::Char('G') => Action::Bottom,
        _ => Action::None,
    }
}

fn render_frame(f: &mut ratatui::Frame, doc: &Doc, file_name: &str) {
    let area = f.area();
    let chunks = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Min(0),    // body
        Constraint::Length(1), // footer
    ])
    .split(area);

    // Header (bold)
    let header = Paragraph::new(format!(" {}", file_name))
        .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(header, chunks[0]);

    // Body (virtual viewport)
    let viewport = Viewport {
        lines: &doc.lines,
        top: doc.top,
    };
    f.render_widget(viewport, chunks[1]);

    // Footer (dim)
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        FOOTER,
        Style::default().add_modifier(Modifier::DIM),
    )]))
    .alignment(Alignment::Left);
    f.render_widget(footer, chunks[2]);
}

/// 根据 mode + content 生成 Vec<Line>。
fn build_lines(
    content: &str,
    mode: Mode,
    syntax_token: Option<&str>,
    width: u16,
    hl: &Highlighter,
    skin: &termimad::MadSkin,
) -> Vec<Line<'static>> {
    match mode {
        Mode::Markdown => markdown::markdown_to_lines(content, width, skin, hl),
        Mode::Code => {
            let ansi = hl.highlight_to_ansi(content, syntax_token);
            ansi_lines::to_lines(&ansi, width)
        }
        Mode::Mermaid => match mermaid::render_mermaid_to_ansi(content, width) {
            Ok(ansi) => ansi_lines::to_lines_untruncated(&ansi),
            Err(_) => ansi_lines::to_lines(content, width),
        },
    }
}
