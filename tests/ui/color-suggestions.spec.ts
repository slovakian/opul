import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { snapshotSchema } from "../../frontend/lib/model";
import treatments from "../../src/application/folder-treatments.json" with { type: "json" };
import { mockDesktop, snapshot } from "./fixture";

const artwork = '[data-slot="emoji-artwork"]';

const picker = (page: Page) =>
  page.getByRole("dialog", {
    exact: true,
    name: /^(?:Change icon|Choose workspace icon)$/u,
  });

const suggestions = (page: Page) =>
  picker(page).getByRole("group", { exact: true, name: "Color suggestions" });

const openPicker = async (page: Page) => {
  await page
    .getByRole("button", { exact: true, name: "Downloads actions" })
    .click();
  await page.getByRole("button", { exact: true, name: "Change icon…" }).click();
};

const choose = async (page: Page, search: string, label: string) => {
  await picker(page)
    .getByRole("textbox", { name: "Search emoji" })
    .fill(search);
  await picker(page)
    .getByRole("gridcell", { exact: true, name: label })
    .click();
};

const identities = async (page: Page) =>
  await suggestions(page)
    .locator(artwork)
    .evaluateAll((nodes) => nodes.map((node) => node.dataset.preset));

const saved = async (page: Page) =>
  snapshotSchema.parse(
    await page.evaluate(async () => {
      const path = "/frontend/lib/bridge.ts";
      const { bridge } = await import(path);

      return bridge.snapshot();
    })
  );

test("color suggestions have square targets and equal spacing in both directions", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  await picker(page)
    .getByRole("button", { exact: true, name: "Show 6 more color suggestions" })
    .click();

  const cells = await suggestions(page)
    .getByRole("button")
    .evaluateAll((nodes) =>
      nodes.map((node) => {
        const { x, y, width, height } = node.getBoundingClientRect();

        return { height, width, x, y };
      })
    );

  expect(cells).toHaveLength(12);
  const horizontalGap = cells[1].x - cells[0].x - cells[0].width;
  const verticalGap = cells[6].y - cells[0].y - cells[0].height;
  expect(horizontalGap).toBeGreaterThan(0);
  expect(horizontalGap).toBeCloseTo(verticalGap, 1);

  for (const [index, cell] of cells.entries()) {
    expect(cell.width).toBeCloseTo(cell.height, 1);
    expect(cell.x).toBeCloseTo(cells[index % 6].x, 1);

    if (index % 6 > 0) {
      expect(cell.x - cells[index - 1].x - cells[index - 1].width).toBeCloseTo(
        horizontalGap,
        1
      );
    }
  }

  const grid = await suggestions(page).boundingBox();
  expect(cells[0].x).toBeCloseTo(grid?.x ?? 0, 1);
  expect(cells[5].x + cells[5].width).toBeCloseTo(
    (grid?.x ?? 0) + (grid?.width ?? 0),
    1
  );
  await page.screenshot({ path: "test-results/color-suggestions-spacing.png" });
});

test("six unique suggestions include a saved preset; expansion preserves selection, focus and emoji scroll", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  library.workspaces[0].appearance.preference = {
    kind: "Preset",
    treatment: treatments[11],
  };
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  await openPicker(page);
  const initial = await identities(page);
  expect(initial).toHaveLength(6);
  expect(new Set(initial).size).toBe(6);
  expect(initial).toContain("lime");

  const selected = suggestions(page).getByRole("button", {
    exact: true,
    name: "Preview Lime Treatment",
  });

  await expect(selected).toHaveAttribute("aria-pressed", "true");

  for (let i = 0; i < 10; i += 1) {
    // oxlint-disable-next-line eslint/no-await-in-loop
    await page.keyboard.press("ArrowDown");
  }

  const viewport = picker(page).locator('[data-slot="emoji-viewport"]');
  const scroll = await viewport.evaluate((node) => node.scrollTop);
  expect(scroll).toBeGreaterThan(0);

  const more = picker(page).getByRole("button", {
    exact: true,
    name: "Show 6 more color suggestions",
  });

  await expect(more).toHaveText("+6");
  await more.focus();
  await page.keyboard.press("Enter");

  const allIdentities = await identities(page);

  expect(allIdentities).toHaveLength(12);
  expect(new Set(allIdentities).size).toBe(12);
  expect(allIdentities.slice(0, 6)).toEqual(initial);
  await expect(more).toHaveCount(0);
  await expect(selected).toHaveAttribute("aria-pressed", "true");
  await expect(suggestions(page).getByRole("button").nth(6)).toBeFocused();
  expect(await viewport.evaluate((node) => node.scrollTop)).toBe(scroll);
  await page.screenshot({
    path: "test-results/color-suggestions-expanded.png",
  });
});

