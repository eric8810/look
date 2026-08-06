/**
 * argv 解析：look <file> | --help/-h | --version/-V
 *
 * 退出码：
 *   无参 / 参数 >1 → 2
 *   --help / --version → 0（打印后直接退出）
 *   正好 1 个位置参数 → 返回该文件路径
 */

export const VERSION = "look 0.1.0";

const HELP = `\
look — a minimal terminal file previewer

Usage:
  look <file>        Preview a markdown / code / text file in the terminal
  look --help, -h    Show this help
  look --version, -V Show version

Keys:
  q / Esc            Quit
  j / k              Scroll down / up one line
  Space / PageDown   Scroll down one page
  PageUp             Scroll up one page
  g / G              Go to top / bottom
  Arrow Up/Down      Scroll one line
  Home / End         Go to top / bottom
  Ctrl+C             Quit (exit 130)
`;

export interface ParsedArgs {
  file: string;
}

function printAndExit(code: number, text: string): never {
  process.stdout.write(text + "\n");
  process.exit(code);
}

export function parseArgs(argv: string[]): ParsedArgs {
  // argv 不含 node 与脚本路径
  const positional: string[] = [];

  for (const arg of argv) {
    if (arg === "--help" || arg === "-h") {
      printAndExit(0, HELP);
    }
    if (arg === "--version" || arg === "-V") {
      printAndExit(0, VERSION);
    }
    positional.push(arg);
  }

  if (positional.length === 0) {
    process.stderr.write(HELP + "\n");
    process.exit(2);
  }

  if (positional.length > 1) {
    process.stderr.write(
      `error: too many arguments (expected 1, got ${positional.length})\n\n${HELP}\n`,
    );
    process.exit(2);
  }

  return { file: positional[0] };
}

export { HELP };
