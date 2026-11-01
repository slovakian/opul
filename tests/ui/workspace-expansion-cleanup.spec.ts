import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { mockDesktop, preferences } from "./fixture";
import {
  input,
  library,
  replaceSnapshot,
  sidebar,
} from "./workspace-sidebar-fixture";

test("confirmed workspace deletion prunes only its remembered choice and preserves unrelated navigation", async ({
  page,
}) => {
  const saved = library();
  await mockDesktop(page, {
    persist: true,
    preferences: {
      ...preferences,
      workspace_expansion: { media: true, "workspace-1": false },
    },
    snapshot: saved,
  });
  await page.goto("/");
  await sidebar(page)
    .getByRole("button", {
      exact: true,
      name: `${saved.workspaces[0].name} workspace`,
    })
    .click();
  await input(page).fill("Report");
  saved.workspaces.pop();
  saved.sequence += 1;
  await replaceSnapshot(page, saved);
  await expect(
    sidebar(page).getByRole("button", { exact: true, name: "Media workspace" })
  ).toHaveCount(0);
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          JSON.parse(sessionStorage.getItem("test-preferences") ?? "null")
            ?.workspace_expansion
      )
    )
    .toEqual({ "workspace-1": false });
  await expect(input(page)).toHaveValue("Report");
  await expect(
    sidebar(page).getByRole("button", {
      exact: true,
      name: `${saved.workspaces[0].name} workspace`,
    })
  ).toHaveAttribute("data-active", "");
});

const remembered = (page: Page) =>
  page.evaluate(
    () =>
      JSON.parse(sessionStorage.getItem("test-preferences") ?? "null")
        ?.workspace_expansion
  );

const attempts = (page: Page) =>
  page.evaluate(() =>
    JSON.parse(sessionStorage.getItem("test-expansion-attempts") ?? "[]")
  );

for (const failFirstExpansion of [false, true]) {
  test(`deletion supersedes an in-flight choice when the old write ${failFirstExpansion ? "fails" : "succeeds"}`, async ({
    page,
  }) => {
    const saved = library();
    await mockDesktop(page, {
      failFirstExpansion,
      holdFirstExpansion: true,
      persist: true,
      preferences: {
        ...preferences,
        workspace_expansion: { media: false, "workspace-1": true },
      },
      snapshot: saved,
    });
    await page.goto("/");
    await sidebar(page)
      .getByRole("button", { name: "Toggle Media folders" })
      .click();
    await expect.poll(() => attempts(page)).toEqual([{ media: true }]);
    saved.workspaces.pop();
    saved.sequence += 1;
    await replaceSnapshot(page, saved);
    await expect(
      sidebar(page).getByRole("button", {
        exact: true,
        name: "Media workspace",
      })
    ).toHaveCount(0);
    await page.evaluate(() =>
      window.dispatchEvent(new Event("test:release-expansion"))
    );

    if (failFirstExpansion) {
      const error = page
        .getByRole("alert")
        .filter({ hasText: "Couldn't save workspace expansion" });

      await expect(error).toBeVisible();
      saved.sequence += 1;
      await replaceSnapshot(page, saved);
      await expect
        .poll(() =>
          page.evaluate(() => sessionStorage.getItem("test-snapshot-sequence"))
        )
        .toBe(String(saved.sequence));
      await expect.poll(() => attempts(page)).toEqual([{ media: true }]);
      await error.getByRole("button", { exact: true, name: "Retry" }).click();
      await expect(error).toHaveCount(0);
    }

    await expect.poll(() => remembered(page)).toEqual({ "workspace-1": true });
    await expect
      .poll(() => attempts(page))
      .toEqual([{ media: true }, { media: null }]);
    await page.reload();
    await expect(
      sidebar(page).getByRole("button", {
        exact: true,
        name: "Media workspace",
      })
    ).toHaveCount(0);
    await expect.poll(() => remembered(page)).toEqual({ "workspace-1": true });
  });
}

