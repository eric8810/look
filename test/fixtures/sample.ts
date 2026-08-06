// A sample TypeScript file for syntax-highlight testing.
import { readFile } from "node:fs";

interface Config {
  port: number;
  host: string;
  debug?: boolean;
}

const DEFAULT: Config = {
  port: 3000,
  host: "127.0.0.1",
  debug: true,
};

/** Load a JSON config file from disk. */
export async function loadConfig(path: string): Promise<Config> {
  const raw = await readFile(path, "utf8");
  const parsed = JSON.parse(raw) as Config;
  return { ...DEFAULT, ...parsed };
}

function log(msg: string, debug = false): void {
  if (debug) {
    console.log(`[debug] ${msg}`);
  }
}

class Server {
  constructor(private cfg: Config) {}

  start(): void {
    log(`listening on ${this.cfg.host}:${this.cfg.port}`, this.cfg.debug);
  }
}

const numbers = [1, 2, 3, 4, 5].map((n) => n * 2);
const tuple: [string, number] = ["answer", 42];

export { Server, loadConfig, DEFAULT, type Config };

// Tail content to exceed a single screen for scroll testing.
// Line 1 of extra content for scrolling.
// Line 2 of extra content for scrolling.
// Line 3 of extra content for scrolling.
// Line 4 of extra content for scrolling.
// Line 5 of extra content for scrolling.
// Line 6 of extra content for scrolling.
// Line 7 of extra content for scrolling.
// Line 8 of extra content for scrolling.
// Line 9 of extra content for scrolling.
// Line 10 of extra content for scrolling.
