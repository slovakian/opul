import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { mockDesktop, snapshot } from "./fixture";
import { snapshotSchema } from "../../frontend/lib/model";

const artwork = '[data-slot="emoji-artwork"]';
const picker = (page: Page) =>
  page.getByRole("dialog", { name: /^(Change icon|Choose workspace icon)$/ });
const preview = (page: Page) =>
  picker(page)
    .getByRole("img", { name: /^Icon preview:/ })
    .locator(artwork);
async function openPicker(page: Page) {
  await page
    .getByRole("button", { name: "Downloads actions", exact: true })
    .click();
  await page.getByRole("button", { name: "Change icon…", exact: true }).click();
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
async function choose(page: Page, name: string) {
  await picker(page).getByRole("textbox", { name: "Search emoji" }).fill(name);
  await picker(page).getByRole("gridcell", { name, exact: true }).click();
}
async function customize(page: Page) {
  await picker(page)
    .getByRole("button", { name: "Custom", exact: true })
    .click();
}

test("Custom replaces browsing, starts from the preview, and back preserves the draft and browsing session", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  await choose(page, "rainbow");
  await picker(page)
    .getByRole("group", { name: "Color suggestions", exact: true })
    .getByRole("button")
    .first()
    .click();
  await picker(page)
    .getByRole("button", { name: "Show 6 more color suggestions" })
    .click();
  const identities = await picker(page)
    .getByRole("group", { name: "Color suggestions", exact: true })
    .locator(artwork)
    .evaluateAll((nodes) =>
      nodes.map((node) => node.getAttribute("data-preset")),
    );
  const before = await preview(page).evaluate(
    (node) => getComputedStyle(node).filter,
  );
  await picker(page)
    .getByRole("textbox", { name: "Search emoji" })
    .fill("face");
  await expect(
    picker(page).getByRole("gridcell", { name: "grinning face", exact: true }),
  ).toBeVisible();
  await picker(page).locator("[frimousse-viewport]").hover();
  await page.mouse.wheel(0, 220);
  await expect
    .poll(() =>
      picker(page)
        .locator("[frimousse-viewport]")
        .evaluate((node) => node.scrollTop),
    )
    .toBeGreaterThan(0);
  const scroll = await picker(page)
    .locator("[frimousse-viewport]")
    .evaluate((node) => node.scrollTop);
  expect(scroll).toBeGreaterThan(0);
  await customize(page);
  await expect(
    picker(page).getByRole("textbox", { name: "Search emoji" }),
  ).toBeHidden();
  await expect(
    picker(page).getByRole("group", { name: "Color suggestions", exact: true }),
  ).toBeHidden();
  await expect(preview(page)).toHaveCSS("filter", before);
  const hue = picker(page).getByRole("slider", { name: "Hue", exact: true });
  await expect(hue).toBeFocused();
  await hue.press("Home");
  await hue.press("ArrowRight");
  await expect(hue).toHaveAttribute("aria-valuenow", "1");
  for (const name of ["Saturation", "Brightness"])
    await expect(
      picker(page).getByRole("slider", { name, exact: true }),
    ).toBeVisible();
  const edited = await preview(page).evaluate(
    (node) => getComputedStyle(node).filter,
  );
  expect((await saved(page)).workspaces[0].appearance.preference.kind).toBe(
    "Automatic",
  );
  await picker(page)
    .getByRole("button", { name: "Back to emoji browsing" })
    .click();
  await expect(
    picker(page).getByRole("button", { name: "Custom", exact: true }),
  ).toBeFocused();
  await expect(
    picker(page).getByRole("textbox", { name: "Search emoji" }),
  ).toHaveValue("face");
  expect(
    await picker(page)
      .locator("[frimousse-viewport]")
      .evaluate((node) => node.scrollTop),
  ).toBe(scroll);
  expect(
    await picker(page)
      .getByRole("group", { name: "Color suggestions", exact: true })
      .locator(artwork)
      .evaluateAll((nodes) =>
        nodes.map((node) => node.getAttribute("data-preset")),
      ),
  ).toEqual(identities);
  await customize(page);
  await expect(hue).toHaveAttribute("aria-valuenow", "1");
  await expect(preview(page)).toHaveCSS("filter", edited);
  await page.screenshot({ path: "test-results/custom-treatment.png" });
});

