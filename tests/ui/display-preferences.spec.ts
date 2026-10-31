import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { mockDesktop, snapshot } from "./fixture";

const navigate = async (page: Page, name: string) => {
  await page
    .getByRole("navigation", { name: "Workspace navigation" })
    .getByRole("button", { exact: true, name })
    .click();
};

const chooseView = async (page: Page, label: string, group: string) => {
  await page.getByRole("button", { name: /sort and group$/u }).click();
  await page
    .locator('[data-slot="popover-content"] [data-slot="sidebar-group"]')
    .filter({
      has: page.locator('[data-slot="sidebar-group-label"]', {
        hasText: group,
      }),
    })
    .getByRole("button", { exact: true, name: label })
    .click();
};

const expectView = async (
  page: Page,
  layout: string,
  sort: string,
  grouping: string
) => {
  await expect(
    page.getByRole("button", { exact: true, name: `${layout} view` })
  ).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: `${sort}: sort and group` }).click();

  const groups = page.locator(
    '[data-slot="popover-content"] [data-slot="sidebar-group"]'
  );

  await expect(
    groups
      .filter({
        has: page.locator('[data-slot="sidebar-group-label"]', {
          hasText: "Sort by",
        }),
      })
      .getByRole("button", { exact: true, name: sort })
  ).toHaveAttribute("data-active");
  await expect(
    groups
      .filter({
        has: page.locator('[data-slot="sidebar-group-label"]', {
          hasText: "Group by",
        }),
      })
      .getByRole("button", { exact: true, name: grouping })
  ).toHaveAttribute("data-active");
  await page.keyboard.press("Escape");
};

test("aggregate views remember independent display preferences without changing workspaces", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  library.workspaces.push({
    ...structuredClone(library.workspaces[0]),
    downloads: [],
    grouping: "Kind",
    id: "workspace-2",
    name: "Media",
    sort: "Source",
  });
  await mockDesktop(page, { persist: true, snapshot: library });
  await page.goto("/");
  await page.getByRole("button", { exact: true, name: "Table view" }).click();
  await chooseView(page, "Name", "Sort by");
  await chooseView(page, "File type", "Group by");
  await expect(page.getByRole("table")).toHaveCount(3);
  await navigate(page, "In progress");
  await expectView(page, "List", "Date added", "No grouping");
  await chooseView(page, "Status", "Sort by");
  await chooseView(page, "Save location", "Group by");
  await navigate(page, "Completed");
  await expectView(page, "List", "Date added", "No grouping");
  await page.getByRole("button", { exact: true, name: "Table view" }).click();
  await chooseView(page, "Source", "Sort by");
  await chooseView(page, "Status", "Group by");
  await navigate(page, "Downloads workspace");
  await expectView(page, "List", "Date added", "No grouping");
  await chooseView(page, "Status", "Sort by");
  await navigate(page, "Media workspace");
  await expectView(page, "List", "Source", "File type");
  await navigate(page, "All downloads");
  await expectView(page, "Table", "Name", "File type");
  await navigate(page, "In progress");
  await expectView(page, "List", "Status", "Save location");
  await navigate(page, "Completed");
  await expectView(page, "Table", "Source", "Status");
  await navigate(page, "Downloads workspace");
  await expectView(page, "List", "Status", "No grouping");
  await page.reload();
  await expectView(page, "Table", "Name", "File type");
  await navigate(page, "In progress");
  await expectView(page, "List", "Status", "Save location");
  await navigate(page, "Completed");
  await expectView(page, "Table", "Source", "Status");
  await navigate(page, "Downloads workspace");
  await expectView(page, "List", "Status", "No grouping");
});

test("In progress and Completed scopes hide the status filter", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  const header = page.locator("main > header");

  for (const scope of ["In progress", "Completed"] as const) {
    // oxlint-disable-next-line eslint/no-await-in-loop
    await navigate(page, scope);
    // oxlint-disable-next-line eslint/no-await-in-loop
    await expect(
      header.getByRole("button", { name: /downloads$/u })
    ).toHaveCount(0);
    // oxlint-disable-next-line eslint/no-await-in-loop
    await expect(
      page.getByRole("heading", { exact: true, name: scope })
    ).toBeVisible();
    // oxlint-disable-next-line eslint/no-await-in-loop
    await expect(
      page.getByRole("button", {
        exact: true,
        name:
          scope === "In progress"
            ? "Design resources.zip"
            : "The creative process.mp4",
      })
    ).toBeVisible();
  }
});

