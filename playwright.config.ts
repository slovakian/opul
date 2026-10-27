import { defineConfig, devices } from "@playwright/test";

const port = Number(process.env.OPUL_TEST_PORT ?? 1420);
if (!Number.isInteger(port) || port < 1 || port > 65535)
  throw new Error("OPUL_TEST_PORT must be a valid TCP port.");
const baseURL = `http://localhost:${port}`;

export default defineConfig({
  testDir: "./tests/ui",
  fullyParallel: true,
  use: {
    baseURL,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    reducedMotion: "reduce",
  },
  projects: [
    {
      name: "webkit",
      use: {
        ...devices["Desktop Safari"],
        viewport: { width: 1220, height: 800 },
      },
    },
  ],
  webServer: {
    command: `npm run dev -- --port ${port}`,
    url: baseURL,
    reuseExistingServer: !process.env.CI && !process.env.OPUL_TEST_PORT,
  },
});