test("custom save matches every workspace surface and a fresh page; exclusions suspend and restore it", async ({
  page,
  context,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  await customize(page);
  for (const [name, key] of [
    ["Hue", "End"],
    ["Saturation", "Home"],
    ["Brightness", "End"],
  ]) {
    await picker(page).getByRole("slider", { name, exact: true }).press(key);
  }
  await picker(page)
    .getByRole("button", { name: "Back to emoji browsing" })
    .click();
  await choose(page, "white circle");
  await expect(preview(page)).toHaveCSS("filter", "none");
  await expect(
    picker(page).getByRole("button", { name: "Custom", exact: true }),
  ).toHaveCount(0);
  await picker(page).getByRole("button", { name: "Save", exact: true }).click();
  const custom = { kind: "Custom", hue: 359, saturation: 0, brightness: 150 };
  expect((await saved(page)).workspaces[0].appearance.preference).toEqual(
    custom,
  );
  const reopened = await context.newPage();
  await mockDesktop(reopened, { snapshot: await saved(page) });
  await reopened.goto("/");
  await openPicker(reopened);
  await choose(reopened, "rainbow");
  await customize(reopened);
  await expect(
    reopened.getByRole("slider", { name: "Hue", exact: true }),
  ).toHaveAttribute("aria-valuenow", "359");
  const filter = "hue-rotate(359deg) saturate(0) brightness(1.5)";
  await expect(preview(reopened)).toHaveCSS("filter", filter);
  await picker(reopened)
    .getByRole("button", { name: "Save", exact: true })
    .click();
  await reopened
    .getByRole("button", { name: "Downloads workspace", exact: true })
    .click();
  await expect(
    reopened
      .getByRole("button", { name: "Downloads workspace", exact: true })
      .locator(artwork),
  ).toHaveCSS("filter", filter);
  await expect(
    reopened
      .getByRole("heading", { name: "Downloads", exact: true })
      .locator(artwork),
  ).toHaveCSS("filter", filter);
  await reopened
    .getByRole("button", { name: "Download options", exact: true })
    .click();
  const selector = reopened.getByRole("button", {
    name: "Download workspace: Downloads",
    exact: true,
  });
  await expect(selector.locator(artwork)).toHaveCSS("filter", filter);
  await expect(selector).toHaveCSS("filter", "none");
});

test("custom mode stays accessible in the minimum window and retains edits on failed save; Cancel discards them", async ({
  page,
}) => {
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
  await customize(page);
  await picker(page)
    .getByRole("slider", { name: "Hue", exact: true })
    .press("End");
  await picker(page).getByRole("button", { name: "Save", exact: true }).click();
  await expect(picker(page).getByRole("alert")).toBeVisible();
  for (const name of ["Hue", "Saturation", "Brightness"])
    await expect(
      picker(page).getByRole("slider", { name, exact: true }),
    ).toBeInViewport({ ratio: 1 });
  await expect(
    picker(page).getByRole("button", { name: "Save", exact: true }),
  ).toBeInViewport({ ratio: 1 });
  await expect(picker(page)).toHaveCSS("animation-name", "none");
  const accessibility = await new AxeBuilder({ page })
    .include('[data-slot="popover-content"]')
    .disableRules(["button-name"])
    .analyze();
  expect(accessibility.violations).toEqual([]);
  await picker(page)
    .getByRole("button", { name: "Cancel", exact: true })
    .click();
  expect((await saved(page)).workspaces[0].appearance.preference.kind).toBe(
    "Automatic",
  );
});

test("creation retains custom edits through inner close and saves exact parameters", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  await page
    .getByRole("button", { name: "New workspace", exact: true })
    .last()
    .click();
  await page
    .getByRole("textbox", { name: "Name", exact: true })
    .fill("Custom workspace");
  await page
    .getByRole("button", { name: "Workspace emoji: 📁", exact: true })
    .click();
  await customize(page);
  await picker(page)
    .getByRole("slider", { name: "Saturation", exact: true })
    .press("End");
  const filter = await preview(page).evaluate(
    (node) => getComputedStyle(node).filter,
  );
  await page.keyboard.press("Escape");
  const trigger = page.getByRole("button", {
    name: "Workspace emoji: 📁",
    exact: true,
  });
  await expect(trigger).toBeFocused();
  await expect(trigger.locator(artwork)).toHaveCSS("filter", filter);
  await trigger.click();
  await customize(page);
  await expect(
    picker(page).getByRole("slider", { name: "Saturation", exact: true }),
  ).toHaveAttribute("aria-valuenow", "1000");
  await picker(page).getByRole("button", { name: "Done", exact: true }).click();
  await page
    .getByRole("button", { name: "Create workspace", exact: true })
    .click();
  expect(
    (await saved(page)).workspaces.find(
      (workspace) => workspace.name === "Custom workspace",
    )?.appearance.preference,
  ).toMatchObject({ kind: "Custom", saturation: 1000 });
  await expect(
    page
      .getByRole("button", { name: "Custom workspace workspace", exact: true })
      .locator(artwork),
  ).toHaveCSS("filter", filter);
});

