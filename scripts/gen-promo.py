#!/usr/bin/env python3
"""gen-promo.py — 从真实 dlook 渲染输出生成宣传图。

原理:在 pty 里跑 dlook,捕获原始 ANSI 流,解析 SGR/CUP 还原出
「每个字符 + 颜色/样式」的屏幕网格,再渲染成 SVG(终端窗口/横幅)。
图里的内容因此是产品的真实渲染效果(颜色、布局、选中反显均为实拍)。

用法:
    python3 scripts/gen-promo.py            # 生成 docs/*.svg
    python3 scripts/gen-promo.py --png      # 同时用 headless chrome 导出 PNG(@2x)

依赖:Python3 + pyte(test/e2e/lib 的 pty harness);--png 需要 google-chrome。
"""
import argparse
import html
import os
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, os.path.join(ROOT, "test", "e2e", "lib"))
from pty_harness import PtySession  # noqa: E402

BIN = os.environ.get("BIN", os.path.join(ROOT, "rs", "target", "release", "dlook"))

# ---------------------------------------------------------------------------
# xterm-256 色板
# ---------------------------------------------------------------------------

def _palette():
    base = [
        "#000000", "#800000", "#008000", "#808000", "#000080", "#800080",
        "#008080", "#c0c0c0", "#808080", "#ff0000", "#00ff00", "#ffff00",
        "#5f5fff", "#ff00ff", "#00ffff", "#ffffff",
    ]
    steps = [0, 95, 135, 175, 215, 255]
    cube = ["#%02x%02x%02x" % (r, g, b) for r in steps for g in steps for b in steps]
    gray = ["#%02x%02x%02x" % (v, v, v) for v in range(8, 248, 10)]
    return base + cube + gray


PALETTE = _palette()

# 终端默认前景(暗底终端的常规字色)
DEFAULT_FG = "#c0c5ce"
DEFAULT_BG = None  # 透明 → 用窗口底色

MONO = "SF Mono, Cascadia Code, JetBrains Mono, Consolas, Menlo, DejaVu Sans Mono, monospace"
SANS = "system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', sans-serif"

CHAR_W = 0.6  # 等宽字体每字符 advance(em)


# ---------------------------------------------------------------------------
# ANSI 解析:数据流 → 屏幕网格(每 cell = (char, style))
# ---------------------------------------------------------------------------

class Screen:
    def __init__(self, rows, cols):
        self.rows, self.cols = rows, cols
        self.cells = [[None] * cols for _ in range(rows)]
        self.fg = None          # None | ('i', idx) | ('rgb', r, g, b)
        self.bg = None
        self.bold = self.dim = self.rev = self.ul = False
        self.r = self.c = 0

    def style(self):
        return (self.fg, self.bg, self.bold, self.dim, self.rev, self.ul)

    def put(self, ch):
        if 0 <= self.r < self.rows and 0 <= self.c < self.cols:
            self.cells[self.r][self.c] = (ch, self.style())
        self.c += 1

    def clear_line(self, mode):
        if not (0 <= self.r < self.rows):
            return
        if mode == 0:
            rng = range(self.c, self.cols)
        elif mode == 1:
            rng = range(0, min(self.c + 1, self.cols))
        else:
            rng = range(0, self.cols)
        for x in rng:
            self.cells[self.r][x] = None

    def clear_screen(self, mode):
        if mode == 0:
            rows = range(self.r, self.rows)
        elif mode == 1:
            rows = range(0, self.r + 1)
        else:
            rows = range(0, self.rows)
        for y in rows:
            for x in range(self.cols):
                self.cells[y][x] = None


