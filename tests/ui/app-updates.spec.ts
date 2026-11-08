import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import type { AppUpdates } from "../../frontend/lib/app-updates";
import { appUpdates } from "./app-updates-fixture";
import { mockDesktop, snapshot } from "./fixture";

const available: AppUpdates = {
  ...appUpdates,
  last_checked: 1_793_000_000,
  phase: { kind: "available", version: "0.4.0" },
  release_notes_url: "https://github.com/slovakian/opul/releases/tag/v0.4.0",
};

const idleLibrary = {
  ...snapshot,
  workspaces: snapshot.workspaces.map((workspace) => ({
    ...workspace,
    downloads: [],
  })),
};

const publish = async (page: Page, release: string | null) => {
  await page.evaluate((version) => {
    if (version) {
      sessionStorage.setItem("test-app-update-release", version);
    } else {
      sessionStorage.removeItem("test-app-update-release");
    }
  }, release);
};

/** Simulates a scheduled check that found something, as Rust would emit it. */
const push = async (page: Page, updates: AppUpdates) => {
  await page.evaluate((value) => {
    sessionStorage.setItem("test-app-updates", JSON.stringify(value));
    window.dispatchEvent(new Event("test:app-updates"));
  }, updates);
};

const openUpdates = async (page: Page) => {
  await page.getByRole("button", { exact: true, name: "Settings" }).click();
  const settings = page.getByRole("dialog", { exact: true, name: "Settings" });
  await settings.getByRole("button", { exact: true, name: "Updates" }).click();

  return settings;
};

const updateToast = (page: Page) =>
  page.locator('[data-slot="toast"]').filter({ hasText: /Opul|update/u });

const lastCommand = (page: Page) =>
  page.evaluate(() =>
    JSON.parse(sessionStorage.getItem("test-app-update-command") ?? "null")
  );