for (const scope of ["All downloads", "Downloads workspace"]) {
  test(`${scope} highlights optional filters without changing scope`, async ({
    page,
  }) => {
    await mockDesktop(page);
    await page.goto("/");
    const header = page.locator("main > header");
    await navigate(page, scope);

    const filter = header.getByRole("button", {
      exact: true,
      name: "Filter downloads",
    });

    await expect(filter).toHaveAttribute("aria-pressed", "false");

    const inactiveColor = await filter.evaluate(
      (element) => getComputedStyle(element).backgroundColor
    );

    await filter.click();
    await page
      .locator('[data-slot="popover-content"]')
      .getByRole("button", { exact: true, name: "Paused" })
      .click();

    const activeFilter = header.getByRole("button", {
      exact: true,
      name: "Paused downloads",
    });

    await expect(activeFilter).toHaveAttribute("aria-pressed", "true");
    await expect
      .poll(() =>
        activeFilter.evaluate(
          (element) => getComputedStyle(element).backgroundColor
        )
      )
      .not.toBe(inactiveColor);
    await page.screenshot({
      path: `test-results/filter-${scope.replaceAll(" ", "-")}.png`,
    });
    await expect(
      page.getByRole("button", { exact: true, name: "Field notes.pdf" })
    ).toBeVisible();
    await expect(
      page.getByRole("button", { exact: true, name: "Design resources.zip" })
    ).toHaveCount(0);
    await activeFilter.click();
    await page
      .locator('[data-slot="popover-content"]')
      .getByRole("button", { exact: true, name: "In progress" })
      .click();
    await expect(
      page.getByRole("heading", {
        exact: true,
        name: scope === "All downloads" ? "All downloads" : "Downloads",
      })
    ).toBeVisible();
    await expect(
      header.getByRole("button", { exact: true, name: "In progress downloads" })
    ).toHaveAttribute("aria-pressed", "true");
    await header
      .getByRole("button", { exact: true, name: "In progress downloads" })
      .click();
    await page
      .locator('[data-slot="popover-content"]')
      .getByRole("button", { exact: true, name: "All downloads" })
      .click();
    await expect(filter).toHaveAttribute("aria-pressed", "false");
    await navigate(page, "Completed");
    await navigate(page, "All downloads");
    await expect(
      header.getByRole("button", { exact: true, name: "Filter downloads" })
    ).toHaveAttribute("aria-pressed", "false");
  });
}

test("failed aggregate display saves retain the saved view and allow retry", async ({
  page,
}) => {
  await mockDesktop(page, { failFirstView: true });
  await page.goto("/");
  await page.getByRole("button", { exact: true, name: "Table view" }).click();
  await expect(
    page.getByText("Couldn't save display preferences", { exact: true })
  ).toBeVisible();
  await expectView(page, "List", "Date added", "No grouping");
  await page.getByRole("button", { exact: true, name: "Table view" }).click();
  await expect(page.getByRole("table")).toBeVisible();
  await navigate(page, "Downloads workspace");
  await expectView(page, "List", "Date added", "No grouping");
  await navigate(page, "All downloads");
  await expectView(page, "Table", "Date added", "No grouping");
});

test("aggregate sorting and grouping change the rendered downloads", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await page.getByRole("button", { exact: true, name: "Table view" }).click();
  const names = page.locator("tbody tr td:first-child button");
  await chooseView(page, "Status", "Sort by");
  await expect(names).toHaveText([
    "Design resources.zip",
    "The creative process.mp4",
    "Field notes.pdf",
  ]);
  await chooseView(page, "Name", "Sort by");
  await expect(names).toHaveText([
    "Design resources.zip",
    "Field notes.pdf",
    "The creative process.mp4",
  ]);
  await chooseView(page, "Source", "Sort by");
  await expect(names).toHaveText([
    "Design resources.zip",
    "Field notes.pdf",
    "The creative process.mp4",
  ]);
  await chooseView(page, "Date added", "Sort by");
  await expect(names).toHaveText([
    "Design resources.zip",
    "The creative process.mp4",
    "Field notes.pdf",
  ]);
  await chooseView(page, "Status", "Group by");
  await expect(page.getByRole("heading", { level: 2 })).toHaveText([
    "Downloading · 1",
    "Complete · 1",
    "Paused · 1",
  ]);
});

test("changing sort, grouping, or view layout in workspace view does not toggle disabled or flicker search bar controls", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await navigate(page, "Downloads workspace");

  const destinationButton = page.getByRole("button", {
    name: /^Save location:/u,
  });

  await expect(destinationButton).toBeEnabled();

  await page.evaluate(() => {
    sessionStorage.setItem("disabled-changes", "0");
    const inputGroup = document.querySelector('[data-slot="input-group"]');

    const observer = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        if (
          mutation.type === "attributes" &&
          mutation.attributeName === "disabled"
        ) {
          const current = Number(
            sessionStorage.getItem("disabled-changes") ?? "0"
          );

          sessionStorage.setItem("disabled-changes", String(current + 1));
        }
      }
    });

    if (inputGroup) {
      observer.observe(inputGroup, {
        attributeFilter: ["disabled"],
        attributes: true,
        subtree: true,
      });
    }
  });

  await page.getByRole("button", { exact: true, name: "Table view" }).click();
  await expect(page.getByRole("table")).toBeVisible();

  await chooseView(page, "Name", "Sort by");
  await chooseView(page, "File type", "Group by");

  const changes = await page.evaluate(
    () => sessionStorage.getItem("disabled-changes") ?? "0"
  );

  expect(changes).toBe("0");
});
