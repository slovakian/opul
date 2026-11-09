import { defineConfig, devices } from "@playwright/test";

const port = Number(process.env.OPUL_TEST_PORT ?? 1420);
const baseURL = `http://localhost:${port}`;

/** Benchmarks run one at a time so they don't compete for the CPU. */
export default defineConfig({
  projects: [
    {
      name: "webkit",
      use: {
        ...devices["Desktop Safari"],
        viewport: { height: 800, width: 1220 },
      },
    },
  ],
  reporter: "list",
  testDir: ".",
  testMatch: "*.bench.ts",
  use: { baseURL, reducedMotion: "reduce" },
  webServer: {
    command: `vp run dev --port ${port}`,
    reuseExistingServer: !process.env.CI,
    url: baseURL,
  },
  workers: 1,
});