def _sgr(scr, params):
    ps = []
    for p in params.split(";"):
        ps.append(int(p) if p.isdigit() else 0)
    if not ps:
        ps = [0]
    i = 0
    while i < len(ps):
        p = ps[i]
        if p == 0:
            scr.fg = scr.bg = None
            scr.bold = scr.dim = scr.rev = scr.ul = False
        elif p == 1:
            scr.bold = True
        elif p == 2:
            scr.dim = True
        elif p == 4:
            scr.ul = True
        elif p == 7:
            scr.rev = True
        elif p == 22:
            scr.bold = scr.dim = False
        elif p == 24:
            scr.ul = False
        elif p == 27:
            scr.rev = False
        elif p == 39:
            scr.fg = None
        elif p == 49:
            scr.bg = None
        elif 30 <= p <= 37:
            scr.fg = ("i", p - 30)
        elif 40 <= p <= 47:
            scr.bg = ("i", p - 40)
        elif 90 <= p <= 97:
            scr.fg = ("i", p - 90 + 8)
        elif 100 <= p <= 107:
            scr.bg = ("i", p - 100 + 8)
        elif p in (38, 48) and i + 1 < len(ps):
            kind = ps[i + 1]
            if kind == 5 and i + 2 < len(ps):
                color = ("i", ps[i + 2])
                i += 2
            elif kind == 2 and i + 4 < len(ps):
                color = ("rgb", ps[i + 2], ps[i + 3], ps[i + 4])
                i += 4
            else:
                color = None
            if p == 38:
                scr.fg = color
            else:
                scr.bg = color
        i += 1


def _csi(scr, params, final):
    priv = params.startswith("?")
    body = params[1:] if priv else params
    args = [int(p) if p.isdigit() else 0 for p in body.split(";")] if body else []

    if final in ("H", "f"):  # CUP
        row = args[0] if len(args) > 0 and args[0] else 1
        col = args[1] if len(args) > 1 and args[1] else 1
        scr.r, scr.c = row - 1, col - 1
    elif final == "A":
        scr.r -= max(1, args[0] if args else 1)
    elif final == "B":
        scr.r += max(1, args[0] if args else 1)
    elif final == "C":
        scr.c += max(1, args[0] if args else 1)
    elif final == "D":
        scr.c -= max(1, args[0] if args else 1)
    elif final == "G":  # CHA
        scr.c = (args[0] if args and args[0] else 1) - 1
    elif final == "d":  # VPA
        scr.r = (args[0] if args and args[0] else 1) - 1
    elif final == "J":
        scr.clear_screen(args[0] if args else 0)
    elif final == "K":
        scr.clear_line(args[0] if args else 0)
    elif final == "m" and not priv:
        _sgr(scr, body)


def parse_ansi(data, rows, cols):
    scr = Screen(rows, cols)
    i, n = 0, len(data)
    while i < n:
        ch = data[i]
        if ch == "\x1b":
            if i + 1 < n and data[i + 1] == "[":
                j = i + 2
                while j < n and not ("@" <= data[j] <= "~"):
                    j += 1
                if j >= n:
                    break
                _csi(scr, data[i + 2:j], data[j])
                i = j + 1
                continue
            if i + 1 < n and data[i + 1] == "]":  # OSC
                j = i + 2
                while j < n:
                    if data[j] == "\x07":
                        j += 1
                        break
                    if data[j] == "\x1b" and j + 1 < n and data[j + 1] == "\\":
                        j += 2
                        break
                    j += 1
                i = j
                continue
            i += 2
            continue
        if ch == "\r":
            scr.c = 0
        elif ch == "\n":
            scr.r += 1
        elif ch >= " ":
            scr.put(ch)
        i += 1
    return scr


def resolve(color):
    if color is None:
        return None
    if color[0] == "i":
        idx = max(0, min(255, color[1]))
        return PALETTE[idx]
    _, r, g, b = color
    return "#%02x%02x%02x" % (r, g, b)


# ---------------------------------------------------------------------------
# 捕获:跑 dlook → (可选拖选)→ raw ANSI
# ---------------------------------------------------------------------------

