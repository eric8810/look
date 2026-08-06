/**
 * 扩展名 → shiki 语言 ID 映射。
 * 未知扩展名返回 null（→ 无色纯文本，用作高亮断言对照组）。
 */
const EXT_MAP: Record<string, string> = {
  ts: "typescript",
  tsx: "tsx",
  mts: "typescript",
  cts: "typescript",
  js: "javascript",
  jsx: "jsx",
  mjs: "javascript",
  cjs: "javascript",
  py: "python",
  pyi: "python",
  rs: "rust",
  go: "go",
  java: "java",
  c: "c",
  h: "c",
  cpp: "cpp",
  cc: "cpp",
  cxx: "cpp",
  hpp: "cpp",
  hxx: "cpp",
  cs: "csharp",
  rb: "ruby",
  php: "php",
  swift: "swift",
  kt: "kotlin",
  dart: "dart",
  scala: "scala",
  sh: "bash",
  bash: "bash",
  zsh: "bash",
  fish: "bash",
  json: "json",
  jsonc: "json",
  yaml: "yaml",
  yml: "yaml",
  toml: "toml",
  ini: "ini",
  html: "html",
  htm: "html",
  xml: "xml",
  css: "css",
  scss: "scss",
  less: "less",
  vue: "vue",
  svelte: "svelte",
  sql: "sql",
  graphql: "graphql",
  gql: "graphql",
  md: "markdown",
  markdown: "markdown",
  dockerfile: "dockerfile",
  makefile: "makefile",
  lua: "lua",
  r: "r",
  pl: "perl",
  pm: "perl",
  diff: "diff",
  patch: "diff",
  bat: "bat",
  ps1: "powershell",
  ps: "powershell",
};

/** 特殊文件名（无扩展名但有意义的语言）。 */
const NAME_MAP: Record<string, string> = {
  dockerfile: "dockerfile",
  makefile: "makefile",
  justfile: "makefile",
  gemfile: "ruby",
  rakefile: "ruby",
};

export function detectLang(fileName: string): string | null {
  const base = fileName.split("/").pop() ?? fileName;
  const lower = base.toLowerCase();
  if (NAME_MAP[lower]) return NAME_MAP[lower];
  const dot = lower.lastIndexOf(".");
  if (dot < 0) return null;
  const ext = lower.slice(dot + 1);
  return EXT_MAP[ext] ?? null;
}

/** 是否为 markdown 扩展名（走 TVirtualMarkdown 渲染）。 */
export function isMarkdownExt(fileName: string): boolean {
  const base = fileName.split("/").pop() ?? fileName;
  const lower = base.toLowerCase();
  const dot = lower.lastIndexOf(".");
  if (dot < 0) return false;
  const ext = lower.slice(dot + 1);
  return ext === "md" || ext === "markdown";
}
