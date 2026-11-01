/* oxlint-disable no-await-in-loop */
import { expect, test } from "@playwright/test";

import { mockDesktop } from "./fixture";
import {
  addFolder,
  additionLibrary,
  chooseMedia,
  confirmFolder,
  folderRequest,
} from "./saved-folder-fixture";
import { breadcrumb, input, sidebar } from "./workspace-sidebar-fixture";

for (const entry of ["sidebar", "search"] as const) {
  test(`${entry} polling can confirm an accepted addition while its original refresh is still held`, async ({
    page,
  }) => {
    await mockDesktop(page, {
      folderAddition: "deferred",
      folderPickerPath: "/Users/test/New reports",
      holdFolderRefresh: true,
      snapshot: additionLibrary(1),
    });
    await page.goto("/");
    await sidebar(page)
      .getByRole("button", { exact: true, name: "Research workspace" })
      .click();
    await chooseMedia(page);
    await input(page).fill("Report");
    await addFolder(page, entry);
    await expect
      .poll(() =>
        page.evaluate(() => sessionStorage.getItem("test-held-folder-refresh"))
      )
      .toBe("true");
    await confirmFolder(page);
    await expect(
      breadcrumb(page).getByRole("heading", {
        exact: true,
        name: entry === "sidebar" ? "New reports" : "Research",
      })
    ).toBeVisible();
    await expect(
      sidebar(page).getByRole("button", { name: "Toggle Media folders" })
    ).toHaveAttribute("aria-expanded", "true");
    await expect(
      page.getByRole("button", { name: /^Save location:/u })
    ).toHaveAttribute(
      "title",
      `${entry === "search" ? "Media › " : ""}New reports\n/Users/test/New reports`
    );
    await page.evaluate(() =>
      window.dispatchEvent(new Event("test:release-folder-refresh"))
    );
    await expect(
      page.getByRole("button", { exact: true, name: "Filter downloads" })
    ).toBeEnabled();
    await expect(input(page)).toHaveValue("Report");
    await expect(
      sidebar(page).getByRole("button", { exact: true, name: "New reports" })
    ).toBeVisible();
    await expect
      .poll(() =>
        page.evaluate(() => sessionStorage.getItem("test-folder-attempts"))
      )
      .toBe("1");
  });

  test(`${entry} waits for a non-null accepted receipt even when polling already contains the folder`, async ({
    page,
  }) => {
    await mockDesktop(page, {
      folderPickerPath: "/Users/test/New reports",
      holdFolderReceipt: true,
      snapshot: additionLibrary(2),
    });
    await page.goto("/");
    await sidebar(page)
      .getByRole("button", { exact: true, name: "Research workspace" })
      .click();
    await chooseMedia(page);
    await input(page).fill("Report");
    await addFolder(page, entry);
    await expect
      .poll(() =>
        page.evaluate(() => sessionStorage.getItem("test-held-folder-receipt"))
      )
      .toBe("true");

    // Disclosure remains available while the command response is held.
    const toggle = sidebar(page).getByRole("button", {
      name: "Toggle Media folders",
    });

    await toggle.click();
    await expect(
      sidebar(page).getByRole("button", { exact: true, name: "New reports" })
    ).toBeVisible();
    await toggle.click();
    await expect(
      breadcrumb(page).getByRole("heading", { exact: true, name: "Research" })
    ).toBeVisible();
    await expect(
      page.getByRole("button", {
        exact: true,
        name: "Save location: Media › Films",
      })
    ).toBeVisible();
    await expect(
      sidebar(page).getByRole("button", { name: "Toggle Media folders" })
    ).toHaveAttribute("aria-expanded", "false");
    await page.evaluate(() =>
      window.dispatchEvent(new Event("test:release-folder-receipt"))
    );
    await expect(
      page.getByRole("button", { name: /^Save location:/u })
    ).toHaveAttribute(
      "title",
      `${entry === "search" ? "Media › " : ""}New reports\n/Users/test/New reports`
    );
    await expect(
      breadcrumb(page).getByRole("heading", {
        exact: true,
        name: entry === "sidebar" ? "New reports" : "Research",
      })
    ).toBeVisible();
  });
}