def capture(path, cols, rows, wait, select=None):
    s = PtySession([BIN, path], cols=cols, rows=rows)
    s.start()
    if not s.wait_for(wait, 8):
        raise RuntimeError(f"timeout waiting for {wait!r} in {path}")
    s.feed(0.6)
    if select:
        (x0, y0, x1, y1) = select
        s.send("\x1b[<0;%d;%dM" % (x0, y0)); s.feed(0.15)
        s.send("\x1b[<32;%d;%dM" % (x1, y1)); s.feed(0.15)
        s.send("\x1b[<0;%d;%dm" % (x1 + 1, y1)); s.feed(0.35)
    raw = s.raw_text()
    s.send_key("q")
    s.wait_exit(3)
    s.close()
    return raw


# ---------------------------------------------------------------------------
# SVG 渲染
# ---------------------------------------------------------------------------

def grid_runs(scr):
    """每行 → [(style, text), ...](相邻同 style 的 cell 合并;保留词间空格,
    仅修剪行首/行尾无反显的空白 run)。"""
    out = []
    for y in range(scr.rows):
        runs = []  # [ [style, text], ... ]
        for x in range(scr.cols):
            cell = scr.cells[y][x]
            if cell is None:
                st, ch = None, " "
            else:
                ch, st = cell
            if runs and runs[-1][0] == st:
                runs[-1][1] += ch
            else:
                runs.append([st, ch])
        # 修剪行首纯空白 run(反显选中除外)
        if runs and runs[0][1].strip() == "" and not (runs[0][0] and runs[0][0][4]):
            runs.pop(0)
        # 修剪行尾空白(反显除外)
        if runs:
            last = runs[-1]
            trimmed = last[1].rstrip(" ")
            if trimmed != last[1] and not (last[0] and last[0][4]):
                if trimmed:
                    last[1] = trimmed
                else:
                    runs.pop()
        out.append([(st, t) for st, t in runs])
    return out


def style_fg_bg(st):
    fg = resolve(st[0]) or DEFAULT_FG
    bg = resolve(st[1]) if st[1] is not None else None
    if st[4]:  # reversed:交换前景/背景(默认背景下即浅底深字)
        bg = bg or DEFAULT_FG
        fg = DEFAULT_BG or "#12161d"
    return fg, bg


def svg_terminal(scr, title, x, y, fs=15, win_bg="#0f131a", border="#2b3340",
                 bar_h=34, pad_x=16, pad_top=10):
    """渲染一个终端窗口,返回 (svg 片段, 宽, 高)。"""
    lh = round(fs * 1.38)
    cw = fs * CHAR_W
    cols = scr.cols
    win_w = round(cols * cw + pad_x * 2)
    win_h = bar_h + pad_top + scr.rows * lh + 12

    parts = []
    # 窗体
    parts.append(
        f'<rect x="{x}" y="{y}" width="{win_w}" height="{win_h}" rx="10" '
        f'fill="{win_bg}" stroke="{border}" stroke-width="1.5"/>')
    # 标题栏
    parts.append(
        f'<path d="M {x} {y+10} a 10 10 0 0 1 10 -10 h {win_w-20} a 10 10 0 0 1 10 10 '
        f'v {bar_h-10} h -{win_w} z" fill="#171c25"/>')
    for i, c in enumerate(("#ff5f57", "#febc2e", "#28c840")):
        parts.append(f'<circle cx="{x+18+i*20}" cy="{y+bar_h/2}" r="5.5" fill="{c}"/>')
    parts.append(
        f'<text x="{x+win_w/2}" y="{y+bar_h/2+5}" text-anchor="middle" '
        f'font-family="{SANS}" font-size="12" fill="#8a95a6">{html.escape(title)}</text>')

    # 内容:先画反显背景块,再画文本
    rev_rects = []
    texts = []
    ty0 = y + bar_h + pad_top
    for ry, runs in enumerate(grid_runs(scr)):
        if not runs:
            continue
        cx = 0.0
        ty = ty0 + ry * lh + fs
        tspans = []
        for st, text in runs:
            if not text:
                continue
            fg, bg = style_fg_bg(st if st else (None, None, False, False, False, False))
            attrs = f'fill="{fg}"'
            if st and st[2]:
                attrs += ' font-weight="bold"'
            if st and st[3]:
                attrs += ' fill-opacity="0.7"'
            if st and st[5]:
                attrs += ' text-decoration="underline"'
            if bg:
                rev_rects.append(
                    f'<rect x="{x+pad_x+cx:.1f}" y="{ty0+ry*lh+1}" '
                    f'width="{len(text)*cw:.1f}" height="{lh-2}" rx="2" fill="{bg}"/>')
            tspans.append(f'<tspan {attrs}>{html.escape(text)}</tspan>')
            cx += len(text) * cw
        texts.append(
            f'<text x="{x+pad_x}" y="{ty}" font-family="{MONO}" font-size="{fs}">{"".join(tspans)}</text>')

    return ("\n  ".join(parts + rev_rects + texts), win_w, win_h)


