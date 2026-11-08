import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import type { VideoToolsSnapshot } from "../../frontend/lib/video-tools";
import { mockDesktop, snapshot as librarySnapshot } from "./fixture";

const initial: VideoToolsSnapshot = {
  keep_up_to_date: true,
  nightly_available: false,
  published_size: null,
  sequence: 1,
  state: { kind: "not_set_up" },
  update_available: false,
  use_nightly: false,
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
      name: "Download ~23 MB",
    })
  ).toBeVisible();
  await expect(
    dialog.getByRole("switch", { name: "Keep up to date" })
  ).toBeChecked();
  await expect(
    dialog.getByRole("button", { name: "Save changes" })
  ).toHaveCount(0);
  await expect(
    dialog.getByRole("button", { name: "Cancel setup" })
  ).toHaveCount(0);
  await dialog.getByRole("switch", { name: "Keep up to date" }).click();
  await expect(
    dialog.getByRole("switch", { name: "Keep up to date" })
  ).not.toBeChecked();
  await dialog.getByRole("button", { exact: true, name: "Done" }).click();
  await page.reload();
  const reopened = await openTools(page);
  await expect(
    reopened.getByRole("switch", { name: "Keep up to date" })
  ).not.toBeChecked();
  await expect(
    reopened.getByRole("button", { name: /^Download ~?[\d.]+ MB$/u })
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
  await dialog.getByRole("button", { name: "Don't offer again" }).click();
  await expect(dialog.getByText("Turned off", { exact: true })).toBeVisible();
  await dialog.getByRole("button", { exact: true, name: "Done" }).click();
  await page.reload();
  const reopened = await openTools(page);
  await expect(reopened.getByText("Turned off", { exact: true })).toBeVisible();
  await reopened
    .getByRole("button", { name: /^Download ~?[\d.]+ MB$/u })
    .click();
  await expect(
    reopened.getByRole("progressbar", { name: "Setting up Python progress" })
  ).toBeVisible();
  await expect(
    reopened.getByRole("progressbar", { name: "Setting up yt-dlp progress" })
  ).toBeVisible();
  await reopened
    .getByRole("button", { exact: true, name: "Cancel setup" })
    .click();
  await expect(
    reopened.getByRole("button", { name: /^Download ~?[\d.]+ MB$/u })
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
      name: "Download 20.1 MB",
    })
  ).toBeVisible();
  await dialog.getByRole("button", { name: /^Download ~?[\d.]+ MB$/u }).click();
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
    dialog.getByText("3.13.18 · runtime 1", { exact: true })
  ).toBeVisible();
  await expect(dialog.getByText(/^2026\.10\.26/u)).toBeVisible();
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
    dialog.getByText("Couldn't verify", { exact: true })
  ).toBeVisible();
  await dialog.getByRole("switch", { name: "Keep up to date" }).click();
  await expect(dialog.getByRole("alert")).toHaveText(
    "Couldn't save the video tools preference."
  );
  await expect(
    dialog.getByRole("switch", { name: "Keep up to date" })
  ).toBeChecked();
  await dialog.getByRole("button", { exact: true, name: "Try again" }).click();
  await expect(
    dialog.getByRole("progressbar", { name: "Setting up Python progress" })
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
    await expect(dialog.getByText(/· checked /u)).toBeVisible();

    const audit = await new AxeBuilder({ page })
      .include('[role="dialog"]')
      .analyze();

    expect(audit.violations).toEqual([]);
  });
}

test("offline setup keeps its progress, resumes on a pushed snapshot and can be cancelled", async ({
  page,
}) => {
  const waiting: VideoToolsSnapshot = {
    ...initial,
    state: {
      kind: "setting_up",
      steps: [
        {
          component: "python",
          downloaded_bytes: 10_000_000,
          status: "waiting_for_internet",
          total_bytes: 20_000_000,
        },
        {
          component: "yt_dlp",
          downloaded_bytes: 0,
          status: "pending",
          total_bytes: null,
        },
      ],
    },
  };

  await mockDesktop(page, { videoTools: waiting });
  await page.goto("/");
  const dialog = await openTools(page);
  await expect(
    dialog.getByText("Waiting for internet…", { exact: true })
  ).toBeVisible();

  const progress = dialog.getByRole("progressbar", {
    name: "Setting up Python progress",
  });

  await expect(progress).toHaveAttribute("aria-valuenow", "50");
  await expect(dialog.getByRole("button", { name: "Try again" })).toHaveCount(
    0
  );
  await push(page, {
    ...initial,
    sequence: 10,
    state: {
      kind: "setting_up",
      steps: [
        {
          component: "python",
          downloaded_bytes: 15_000_000,
          status: "downloading",
          total_bytes: 20_000_000,
        },
        {
          component: "yt_dlp",
          downloaded_bytes: 0,
          status: "pending",
          total_bytes: null,
        },
      ],
    },
  });
  await expect(dialog.getByText("Waiting for internet…")).toHaveCount(0);
  await expect(progress).toHaveAttribute("aria-valuenow", "75");
  await push(page, { ...waiting, sequence: 11 });

  const audit = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .analyze();

  expect(audit.violations).toEqual([]);
  await page.screenshot({ path: "test-results/video-tools-offline.png" });
  await dialog
    .getByRole("button", { exact: true, name: "Cancel setup" })
    .click();
  await expect(
    dialog.getByRole("button", { name: /^Download ~?[\d.]+ MB$/u })
  ).toBeVisible();
});

