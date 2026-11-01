// Interactions and viewport changes share a page and must run in order.
/* oxlint-disable no-await-in-loop */
import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

import { mockDesktop } from "./fixture";
import {
  breadcrumb,
  input,
  library,
  replaceSnapshot,
  sidebar,
} from "./workspace-sidebar-fixture";

const multiFolderLibrary = () => {
  const value = library();
  value.workspaces[0].folders.push({
    id: "drafts",
    is_default: false,
    name: "Drafts",
    path: "/Users/test/Drafts",
  });
  value.workspaces[0].downloads[1].folder_id = "drafts";

  return value;
};

test("branches start collapsed and disclosure preserves folder scope, query, and contextual destination", async ({
  page,
}) => {
  const saved = multiFolderLibrary();
  await mockDesktop(page, { snapshot: saved });
  await page.goto("/");
  const [{ name }] = saved.workspaces;

  const row = sidebar(page).getByRole("button", {
    exact: true,
    name: `${name} workspace`,
  });

  const toggle = sidebar(page).getByRole("button", {
    name: `Toggle ${name} folders`,
  });

  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(
    sidebar(page).getByRole("button", { exact: true, name: "Drafts" })
  ).toHaveCount(0);
  await input(page).fill("Report");
  await row.click();
  await expect(row).toHaveAttribute("data-active");
  await expect(
    breadcrumb(page).getByRole("heading", { exact: true, name })
  ).toBeVisible();
  await expect(
    breadcrumb(page).locator('[data-slot="breadcrumb-item"]')
  ).toHaveCount(1);
  await expect(
    page.getByRole("button", { exact: true, name: "Report designs.zip" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Report interview.mp4" })
  ).toBeVisible();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await toggle.click();
  await expect(row).toHaveAttribute("data-active");

  const drafts = sidebar(page).getByRole("button", {
    exact: true,
    name: "Drafts",
  });

  await drafts.click();
  await expect(drafts).toHaveAttribute("data-active");
  await expect(row).not.toHaveAttribute("data-active");
  await expect(
    breadcrumb(page).getByRole("heading", { exact: true, name: "Drafts" })
  ).toBeVisible();
  await expect(
    breadcrumb(page).locator('[data-slot="breadcrumb-item"]')
  ).toHaveCount(2);
  await expect(
    page.getByRole("button", { exact: true, name: "Report designs.zip" })
  ).toHaveCount(0);
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(row).toHaveAttribute("data-active");
  await expect(
    breadcrumb(page).getByRole("heading", { exact: true, name: "Drafts" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Report interview.mp4" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Report designs.zip" })
  ).toHaveCount(0);
  await expect(input(page)).toHaveValue("Report");
  await expect(
    page.getByRole("button", { exact: true, name: "Save location: Drafts" })
  ).toHaveAttribute("title", "Drafts\n/Users/test/Drafts");
  await toggle.click();
  await expect(drafts).toHaveAttribute("data-active");
  await expect(row).not.toHaveAttribute("data-active");
  await input(page).fill("https://example.com/context.zip");
  await input(page).press("Enter");
  await expect
    .poll(() =>
      page.evaluate(() =>
        JSON.parse(sessionStorage.getItem("test-command") ?? "null")
      )
    )
    .toMatchObject({
      folder_id: "drafts",
      kind: "add_url",
      workspace_id: saved.workspaces[0].id,
    });
  await input(page).fill("Report");
  await breadcrumb(page).getByRole("button", { exact: true, name }).click();
  await expect(row).toHaveAttribute("data-active");
  await expect(drafts).not.toHaveAttribute("data-active");
  await expect(
    breadcrumb(page).locator('[data-slot="breadcrumb-item"]')
  ).toHaveCount(1);
  await expect(
    page.getByRole("button", { exact: true, name: "Report designs.zip" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Report interview.mp4" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", {
      exact: true,
      name: "Save location: Saved reports",
    })
  ).toBeVisible();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
});

test("disclosure preserves a cross-workspace temporary destination and status filter", async ({
  page,
}) => {
  const saved = multiFolderLibrary();
  await mockDesktop(page, { snapshot: saved });
  await page.goto("/");
  await sidebar(page)
    .getByRole("button", {
      exact: true,
      name: `${saved.workspaces[0].name} workspace`,
    })
    .click();
  await input(page).fill("Report");
  await page
    .getByRole("button", { exact: true, name: "Filter downloads" })
    .click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Completed" })
    .click();
  await page
    .getByRole("button", { exact: true, name: "Save location: Saved reports" })
    .click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Archive" })
    .click();

  for (const workspace of saved.workspaces) {
    const toggle = sidebar(page).getByRole("button", {
      name: `Toggle ${workspace.name} folders`,
    });

    await toggle.click();
    await toggle.click();
  }

  await expect(input(page)).toHaveValue("Report");
  await expect(
    breadcrumb(page).getByRole("heading", {
      exact: true,
      name: saved.workspaces[0].name,
    })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Report designs.zip" })
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { exact: true, name: "Report interview.mp4" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", {
      exact: true,
      name: "Save location: Media › Archive",
    })
  ).toBeVisible();
  await input(page).fill("https://example.com/temporary.zip");
  await input(page).press("Enter");
  await expect
    .poll(() =>
      page.evaluate(() =>
        JSON.parse(sessionStorage.getItem("test-command") ?? "null")
      )
    )
    .toMatchObject({
      folder_id: "archive",
      kind: "add_url",
      workspace_id: "media",
    });
});

test("independent session choices survive navigation, settings, polling, rename, reorder, and sidebar remounts", async ({
  page,
}) => {
  const saved = multiFolderLibrary();
  await mockDesktop(page, { persist: true, snapshot: saved });
  await page.goto("/");

  const firstToggle = sidebar(page).getByRole("button", {
    name: `Toggle ${saved.workspaces[0].name} folders`,
  });

  const mediaToggle = sidebar(page).getByRole("button", {
    name: "Toggle Media folders",
  });

  await firstToggle.click();
  await mediaToggle.click();
  await mediaToggle.click();
  await sidebar(page)
    .getByRole("button", { exact: true, name: "Media workspace" })
    .click();
  await expect(firstToggle).toHaveAttribute("aria-expanded", "true");
  await expect(mediaToggle).toHaveAttribute("aria-expanded", "false");

  for (const action of ["Cancel", "Save changes"]) {
    await sidebar(page)
      .getByRole("button", { exact: true, name: "Settings" })
      .click();
    await page
      .getByRole("dialog", { exact: true, name: "Settings" })
      .getByRole("button", { exact: true, name: action })
      .click();
    await expect(
      page.getByRole("dialog", { exact: true, name: "Settings" })
    ).toHaveCount(0);
    await expect(firstToggle).toHaveAttribute("aria-expanded", "true");
    await expect(mediaToggle).toHaveAttribute("aria-expanded", "false");
  }

  saved.workspaces[0].name = "Renamed research";
  saved.workspaces.reverse();
  const [media] = saved.workspaces;

  saved.workspaces.push({
    ...structuredClone(media),
    folders: media.folders.map((folder) => ({
      ...folder,
      id: `later-${folder.id}`,
    })),
    id: "later",
    name: "Later workspace",
  });
  saved.sequence += 1;
  saved.revision += 1;
  await replaceSnapshot(page, saved);

  const renamedToggle = sidebar(page).getByRole("button", {
    name: "Toggle Renamed research folders",
  });

  await expect(renamedToggle).toHaveAttribute("aria-expanded", "true");
  await expect(mediaToggle).toHaveAttribute("aria-expanded", "false");
  await expect(
    sidebar(page).getByRole("button", {
      name: "Toggle Later workspace folders",
    })
  ).toHaveAttribute("aria-expanded", "false");
  await expect(
    sidebar(page).getByRole("button", {
      name: /^(?:Media|Renamed research) workspace$/u,
    })
  ).toHaveText(["🍿Media", "📁Renamed research"]);

  for (const width of [720, 1220, 720]) {
    await page.setViewportSize({ height: 800, width });

    if (width === 720) {
      await page
        .locator("main > header")
        .getByRole("button", { exact: true, name: "Toggle Sidebar" })
        .click();

      const sheet = page.getByRole("dialog", {
        exact: true,
        name: "Workspace navigation",
      });

      await expect(
        sheet.getByRole("button", { name: "Toggle Renamed research folders" })
      ).toHaveAttribute("aria-expanded", "true");
      await expect(
        sheet.getByRole("button", { name: "Toggle Media folders" })
      ).toHaveAttribute("aria-expanded", "false");
      await page.keyboard.press("Escape");
      await expect(sheet).toHaveCount(0);
      await expect(
        page
          .locator("main > header")
          .getByRole("button", { exact: true, name: "Toggle Sidebar" })
      ).toBeFocused();
    } else {
      await expect(renamedToggle).toHaveAttribute("aria-expanded", "true");
      await expect(mediaToggle).toHaveAttribute("aria-expanded", "false");
    }
  }

  await page
    .locator("main > header")
    .getByRole("button", { exact: true, name: "Toggle Sidebar" })
    .click();

  const sheet = page.getByRole("dialog", {
    exact: true,
    name: "Workspace navigation",
  });

  await sheet
    .getByRole("button", { name: "Toggle Renamed research folders" })
    .click();
  await expect(sheet).toBeVisible();
  await page.keyboard.press("Escape");
  await page
    .locator("main > header")
    .getByRole("button", { exact: true, name: "Toggle Sidebar" })
    .click();
  await expect(
    sheet.getByRole("button", { name: "Toggle Renamed research folders" })
  ).toHaveAttribute("aria-expanded", "false");
  await sheet.getByRole("button", { name: "Toggle Media folders" }).click();
  await expect(
    sheet.getByRole("button", { name: "Toggle Media folders" })
  ).toHaveAttribute("aria-expanded", "true");
  await expect
    .poll(() =>
      page.evaluate(() =>
        JSON.parse(sessionStorage.getItem("test-preferences") ?? "null")
      )
    )
    .toMatchObject({ workspace_expansion: { media: true } });
  await page.reload();
  await page
    .locator("main > header")
    .getByRole("button", { exact: true, name: "Toggle Sidebar" })
    .click();
  await expect(
    sheet.getByRole("button", { name: "Toggle Media folders" })
  ).toHaveAttribute("aria-expanded", "true");
});

test("keyboard disclosure, navigation, artwork, and management remain independent in a narrow window", async ({
  page,
}) => {
  const saved = library();
  saved.workspaces[1].name =
    "Media and archived films with a very long workspace name";
  await page.setViewportSize({ height: 800, width: 720 });
  await mockDesktop(page, { snapshot: saved });
  await page.goto("/");
  await input(page).fill("Report");
  await page
    .locator("main > header")
    .getByRole("button", { exact: true, name: "Toggle Sidebar" })
    .focus();
  await page.keyboard.press("Enter");

  const sheet = page.getByRole("dialog", {
    exact: true,
    name: "Workspace navigation",
  });

  const [, { name }] = saved.workspaces;

  const row = sheet.getByRole("button", {
    exact: true,
    name: `${name} workspace`,
  });

  const toggle = sheet.getByRole("button", { name: `Toggle ${name} folders` });
  const artwork = row.locator('[data-slot="emoji-artwork"]');
  await expect(artwork).toBeVisible();
  await row.hover();
  await expect(artwork).not.toBeVisible();
  await expect(toggle).toHaveCSS("opacity", "1");
  await row.focus();
  await page.keyboard.press("Tab");
  await expect(toggle).toBeFocused();
  await expect(toggle).not.toHaveCSS("box-shadow", "none");
  await expect(toggle).toHaveCSS("transition-duration", "0s");
  await page.keyboard.press("Space");
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(sheet).toBeVisible();
  await expect(page.locator("main > header h1")).toHaveText("All downloads");
  await page.keyboard.press("Tab");

  const actions = sheet.getByRole("button", {
    exact: true,
    name: `${name} actions`,
  });

  await expect(actions).toBeFocused();
  await page.keyboard.press("Enter");
  const menu = page.locator('[data-slot="popover-content"]');

  for (const label of ["Change icon…", "Edit workspace", "Add saved folder"]) {
    await expect(
      menu.getByRole("button", { exact: true, name: label })
    ).toBeEnabled();
  }

  await page.keyboard.press("Escape");
  await expect(menu).toHaveCount(0);
  await expect(actions).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(
    sheet.getByRole("button", { exact: true, name: "Films" })
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(sheet).toHaveCount(0);
  await expect(
    breadcrumb(page).getByRole("heading", { exact: true, name: "Films" })
  ).toBeVisible();
  await expect(input(page)).toHaveValue("Report");
  await expect(
    page.getByText("No matching downloads", { exact: true })
  ).toBeVisible();
  await page
    .locator("main > header")
    .getByRole("button", { exact: true, name: "Toggle Sidebar" })
    .click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(
    sheet.getByRole("button", { exact: true, name: "Films" })
  ).toHaveAttribute("data-active");
  await toggle.focus();
  await page.keyboard.press("Enter");
  await expect(sheet).toBeVisible();
  await expect(row).toHaveAttribute("data-active");
  await page.keyboard.press("Space");
  await sheet
    .getByRole("button", { exact: true, name: "Films folder actions" })
    .click();
  await expect(
    menu.getByRole("button", { exact: true, name: "Set as default" })
  ).toBeDisabled();
  await expect(
    menu.getByRole("button", { exact: true, name: "Remove saved folder" })
  ).toBeEnabled();
  await page.keyboard.press("Escape");
  await sheet
    .getByRole("button", { exact: true, name: "Archive folder actions" })
    .click();
  await expect(
    menu.getByRole("button", { exact: true, name: "Set as default" })
  ).toBeEnabled();
  await expect(
    menu.getByRole("button", { exact: true, name: "Remove saved folder" })
  ).toBeEnabled();
  await page.keyboard.press("Escape");

  const accessibility = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .analyze();

  expect(accessibility.violations).toEqual([]);
  await page.screenshot({ path: "test-results/expandable-workspace-720.png" });
  await row.focus();
  await page.keyboard.press("Enter");
  await expect(sheet).toHaveCount(0);
  await expect(
    breadcrumb(page).getByRole("heading", { exact: true, name })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Save location: Films" })
  ).toBeVisible();
});
