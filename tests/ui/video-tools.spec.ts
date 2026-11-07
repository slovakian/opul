import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import type { VideoToolsSnapshot } from "../../frontend/lib/video-tools";
import { mockDesktop } from "./fixture";

const initial: VideoToolsSnapshot = {
  keep_up_to_date: true,
  published_size: null,
  sequence: 1,
  state: { kind: "not_set_up" },
};

const openTools = async (page: Page) => {
  await page.getByRole("button", { exact: true, name: "Settings" }).click();
  const dialog = page.getByRole("dialog", { exact: true, name: "Settings" });
  await dialog.getByRole("button", { exact: true, name: "Tools" }).click();

  return dialog;
};

const push = async (page: Page, snapshot: VideoToolsSnapshot) => {
  await page.evaluate((video) => {
    sessionStorage.setItem("test-video-tools", JSON.stringify(video));
    window.dispatchEvent(new Event("test:video-tools"));
  }, snapshot);
};

test("Settings requires consent and saves the tools preference immediately", async ({
  page,
}) => {
  await mockDesktop(page, { persist: true });
  await page.goto("/");
  const dialog = await openTools(page);
  await expect(
    dialog.getByRole("button", {
      exact: true,
      name: "Download video tools (about 23 MB)",
    })
  ).toBeVisible();
  await expect(
    dialog.getByRole("switch", { name: "Keep video tools up to date" })
  ).toBeChecked();
  await expect(
    dialog.getByRole("button", { name: "Save changes" })
  ).toHaveCount(0);
  await expect(
    dialog.getByRole("button", { name: "Cancel setup" })
  ).toHaveCount(0);
  await dialog
    .getByRole("switch", { name: "Keep video tools up to date" })
    .click();
  await expect(
    dialog.getByRole("switch", { name: "Keep video tools up to date" })
  ).not.toBeChecked();
  await dialog.getByRole("button", { exact: true, name: "Done" }).click();
  await page.reload();
  const reopened = await openTools(page);
  await expect(
    reopened.getByRole("switch", { name: "Keep video tools up to date" })
  ).not.toBeChecked();
  await expect(
    reopened.getByRole("button", { name: /Download video tools/u })
  ).toBeVisible();

  const audit = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .analyze();

  expect(audit.violations).toEqual([]);
});

test("turning off persists and explicit Settings consent starts setup", async ({
  page,
}) => {
  await mockDesktop(page, { persist: true });
  await page.goto("/");
  const dialog = await openTools(page);
  await dialog
    .getByRole("button", { name: "Don't offer video tools again" })
    .click();
  await expect(
    dialog.getByText("Video tools are turned off. Opul won't offer them.")
  ).toBeVisible();
  await dialog.getByRole("button", { exact: true, name: "Done" }).click();
  await page.reload();
  const reopened = await openTools(page);
  await expect(
    reopened.getByText("Video tools are turned off. Opul won't offer them.")
  ).toBeVisible();
  await reopened.getByRole("button", { name: /Download video tools/u }).click();
  await expect(
    reopened.getByText("Setting up Python", { exact: true })
  ).toBeVisible();
  await expect(
    reopened.getByText("Setting up yt-dlp", { exact: true })
  ).toBeVisible();
  await reopened
    .getByRole("button", { exact: true, name: "Cancel setup" })
    .click();
  await expect(
    reopened.getByRole("button", { name: /Download video tools/u })
  ).toBeVisible();
});