test("choosing the automatic look records a deliberate preset that follows eligible emoji and survives reopening", async ({
  page,
  context,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  await suggestions(page)
    .getByRole("button", { exact: true, name: "Preview Gold Treatment" })
    .click();

  const preview = picker(page)
    .getByRole("img", { name: "Icon preview: 📁" })
    .locator(artwork);

  await expect(preview).toHaveAttribute("data-preset", "gold");
  await choose(page, "panda", "panda");
  await expect(picker(page).getByRole("img").locator(artwork)).toHaveAttribute(
    "data-preset",
    "gold"
  );
  await choose(page, "white circle", "white circle");
  await expect(suggestions(page)).toHaveCount(0);
  await expect(picker(page).getByRole("img").locator(artwork)).toHaveCSS(
    "filter",
    "none"
  );
  await choose(page, "panda", "panda");
  await picker(page).getByRole("button", { exact: true, name: "Save" }).click();
  const savedState = await saved(page);
  expect(savedState.workspaces[0].appearance.preference).toEqual({
    kind: "Preset",
    treatment: treatments[0],
  });
  const reopened = await context.newPage();
  await mockDesktop(reopened, { snapshot: savedState });
  await reopened.goto("/");
  await openPicker(reopened);
  await expect(
    suggestions(reopened).getByRole("button", {
      exact: true,
      name: "Preview color suggestion 1",
    })
  ).toHaveAttribute("aria-pressed", "true");
});

test("treatment names are title case for folders and hidden for other emoji", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  const options = suggestions(page).getByRole("button");

  const checkFolderLabels = async () => {
    const allOptions = await options.all();

    await Promise.all(
      allOptions.map(async (option) => {
        const preset = await option
          .locator(artwork)
          .getAttribute("data-preset");

        expect(preset).toBeTruthy();

        const label = `${preset?.charAt(0).toUpperCase()}${preset?.slice(1)} Treatment`;

        await expect(option).toHaveAttribute("title", label);
        await expect(option).toHaveAccessibleName(`Preview ${label}`);
      })
    );
  };

  await checkFolderLabels();

  for (const [search, label] of [
    ["panda", "panda"],
    ["rainbow", "rainbow"],
  ]) {
    // oxlint-disable-next-line eslint/no-await-in-loop
    await choose(page, search, label);
    // oxlint-disable-next-line eslint/no-await-in-loop
    await expect(options).toHaveCount(6);

    // oxlint-disable-next-line eslint/no-await-in-loop
    const allOptions = await options.all();

    // oxlint-disable-next-line eslint/no-await-in-loop
    await Promise.all(
      allOptions.flatMap((option, index) => [
        expect(option).not.toHaveAttribute("title"),
        expect(option).toHaveAccessibleName(
          `Preview color suggestion ${index + 1}`
        ),
      ])
    );
  }

  await choose(page, "file folder", "file folder");
  await checkFolderLabels();
});

test("folder suggestions stay stable across changes, polling and reopening; other emoji sample once per opening", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await page
    .getByRole("button", { exact: true, name: "Downloads workspace" })
    .click();
  await openPicker(page);
  const folder = await identities(page);
  await choose(page, "panda", "panda");
  const sampled = await identities(page);
  await picker(page)
    .getByRole("button", { exact: true, name: "Original" })
    .click();
  await page.evaluate(async () => {
    const path = "/frontend/lib/bridge.ts";
    const { bridge } = await import(path);
    await bridge.execute({
      grouping: "State",
      kind: "view",
      layout: "Table",
      sort: "Name",
      workspace_id: "workspace-1",
    });
  });
  await expect(
    page.getByRole("button", {
      exact: true,
      includeHidden: true,
      name: "Table view",
    })
  ).toHaveAttribute("aria-pressed", "true");
  expect(await identities(page)).toEqual(sampled);
  await choose(page, "rainbow", "rainbow");
  expect(await identities(page)).toEqual(sampled);
  await choose(page, "file folder", "file folder");
  expect(await identities(page)).toEqual(folder);
  await page.keyboard.press("Escape");
  await openPicker(page);
  expect(await identities(page)).toEqual(folder);
  await choose(page, "panda", "panda");
  expect(await identities(page)).not.toEqual(sampled);
});

