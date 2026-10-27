import { defineConfig } from "oxfmt";
import ultracite from "ultracite/oxfmt";

export default defineConfig({
  ...ultracite,
  ignorePatterns: [
    ...(ultracite.ignorePatterns ?? []),
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
});