test("dragging updates only the local preview, survives polling, and Escape discards it", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  await picker(page)
    .getByRole("button", { name: "Original", exact: true })
    .click();
  await customize(page);
  for (const [name, value] of [
    ["Hue", "0"],
    ["Saturation", "100"],
    ["Brightness", "100"],
  ]) {
    await expect(
      picker(page).getByRole("slider", { name, exact: true }),
    ).toHaveAttribute("aria-valuenow", value);
  }
  const before = await saved(page);
  const bounds = await picker(page)
    .locator('[data-slot="slider-track"]')
    .first()
    .boundingBox();
  if (!bounds) throw new Error("The hue track must be visible");
  await page.mouse.move(
    bounds.x + bounds.width / 4,
    bounds.y + bounds.height / 2,
  );
  await page.mouse.down();
  await page.mouse.move(
    bounds.x + (bounds.width * 3) / 4,
    bounds.y + bounds.height / 2,
    { steps: 30 },
  );
  await page.mouse.up();
  const hue = picker(page).getByRole("slider", { name: "Hue", exact: true });
  expect(Number(await hue.getAttribute("aria-valuenow"))).toBeGreaterThan(250);
  const filter = await preview(page).evaluate(
    (node) => getComputedStyle(node).filter,
  );
  expect((await saved(page)).revision).toBe(before.revision);
  expect((await saved(page)).workspaces[0].appearance).toEqual(
    before.workspaces[0].appearance,
  );
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
  await expect(preview(page)).toHaveCSS("filter", filter);
  await expect(
    picker(page).getByRole("button", { name: "Back to emoji browsing" }),
  ).toHaveCSS("filter", "none");
  await page.keyboard.press("Escape");
  await expect(picker(page)).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Downloads actions", exact: true }),
  ).toBeFocused();
  await openPicker(page);
  await expect(
    picker(page).getByRole("button", { name: "Automatic", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  expect(errors).toEqual([]);
});

test("failed custom save retries without losing slider values", async ({
  page,
}) => {
  await mockDesktop(page, { failFirstAppearance: true });
  await page.goto("/");
  await openPicker(page);
  await customize(page);
  const hue = picker(page).getByRole("slider", { name: "Hue", exact: true });
  await hue.press("End");
  const filter = await preview(page).evaluate(
    (node) => getComputedStyle(node).filter,
  );
  await picker(page).getByRole("button", { name: "Save", exact: true }).click();
  await expect(picker(page).getByRole("alert")).toBeVisible();
  await expect(hue).toHaveAttribute("aria-valuenow", "359");
  expect((await saved(page)).workspaces[0].appearance.preference.kind).toBe(
    "Automatic",
  );
  await picker(page).getByRole("button", { name: "Save", exact: true }).click();
  expect((await saved(page)).workspaces[0].appearance.preference).toMatchObject(
    { kind: "Custom", hue: 359 },
  );
  await openPicker(page);
  await expect(preview(page)).toHaveCSS("filter", filter);
});
