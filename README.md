# look

A minimal terminal file previewer — renders **markdown** with native layout and
view **code/text** with token-level syntax highlighting. Built with
[vue-tui](https://vue-tui.pages.dev/).

```
$ look README.md          # markdown: headings/lists/tables/code/links
$ look src/terminal.ts    # code: shiki truecolor syntax highlight
$ look plain.txt          # plain text: no highlight
```

## Install

### npm / npx (有 Node 环境即可，零依赖安装)

```bash
# 一次性运行（无需安装）：
npx preview README.md        # 或 npx look README.md

# 全局安装：
npm install -g look
preview README.md            # 或 look README.md
```

> 包体积仅 ~620KB（gzip），**零运行时依赖**（vue-tui + shiki 全部内联到单文件 bundle）。
> 要求 Node >= 18。
>
> **命令名冲突**：部分 Linux 自带 `look`（util-linux / bsdmainutils）。
> npm 包同时注册 `look` 和 `preview` 两个命令，若 `look` 被系统命令占用，用 `preview` 即可。

### SEA 单文件二进制（无需 Node 环境）

```bash
# 从源码构建（含 Node 运行时，~110MB）：
pnpm install
pnpm build
node scripts/build-binary.mjs
./look README.md
```

## 从源码开发

```bash
pnpm install
pnpm build                # → dist/terminal.cjs (single-file bundle)
node dist/terminal.cjs README.md
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

## Behavior

- **Markdown** (`.md`/`.markdown`): rendered by `TVirtualMarkdown` (headings,
  lists, tables, fenced code blocks, links, quotes).
- **Code / text**: rendered by a self-built `CodeView` with **shiki** token-level
  24-bit (truecolor) highlighting. Long lines are truncated (`less -S` style);
  tabs expand to 2 spaces.
- **Unknown extension**: shown as uncolored plain text.
- **Binary files** (NUL byte in first 8KB): refused with exit 1.
- **Non-TTY** (piped output): raw content is written to stdout, exit 0 — no TUI.

### Exit codes

| Situation | Code |
|---|---|
| Normal quit (`q`/`Esc`) | 0 |
| Bad arguments (none / too many) | 2 |
| File not found / unreadable / binary / directory | 1 |
| `Ctrl+C` | 130 |

## Project structure

```
src/
  terminal.ts     # entry: argv + binary detection + shiki init + runtime assembly
  PreviewApp.vue  # header + body(md/code) + footer; reactive resize
  CodeView.vue    # self-built highlighted code view (vue-tui composables)
  content.ts      # file reading + binary detection + mode detection
  highlight.ts    # shiki (@shikijs/core) init + tokenize
  lang.ts         # extension → shiki lang mapping
  args.ts         # argv parsing + --help/--version
scripts/
  build-binary.mjs  # Node SEA packaging
test/
  fixtures/         # sample.md, sample.ts, plain.txt, unknown.xyz, binary.bin, large.txt
  e2e/
    run_acceptance.py   # A–H acceptance suite (pty + pyte harness)
    lib/pty_harness.py  # pty + pyte terminal emulator (SU/SD + alt-screen patched)
    gen-large.sh        # generates large.txt (2000 lines)
```

## Testing

The E2E acceptance suite runs the binary in a real pseudo-terminal (pty) and
asserts on the emulated screen + raw ANSI output + exit codes. It covers
rendering, scrolling, exit/alt-screen, resize, error codes, non-TTY, large
files, and markdown elements (DESIGN.md §15, scenarios A–H).

```bash
# Against the SEA binary:
BIN=./look python3 test/e2e/run_acceptance.py

# Against the plain bundle:
BIN="node dist/terminal.cjs" python3 test/e2e/run_acceptance.py
```

Requires Python 3 with `pyte` (`pip install --user --break-system-packages pyte`).

## How it works

- **Runtime assembly** (`terminal.ts`): `createTerminalApp` →
  `createStdoutRenderer` (truecolor, alt-screen) → `createStdinDriver` (raw mode,
  `onExit`→130) → `installTerminalCleanup`.
- **Dual body components**: `TVirtualMarkdown` for `.md`; `CodeView` (self-built
  with `useTerminal`/`useTerminalNode`/`useRenderNode`/`useVisibility`) for code,
  replicating `TVirtualMarkdown`'s scroll/keyboard contract and painting shiki
  tokens via `terminal.write(text, {x, y, style:{fg:hex}})`.
- **Highlighting**: `@shikijs/core` + statically-imported grammars/themes (only
  the languages in `lang.ts` are bundled, keeping the SEA blob lean) + the JS
  regex engine — no dynamic imports, so it bundles into a single file.
- **Zero-dependency npm package**: all runtime deps (vue-tui, shiki, vue) are
  inlined into `dist/terminal.cjs` via `inlineDynamicImports`. The published npm
  package has `dependencies: {}` — `npm install -g look` or `npx look` pulls
  only the ~620KB tarball, no transitive deps.
- **Packaging**: Vite single-file CJS bundle → `node --experimental-sea-config`
  → `postject` injection into a copy of the Node binary (SEA, for Node-less
  environments).

See [DESIGN.md](DESIGN.md) for the full design and [DESIGN.md §19](DESIGN.md)
for implementation notes (deviations from the original plan + rationale).
