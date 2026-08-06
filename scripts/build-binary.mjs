#!/usr/bin/env node
/**
 * Node SEA 单文件二进制打包脚本（含体积削减）。
 *
 * 流程（DESIGN 第 10、19 节）：
 *   1. vite build（minify）→ dist/terminal.cjs（单文件 CJS）
 *   2. cp $(which node) preview
 *   3. strip preview           ← 削减：移除调试/符号信息（省 ~18MB）
 *   4. node --experimental-sea-config sea-config.json → sea-prep.blob
 *   5. postject preview NODE_SEA_BLOB sea-prep.blob --sentinel-fuse ...
 *   6. macOS: codesign --sign - preview
 *
 * 体积削减说明：
 *   - strip 必须在 postject 注入【之前】执行（注入后 strip 会段错误）。
 *   - UPX 不兼容 postject 修改的 ELF（bad e_phoff），已排除。
 *   - minify 省 ~1MB bundle 体积。
 *   - useCodeCache:true 加速启动（CJS 兼容），不增体积。
 *
 * 结果：~110MB（原 128MB，省 ~17MB / 13%）。
 *
 * 产物名为 `preview`（而非 `look`），避免与系统自带 `look`
 * （util-linux / bsdmainutils）命令冲突。
 */
import { execSync } from "node:child_process";
import { copyFileSync, existsSync, rmSync, statSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);

const OUT = resolve(root, "preview");
const BLOB = resolve(root, "sea-prep.blob");
const SENTINEL = "NODE_SEA_FUSE_fce680ab2cc467b6e072b8b5df1996b2";

function run(cmd, opts = {}) {
  console.log(`> ${cmd}`);
  execSync(cmd, { stdio: "inherit", cwd: root, ...opts });
}

function sizeMB(path) {
  return (statSync(path).size / 1048576).toFixed(1) + "MB";
}

// 1. 确保 dist/terminal.cjs 已构建（minify）
if (!existsSync(resolve(root, "dist/terminal.cjs"))) {
  console.log("dist/terminal.cjs not found, running build...");
  run("pnpm build");
}
console.log(`  bundle: ${sizeMB(resolve(root, "dist/terminal.cjs"))}`);

// 2. 复制 node 二进制
const nodeBin = process.execPath;
console.log(`> cp ${nodeBin} preview  (node: ${sizeMB(nodeBin)})`);
copyFileSync(nodeBin, OUT);

// 3. strip —— 必须在 SEA 注入之前（注入后 strip 会段错误）
if (process.platform !== "win32") {
  console.log(`> strip preview  (before: ${sizeMB(OUT)})`);
  try {
    execSync("strip preview", { stdio: "pipe", cwd: root });
  } catch (e) {
    // 某些环境 strip 报 section 警告但仍成功；检查体积是否减小
    console.log("  strip emitted warnings, continuing...");
  }
  console.log(`  after strip: ${sizeMB(OUT)}`);
}

// 4. 生成 SEA blob
run("node --experimental-sea-config sea-config.json");
if (!existsSync(BLOB)) {
  console.error("ERROR: sea-prep.blob was not generated");
  process.exit(1);
}

// 5. 注入 blob
const postjectBin = require.resolve("postject/dist/cli.js");
run(`node ${JSON.stringify(postjectBin)} preview NODE_SEA_BLOB ${JSON.stringify(BLOB)} --sentinel-fuse ${SENTINEL}`);
console.log(`  after inject: ${sizeMB(OUT)}`);

// 6. macOS 重新签名
if (process.platform === "darwin") {
  try { run("codesign --remove-signature preview", { stdio: "pipe" }); } catch { /* noop */ }
  run("codesign --sign - preview");
}

// 清理 blob
rmSync(BLOB, { force: true });

console.log(`\n✓ Built SEA binary: ${OUT}  (${sizeMB(OUT)})`);
console.log(`  Run: ./preview README.md`);
