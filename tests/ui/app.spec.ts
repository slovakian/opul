import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { mockDesktop, snapshot } from "./fixture";

test("search, transfer actions, table view, inspector, and removal", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await mockDesktop(page);
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "All downloads" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Design resources.zip", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Pause Design resources.zip", exact: true })
    .click();
  await expect(
    page.getByRole("button", {
      name: "Resume Design resources.zip",
      exact: true,
    }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Resume Design resources.zip", exact: true })
    .click();
  await page
    .getByRole("searchbox", { name: "Search downloads" })
    .fill("Field notes");
  await expect(
    page.getByRole("button", { name: "Design resources.zip", exact: true }),
  ).toHaveCount(0);
  await page.getByRole("searchbox", { name: "Search downloads" }).clear();
  await page
    .getByRole("button", { name: "Design resources.zip", exact: true })
    .click();
  await expect(
    page.getByRole("complementary", { name: "Download details" }),
  ).toBeVisible();
  await page.screenshot({ path: "test-results/inspector.png" });
  await page.getByRole("button", { name: "Close details" }).click();
  await page.getByRole("button", { name: "Table view", exact: true }).click();
  await expect(page.getByRole("table")).toBeVisible();
  await page.screenshot({ path: "test-results/table.png" });
  await page
    .getByRole("button", { name: "Actions for Field notes.pdf" })
    .click();
  const menu = page.locator('[data-slot="popover-content"]');
  await expect(menu.locator('[data-slot="sidebar"]')).toBeVisible();
  await menu.getByRole("button", { name: "Remove from library" }).click();
  await page
    .getByRole("button", { name: "Remove download", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Field notes.pdf", exact: true }),
  ).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("emoji workspace creation and Base UI settings dialog", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Downloads workspace", exact: true }),
  ).toContainText("📁");
  await page
    .getByRole("button", { name: "New workspace", exact: true })
    .last()
    .click();
  await page.getByLabel("Name", { exact: true }).fill("Research");
  await page.getByRole("button", { name: /^Workspace emoji:/ }).click();
  await page.getByRole("textbox", { name: "Search emoji" }).fill("books");
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("gridcell", { name: "books", exact: true })
    .click();
  await page.getByRole("button", { name: "Done", exact: true }).click();
  await page
    .getByRole("button", { name: "Create workspace", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Research", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Research workspace", exact: true }),
  ).toContainText("📚");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Settings", exact: true });
  await expect(dialog.locator('[data-slot="sidebar"]')).toBeVisible();
  await dialog.getByRole("button", { name: "Dark", exact: true }).click();
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
    .getByRole("button", { name: "Download options", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "New download",
    exact: true,
  });
  await dialog.getByLabel("Link", { exact: true }).fill("file:///private/file");
  await dialog
    .getByRole("button", { name: "Add download", exact: true })
    .click();
  await expect(dialog).toBeVisible();
  await expect(
    page.getByText("Use an HTTP, HTTPS, or magnet link."),
  ).toBeVisible();
  await dialog
    .getByLabel("Link", { exact: true })
    .fill("https://example.com/new-file.zip");
  await dialog.getByRole("button", { name: /^Download type:/ }).click();
  await expect(
    page.locator('[data-slot="popover-content"] [data-slot="sidebar"]'),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await dialog
    .getByRole("button", { name: "Add download", exact: true })
    .click();
  await expect(dialog).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "new-file.zip", exact: true }),
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
      name: "Research",
      path: "/Users/test/Research",
      is_default: true,
    },
    {
      id: "folder-3",
      name: "Archive",
      path: "/Users/test/Archive",
      is_default: false,
    },
  );
  library.workspaces.push({
    ...workspace,
    id: "workspace-2",
    name: "Media",
    folders: [
      {
        id: "folder-4",
        name: "Videos",
        path: "/Users/test/Videos",
        is_default: true,
      },
    ],
    downloads: [],
  });
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  const header = page.locator("main > header");
  await expect(
    header.getByRole("searchbox", { name: "Search downloads" }),
  ).toBeVisible();
  await expect(
    header.getByRole("button", {
      name: "Save location: Research",
      exact: true,
    }),
  ).toBeVisible();
  await header
    .getByRole("button", { name: "Save location: Research", exact: true })
    .click();
  const folders = page.locator('[data-slot="popover-content"]');
  await expect(folders.getByRole("button").first()).toHaveText(
    "Research (Default)",
  );
  await folders.getByRole("button", { name: "Archive", exact: true }).click();
  const link = header.getByRole("textbox", {
    name: "Download link",
    exact: true,
  });
  await link.fill("file:///private/file");
  await header
    .getByRole("button", { name: "Download link", exact: true })
    .click();
  await expect(
    page.getByText("Use an HTTP, HTTPS, or magnet link."),
  ).toBeVisible();
  await expect(link).toHaveValue("file:///private/file");
  await link.fill("https://example.com/new-file.zip");
  await header
    .getByRole("button", { name: "Download options", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "New download",
    exact: true,
  });
  await expect(dialog.getByLabel("Link", { exact: true })).toHaveValue(
    "https://example.com/new-file.zip",
  );
  await expect(
    dialog.getByRole("button", { name: "Save location: Archive", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await link.press("Enter");
  await expect(link).toHaveValue("");
  await page.getByRole("button", { name: "new-file.zip", exact: true }).click();
  const details = page.getByRole("complementary", { name: "Download details" });
  await expect(
    details.locator("dd").filter({ hasText: /^Archive$/ }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Close details", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Media workspace", exact: true })
    .click();
  await expect(
    header.getByRole("button", { name: "Save location: Videos", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Downloads workspace", exact: true })
    .click();
  await expect(
    header.getByRole("button", {
      name: "Save location: Research",
      exact: true,
    }),
  ).toBeVisible();
});

test("add a saved location from the download input without changing the default", async ({
  page,
}) => {
  await mockDesktop(page, { folderPickerPath: "/Users/test/Research" });
  await page.goto("/");
  const header = page.locator("main > header");
  const link = header.getByRole("textbox", {
    name: "Download link",
    exact: true,
  });
  await link.fill("https://example.com/new-file.zip");
  await header
    .getByRole("button", { name: "Save location: Downloads", exact: true })
    .click();
  const menu = page.locator('[data-slot="popover-content"]');
  await menu
    .getByRole("button", { name: "Add new location…", exact: true })
    .click();
  await expect(
    header.getByRole("button", {
      name: "Save location: Research",
      exact: true,
    }),
  ).toBeVisible();
  await expect(link).toHaveValue("https://example.com/new-file.zip");
  await header
    .getByRole("button", { name: "Save location: Research", exact: true })
    .click();
  await expect(menu.locator('[data-slot="sidebar-group-label"]')).toHaveCount(
    0,
  );
  await expect(menu.getByRole("button").first()).toHaveText(
    "Downloads (Default)",
  );
  await expect(
    menu.getByRole("button", { name: "Research", exact: true }),
  ).toHaveAttribute("data-active");
  await page.screenshot({ path: "test-results/location-menu.png" });
  await page.keyboard.press("Escape");
  await link.press("Enter");
  await page.getByRole("button", { name: "new-file.zip", exact: true }).click();
  await expect(
    page
      .getByRole("complementary", { name: "Download details" })
      .locator("dd")
      .filter({ hasText: /^Research$/ }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Close details", exact: true })
    .click();
  await page
    .getByRole("button", { name: "All downloads", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Research", exact: true }),
  ).toBeVisible();
});

for (const outcome of ["cancelled", "failed"]) {
  test(`adding a location keeps the draft and selection when the picker is ${outcome}`, async ({
    page,
  }) => {
    await mockDesktop(page, {
      folderPickerPath: null,
      folderPickerError:
        outcome === "failed" ? "Folder picker unavailable" : undefined,
    });
    await page.goto("/");
    const header = page.locator("main > header");
    const link = header.getByRole("textbox", {
      name: "Download link",
      exact: true,
    });
    await link.fill("https://example.com/new-file.zip");
    await header
      .getByRole("button", { name: "Save location: Downloads", exact: true })
      .click();
    await page
      .locator('[data-slot="popover-content"]')
      .getByRole("button", { name: "Add new location…", exact: true })
      .click();
    if (outcome === "failed")
      await expect(page.getByText("Folder picker unavailable")).toBeVisible();
    await expect(
      header.getByRole("button", {
        name: "Save location: Downloads",
        exact: true,
      }),
    ).toBeVisible();
    await expect(link).toHaveValue("https://example.com/new-file.zip");
  });
}

test("empty, recovery, and startup failure states", async ({ browser }) => {
  for (const state of ["empty", "recovery", "fail"]) {
    const page = await browser.newPage();
    await mockDesktop(page, {
      empty: state === "empty",
      recovery: state === "recovery",
      fail: state === "fail",
    });
    await page.goto("/");
    if (state === "empty") {
      await expect(page.getByText("No downloads yet")).toBeVisible();
      await page.screenshot({ path: "test-results/empty.png" });
    }
    if (state === "recovery") {
      await expect(
        page.getByText("Your library needs attention"),
      ).toBeVisible();
      await page.getByRole("button", { name: "Acknowledge recovery" }).click();
      await expect(page.getByText("Your library needs attention")).toHaveCount(
        0,
      );
    }
    if (state === "fail")
      await expect(
        page.getByText("The state directory is already in use"),
      ).toBeVisible();
    await page.close();
  }
});

test("keyboard navigation, accessibility, and narrow layout", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "All downloads" }),
  ).toBeVisible();
  const result = await new AxeBuilder({ page }).analyze();
  expect(result.violations).toEqual([]);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  // Base UI's WebKit VoiceOver focus sentinels are not user controls.
  const dialogResult = await new AxeBuilder({ page })
    .exclude("[data-base-ui-focus-guard]")
    .analyze();
  expect(dialogResult.violations).toEqual([]);
  await page.keyboard.press("Escape");
  await page.setViewportSize({ width: 768, height: 800 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({ path: "test-results/narrow.png" });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole("button", { name: "Toggle Sidebar" }).click();
  await expect(
    page.getByRole("button", { name: "Downloads workspace", exact: true }),
  ).toBeVisible();
});