def chip(cx, cy, text, fs=14):
    w = len(text) * fs * 0.62 + 30
    return (w,
            f'<rect x="{cx}" y="{cy}" width="{w:.0f}" height="30" rx="15" fill="#141a23" '
            f'stroke="#2b3340"/>'
            f'<text x="{cx+w/2:.0f}" y="{cy+20}" text-anchor="middle" '
            f'font-family="{SANS}" font-size="{fs}" fill="#c9d3e0">{html.escape(text)}</text>')


def gen_banner(md_scr, path_out, width=1280, height=940):
    """横幅:标题 + 特性 chips + 终端窗口 + 安装命令。"""
    parts = []
    parts.append(f'<rect width="{width}" height="{height}" fill="#0b0e14"/>')
    parts.append(f'<rect width="{width}" height="{height}" fill="url(#glow)"/>')

    # 左上:wordmark + tagline
    parts.append(
        f'<text x="64" y="104" font-family="{SANS}" font-size="60" font-weight="800" fill="#e6edf3">'
        f'<tspan fill="#00ffff">d</tspan>look</text>')
    parts.append(
        f'<text x="67" y="142" font-family="{SANS}" font-size="20" fill="#93a1b5">'
        f'Markdown · Code · Mermaid — beautifully rendered in your terminal</text>')

    # chips:tagline 下一行,从左 64 排开
    labels = [
        "truecolor markdown",
        "40+ languages",
        "mermaid → ASCII",
        "select & copy (OSC 52)",
        "live reload",
        "6.7 MB static",
    ]
    cx, cy = 64, 166
    for label in labels:
        w, svg = chip(cx, cy, label, 13)
        parts.append(svg)
        cx += w + 12

    # 终端窗口(居中)
    term, tw, th = svg_terminal(md_scr, "dlook — docs/demo.md", 0, 0, fs=14)
    tx = (width - tw) // 2
    ty = 222
    parts.append(f'<g transform="translate({tx},{ty})">{term}</g>')

    # 安装命令 pill
    cmd = "curl -fsSL https://raw.githubusercontent.com/eric8810/dlook/main/scripts/install.sh | bash"
    cfs = 15
    cw = cfs * CHAR_W
    pw = len(cmd) * cw + 44
    px = (width - pw) // 2
    py = height - 96
    parts.append(
        f'<rect x="{px}" y="{py}" width="{pw:.0f}" height="44" rx="22" fill="#10151d" '
        f'stroke="#33415a"/>')
    parts.append(
        f'<text x="{px+22}" y="{py+29}" font-family="{MONO}" font-size="{cfs}" fill="#7ee2a8">'
        f'{html.escape(cmd)}</text>')
    parts.append(
        f'<text x="{width/2}" y="{py+70}" text-anchor="middle" font-family="{SANS}" '
        f'font-size="13" fill="#5d6b80">Linux · macOS · Windows — single static binary, no runtime</text>')

    svg = (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}" font-family="{SANS}">'
        f'<defs><radialGradient id="glow" cx="0.22" cy="0.10" r="0.9">'
        f'<stop offset="0" stop-color="#123a44" stop-opacity="0.35"/>'
        f'<stop offset="0.5" stop-color="#0b0e14" stop-opacity="0"/>'
        f'</radialGradient></defs>\n'
        + "\n".join(parts) + "\n</svg>\n")
    with open(path_out, "w") as f:
        f.write(svg)


