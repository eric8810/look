#!/usr/bin/env bash
# install.sh — 安装 dlook (Rust 终端文件预览器) 的最新 release 二进制
#
# 用法:
#   curl -fsSL https://raw.githubusercontent.com/eric8810/dlook/main/scripts/install.sh | bash
#
# 可用环境变量(注意要作用于管道右侧的 bash,如 `curl ... | VERSION=... bash`):
#   VERSION   指定版本 (默认: latest), 例如 VERSION=v0.2.0
#   INSTALL_DIR 安装目录 (默认: ~/.local/bin)

set -euo pipefail

OWNER="eric8810"
REPO="dlook"
BIN_NAME="dlook"

# --- 平台检测 ---
detect_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)  os="unknown-linux-gnu" ;;
    Darwin) os="apple-darwin" ;;
    *) err "不支持的操作系统: $os (仅支持 Linux / macOS)" ;;
  esac

  case "$arch" in
    x86_64|amd64)  arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *) err "不支持的架构: $arch (仅支持 x86_64 / aarch64)" ;;
  esac

  # macOS aarch64 与 x86_64 均已提供预编译包
  echo "${arch}-${os}"
}

err()   { printf '\033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }
info()  { printf '\033[36m==>\033[0m %s\n' "$*"; }

need() { command -v "$1" >/dev/null 2>&1 || err "缺少依赖: $1"; }

# 全局：供 EXIT trap 清理（trap 在 main 返回后触发，不能用 local）。
tmpdir=""

main() {
  need curl
  need tar

  local target
  target="$(detect_platform)"

  local version="${VERSION:-latest}"
  local url
  if [ "$version" = "latest" ]; then
    url="https://github.com/${OWNER}/${REPO}/releases/latest/download/dlook-${target}.tar.gz"
  else
    # VERSION 形如 v0.1.0
    url="https://github.com/${OWNER}/${REPO}/releases/download/${version}/dlook-${target}.tar.gz"
  fi

  local install_dir="${INSTALL_DIR:-${HOME}/.local/bin}"
  mkdir -p "$install_dir"

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  info "目标平台: $target"
  info "下载: $url"

  local archive="$tmpdir/look.tar.gz"
  curl -fsSL "$url" -o "$archive"
  info "解压..."
  tar -xzf "$archive" -C "$tmpdir"

  # 包内是单个名为 dlook 的二进制
  if [ ! -f "$tmpdir/$BIN_NAME" ]; then
    err "压缩包内未找到二进制 $BIN_NAME"
  fi

  install -m 0755 "$tmpdir/$BIN_NAME" "$install_dir/$BIN_NAME"

  printf '\n'
  info "已安装到: $install_dir/$BIN_NAME"

  case ":$PATH:" in
    *":$install_dir:"*) ;;
    *)
      printf '\033[33m注意:\033[0m %s 不在 PATH 中。请添加:\n' "$install_dir"
      printf '  export PATH="%s:$PATH"\n' "$install_dir"
      ;;
  esac

  printf '\n运行 \033[32mdlook README.md\033[0m 开始使用。\n'
}

main "$@"
