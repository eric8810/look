#!/usr/bin/env python3
"""gen-mascot-banner.py — 卡通吉祥物版 GitHub banner(1280×640,社交预览标准比例)。

构成:
  - docs/mascot-raw.png:gpt-image-2 生成的卡通猫(拿放大镜看文档,无文字)
  - SVG 矢量排版:wordmark / tagline / 特性 chips / 安装命令(文字必须矢量,避免 AI 写错字)
  - 背景色从吉祥物图片采样,边缘羽化,无缝融合
  - headless chrome 导出 @2x PNG

用法:
    python3 scripts/gen-mascot-banner.py           # 产出 docs/banner-mascot.svg/.png
    MASCOT=别的猫.png python3 scripts/gen-mascot-banner.py   # 换吉祥物重新生成
"""
import base64
import html
import os
import re
import subprocess
import sys

from PIL import Image, ImageFilter

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
DOCS = os.path.join(ROOT, "docs")
MASCOT = os.environ.get("MASCOT", os.path.join(DOCS, "mascot-raw.png"))

W, H = 1280, 640

MONO = "SF Mono, Cascadia Code, JetBrains Mono, Consolas, Menlo, DejaVu Sans Mono, monospace"
SANS = "system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', sans-serif"
CHAR_W = 0.6

# ---------------------------------------------------------------------------
# 吉祥物预处理:采样底色 + 边缘羽化(压平为纯 RGB,避免与 SVG 底色出现接缝)
# ---------------------------------------------------------------------------

def process_mascot(src, out, feather=48):
    img = Image.open(src).convert("RGB")
    w, h = img.size
    bg = img.getpixel((8, 8))

    # 羽化 mask:边缘向内 feather 像素渐隐
    mask = Image.new("L", (w, h), 0)
    from PIL import ImageDraw
    d = ImageDraw.Draw(mask)
    d.rectangle([feather, feather, w - feather, h - feather], fill=255)
    mask = mask.filter(ImageFilter.GaussianBlur(feather // 2))

    flat = Image.new("RGB", (w, h), bg)
    flat.paste(img, (0, 0), mask)
    flat.save(out)
    return bg, flat.size


# ---------------------------------------------------------------------------
# SVG 构件
# ---------------------------------------------------------------------------

def chip(cx, cy, text, fs=13):
    w = len(text) * fs * 0.62 + 28
    return (w,
            f'<rect x="{cx}" y="{cy}" width="{w:.0f}" height="28" rx="14" fill="#141a23" '
            f'stroke="#2b3340"/>'
            f'<text x="{cx+w/2:.0f}" y="{cy+19}" text-anchor="middle" '
            f'font-family="{SANS}" font-size="{fs}" fill="#c9d3e0">{html.escape(text)}</text>')


def build_svg(bg_hex, mascot_path, mascot_size):
    parts = []
    parts.append(f'<rect width="{W}" height="{H}" fill="{bg_hex}"/>')
    parts.append(
        f'<rect width="{W}" height="{H}" fill="url(#glow)"/>')

    # ---- 吉祥物(左侧)----
    mw, mh = mascot_size
    target_h = 430
    scale = target_h / mh
    tw, th = round(mw * scale), target_h
    mx, my = 8, (H - th) // 2 - 12
    with open(mascot_path, "rb") as f:
        b64 = base64.b64encode(f.read()).decode()
    parts.append(
        f'<image x="{mx}" y="{my}" width="{tw}" height="{th}" '
        f'href="data:image/png;base64,{b64}"/>')

    # ---- 右侧文字列 ----
    tx = 560

    # wordmark
    parts.append(
        f'<text x="{tx}" y="185" font-family="{SANS}" font-size="84" font-weight="800" '
        f'fill="#e6edf3"><tspan fill="#00ffff">d</tspan>look</text>')
    # tagline
    parts.append(
        f'<text x="{tx+3}" y="228" font-family="{SANS}" font-size="19" fill="#93a1b5">'
        f'Markdown · Code · Mermaid — beautifully rendered in your terminal</text>')

    # chips 2×2
    row1 = ["truecolor markdown", "40+ languages"]
    row2 = ["mermaid → ASCII", "select & copy (OSC 52)"]
    for ri, row in enumerate((row1, row2)):
        cx = tx
        cy = 262 + ri * 44
        for label in row:
            w, svg = chip(cx, cy, label)
            parts.append(svg)
            cx += w + 12

    # 安装命令 pill
    cmd = "curl -fsSL https://raw.githubusercontent.com/eric8810/look/main/scripts/install.sh | bash"
    cfs = 13
    pw = len(cmd) * cfs * CHAR_W + 36
    py = 402
    parts.append(
        f'<rect x="{tx}" y="{py}" width="{pw:.0f}" height="42" rx="21" fill="#10151d" '
        f'stroke="#33415a"/>')
    parts.append(
        f'<text x="{tx+18}" y="{py+27}" font-family="{MONO}" font-size="{cfs}" fill="#7ee2a8">'
        f'{html.escape(cmd)}</text>')

    # 版本/平台注脚
    parts.append(
        f'<text x="{tx+3}" y="{py+72}" font-family="{SANS}" font-size="13" fill="#5d6b80">'
        f'v0.2.1 · ~6.6 MB static binary · Linux · macOS · Windows · no runtime</text>')

    svg = (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" '
        f'viewBox="0 0 {W} {H}">\n'
        f'<defs><radialGradient id="glow" cx="0.18" cy="0.15" r="0.75">'
        f'<stop offset="0" stop-color="#123a44" stop-opacity="0.30"/>'
        f'<stop offset="0.55" stop-color="{bg_hex}" stop-opacity="0"/>'
        f'</radialGradient></defs>\n'
        + "\n".join(parts) + "\n</svg>\n")
    return svg


def svg_to_png(svg_path, png_path, width, height, scale=2):
    subprocess.run([
        "google-chrome", "--headless=new", "--disable-gpu", "--hide-scrollbars",
        f"--force-device-scale-factor={scale}",
        f"--window-size={width},{height}",
        f"--screenshot={png_path}", "file://" + svg_path,
    ], check=True, capture_output=True, timeout=120)


def main():
    if not os.path.exists(MASCOT):
        sys.exit(f"mascot not found: {MASCOT}")
    os.makedirs(DOCS, exist_ok=True)

    processed = os.path.join(DOCS, "mascot-flat.png")
    bg, size = process_mascot(MASCOT, processed)
    bg_hex = "#%02x%02x%02x" % bg
    print(f"mascot: {MASCOT} {size} bg={bg_hex}")

    svg = build_svg(bg_hex, processed, size)
    svg_path = os.path.join(DOCS, "banner-mascot.svg")
    with open(svg_path, "w") as f:
        f.write(svg)
    print("wrote", svg_path)

    png_path = os.path.join(DOCS, "banner-mascot.png")
    svg_to_png(svg_path, png_path, W, H)
    print("wrote", png_path)


if __name__ == "__main__":
    main()
