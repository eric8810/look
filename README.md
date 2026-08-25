# look

A minimal terminal file previewer — renders **markdown** with native layout,
**mermaid** diagrams as ASCII art, and **code/text** with token-level syntax
highlighting. Written in Rust (ratatui + crossterm + syntect).

<img width="909" height="683" alt="image" src="https://github.com/user-attachments/assets/d2132127-3282-4403-89cb-0a981735dde6" />

```
$ look README.md          # markdown: headings/lists/tables/code/links
$ look src/main.rs        # code: syntect truecolor syntax highlight
$ look flow.mmd           # mermaid: rendered to ASCII art
$ look plain.txt          # plain text: no highlight
```

## Install

### 一行命令安装（预编译二进制，无需 Rust 环境）

```bash
curl -fsSL https://raw.githubusercontent.com/eric8810/look/main/scripts/install.sh | bash
```

自动检测平台（Linux/macOS, x86_64/aarch64），从 [GitHub Release](https://github.com/eric8810/look/releases)
下载对应的预编译二进制并安装到 `~/.local/bin`。可用环境变量自定义：

```bash
VERSION=v0.1.0 INSTALL_DIR=/usr/local/bin \
  curl -fsSL https://raw.githubusercontent.com/eric8810/look/main/scripts/install.sh | sudo bash
```

Windows 用户请直接到 [Releases](https://github.com/eric8810/look/releases) 下载
`look-x86_64-pc-windows-msvc.zip` 解压使用。

### 从源码构建

```bash
cd rs
cargo build --release
./target/release/look README.md
```

## 从源码开发

```bash
cd rs
cargo run -- README.md
```

## Keys

| Key | Action |
|---|---|
| `q` / `Esc` | Quit (exit 0) |
| `j` / `k` | Scroll down / up one line |
| `Space` / `PageDown` | Scroll down one page |
| `PageUp` | Scroll up one page |
| `g` / `G` | Go to top / bottom |
| `↑` `↓` | Scroll one line |
| `Home` / `End` | Go to top / bottom |
| `Ctrl+C` | Quit (exit 130) |
| Mouse wheel | Scroll up / down |

## Behavior

- **Markdown** (`.md`/`.markdown`): rendered by `termimad` (headings, lists,
  tables, fenced code blocks, links, quotes).
- **Mermaid** (`.mmd`/`.mermaid`): rendered to ASCII art via `mermansi`.
- **Code / text**: rendered with **syntect** token-level 24-bit (truecolor)
  highlighting. Long lines are truncated (`less -S` style).
- **Unknown extension**: shown as uncolored plain text.
- **Binary files** (NUL byte in first 8KB): refused with exit 1.
- **Non-TTY** (piped output): raw content is written to stdout, exit 0 — no TUI.
  (Mermaid files are rendered to ASCII art first.)
- **Live reload**: watches the file with `notify` and re-renders on change.

### Exit codes

| Situation | Code |
|---|---|
| Normal quit (`q`/`Esc`) | 0 |
| Bad arguments (none / too many) | 2 |
| File not found / unreadable / binary / directory | 1 |
| `Ctrl+C` | 130 |

## Project structure

```
rs/
  Cargo.toml
  src/
    main.rs       # entry: argv + binary detection + mode dispatch + TUI assembly
    args.rs       # argv parsing + --help/--version
    content.rs    # file reading + binary detection + mode detection + hot reload
    lang.rs       # extension → mode + syntect syntax token mapping
    highlight.rs  # syntect highlighter init + tokenize
    markdown.rs   # termimad markdown rendering
    mermaid.rs    # mermaid → ASCII art rendering
    doc.rs        # Doc.lines builder (md/code/mermaid → styled lines)
    viewport.rs   # scroll viewport
    termio.rs     # crossterm setup + event loop + keys/wheel + resize + live reload
    ansi_lines.rs # ANSI escape handling
test/
  fixtures/         # sample.md, sample.ts, plain.txt, unknown.xyz, binary.bin, large.txt
  e2e/
    run_acceptance.py   # A–H acceptance suite (pty + pyte harness)
    lib/pty_harness.py  # pty + pyte terminal emulator
    gen-large.sh        # generates large.txt (2000 lines)
.github/workflows/
  release.yml       # tag-triggered multi-platform release build
scripts/
  install.sh        # curl|sh installer
```

## Testing

The E2E acceptance suite runs the binary in a real pseudo-terminal (pty) and
asserts on the emulated screen + raw ANSI output + exit codes. It covers
rendering, scrolling, exit/alt-screen, resize, error codes, non-TTY, large
files, and markdown elements (scenarios A–H).

```bash
cd rs && cargo build --release
BIN=../rs/target/release/look python3 test/e2e/run_acceptance.py
```

Requires Python 3 with `pyte` (`pip install --user --break-system-packages pyte`).

## How it works

- **Mode dispatch** (`main.rs`): `parse_args` → `load_content` (binary check +
  mode detection) → non-TTY passthrough → `termio::run` TUI loop.
- **Markdown** (`markdown.rs`): `termimad` parses CommonMark and produces
  styled spans; headers rendered bold (no underline) with side margins.
- **Code** (`highlight.rs` + `doc.rs`): `syntect` with the default-fancy syntax
  set tokenizes the file into truecolor spans; unknown extensions fall back to
  plain text. TS/Vue/Svelte/etc. (absent from the default set) fall back to the
  nearest available syntax (e.g. `ts`→`js`).
- **Mermaid** (`mermaid.rs`): `mermansi` renders the diagram to ASCII art lines.
- **Terminal** (`termio.rs`): crossterm raw mode + alt-screen + mouse capture;
  ratatui `Paragraph` paints the viewport; `notify` watcher triggers a re-read
  and re-render on file change (live reload).
- **Packaging**: `cargo build --release` with `opt-level="z"`, fat LTO,
  `codegen-units=1`, `strip=true`, `panic="abort"` for a small static binary.
  CI builds per-target release binaries and attaches them to the GitHub Release.

See [DESIGN-rust.md](DESIGN-rust.md) for the full design.
