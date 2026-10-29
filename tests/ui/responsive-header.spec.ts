// Viewport changes and keyboard actions share one page and must run in order.
/* oxlint-disable no-await-in-loop */
import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

import { mockDesktop, snapshot } from "./fixture";

test("breadcrumbs shorten before destinations and display controls combine", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  library.workspaces[0].name = "Research and reference material";
  library.workspaces[0].folders.push({
    id: "archive",
    is_default: false,
    name: "Collected references",
    path: "/Users/test/Research/Collected references",
  });
  await page.setViewportSize({ height: 800, width: 1600 });
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  await page
    .getByRole("button", { exact: true, name: "Collected references" })
    .click();
  const header = page.locator("main > header");

  const parent = header.getByRole("button", {
    exact: true,
    name: library.workspaces[0].name,
  });

  const destination = header.locator(".header-location");
  const parentWidth = await parent.evaluate((element) => element.clientWidth);

  const destinationWidth = await destination.evaluate(
    (element) => element.clientWidth
  );

  await page.setViewportSize({ height: 800, width: 1300 });
  await expect
    .poll(() => parent.evaluate((element) => element.clientWidth))
    .toBeLessThan(parentWidth);
  expect(await destination.evaluate((element) => element.clientWidth)).toBe(
    destinationWidth
  );
  await expect(parent).toBeVisible();
  await expect(
    header.getByRole("heading", { name: "Collected references" })
  ).toBeVisible();
  await expect(
    header.getByRole("navigation", { name: "breadcrumb" })
  ).toHaveAttribute(
    "title",
    "Research and reference material › Collected references"
  );
  await expect(header.locator("h1 > [aria-hidden]")).toBeVisible();
  await page.setViewportSize({ height: 800, width: 1220 });
  await expect
    .poll(() => destination.evaluate((element) => element.clientWidth))
    .toBeLessThan(destinationWidth);
  await expect(destination).toBeVisible();
  await expect(
    header.getByRole("button", { name: /^Save location:/u })
  ).toHaveAttribute("title", /\/Users\/test\/Research\/Collected references/u);
  await expect(
    header.getByRole("button", { exact: true, name: "Table view" })
  ).toBeVisible();
  await page.setViewportSize({ height: 800, width: 1120 });
  await expect(
    header.getByRole("button", { exact: true, name: "View" })
  ).toBeVisible();
  await expect(
    header.getByRole("button", { exact: true, name: "Table view" })
  ).toBeHidden();
  await expect(
    header.getByRole("button", { exact: true, name: "Filter downloads" })
  ).toBeVisible();
  await expect(
    header.getByRole("button", { exact: true, name: "Workspace actions" })
  ).toBeVisible();
  await page.screenshot({ path: "test-results/responsive-intermediate.png" });
});

test("compact View preserves preferences, keyboard focus, drafts and 720px typing room", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.setViewportSize({ height: 800, width: 720 });
  await mockDesktop(page, { persist: true });
  await page.goto("/");
  const header = page.locator("main > header");
  const input = header.getByRole("searchbox");
  const view = header.getByRole("button", { exact: true, name: "View" });
  await input.fill("keep this draft");
  await view.focus();
  await page.keyboard.press("Enter");
  const menu = page.locator('[data-slot="popover-content"]');
  await expect(
    menu.getByRole("button", { exact: true, name: "Table view" })
  ).toBeVisible();
  await page.keyboard.press("End");
  await expect(
    menu.getByRole("button", { exact: true, name: "Source" }).last()
  ).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(view).toBeFocused();
  await expect(input).toHaveValue("keep this draft");

  for (const name of ["Table view", "Name", "File type"]) {
    await view.click();
    await menu.getByRole("button", { exact: true, name }).click();
    await expect(view).toBeFocused();
  }

  await input.clear();
  await expect(page.getByRole("table").first()).toBeVisible();
  await page.reload();
  await view.click();

  for (const name of ["Table view", "Name", "File type"]) {
    await expect(
      menu.getByRole("button", { exact: true, name })
    ).toHaveAttribute("data-active", "");
  }

  await page.keyboard.press("Escape");
  await input.fill("https://example.com/a-long-download-name.zip");
  const box = await input.boundingBox();
  expect(box?.width).toBeGreaterThanOrEqual(120);

  for (const control of await header.locator("button:visible, input").all()) {
    const bounds = await control.boundingBox();
    expect(bounds?.x).toBeGreaterThanOrEqual(0);
    expect((bounds?.x ?? 0) + (bounds?.width ?? 0)).toBeLessThanOrEqual(720);
    expect(bounds?.height).toBeGreaterThanOrEqual(24);
  }

  expect(
    await header.evaluate(
      (element) => element.scrollWidth <= element.clientWidth
    )
  ).toBe(true);
  await page.screenshot({ path: "test-results/responsive-720.png" });
  await view.click();

  const results = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .exclude("[data-base-ui-focus-guard]")
    .analyze();

  expect(results.violations).toEqual([]);
  await page.keyboard.press("Escape");
  await page.setViewportSize({ height: 800, width: 1600 });
  const table = header.getByRole("button", { exact: true, name: "Table view" });
  await table.focus();
  await page.setViewportSize({ height: 800, width: 720 });
  await expect(view).toBeFocused();
  await view.click();
  await menu.getByRole("button", { exact: true, name: "Table view" }).focus();
  await page.setViewportSize({ height: 800, width: 1600 });
  await expect(menu).toHaveCount(0);
  await expect(
    header.getByRole("button", { name: /sort and group/u })
  ).toBeFocused();
  expect(errors).toEqual([]);
});

