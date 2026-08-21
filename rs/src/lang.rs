//! 扩展名 → 语言 ID / 模式映射。
//! .md/.markdown → Markdown 模式；.mmd/.mermaid → Mermaid 模式；其它 → Code 模式。
//!
//! 注：syntect 默认语法集（default-fancy）不含 TypeScript/Vue/Svelte/GraphQL/
//! Docker/PowerShell/Dart/Swift/Kotlin/Toml/INI/SCSS/Less。这些扩展名回退到最接近的
//! 可用语法（如 ts→js），或无色纯文本。保持 find_syntax_by_extension 能命中。

/// 预览模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Markdown,
    Code,
    Mermaid,
}

/// 扩展名 → syntect 可识别的文件扩展名 token（用于 find_syntax_by_extension）。
/// 只映射 syntect 默认语法集中存在的扩展名；未知返回 None（→ 无色纯文本）。
fn ext_to_syntax_token(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "ts" | "tsx" | "mts" | "cts" => "js", // TS 回退到 JS（默认集无 TS）
        "js" | "jsx" | "mjs" | "cjs" => "js",
        "py" | "pyi" => "py",
        "rs" => "rs",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
        "cs" => "cs",
        "rb" => "rb",
        "php" => "php",
        "scala" => "scala",
        "sh" | "bash" | "zsh" | "fish" => "sh",
        "json" | "jsonc" => "json",
        "yaml" | "yml" => "yaml",
        "html" | "htm" => "html",
        "xml" => "xml",
        "css" => "css",
        "sql" => "sql",
        "md" | "markdown" => "md",
        "lua" => "lua",
        "r" => "R",
        "pl" | "pm" => "pl",
        "diff" | "patch" => "diff",
        "bat" | "cmd" => "bat",
        // 以下默认集无语法：返回 None → 无色纯文本
        // swift, kt, dart, toml, ini, vue, svelte, graphql, dockerfile, ps1, scss, less
        _ => return None,
    })
}

/// 特殊文件名（无扩展名但有意义的语言）→ syntect 扩展名 token。
fn name_to_syntax_token(name: &str) -> Option<&'static str> {
    Some(match name {
        "makefile" | "justfile" => "makefile",
        "gemfile" | "rakefile" => "rb",
        // dockerfile 默认集无语法 → None
        _ => return None,
    })
}

/// 返回文件名对应的 syntect 扩展名 token（用于代码高亮查找）。
/// 仅 Code 模式有意义。
pub fn detect_syntax_token(file_name: &str) -> Option<&'static str> {
    let base = file_name.rsplit('/').next().unwrap_or(file_name);
    let lower = base.to_lowercase();
    if let Some(t) = name_to_syntax_token(&lower) {
        return Some(t);
    }
    match lower.rfind('.') {
        Some(dot) => ext_to_syntax_token(&lower[dot + 1..]),
        None => None,
    }
}

/// 是否为 markdown 扩展名（走 termimad 渲染）。
pub fn is_markdown_ext(file_name: &str) -> bool {
    let base = file_name.rsplit('/').next().unwrap_or(file_name);
    let lower = base.to_lowercase();
    match lower.rfind('.') {
        Some(dot) => {
            let ext = &lower[dot + 1..];
            ext == "md" || ext == "markdown"
        }
        None => false,
    }
}

/// 是否为 mermaid 扩展名。
pub fn is_mermaid_ext(file_name: &str) -> bool {
    let base = file_name.rsplit('/').next().unwrap_or(file_name);
    let lower = base.to_lowercase();
    match lower.rfind('.') {
        Some(dot) => {
            let ext = &lower[dot + 1..];
            ext == "mmd" || ext == "mermaid"
        }
        None => false,
    }
}

/// 判定预览模式 + 语法 token。
pub fn detect_mode_lang(file_name: &str) -> (Mode, Option<&'static str>) {
    if is_markdown_ext(file_name) {
        return (Mode::Markdown, None);
    }
    if is_mermaid_ext(file_name) {
        return (Mode::Mermaid, None);
    }
    (Mode::Code, detect_syntax_token(file_name))
}
