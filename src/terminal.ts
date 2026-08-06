/**
 * look —— 终端文件预览 CLI 入口。
 *
 * 装配流程（DESIGN 第 6 节）：
 *   parseArgs → loadContent(二进制检测) → 非 TTY 直出 → code 模式 init shiki
 *   → createTerminalApp → createStdoutRenderer(truecolor/altScreen)
 *   → createStdinDriver(raw, onExit=130) → installTerminalCleanup
 *
 * 退出码：q/Esc→0；Ctrl+C→130；参数错误→2；文件不存在/二进制/目录→1。
 */
import { createTerminalApp, createStdoutRenderer, createStdinDriver, installTerminalCleanup } from "@simon_he/vue-tui/cli";
import PreviewApp from "./PreviewApp.vue";
import { parseArgs } from "./args";
import { loadContent } from "./content";
import { initHighlighter, tokenize, type HighlightToken } from "./highlight";

async function main() {
  const { file } = parseArgs(process.argv.slice(2));
  const loaded = loadContent(file);

  // 非 TTY（管道重定向）：pager 无意义 → 直出原始内容
  if (!process.stdout.isTTY) {
    process.stdout.write(loaded.content);
    process.exit(0);
  }

  // code 模式需 shiki；md 模式跳过以加速启动
  let tokens: HighlightToken[][] | null = null;
  if (loaded.mode === "code") {
    await initHighlighter();
    tokens = tokenize(loaded.content, loaded.lang);
  }

  const cols = process.stdout.columns || 80;
  const rows = process.stdout.rows || 24;

  const app = createTerminalApp({
    cols,
    rows,
    component: PreviewApp,
    props: {
      content: loaded.content,
      tokens,
      fileName: loaded.fileName,
      onQuit: () => quit(0),
    },
  });
  app.mount();

  const renderer = createStdoutRenderer(app.terminal, {
    output: process.stdout,
    hideCursor: true,
    altScreen: true,
    colorMode: "truecolor",
    trackResize: true,
  });

  const driver = createStdinDriver({
    dispatch: (e) => {
      app.events.dispatch(e);
      app.scheduler.flush();
    },
    enableMouse: true,
    onExit: () => quit(130),
  });

  let cleaned = false;
  function cleanup() {
    if (cleaned) return;
    cleaned = true;
    try { driver.dispose(); } catch { /* noop */ }
    try { renderer.dispose(); } catch { /* noop */ }
    try { app.dispose(); } catch { /* noop */ }
  }
  function quit(code: number): never {
    cleanup();
    process.exit(code);
  }

  installTerminalCleanup(cleanup, { signalPolicy: "exit" });
}

main().catch((err) => {
  process.stderr.write(`fatal: ${err?.stack ?? err}\n`);
  process.exit(1);
});
