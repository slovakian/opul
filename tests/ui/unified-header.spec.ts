// Browser actions share one page and must run in order.
/* oxlint-disable no-await-in-loop */
import { expect, test } from "@playwright/test";

import { mockDesktop, snapshot } from "./fixture";

test("workspace identity stays first when navigating into a saved folder", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  library.workspaces[0].name = "Research";
  library.workspaces[0].icon = { kind: "Emoji", value: "🤢" };
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");

  const breadcrumb = page
    .locator("main > header")
    .getByRole("navigation", { name: "breadcrumb" });

  await expect(breadcrumb.locator('[data-slot="emoji-artwork"]')).toHaveCount(
    0
  );
  await page.getByRole("button", { exact: true, name: "Downloads" }).click();
  await expect(breadcrumb.locator('[data-slot="breadcrumb-item"]')).toHaveText([
    "🤢Research",
    "Downloads",
  ]);

  const workspace = breadcrumb.getByRole("button", {
    exact: true,
    name: "Research",
  });

  await expect(workspace.locator('[data-slot="emoji-artwork"]')).toHaveText(
    "🤢"
  );
  await expect(
    breadcrumb.getByRole("heading", { exact: true, name: "Downloads" })
  ).toHaveAttribute("aria-current", "page");
  await workspace.click();
  await expect(breadcrumb.locator('[data-slot="breadcrumb-item"]')).toHaveText([
    "🤢Research",
  ]);
  await expect(
    breadcrumb.getByRole("heading", { exact: true, name: "Research" })
  ).toHaveAttribute("aria-current", "page");
});

test("one header row preserves searches across views and breadcrumb navigation", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  library.workspaces[0].folders.push({
    id: "archive",
    is_default: false,
    name: "Archive",
    path: "/Users/test/Archive",
  });
  library.workspaces[0].downloads[2].folder_id = "archive";
  library.workspaces[0].downloads[2].state = "Finished";
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  const header = page.locator("main > header");
  await expect(header.locator("input")).toHaveCount(1);

  const input = header.getByRole("searchbox", {
    name: "Search downloads or paste a link",
  });

  await expect(header.getByRole("button", { name: "Clear input" })).toHaveCount(
    0
  );
  await input.fill("Field notes");
  await expect(
    page.getByRole("button", { exact: true, name: "Design resources.zip" })
  ).toHaveCount(0);
  await page.getByRole("button", { exact: true, name: "In progress" }).click();
  await expect(input).toHaveValue("Field notes");
  await expect(page.getByText("No matching downloads")).toBeVisible();
  await page.getByRole("button", { exact: true, name: "Completed" }).click();
  await expect(
    page.getByRole("button", { exact: true, name: "Field notes.pdf" })
  ).toBeVisible();
  await page.getByRole("button", { exact: true, name: "Archive" }).click();
  await expect(input).toHaveValue("Field notes");
  const breadcrumb = header.getByRole("navigation", { name: "breadcrumb" });
  await expect(
    breadcrumb.getByRole("heading", { exact: true, name: "Archive" })
  ).toHaveAttribute("aria-current", "page");
  await expect(
    breadcrumb.getByRole("button", { exact: true, name: "Archive" })
  ).toHaveCount(0);
  await breadcrumb
    .getByRole("button", { exact: true, name: "Downloads" })
    .click();
  await expect(
    header.getByRole("heading", { exact: true, name: "Downloads" })
  ).toBeVisible();
  await expect(input).toHaveValue("Field notes");
  await header.getByRole("button", { name: "Clear input" }).click();
  await expect(input).toHaveValue("");
  await expect(input).toBeFocused();
  await expect(
    page.getByRole("button", { exact: true, name: "Design resources.zip" })
  ).toBeVisible();
});

test("only complete supported links enable download, and typing never submits", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  const header = page.locator("main > header");

  const input = header.getByRole("searchbox", {
    name: "Search downloads or paste a link",
  });

  const download = header.getByRole("button", {
    exact: true,
    name: "Download link",
  });

  await input.fill("report.pdf");
  await expect(download).toBeDisabled();
  await expect(header.getByText("Search query", { exact: true })).toBeVisible();
  await input.press("Enter");
  await expect(input).toHaveValue("report.pdf");
  await expect(
    page.getByText("Use an HTTP, HTTPS, or magnet link.")
  ).toHaveCount(0);

  await input.fill("https://example.com/resources.zip");
  await expect(download).toBeEnabled();
  await expect(
    header.getByText("Download link recognized", { exact: true })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "new-file.zip" })
  ).toHaveCount(0);

  await input.fill("https://example.com/resources.zip");
  await expect(
    page.getByRole("button", { exact: true, name: "Design resources.zip" })
  ).toBeVisible();
  await input.press("Enter");
  await expect(input).toHaveValue("");
  await expect(
    page.getByRole("button", { exact: true, name: "new-file.zip" })
  ).toBeVisible();
});

