//! crossterm 装配 + TUI 事件循环。
//!
//! 职责：raw mode / alt-screen / 鼠标捕获 / 事件读取 / 键位+滚轮映射 / resize /
//! 文件变更热重载（notify） / cleanup。

use std::io::{self, IsTerminal, Stdout};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind};
use crossterm::style::{Attribute, Colored};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::execute;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

use crate::ansi_lines;
use crate::content;
use crate::doc::Doc;
use crate::highlight::Highlighter;
use crate::lang::Mode;
use crate::markdown;
use crate::mermaid;
use crate::viewport::Viewport;

type Term = Terminal<CrosstermBackend<Stdout>>;

/// 左右边距（字符数）。
const H_MARGIN: u16 = 1;

const FOOTER: &str = "q quit  ↑↓/jk/scroll  space/pgdn  g/G top/bottom  Ctrl+C quit";

/// 判定 stdout 是否为 TTY。
pub fn is_tty() -> bool {
    io::stdout().is_terminal()
}

/// 构建 markdown skin：标题用粗体而非下划线。
fn build_skin() -> termimad::MadSkin {
    let mut skin = termimad::MadSkin::default();
    for h in &mut skin.headers {
        h.compound_style
            .object_style
            .attributes
            .unset(Attribute::Underlined);
        h.compound_style
            .object_style
            .attributes
            .set(Attribute::Bold);
    }
    skin
}

/// 运行 TUI 循环，返回退出码。
///
/// `file_path` 用于文件变更监听（热重载）；`mode`/`syntax_token` 由扩展名决定，
/// 文件类型不会因内容修改而变化，故在加载时一次性确定。
pub fn run(
    file_path: &str,
    mode: Mode,
    syntax_token: Option<&str>,
    initial_content: &str,
) -> i32 {
    // 强制启用 ANSI 颜色输出（忽略 NO_COLOR 环境变量）。
    Colored::set_ansi_color_disabled(false);

    let mut stdout = io::stdout();
    let _ = execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture);
    let _ = enable_raw_mode();

    let mut terminal = match Terminal::new(CrosstermBackend::new(stdout)) {
        Ok(t) => t,
        Err(_) => return 1,
    };

    let highlighter = Highlighter::new();
    let skin = build_skin();

    let (w, _h) = current_size(&terminal);
    let content_w = content_width(w);
    let lines = build_lines(initial_content, mode, syntax_token, content_w, &highlighter, &skin);
    let mut doc = Doc::new(lines, mode, content_w);

    // 文件变更监听：watcher 线程 → mpsc channel → 事件循环
    let (tx_file, rx_file) = channel::<()>();
    let mut watcher = match start_file_watcher(file_path, tx_file) {
        Some(w) => w,
        None => {
            // watcher 启动失败不致命：仍可正常预览，只是无热重载
            eprintln!("warning: file watch unavailable, live reload disabled");
            // 走无 watcher 的事件循环
            let exit_code = event_loop(&mut terminal, &mut doc, file_path, mode, syntax_token, None, &highlighter, &skin);
            let _ = disable_raw_mode();
            let _ = execute!(terminal.backend_mut(), crossterm::event::DisableMouseCapture, LeaveAlternateScreen);
            return exit_code;
        }
    };

    let exit_code = event_loop(
        &mut terminal,
        &mut doc,
        file_path,
        mode,
        syntax_token,
        Some(&rx_file),
        &highlighter,
        &skin,
    );

    // Cleanup
    let _ = watcher.unwatch(Path::new(file_path));
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        crossterm::event::DisableMouseCapture,
        LeaveAlternateScreen
    );

    exit_code
}

/// 启动文件监听线程。文件被修改时向 channel 发送信号。
fn start_file_watcher(
    file_path: &str,
    tx: std::sync::mpsc::Sender<()>,
) -> Option<RecommendedWatcher> {
    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                // 只关心修改/创建事件（忽略 Access 等噪声）
                if matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_)
                ) {
                    let _ = tx.send(());
                }
            }
        },
        notify::Config::default(),
    )
    .ok()?;

    // 监听文件本身（非递归）
    watcher
        .watch(Path::new(file_path), RecursiveMode::NonRecursive)
        .ok()?;

    Some(watcher)
}

fn current_size(terminal: &Term) -> (u16, u16) {
    terminal
        .size()
        .map(|r| (r.width, r.height))
        .unwrap_or((80, 24))
}

/// 内容区宽度 = 终端宽度 - 左右边距。
fn content_width(term_w: u16) -> u16 {
    term_w.saturating_sub(H_MARGIN * 2)
}

#[allow(clippy::too_many_arguments)]
fn event_loop(
    terminal: &mut Term,
    doc: &mut Doc,
    file_path: &str,
    mode: Mode,
    syntax_token: Option<&str>,
    file_rx: Option<&std::sync::mpsc::Receiver<()>>,
    hl: &Highlighter,
    skin: &termimad::MadSkin,
) -> i32 {
    // 当前文件内容（用于 resize / 文件变更时重排）
    let mut content: String = content::reload_content(file_path).unwrap_or_default();

    loop {
        let _ = terminal.draw(|f| render_frame(f, doc, file_path));

        // 用 poll 非阻塞检查终端事件，间隔检查文件变更 channel
        if event::poll(Duration::from_millis(200)).unwrap_or(false) {
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
                Ok(Event::Mouse(m)) => {
                    let body_h = body_height(terminal);
                    match m.kind {
                        MouseEventKind::ScrollUp => doc.scroll(-1, body_h),
                        MouseEventKind::ScrollDown => doc.scroll(1, body_h),
                        _ => {}
                    }
                }
                Ok(Event::Resize(w, h)) => {
                    let body_h = (h as usize).saturating_sub(2);
                    let cw = content_width(w);
                    let new_lines =
                        build_lines(&content, mode, syntax_token, cw, hl, skin);
                    doc.replace_lines(new_lines, cw, body_h);
                }
                Ok(_) => {}
                Err(_) => return 1,
            }
        }

        // 检查文件变更 channel
        if let Some(rx) = file_rx {
            if rx.try_recv().is_ok() {
                // 文件被修改：重新读取并重排
                if let Some(new_content) = content::reload_content(file_path) {
                    content = new_content;
                    let (w, _h) = current_size(terminal);
                    let cw = content_width(w);
                    let body_h = body_height(terminal);
                    let new_lines = build_lines(&content, mode, syntax_token, cw, hl, skin);
                    doc.replace_lines(new_lines, cw, body_h);
                }
            }
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

    // Header (bold, 左边距 1 字符)
    let header = Paragraph::new(format!(" {}", file_name))
        .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(header, chunks[0]);

    // Body — 左右各留 H_MARGIN 字符边距
    let body_area = Rect::new(
        chunks[1].x + H_MARGIN,
        chunks[1].y,
        chunks[1].width.saturating_sub(H_MARGIN * 2),
        chunks[1].height,
    );
    let viewport = Viewport {
        lines: &doc.lines,
        top: doc.top,
    };
    f.render_widget(viewport, body_area);

    // Footer (dim, 左边距 1 字符)
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        format!(" {}", FOOTER),
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
