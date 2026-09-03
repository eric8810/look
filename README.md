# look

A minimal terminal file previewer — renders **markdown** with native layout,
**mermaid** diagrams as ASCII art, and **code/text** with token-level syntax
highlighting. Written in Rust (ratatui + crossterm + syntect).

<img width="909" height="683" alt="image" src="https://github.com/user-attachments/assets/d2132127-3282-4403-89cb-0a981735dde6" />

```
$ dlook README.md          # markdown: headings/lists/tables/code/links
$ dlook src/main.rs        # code: syntect truecolor syntax highlight
$ dlook flow.mmd           # mermaid: rendered to ASCII art
$ dlook plain.txt          # plain text: no highlight
```

## Install

### 一行命令安装（预编译二进制，无需 Rust 环境）

```bash
curl -fsSL https://raw.githubusercontent.com/eric8810/look/main/scripts/install.sh | bash
```

自动检测平台（Linux/macOS, x86_64/aarch64），从 [GitHub Release](https://github.com/eric8810/look/releases)
下载对应的预编译二进制并安装到 `~/.local/bin`。可用环境变量自定义：

```bash
VERSION=v0.2.0 INSTALL_DIR=/usr/local/bin \
  curl -fsSL https://raw.githubusercontent.com/eric8810/look/main/scripts/install.sh | sudo bash
```

Windows 用户请直接到 [Releases](https://github.com/eric8810/look/releases) 下载
`dlook-x86_64-pc-windows-msvc.zip` 解压使用。

### 从源码构建

```bash
cd rs
cargo build --release
./target/release/dlook README.md
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
| Mouse drag | Select text (reversed highlight); auto-scrolls at viewport edges |
| Release mouse | Copy selection to clipboard via **OSC 52** (works over SSH) |
| `Shift` + click | Extend selection |
| `y` / `Enter` | Copy active selection (OSC 52) |
| `Esc` | Clear selection — quits only when no selection is active |

> Tip: your terminal's **native** selection still works with `Shift` + drag
> (the app enables mouse reporting, so plain drag is captured by the app).

## Behavior

- **Markdown** (`.md`/`.markdown`): rendered by `termimad` — headings
  (**h1/h2 cyan, h3/h4 blue**, bold), nested lists, task lists (`- [x]` →
  ☑ / ☐), tables (**rounded borders** + per-column alignment), quotes,
  fenced code blocks, strikethrough. Inline links `[label](url)` render as
  a blue underlined label + a gray URL.
- **Mermaid** (`.mmd`/`.mermaid`): rendered to ASCII art via `mermansi`
  (28 diagram types, truecolor). Mermaid code blocks inside markdown are
  rendered too.
- **Code / text**: rendered with **syntect** token-level 24-bit (truecolor)
  highlighting, using the **two-face** full Sublime syntax set (TypeScript,
  Vue, Svelte, TOML, INI, GraphQL, Dockerfile, PowerShell, SCSS, Less,
  Swift, Kotlin, Dart, …). Long lines are truncated (`less -S` style).
- **Unknown extension**: shown as uncolored plain text.
- **Text selection & copy**: mouse drag selects (reversed highlight,
  edge auto-scroll, Shift+click extends); releasing the mouse copies via
  OSC 52 (`y`/`Enter` copies manually). Unsupported terminals degrade
  silently with a status message.
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
    highlight.rs  # syntect (two-face) highlighter init + tokenize
    markdown.rs   # termimad rendering + task checkboxes + link styling + table frame
    mermaid.rs    # mermaid → ASCII art rendering
    selection.rs  # text selection model (content coords, highlight, copy text)
    doc.rs        # Doc.lines builder (md/code/mermaid → styled lines)
    viewport.rs   # scroll viewport + selection highlight
    termio.rs     # crossterm setup + event loop + keys/wheel/mouse-drag + resize + live reload
    ansi_lines.rs # ANSI escape handling
test/
  fixtures/         # sample.md, sample.ts, plain.txt, unknown.xyz, binary.bin, large.txt,
                    # style.md (J), lang.toml / lang.vue (L)
  e2e/
    run_acceptance.py   # A–L acceptance suite (pty + pyte harness)
    run-tmux.sh         # T1–T29 real-terminal suite (tmux + OSC 52 clipboard)
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
files, markdown elements, markdown styling, mouse selection + OSC 52 copy,
and language coverage (scenarios A–L, 96 checks). Unit tests cover link
parsing, task checkboxes, table framing, and the selection model
(`cd rs && cargo test`).

```bash
cd rs && cargo build --release
BIN=../rs/target/release/dlook python3 test/e2e/run_acceptance.py
```

Requires Python 3 with `pyte` (`pip install --user --break-system-packages pyte`).

### Real-terminal verification (tmux)

A second harness runs the same binary inside a real terminal — a dedicated
tmux server (`-f /dev/null`, own socket, `set-clipboard on`). This verifies
what an emulator cannot: **the actual clipboard content** delivered by
OSC 52 (tmux captures it into a paste buffer), mouse-drag input injection,
alt-screen behavior, truecolor passthrough, resize, and exit codes.
Scenarios cover markdown styling (T1–T10), language coverage (T11–T12),
mermaid (T13), exit codes (T14–T16), drag-select + OSC 52 copy (T17–T25),
edge auto-scroll (T26–T27), and resize (T28–T29).

```bash
BIN=rs/target/release/dlook bash test/e2e/run-tmux.sh
```

Requires tmux ≥ 3.x.

## How it works

- **Mode dispatch** (`main.rs`): `parse_args` → `load_content` (binary check +
  mode detection) → non-TTY passthrough → `termio::run` TUI loop.
- **Markdown** (`markdown.rs`): `termimad` parses CommonMark and produces
  styled spans. The skin sets bold colored headings (h1/h2 cyan, h3/h4 blue),
  left-aligned H1, and rounded table borders. Post-processing adds task-list
  checkboxes (☑/☐), inline link styling (blue underlined label + gray URL),
  and a rounded top/bottom frame around tables.
- **Code** (`highlight.rs` + `doc.rs`): `syntect` with the **two-face** full
  syntax set tokenizes the file into truecolor spans; unknown extensions fall
  back to plain text. Missing syntaxes fall back to the nearest one
  (e.g. `vue`→`html`).
- **Mermaid** (`mermaid.rs`): `mermansi` renders the diagram to ASCII art lines.
- **Selection** (`selection.rs` + `viewport.rs` + `termio.rs`): mouse drag
  builds a selection in **content coordinates** (stable across scrolling);
  the viewport renders selected spans reversed; releasing the mouse copies
  the text via OSC 52 (crossterm `osc52` feature). Dragging at the viewport
  edge auto-scrolls; resize/hot-reload clears the selection.
- **Terminal** (`termio.rs`): crossterm raw mode + alt-screen + mouse capture;
  ratatui `Paragraph` paints the viewport; `notify` watcher triggers a re-read
  and re-render on file change (live reload).
- **Packaging**: `cargo build --release` with `opt-level="z"`, fat LTO,
  `codegen-units=1`, `strip=true`, `panic="abort"` for a small static binary.
  CI builds per-target release binaries and attaches them to the GitHub Release.

See [DESIGN-rust.md](DESIGN-rust.md) for the full design,
[GAP.md](GAP.md) + [DECISIONS.md](DECISIONS.md) for the vue-tui capability
comparison and the decisions behind these features.
