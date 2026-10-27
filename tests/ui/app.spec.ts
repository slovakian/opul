import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

import { mockDesktop, snapshot } from "./fixture";

test("search, transfer actions, table view, inspector, and removal", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await mockDesktop(page);
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "All downloads" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Design resources.zip" })
  ).toBeVisible();
  await page
    .getByRole("button", { exact: true, name: "Pause Design resources.zip" })
    .click();
  await expect(
    page.getByRole("button", {
      exact: true,
      name: "Resume Design resources.zip",
    })
  ).toBeVisible();
  await page
    .getByRole("button", { exact: true, name: "Resume Design resources.zip" })
    .click();
  await page
    .getByRole("searchbox", { name: "Search downloads or paste a link" })
    .fill("Field notes");
  await expect(
    page.getByRole("button", { exact: true, name: "Design resources.zip" })
  ).toHaveCount(0);
  await page
    .getByRole("searchbox", { name: "Search downloads or paste a link" })
    .clear();
  await page
    .getByRole("button", { exact: true, name: "Design resources.zip" })
    .click();
  await expect(
    page.getByRole("complementary", { name: "Download details" })
  ).toBeVisible();
  await page.screenshot({ path: "test-results/inspector.png" });
  await page.getByRole("button", { name: "Close details" }).click();
  await page.getByRole("button", { exact: true, name: "Table view" }).click();
  await expect(page.getByRole("table")).toBeVisible();
  await page.screenshot({ path: "test-results/table.png" });
  await page
    .getByRole("button", { name: "Actions for Field notes.pdf" })
    .click();
  const menu = page.locator('[data-slot="popover-content"]');
  await expect(menu.locator('[data-slot="sidebar"]')).toBeVisible();
  await menu.getByRole("button", { name: "Remove from library" }).click();
  await page
    .getByRole("button", { exact: true, name: "Remove download" })
    .click();
  await expect(
    page.getByRole("button", { exact: true, name: "Field notes.pdf" })
  ).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("emoji workspace creation and Base UI settings dialog", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await expect(
    page.getByRole("button", { exact: true, name: "Downloads workspace" })
  ).toContainText("📁");
  await page
    .getByRole("button", { exact: true, name: "New workspace" })
    .last()
    .click();
  await page.getByLabel("Name", { exact: true }).fill("Research");
  await page.getByRole("button", { name: /^Workspace emoji:/ }).click();
  await page.getByRole("textbox", { name: "Search emoji" }).fill("books");
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("gridcell", { exact: true, name: "books" })
    .click();
  await page.getByRole("button", { exact: true, name: "Done" }).click();
  await page
    .getByRole("button", { exact: true, name: "Create workspace" })
    .click();
  await expect(
    page.getByRole("heading", { exact: true, name: "Research" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Research workspace" })
  ).toContainText("📚");
  await page.getByRole("button", { exact: true, name: "Settings" }).click();
  const dialog = page.getByRole("dialog", { exact: true, name: "Settings" });
  await expect(dialog.locator('[data-slot="sidebar"]')).toBeVisible();
  await dialog.getByRole("button", { exact: true, name: "Dark" }).click();
  await page.screenshot({ path: "test-results/settings.png" });
  await dialog.getByRole("button", { name: "Save changes" }).click();
  await expect(page.locator("html")).toHaveClass(/dark/);
  await page.screenshot({ path: "test-results/dark.png" });
});

test("imports retain errors, close on success, and use sidebar choices", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await page
    .getByRole("button", { exact: true, name: "Download options" })
    .click();

  const dialog = page.getByRole("dialog", {
    exact: true,
    name: "New download",
  });

  await dialog.getByLabel("Link", { exact: true }).fill("file:///private/file");
  await dialog
    .getByRole("button", { exact: true, name: "Add download" })
    .click();
  await expect(dialog).toBeVisible();
  await expect(
    page.getByText("Use an HTTP, HTTPS, or magnet link.")
  ).toBeVisible();
  await dialog
    .getByLabel("Link", { exact: true })
    .fill("https://example.com/new-file.zip");
  await dialog.getByRole("button", { name: /^Download type:/ }).click();
  await expect(
    page.locator('[data-slot="popover-content"] [data-slot="sidebar"]')
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await dialog
    .getByRole("button", { exact: true, name: "Add download" })
    .click();
  await expect(dialog).toHaveCount(0);
  await expect(
    page.getByRole("button", { exact: true, name: "new-file.zip" })
  ).toBeVisible();
});

test("header downloads use the workspace default and selected saved folder", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  const workspace = library.workspaces[0];
  workspace.folders[0].is_default = false;
  workspace.folders.push(
    {
      id: "folder-2",
      is_default: true,
      name: "Research",
      path: "/Users/test/Research",
    },
    {
      id: "folder-3",
      is_default: false,
      name: "Archive",
      path: "/Users/test/Archive",
    }
  );
  library.workspaces.push({
    ...workspace,
    downloads: [],
    folders: [
      {
        id: "folder-4",
        name: "Videos",
        path: "/Users/test/Videos",
        is_default: true,
      },
    ],
    id: "workspace-2",
    name: "Media",
  });
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  await page
    .getByRole("button", { exact: true, name: "Downloads workspace" })
    .click();
  const header = page.locator("main > header");
  await expect(
    header.getByRole("searchbox", { name: "Search downloads or paste a link" })
  ).toBeVisible();
  await expect(
    header.getByRole("button", {
      exact: true,
      name: "Save location: Research",
    })
  ).toBeVisible();
  await header
    .getByRole("button", { exact: true, name: "Save location: Research" })
    .click();
  const folders = page.locator('[data-slot="popover-content"]');
  await expect(folders.getByRole("button").first()).toHaveText(
    "Research (Default)"
  );
  await folders.getByRole("button", { exact: true, name: "Archive" }).click();

  const link = header.getByRole("searchbox", {
    exact: true,
    name: "Search downloads or paste a link",
  });

  await link.fill("file:///private/file");
  await expect(
    header.getByRole("button", { exact: true, name: "Download link" })
  ).toBeDisabled();
  await link.press("Enter");
  await expect(
    page.getByText("Use an HTTP, HTTPS, or magnet link.")
  ).toHaveCount(0);
  await expect(link).toHaveValue("file:///private/file");
  await link.fill("https://example.com/new-file.zip");
  await header
    .getByRole("button", { exact: true, name: "Download options" })
    .click();

  const dialog = page.getByRole("dialog", {
    exact: true,
    name: "New download",
  });

  await expect(dialog.getByLabel("Link", { exact: true })).toHaveValue(
    "https://example.com/new-file.zip"
  );
  await expect(
    dialog.getByRole("button", { exact: true, name: "Save location: Archive" })
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await link.press("Enter");
  await expect(link).toHaveValue("");
  await page.getByRole("button", { exact: true, name: "new-file.zip" }).click();
  const details = page.getByRole("complementary", { name: "Download details" });
  await expect(
    details.locator("dd").filter({ hasText: /^Archive$/ })
  ).toBeVisible();
  await page
    .getByRole("button", { exact: true, name: "Close details" })
    .click();
  await page
    .getByRole("button", { exact: true, name: "Media workspace" })
    .click();
  await expect(
    header.getByRole("button", { exact: true, name: "Save location: Videos" })
  ).toBeVisible();
  await page
    .getByRole("button", { exact: true, name: "Downloads workspace" })
    .click();
  await expect(
    header.getByRole("button", {
      exact: true,
      name: "Save location: Research",
    })
  ).toBeVisible();
});

test("add a saved location from the download input without changing the default", async ({
  page,
}) => {
  await mockDesktop(page, { folderPickerPath: "/Users/test/Research" });
  await page.goto("/");
  const header = page.locator("main > header");

  const link = header.getByRole("searchbox", {
    exact: true,
    name: "Search downloads or paste a link",
  });

  await link.fill("https://example.com/new-file.zip");
  await header
    .getByRole("button", {
      exact: true,
      name: "Save location: Downloads › Downloads",
    })
    .click();
  const menu = page.locator('[data-slot="popover-content"]');
  await menu
    .getByRole("button", { exact: true, name: "Add new location…" })
    .click();
  await expect(
    header.getByRole("button", {
      exact: true,
      name: "Save location: Downloads › Research",
    })
  ).toBeVisible();
  await expect(link).toHaveValue("https://example.com/new-file.zip");
  await header
    .getByRole("button", {
      exact: true,
      name: "Save location: Downloads › Research",
    })
    .click();
  await expect(menu.locator('[data-slot="sidebar-group-label"]')).toHaveText(
    "Downloads"
  );
  await expect(menu.getByRole("button").first()).toHaveText(
    "Downloads (Default)"
  );
  await expect(
    menu.getByRole("button", { exact: true, name: "Research" })
  ).toHaveAttribute("data-active");
  await page.screenshot({ path: "test-results/location-menu.png" });
  await page.keyboard.press("Escape");
  await link.press("Enter");
  await page.getByRole("button", { exact: true, name: "new-file.zip" }).click();
  await expect(
    page
      .getByRole("complementary", { name: "Download details" })
      .locator("dd")
      .filter({ hasText: /^Research$/ })
  ).toBeVisible();
  await page
    .getByRole("button", { exact: true, name: "Close details" })
    .click();
  await page
    .getByRole("button", { exact: true, name: "All downloads" })
    .click();
  await expect(
    page.getByRole("button", { exact: true, name: "Research" })
  ).toBeVisible();
});

for (const outcome of ["cancelled", "failed"]) {
  test(`adding a location keeps the draft and selection when the picker is ${outcome}`, async ({
    page,
  }) => {
    await mockDesktop(page, {
      folderPickerError:
        outcome === "failed" ? "Folder picker unavailable" : undefined,
      folderPickerPath: null,
    });
    await page.goto("/");
    const header = page.locator("main > header");

    const link = header.getByRole("searchbox", {
      exact: true,
      name: "Search downloads or paste a link",
    });

    await link.fill("https://example.com/new-file.zip");
    await header
      .getByRole("button", {
        exact: true,
        name: "Save location: Downloads › Downloads",
      })
      .click();
    await page
      .locator('[data-slot="popover-content"]')
      .getByRole("button", { exact: true, name: "Add new location…" })
      .click();

    if (outcome === "failed") {
      await expect(page.getByText("Folder picker unavailable")).toBeVisible();
    }
    await expect(
      header.getByRole("button", {
        exact: true,
        name: "Save location: Downloads › Downloads",
      })
    ).toBeVisible();
    await expect(link).toHaveValue("https://example.com/new-file.zip");
  });
}

test("empty, recovery, and startup failure states", async ({ browser }) => {
  for (const state of ["empty", "recovery", "fail"]) {
    const page = await browser.newPage();
    await mockDesktop(page, {
      empty: state === "empty",
      fail: state === "fail",
      recovery: state === "recovery",
    });
    await page.goto("/");

    if (state === "empty") {
      await expect(page.getByText("No downloads yet")).toBeVisible();
      await page.screenshot({ path: "test-results/empty.png" });
    }

    if (state === "recovery") {
      await expect(
        page.getByText("Your library needs attention")
      ).toBeVisible();
      await page.getByRole("button", { name: "Acknowledge recovery" }).click();
      await expect(page.getByText("Your library needs attention")).toHaveCount(
        0
      );
    }

    if (state === "fail") {
      await expect(
        page.getByText("The state directory is already in use")
      ).toBeVisible();
    }
    await page.close();
  }
});

test("keyboard navigation, accessibility, and narrow layout", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "All downloads" })
  ).toBeVisible();
  const result = await new AxeBuilder({ page }).analyze();
  expect(result.violations).toEqual([]);
  await page.getByRole("button", { exact: true, name: "Settings" }).click();

  // Base UI's WebKit VoiceOver focus sentinels are not user controls.
  const dialogResult = await new AxeBuilder({ page })
    .exclude("[data-base-ui-focus-guard]")
    .analyze();

  expect(dialogResult.violations).toEqual([]);
  await page.keyboard.press("Escape");
  await page.setViewportSize({ height: 800, width: 768 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth
    )
  ).toBe(true);
  await page.screenshot({ path: "test-results/narrow.png" });
  await page.setViewportSize({ height: 844, width: 390 });
  await page.getByRole("button", { name: "Toggle Sidebar" }).click();
  await expect(
    page.getByRole("button", { exact: true, name: "Downloads workspace" })
  ).toBeVisible();
});
