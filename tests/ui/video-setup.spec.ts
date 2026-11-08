import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import type { VideoToolsSnapshot } from "../../frontend/lib/video-tools";
import { mockDesktop, preferences } from "./fixture";

const initial: VideoToolsSnapshot = {
  keep_up_to_date: true,
  nightly_available: false,
  published_size: null,
  sequence: 1,
  state: { kind: "not_set_up" },
  update_available: false,
  use_nightly: false,
};

const setup = (page: Page) =>
  page.getByRole("dialog", { exact: true, name: "Set Up Video Tools" });

const launch = async (page: Page, videoTools = initial) => {
  await mockDesktop(page, {
    persist: true,
    preferences: { ...preferences, video_tools_setup_shown: false },
    videoTools,
  });
  await page.goto("/");
};

test("first launch offers optional tools across the app window without consent", async ({
  page,
}) => {
  await launch(page);
  await expect(setup(page)).toBeVisible();
  await expect(
    setup(page).getByRole("checkbox", { name: "Keep video tools up to date" })
  ).toBeChecked();
  await expect(
    setup(page).getByRole("button", {
      exact: true,
      name: "Download video tools (about 23 MB)",
    })
  ).toBeVisible();
  await expect(
    setup(page).getByText("Video pages and playlists", { exact: true })
  ).toHaveAttribute("data-unavailable", "true");
  const bounds = await setup(page).boundingBox();
  expect(bounds).toMatchObject({ height: 800, width: 1220, x: 0, y: 0 });

  const audit = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .analyze();

  expect(audit.violations).toEqual([]);
  expect(
    await page.evaluate(() =>
      sessionStorage.getItem("test-video-tools-operation")
    )
  ).not.toContain("set_up");
});

for (const dismissal of [
  "Not now",
  "Escape",
  "Close",
  "Don't show this again",
]) {
  test(`${dismissal} dismisses without downloading and stays dismissed after launch`, async ({
    page,
  }) => {
    await launch(page);
    await expect(setup(page)).toBeVisible();

    await (dismissal === "Escape"
      ? page.keyboard.press("Escape")
      : setup(page)
          .getByRole("button", { exact: true, name: dismissal })
          .click());
    await expect(setup(page)).toBeHidden();

    const state = await page.evaluate(
      () => window.testVideoTools.snapshot().state.kind
    );

    expect(state).toBe(
      dismissal === "Don't show this again" ? "turned_off" : "not_set_up"
    );
    await page.reload();
    await expect(
      page.getByRole("button", { exact: true, name: "Settings" })
    ).toBeVisible();
    await expect(setup(page)).toBeHidden();
    await page.evaluate(() =>
      window.dispatchEvent(new Event("test:open-video-setup"))
    );
    await expect(setup(page)).toBeVisible();
    await expect(
      setup(page).getByRole("button", { name: /Download video tools/u })
    ).toBeVisible();
    await setup(page)
      .getByRole("button", { name: /Download video tools/u })
      .click();
    await expect(
      setup(page).getByText("Setting up Python", { exact: true })
    ).toBeVisible();
  });
}

const push = async (page: Page, video: VideoToolsSnapshot) => {
  await page.evaluate((value) => {
    sessionStorage.setItem("test-video-tools", JSON.stringify(value));
    window.dispatchEvent(new Event("test:video-tools"));
  }, video);
};

const installing: VideoToolsSnapshot = {
  ...initial,
  sequence: 20,
  state: {
    kind: "setting_up",
    steps: [
      {
        component: "python",
        downloaded_bytes: 100,
        status: "done",
        total_bytes: 100,
      },
      {
        component: "yt_dlp",
        downloaded_bytes: 50,
        status: "downloading",
        total_bytes: 100,
      },
    ],
  },
};

const ready: VideoToolsSnapshot = {
  ...initial,
  sequence: 30,
  state: {
    installed: { python_version: "3.13.9", runtime: 1, yt_dlp: "2026.10.26" },
    kind: "ready",
    last_checked: null,
    update_flag: { kind: "none" },
  },
};

