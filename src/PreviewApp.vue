<script setup lang="ts">
/**
 * PreviewApp —— header + body(md/code) + footer。
 * body 组件对外契约一致，父级不感知 md/code 差异。
 * onKey 只管 pager 习惯键（q/Esc/j/k/space/g/G），箭头/Page/Home/End/滚轮由 body 原生处理。
 * 尺寸从 terminal 实时读取（响应 resize），不依赖静态 props。
 */
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { TText } from "@simon_he/vue-tui";
import { TVirtualMarkdown } from "@simon_he/vue-tui/markdown";
import { useTerminal } from "@simon_he/vue-tui/vue";
import CodeView from "./CodeView.vue";
import type { HighlightToken } from "./highlight";

const props = defineProps<{
  content: string;
  /** null = markdown 模式；否则为 code 模式的 token 行。 */
  tokens: HighlightToken[][] | null;
  fileName: string;
  onQuit: () => void;
}>();

const { terminal, scheduler } = useTerminal();

// 尺寸响应式：初始取 terminal 当前尺寸，监听 resize 更新
const size = ref(terminal.size());
const resizeNonce = ref(0); // 强制 header/footer 在 resize 后重绘
let offResize: (() => void) | null = null;
onMounted(() => {
  offResize = terminal.on("resize", (e) => {
    size.value = { cols: e.cols, rows: e.rows };
    resizeNonce.value++;
    // 强制全量重绘：resize 后渲染器的"清旧行"可能擦掉末行(footer)，
    // invalidate 让下一帧重新输出所有行。
    scheduler.invalidate();
  });
});
onBeforeUnmount(() => offResize?.());

const cols = computed(() => size.value.cols);
const rows = computed(() => size.value.rows);
const bodyH = computed(() => Math.max(1, rows.value - 2));

const top = ref(0);
const isMd = computed(() => props.tokens === null);

const FOOTER = "q quit  ↑↓/jk scroll  space/pgdn  g/G top/bottom  Ctrl+C quit";

function onKey(e: { key: string; preventDefault: () => void }) {
  if (e.key === "q" || e.key === "Escape") {
    e.preventDefault();
    props.onQuit();
    return;
  }
  if (e.key === "j") {
    e.preventDefault();
    top.value += 1;
  } else if (e.key === "k") {
    e.preventDefault();
    top.value -= 1;
  } else if (e.key === " ") {
    e.preventDefault();
    top.value += bodyH.value;
  } else if (e.key === "g") {
    e.preventDefault();
    top.value = 0;
  } else if (e.key === "G") {
    e.preventDefault();
    top.value = 1e9; // 组件会 clamp 到 maxScrollTop
  }
}
</script>

<template>
  <TText :x="0" :y="0" :w="cols" :h="1" :value="fileName" :style="{ bold: true }" :deps-key="resizeNonce" />

  <TVirtualMarkdown
    v-if="isMd"
    :x="0"
    :y="1"
    :w="cols"
    :h="bodyH"
    :content="content"
    v-model:scrollTop="top"
    :auto-focus="true"
    @keydown="onKey"
  />

  <CodeView
    v-else
    :x="0"
    :y="1"
    :w="cols"
    :h="bodyH"
    :tokens="tokens!"
    v-model:scrollTop="top"
    :auto-focus="true"
    @keydown="onKey"
  />

  <TText :x="0" :y="rows - 1" :w="cols" :h="1" :value="FOOTER" :style="{ dim: true }" :deps-key="resizeNonce" />
</template>
