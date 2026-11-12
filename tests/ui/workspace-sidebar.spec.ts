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

test("workspace labels share the top nav icon→label left edge", async ({
  page,
}) => {
  const saved = library();
  // Wide ZWJ sequences must not expand the emoji slot past lucide size-4.
  saved.workspaces[0].icon = { kind: "Emoji", value: "👩‍💻" };
  saved.workspaces[1].icon = { kind: "Emoji", value: "🏳️‍🌈" };
  await mockDesktop(page, { snapshot: saved });
  await page.goto("/");

  const edges = await sidebar(page).evaluate((root) => {
    const aggregates: number[] = [];
    const workspaces: number[] = [];

    for (const button of root.querySelectorAll(
      '[data-slot="sidebar-menu-button"]'
    )) {
      const spans = button.querySelectorAll(":scope > span");
      const label = spans.item(spans.length - 1);
      const left = label?.getBoundingClientRect().left;

      if (left === undefined) {
        continue;
      }

      if (aggregates.length < 3) {
        aggregates.push(left);
      }

      if (
        button instanceof HTMLElement &&
        Object.hasOwn(button.dataset, "workspaceNavigation")
      ) {
        workspaces.push(left);
      }
    }

    return { aggregates, workspaces };
  });

  expect(edges.aggregates).toHaveLength(3);
  expect(edges.workspaces.length).toBeGreaterThanOrEqual(2);

  const [expected] = edges.aggregates;

  expect(expected).toEqual(expect.any(Number));

  for (const left of [...edges.aggregates, ...edges.workspaces]) {
    expect(left).toBeCloseTo(expected, 0);
  }
});

test("one saved folder has a simple workspace row with persistent artwork and keyboard actions", async ({
  page,
}) => {
  const saved = library();
  await mockDesktop(page, { snapshot: saved });
  await page.goto("/");
  const [workspace] = saved.workspaces;

  const row = sidebar(page).getByRole("button", {
    exact: true,
    name: `${workspace.name} workspace`,
  });

  const artwork = row.locator('[data-slot="emoji-artwork"]');

  await expect(
    sidebar(page).getByText("Workspaces", { exact: true })
  ).toBeVisible();
  await expect(
    sidebar(page).getByRole("button", {
      name: /^(?:Research|Media).* workspace$/u,
    })
  ).toHaveText([`📁${workspace.name}`, "🍿Media"]);
  await expect(
    sidebar(page).getByRole("button", {
      name: `Toggle ${workspace.name} folders`,
    })
  ).toHaveCount(0);
  await expect(
    sidebar(page).getByRole("button", { exact: true, name: "Saved reports" })
  ).toHaveCount(0);
  await expect(artwork).toHaveAttribute("data-preset", "gold");
  await row.hover();
  await expect(artwork).toBeVisible();
  await row.focus();
  await expect(artwork).toBeVisible();
  await expect(row).toBeFocused();
  await page.keyboard.press("Tab");
  await row.focus();
  await expect(row).toBeFocused();
  await expect(row).not.toHaveCSS("box-shadow", "none");
  await input(page).fill("Report");
  await row.focus();
  await page.keyboard.press("Enter");
  await expect(
    breadcrumb(page).getByRole("heading", { exact: true, name: workspace.name })
  ).toBeVisible();
  await expect(
    breadcrumb(page).locator('[data-slot="breadcrumb-item"]')
  ).toHaveCount(1);
  await expect(row).toHaveAttribute("data-active");
  await expect(input(page)).toHaveValue("Report");
  await expect(
    page.getByRole("button", { exact: true, name: "Report designs.zip" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Report interview.mp4" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Unrelated notes.pdf" })
  ).toHaveCount(0);
  await expect(
    page.locator("main > header").getByRole("button", {
      exact: true,
      name: "Save location: Saved reports",
    })
  ).toHaveAttribute(
    "title",
    "Saved reports\n/Users/test/Research/Saved reports"
  );

  const actions = sidebar(page).getByRole("button", {
    exact: true,
    name: `${workspace.name} actions`,
  });

  await actions.focus();
  await expect(artwork).toBeVisible();
  await page.keyboard.press("Space");
  const menu = page.locator('[data-slot="popover-content"]');
  await expect(menu.locator('[data-slot="sidebar"]')).toBeVisible();
  await expect(
    menu.getByRole("button", { exact: true, name: "Change icon…" })
  ).toBeEnabled();
  await expect(
    menu.getByRole("button", { exact: true, name: "Edit workspace" })
  ).toBeEnabled();
  await expect(
    menu.getByRole("button", { exact: true, name: "Add saved folder" })
  ).toBeEnabled();
  await page.keyboard.press("Escape");
  await expect(actions).toBeFocused();

  const accessibility = await new AxeBuilder({ page })
    .include('[aria-label="Workspace navigation"]')
    .analyze();

  expect(accessibility.violations).toEqual([]);
  await page.screenshot({ path: "test-results/simple-workspace-sidebar.png" });
});

