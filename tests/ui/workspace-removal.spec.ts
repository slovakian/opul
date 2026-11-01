// Each scenario exercises consecutive interactions in the same rendered app.
/* oxlint-disable no-await-in-loop */
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import type { Snapshot } from "../../frontend/lib/model";
import { mockDesktop } from "./fixture";
import {
  breadcrumb,
  input,
  library,
  replaceSnapshot,
  sidebar,
} from "./workspace-sidebar-fixture";

const removalLibrary = (folderCount = 3) => {
  const saved = library();
  const [workspace] = saved.workspaces;
  workspace.name = "Research";
  workspace.folders.push({
    id: "drafts",
    is_default: false,
    name: "Drafts",
    path: "/Users/test/Drafts",
  });
  workspace.downloads[1].folder_id = "drafts";

  if (folderCount === 3) {
    workspace.folders.push({
      id: "review",
      is_default: false,
      name: "Review",
      path: "/Users/test/Review",
    });
    workspace.downloads.push({
      ...workspace.downloads[0],
      folder_id: "review",
      id: "review-download",
      name: "Report review.zip",
    });
  }

  return saved;
};

const withoutFolder = (
  saved: Snapshot,
  workspaceId: string,
  folderId: string
) => {
  const next = structuredClone(saved);
  const workspace = next.workspaces.find((item) => item.id === workspaceId);

  if (!workspace) {
    throw new Error("Missing test workspace");
  }

  workspace.folders = workspace.folders.filter((item) => item.id !== folderId);
  workspace.downloads = workspace.downloads.filter(
    (item) => item.folder_id !== folderId
  );

  if (!workspace.folders.some((item) => item.is_default)) {
    workspace.folders[0].is_default = true;
  }

  next.sequence += 10;
  next.revision += 10;

  return next;
};

const button = (page: Page, name: string) =>
  sidebar(page)
    .or(page.getByRole("dialog", { exact: true, name: "Workspace navigation" }))
    .getByRole("button", { exact: true, name });

const destination = (page: Page) =>
  page
    .locator("main > header")
    .getByRole("button", { name: /^Save location:/u });

const openFolder = async (page: Page, folderName: string) => {
  await button(page, "Toggle Research folders").click();
  await button(page, folderName).click();
  await input(page).fill("Report");
};

const removeFolder = async (page: Page, folderName: string) => {
  await button(page, `${folderName} folder actions`).click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Remove saved folder" })
    .click();
};

const selectArchive = async (page: Page) => {
  await destination(page).click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Archive" })
    .click();
};

const expectView = async (
  page: Page,
  scope: string,
  destinationName: string,
  downloads: string[]
) => {
  await expect(
    breadcrumb(page).getByRole("heading", { exact: true, name: scope })
  ).toBeVisible();
  await expect(
    breadcrumb(page).locator('[data-slot="breadcrumb-item"]')
  ).toHaveCount(scope === "Drafts" || scope === "Saved reports" ? 2 : 1);
  await expect(input(page)).toHaveValue("Report");
  await expect(destination(page)).toHaveAccessibleName(
    `Save location: ${destinationName}`
  );

  const activeName =
    scope === "Research" || scope === "Media" ? `${scope} workspace` : scope;

  await expect(button(page, activeName)).toHaveAttribute("data-active");
  await expect(sidebar(page).locator("[data-active]")).toHaveCount(1);

  for (const name of [
    "Report designs.zip",
    "Report interview.mp4",
    "Report review.zip",
    "Unrelated notes.pdf",
  ]) {
    const result = page.getByRole("button", { exact: true, name });

    await (downloads.includes(name)
      ? expect(result).toBeVisible()
      : expect(result).toHaveCount(0));
  }
};