test("consent shows real size, step progress and completion while Continue keeps Opul usable", async ({
  page,
}) => {
  await launch(page, { ...initial, published_size: 23_400_000 });
  await setup(page)
    .getByRole("checkbox", { name: "Keep video tools up to date" })
    .uncheck();
  await setup(page)
    .getByRole("button", {
      exact: true,
      name: "Download video tools (23.4 MB)",
    })
    .click();
  await expect(
    setup(page).getByText("Setting up Python", { exact: true })
  ).toBeVisible();
  await expect(
    setup(page).getByText(
      "The language yt-dlp runs on, plus a JavaScript engine for some sites."
    )
  ).toBeVisible();
  await expect(setup(page).getByText("Waiting", { exact: true })).toBeVisible();
  await push(page, { ...installing, keep_up_to_date: false });
  await expect(
    setup(page).getByRole("progressbar", { name: "Setting up Python progress" })
  ).toHaveAttribute("aria-valuenow", "100");
  await expect(
    setup(page).getByRole("progressbar", { name: "Setting up yt-dlp progress" })
  ).toHaveAttribute("aria-valuenow", "50");
  await expect(setup(page).getByText("Done", { exact: true })).toBeVisible();
  await setup(page).getByRole("button", { name: "Continue to Opul" }).click();

  const sidebarProgress = page.getByRole("button", {
    exact: true,
    name: "Setting up video tools… 75%",
  });

  await expect(sidebarProgress).toBeVisible();
  await sidebarProgress.click();
  await expect(setup(page).getByText("Done", { exact: true })).toBeVisible();
  await setup(page).getByRole("button", { name: "Continue to Opul" }).click();

  const input = page.getByRole("searchbox", {
    name: "Search downloads or paste a link",
  });

  await input.fill("https://example.com/public-domain.zip");
  await page
    .getByRole("button", { exact: true, name: "Download link" })
    .click();
  await expect(
    page.getByRole("button", { exact: true, name: "new-file.zip" })
  ).toBeVisible();
  expect(
    await page.evaluate(() =>
      JSON.parse(sessionStorage.getItem("test-command") ?? "{}")
    )
  ).toMatchObject({ kind: "add_url", mode: "Direct" });
  await input.fill(
    "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567"
  );
  await page
    .getByRole("button", { exact: true, name: "Download link" })
    .click();
  await expect
    .poll(() =>
      page.evaluate(() => sessionStorage.getItem("test-add-attempts"))
    )
    .toBe("2");
  expect(
    await page.evaluate(() =>
      JSON.parse(sessionStorage.getItem("test-command") ?? "{}")
    )
  ).toMatchObject({ kind: "add_url", mode: "Torrent" });
  await expect(sidebarProgress).toBeVisible();
  await page.reload();
  await expect(setup(page)).toBeHidden();
  await expect(sidebarProgress).toBeVisible();
  await push(page, ready);
  await expect(sidebarProgress).toBeHidden();
  await expect(
    page.getByText("Video tools are ready", { exact: true })
  ).toHaveCount(1);
  await push(page, { ...ready, sequence: 31 });
  await expect(
    page.getByText("Video tools are ready", { exact: true })
  ).toHaveCount(1);
  await page.reload();
  await expect(
    page.getByRole("button", { exact: true, name: "Settings" })
  ).toBeVisible();
  await expect(
    page.getByText("Video tools are ready", { exact: true })
  ).toHaveCount(0);
});

test("Settings reopens Setup without starting it and shares its progress", async ({
  page,
}) => {
  await launch(page);
  await setup(page)
    .getByRole("button", { exact: true, name: "Not now" })
    .click();
  await page.getByRole("button", { exact: true, name: "Settings" }).click();
  const settings = page.getByRole("dialog", { exact: true, name: "Settings" });
  await settings.getByRole("button", { exact: true, name: "Tools" }).click();
  await settings
    .getByRole("button", { exact: true, name: "Set Up Video Tools…" })
    .click();
  await expect(settings).toBeHidden();
  await setup(page)
    .getByRole("button", { name: /Download video tools/u })
    .click();
  await setup(page).getByRole("button", { name: "Continue to Opul" }).click();
  await page.getByRole("button", { exact: true, name: "Settings" }).click();
  await settings.getByRole("button", { exact: true, name: "Tools" }).click();
  await expect(
    settings.getByRole("progressbar", { name: "Setting up Python progress" })
  ).toBeVisible();
  await settings
    .getByRole("button", { exact: true, name: "Set Up Video Tools…" })
    .click();
  await expect(
    setup(page).getByText("Setting up Python", { exact: true })
  ).toBeVisible();
});

test("live steps show waiting, verification, installation and failure recovery", async ({
  page,
}) => {
  await launch(page);
  await setup(page)
    .getByRole("button", { name: /Download video tools/u })
    .click();

  const showStepStatus = async (
    status: "waiting_for_internet" | "verifying" | "installing",
    label: string,
    sequence: number
  ) => {
    const steps =
      installing.state.kind === "setting_up" ? installing.state.steps : [];

    await push(page, {
      ...installing,
      sequence,
      state: {
        kind: "setting_up",
        steps: steps.map((step) =>
          step.component === "yt_dlp" ? { ...step, status } : step
        ),
      },
    });
    await expect(setup(page).getByText(label, { exact: true })).toBeVisible();
    await expect(
      setup(page).getByRole("button", { name: "Continue to Opul" })
    ).toBeEnabled();
  };

  await showStepStatus(
    "waiting_for_internet",
    "Waiting for an internet connection…",
    21
  );
  await showStepStatus("verifying", "Verifying", 22);
  await showStepStatus("installing", "Installing", 23);

  const audit = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .analyze();

  expect(audit.violations).toEqual([]);
  await page.screenshot({ path: test.info().outputPath("setup-progress.png") });
  await push(page, {
    ...initial,
    sequence: 25,
    state: {
      details: "The download couldn't be verified.",
      kind: "not_working",
      reason: "verification",
    },
  });
  await expect(setup(page).getByRole("alert")).toContainText(
    "couldn't be verified"
  );
  await setup(page)
    .getByRole("button", { exact: true, name: "Try again" })
    .click();
  await expect(
    setup(page).getByText("Setting up Python", { exact: true })
  ).toBeVisible();
  await push(page, ready);
  await expect(setup(page).getByText("Done", { exact: true })).toHaveCount(2);
  await setup(page).getByRole("button", { name: "Continue to Opul" }).click();
});

for (const kind of ["turned_off", "ready"] as const) {
  test(`first launch does not offer Setup when tools are ${kind}`, async ({
    page,
  }) => {
    await launch(
      page,
      kind === "ready" ? ready : { ...initial, state: { kind } }
    );
    await expect(
      page.getByRole("button", { exact: true, name: "Settings" })
    ).toBeVisible();
    await expect(setup(page)).toBeHidden();
    await page.evaluate(() =>
      window.dispatchEvent(new Event("test:open-video-setup"))
    );
    await expect(setup(page)).toBeVisible();
  });
}