test("inspector and navigation become modal overlays with focus restoration", async ({
  page,
}) => {
  await page.setViewportSize({ height: 800, width: 1600 });
  await mockDesktop(page);
  await page.goto("/");

  const download = page.getByRole("button", {
    exact: true,
    name: "Design resources.zip",
  });

  await download.click();
  await expect(
    page.getByRole("complementary", { name: "Download details" })
  ).toBeVisible();
  await expect(
    page.getByRole("dialog", { exact: true, name: "Download details" })
  ).toHaveCount(0);
  await page.setViewportSize({ height: 800, width: 1040 });

  const inspector = page.getByRole("dialog", {
    exact: true,
    name: "Download details",
  });

  await expect(inspector).toBeVisible();
  await expect(inspector).toHaveAttribute("aria-modal", "true");
  await expect(
    page.locator('[data-sidebar="sidebar"][data-mobile="true"]')
  ).toHaveCount(0);
  await inspector
    .getByRole("button", { exact: true, name: "Close details" })
    .focus();
  await page.keyboard.press("Shift+Tab");
  expect(
    await inspector.evaluate((element) =>
      element.contains(document.activeElement)
    )
  ).toBe(true);
  await page.keyboard.press("Escape");
  await expect(inspector).toHaveCount(0);
  await expect(download).toBeFocused();
  await page.setViewportSize({ height: 800, width: 720 });

  const actions = page.getByRole("button", {
    exact: true,
    name: "Actions for Design resources.zip",
  });

  await actions.click();
  await page.getByRole("button", { exact: true, name: "Show details" }).focus();
  await page.keyboard.press("Enter");
  await expect(inspector).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(inspector).toHaveCount(0);
  await expect(actions).toBeFocused();
  await page.setViewportSize({ height: 800, width: 880 });

  const toggle = page
    .locator("main > header")
    .getByRole("button", { name: "Toggle Sidebar" });

  await toggle.click();

  const navigation = page.getByRole("dialog", {
    exact: true,
    name: "Workspace navigation",
  });

  await expect(navigation).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(navigation).toHaveCount(0);
  await expect(toggle).toBeFocused();
  await toggle.click();
  await expect(navigation).toBeVisible();
  await page.setViewportSize({ height: 800, width: 1220 });
  await expect(navigation).toHaveCount(0);
  await page.setViewportSize({ height: 800, width: 880 });
  await expect(navigation).toHaveCount(0);
  await toggle.click();
  await navigation
    .getByRole("button", { exact: true, name: "Completed" })
    .click();
  await expect(navigation).toHaveCount(0);
  await expect(toggle).toBeFocused();
  await expect(
    page.getByRole("heading", { exact: true, name: "Completed" })
  ).toBeVisible();
  await page.setViewportSize({ height: 800, width: 720 });
  await page
    .getByRole("button", { exact: true, name: "The creative process.mp4" })
    .click();
  await expect(inspector).toBeVisible();

  const results = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .exclude("[data-base-ui-focus-guard]")
    .analyze();

  expect(results.violations).toEqual([]);
  await page.setViewportSize({ height: 800, width: 1600 });
  await expect(inspector).toHaveCount(0);
  await expect(
    page.getByRole("complementary", { name: "Download details" })
  ).toBeVisible();
});

test("long folder breadcrumbs and Choose remain usable throughout window resizing", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  library.workspaces[0].name = "Workspace with a very long descriptive name";
  library.workspaces[0].folders[0].name =
    "A long descriptive saved folder name";
  library.workspaces.push({
    ...structuredClone(library.workspaces[0]),
    downloads: [],
    folders: [
      {
        id: "other-folder",
        is_default: true,
        name: "Other folder",
        path: "/Users/test/Other",
      },
    ],
    id: "workspace-2",
    name: "Other",
  });
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  const header = page.locator("main > header");
  const input = header.getByRole("searchbox");
  await input.fill("https://example.com/long-link.zip");

  for (const width of [
    1600, 1400, 1300, 1220, 1156, 1120, 1100, 1040, 960, 901, 900, 880, 800,
    768, 720,
  ]) {
    await page.setViewportSize({ height: 800, width });
    await expect(header.locator('[data-choose="true"]')).toHaveText("Choose…");
    expect(
      await input.evaluate((element) => element.clientWidth)
    ).toBeGreaterThanOrEqual(120);
    expect(
      await header.evaluate(
        (element) => element.scrollWidth <= element.clientWidth
      )
    ).toBe(true);
  }

  await page.setViewportSize({ height: 800, width: 1600 });
  await page
    .getByRole("button", {
      exact: true,
      name: library.workspaces[0].folders[0].name,
    })
    .click();

  for (const width of [1600, 1300, 1220, 1120, 1040, 901, 900, 720]) {
    await page.setViewportSize({ height: 800, width });
    expect(
      await input.evaluate((element) => element.clientWidth)
    ).toBeGreaterThanOrEqual(120);
    expect(
      await header.evaluate(
        (element) => element.scrollWidth <= element.clientWidth
      )
    ).toBe(true);
    await expect(
      header.getByRole("heading", {
        exact: true,
        name: library.workspaces[0].folders[0].name,
      })
    ).toBeVisible();
  }
});