for (const completion of ["immediate", "deferred"] as const) {
  test(`${completion} removal of the selected folder clears scope, status, and temporary destination only after confirmation`, async ({
    page,
  }) => {
    const saved = removalLibrary();
    await mockDesktop(page, { folderRemoval: completion, snapshot: saved });
    await page.goto("/");
    await openFolder(page, "Drafts");
    await page.getByRole("button", { name: "Filter downloads" }).click();
    await page
      .locator('[data-slot="popover-content"]')
      .getByRole("button", { exact: true, name: "Completed" })
      .click();
    await selectArchive(page);
    await removeFolder(page, "Drafts");

    if (completion === "deferred") {
      await expectView(page, "Drafts", "Media › Archive", [
        "Report interview.mp4",
      ]);
      await expect(button(page, "Drafts folder actions")).toBeFocused();
      const acceptedSequence = saved.sequence + 1;
      await expect
        .poll(() =>
          page.evaluate(() => sessionStorage.getItem("test-snapshot-sequence"))
        )
        .toBe(String(acceptedSequence));
      await replaceSnapshot(
        page,
        withoutFolder(saved, "workspace-1", "drafts")
      );
    }

    await expectView(page, "Research", "Saved reports", [
      "Report designs.zip",
      "Report review.zip",
    ]);
    await expect(button(page, "Research workspace")).toBeFocused();
    await expect(button(page, "Toggle Research folders")).toHaveAttribute(
      "aria-expanded",
      "true"
    );
    await expect(button(page, "Toggle Media folders")).toHaveAttribute(
      "aria-expanded",
      "false"
    );

    // Restoring the old catalog must not resurrect the removed folder scope.
    saved.sequence += 20;
    saved.revision += 20;
    await replaceSnapshot(page, saved);
    await expectView(page, "Research", "Saved reports", [
      "Report designs.zip",
      "Report interview.mp4",
      "Report review.zip",
    ]);
  });
}

for (const selectedFolder of ["Saved reports", "Drafts"]) {
  test(`two-to-one removal normalizes a ${selectedFolder} view and keeps other branches expanded`, async ({
    page,
  }) => {
    const saved = removalLibrary(2);
    await mockDesktop(page, { snapshot: saved });
    await page.goto("/");
    await button(page, "Toggle Media folders").click();
    await openFolder(page, selectedFolder);
    await selectArchive(page);
    await removeFolder(page, "Drafts");
    await expectView(page, "Research", "Saved reports", ["Report designs.zip"]);
    await expect(button(page, "Research workspace")).toBeFocused();
    await expect(button(page, "Toggle Research folders")).toHaveCount(0);
    await expect(button(page, "Saved reports")).toHaveCount(0);
    await expect(button(page, "Toggle Media folders")).toHaveAttribute(
      "aria-expanded",
      "true"
    );
  });
}

for (const invalidDestination of [false, true]) {
  test(`unselected removal preserves folder scope and ${invalidDestination ? "falls back from an invalid" : "keeps a valid"} temporary destination`, async ({
    page,
  }) => {
    const saved = removalLibrary();
    await mockDesktop(page, { snapshot: saved });
    await page.goto("/");
    await openFolder(page, "Saved reports");

    if (invalidDestination) {
      await destination(page).click();
      await page
        .locator('[data-slot="popover-content"]')
        .getByRole("button", { exact: true, name: "Drafts" })
        .click();
    } else {
      await selectArchive(page);
    }

    await removeFolder(page, "Drafts");
    await expectView(
      page,
      "Saved reports",
      invalidDestination ? "Saved reports" : "Media › Archive",
      ["Report designs.zip"]
    );
    await expect(button(page, "Research workspace")).toBeFocused();
  });
}

for (const scope of ["All downloads", "Media workspace"]) {
  test(`removal in another workspace preserves ${scope}, query, destination, and search focus`, async ({
    page,
  }) => {
    const saved = removalLibrary(2);
    await mockDesktop(page, { snapshot: saved });
    await page.goto("/");

    if (scope === "Media workspace") {
      await button(page, scope).click();
    }

    await input(page).fill("Report");
    await selectArchive(page);
    await input(page).focus();
    await replaceSnapshot(page, withoutFolder(saved, "workspace-1", "drafts"));
    const title = scope === "Media workspace" ? "Media" : scope;
    const destinationName = scope === "Media workspace" ? "Archive" : "Media";
    await expectView(
      page,
      title,
      destinationName,
      scope === "Media workspace" ? [] : ["Report designs.zip"]
    );
    await expect(input(page)).toBeFocused();
    await expect(button(page, "Toggle Research folders")).toHaveCount(0);
    await expect(button(page, "Toggle Media folders")).toHaveAttribute(
      "aria-expanded",
      "false"
    );
  });
}

