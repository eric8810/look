//! markdown 渲染：minimad/termimad 排版 + code fence 拦截分流。
//!
//! 流程：
//!   1. 预扫描 raw markdown，识别围栏代码块（``` 或 ~~~）
//!   2. 将文档拆分为 prose 段落 + code block 段落
//!   3. prose → termimad FmtText(Display 产出 ANSI) → ansi-to-tui → Vec<Line>
//!   4. code block:
//!      - lang="mermaid" → mermansi 渲染图(ANSI) → ansi-to-tui
//!      - lang=其它       → syntect 高亮(ANSI)   → ansi-to-tui
//!      - 无 lang         → 纯文本

use std::fmt::Write;

use ratatui::text::Line;
use termimad::{FmtText, MadSkin};

use crate::ansi_lines;
use crate::highlight::Highlighter;
use crate::mermaid;

/// 将 markdown 源码渲染为 ratatui Vec<Line>。
pub fn markdown_to_lines(
    md: &str,
    width: u16,
    skin: &MadSkin,
    hl: &Highlighter,
) -> Vec<Line<'static>> {
    let segments = split_at_fences(md);
    let mut out: Vec<Line<'static>> = Vec::new();

    for seg in segments {
        match seg {
            Segment::Prose(text) => {
                let ansi = render_prose_to_ansi(&text, width, skin);
                out.extend(ansi_lines::to_lines(&ansi, width));
            }
            Segment::Code { lang, code } => {
                out.extend(render_code_fence(&code, lang.as_deref(), width, hl));
            }
        }
    }
    out
}

/// 一个文档段：prose 或 code block。
enum Segment {
    Prose(String),
    Code { lang: Option<String>, code: String },
}

/// 预扫描 markdown，按围栏代码块拆分。
fn split_at_fences(md: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut prose_buf = String::new();
    let mut lines = md.lines().peekable();

    while let Some(line) = lines.next() {
        if let Some((fence_char, info)) = parse_fence_open(line) {
            // Flush prose
            if !prose_buf.is_empty() {
                prose_buf.pop(); // 去掉末尾多余 \n
                segments.push(Segment::Prose(std::mem::take(&mut prose_buf)));
            }
            // Accumulate code until closing fence
            let mut code = String::new();
            let close_marker: String = (0..3).map(|_| fence_char).collect();
            for code_line in lines.by_ref() {
                if is_fence_close(code_line, &close_marker) {
                    break;
                }
                code.push_str(code_line);
                code.push('\n');
            }
            let lang = info.trim().split_whitespace().next().map(|s| s.to_string());
            segments.push(Segment::Code { lang, code });
        } else {
            prose_buf.push_str(line);
            prose_buf.push('\n');
        }
    }

    // Flush remaining prose
    if !prose_buf.is_empty() {
        prose_buf.pop();
        segments.push(Segment::Prose(prose_buf));
    }

    segments
}

/// 检测围栏开始行，返回 (围栏字符, info 字符串)。
/// 支持 ``` 和 ~~~（3+ 个相同字符），允许最多 3 个前导空格。
fn parse_fence_open(line: &str) -> Option<(char, String)> {
    let trimmed = line.trim_start();
    let leading_spaces = line.len() - trimmed.len();
    if leading_spaces > 3 {
        return None;
    }

    let first = trimmed.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }

    let fence_str: String = trimmed.chars().take_while(|&c| c == first).collect();
    if fence_str.len() < 3 {
        return None;
    }

    let info = trimmed[fence_str.len()..].to_string();
    // 反引号围栏的 info 中不能有反引号
    if first == '`' && info.contains('`') {
        return None;
    }

    Some((first, info))
}

/// 检测围栏结束行。
fn is_fence_close(line: &str, close_marker: &str) -> bool {
    let trimmed = line.trim_start();
    let leading_spaces = line.len() - trimmed.len();
    if leading_spaces > 3 {
        return false;
    }
    // 结束行只有围栏字符（可能有尾部空格）
    if !trimmed.starts_with(close_marker) {
        return false;
    }
    let rest = &trimmed[close_marker.len()..];
    // 允许更多围栏字符（如 ```` 关闭 ```）
    if !rest.chars().all(|c| c == close_marker.chars().next().unwrap()) {
        return rest.trim().is_empty();
    }
    true
}

/// 用 termimad 渲染 prose 段 → ANSI 字符串。
fn render_prose_to_ansi(prose: &str, width: u16, skin: &MadSkin) -> String {
    let fmt = FmtText::from(skin, prose, Some(width as usize));
    let mut buf = String::new();
    let _ = write!(buf, "{}", fmt);
    buf
}

/// 渲染围栏代码块 → Vec<Line>。
fn render_code_fence(
    code: &str,
    lang: Option<&str>,
    width: u16,
    hl: &Highlighter,
) -> Vec<Line<'static>> {
    match lang {
        Some("mermaid") => {
            match mermaid::render_mermaid_to_ansi(code, width) {
                Ok(ansi) => ansi_lines::to_lines_untruncated(&ansi),
                Err(_) => {
                    // 降级：当普通代码显示
                    let ansi = hl.highlight_to_ansi(code, None);
                    ansi_lines::to_lines(&ansi, width)
                }
            }
        }
        Some(lang) => {
            // 归一化代码块语言名 → syntect 可识别的 token
            let token = normalize_fence_lang(lang);
            let ansi = hl.highlight_to_ansi(code, token);
            ansi_lines::to_lines(&ansi, width)
        }
        None => {
            let ansi = hl.highlight_to_ansi(code, None);
            ansi_lines::to_lines(&ansi, width)
        }
    }
}

/// 将代码块的语言标识（可能带大小写/别名）归一化为 syntect 可识别的 token。
/// syntect 默认集无 TypeScript，回退到 JS；其它按原样返回（find_syntax 会按名/扩展名查）。
fn normalize_fence_lang(lang: &str) -> Option<&str> {
    let lower = lang.to_lowercase();
    let token = match lower.as_str() {
        "typescript" | "ts" | "tsx" | "mts" | "cts" => "js",
        "javascript" | "js" | "jsx" | "mjs" | "cjs" => "js",
        "python" | "py" | "pyi" => "py",
        "rust" | "rs" => "rs",
        "go" | "golang" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
        "csharp" | "cs" => "cs",
        "ruby" | "rb" => "rb",
        "php" => "php",
        "scala" => "scala",
        "bash" | "sh" | "shell" | "zsh" | "fish" => "sh",
        "json" | "jsonc" => "json",
        "yaml" | "yml" => "yaml",
        "html" | "htm" => "html",
        "xml" => "xml",
        "css" => "css",
        "sql" => "sql",
        "markdown" | "md" => "md",
        "lua" => "lua",
        "r" => "R",
        "perl" | "pl" | "pm" => "pl",
        "diff" | "patch" => "diff",
        "bat" | "batch" | "cmd" => "bat",
        // 默认集无语法（swift/kt/dart/toml/ini/vue/svelte/graphql/dockerfile/ps1/scss/less）→ None
        _ => return None,
    };
    Some(token)
}