test("saved custom remains independently selectable after Original and preset choices", async ({
  page,
}) => {
  const library = structuredClone(snapshot);

  const custom = {
    brightness: 95,
    hue: 120,
    kind: "Custom",
    saturation: 180,
  } satisfies (typeof library.workspaces)[number]["appearance"]["preference"];

  library.workspaces[0].appearance.preference = custom;
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  await openPicker(page);
  expect(await identities(page)).toHaveLength(6);
  await expect(
    picker(page).getByRole("button", { exact: true, name: "Custom" })
  ).toBeVisible();

  const savedCustom = picker(page).getByRole("button", {
    exact: true,
    name: "Saved custom treatment",
  });

  await picker(page)
    .getByRole("button", { exact: true, name: "Original" })
    .click();
  await suggestions(page).getByRole("button").first().click();
  await savedCustom.click();
  await expect(picker(page).getByRole("img").locator(artwork)).toHaveCSS(
    "filter",
    "hue-rotate(120deg) saturate(1.8) brightness(0.95)"
  );
  await picker(page).getByRole("button", { exact: true, name: "Save" }).click();

  const savedAfterSave = await saved(page);

  expect(savedAfterSave.workspaces[0].appearance.preference).toEqual(custom);
});

for (const fail of [false, true]) {
  test(`creation previews and retains a deliberate treatment through nested close and ${fail ? "retry" : "save"}`, async ({
    page,
  }) => {
    await page.setViewportSize({ height: 520, width: 720 });
    await mockDesktop(page, { failFirstCreate: fail });
    await page.goto("/");
    await page
      .getByRole("button", { exact: true, name: "Toggle Sidebar" })
      .click();
    await page
      .getByRole("button", { exact: true, name: "New workspace" })
      .last()
      .click();
    const name = page.getByRole("textbox", { exact: true, name: "Name" });
    await expect(name).toBeFocused();
    await name.fill("Research");

    const trigger = page.getByRole("button", {
      exact: true,
      name: "Workspace emoji: 📁",
    });

    await trigger.click();
    await expect(
      picker(page).getByRole("textbox", { name: "Search emoji" })
    ).toBeFocused();
    expect(await identities(page)).toHaveLength(6);
    await suggestions(page)
      .getByRole("button", { exact: true, name: "Preview Coral Treatment" })
      .click();
    await choose(page, "panda", "panda");

    const more = picker(page).getByRole("button", {
      exact: true,
      name: "Show 6 more color suggestions",
    });

    await more.click();

    await Promise.all(
      treatments.flatMap((treatment) => {
        const option = suggestions(page)
          .getByRole("button")
          .filter({
            has: page.locator(`[data-preset="${treatment.preset_id}"]`),
          });

        return [
          expect(option.locator(artwork)).toHaveText("🐼"),
          expect(option).toHaveCSS("filter", "none"),
          expect(option.locator(artwork)).toHaveCSS(
            "filter",
            `hue-rotate(${treatment.hue}deg) saturate(${treatment.saturation / 100}) brightness(${treatment.brightness / 100})`
          ),
        ];
      })
    );

    const done = picker(page).getByRole("button", {
      exact: true,
      name: "Done",
    });

    await expect(done).toBeInViewport({ ratio: 1 });
    await page.screenshot({
      path: `test-results/color-suggestions-creation-${fail}.png`,
    });
    await page.keyboard.press("Escape");
    await expect(picker(page)).toHaveCount(0);
    await expect(
      page.getByRole("button", { exact: true, name: "Workspace emoji: 🐼" })
    ).toBeFocused();
    await expect(name).toHaveValue("Research");
    await expect(
      page
        .getByRole("button", { exact: true, name: "Workspace emoji: 🐼" })
        .locator(artwork)
    ).toHaveAttribute("data-preset", "coral");
    await page
      .getByRole("button", { exact: true, name: "Create workspace" })
      .click();

    if (fail) {
      await expect(
        page.getByText("Couldn't save the checkpoint. Try again.")
      ).toBeVisible();
      await page
        .getByRole("button", { exact: true, name: "Create workspace" })
        .click();
    }

    await page
      .getByRole("button", { exact: true, name: "Research workspace" })
      .click();
    await expect(
      page
        .getByRole("heading", { exact: true, name: "Research" })
        .locator(artwork)
    ).toHaveText("🐼");

    const savedSnapshot = await saved(page);

    const created = savedSnapshot.workspaces.find(
      (item) => item.name === "Research"
    );

    expect(created?.appearance.preference).toEqual({
      kind: "Preset",
      treatment: treatments[1],
    });
  });
}

