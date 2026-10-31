import { fileURLToPath, URL } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import ultraciteFmt from "ultracite/oxfmt";
import core from "ultracite/oxlint/core";
import playwright from "ultracite/oxlint/playwright";
import ultraciteReact from "ultracite/oxlint/react";
import { defineConfig, lazyPlugins } from "vite-plus";

const sharedIgnorePatterns = [
  // Agent skills and AI tooling
  "**/.agent/**",
  "**/.agents/**",
  "**/.aide/**",
  "**/.aider/**",
  "**/.antigravity/**",
  "**/.claude/**",
  "**/.cline/**",
  "**/.codex/**",
  "**/.continue/**",
  "**/.copilot/**",
  "**/.cursor/**",
  "**/.gemini/**",
  "**/.opencode/**",
  "**/.pi/**",
  "**/.roo/**",
  "**/.windsurf/**",
  "skills-lock.json",
  "AGENTS.md",
  // Internal tooling and vendored plugins
  "**/.vite-hooks/**",
  "tools/oxlint/anti-slop/**",
  // Rust / Tauri backend sources, builds, and manifests
  "src-tauri/**",
  "src/**",
  "target/**",
  "**/*.toml",
  "Cargo.lock",
  // Generated datasets and static media assets
  "frontend/assets/**",
  "assets/**",
  // Test results, coverage, reports, and build artifacts
  "test-results/**",
  "playwright-report/**",
  "blob-report/**",
  ".vitest/**",
  "**/.build/**",
  "build/**",
];

export default defineConfig({
  build: {
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              name: "emoji-data",
              test: /frontend\/assets\/emoji\/data\.json$/u,
            },
            {
              name: "react",
              test: /node_modules\/(?:react|react-dom|scheduler)\//u,
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
      ...sharedIgnorePatterns,
    ],
  },
  lint: {
    env: core.env,
    // oxlint-disable-next-line typescript/no-explicit-any, anti-slop/require-safety-comment-for-type-assertion
    extends: [core, ultraciteReact, playwright] as any,
    ignorePatterns: [...(core.ignorePatterns ?? []), ...sharedIgnorePatterns],
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
    "frontend/**": "vp fmt",
  },
  test: {
    clearMocks: false,
    exclude: ["tests/ui/**", "**/node_modules/**"],
    passWithNoTests: true,
  },
});