test("a rejected removal preserves the selected view, destination, expansion, and action-trigger focus", async ({
  page,
}) => {
  await mockDesktop(page, {
    folderRemoval: "rejected",
    snapshot: removalLibrary(),
  });
  await page.goto("/");
  await openFolder(page, "Drafts");
  await selectArchive(page);
  await removeFolder(page, "Drafts");
  await expect(
    page.getByText("Couldn't remove saved folder. Try again.", { exact: true })
  ).toBeVisible();
  await expectView(page, "Drafts", "Media › Archive", ["Report interview.mp4"]);
  await expect(button(page, "Drafts folder actions")).toBeFocused();
  await expect(button(page, "Toggle Research folders")).toHaveAttribute(
    "aria-expanded",
    "true"
  );
});

test("a failed read during accepted retirement preserves the last confirmed view until polling succeeds", async ({
  page,
}) => {
  const saved = removalLibrary();
  await mockDesktop(page, { folderRemoval: "deferred", snapshot: saved });
  await page.goto("/");
  await openFolder(page, "Drafts");
  await selectArchive(page);
  await page.evaluate(() =>
    window.dispatchEvent(new Event("test:fail-snapshot"))
  );
  await removeFolder(page, "Drafts");
  await expect
    .poll(() =>
      page.evaluate(() => sessionStorage.getItem("test-snapshot-failed"))
    )
    .toBe("true");
  await expectView(page, "Drafts", "Media › Archive", ["Report interview.mp4"]);
  await expect(button(page, "Drafts folder actions")).toBeFocused();
  await replaceSnapshot(page, withoutFolder(saved, "workspace-1", "drafts"));
  await expectView(page, "Drafts", "Media › Archive", ["Report interview.mp4"]);
  await page.evaluate(() =>
    window.dispatchEvent(new Event("test:resume-snapshot"))
  );
  await expectView(page, "Research", "Saved reports", [
    "Report designs.zip",
    "Report review.zip",
  ]);
  await expect(button(page, "Research workspace")).toBeFocused();
});

for (const focusedControl of [
  "Drafts",
  "Drafts folder actions",
  "Toggle Research folders",
  "Saved reports",
]) {
  test(`polling a simple row restores disappearing ${focusedControl} focus`, async ({
    page,
  }) => {
    const saved = removalLibrary(2);
    await mockDesktop(page, { snapshot: saved });
    await page.goto("/");
    await openFolder(page, "Drafts");
    await button(page, focusedControl).focus();
    await replaceSnapshot(page, withoutFolder(saved, "workspace-1", "drafts"));
    await expectView(page, "Research", "Saved reports", ["Report designs.zip"]);
    await expect(button(page, "Research workspace")).toBeFocused();
  });
}

test("polling removal preserves focus on a surviving folder control", async ({
  page,
}) => {
  const saved = removalLibrary();
  await mockDesktop(page, { snapshot: saved });
  await page.goto("/");
  await openFolder(page, "Saved reports");
  await button(page, "Saved reports").focus();
  await replaceSnapshot(page, withoutFolder(saved, "workspace-1", "drafts"));
  await expectView(page, "Saved reports", "Saved reports", [
    "Report designs.zip",
  ]);
  await expect(button(page, "Saved reports")).toBeFocused();
});

