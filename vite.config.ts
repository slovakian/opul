import { fileURLToPath, URL } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import ultraciteFmt from "ultracite/oxfmt";
import core from "ultracite/oxlint/core";
import playwright from "ultracite/oxlint/playwright";
import ultraciteReact from "ultracite/oxlint/react";
import { defineConfig, lazyPlugins } from "vite-plus";

export default defineConfig({
  build: {
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              name: "emoji-data",
              test: /frontend\/assets\/emoji\/data\.json$/,
            },
            {
              name: "react",
              test: /node_modules\/(react|react-dom|scheduler)\//,
            },
          ],
        },
      },
    },
  },
  clearScreen: false,
  fmt: {
    ...ultraciteFmt,
    ignorePatterns: [
      ...(ultraciteFmt.ignorePatterns ?? []),
      "**/.agent/**",
      "**/.agents/**",
      "**/.claude/**",
      "**/.codex/**",
      "**/.continue/**",
      "**/.cursor/**",
      "**/.gemini/**",
      "**/.opencode/**",
      "**/.pi/**",
      "**/.roo/**",
      "**/.windsurf/**",
      "**/.vite-hooks/**",
      "tools/oxlint/anti-slop/**",
      "src-tauri/**",
      "src/**",
      "target/**",
      "frontend/assets/**",
      "**/*.toml",
      "Cargo.lock",
      "skills-lock.json",
    ],
  },
  lint: {
    env: core.env,
    extends: [core, ultraciteReact, playwright] as any,
    ignorePatterns: [
      ...(core.ignorePatterns ?? []),
      "**/.agent/**",
      "**/.agents/**",
      "**/.claude/**",
      "**/.codex/**",
      "**/.continue/**",
      "**/.cursor/**",
      "**/.gemini/**",
      "**/.opencode/**",
      "**/.pi/**",
      "**/.roo/**",
      "**/.windsurf/**",
      "**/.vite-hooks/**",
      "tools/oxlint/anti-slop/**",
      "src-tauri/**",
      "src/**",
      "target/**",
      "frontend/assets/**",
    ],
    jsPlugins: [
      { name: "vite-plus", specifier: "vite-plus/oxlint-plugin" },
      { name: "anti-slop", specifier: "./tools/oxlint/anti-slop/index.ts" },
    ],
    rules: {
      "anti-slop/no-array-filter-map": "error",
      "anti-slop/no-chained-type-assertions": "error",
      "anti-slop/no-conditional-empty-object-spread": "error",
      "anti-slop/no-known-value-widening": "error",
      "anti-slop/no-module-mocking": "error",
      "anti-slop/no-object-parameters": "error",
      "anti-slop/no-reduce-accumulator-copy": "error",
      "anti-slop/no-reflect-apply": "error",
      "anti-slop/no-reflect-get": "error",
      "anti-slop/no-runtime-typeof": "error",
      "anti-slop/no-shape-in-symbol-names": "error",
      "anti-slop/no-unknown-parameters": "error",
      "anti-slop/no-unknown-returns": "error",
      "anti-slop/no-unknown-type-aliases": "error",
      "anti-slop/no-unsafe-dictionary-type": "error",
      "anti-slop/no-widen-then-assert": "error",
      "anti-slop/require-readable-spacing": "error",
      "anti-slop/require-safety-comment-for-type-assertion": "error",
      "oxc/no-accumulating-spread": "error",
      "vite-plus/prefer-vite-plus-imports": "error",
    },
  },
  plugins: lazyPlugins(() => [react(), tailwindcss()]),
  resolve: {
    alias: { "@": fileURLToPath(new URL("frontend", import.meta.url)) },
  },
  server: { port: 1420, strictPort: true },
  staged: {
    "*": "vp fmt",
  },
  test: {
    clearMocks: false,
    exclude: ["tests/ui/**", "**/node_modules/**"],
    passWithNoTests: true,
  },
});
