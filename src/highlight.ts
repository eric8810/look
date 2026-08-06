/**
 * Shiki 语法高亮集成。
 *
 * 为支持单文件 SEA 打包，使用 @shikijs/core + 静态导入的 grammar/theme
 * （而非 shiki 主包的 createHighlighter，后者用动态 import 会被拆成多 chunk）。
 * 仅导入用到的语言 → bundle 只含这些 grammar，体积可控。
 */
import { createHighlighterCore, type HighlighterCore } from "@shikijs/core";
import { createJavaScriptRegexEngine } from "@shikijs/engine-javascript";

// 静态导入 grammar（按需，避免打包全部 ~200 语言）
import typescript from "@shikijs/langs/typescript";
import tsx from "@shikijs/langs/tsx";
import javascript from "@shikijs/langs/javascript";
import jsx from "@shikijs/langs/jsx";
import python from "@shikijs/langs/python";
import rust from "@shikijs/langs/rust";
import go from "@shikijs/langs/go";
import java from "@shikijs/langs/java";
import c from "@shikijs/langs/c";
import cpp from "@shikijs/langs/cpp";
import csharp from "@shikijs/langs/csharp";
import ruby from "@shikijs/langs/ruby";
import php from "@shikijs/langs/php";
import swift from "@shikijs/langs/swift";
import kotlin from "@shikijs/langs/kotlin";
import dart from "@shikijs/langs/dart";
import scala from "@shikijs/langs/scala";
import bash from "@shikijs/langs/bash";
import json from "@shikijs/langs/json";
import yaml from "@shikijs/langs/yaml";
import toml from "@shikijs/langs/toml";
import ini from "@shikijs/langs/ini";
import html from "@shikijs/langs/html";
import xml from "@shikijs/langs/xml";
import css from "@shikijs/langs/css";
import scss from "@shikijs/langs/scss";
import less from "@shikijs/langs/less";
import vue from "@shikijs/langs/vue";
import svelte from "@shikijs/langs/svelte";
import sql from "@shikijs/langs/sql";
import graphql from "@shikijs/langs/graphql";
import markdown from "@shikijs/langs/markdown";
import dockerfile from "@shikijs/langs/dockerfile";
import makefile from "@shikijs/langs/makefile";
import lua from "@shikijs/langs/lua";
import r from "@shikijs/langs/r";
import perl from "@shikijs/langs/perl";
import diff from "@shikijs/langs/diff";
import bat from "@shikijs/langs/bat";
import powershell from "@shikijs/langs/powershell";

import githubDark from "@shikijs/themes/github-dark";

export const THEME = "github-dark";

/** 支持的语言 ID 集合（与 lang.ts 的映射对应）。 */
export const SUPPORTED_LANGS = new Set<string>([
  "typescript", "tsx", "javascript", "jsx", "python", "rust", "go", "java",
  "c", "cpp", "csharp", "ruby", "php", "swift", "kotlin", "dart", "scala",
  "bash", "json", "yaml", "toml", "ini", "html", "xml", "css", "scss", "less",
  "vue", "svelte", "sql", "graphql", "markdown", "dockerfile", "makefile",
  "lua", "r", "perl", "diff", "bat", "powershell",
]);

const GRAMMARS = [
  typescript, tsx, javascript, jsx, python, rust, go, java, c, cpp, csharp,
  ruby, php, swift, kotlin, dart, scala, bash, json, yaml, toml, ini, html,
  xml, css, scss, less, vue, svelte, sql, graphql, markdown, dockerfile,
  makefile, lua, r, perl, diff, bat, powershell,
];

/** 一个高亮 token：文本 + hex 颜色（无色为 undefined）。 */
export interface HighlightToken {
  text: string;
  color: string | undefined;
}

let highlighter: HighlighterCore | null = null;
let initPromise: Promise<HighlighterCore> | null = null;

export async function initHighlighter(): Promise<HighlighterCore> {
  if (highlighter) return highlighter;
  if (initPromise) return initPromise;
  initPromise = createHighlighterCore({
    langs: GRAMMARS,
    themes: [githubDark],
    engine: createJavaScriptRegexEngine(),
  }).then((hl) => {
    highlighter = hl;
    return hl;
  });
  return initPromise;
}

/**
 * 将代码 tokenize 为每行的 token 数组。
 * lang 为 null 或不支持 → 返回无色纯文本（每行一个 token）。
 */
export function tokenize(code: string, lang: string | null): HighlightToken[][] {
  if (!lang || !SUPPORTED_LANGS.has(lang) || !highlighter) {
    return code.split("\n").map((line) => [{ text: line, color: undefined }]);
  }
  try {
    const result = highlighter.codeToTokens(code, { lang, theme: THEME });
    return result.tokens.map((line) =>
      line.map((t) => ({ text: t.content, color: t.color ?? undefined })),
    );
  } catch {
    // grammar 加载失败等异常 → 降级为纯文本
    return code.split("\n").map((line) => [{ text: line, color: undefined }]);
  }
}
