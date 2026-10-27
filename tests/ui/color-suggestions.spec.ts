import { expect, test, type Page } from "@playwright/test";
import { mockDesktop, snapshot } from "./fixture";
import { snapshotSchema } from "../../frontend/lib/model";
import treatments from "../../src/application/folder-treatments.json" with { type: "json" };

const artwork = '[data-slot="emoji-artwork"]';
const picker = (page: Page) =>
  page.getByRole("dialog", {
    name: /^(Change icon|Choose workspace icon)$/,
    exact: true,
  });
const suggestions = (page: Page) =>
  picker(page).getByRole("group", { name: "Color suggestions", exact: true });
async function openPicker(page: Page) {
  await page
    .getByRole("button", { name: "Downloads actions", exact: true })
    .click();
  await page.getByRole("button", { name: "Change icon…", exact: true }).click();
}
async function choose(page: Page, search: string, label: string) {
  await picker(page)
    .getByRole("textbox", { name: "Search emoji" })
    .fill(search);
  await picker(page)
    .getByRole("gridcell", { name: label, exact: true })
    .click();
}
async function identities(page: Page) {
  return suggestions(page)
    .locator(artwork)
    .evaluateAll((nodes) =>
      nodes.map((node) => node.getAttribute("data-preset")),
    );
}
async function saved(page: Page) {
  return snapshotSchema.parse(
    await page.evaluate(async () => {
      const path = "/frontend/lib/bridge.ts";
      const { bridge } = await import(path);
      return bridge.snapshot();
    }),
  );
}

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
    name: "Preview lime treatment",
    exact: true,
  });
  await expect(selected).toHaveAttribute("aria-pressed", "true");
  for (let i = 0; i < 10; i++) await page.keyboard.press("ArrowDown");
  const viewport = picker(page).locator("[frimousse-viewport]");
  const scroll = await viewport.evaluate((node) => node.scrollTop);
  expect(scroll).toBeGreaterThan(0);
  const more = picker(page).getByRole("button", {
    name: "Show 6 more color suggestions",
    exact: true,
  });
  await expect(more).toHaveText("+6");
  await more.focus();
  await page.keyboard.press("Enter");
  expect(await identities(page)).toHaveLength(12);
  expect(new Set(await identities(page)).size).toBe(12);
  expect((await identities(page)).slice(0, 6)).toEqual(initial);
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
    .getByRole("button", { name: "Preview gold treatment", exact: true })
    .click();
  const preview = picker(page)
    .getByRole("img", { name: "Icon preview: 📁" })
    .locator(artwork);
  await expect(preview).toHaveAttribute("data-preset", "gold");
  await choose(page, "panda", "panda");
  await expect(picker(page).getByRole("img").locator(artwork)).toHaveAttribute(
    "data-preset",
    "gold",
  );
  await choose(page, "white circle", "white circle");
  await expect(suggestions(page)).toHaveCount(0);
  await expect(picker(page).getByRole("img").locator(artwork)).toHaveCSS(
    "filter",
    "none",
  );
  await choose(page, "panda", "panda");
  await picker(page).getByRole("button", { name: "Save", exact: true }).click();
  expect((await saved(page)).workspaces[0].appearance.preference).toEqual({
    kind: "Preset",
    treatment: treatments[0],
  });
  const reopened = await context.newPage();
  await mockDesktop(reopened, { snapshot: await saved(page) });
  await reopened.goto("/");
  await openPicker(reopened);
  await expect(
    suggestions(reopened).getByRole("button", {
      name: "Preview gold treatment",
      exact: true,
    }),
  ).toHaveAttribute("aria-pressed", "true");
});

test("folder suggestions stay stable across changes, polling and reopening; other emoji sample once per opening", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  const folder = await identities(page);
  await choose(page, "panda", "panda");
  const sampled = await identities(page);
  await picker(page)
    .getByRole("button", { name: "Original", exact: true })
    .click();
  await page.evaluate(async () => {
    const path = "/frontend/lib/bridge.ts";
    const { bridge } = await import(path);
    await bridge.execute({
      kind: "view",
      workspace_id: "workspace-1",
      layout: "Table",
      sort: "Name",
      grouping: "State",
    });
  });
  await expect(
    page.getByRole("button", {
      name: "Table view",
      exact: true,
      includeHidden: true,
    }),
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
    kind: "Custom",
    hue: 120,
    saturation: 180,
    brightness: 95,
  } satisfies (typeof library.workspaces)[number]["appearance"]["preference"];
  library.workspaces[0].appearance.preference = custom;
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  await openPicker(page);
  expect(await identities(page)).toHaveLength(6);
  await expect(
    picker(page).getByRole("button", { name: "Custom", exact: true }),
  ).toBeVisible();
  const savedCustom = picker(page).getByRole("button", {
    name: "Saved custom treatment",
    exact: true,
  });
  await picker(page)
    .getByRole("button", { name: "Original", exact: true })
    .click();
  await suggestions(page).getByRole("button").first().click();
  await savedCustom.click();
  await expect(picker(page).getByRole("img").locator(artwork)).toHaveCSS(
    "filter",
    "hue-rotate(120deg) saturate(1.8) brightness(0.95)",
  );
  await picker(page).getByRole("button", { name: "Save", exact: true }).click();
  expect((await saved(page)).workspaces[0].appearance.preference).toEqual(
    custom,
  );
});