for (const failure of [
  {
    details: "HTTP 503 after three retries.",
    message: "Download failed",
    reason: "download",
  },
  {
    details: "The SHA-256 checksum did not match.",
    message: "Couldn't verify",
    reason: "verification",
  },
  {
    details: "bin/python3: Permission denied.",
    message: "Stopped working",
    reason: "stopped_working",
  },
] as const) {
  test(`${failure.reason} offers recovery and copies the error, versions and chip`, async ({
    page,
  }) => {
    await page.addInitScript(() => {
      Object.defineProperty(navigator, "clipboard", {
        value: {
          writeText: (value: string) => {
            sessionStorage.setItem("test-copied-tool-details", value);

            return Promise.resolve();
          },
        },
      });
    });
    await mockDesktop(page, {
      videoTools: {
        ...initial,
        state: {
          details: failure.details,
          kind: "not_working",
          reason: failure.reason,
        },
      },
    });
    await page.goto("/");
    const dialog = await openTools(page);
    await expect(
      dialog.getByText(failure.message, { exact: true })
    ).toBeVisible();
    await dialog.getByText("Details", { exact: true }).click();
    await expect(
      dialog.getByText(failure.details, { exact: true })
    ).toBeVisible();
    await dialog
      .getByRole("button", { exact: true, name: "Copy details" })
      .click();
    await expect(
      dialog.getByRole("button", { exact: true, name: "Copied" })
    ).toBeVisible();

    const copied = await page.evaluate(() =>
      sessionStorage.getItem("test-copied-tool-details")
    );

    expect(copied).toContain(failure.details);
    expect(copied).toContain("Python runtime 1");
    expect(copied).toContain("yt-dlp 2026.10.26");
    expect(copied).toContain("chip arm64");
    await dialog
      .getByRole("button", { exact: true, name: "Try again" })
      .click();
    await expect(
      dialog.getByRole("progressbar", { name: "Setting up Python progress" })
    ).toBeVisible();
  });
}

test("manual check and Update work in Ready with one live status line", async ({
  page,
}) => {
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
        last_checked: null,
        update_flag: { kind: "none" },
      },
    },
  });
  await page.goto("/");
  const dialog = await openTools(page);
  await dialog
    .getByRole("button", { exact: true, name: "Check for updates" })
    .click();
  await expect(dialog.getByText("2026.11.01 is available.")).toBeVisible();
  await expect(dialog.getByText(/· checked /u)).toBeVisible();
  await dialog.getByRole("button", { exact: true, name: "Update" }).click();
  await expect(
    dialog.getByText("Updating to 2026.11.01…", { exact: true })
  ).toBeVisible();
  await expect(dialog.getByText("Ready", { exact: true })).toBeVisible();
  await expect(
    dialog.getByRole("status").filter({ hasText: "2026.11.01" })
  ).toHaveCount(1);
  await expect(
    dialog.getByRole("button", { exact: true, name: "Update" })
  ).toHaveCount(0);
  await expect(
    dialog.getByRole("button", { name: "Check for updates" })
  ).toBeDisabled();
  const current = await page.evaluate(() => window.testVideoTools.snapshot());

  if (current.state.kind !== "ready") {
    throw new Error("Update must remain Ready.");
  }

  await push(page, {
    ...current,
    sequence: current.sequence + 1,
    state: {
      ...current.state,
      installed: { ...current.state.installed, yt_dlp: "2026.11.01" },
      update_flag: { kind: "none" },
    },
  });
  await expect(dialog.getByText(/^2026\.11\.01 · checked/u)).toBeVisible();
  await expect(
    dialog.getByRole("status").filter({ hasText: "Updating to" })
  ).toHaveCount(0);

  const audit = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .analyze();

  expect(audit.violations).toEqual([]);
});