for (const closeSheet of [false, true]) {
  test(`narrow removal ${closeSheet ? "retains closed-sheet trigger restoration" : "focuses the workspace in the open sheet"}`, async ({
    page,
  }) => {
    const saved = removalLibrary(2);
    await page.setViewportSize({ height: 800, width: 720 });
    await mockDesktop(page, { snapshot: saved });
    await page.goto("/");
    await input(page).fill("Report");

    const trigger = page
      .locator("main > header")
      .getByRole("button", { exact: true, name: "Toggle Sidebar" });

    await trigger.click();
    await button(page, "Toggle Research folders").click();
    await button(page, "Drafts folder actions").focus();

    if (closeSheet) {
      await page.keyboard.press("Escape");
      await expect(trigger).toBeFocused();
    }

    await replaceSnapshot(page, withoutFolder(saved, "workspace-1", "drafts"));
    await expect
      .poll(() =>
        page.evaluate(() => sessionStorage.getItem("test-snapshot-sequence"))
      )
      .toBe(String(saved.sequence + 10));

    if (closeSheet) {
      await expect(
        page.getByRole("dialog", { name: "Workspace navigation" })
      ).toHaveCount(0);
      await expect(trigger).toBeFocused();
    } else {
      await expect(button(page, "Research workspace")).toBeFocused();
      await expect(
        page.getByRole("dialog", { name: "Workspace navigation" })
      ).toBeVisible();
      await page.keyboard.press("Escape");
    }

    await expect(input(page)).toHaveValue("Report");
  });
}

test("removing the default folder selects the surviving default after confirmed normalization", async ({
  page,
}) => {
  const saved = removalLibrary(2);
  await mockDesktop(page, { snapshot: saved });
  await page.goto("/");
  await openFolder(page, "Saved reports");
  await removeFolder(page, "Saved reports");
  await expectView(page, "Research", "Drafts", ["Report interview.mp4"]);
  await expect(button(page, "Research workspace")).toBeFocused();
  await expect(destination(page)).toHaveAttribute(
    "title",
    "Drafts\n/Users/test/Drafts"
  );
});

test("an unrelated folder view falls back from a removed cross-workspace destination without navigating", async ({
  page,
}) => {
  const saved = removalLibrary(2);
  await mockDesktop(page, { snapshot: saved });
  await page.goto("/");
  await button(page, "Toggle Media folders").click();
  await button(page, "Archive").click();
  await input(page).fill("Report");
  await destination(page).click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Drafts" })
    .click();
  await input(page).focus();
  await replaceSnapshot(page, withoutFolder(saved, "workspace-1", "drafts"));
  await expect(
    breadcrumb(page).getByRole("heading", { exact: true, name: "Archive" })
  ).toBeVisible();
  await expect(
    breadcrumb(page).locator('[data-slot="breadcrumb-item"]')
  ).toHaveCount(2);
  await expect(button(page, "Archive")).toHaveAttribute("data-active");
  await expect(destination(page)).toHaveAccessibleName(
    "Save location: Archive"
  );
  await expect(
    page.getByText("No matching downloads", { exact: true })
  ).toBeVisible();
  await expect(input(page)).toHaveValue("Report");
  await expect(input(page)).toBeFocused();
  await expect(button(page, "Toggle Media folders")).toHaveAttribute(
    "aria-expanded",
    "true"
  );
});

test("normalization runs once and stale snapshots cannot restore the removed selection", async ({
  page,
}) => {
  const saved = removalLibrary();
  await mockDesktop(page, { snapshot: saved });
  await page.goto("/");
  await openFolder(page, "Drafts");
  const completed = withoutFolder(saved, "workspace-1", "drafts");
  await replaceSnapshot(page, completed);
  await expectView(page, "Research", "Saved reports", [
    "Report designs.zip",
    "Report review.zip",
  ]);
  await selectArchive(page);
  await input(page).focus();
  completed.sequence += 1;
  await replaceSnapshot(page, completed);
  await expect
    .poll(() =>
      page.evaluate(() => sessionStorage.getItem("test-snapshot-sequence"))
    )
    .toBe(String(completed.sequence));
  await expectView(page, "Research", "Media › Archive", [
    "Report designs.zip",
    "Report review.zip",
  ]);
  await replaceSnapshot(page, saved);
  await expect
    .poll(() =>
      page.evaluate(() => sessionStorage.getItem("test-snapshot-sequence"))
    )
    .toBe(String(saved.sequence));
  await expectView(page, "Research", "Media › Archive", [
    "Report designs.zip",
    "Report review.zip",
  ]);
  await expect(button(page, "Drafts")).toHaveCount(0);
  await expect(input(page)).toBeFocused();
  await page.screenshot({ path: "test-results/confirmed-folder-removal.png" });
});
