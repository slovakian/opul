import { defineConfig, devices } from "@playwright/test";

const port = Number(process.env.OPUL_TEST_PORT ?? 1420);
const baseURL = `http://127.0.0.1:${port}`;

export default defineConfig({
  fullyParallel: false,
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        colorScheme: "dark",
        viewport: { height: 820, width: 1280 },
      },
    },
  ],
  testDir: "./tests/ui",
  testMatch: "readme-hero.spec.ts",
  use: {
    baseURL,
    reducedMotion: "reduce",
  },
  webServer: {
    command: `vp run dev --port ${port} --host 127.0.0.1`,
    reuseExistingServer: !process.env.CI,
    url: baseURL,
  },
});
