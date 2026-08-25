//! mermaid 图表渲染（mermansi）。
//!
//! mermansi 是纯 Rust mermaid 终端渲染器，支持 28 种图类型，
//! 输出带 ANSI truecolor 的 ASCII/Unicode 文本。
//! max_width 适配终端宽度，图不溢出。

use mermansi::{render_source, ColorMode, MermansiOptions, OutputMode};

/// 盒绘字符集合，用于识别图的最后一行（截掉 mermansi 追加的边列表）。
/// 去掉 mermansi 在图后追加的边列表（如 "A --> B"），只保留盒绘图。
/// 边列表行含 ASCII "-->"（图本身用 Unicode 盒绘字符 ─▶，不会误伤）。
fn strip_edge_legend(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    // 从末尾往前找：去掉所有"边列表行"（含 --> / --- / ==> 等ASCII边标记）
    // 以及它们前面的空行
    let mut cut = lines.len();
    let mut saw_edge = false;
    for i in (0..lines.len()).rev() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            // 空行：如果在边列表区域，也去掉
            if saw_edge {
                cut = i;
                continue;
            } else {
                break;
            }
        }
        if is_edge_line(trimmed) {
            saw_edge = true;
            cut = i;
        } else if saw_edge {
            // 遇到非边列表行，停止
            break;
        }
    }
    if cut < lines.len() {
        lines[..cut].join("\n") + "\n"
    } else {
        text.to_string()
    }
}

/// 判断一行是否为 mermansi 追加的边列表行（ASCII 边标记）。
fn is_edge_line(s: &str) -> bool {
    s.contains("-->") || s.contains("---") || s.contains("==>") || s.contains("-.->")
}

/// mermansi 的 canvas cell 上限（max_width × max_height ≤ 此值）。
const MAX_CANVAS_CELLS: usize = 250_000;

/// 根据宽度动态计算 max_height，确保 canvas cells 不超限。
/// 宽终端时自动降低高度上限；窄终端时仍允许较高的图（上限 2000）。
fn safe_max_height(width: usize) -> usize {
    (MAX_CANVAS_CELLS / width.max(1)).min(2000)
}

/// 渲染 mermaid 源码 → 带 truecolor ANSI 的终端文本（TUI 模式用）。
/// 解析失败时返回 Err（调用方降级为单色源码显示）。
pub fn render_mermaid_to_ansi(src: &str, width: u16) -> Result<String, String> {
    let max_w = width as usize;
    let opts = MermansiOptions::unicode()
        .with_color(ColorMode::TrueColor) // truecolor ANSI
        .with_output_mode(OutputMode::Concise) // 只输出预览，不含语义 JSON
        .with_max_width(max_w)
        .with_max_height(safe_max_height(max_w));

    render_source(src, &opts)
        .map(|text| strip_edge_legend(&text))
        .map_err(|e| e.to_string())
}

/// 渲染 mermaid 源码 → 纯文本（无 ANSI 颜色），用于非 TTY 管道输出。
/// 解析失败时返回 Err（调用方降级为原始源码）。
pub fn render_mermaid_to_plain(src: &str) -> Result<String, String> {
    let max_w = 120;
    let opts = MermansiOptions::unicode()
        .with_color(ColorMode::Plain) // 无颜色，纯文本
        .with_output_mode(OutputMode::Concise)
        .with_max_width(max_w)
        .with_max_height(safe_max_height(max_w));

    render_source(src, &opts)
        .map(|text| strip_edge_legend(&text))
        .map_err(|e| e.to_string())
}