for (const fail of [false, true]) {
  test(`creation previews and retains a deliberate treatment through nested close and ${fail ? "retry" : "save"}`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 720, height: 520 });
    await mockDesktop(page, { failFirstCreate: fail });
    await page.goto("/");
    await page
      .getByRole("button", { name: "Toggle Sidebar", exact: true })
      .click();
    await page
      .getByRole("button", { name: "New workspace", exact: true })
      .last()
      .click();
    const name = page.getByRole("textbox", { name: "Name", exact: true });
    await expect(name).toBeFocused();
    await name.fill("Research");
    const trigger = page.getByRole("button", {
      name: "Workspace emoji: 📁",
      exact: true,
    });
    await trigger.click();
    await expect(
      picker(page).getByRole("textbox", { name: "Search emoji" }),
    ).toBeFocused();
    expect(await identities(page)).toHaveLength(6);
    await suggestions(page)
      .getByRole("button", { name: "Preview coral treatment", exact: true })
      .click();
    await choose(page, "panda", "panda");
    const more = picker(page).getByRole("button", {
      name: "Show 6 more color suggestions",
      exact: true,
    });
    await more.click();
    for (const treatment of treatments) {
      const option = suggestions(page).getByRole("button", {
        name: `Preview ${treatment.preset_id} treatment`,
        exact: true,
      });
      await expect(option.locator(artwork)).toHaveText("🐼");
      await expect(option).toHaveCSS("filter", "none");
      await expect(option.locator(artwork)).toHaveCSS(
        "filter",
        `hue-rotate(${treatment.hue}deg) saturate(${treatment.saturation / 100}) brightness(${treatment.brightness / 100})`,
      );
    }
    const done = picker(page).getByRole("button", {
      name: "Done",
      exact: true,
    });
    await expect(done).toBeInViewport({ ratio: 1 });
    await page.screenshot({
      path: `test-results/color-suggestions-creation-${fail}.png`,
    });
    await page.keyboard.press("Escape");
    await expect(picker(page)).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Workspace emoji: 🐼", exact: true }),
    ).toBeFocused();
    await expect(name).toHaveValue("Research");
    await expect(
      page
        .getByRole("button", { name: "Workspace emoji: 🐼", exact: true })
        .locator(artwork),
    ).toHaveAttribute("data-preset", "coral");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    if (fail) {
      await expect(
        page.getByText("Couldn't save the checkpoint. Try again."),
      ).toBeVisible();
      await page
        .getByRole("button", { name: "Create workspace", exact: true })
        .click();
    }
    await page
      .getByRole("button", { name: "Research workspace", exact: true })
      .click();
    await expect(
      page
        .getByRole("heading", { name: "Research", exact: true })
        .locator(artwork),
    ).toHaveText("🐼");
    const created = (await saved(page)).workspaces.find(
      (workspace) => workspace.name === "Research",
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
  await page.setViewportSize({ width: 720, height: 520 });
  await mockDesktop(page, { failFirstAppearance: true });
  await page.goto("/");
  await page
    .getByRole("button", { name: "Toggle Sidebar", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Downloads workspace", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Workspace actions", exact: true })
    .click();
  await page.getByRole("button", { name: "Change icon…", exact: true }).click();
  await picker(page)
    .getByRole("button", { name: "Show 6 more color suggestions", exact: true })
    .click();
  await suggestions(page).getByRole("button").last().click();
  await picker(page).getByRole("button", { name: "Save", exact: true }).click();
  await expect(picker(page).getByRole("alert")).toBeVisible();
  await expect(
    picker(page).getByRole("textbox", { name: "Search emoji" }),
  ).toBeInViewport({ ratio: 1 });
  await expect(
    picker(page).getByRole("button", { name: "Save", exact: true }),
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
    .getByRole("button", { name: "New workspace", exact: true })
    .last()
    .click();
  await page
    .getByRole("textbox", { name: "Name", exact: true })
    .fill("Canceled draft");
  await page
    .getByRole("button", { name: "Workspace emoji: 📁", exact: true })
    .click();
  const preview = picker(page).getByRole("img").locator(artwork);
  const automaticFilter = await preview.evaluate(
    (node) => getComputedStyle(node).filter,
  );
  await picker(page)
    .getByRole("button", { name: "Custom", exact: true })
    .click();
  await expect(preview).toHaveCSS("filter", automaticFilter);
  await expect(
    picker(page).getByRole("button", { name: "Custom", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await choose(page, "rainbow", "rainbow");
  await picker(page).getByRole("button", { name: "Done", exact: true }).click();
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  expect((await saved(page)).workspaces).toHaveLength(1);
  await page
    .getByRole("button", { name: "New workspace", exact: true })
    .last()
    .click();
  await expect(
    page.getByRole("textbox", { name: "Name", exact: true }),
  ).toHaveValue("");
  await page
    .getByRole("button", { name: "Workspace emoji: 📁", exact: true })
    .click();
  await expect(
    picker(page).getByRole("button", { name: "Automatic", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await expect(picker(page).getByRole("img").locator(artwork)).toHaveCSS(
    "filter",
    automaticFilter,
  );
});
