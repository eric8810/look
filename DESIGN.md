# dlook 设计文档

> 一个最简单的终端文件预览 CLI，能渲染 markdown 格式，也能看代码/纯文本。
> 技术栈：[vue-tui](https://vue-tui.pages.dev/)（Vue 3 terminal UI + stdout renderer）。
> 打包：Node SEA 单文件二进制。
>
> **注**:本项目原名 `look`,2026-09 起项目名/仓库名统一改为 **dlook**(产物二进制自 v0.2.0 起即为 dlook)。
> 文中旧称 `look`/`preview` 均指同一项目的历史阶段。

---

## 1. 调查结论（关键事实，均经源码验证）

| 结论 | 依据 |
|---|---|
| CLI 运行时入口为 `@simon_he/vue-tui/cli`，提供 `createTerminalApp` / `createStdoutRenderer` / `createStdinDriver` / `installTerminalCleanup` | `examples/basic/src/terminal.ts`、stdout renderer 文档 |
| 官方终端构建方式：`vite build --mode terminal` → 产出单文件 ESM → `node dist/terminal.js`；node 内置模块 external | `examples/basic/vite.config.ts` |
| `TVirtualMarkdown` 是**虚拟化滚动**组件，支持 `v-model:scrollTop`、`autoFocus`、原生处理 `↑↓/PageUp/PageDown/Home/End/滚轮`，并先 emit `keydown` | `src/vue/components/TVirtualMarkdown.ts` 的 `onKeydown` |
| `TMarkdownText` 是非虚拟、自适应高度、**无滚动**——不适合 pager | `src/vue/components/TMarkdownText.ts` |
| 代码块**无语法高亮**：markdown theme 只有 `codeBlock`/`inlineCode` 单一样式（默认 `yellowBright`），仓库无 shiki/prism 依赖 | `src/vue/markdown/theme.ts`、root `package.json` |
| `createStdinDriver` 自动 `setRawMode(true)`，未处理的 `Ctrl+C` 触发 `onExit`；`q`/`Escape` 作为 `keydown` 派发 | `src/cli/input.ts` |
| `createTerminalApp({ component, props })` 在 CLI 模式下自带 terminal context，子组件可直接用 `useTerminal()`，**无需** `<TerminalProvider>` | 两个 terminal.ts 示例 |
| `Style.fg` 接受 **hex 字符串**（`'#a9dc76'` → 转 truecolor/ansi256），渲染器 `colorMode:"truecolor"` 即可输出 24-bit 色 | `src/core/types.ts` |
| `useRenderNode` / `useTerminalNode` / `useTerminal` / `useLayout` / `useVisibility` 均从 `@simon_he/vue-tui/vue` 导出，自建组件可完整复刻 TVirtualMarkdown 契约 | `src/vue/index.ts` |
| Node SEA 支持 `mainFormat: "module"`（ESM）；Node 25.5+ 可 `--build-sea` 一步打包，Node 24 用经典 `--experimental-sea-config` + `postject` 流程 | nodejs.org SEA 文档 |

**核心判断**：

- markdown 文件 → 直接用 `TVirtualMarkdown`，一个组件同时承担渲染 + 滚动。
- 代码/文本文件（需 token 级高亮）→ 自建 `CodeView`，复刻 `TVirtualMarkdown` 的滚动/键盘契约，paint 层用 shiki token 的 hex 色。
- 两个 body 组件对外接口完全一致，父组件 `PreviewApp` 不感知差异。

---

## 2. CLI 接口

```
look <file>            # 预览单个文件
look --help | -h       # 用法
look --version | -V    # 版本
```

> 注：`look` 与部分 Linux（bsdmainutils）已有命令同名。本工具主命令定为 `look`，如需可加 `preview` 别名（两个 bin 指向同一入口）。

### 输入校验（fail-fast）

- 无参数 / 参数 >1 → 打印 usage，退出码 2
- 文件不存在 / 不可读 → 报错，退出码 1
- **二进制检测**：读前 8KB，含 `\0` → "binary file, skip"，退出码 1
- **非 TTY**（输出被管道重定向）：pager 无意义 → 直接 `process.stdout.write(原始内容)` 后退出（兼容 `look x.md | grep`）

### 退出码

| 情形 | 退出码 |
|---|---|
| 正常退出（`q`/`Esc`/文件读完） | 0 |
| 参数错误 | 2 |
| 文件不存在/不可读/二进制 | 1 |
| `Ctrl+C`（由 `onExit`/cleanup 决定） | 130 |

---

## 3. 渲染模型：双 body 组件，统一契约

| 文件类型 | body 组件 | 高亮 | 滚动 |
|---|---|---|---|
| `.md`/`.markdown` | `TVirtualMarkdown` | markdown 原生（标题/列表/表格/链接；其中围栏代码块单色，v1 可接受） | 组件原生 `↑↓ PgUp/Dn Home/End 滚轮` |
| 代码/纯文本 | **`CodeView`（自建）** | **shiki token 级 truecolor 高亮** | 自建，契约同上 |

两个组件对外接口完全一致：

```
:auto-focus="true"   v-model:scrollTop="top"   @keydown="onKey"
:content / :tokens   :x :y :w :h
```

`onKey`（父级，两模式共用）只管：`q`/`Esc`→退出，`j`/`k`→行滚，`space`/`PageDown`→页滚，`g`/`G`→首末。箭头/Page/Home/End/滚轮由各 body 自身处理，**父级不碰**，避免双重滚动。

### 3.1 markdown 文件

- `content` 原样喂给 `TVirtualMarkdown`。
- 通过 `theme` prop 调 `codeBlock` 样式（如 `{ fg: 'white' }`），避免默认全黄。

### 3.2 代码/文本文件

- 扩展名 → shiki lang 映射（`lang.ts`）：`ts/js/py/rs/go/java/c/cpp/sh/json/yaml/html/css/vue/sql/tsx/jsx/...`，未知扩展名 → 无色纯文本。
- `token.color`（hex）直接喂 `Style.fg`，渲染器 `colorMode:"truecolor"` 输出 24-bit 色。
- **长行**：v1 截断到 `w`（类似 `less -S`），不做软换行（软换行会拆 token，留 v2 + 横向滚动）。
- **Tab**：渲染前展开为空格（tabstop=2）。

---

## 4. `CodeView.vue`（自建，复刻 TVirtualMarkdown 契约）

用 `@simon_he/vue-tui/vue` 的 composables 实现，paint 用 `terminal.write(text,{x,y,style})`。

### 关键结构（精简伪代码）

```ts
const { terminal, defaultStyle, events, scheduler, widthProvider } = useTerminal();
const layout = useLayout();
const { visible } = useVisibility();

const top = ref(0);

const eventNode = useTerminalNode(() => ({
  rect: normalizedRect(),
  zIndex: 0, visible: visible.value, focusable: true, selectable: false,
  handlers: {
    keydown: (e) => {
      emit("keydown", e);                 // 先抛给父级处理 q/j/k/space/g/G
      const page = props.h;
      if (e.key === "ArrowDown") { e.preventDefault(); setTop(top.value + 1); }
      else if (e.key === "ArrowUp")   { e.preventDefault(); setTop(top.value - 1); }
      else if (e.key === "PageDown")  { e.preventDefault(); setTop(top.value + page); }
      else if (e.key === "PageUp")    { e.preventDefault(); setTop(top.value - page); }
      else if (e.key === "Home")      { e.preventDefault(); setTop(0); }
      else if (e.key === "End")       { e.preventDefault(); setTop(maxTop()); }
    },
    wheel: (e) => { /* 同 TVirtualMarkdown：applyWheelScroll → setTop */ },
    focus: () => emit("focus"), blur: () => emit("blur"),
  },
}));

watchEffect(() => {                       // autoFocus，镜像 TVirtualMarkdown
  if (!props.autoFocus || !visible.value) return;
  const id = eventNode.id.value, mgr = events.value;
  if (id && mgr && mgr.getFocused() !== id) mgr.focus(id);
});

useRenderNode(() => ({
  zIndex: 0,
  rect: visible.value ? normalizedRect() : { x:0, y:0, w:0, h:0 },
  deps: [visible.value, rect, top.value, tokensVersion],
  paint: (dirtyRows) => {                 // 按可见行逐 token 写色
    const r = normalizedRect();
    for (const y of (dirtyRows ?? range(r.y, r.y + r.h))) {
      const li = top.value + (y - r.y);          // 逻辑行
      const tokens = lines[li] ?? [];            // [{text,color}]
      let x = r.x;
      for (const t of tokens) {
        terminal.write(t.text, { x, y, style: { fg: t.color } });  // color 为 hex → truecolor
        x += cellWidth(t.text);
      }
      if (x < r.x + r.w) terminal.write(spaces(r.x + r.w - x), { x, y, style: {} }); // 清余白
    }
  },
}));
```

### 要点

- **虚拟化**：只 paint `[top, top+h)` 行，大文件无压力；`maxTop = max(0, lines.length - h)`。
- `top` 即 `scrollTop`，`v-model:scrollTop` 双向（组件内 `setTop` emit `update:scrollTop`）。
- paint 时先 emit `keydown`，父级未 preventDefault 的才落入组件原生滚动逻辑。

---

## 5. 交互模型

`TVirtualMarkdown` / `CodeView` 设 `autoFocus`，原生处理箭头/Page/滚轮。父级 `onKey` 补齐 pager 习惯键：

| 键 | 行为 | 处理方 |
|---|---|---|
| `q` / `Esc` | 退出（调 `onQuit` → cleanup + exit 0） | 父级 onKey |
| `j` / `k` | 下/上 1 行 | 父级 onKey（改 `scrollTop` ref） |
| `Space` / `PageDown` | 下翻 1 页 | 父级 onKey |
| `g` / `G` | 顶部 / 底部 | 父级 onKey |
| `↑` / `↓` | 行滚 | body 原生 |
| `PageUp` | 上翻 1 页 | body 原生 |
| `Home` / `End` | 首末 | body 原生 |
| 滚轮 | 上下滚 | body 原生 |
| `Ctrl+C` | 退出 | stdin driver `onExit` 兜底 |

`scrollTop` 用 `v-model` 双向绑定：组件原生滚动会 `emit('update:scrollTop')` 同步 ref；父级改 ref 时组件 `watch(props.scrollTop)` 应用。无冲突。

---

## 6. 运行时装配（`src/terminal.ts` 骨架）

```ts
import { createTerminalApp, createStdoutRenderer, createStdinDriver,
         installTerminalCleanup } from "@simon_he/vue-tui/cli";
import PreviewApp from "./PreviewApp.vue";
import { loadPreviewContent } from "./content";
import { initHighlighter } from "./highlight";

const { file, mode } = parseArgs(process.argv.slice(2));        // 校验 + 二进制检测
if (!process.stdout.isTTY) { process.stdout.write(rawContent); process.exit(0); }

// 仅 code 模式需要 shiki；md 模式可跳过以加速启动
let tokens = null;
if (mode === "code") {
  await initHighlighter();
  tokens = tokenize(content, lang);
}

const cols = process.stdout.columns || 80;
const rows = process.stdout.rows    || 24;

const app = createTerminalApp({
  cols, rows,
  component: PreviewApp,
  props: { content, tokens, fileName: file, cols, rows, onQuit: () => exit(0) },
});
app.mount();

const out = createStdoutRenderer(app.terminal, {
  output: process.stdout, hideCursor: true, altScreen: true,
  colorMode: "truecolor", trackResize: true,        // truecolor 支持高亮色
});

const driver = createStdinDriver({
  dispatch: (e) => { const p = app.events.dispatch(e); app.scheduler.flush(); return p; },
  enableMouse: true,
  onExit: () => exit(130),   // 未处理的 Ctrl+C → 130（与退出码表一致）；q/Esc 走 onQuit → 0
});
installTerminalCleanup(() => { driver.dispose(); out.dispose(); app.dispose(); },
                       { signalPolicy: "exit" });

process.stdout.on("resize", () =>
  app.terminal.resize(process.stdout.columns || cols, process.stdout.rows || rows));
```

### 退出/cleanup 顺序

```
onQuit / onExit(Ctrl+C)
  → cleanup(): driver.dispose() → out.dispose() → app.dispose()
  → process.exit(0)
```

---

## 7. `PreviewApp.vue`

```vue
<script setup lang="ts">
import { ref, computed } from "vue";
import { TText } from "@simon_he/vue-tui";
import { TVirtualMarkdown } from "@simon_he/vue-tui/markdown";
import CodeView from "./CodeView.vue";

const props = defineProps<{
  content: string;
  tokens: any[][] | null;       // null = markdown 模式
  fileName: string;
  cols: number;
  rows: number;
  onQuit: () => void;
}>();

const top = ref(0);
const bodyH = props.rows - 2;
const isMd = computed(() => props.tokens === null);

function onKey(e: any) {
  if (e.key === "q" || e.key === "Escape") return props.onQuit();
  if (e.key === "j") top.value += 1;
  else if (e.key === "k") top.value -= 1;
  else if (e.key === " ") top.value += bodyH;
  else if (e.key === "g") top.value = 0;
  else if (e.key === "G") top.value = 1e9;            // 组件会 clamp 到 maxTop
}
</script>

<template>
  <TText :x="0" :y="0" :w="cols" :h="1" :style="{ bold: true }">{{ fileName }}</TText>

  <component :is="isMd ? TVirtualMarkdown : CodeView"
    :x="0" :y="1" :w="cols" :h="bodyH"
    :content="content" v-model:scrollTop="top" :auto-focus="true"
    :tokens="tokens" @keydown="onKey" />

  <TText :x="0" :y="rows - 1" :w="cols" :h="1" :style="{ dim: true }">
    q quit · ↑↓/jk scroll · space pgdn · g/G top/bottom
  </TText>
</template>
```

布局：

```
┌─ row 0 ─── header（文件名，bold）──────────────────┐
│                                                     │
│  TVirtualMarkdown / CodeView  (h = rows - 2)        │
│                                                     │
└─ row rows-1 ─ footer（按键提示，dim）──────────────┘
```

---

## 8. Shiki 集成（`src/highlight.ts`）

```ts
import { createHighlighter } from "shiki";

let hl: Awaited<ReturnType<typeof createHighlighter>>;

const LANGS = [
  "typescript", "javascript", "python", "rust", "go", "java", "c", "cpp",
  "bash", "json", "yaml", "html", "css", "vue", "markdown", "sql", "tsx", "jsx",
];

export async function initHighlighter() {
  hl = await createHighlighter({ langs: LANGS, themes: ["github-dark"] });
}

export function tokenize(code: string, lang: string | null) {
  if (!lang || !LANGS.includes(lang)) {
    return code.split("\n").map(l => [{ text: l, color: undefined }]);
  }
  const { tokens } = hl.codeToTokens(code, { lang, theme: "github-dark" });
  return tokens.map(line => line.map(t => ({ text: t.content, color: t.color })));  // color = '#hex'
}
```

要点：

- `token.color` 是 hex → 直接喂 `Style.fg`（已验证接受 hex，渲染器转 truecolor）。
- stdout renderer 设 `colorMode: "truecolor"`。
- **未知语言** → 无色纯文本。
- 启动：`terminal.ts` 中 `await initHighlighter()` 后再 `createTerminalApp`（仅 code 模式）。
- **打包**：shiki 为 ESM，Vite 静态打包。用 `createHighlighter` + 固定 `langs/themes`，Vite 会 bundle 对应 grammar/theme。若遇动态 import 警告，改用 `@shikijs/core` + `bundledLanguages`/`bundledThemes` 显式引入。SEA 体积增量约 1–2MB（相对 ~90MB Node 运行时可忽略）。

---

## 9. 项目结构

```
doc-preview/
  package.json            # bin: { look }; type: module
  tsconfig.json
  vite.config.ts          # terminal lib 构建（es, external node 内置, target node18）
  sea-config.json         # { main, mainFormat:"module", useCodeCache:true }
  src/
    terminal.ts           # 入口：argv + 二进制检测 + shiki init + 运行时装配
    PreviewApp.vue        # UI：header + body(md/code) + footer
    CodeView.vue          # 自建高亮代码视图（/vue composables）
    content.ts            # 读文件 + 二进制检测 + 围栏安全(md)
    highlight.ts          # shiki init + tokenize
    lang.ts               # 扩展名 → shiki lang 映射
    args.ts               # argv 解析 + --help/--version
  scripts/
    build-binary.mjs      # Node SEA 打包脚本
  README.md
  DESIGN.md               # 本文档
```

### 依赖

- 运行时：`@simon_he/vue-tui`、`vue`、`shiki`
- dev：`vite`、`@vitejs/plugin-vue`、`typescript`、`postject`（SEA 用）

---

## 10. 构建与打包

### 开发/运行（与官方示例一致）

```bash
pnpm i
pnpm build          # vite build --mode terminal → dist/terminal.js
node dist/terminal.js README.md    # 直接跑
```

### 打包成单文件二进制（Node SEA）

```bash
# 1) Vite 产出单文件 ESM（dist/terminal.js）
pnpm build

# 2) 生成 SEA blob
node --experimental-sea-config sea-config.json
#   sea-config.json:
#   { "main":"dist/terminal.js", "mainFormat":"module",
#     "output":"sea-prep.blob", "useCodeCache": true }

# 3) 复制 node 二进制并注入
cp $(command -v node) look
npx postject look NODE_SEA_BLOB sea-prep.blob \
  --sentinel NODE_SEA_FUSE_fce680ab2cc467b6e072b8b5df1996b2

# 4) macOS 需签名
codesign --sign - look   # 仅 macOS

# 5) 运行
./preview README.md
```

`scripts/build-binary.mjs` 自动化上述步骤。产物 ~90MB（含 Node 运行时）。Node 25.5+ 可用 `node --build-sea sea-config.json` 一步完成。

### Vite 配置要点（`vite.config.ts`）

参考官方 `examples/basic/vite.config.ts`：

- `build.lib.entry = src/terminal.ts`，`formats: ["es"]`
- `rollupOptions.external` = node 内置模块（fs/path/process/buffer/...）
- `build.target = "node18"`，`minify: false`
- resolve alias：`@simon_he/vue-tui` → 包入口（实际用包名即可，非 monorepo 不需 alias）

---

## 11. 风险与取舍

| 风险/取舍 | 说明 | 应对 |
|---|---|---|
| markdown 代码块单色 | vue-tui markdown 的 codeBlock 是单一 Style，无 token 高亮 | md 文件内的围栏代码块 v1 单色（可调主题色）；独立代码文件走 `CodeView` 高亮 |
| `look` 命令名冲突 | 部分 Linux（bsdmainutils）已有 `look` | v1 仍用 `look`；如冲突可加 `preview` 别名或自定义名 |
| 超大文件 | TVirtualMarkdown/CodeView 虚拟化渲染无压力，但全量读入内存 | 可加 `--max-size` 软上限告警（v2） |
| shiki 启动开销 | 首次 `createHighlighter` 需加载 grammar/theme | 仅 code 模式初始化；md 模式跳过 |
| SEA 体积 | 含 Node 运行时 ~90MB | 可接受；若需更小可换 `bun build --compile`（备选） |
| 围栏安全（md） | 代码内含 ``` 可能破坏 markdown 解析 | 用"最长反引号 run +1"作围栏长度 |
| 长行（code） | v1 截断到 `w`（`less -S` 风格） | 软换行 + 横向滚动留 v2 |
| 非 TTY | pager 无意义 | 直接输出原始文本，不做 TUI 渲染 |

---

## 12. 实现路线（建议顺序）

1. 脚手架：`package.json` / `tsconfig.json` / `vite.config.ts`，跑通空 `terminal.ts`
2. `args.ts` + `content.ts`：argv 解析 + 文件读取 + 二进制检测 + 非 TTY 直出
3. `PreviewApp.vue` + `TVirtualMarkdown`：先只做 md 模式，跑通 `look README.md`
4. `highlight.ts` + `CodeView.vue`：shiki 集成 + 高亮代码视图，跑通 `look src/terminal.ts`
5. `lang.ts`：扩展名映射补全
6. 交互打磨：`onKey` 全键位 + footer 提示 + resize
7. `scripts/build-binary.mjs` + `sea-config.json`：SEA 打包，跑通 `./preview README.md`
8. README
9. `test/` 验收脚本 + tmux E2E 跑通（见第 13–18 节）

---

## 13. 测试分层

| 层 | 名称 | 方式 | 速度 | 用途 |
|---|---|---|---|---|
| L0 | 静态检查 | `tsc --noEmit` + `vite build` | 秒级 | 类型/构建不破 |
| L1 | headless smoke | `VT_SMOKE=1` 渲染一帧，读 `app.terminal.getRow(y)` 断言 | 秒级 | 布局/滚动数学/高亮 token 的确定性逻辑（无 tmux，CI 友好） |
| **L2** | **tmux E2E 验收**（主） | 真实 pty + `send-keys` 模拟按键 + `capture-pane` 断言 | 秒级 | **用户视角实际使用验收**：真实 TTY/raw mode/ANSI/alt-screen/resize |

L2 是本节核心：在 tmux pane 里跑真实二进制，用 `send-keys` 模拟用户敲键，用 `capture-pane` 抓取渲染结果做断言。这与真实终端使用等价（`process.stdout.isTTY` 为真，raw mode 生效，alt-screen 真实切换）。

### L1 headless smoke（可选补充）

复用 vue-tui 官方 `VT_SMOKE` 模式：在 `terminal.ts` 里识别 `VT_SMOKE=1`，渲染一帧后读 buffer 行断言，不接 stdin、不进 alt-screen、不真实输出。

```ts
if (process.env.VT_SMOKE === "1") {
  app.scheduler.flushNow();
  const header = rowText(app, 0);                 // app.terminal.getRow(0).map(c=>c.ch).join("")
  const bodyTop = rowText(app, 1);
  // 断言 header 含文件名、bodyTop 含首行、CodeView 行的 cell.style.fg 为 hex（高亮生效）
  console.log(JSON.stringify({ header, bodyTop, hasColor: cellHasTruecolor(app, 1) }));
  exit(0);
}
```

适合在 CI 里快速卡 layout 与高亮逻辑。L2 覆盖真实交互，二者互补。

---

## 14. tmux 验收环境

### 14.1 原理

- `tmux new-session -d -s look-acc -x 80 -y 24`：创建固定尺寸的脱离会话，pane 即一个真实 pty。
- `tmux send-keys -t look-acc <key>`：向 pane pty 写入按键 = 用户敲键。app 在 raw mode 下逐字节读取，与真实终端一致。
- `tmux capture-pane -p -t look-acc`：抓取 pane 可见文本（纯文本）。
- `tmux capture-pane -p -e -t look-acc`：抓取时保留 ANSI 转义码，用于断言颜色。
- 退出码：在 pane 里跑 `./preview FILE; echo "EXIT:$?"`，退出后 `EXIT:N` 出现在 pane 文本里，断言之。
- alt-screen 恢复：app 用 `altScreen:true`，退出后恢复 alt-screen 保存的画面。启动前先 `echo MARKER` 打个标记，退出后断言标记重现，证明 alt-screen 正确还原。
- resize：`tmux resize-window -t look-acc -x 120 -y 40` → pane pty 尺寸变化 → Node stdout `resize` 事件 → app 重排。

### 14.2 tmux 按键名对照

| 用户键 | tmux send-keys 参数 | app 收到的 key |
|---|---|---|
| `q` / `g` | `q` / `g` | `"q"` / `"g"` |
| `G`（Shift+g） | `G` | `"G"` |
| 空格 | `Space` | `" "` |
| `↑` `↓` | `Up` `Down` | `"ArrowUp"`/`"ArrowDown"` |
| `PageUp` `PageDown` | `PageUp` `PageDown` | `"PageUp"`/`"PageDown"` |
| `Home` `End` | `Home` `End` | `"Home"`/`"End"` |
| `Esc` | `Escape` | `"Escape"` |
| `Ctrl+C` | `C-c` | raw `\x03` → 未处理 → `onExit` |

### 14.3 公共辅助库

`test/e2e/lib/tmux.sh`：

```bash
SESSION="look-acc"
BIN="${BIN:-./preview}"
FIX="$(cd "$(dirname "$0")/../fixtures" && pwd)"

tmx_new() {                       # $1=cols $2=rows
  tmux kill-session -t "$SESSION" 2>/dev/null
  tmux new-session -d -s "$SESSION" -x "$1" -y "$2"
  tmux set -g status off          # 不占行
  tmux set -g mouse off           # 不拦截鼠标（若测滚轮）
}
tmx_send()  { tmux send-keys -t "$SESSION" "$@"; }
tmx_enter() { tmux send-keys -t "$SESSION" Enter; }
tmx_run()   { tmx_send "$1"; tmx_enter; }                 # 在 shell 跑一条命令
tmx_cap()   { tmux capture-pane -p -e -t "$SESSION"; }    # 含 ANSI
tmx_capp()  { tmux capture-pane -p   -t "$SESSION"; }     # 纯文本
tmx_row()   { tmux capture-pane -p -t "$SESSION" | sed -n "${1}p"; }  # 第 N 行(1-indexed)
tmx_kill()  { tmux kill-session -t "$SESSION" 2>/dev/null; }

# 轮询直到 pane 文本匹配正则，超时返回 1
wait_for() {                      # $1=regex  $2=timeout秒(默认5)
  local pat="$1" t="${2:-5}" i
  for ((i=0;i<t*10;i++)); do
    if tmx_capp | grep -Eq "$pat"; then return 0; fi
    sleep 0.1
  done
  return 1
}
```

`test/e2e/lib/assert.sh`：

```bash
PASS=0; FAIL=0
say_contains() {                  # $1=desc $2=needle（在 $CAP 中）
  if printf '%s' "$CAP" | grep -Fq -- "$2"; then echo "  ✓ $1"; PASS=$((PASS+1));
  else echo "  ✗ $1"; FAIL=$((FAIL+1)); echo "    缺少: $2"; fi
}
say_matches() {                   # $1=desc $2=regex
  if printf '%s' "$CAP" | grep -Eq -- "$2"; then echo "  ✓ $1"; PASS=$((PASS+1));
  else echo "  ✗ $1"; FAIL=$((FAIL+1)); fi
}
say_not_match() {                 # $1=desc $2=regex（不应出现）
  if printf '%s' "$CAP" | grep -Eq -- "$2"; then echo "  ✗ $1"; FAIL=$((FAIL+1));
  else echo "  ✓ $1"; PASS=$((PASS+1)); fi
}
say_exit() {                      # $1=desc $2=期望退出码
  if printf '%s' "$CAP" | grep -Eq "EXIT:$2($|[^0-9])"; then echo "  ✓ $1"; PASS=$((PASS+1));
  else echo "  ✗ $1 (期望 EXIT:$2)"; FAIL=$((FAIL+1)); fi
}
```

---

## 15. 验收用例矩阵（用户视角）

> 每个用例：**命令 → 操作 → 捕获 → 断言 → 通过标准**。`CAP` 为最近一次 `tmx_cap`/`tmx_capp` 结果。

### 15.1 启动与渲染

| # | 用例 | 命令 | 操作 | 断言 | 通过标准 |
|---|---|---|---|---|---|
| A1 | 启动渲染 markdown | `look sample.md` | wait_for `sample.md` | header(row1) 含 `sample.md`；末行含 footer `q quit`；body 含标题文本 | 三者全中 |
| A2 | 启动渲染代码（高亮） | `look sample.ts` | wait_for `sample.ts`(8s) | header 含 `sample.ts`；`CAP`(-e) 含 truecolor `\x1b\[38;2;` | 全中 |
| A3 | 未知扩展名无高亮 | `look unknown.xyz` | wait_for | header 含文件名；`CAP`(-e) **不含** `38;2;` | 全中 |
| A4 | 纯文本 | `look plain.txt` | wait_for | body 含纯文本行；无 truecolor | 全中 |

### 15.2 滚动（用 `large.txt`，1 源行 = 1 视觉行，精确）

`large.txt`：2000 行，第 N 行为 `LINE_NNNN`，每 10 的倍数行 `MARKER:NNNN`。body 高 22，header row1，body 顶 = pane row2。

| # | 用例 | 操作 | 断言（pane row2 = body 顶） | 通过标准 |
|---|---|---|---|---|
| B1 | 初始顶部 | wait_for `LINE_0000` | `tmx_row 2` == `LINE_0000` | 中 |
| B2 | j 行滚 | `tmx_send j`×10 | `tmx_row 2` == `MARKER:0010` | 中 |
| B3 | k 回滚 | `tmx_send k`×5 | `tmx_row 2` == `MARKER:0005` | 中 |
| B4 | space 翻页 | `tmx_send Space` | `tmx_row 2` == `LINE_0022`（+22） | 中 |
| B5 | PageDown | `tmx_send PageDown` | row2 == `LINE_0044`（再 +22） | 中 |
| B6 | PageUp | `tmx_send PageUp` | row2 == `LINE_0022` | 中 |
| B7 | ↓ 箭头 | `tmx_send Down`×3 | row2 == `LINE_0025` | 中 |
| B8 | ↑ 箭头 | `tmx_send Up`×3 | row2 == `LINE_0022` | 中 |
| B9 | Home | `tmx_send Home` | row2 == `LINE_0000` | 中 |
| B10 | End | `tmx_send End` | 末页可见最后行 `LINE_1999`（在 CAP 中） | 中 |
| B11 | g 顶部 | `tmx_send g` | row2 == `LINE_0000` | 中 |
| B12 | G 底部 | `tmx_send G` | CAP 含 `LINE_1999` | 中 |
| B13 | g→G→g 往返 | g, G, g | 首行→末页→首行 | 各步断言中 |

### 15.3 退出与 alt-screen 恢复

启动前 seed 标记：`tmx_run "echo MK_$_OK; ./preview sample.md; echo EXIT:$?"`。

| # | 用例 | 操作 | 断言 | 通过标准 |
|---|---|---|---|---|
| C1 | q 退出 | `tmx_send q` | wait_for `EXIT:`；`say_exit "q→0" 0`；`say_contains "alt-screen restore" "MK_$_OK"` | 全中 |
| C2 | Esc 退出 | （重启）`tmx_send Escape` | `say_exit "Esc→0" 0`；标记重现 | 全中 |
| C3 | Ctrl+C 退出 | `tmx_send C-c` | `say_exit "Ctrl+C→130" 130`；标记重现 | 全中 |

### 15.4 终端 resize

| # | 用例 | 操作 | 断言 | 通过标准 |
|---|---|---|---|---|
| D1 | 放大 | `look sample.ts` → `tmux resize-window -x 120 -y 40` | wait 后：header 仍在 row1 含文件名；footer 在末行含 `q quit`；CAP 行数 ≥ 40；body 可见行变多 | 全中 |
| D2 | 缩小 | resize 回 `-x 60 -y 12` | header/footer 仍正确定位；不崩；body 内容仍在 | 全中 |

### 15.5 错误处理（退出码）

每个用 `tmx_run "./preview <args>; echo EXIT:$?"`，app 自行退出，wait_for `EXIT:`。

| # | 用例 | 命令 | 断言 | 通过标准 |
|---|---|---|---|---|
| E1 | 无参数 | `./preview` | `say_exit 2`；CAP 含 usage/help | 中 |
| E2 | 文件不存在 | `./preview nope.md` | `say_exit 1`；CAP 含错误信息 | 中 |
| E3 | 二进制文件 | `./preview binary.bin` | `say_exit 1`；CAP 含 `binary` | 中 |
| E4 | 目录 | `./preview test/fixtures` | `say_exit 1`；CAP 含错误 | 中 |
| E5 | 多余参数 | `./preview a.md b.md` | `say_exit 2` | 中 |

### 15.6 非 TTY（管道）

| # | 用例 | 命令 | 断言 | 通过标准 |
|---|---|---|---|---|
| F1 | 管道直出 | `./preview sample.md \| cat; echo EXIT:$?` | `say_exit 0`；CAP 含 sample.md 的**原始**标题文本（非 TUI 样式） | 中 |
| F2 | 代码管道 | `./preview sample.ts \| cat` | `say_exit 0`；CAP 含原始代码（无 truecolor） | 中 |

### 15.7 超大文件 / 虚拟化

| # | 用例 | 操作 | 断言 | 通过标准 |
|---|---|---|---|---|
| G1 | 大文件可滚 | `look large.txt` → G → g → 反复 j/k | 不崩；row2 随滚动正确变化；内存平稳 | 中 |
| G2 | 启动延迟 | 计时 `look large.txt` 到 wait_for 完成 | < 1.5s（code 模式含 shiki < 3s） | 中 |

### 15.8 markdown 元素渲染

用 `sample.md`（含标题/列表/表格/围栏代码/链接/引用）。

| # | 用例 | 操作 | 断言 | 通过标准 |
|---|---|---|---|---|
| H1 | 标题加粗 | wait_for | row 内容对应标题（bold，`-e` 含 `\x1b[1m`） | 中 |
| H2 | 列表项 | — | CAP 含 `- item one` | 中 |
| H3 | 表格 | — | CAP 含 `|` 分隔的表行 | 中 |
| H4 | 围栏代码块 | — | CAP 含代码行（单色，无 truecolor token） | 中 |
| H5 | 链接 | — | CAP 含 `example.com` 或 link 文本 | 中 |

---

## 16. 测试夹具（`test/fixtures/`）

| 文件 | 内容规格 | 用途 |
|---|---|---|
| `sample.md` | 标题 + 粗体 + 行内代码 + 无序/有序列表 + 表格 + 围栏 ` ```ts ` 块 + 链接 + 引用，约 30 行（超出单屏可滚） | A1/C/H |
| `sample.ts` | 含 import/接口/函数/字符串/注释/模板串，>24 行，跨屏可滚 | A2/D/H4 |
| `plain.txt` | 普通多行文本 | A4 |
| `unknown.xyz` | 任意文本，扩展名不在映射表 | A3 |
| `binary.bin` | 含 `\0` 字节 | E3 |
| `large.txt` | 2000 行；第 N 行 `LINE_NNNN`；N%10==0 则 `MARKER:NNNN` | B/G |

`test/e2e/gen-large.sh`：

```bash
#!/usr/bin/env bash
out="$1"; out="${out:-test/fixtures/large.txt}"
: > "$out"
for ((i=0;i<2000;i++)); do
  if (( i % 10 == 0 )); then printf 'MARKER:%04d\n' "$i" >> "$out";
  else printf 'LINE_%04d\n' "$i" >> "$out"; fi
done
```

---

## 17. 验收脚本结构与运行

```
test/
  fixtures/                # 第 16 节
    sample.md  sample.ts  plain.txt  unknown.xyz  binary.bin  large.txt
  e2e/
    run-acceptance.sh      # 总入口：source 各 scenario，汇总 PASS/FAIL
    lib/tmux.sh            # 第 14.3 节
    lib/assert.sh          # 第 14.3 节
    gen-large.sh           # 生成 large.txt
    scenarios/
      A-render.sh          # 15.1
      B-scroll.sh          # 15.2
      C-exit.sh            # 15.3
      D-resize.sh          # 15.4
      E-errors.sh          # 15.5
      F-nontty.sh          # 15.6
      G-large.sh           # 15.7
      H-markdown.sh        # 15.8
```

### 17.1 单个场景示例（`scenarios/B-scroll.sh`）

```bash
#!/usr/bin/env bash
set -u
source "$(dirname "$0")/../lib/tmux.sh"
source "$(dirname "$0")/../lib/assert.sh"

tmx_new 80 24
tmx_run "$BIN $FIX/large.txt"
wait_for "LINE_0000" 4

# B1 初始顶部
[ "$(tmx_row 2)" = "LINE_0000" ] && { echo "  ✓ B1 顶部"; PASS=$((PASS+1)); } \
  || { echo "  ✗ B1 顶部 got: $(tmx_row 2)"; FAIL=$((FAIL+1)); }

# B2 j×10 → MARKER:0010
for i in $(seq 10); do tmx_send j; done; sleep 0.2
[ "$(tmx_row 2)" = "MARKER:0010" ] && { echo "  ✓ B2 j 滚"; PASS=$((PASS+1)); } \
  || { echo "  ✗ B2 j 滚 got: $(tmx_row 2)"; FAIL=$((FAIL+1)); }

# B11 g 顶部
tmx_send g; sleep 0.2
[ "$(tmx_row 2)" = "LINE_0000" ] && { echo "  ✓ B11 g"; PASS=$((PASS+1)); } \
  || { echo "  ✗ B11 g got: $(tmx_row 2)"; FAIL=$((FAIL+1)); }

# B12 G 底部
tmx_send G; sleep 0.2
CAP=$(tmx_capp); say_contains "B12 G 末页" "LINE_1999"

tmx_kill
```

### 17.2 总入口（`run-acceptance.sh`）

```bash
#!/usr/bin/env bash
set -u
BIN="${BIN:-./preview}"
command -v tmux >/dev/null || { echo "需要 tmux"; exit 127; }
[ -x "$BIN" ] || { echo "先 build: pnpm build && bash scripts/build-binary.mjs"; exit 1; }
bash test/e2e/gen-large.sh
for s in test/e2e/scenarios/*.sh; do
  echo "== $(basename "$s") =="
  BIN="$BIN" bash "$s"
done
echo "PASS=$PASS FAIL=$FAIL"
[ "$FAIL" -eq 0 ]
```

### 17.3 运行

```bash
pnpm build && bash scripts/build-binary.mjs    # 产出 ./preview
BIN=./preview bash test/e2e/run-acceptance.sh      # 跑全部验收
# 或只测滚动：
BIN=./preview bash test/e2e/scenarios/B-scroll.sh
```

### 17.4 CI 集成

- GitHub Actions：`apt-get install -y tmux`（ubuntu runner 自带或一键装），`BIN=./preview bash test/e2e/run-acceptance.sh`。
- tmux 脱离会话非交互，CI 环境可直接跑，无需真实显示器。

---

## 18. 验收通过标准（Definition of Done）

全部满足即验收通过：

- [x] **L0**：`tsc --noEmit` 与 `vite build` 无错
- [x] **L2 全场景 PASS**：第 15 节 A–H 全部用例通过（`FAIL=0`，63/63）
- [x] **渲染**：md 标题/列表/表格/代码块/链接可见；代码文件 token 级 truecolor 高亮；未知扩展名无色
- [x] **滚动**：j/k/space/PageUp/PageDown/↑↓/Home/End/g/G 行为正确，`large.txt` 精确对齐
- [x] **退出**：q/Esc→0 并恢复 alt-screen；Ctrl+C→130
- [x] **错误**：无参→2；不存在/二进制/目录→1；多余参数→2
- [x] **非 TTY**：管道输出原始内容、退出 0
- [x] **resize**：放大/缩小后 header/footer 定位正确、不崩
- [x] **大文件**：2000 行可滚、启动 < 阈值（实测 ~0.4s）
- [x] **二进制**：`./preview README.md` 真实可用，SEA 产物同样可用（63/63 PASS）

---

## 附：关键源码引用

- CLI 运行时示例：`examples/basic/src/terminal.ts`、`examples/agent-console/src/terminal.ts`
- stdout renderer：`src/cli/headless-renderer.ts` + 文档 `/guide/cli-stdout-renderer`
- stdin driver：`src/cli/input.ts`（`createStdinDriver`、raw mode、Ctrl+C→onExit）
- `TVirtualMarkdown`：`src/vue/components/TVirtualMarkdown.ts`（scrollTop v-model、onKeydown、autoFocus、useTerminalNode/useRenderNode 契约）
- `TText`：`src/vue/components/TText.ts`（paint 范式：`terminal.write(text,{x,y,style})`）
- `Style` 类型：`src/core/types.ts`（`fg?: string` 接受 AnsiColorName 或 hex）
- composables 导出：`src/vue/index.ts`（`useRenderNode`/`useTerminalNode`/`useTerminal`/`useLayout`/`useVisibility`）
- markdown theme：`src/vue/markdown/theme.ts`（codeBlock/inlineCode 单色，无 highlighter）
- Node SEA：https://nodejs.org/api/single-executable-applications.html

---

## 19. 实现偏差与决策记录（实现后补充）

实现过程中相对原设计的关键偏差，均经源码验证与实际验收。

### 19.1 vue-tui 版本：0.0.7 → 1.1.3

`@simon_he/vue-tui@0.0.7` 的 `exports` 仅导出 `.`（单文件 `dist/index.js`），**没有** `/cli`、`/markdown`、`/vue` 子路径。设计文档调研基于 monorepo 源码，但发布包 0.0.7 是更早的精简版。升级到 **1.1.3** 后才具备完整子路径导出（`./cli`、`./markdown`、`./vue`、`./core`），所有 API（`createTerminalApp`/`createStdoutRenderer`/`createStdinDriver`/`installTerminalCleanup`/`TVirtualMarkdown`/`useTerminal` 等 composables）均经类型声明逐一确认。额外 peer 依赖（`beautiful-mermaid`/`bun-webgpu`/`katex`）均为 optional。

### 19.2 构建格式：ESM → CJS（SEA 兼容）

原设计 `formats:["es"]` + `mainFormat:"module"`。实测 Node 24.19.0 的经典 SEA 流程（`--experimental-sea-config` + `postject`）在运行时以 CJS 入口执行 blob（`embedderRunCjs`），忽略 `mainFormat:"module"`，导致 `import` 语法报 `Cannot use import statement outside a module`。且 `useCodeCache:true` 对 ESM 无法生成 code cache。

改为 **CJS 输出**（`formats:["cjs"]`、`fileName:terminal.cjs`、`mainFormat:"commonjs"`、`useCodeCache:true`）。CJS 经典 SEA 流程稳定可用。Vite 将 vue-tui（ESM）与 shiki grammars 静态打包为单文件 CJS，`inlineDynamicImports:true` 确保无多 chunk。Node 25.5+ 可用 `--build-sea` 一步完成（支持 ESM），本环境为 Node 24.19 故走 CJS。

### 19.3 shiki：`@shikijs/core` + 静态 grammar 导入

原设计 `createHighlighter`（shiki 主包）会动态 `import()` 各 grammar，Vite 拆成多 chunk，SEA 单文件无法加载。改用 **`@shikijs/core` 的 `createHighlighterCore`** + `@shikijs/engine-javascript`（JS regex 引擎）+ 仅静态导入 `lang.ts` 中映射的 grammar（`@shikijs/langs/<id>`）与 `@shikijs/themes/github-dark`。无动态 import，单文件 bundle 仅含用到的语言（约 40 种）。`token.color`（hex）直接喂 `Style.fg`，渲染器 `colorMode:"truecolor"` 输出 24-bit 色，已验收。

### 19.4 `v-model:scrollTop`（camelCase）

`v-model:scroll-top`（kebab）在 vnode.props 中存为 `"scroll-top"`，而 `TVirtualMarkdown`/`CodeView` 的受控检测 `hasControlledScrollTop()` 查 `"scrollTop"`（camel），导致父级驱动的 j/k/space/g/G 滚动失效。改为 SFC 模板中 `v-model:scrollTop`（camelCase），vnode.props key 匹配，受控滚动正常。

### 19.5 测试框架：Python pty + pyte（tmux 不可用）

环境无 tmux 且无 sudo。改用 **Python `pty` + `pyte`** 终端模拟器搭建等效 harness（真实 pty → `isTTY=true`、raw mode、alt-screen 均生效；`send-keys` 对应写字节；`capture-pane` 对应 pyte screen）。补丁 pyte 0.8.2 三处缺失能力：

- **`ESC[<n>S`（SU）/`ESC[<n>T`（SD）**：vue-tui 渲染器用 SU 做滚动优化，pyte 0.8.2 未实现 → 补 `scroll_up`/`scroll_down` 并注册到 CSI 派发表。
- **alt-screen（`ESC[?1049h/l`）**：pyte 0.8.2 无独立 alt buffer，alt 内容会覆盖主屏 → 补 `set_mode`/`reset_mode` 在 1049h/l 时切换 `self.buffer`（main↔alt）并保存/恢复光标。

主入口 `test/e2e/run_acceptance.py` 覆盖 A–H 全部场景（63 项断言）。`lib/tmux.sh`/`lib/assert.sh` 按 §14.3 保留，供有 tmux 的环境使用。

### 19.6 resize：PreviewApp 响应式尺寸

PreviewApp 从 `useTerminal()` 取 terminal，监听 `terminal.on("resize")` 响应式更新 cols/rows（不依赖静态 props），header/body/footer 随 resize 重排。已知 vue-tui 渲染器在 **shrink resize** 时清旧行（`CSI 13;1H EL…`）会因游标 clamp 到新末行而短暂擦掉 footer，下一帧重绘即恢复（D2 用例在触发一次重绘后断言）。

### 19.7 退出码

- q/Esc → `onQuit` → cleanup + `exit(0)`
- Ctrl+C（raw `\x03`）→ stdin driver `isUnhandledCtrlC` → `onExit` → `exit(130)`
- `installTerminalCleanup({ signalPolicy:"exit" })` 兜底信号场景
- cleanup 幂等（`cleaned` flag），顺序：driver.dispose → renderer.dispose → app.dispose

### 19.8 体积削减

成品 SEA binary 从 **128MB → 110MB**（省 18MB / 14%），E2E 63/63 无回归。

| 手段 | 效果 | 说明 |
|---|---|---|
| **strip（注入前）** | 120→103MB（省 17MB） | 移除 node 二进制的调试符号/符号表/.comment。**必须在 postject 注入之前**执行——注入后 strip 会段错误（postject 修改了 ELF section 布局） |
| **minify（esbuild）** | bundle 4.6→3.5MB（省 1MB） | `vite build` 设 `minify:"esbuild"`，压缩 JS bundle |
| **useCodeCache** | 0（仅加速启动） | CJS 模式可生成 code cache，不增体积 |
| ~~UPX 压缩~~ | 不兼容 | postject 修改 ELF 程序头偏移（`bad e_phoff`），UPX 无法打包已注入的二进制；注入前 UPX 则 postject 找不到 sentinel |
| ~~Node 25 `--build-sea`~~ | 127MB（更差） | 一步打包支持 ESM，但其产物 strip 无效（strip 后体积不变 127MB），不如 Node 24 经典 SEA + strip（110MB） |

**结论**：体积瓶颈是 Node 运行时本身（`.text` 段 ~105MB），JS bundle 仅 3.5MB。strip 是当前环境下最有效的削减手段。进一步削减需更换运行时（如 bun `--compile` 约 50MB，但环境无 bun）或用 musl/Alpine 链接的 node 构建。`scripts/build-binary.mjs` 已集成 strip 步骤。
