import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { builtinModules } from "module";

const nodeBuiltins = [
  ...builtinModules,
  ...builtinModules.map((m) => `node:${m}`),
];

export default defineConfig({
  plugins: [vue()],
  build: {
    target: "node18",
    minify: "esbuild",
    lib: {
      entry: "src/terminal.ts",
      formats: ["cjs"],
      fileName: () => "terminal.cjs",
    },
    rollupOptions: {
      external: nodeBuiltins,
      output: {
        inlineDynamicImports: true,
        manualChunks: undefined,
        exports: "named",
        banner: "#!/usr/bin/env node",
      },
    },
  },
});