test("deletion discards an unsaved failed choice without creating a redundant write", async ({
  page,
}) => {
  const saved = library();
  await mockDesktop(page, {
    failFirstExpansion: true,
    persist: true,
    preferences: {
      ...preferences,
      workspace_expansion: { "workspace-1": true },
    },
    snapshot: saved,
  });
  await page.goto("/");
  await sidebar(page)
    .getByRole("button", { name: "Toggle Media folders" })
    .click();

  const error = page
    .getByRole("alert")
    .filter({ hasText: "Couldn't save workspace expansion" });

  await expect(error).toBeVisible();
  saved.workspaces.pop();
  saved.sequence += 1;
  await replaceSnapshot(page, saved);
  await expect(
    sidebar(page).getByRole("button", { exact: true, name: "Media workspace" })
  ).toHaveCount(0);
  await expect(error).toHaveCount(0);
  await expect.poll(() => attempts(page)).toEqual([{ media: true }]);
  await page.reload();
  await expect.poll(() => attempts(page)).toEqual([{ media: true }]);
});

test("failed cleanup preserves prior choices until explicit Retry and preserves other settings", async ({
  page,
}) => {
  const saved = library();
  await mockDesktop(page, {
    failFirstExpansion: true,
    persist: true,
    preferences: {
      ...preferences,
      theme: "dark",
      workspace_expansion: { media: true, "workspace-1": false },
      workspace_expansion_startup: "collapsed",
    },
    snapshot: saved,
  });
  await page.goto("/");
  saved.workspaces.pop();
  saved.sequence += 1;
  await replaceSnapshot(page, saved);

  const error = page
    .getByRole("alert")
    .filter({ hasText: "Couldn't save workspace expansion" });

  await expect(error).toBeVisible();
  await expect.poll(() => remembered(page)).toBeUndefined();
  saved.sequence += 1;
  await replaceSnapshot(page, saved);
  await expect
    .poll(() =>
      page.evaluate(() => sessionStorage.getItem("test-snapshot-sequence"))
    )
    .toBe(String(saved.sequence));
  await expect.poll(() => attempts(page)).toEqual([{ media: null }]);
  await error.getByRole("button", { exact: true, name: "Retry" }).click();
  await expect(error).toHaveCount(0);
  await expect.poll(() => remembered(page)).toEqual({ "workspace-1": false });
  await expect
    .poll(() =>
      page.evaluate(() =>
        JSON.parse(sessionStorage.getItem("test-preferences") ?? "null")
      )
    )
    .toMatchObject({ theme: "dark", workspace_expansion_startup: "collapsed" });
});

test("failed snapshots and stale snapshots cannot confirm deletion; dormant choices survive a one-to-two transition", async ({
  page,
}) => {
  const saved = library();
  await mockDesktop(page, {
    persist: true,
    preferences: {
      ...preferences,
      workspace_expansion: { media: false, "workspace-1": true },
    },
    snapshot: saved,
  });
  await page.goto("/");

  const media = sidebar(page).getByRole("button", {
    name: "Toggle Media folders",
  });

  await expect(media).toHaveAttribute("aria-expanded", "false");
  const deleted = structuredClone(saved);
  deleted.workspaces.pop();
  deleted.sequence += 1;
  await page.evaluate(() =>
    window.dispatchEvent(new Event("test:fail-snapshot"))
  );
  await replaceSnapshot(page, deleted);
  await expect
    .poll(() =>
      page.evaluate(() => sessionStorage.getItem("test-snapshot-failed"))
    )
    .toBe("true");
  await expect(media).toBeVisible();
  await expect.poll(() => attempts(page)).toEqual([]);
  await replaceSnapshot(page, saved);
  await page.evaluate(() =>
    window.dispatchEvent(new Event("test:resume-snapshot"))
  );
  saved.sequence += 2;
  saved.workspaces[1].folders.pop();
  await replaceSnapshot(page, saved);
  await expect(media).toHaveCount(0);
  await expect.poll(() => attempts(page)).toEqual([]);
  saved.sequence += 1;
  saved.workspaces[1].folders.push({
    id: "new",
    is_default: false,
    name: "New",
    path: "/Users/test/New",
  });
  await replaceSnapshot(page, saved);
  await expect(media).toHaveAttribute("aria-expanded", "true");
  await expect
    .poll(() => remembered(page))
    .toEqual({ media: true, "workspace-1": true });
  await replaceSnapshot(page, deleted);
  await expect
    .poll(() =>
      page.evaluate(() => sessionStorage.getItem("test-snapshot-sequence"))
    )
    .toBe(String(deleted.sequence));
  await expect(media).toBeVisible();
  await expect.poll(() => attempts(page)).toEqual([{ media: true }]);
});