test("search addition preserves an expanded folder view, results, and filter", async ({
  page,
}) => {
  const saved = additionLibrary(2);
  const [, media] = saved.workspaces;
  media.downloads = [
    {
      ...saved.workspaces[0].downloads[1],
      folder_id: "archive",
      id: "media-report",
      name: "Report film.mp4",
    },
  ];
  await mockDesktop(page, {
    folderPickerPath: "/Users/test/New reports",
    snapshot: saved,
  });
  await page.goto("/");
  await sidebar(page)
    .getByRole("button", { name: "Toggle Media folders" })
    .click();
  await sidebar(page)
    .getByRole("button", { exact: true, name: "Archive" })
    .click();
  await input(page).fill("Report");
  await page
    .getByRole("button", { exact: true, name: "Filter downloads" })
    .click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Completed" })
    .click();
  await addFolder(page, "search");
  await expect(
    page.getByRole("button", {
      exact: true,
      name: "Save location: New reports",
    })
  ).toBeVisible();
  await expect(
    sidebar(page).getByRole("button", { name: "Toggle Media folders" })
  ).toHaveAttribute("aria-expanded", "true");
  await expect(
    sidebar(page).getByRole("button", { exact: true, name: "Archive" })
  ).toHaveAttribute("data-active");
  await expect(
    breadcrumb(page).getByRole("heading", { exact: true, name: "Archive" })
  ).toBeVisible();
  await expect(
    breadcrumb(page).getByRole("button", { exact: true, name: "Media" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Report film.mp4" })
  ).toBeVisible();
  await expect(input(page)).toHaveValue("Report");
  await page
    .getByRole("button", { exact: true, name: "Completed downloads" })
    .click();
  await expect(
    page
      .locator('[data-slot="popover-content"]')
      .getByRole("button", { exact: true, name: "Completed" })
  ).toHaveAttribute("data-active");
});

for (const closeBeforeConfirmation of [false, true]) {
  test(`sidebar confirmation ${closeBeforeConfirmation ? "preserves a closed" : "closes an open"} narrow sheet`, async ({
    page,
  }) => {
    await page.setViewportSize({ height: 800, width: 720 });
    await mockDesktop(page, {
      folderAddition: "deferred",
      folderPickerPath: "/Users/test/New reports",
      snapshot: additionLibrary(1),
    });
    await page.goto("/");
    await input(page).fill("Report");

    const trigger = page
      .locator("main > header")
      .getByRole("button", { exact: true, name: "Toggle Sidebar" });

    await trigger.click();
    await addFolder(page, "sidebar");
    await folderRequest(page);

    const sheet = page.getByRole("dialog", {
      exact: true,
      name: "Workspace navigation",
    });

    await expect(sheet).toBeVisible();

    if (closeBeforeConfirmation) {
      await page.keyboard.press("Escape");
      await expect(sheet).toHaveCount(0);
      await input(page).focus();
    }

    await confirmFolder(page);
    await expect(
      breadcrumb(page).getByRole("heading", {
        exact: true,
        name: "New reports",
      })
    ).toBeVisible();
    await expect(sheet).toHaveCount(0);
    await expect(input(page)).toHaveValue("Report");

    await expect(closeBeforeConfirmation ? input(page) : trigger).toBeFocused();

    await trigger.click();
    await expect(
      sheet.getByRole("button", { name: "Toggle Media folders" })
    ).toHaveAttribute("aria-expanded", "true");
    await expect(
      sheet.getByRole("button", { exact: true, name: "New reports" })
    ).toHaveAttribute("data-active");
  });
}

test("sidebar addition reveals the selected folder at the end of a long branch without moving focus", async ({
  page,
}) => {
  const saved = additionLibrary(2);

  for (let index = 0; index < 35; index += 1) {
    saved.workspaces[1].folders.push({
      id: `extra-${index}`,
      is_default: false,
      name: `Folder ${index}`,
      path: `/Users/test/Folder ${index}`,
    });
  }

  await mockDesktop(page, {
    folderAddition: "deferred",
    folderPickerPath: "/Users/test/New reports",
    snapshot: saved,
  });
  await page.goto("/");
  await addFolder(page, "sidebar");
  await folderRequest(page);
  await input(page).focus();
  await confirmFolder(page);

  const folder = sidebar(page).getByRole("button", {
    exact: true,
    name: "New reports",
  });

  await expect(folder).toHaveAttribute("data-active");
  await expect(folder).toBeInViewport();
  await expect(input(page)).toBeFocused();
  await page.screenshot({ path: "test-results/confirmed-folder-revealed.png" });
});

for (const entry of ["sidebar", "search"] as const) {
  test(`${entry} confirmed second-folder expansion is remembered across relaunch`, async ({
    page,
  }) => {
    await mockDesktop(page, {
      folderPickerPath: "/Users/test/New reports",
      persist: true,
      snapshot: additionLibrary(1),
    });
    await page.goto("/");
    await sidebar(page)
      .getByRole("button", { exact: true, name: "Media workspace" })
      .click();
    await addFolder(page, entry);
    await expect(
      sidebar(page).getByRole("button", { name: "Toggle Media folders" })
    ).toHaveAttribute("aria-expanded", "true");
    await expect
      .poll(() =>
        page.evaluate(() =>
          JSON.parse(sessionStorage.getItem("test-preferences") ?? "null")
        )
      )
      .toMatchObject({ workspace_expansion: { media: true } });
    await page.reload();
    await expect(
      sidebar(page).getByRole("button", { name: "Toggle Media folders" })
    ).toHaveAttribute("aria-expanded", "true");
    await expect(
      sidebar(page).getByRole("button", { exact: true, name: "New reports" })
    ).toBeVisible();
  });
}

test("late search confirmation preserves a newer destination chosen after navigation", async ({
  page,
}) => {
  await mockDesktop(page, {
    folderAddition: "deferred",
    folderPickerPath: "/Users/test/New reports",
    snapshot: additionLibrary(2),
  });
  await page.goto("/");
  await sidebar(page)
    .getByRole("button", { exact: true, name: "Research workspace" })
    .click();
  await chooseMedia(page);
  await addFolder(page, "search");
  await folderRequest(page);
  await sidebar(page)
    .getByRole("button", { exact: true, name: "Media workspace" })
    .click();
  await page
    .getByRole("button", { exact: true, name: "Save location: Films" })
    .click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Archive" })
    .click();
  await expect(
    page.getByRole("button", { exact: true, name: "Save location: Archive" })
  ).toBeVisible();
  await confirmFolder(page);
  await expect(
    page.getByRole("button", { exact: true, name: "Save location: Archive" })
  ).toBeVisible();
  await input(page).fill("https://example.com/newer-choice.zip");
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

for (const entry of ["sidebar", "search"] as const) {
  test(`${entry} second-folder snapshot waits for command acceptance before expansion`, async ({
    page,
  }) => {
    await mockDesktop(page, {
      folderPickerPath: "/Users/test/New reports",
      holdFolderReceipt: true,
      persist: true,
      snapshot: additionLibrary(1),
    });
    await page.goto("/");
    await sidebar(page)
      .getByRole("button", { exact: true, name: "Media workspace" })
      .click();
    await input(page).fill("Report");
    await addFolder(page, entry);
    await expect
      .poll(() =>
        page.evaluate(() => sessionStorage.getItem("test-held-folder-receipt"))
      )
      .toBe("true");
    await expect
      .poll(() =>
        page.evaluate(() =>
          Number(sessionStorage.getItem("test-snapshot-sequence"))
        )
      )
      .toBeGreaterThan(1);

    const toggle = sidebar(page).getByRole("button", {
      name: "Toggle Media folders",
    });

    await expect(toggle).toHaveAttribute("aria-expanded", "false");
    await expect
      .poll(() =>
        page.evaluate(() => sessionStorage.getItem("test-expansion-attempts"))
      )
      .toBeNull();
    await expect(
      breadcrumb(page).getByRole("heading", { exact: true, name: "Media" })
    ).toBeVisible();
    await expect(input(page)).toHaveValue("Report");
    await page.evaluate(() =>
      window.dispatchEvent(new Event("test:release-folder-receipt"))
    );
    await expect(toggle).toHaveAttribute("aria-expanded", "true");
    await expect(
      page.getByRole("button", {
        exact: true,
        name: "Save location: New reports",
      })
    ).toBeVisible();
  });
}
