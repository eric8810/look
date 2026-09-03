//! syntect 语法高亮集成。
//!
//! 语法集用 two-face(默认集 + 全量 Sublime 扩展语法,DECISIONS D4),
//! 覆盖 TypeScript/Vue/Svelte/TOML/INI/GraphQL/Dockerfile/PowerShell 等;
//! 缺失语言按回退链兜底(如 tsx→js、vue→html)。
//! 输出带 truecolor ANSI SGR 的字符串,经 ansi_lines 转为 ratatui Line。

use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::as_24_bit_terminal_escaped;

pub struct Highlighter {
    ss: SyntaxSet,
    ts: ThemeSet,
    theme_name: String,
}

impl Highlighter {
    pub fn new() -> Self {
        // two-face:默认语法集 + 扩展语法(纯 Rust fancy-regex 引擎,无 C 依赖)
        let ss = two_face::syntax::extra_newlines();
        let ts = ThemeSet::load_defaults();
        // 使用暗色主题；E2E 只查 truecolor 存在性，不查具体色值
        let theme_name = "base16-ocean.dark".to_string();
        Self {
            ss,
            ts,
            theme_name,
        }
    }

    /// 高亮代码 → 带 truecolor ANSI 的字符串。
    /// lang_token 是 syntect 扩展名 token（如 "js"、"rs"）或代码块语言名（如 "python"），
    /// None → 无色纯文本。
    pub fn highlight_to_ansi(&self, code: &str, lang_token: Option<&str>) -> String {
        let syntax = lang_token.and_then(|t| self.find_syntax(t));

        match syntax {
            Some(syntax) => {
                let theme = match self.ts.themes.get(&self.theme_name) {
                    Some(t) => t,
                    None => return code.to_string(),
                };
                let mut h = HighlightLines::new(syntax, theme);
                let mut out = String::new();
                for line in code.lines() {
                    // tab 展开为 2 空格（与 TS 版 expandTabs 一致）
                    let expanded = expand_tabs(line, 2);
                    match h.highlight_line(&expanded, &self.ss) {
                        Ok(regions) => {
                            // bg=false → 不输出背景色，只输出前景 truecolor
                            out.push_str(&as_24_bit_terminal_escaped(&regions, false));
                        }
                        Err(_) => out.push_str(&expanded),
                    }
                    out.push('\n');
                }
                out
            }
            None => {
                // 未知语言：无色纯文本（tab 展开）
                let mut out = String::new();
                for line in code.lines() {
                    out.push_str(&expand_tabs(line, 2));
                    out.push('\n');
                }
                out
            }
        }
    }

    /// 查找语法：先按扩展名，再按 token（名称/扩展名），最后按回退链兜底。
    fn find_syntax(&self, token: &str) -> Option<&SyntaxReference> {
        // 先按扩展名查
        if let Some(s) = self.ss.find_syntax_by_extension(token) {
            return Some(s);
        }
        // 再按 token 查（支持 "python"/"rust"/"kotlin" 等语言名）
        if let Some(s) = self.ss.find_syntax_by_token(token) {
            return Some(s);
        }
        // 回退链：语法集中缺失的语言用最接近的可用语法
        let fallbacks: &[&str] = match token {
            "ts" | "tsx" | "typescript" | "mts" | "cts" => &["js"],
            "jsx" => &["js"],
            "vue" | "svelte" => &["html"],
            "kotlin" | "kt" | "kts" => &["java"],
            _ => &[],
        };
        fallbacks
            .iter()
            .find_map(|t| self.ss.find_syntax_by_extension(t))
    }
}

/// 将 Tab 展开为空格（tabstop 指定）。
fn expand_tabs(s: &str, tabstop: usize) -> String {
    if !s.contains('\t') {
        return s.to_string();
    }
    let mut out = String::new();
    let mut col = 0usize;
    for ch in s.chars() {
        if ch == '\t' {
            let n = tabstop - (col % tabstop);
            for _ in 0..n {
                out.push(' ');
            }
            col += n;
        } else {
            out.push(ch);
            col += 1;
        }
    }
    out
}