test("initial snapshot failure does not prune unread library entries", async ({
  page,
}) => {
  await mockDesktop(page, {
    fail: true,
    persist: true,
    preferences: { ...preferences, workspace_expansion: { media: true } },
    snapshot: library(),
  });
  await page.goto("/");
  await expect
    .poll(() =>
      page.evaluate(() => sessionStorage.getItem("test-snapshot-failed"))
    )
    .toBe("true");
  await expect.poll(() => attempts(page)).toEqual([]);
});

for (const workspaceDeletion of [
  "immediate",
  "deferred",
  "rejected",
] as const) {
  test(`${workspaceDeletion} deletion prunes only after confirmed disappearance`, async ({
    page,
  }) => {
    const saved = library();
    await mockDesktop(page, {
      persist: true,
      preferences: {
        ...preferences,
        workspace_expansion: { media: true, "workspace-1": false },
      },
      snapshot: saved,
      workspaceDeletion,
    });
    await page.goto("/");
    await sidebar(page)
      .getByRole("button", { exact: true, name: "Media workspace" })
      .click();
    await page
      .getByRole("button", { exact: true, name: "Workspace actions" })
      .click();
    await page
      .getByRole("button", { exact: true, name: "Delete workspace" })
      .click();
    await page
      .getByRole("dialog")
      .getByRole("button", { exact: true, name: "Delete workspace" })
      .click();

    if (workspaceDeletion === "rejected") {
      await expect(
        page.getByText("Couldn't complete that action", { exact: true })
      ).toBeVisible();
      await page
        .getByRole("dialog")
        .getByRole("button", { exact: true, name: "Keep it" })
        .click();
    } else {
      await expect(page.getByRole("dialog")).toHaveCount(0);
    }

    if (workspaceDeletion !== "immediate") {
      await expect(
        sidebar(page).getByRole("button", {
          exact: true,
          name: "Media workspace",
        })
      ).toBeVisible();
      await expect.poll(() => attempts(page)).toEqual([]);
      saved.sequence += 100;
      await replaceSnapshot(page, saved);
      await expect
        .poll(() =>
          page.evaluate(() => sessionStorage.getItem("test-snapshot-sequence"))
        )
        .toBe(String(saved.sequence));
      await expect.poll(() => attempts(page)).toEqual([]);

      if (workspaceDeletion === "rejected") {
        return;
      }

      saved.workspaces.pop();
      saved.sequence += 1;
      await replaceSnapshot(page, saved);
    }

    await expect(
      sidebar(page).getByRole("button", {
        exact: true,
        name: "Media workspace",
      })
    ).toHaveCount(0);
    await expect.poll(() => remembered(page)).toEqual({ "workspace-1": false });
  });
}

test("Retry after failed writes and confirmed deletion removes durable stale intent and saves the other latest choice", async ({
  page,
}) => {
  const saved = library();
  saved.workspaces[0].folders.push({
    id: "extra",
    is_default: false,
    name: "Extra",
    path: "/Users/test/Extra",
  });
  await mockDesktop(page, {
    failExpansionAttempts: 3,
    persist: true,
    preferences: {
      ...preferences,
      workspace_expansion: { media: false, "workspace-1": false },
    },
    snapshot: saved,
  });
  await page.goto("/");
  await sidebar(page)
    .getByRole("button", { name: "Toggle Media folders" })
    .click();

  const error = page
    .getByRole("alert")
    .filter({ hasText: "Couldn't save workspace expansion" });

  await expect(error).toBeVisible();
  await sidebar(page)
    .getByRole("button", { name: `Toggle ${saved.workspaces[0].name} folders` })
    .click();
  await expect.poll(() => attempts(page)).toHaveLength(2);
  saved.workspaces.pop();
  saved.sequence += 1;
  await replaceSnapshot(page, saved);
  await expect.poll(() => attempts(page)).toHaveLength(3);
  await expect(
    error.getByRole("button", { exact: true, name: "Retry" })
  ).toBeEnabled();
  await error.getByRole("button", { exact: true, name: "Retry" }).click();
  await expect(error).toHaveCount(0);
  await expect.poll(() => remembered(page)).toEqual({ "workspace-1": true });
  await expect
    .poll(() => attempts(page))
    .toEqual([
      { media: true },
      { media: true, "workspace-1": true },
      { media: null, "workspace-1": true },
      { media: null, "workspace-1": true },
    ]);
});