test("Escape dismisses overlays before clearing the unified input", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");

  const input = page.getByRole("searchbox", {
    name: "Search downloads or paste a link",
  });

  await input.fill("keep this draft");

  for (const name of [
    "Filter downloads",
    "Date added: sort and group",
    "Save location: Downloads › Downloads",
    "Workspace actions",
  ]) {
    await page.getByRole("button", { exact: true, name }).click();
    await expect(page.locator('[data-slot="popover-content"]')).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.locator('[data-slot="popover-content"]')).toHaveCount(0);
    await expect(input).toHaveValue("keep this draft");
  }

  await page
    .getByRole("button", { exact: true, name: "Download options" })
    .click();
  await expect(
    page.getByRole("button", { exact: true, name: "Download type: Automatic" })
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(input).toHaveValue("keep this draft");
  await page
    .getByRole("button", { exact: true, name: "Filter downloads" })
    .click();
  await input.focus();
  await expect(page.locator('[data-slot="popover-content"]')).toBeVisible();
  await input.press("Escape");
  await expect(page.locator('[data-slot="popover-content"]')).toHaveCount(0);
  await expect(input).toHaveValue("keep this draft");
  await input.focus();
  await input.press("Escape");
  await expect(input).toHaveValue("");
});

test("search and new-download shortcuts and menus focus the shared input", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");

  const input = page.getByRole("searchbox", {
    name: "Search downloads or paste a link",
  });

  await input.fill("replace me");

  for (const modifier of ["Meta", "Control"]) {
    await page.getByRole("heading", { name: "All downloads" }).click();
    await page.keyboard.press(`${modifier}+f`);
    await expect(input).toBeFocused();
    await page.keyboard.press(`${modifier}+n`);
    await expect(input).toBeFocused();
    expect(
      await input.evaluate((element: HTMLInputElement) =>
        element.value.slice(
          element.selectionStart ?? 0,
          element.selectionEnd ?? 0
        )
      )
    ).toBe("replace me");
  }

  for (const name of ["Workspace actions", "Opul"]) {
    await page.getByRole("button", { exact: true, name }).click();
    await page
      .locator('[data-slot="popover-content"]')
      .getByRole("button", { exact: true, name: "New download" })
      .click();
    await expect(input).toBeFocused();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    expect(
      await input.evaluate((element: HTMLInputElement) => element.selectionEnd)
    ).toBe("replace me".length);
  }
});

test("header stays in one row and reserves traffic light space only when navigation is hidden", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  const header = page.locator("main > header");
  const toggle = header.getByRole("button", { name: "Toggle Sidebar" });

  for (const width of [1220, 720]) {
    await page.setViewportSize({ height: 800, width });

    for (const collapsed of [false, true]) {
      if (width === 1220 && collapsed) {
        await toggle.click();
      }

      const bounds = await header.boundingBox();

      if (!bounds) {
        throw new Error("Header must be visible");
      }

      expect(bounds.height).toBe(56);

      const controls = header.locator(
        'input, button[aria-label="Workspace actions"], button[aria-label="Toggle Sidebar"]'
      );

      for (const control of await controls.all()) {
        const box = await control.boundingBox();

        if (!box) {
          throw new Error("Header controls must be visible");
        }

        expect(box.y).toBeGreaterThanOrEqual(bounds.y);
        expect(box.y + box.height).toBeLessThanOrEqual(bounds.y + 56);
      }

      const box = await toggle.boundingBox();

      if (!box) {
        throw new Error("Sidebar toggle must be visible");
      }

      expect(box.x).toBeGreaterThanOrEqual(
        width === 720 || collapsed ? 80 : 256
      );
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth
        )
      ).toBe(true);

      if (width === 1220 && collapsed) {
        await toggle.click();
      }
    }
  }

  await page.screenshot({ path: "test-results/unified-header-720.png" });
});

test("New download closes overlay navigation and focuses the input", async ({
  page,
}) => {
  await page.setViewportSize({ height: 800, width: 720 });
  await mockDesktop(page);
  await page.goto("/");

  const input = page.getByRole("searchbox", {
    name: "Search downloads or paste a link",
  });

  await input.fill("replace me");
  await page
    .locator("main > header")
    .getByRole("button", { name: "Toggle Sidebar" })
    .click();
  await page.getByRole("button", { exact: true, name: "Opul" }).click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "New download" })
    .click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(input).toBeFocused();
  await page.keyboard.type("new draft");
  await expect(input).toHaveValue("new draft");
});
