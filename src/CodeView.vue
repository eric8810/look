<script lang="ts">
/**
 * CodeView —— 自建高亮代码视图，复刻 TVirtualMarkdown 的滚动/键盘契约。
 *
 * 用 @simon_he/vue-tui/vue 的 composables 实现：
 *   useTerminal / useLayout / useVisibility / useTerminalNode / useRenderNode
 * paint 用 terminal.write(text, {x, y, style:{fg:hex}}) 逐 token 写色。
 *
 * 对外契约与 TVirtualMarkdown 一致：
 *   :x :y :w :h  :tokens  v-model:scrollTop  :auto-focus  @keydown
 */
import {
  defineComponent,
  h,
  ref,
  computed,
  watch,
  watchEffect,
  getCurrentInstance,
  type PropType,
} from "vue";
import {
  useTerminal,
  useLayout,
  useVisibility,
  useTerminalNode,
  useRenderNode,
  spaces,
  textCellWidth,
  sliceByCells,
} from "@simon_he/vue-tui/vue";
import type { HighlightToken } from "./highlight";

const TABSTOP = 2;

interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

function translateRect(r: Rect, dx: number, dy: number): Rect {
  return { x: r.x + dx, y: r.y + dy, w: r.w, h: r.h };
}

function intersectRect(a: Rect, b: Rect): Rect | null {
  const x0 = Math.max(a.x, b.x);
  const y0 = Math.max(a.y, b.y);
  const x1 = Math.min(a.x + a.w, b.x + b.w);
  const y1 = Math.min(a.y + a.h, b.y + b.h);
  if (x1 <= x0 || y1 <= y0) return null;
  return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
}

function normalizeCellRect(r: Rect): Rect {
  const x0 = Math.floor(r.x);
  const y0 = Math.floor(r.y);
  const x1 = Math.floor(r.x + r.w);
  const y1 = Math.floor(r.y + r.h);
  return { x: x0, y: y0, w: Math.max(0, x1 - x0), h: Math.max(0, y1 - y0) };
}

/** 将一个 token 行中的 Tab 展开为空格（tabstop=2），返回新 token 行。 */
function expandTabs(tokens: HighlightToken[]): HighlightToken[] {
  const out: HighlightToken[] = [];
  let col = 0;
  for (const tok of tokens) {
    const segs = tok.text.split("\t");
    let text = "";
    for (let i = 0; i < segs.length; i++) {
      text += segs[i];
      col += textCellWidth(segs[i]);
      if (i < segs.length - 1) {
        const n = TABSTOP - (col % TABSTOP);
        text += " ".repeat(n);
        col += n;
      }
    }
    out.push({ text, color: tok.color });
  }
  return out;
}