def gen_plain_window(scr, title, path_out, fs=16, margin=24, bg="#0b0e14"):
    # 裁掉尾部全空行(mermaid 图不满一屏时避免窗口下方留白)
    while scr.rows > 1 and all(c is None for c in scr.cells[-1]):
        scr.cells.pop()
        scr.rows -= 1
    term, tw, th = svg_terminal(scr, title, 0, 0, fs=fs)
    w = tw + margin * 2
    h = th + margin * 2
    svg = (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" '
        f'viewBox="0 0 {w} {h}">\n'
        f'<rect width="{w}" height="{h}" fill="{bg}" rx="0"/>\n'
        f'<g transform="translate({margin},{margin})">{term}</g>\n</svg>\n')
    with open(path_out, "w") as f:
        f.write(svg)


def svg_to_png(svg_path, png_path, width, height, scale=2):
    subprocess.run([
        "google-chrome", "--headless=new", "--disable-gpu", "--hide-scrollbars",
        f"--force-device-scale-factor={scale}",
        f"--window-size={width},{height}",
        f"--screenshot={png_path}", "file://" + svg_path,
    ], check=True, capture_output=True, timeout=120)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--png", action="store_true", help="同时导出 PNG(headless chrome @2x)")
    args = ap.parse_args()

    if not os.path.exists(BIN):
        sys.exit(f"binary not found: {BIN} (cd rs && cargo build --release)")

    docs = os.path.join(ROOT, "docs")
    os.makedirs(docs, exist_ok=True)

    # 1) markdown demo:banner 带拖选(展示反显 + copied 状态栏)
    #    两段式:先干净捕获找到代码行位置,再带拖选重捕(选中 `let doc = ...`)
    raw = capture(os.path.join(docs, "demo.md"), 88, 27, "dlook")
    probe = parse_ansi(raw, 27, 88)
    sel_row = None
    for y, runs in enumerate(grid_runs(probe)):
        text = "".join(t for _, t in runs)
        if "let doc" in text:
            sel_row = y + 1  # 1-based 屏幕行
            break
    if sel_row is None:
        raise RuntimeError("demo.md: cannot locate 'let doc' line for selection")
    raw = capture(os.path.join(docs, "demo.md"), 88, 27, "dlook",
                  select=(10, sel_row, 46, sel_row))
    md_scr = parse_ansi(raw, 27, 88)

    # 2) mermaid demo
    raw = capture(os.path.join(docs, "demo.mmd"), 88, 18, "markdown")
    mm_scr = parse_ansi(raw, 18, 88)

    banner = os.path.join(docs, "banner.svg")
    gen_banner(md_scr, banner)
    print("wrote", banner)

    shot = os.path.join(docs, "screenshot.svg")
    gen_plain_window(probe, "dlook — docs/demo.md", shot, fs=16)
    print("wrote", shot)

    mshot = os.path.join(docs, "mermaid.svg")
    gen_plain_window(mm_scr, "dlook — docs/demo.mmd", mshot, fs=16)
    print("wrote", mshot)

    if args.png:
        for name, w, h in (("banner", 1280, 720), ("screenshot", 1120, 640),
                           ("mermaid", 1120, 480)):
            svg = os.path.join(docs, name + ".svg")
            if not os.path.exists(svg):
                continue
            # 实际尺寸以 svg 头为准
            head = open(svg).read(200)
            import re
            m = re.search(r'width="(\d+)" height="(\d+)"', head)
            if m:
                w, h = int(m.group(1)), int(m.group(2))
            png = os.path.join(docs, name + ".png")
            svg_to_png(svg, png, w, h)
            print("wrote", png)


if __name__ == "__main__":
    main()