test("pushed progress, published size and Ready versions replace the initial state", async ({
  page,
}) => {
  await mockDesktop(page, {
    videoTools: { ...initial, published_size: 20_143_107 },
  });
  await page.goto("/");
  const dialog = await openTools(page);
  await expect(
    dialog.getByRole("button", {
      exact: true,
      name: "Download video tools (20.1 MB)",
    })
  ).toBeVisible();
  await dialog.getByRole("button", { name: /Download video tools/u }).click();
  await push(page, {
    ...initial,
    sequence: 10,
    state: {
      kind: "setting_up",
      steps: [
        {
          component: "python",
          downloaded_bytes: 17_143_107,
          status: "done",
          total_bytes: 17_143_107,
        },
        {
          component: "yt_dlp",
          downloaded_bytes: 1_000_000,
          status: "downloading",
          total_bytes: 3_000_000,
        },
      ],
    },
  });
  await expect(dialog.getByText("Done", { exact: true }).first()).toBeVisible();
  await expect(
    dialog.getByRole("progressbar", { name: "Setting up Python progress" })
  ).toHaveAttribute("aria-valuenow", "100");

  const audit = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .analyze();

  expect(audit.violations).toEqual([]);
  await page.screenshot({ path: "test-results/video-tools-progress.png" });
  await push(page, {
    ...initial,
    sequence: 11,
    state: {
      installed: {
        python_version: "3.13.18",
        runtime: 1,
        yt_dlp: "2026.10.26",
      },
      kind: "ready",
      last_checked: null,
      update_flag: { kind: "none" },
    },
  });
  await expect(dialog.getByText("Ready", { exact: true })).toBeVisible();
  await expect(
    dialog.getByText(/Python runtime 1 · Python 3\.13\.18/u)
  ).toBeVisible();
  await expect(dialog.getByText(/yt-dlp 2026\.10\.26/u)).toBeVisible();
  await expect(
    dialog.getByRole("button", { exact: true, name: "Cancel setup" })
  ).toHaveCount(0);
  // A delayed command response/event must not revert a newer pushed snapshot.
  await push(page, { ...initial, sequence: 2 });
  await expect(dialog.getByText("Ready", { exact: true })).toBeVisible();
});

test("failed setup offers retry and reports preference persistence errors", async ({
  page,
}) => {
  await mockDesktop(page, {
    failVideoToolsPreference: true,
    videoTools: {
      ...initial,
      state: {
        details: "The runtime signature did not verify.",
        kind: "not_working",
        reason: "verification",
      },
    },
  });
  await page.goto("/");
  const dialog = await openTools(page);
  await expect(
    dialog.getByText("The download couldn't be verified.")
  ).toBeVisible();
  await dialog
    .getByRole("switch", { name: "Keep video tools up to date" })
    .click();
  await expect(dialog.getByRole("alert")).toHaveText(
    "Couldn't save the video tools preference."
  );
  await expect(
    dialog.getByRole("switch", { name: "Keep video tools up to date" })
  ).toBeChecked();
  await dialog.getByRole("button", { exact: true, name: "Try again" }).click();
  await expect(
    dialog.getByText("Setting up Python", { exact: true })
  ).toBeVisible();
});

for (const update of [
  { kind: "available", version: "2026.11.01" },
  { kind: "updating", progress: 40, version: "2026.11.01" },
  { kind: "failed", version: "2026.11.01" },
  { kind: "waiting_for_runtime", version: "2026.11.01" },
] satisfies Extract<
  VideoToolsSnapshot["state"],
  { kind: "ready" }
>["update_flag"][]) {
  test(`Ready renders the exclusive ${update.kind} flag`, async ({ page }) => {
    await mockDesktop(page, {
      videoTools: {
        ...initial,
        state: {
          installed: {
            python_version: "3.13.18",
            runtime: 1,
            yt_dlp: "2026.10.26",
          },
          kind: "ready",
          last_checked: 1_793_000_000,
          update_flag: update,
        },
      },
    });
    await page.goto("/");
    const dialog = await openTools(page);
    await expect(
      dialog.getByRole("status").filter({ hasText: "2026.11.01" })
    ).toHaveCount(1);
    await expect(dialog.getByText("Ready", { exact: true })).toBeVisible();
    await expect(dialog.getByText(/Last checked/u)).toBeVisible();

    const audit = await new AxeBuilder({ page })
      .include('[role="dialog"]')
      .analyze();

    expect(audit.violations).toEqual([]);
  });
}