test("Settings checks, downloads and offers Restart, with no toast offer for a manual check", async ({
  page,
}) => {
  await mockDesktop(page, { snapshot: idleLibrary });
  await page.goto("/");
  const settings = await openUpdates(page);

  await expect(settings.getByText("Opul 0.3.0")).toBeVisible();
  await expect(settings.getByText("stable", { exact: true })).toBeVisible();
  await expect(settings.getByText("Not checked yet")).toBeVisible();
  await expect(
    settings.getByRole("switch", { name: "Check for updates automatically" })
  ).toBeChecked();
  await expect(
    settings.getByRole("switch", { name: "Install updates automatically" })
  ).not.toBeChecked();

  await settings
    .getByRole("button", { exact: true, name: "Check for Updates" })
    .click();
  await expect(
    settings.getByRole("button", { name: "Checking…" })
  ).toBeDisabled();
  await expect(
    settings.getByText(/You're up to date · checked/u)
  ).toBeVisible();

  await publish(page, "0.4.0");
  await settings
    .getByRole("button", { exact: true, name: "Check for Updates" })
    .click();
  await expect(settings.getByText("Opul 0.4.0 is available")).toBeVisible();
  await expect(
    settings.getByRole("link", { name: "What's new" })
  ).toHaveAttribute(
    "href",
    "https://github.com/slovakian/opul/releases/tag/v0.4.0"
  );
  await expect(
    settings
      .getByRole("listitem")
      .filter({ hasText: "Updates" })
      .getByText("Update available")
  ).toBeAttached();

  const audit = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .analyze();

  expect(audit.violations).toEqual([]);
  await page.screenshot({ path: "test-results/app-updates-available.png" });

  await settings.getByRole("button", { exact: true, name: "Download" }).click();
  await expect(settings.getByRole("progressbar")).toBeVisible();
  await expect(settings.getByText("Downloading 0.4.0… 50%")).toBeVisible();
  await page.screenshot({ path: "test-results/app-updates-downloading.png" });
  await expect(
    settings.getByText("Opul 0.4.0 is ready. Restart to finish.")
  ).toBeVisible();
  await settings.getByRole("button", { exact: true, name: "Restart" }).click();
  await expect
    .poll(() =>
      page.evaluate(() => sessionStorage.getItem("test-app-update-restarted"))
    )
    .toBe("true");
});

test("an available update offers itself once per version, and Not now holds across launches", async ({
  page,
}) => {
  await mockDesktop(page, { appUpdates: available });
  await page.goto("/");
  const offer = updateToast(page);

  await expect(offer).toContainText("Opul 0.4.0 is available");
  await expect(
    offer.getByRole("button", { exact: true, name: "Download" })
  ).toBeVisible();

  const audit = await new AxeBuilder({ page })
    .include('[data-slot="toast"]')
    .analyze();

  expect(audit.violations).toEqual([]);
  await page.screenshot({ path: "test-results/app-update-offer.png" });
  await offer.getByRole("button", { exact: true, name: "Not now" }).click();
  await expect(offer).toHaveCount(0);

  await page.reload();
  await expect(
    page.getByRole("button", { exact: true, name: "Settings" })
  ).toBeVisible();
  await expect(offer).toHaveCount(0);

  await push(page, {
    ...available,
    phase: { kind: "available", version: "0.4.1" },
  });
  await expect(offer).toContainText("Opul 0.4.1 is available");
});

test("Download from the offer shows progress, then Restart asks while downloads are active", async ({
  page,
}) => {
  await mockDesktop(page, { appUpdates: available });
  await page.goto("/");
  const offer = updateToast(page);

  await offer.getByRole("button", { exact: true, name: "Download" }).click();
  await expect(offer).toContainText("Downloading Opul 0.4.0…");
  await expect(
    offer.getByRole("button", { name: /^(?:Download|Not now)$/u })
  ).toHaveCount(0);
  await expect(offer).toContainText("Opul 0.4.0 is ready");
  await expect(
    offer.getByRole("button", { exact: true, name: "Later" })
  ).toBeVisible();
  await page.screenshot({ path: "test-results/app-update-ready.png" });

  await offer.getByRole("button", { exact: true, name: "Restart" }).click();
  const confirm = page.getByRole("dialog", { name: "Restart Opul now?" });

  await expect(confirm).toContainText(/Restarting interrupts \d+ active/u);

  const audit = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .analyze();

  expect(audit.violations).toEqual([]);
  await confirm.getByRole("button", { exact: true, name: "Later" }).click();
  await expect(confirm).toHaveCount(0);
  expect(
    await page.evaluate(() =>
      sessionStorage.getItem("test-app-update-restarted")
    )
  ).toBeNull();
});

test("a failed download offers Try again", async ({ page }) => {
  await mockDesktop(page, {
    appUpdates: available,
    failAppUpdateDownload: true,
  });
  await page.goto("/");
  const offer = updateToast(page);

  await offer.getByRole("button", { exact: true, name: "Download" }).click();
  await expect(offer).toContainText("Couldn't download Opul 0.4.0");
  await expect(offer).toContainText("signature verification");
  await expect(
    offer.getByRole("button", { exact: true, name: "Try again" })
  ).toBeVisible();
});

test("automatic installs skip the offer and only say when the update is ready", async ({
  page,
}) => {
  await mockDesktop(page, {
    appUpdates: { ...appUpdates, install_automatically: true },
  });
  await page.goto("/");
  await expect(
    page.getByRole("button", { exact: true, name: "Settings" })
  ).toBeVisible();
  await push(page, {
    ...available,
    install_automatically: true,
  });
  const notice = updateToast(page);

  await expect(notice).toContainText("Opul 0.4.0 is ready");
  await expect(notice).not.toContainText("is available");
  await expect(notice).not.toContainText("Downloading");
});

test("installing automatically needs automatic checks", async ({ page }) => {
  await mockDesktop(page);
  await page.goto("/");
  const settings = await openUpdates(page);

  const install = settings.getByRole("switch", {
    name: "Install updates automatically",
  });

  await install.click();
  await expect(install).toBeChecked();
  expect(await lastCommand(page)).toEqual({
    args: { checkAutomatically: true, installAutomatically: true },
    name: "app_updates_configure",
  });
  await settings
    .getByRole("switch", { name: "Check for updates automatically" })
    .click();
  await expect(install).toBeDisabled();
  await expect(install).not.toBeChecked();
  await expect(
    settings.getByRole("button", { exact: true, name: "Done" })
  ).toBeVisible();
});

test("a development build can check but explains why it can't install", async ({
  page,
}) => {
  await mockDesktop(page, {
    appUpdates: { ...available, install_blocker: "development" },
  });
  await page.goto("/");
  const settings = await openUpdates(page);

  await expect(settings.getByText("Opul 0.4.0 is available")).toBeVisible();
  await expect(
    settings.getByRole("button", { exact: true, name: "Download" })
  ).toBeDisabled();
  await expect(
    settings.getByText("Development builds can't install updates.")
  ).toBeVisible();
  await expect(updateToast(page)).toHaveCount(0);
});

test("the app menu's Check for Updates… opens Updates and checks", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await expect(
    page.getByRole("button", { exact: true, name: "Settings" })
  ).toBeVisible();
  await page.evaluate(() =>
    window.dispatchEvent(new Event("test:menu-check-for-updates"))
  );
  const settings = page.getByRole("dialog", { exact: true, name: "Settings" });

  await expect(
    settings.getByRole("heading", { name: "Updates" })
  ).toBeVisible();
  await expect(settings.getByText(/You're up to date/u)).toBeVisible();
  await settings.getByRole("button", { exact: true, name: "About" }).click();
  await expect(settings.getByText("Opul 0.3.0")).toBeVisible();
});