test("an empty workspace keeps custom artwork and keyboard navigation in the narrow sidebar", async ({
  page,
}) => {
  const saved = library();
  saved.workspaces[0].downloads = [];
  saved.workspaces[0].appearance.preference = {
    brightness: 120,
    hue: 90,
    kind: "Custom",
    saturation: 80,
  };
  await page.setViewportSize({ height: 800, width: 720 });
  await mockDesktop(page, { snapshot: saved });
  await page.goto("/");
  await input(page).fill("Report");
  await page
    .locator("main > header")
    .getByRole("button", { exact: true, name: "Toggle Sidebar" })
    .click();

  const row = page
    .getByRole("dialog", { exact: true, name: "Workspace navigation" })
    .getByRole("button", {
      exact: true,
      name: `${saved.workspaces[0].name} workspace`,
    });

  const artwork = row.locator('[data-slot="emoji-artwork"]');
  await row.hover();
  await expect(artwork).toBeVisible();
  await row.focus();
  await expect(artwork).toBeVisible();
  await expect(artwork).toHaveCSS(
    "filter",
    "hue-rotate(90deg) saturate(0.8) brightness(1.2)"
  );
  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    breadcrumb(page).getByRole("heading", {
      exact: true,
      name: saved.workspaces[0].name,
    })
  ).toBeVisible();
  await expect(input(page)).toHaveValue("Report");
  await expect(
    page.getByText("No matching downloads", { exact: true })
  ).toBeVisible();
  await expect(
    page.getByRole("button", {
      exact: true,
      name: "Save location: Saved reports",
    })
  ).toHaveAttribute(
    "title",
    "Saved reports\n/Users/test/Research/Saved reports"
  );
  await page.screenshot({ path: "test-results/simple-workspace-720.png" });
});

test("simple row navigation resets temporary destinations and status filters while preserving search", async ({
  page,
}) => {
  const saved = library();
  saved.workspaces[0].icon = { kind: "Emoji", value: "📚" };
  await mockDesktop(page, { snapshot: saved });
  await page.goto("/");

  const row = sidebar(page).getByRole("button", {
    exact: true,
    name: `${saved.workspaces[0].name} workspace`,
  });

  await input(page).fill("Report");
  await row.click();
  await expect(row.locator('[data-slot="emoji-artwork"]')).toBeVisible();
  await page
    .getByRole("button", { exact: true, name: "Filter downloads" })
    .click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Completed" })
    .click();
  await expect(
    page.getByRole("button", { exact: true, name: "Report designs.zip" })
  ).toHaveCount(0);
  await page
    .getByRole("button", { exact: true, name: "Save location: Saved reports" })
    .click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Archive" })
    .click();
  await expect(
    page.getByRole("button", {
      exact: true,
      name: "Save location: Media › Archive",
    })
  ).toBeVisible();
  await row.focus();
  await page.keyboard.press("Space");
  await expect(
    page.getByRole("button", {
      exact: true,
      name: "Save location: Saved reports",
    })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Report designs.zip" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Report interview.mp4" })
  ).toBeVisible();
  await expect(input(page)).toHaveValue("Report");
  await page.reload();
  await expect(
    breadcrumb(page).getByRole("heading", {
      exact: true,
      name: "All downloads",
    })
  ).toBeVisible();
});

for (const selectedFolder of ["Saved reports", "Drafts"]) {
  test(`a one-folder snapshot clears ${selectedFolder} scope before that workspace gains folders again`, async ({
    page,
  }) => {
    const saved = library();
    saved.workspaces[0].folders.push({
      id: "drafts",
      is_default: false,
      name: "Drafts",
      path: "/Users/test/Drafts",
    });
    saved.workspaces[0].downloads[1].folder_id = "drafts";
    await mockDesktop(page, { snapshot: saved });
    await page.goto("/");
    await input(page).fill("Report");
    await sidebar(page)
      .getByRole("button", {
        name: `Toggle ${saved.workspaces[0].name} folders`,
      })
      .click();
    await sidebar(page)
      .getByRole("button", { exact: true, name: selectedFolder })
      .click();
    await expect(
      breadcrumb(page).getByRole("heading", {
        exact: true,
        name: selectedFolder,
      })
    ).toBeVisible();

    const selectedDownload =
      selectedFolder === "Saved reports"
        ? "Report designs.zip"
        : "Report interview.mp4";

    const otherDownload =
      selectedFolder === "Saved reports"
        ? "Report interview.mp4"
        : "Report designs.zip";

    await expect(
      page.getByRole("button", { exact: true, name: selectedDownload })
    ).toBeVisible();
    await expect(
      page.getByRole("button", { exact: true, name: otherDownload })
    ).toHaveCount(0);
    const single = structuredClone(saved);
    single.sequence += 1;
    single.revision += 1;
    single.workspaces[0].folders.pop();
    single.workspaces[0].downloads[1].folder_id = "folder-1";
    await replaceSnapshot(page, single);
    await expect(
      breadcrumb(page).getByRole("heading", {
        exact: true,
        name: single.workspaces[0].name,
      })
    ).toBeVisible();
    await expect(
      breadcrumb(page).locator('[data-slot="breadcrumb-item"]')
    ).toHaveCount(1);
    await expect(
      sidebar(page).getByRole("button", {
        exact: true,
        name: `${single.workspaces[0].name} workspace`,
      })
    ).toHaveAttribute("data-active");
    await expect(
      page.getByRole("button", { exact: true, name: "Report interview.mp4" })
    ).toBeVisible();
    await expect(input(page)).toHaveValue("Report");
    saved.sequence = single.sequence + 1;
    saved.revision = single.revision + 1;
    await replaceSnapshot(page, saved);
    await expect(
      sidebar(page).getByRole("button", { exact: true, name: "Drafts" })
    ).toBeVisible();
    await expect(
      breadcrumb(page).getByRole("heading", {
        exact: true,
        name: saved.workspaces[0].name,
      })
    ).toBeVisible();
    await expect(
      page.getByRole("button", { exact: true, name: "Report interview.mp4" })
    ).toBeVisible();
    await expect(input(page)).toHaveValue("Report");
  });
}