export default defineComponent({
  name: "CodeView",
  props: {
    x: { type: Number, required: true },
    y: { type: Number, required: true },
    w: { type: Number, required: true },
    h: { type: Number, required: true },
    tokens: { type: Array as PropType<HighlightToken[][]>, required: true },
    scrollTop: { type: Number, default: 0 },
    autoFocus: { type: Boolean, default: false },
    zIndex: { type: Number, default: 0 },
  },
  emits: {
    "update:scrollTop": (_v: number) => true,
    keydown: (_e: unknown) => true,
    focus: () => true,
    blur: () => true,
  },
  setup(props, { emit }) {
    const instance = getCurrentInstance();
    const { terminal, defaultStyle, events } = useTerminal();
    const layout = useLayout();
    const { visible, rootProps } = useVisibility();

    const internalScrollTop = ref(0);
    const tokensVersion = ref(0);

    // 预处理：展开 Tab（按行缓存）
    const processedLines = computed<HighlightToken[][]>(() => {
      // 依赖 tokensVersion 触发重算
      void tokensVersion.value;
      return (props.tokens as HighlightToken[][]).map(expandTabs);
    });

    // tokens 引用变化时刷新版本号
    watch(
      () => props.tokens,
      () => {
        tokensVersion.value++;
      },
      { immediate: true },
    );

    // ---- rect 计算（镜像 TVirtualMarkdown）----
    const fullRect = computed(() =>
      translateRect(
        { x: props.x, y: props.y, w: props.w, h: props.h },
        layout.originX,
        layout.originY,
      ),
    );
    const absRect = computed(() => {
      const t = fullRect.value;
      if (!layout.clipRect) return t;
      return intersectRect(t, layout.clipRect) ?? { x: 0, y: 0, w: 0, h: 0 };
    });
    function normalizedRect(): Rect {
      return normalizeCellRect(absRect.value);
    }

    function maxScrollTop(): number {
      const r = normalizedRect();
      return Math.max(0, processedLines.value.length - r.h);
    }

    function clamp(n: number, min: number, max: number): number {
      return Math.min(max, Math.max(min, n));
    }

    // ---- 受控 scrollTop（v-model:scrollTop）----
    function hasControlledScrollTop(): boolean {
      return Object.prototype.hasOwnProperty.call(instance?.vnode.props ?? {}, "scrollTop");
    }

    function setScrollTop(next: number, emitChange = true): void {
      const desired = Math.floor(Number(next) || 0);
      const clamped = clamp(desired, 0, maxScrollTop());
      const changed = internalScrollTop.value !== clamped;
      if (changed) internalScrollTop.value = clamped;
      if (emitChange && changed) {
        emit("update:scrollTop", clamped);
      }
    }

    function applyControlledScrollTop(next: number): void {
      const desired = Math.floor(Number(next) || 0);
      const clamped = clamp(desired, 0, maxScrollTop());
      if (internalScrollTop.value !== clamped) internalScrollTop.value = clamped;
      if (desired !== clamped) emit("update:scrollTop", clamped);
    }

    function reconcileScrollTop(): void {
      const desired = hasControlledScrollTop()
        ? Math.floor(Number(props.scrollTop) || 0)
        : internalScrollTop.value;
      const clamped = clamp(desired, 0, maxScrollTop());
      if (internalScrollTop.value === clamped) return;
      internalScrollTop.value = clamped;
      if (hasControlledScrollTop() && desired !== clamped) {
        emit("update:scrollTop", clamped);
      }
    }

    watch(
      () => props.scrollTop,
      () => {
        if (!hasControlledScrollTop()) return;
        applyControlledScrollTop(props.scrollTop);
      },
    );

    watch(
      [() => props.h, () => absRect.value.h, () => absRect.value.y, () => processedLines.value],
      () => reconcileScrollTop(),
      { immediate: true },
    );

    // ---- keydown：先抛父级，再处理原生滚动键 ----
    function onKeydown(event: { key: string; preventDefault: () => void }) {
      emit("keydown", event);
      const page = Math.max(1, normalizedRect().h);
      if (event.key === "ArrowUp") {
        event.preventDefault();
        setScrollTop(internalScrollTop.value - 1);
        return;
      }
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setScrollTop(internalScrollTop.value + 1);
        return;
      }
      if (event.key === "PageUp") {
        event.preventDefault();
        setScrollTop(internalScrollTop.value - page);
        return;
      }
      if (event.key === "PageDown") {
        event.preventDefault();
        setScrollTop(internalScrollTop.value + page);
        return;
      }
      if (event.key === "Home") {
        event.preventDefault();
        setScrollTop(0);
        return;
      }
      if (event.key === "End") {
        event.preventDefault();
        setScrollTop(maxScrollTop());
      }
    }

    // ---- 事件节点（focusable，接收键盘/滚轮）----
    const eventNode = useTerminalNode(() => ({
      rect: normalizedRect(),
      zIndex: props.zIndex,
      visible: visible.value,
      focusable: true,
      selectable: false,
      handlers: {
        keydown: onKeydown as (e: unknown) => void,
        wheel: (event: { deltaY?: number }) => {
          const dy = event.deltaY ?? 0;
          if (!dy) return;
          setScrollTop(internalScrollTop.value + (dy > 0 ? 1 : -1));
        },
        focus: () => emit("focus"),
        blur: () => emit("blur"),
      },
    }));

    // ---- autoFocus ----
    watchEffect(() => {
      if (!props.autoFocus || !visible.value) return;
      const manager = events.value;
      const nodeId = eventNode.id.value;
      if (!nodeId || !manager) return;
      if (manager.getFocused() === nodeId) return;
      manager.focus(nodeId);
    });

    // ---- 渲染节点（paint）----
    useRenderNode(() => ({
      zIndex: props.zIndex,
      rect: visible.value ? normalizedRect() : { x: 0, y: 0, w: 0, h: 0 },
      deps: [
        visible.value,
        absRect.value,
        fullRect.value,
        internalScrollTop.value,
        tokensVersion.value,
        defaultStyle.value,
      ],
      paint: (dirtyRows) => {
        if (!visible.value) return;
        const r = normalizedRect();
        if (r.w <= 0 || r.h <= 0) return;
        const baseStyle = defaultStyle.value;
        const lines = processedLines.value;
        const top = internalScrollTop.value;

        const paintRow = (y: number) => {
          if (y < r.y || y >= r.y + r.h) return;
          const li = top + (y - r.y);
          const lineTokens = lines[li];

          if (!lineTokens || lineTokens.length === 0) {
            terminal.write(spaces(r.w), { x: r.x, y, style: baseStyle });
            return;
          }

          let cellX = 0;
          for (const tok of lineTokens) {
            if (cellX >= r.w) break;
            const remaining = r.w - cellX;
            if (remaining <= 0) break;
            const sliced = sliceByCells(tok.text, remaining);
            if (sliced) {
              terminal.write(sliced, {
                x: r.x + cellX,
                y,
                style: tok.color ? { fg: tok.color } : baseStyle,
              });
              cellX += textCellWidth(sliced);
            }
            // 若该 token 被截断（cellX 已达 w），跳出
            if (textCellWidth(tok.text) > remaining) break;
          }
          if (cellX < r.w) {
            terminal.write(spaces(r.w - cellX), { x: r.x + cellX, y, style: baseStyle });
          }
        };

        if (dirtyRows?.length) {
          for (const y of dirtyRows) paintRow(y);
          return;
        }
        for (let y = r.y; y < r.y + r.h; y++) paintRow(y);
      },
    }));

    return () => h("span", rootProps);
  },
});
</script>
