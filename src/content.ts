/**
 * 文件读取 + 二进制检测 + 模式判定。
 *
 * - 文件不存在 / 不可读 → exit 1
 * - 目录 → exit 1
 * - 二进制（前 8KB 含 \0）→ exit 1
 * - .md/.markdown → mode "markdown"
 * - 其它 → mode "code"（含纯文本与未知扩展名）
 */
import { readFileSync, statSync } from "node:fs";
import { isMarkdownExt, detectLang } from "./lang";

export type PreviewMode = "markdown" | "code";

export interface LoadedContent {
  /** 文件绝对/原始路径（用于 header 显示）。 */
  fileName: string;
  /** 原始文本内容。 */
  content: string;
  /** 渲染模式。 */
  mode: PreviewMode;
  /** shiki 语言 ID（仅 code 模式有意义；markdown 为 null）。 */
  lang: string | null;
}

const BINARY_SAMPLE = 8192;

function fail(msg: string): never {
  process.stderr.write(msg + "\n");
  process.exit(1);
}

export function loadContent(filePath: string): LoadedContent {
  let st;
  try {
    st = statSync(filePath);
  } catch {
    fail(`error: cannot access '${filePath}': no such file or directory`);
  }

  if (st.isDirectory()) {
    fail(`error: '${filePath}' is a directory`);
  }

  let buf: Buffer;
  try {
    buf = readFileSync(filePath);
  } catch {
    fail(`error: cannot read '${filePath}': permission denied`);
  }

  // 二进制检测：前 8KB 含 NUL 字节视为二进制
  const sample = buf.subarray(0, Math.min(buf.length, BINARY_SAMPLE));
  if (sample.indexOf(0) >= 0) {
    fail(`error: '${filePath}' is a binary file, skip`);
  }

  const content = buf.toString("utf8");
  const fileName = filePath;

  if (isMarkdownExt(fileName)) {
    return { fileName, content, mode: "markdown", lang: null };
  }

  return { fileName, content, mode: "code", lang: detectLang(fileName) };
}