test("expanded suggestions stay accessible and bounded in the minimum window, including a failed save", async ({
  page,
}) => {
  const { default: AxeBuilder } = await import("@axe-core/playwright");
  await page.setViewportSize({ height: 520, width: 720 });
  await mockDesktop(page, { failFirstAppearance: true });
  await page.goto("/");
  await page
    .getByRole("button", { exact: true, name: "Toggle Sidebar" })
    .click();
  await page
    .getByRole("button", { exact: true, name: "Downloads workspace" })
    .click();
  await page
    .getByRole("button", { exact: true, name: "Workspace actions" })
    .click();
  await page.getByRole("button", { exact: true, name: "Change icon…" }).click();
  await picker(page)
    .getByRole("button", { exact: true, name: "Show 6 more color suggestions" })
    .click();
  await suggestions(page).getByRole("button").last().click();
  await picker(page).getByRole("button", { exact: true, name: "Save" }).click();
  await expect(picker(page).getByRole("alert")).toBeVisible();
  await expect(
    picker(page).getByRole("textbox", { name: "Search emoji" })
  ).toBeInViewport({ ratio: 1 });
  await expect(
    picker(page).getByRole("button", { exact: true, name: "Save" })
  ).toBeInViewport({ ratio: 1 });

  const accessibility = await new AxeBuilder({ page })
    .include('[data-slot="popover-content"]')
    .disableRules(["button-name"])
    .analyze();

  expect(accessibility.violations).toEqual([]);
  await page.screenshot({ path: "test-results/color-suggestions-minimum.png" });
});

test("Custom starts from the actual preview and canceling creation discards its choices", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await page
    .getByRole("button", { exact: true, name: "New workspace" })
    .last()
    .click();
  await page
    .getByRole("textbox", { exact: true, name: "Name" })
    .fill("Canceled draft");
  await page
    .getByRole("button", { exact: true, name: "Workspace emoji: 📁" })
    .click();
  const preview = picker(page).getByRole("img").locator(artwork);

  const automaticFilter = await preview.evaluate(
    (node) => getComputedStyle(node).filter
  );

  await picker(page)
    .getByRole("button", { exact: true, name: "Custom" })
    .click();
  await expect(preview).toHaveCSS("filter", automaticFilter);
  await expect(
    picker(page).getByRole("slider", { exact: true, name: "Hue" })
  ).toBeVisible();
  await picker(page)
    .getByRole("button", { name: "Back to emoji browsing" })
    .click();
  await expect(
    picker(page).getByRole("button", { exact: true, name: "Custom" })
  ).toHaveAttribute("aria-pressed", "true");
  await choose(page, "rainbow", "rainbow");
  await picker(page).getByRole("button", { exact: true, name: "Done" }).click();
  await page.getByRole("button", { exact: true, name: "Cancel" }).click();
  const savedAfterCancel = await saved(page);
  expect(savedAfterCancel.workspaces).toHaveLength(1);
  await page
    .getByRole("button", { exact: true, name: "New workspace" })
    .last()
    .click();
  await expect(
    page.getByRole("textbox", { exact: true, name: "Name" })
  ).toHaveValue("");
  await page
    .getByRole("button", { exact: true, name: "Workspace emoji: 📁" })
    .click();
  await expect(
    picker(page).getByRole("button", { exact: true, name: "Automatic" })
  ).toHaveAttribute("aria-pressed", "true");
  await expect(picker(page).getByRole("img").locator(artwork)).toHaveCSS(
    "filter",
    automaticFilter
  );
});