for (const turnOff of [false, true]) {
  test(`Remove confirms the count and persists turn off ${turnOff}`, async ({
    page,
  }) => {
    const library = structuredClone(librarySnapshot);
    const [base] = library.workspaces[0].downloads;
    library.workspaces[0].downloads = [
      {
        ...base,
        id: "video-active",
        name: "Active video",
        needs_video_tools: true,
        source: turnOff ? "Guided" : "VideoPage",
        state: "Fetching",
      },
      {
        ...base,
        id: "video-waiting",
        name: "Waiting video",
        source: "VideoPage",
        state: "WaitingForVideoTools",
      },
      {
        ...base,
        id: "video-finished",
        name: "Finished video",
        source: "VideoPage",
        state: "Finished",
      },
      {
        ...base,
        id: "direct-active",
        name: "Direct file",
        source: "Direct",
        state: "Fetching",
      },
    ];
    await mockDesktop(page, {
      persist: true,
      snapshot: library,
      videoTools: {
        ...initial,
        state: turnOff
          ? {
              details: "Missing tools",
              kind: "not_working",
              reason: "stopped_working",
            }
          : {
              installed: {
                python_version: "3.13.18",
                runtime: 1,
                yt_dlp: "2026.10.26",
              },
              kind: "ready",
              last_checked: null,
              update_flag: { kind: "none" },
            },
      },
    });
    await page.goto("/");
    const settings = await openTools(page);
    await settings
      .getByRole("button", { exact: true, name: "Remove…" })
      .click();

    const confirmation = page.getByRole("dialog", {
      exact: true,
      name: "Remove video tools?",
    });

    await expect(confirmation).toContainText(
      "2 video downloads are in progress"
    );

    const audit = await new AxeBuilder({ page })
      .include('[role="dialog"]')
      .analyze();

    expect(audit.violations).toEqual([]);
    await confirmation
      .getByRole("button", { exact: true, name: "Cancel" })
      .click();
    await expect(
      settings.getByRole("button", { exact: true, name: "Remove…" })
    ).toBeVisible();
    await settings
      .getByRole("button", { exact: true, name: "Remove…" })
      .click();

    if (turnOff) {
      await confirmation
        .getByRole("checkbox", { name: "Don't offer video tools again" })
        .check();
    }

    await confirmation
      .getByRole("button", { exact: true, name: "Remove" })
      .click();
    await expect(confirmation).toHaveCount(0);
    await expect(
      settings.getByRole("button", { name: /^Download ~?[\d.]+ MB$/u })
    ).toBeVisible();

    if (turnOff) {
      await expect(
        settings.getByText("Turned off", { exact: true })
      ).toBeVisible();
    }

    await settings.getByRole("button", { exact: true, name: "Done" }).click();
    await page.reload();
    const reopened = await openTools(page);
    await expect(
      reopened.getByRole("button", { name: /^Download ~?[\d.]+ MB$/u })
    ).toBeVisible();
    await reopened.getByRole("button", { exact: true, name: "Done" }).click();
    await expect(
      page
        .getByText(turnOff ? "Needs video tools" : "Waiting for video tools…", {
          exact: true,
        })
        .first()
    ).toBeVisible();
  });
}

for (const nightly of [false, true]) {
  test(`yt-dlp nightly switch visibility for nightly ${nightly}`, async ({
    page,
  }) => {
    await mockDesktop(page, {
      persist: true,
      videoTools: {
        ...initial,
        nightly_available: nightly,
        use_nightly: false,
      },
    });
    await page.goto("/");
    const dialog = await openTools(page);

    const control = dialog.getByRole("switch", {
      name: "Nightly yt-dlp",
    });

    if (!nightly) {
      await expect(control).toHaveCount(0);

      return;
    }

    await expect(control).not.toBeChecked();
    await control.click();
    await expect(control).toBeChecked();
    await dialog.getByRole("button", { exact: true, name: "Done" }).click();
    await page.reload();
    const reopened = await openTools(page);
    await expect(
      reopened.getByRole("switch", { name: "Nightly yt-dlp" })
    ).toBeChecked();

    const audit = await new AxeBuilder({ page })
      .include('[role="dialog"]')
      .analyze();

    expect(audit.violations).toEqual([]);
    await page.screenshot({ path: "test-results/video-tools-nightly.png" });
  });
}
