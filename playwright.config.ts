import { defineConfig, devices } from "@playwright/test";

const port = Number(process.env.OPUL_TEST_PORT ?? 1420);

if (!Number.isInteger(port) || port < 1 || port > 65_535) {
  throw new Error("OPUL_TEST_PORT must be a valid TCP port.");
}

const baseURL = `http://localhost:${port}`;

export default defineConfig({
  fullyParallel: true,
  projects: [
    {
      name: "webkit",
      use: {
        ...devices["Desktop Safari"],
        viewport: { height: 800, width: 1220 },
      },
    },
  ],
  testDir: "./tests/ui",
  use: {
    baseURL,
    reducedMotion: "reduce",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  webServer: {
    command: `npm run dev -- --port ${port}`,
    reuseExistingServer: !process.env.CI && !process.env.OPUL_TEST_PORT,
    url: baseURL,
  },
});
