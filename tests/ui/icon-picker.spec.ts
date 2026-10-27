import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { snapshotSchema } from "../../frontend/lib/model";
import treatments from "../../src/application/folder-treatments.json" with { type: "json" };
import { mockDesktop, snapshot } from "./fixture";

const artwork = '[data-slot="emoji-artwork"]';

const editor = (page: Page) =>
  page.getByRole("dialog", { exact: true, name: "Change icon" });

async function openPicker(page: Page) {
  await page
    .getByRole("button", { exact: true, name: "Downloads actions" })
    .click();
  await page.getByRole("button", { exact: true, name: "Change icon…" }).click();
  await expect(
    editor(page).getByRole("textbox", { name: "Search emoji" })
  ).toBeFocused();
}

async function choose(page: Page, search: string, label: string) {
  await editor(page)
    .getByRole("textbox", { name: "Search emoji" })
    .fill(search);
  await editor(page)
    .getByRole("gridcell", { exact: true, name: label })
    .click();
  await expect(editor(page)).toBeVisible();
}

async function savedSnapshot(page: Page) {
  return snapshotSchema.parse(
    await page.evaluate(async () => {
      const path = "/frontend/lib/bridge.ts";
      const { bridge } = await import(path);

      return bridge.snapshot();
    })
  );
}

test("search previews locally, keeps the picker open, and saves the full sequence", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  await expect(
    editor(page).locator("[frimousse-category-header]:visible").first()
  ).toHaveText("Smileys & Emotion");
  await choose(page, "family", "family: man, woman, girl, boy");
  await expect(
    editor(page).getByRole("img", { name: "Icon preview: 👨‍👩‍👧‍👦" })
  ).toBeVisible();
  expect((await savedSnapshot(page)).workspaces[0].icon).toEqual(
    snapshot.workspaces[0].icon
  );
  await editor(page).getByRole("button", { exact: true, name: "Save" }).click();
  await expect(editor(page)).toHaveCount(0);
  await expect(
    page
      .getByRole("button", { exact: true, name: "Downloads workspace" })
      .locator(artwork)
  ).toHaveText("👨‍👩‍👧‍👦");
  expect((await savedSnapshot(page)).workspaces[0].icon.value).toBe("👨‍👩‍👧‍👦");
});

for (const close of ["Cancel", "Escape", "outside"]) {
  test(`${close} discards the draft and restores focus to its menu trigger`, async ({
    page,
  }) => {
    await mockDesktop(page);
    await page.goto("/");
    await openPicker(page);
    await choose(page, "panda", "panda");

    if (close === "Cancel") {
      await editor(page)
        .getByRole("button", { name: "Cancel", exact: true })
        .click();
    } else if (close === "Escape") {
      await page.keyboard.press("Escape");
    } else {
      await page.mouse.click(650, 450);
    }
    await expect(editor(page)).toHaveCount(0);
    await expect(
      page.getByRole("button", { exact: true, name: "Downloads actions" })
    ).toBeFocused();
    expect((await savedSnapshot(page)).workspaces[0]).toEqual(
      snapshot.workspaces[0]
    );
    await openPicker(page);
    await expect(
      editor(page).getByRole("img", { name: "Icon preview: 📁" })
    ).toBeVisible();
  });
}

for (const lost of [false, true]) {
  test(`draft survives polling and ${lost ? "a lost response" : "a failed checkpoint"}, and retries safely`, async ({
    page,
  }) => {
    await mockDesktop(page, {
      failFirstAppearance: !lost,
      loseFirstAppearanceResponse: lost,
    });
    await page.goto("/");
    await page
      .getByRole("button", { exact: true, name: "Downloads workspace" })
      .click();
    await openPicker(page);
    await choose(page, "panda", "panda");
    // An unrelated setting changes while the picker owns a local draft.
    await page.evaluate(async () => {
      const path = "/frontend/lib/bridge.ts";
      const { bridge } = await import(path);
      await bridge.execute({
        format: "AudioMp3",
        kind: "format",
        workspace_id: "workspace-1",
      });
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
    await expect(
      editor(page).getByRole("img", { name: "Icon preview: 🐼" })
    ).toBeVisible();
    await editor(page)
      .getByRole("button", { exact: true, name: "Save" })
      .click();
    await expect(editor(page).getByRole("alert")).toContainText(
      "Your changes are kept"
    );
    await expect(
      editor(page).getByRole("textbox", { name: "Search emoji" })
    ).toHaveValue("panda");
    await expect(
      editor(page).getByRole("img", { name: "Icon preview: 🐼" })
    ).toBeVisible();
    await editor(page)
      .getByRole("button", { exact: true, name: "Save" })
      .click();
    await expect(editor(page)).toHaveCount(0);
    const saved = (await savedSnapshot(page)).workspaces[0];
    expect(saved.icon.value).toBe("🐼");
    expect(saved.default_format).toBe("AudioMp3");
    expect(saved.layout).toBe("Table");
    expect(saved.sort).toBe("Name");
    expect(saved.grouping).toBe("State");
    expect(saved.appearance.automatic_folder).toEqual(treatments[0]);
  });
}

test("automatic folder treatment suspends, returns unchanged, and explicit Original stays original", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  const preview = editor(page).getByRole("img").locator(artwork);
  await expect(preview).toHaveAttribute("data-preset", "gold");
  await choose(page, "panda", "panda");
  await expect(preview).toHaveCSS("filter", "none");
  await choose(page, "file folder", "file folder");
  await expect(preview).toHaveAttribute("data-preset", "gold");
  await editor(page)
    .getByRole("button", { exact: true, name: "Original" })
    .click();
  await expect(preview).toHaveCSS("filter", "none");
  await editor(page).getByRole("button", { exact: true, name: "Save" }).click();
  await expect(editor(page)).toHaveCount(0);
  await openPicker(page);
  await expect(
    editor(page).getByRole("button", { exact: true, name: "Original" })
  ).toHaveAttribute("aria-pressed", "true");
  await expect(editor(page).getByRole("img").locator(artwork)).toHaveCSS(
    "filter",
    "none"
  );
  expect((await savedSnapshot(page)).workspaces[0].appearance).toEqual({
    automatic_folder: treatments[0],
    preference: { kind: "Original" },
  });
});

test("manual treatment follows eligible emoji and suspends for excluded presentation variants", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  library.workspaces[0].appearance.preference = {
    kind: "Preset",
    treatment: treatments[1],
  };
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  await openPicker(page);
  const preview = editor(page).getByRole("img").locator(artwork);
  await choose(page, "panda", "panda");
  await expect(preview).toHaveAttribute("data-preset", "coral");
  await choose(page, "white circle", "white circle");
  await expect(preview).toHaveCSS("filter", "none");
  await expect(
    editor(page).getByRole("group", { exact: true, name: "Icon appearance" })
  ).toHaveCount(0);
  await editor(page).getByRole("button", { exact: true, name: "Save" }).click();
  await expect(editor(page)).toHaveCount(0);
  expect(
    (await savedSnapshot(page)).workspaces[0].appearance.preference
  ).toEqual({ kind: "Preset", treatment: treatments[1] });
  await openPicker(page);
  await choose(page, "panda", "panda");
  await expect(editor(page).getByRole("img").locator(artwork)).toHaveAttribute(
    "data-preset",
    "coral"
  );
  await editor(page).getByRole("button", { exact: true, name: "Save" }).click();
  await expect(editor(page)).toHaveCount(0);
  await expect(
    page
      .getByRole("button", { exact: true, name: "Downloads workspace" })
      .locator(artwork)
  ).toHaveAttribute("data-preset", "coral");
});

test("first use has no emoji-data requests and supports keyboard selection at minimum size", async ({
  page,
  context,
  baseURL,
}) => {
  const external: string[] = [];
  page.on("request", (request) => {
    if (!request.url().startsWith(baseURL ?? "http://localhost:1420")) {
      external.push(request.url());
    }
  });
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.setViewportSize({ height: 520, width: 720 });
  await mockDesktop(page);
  await page.goto("/");
  await page
    .getByRole("button", { exact: true, name: "Toggle Sidebar" })
    .click();
  await page
    .getByRole("button", { exact: true, name: "Downloads workspace" })
    .click();
  await expect(
    page.getByRole("heading", { exact: true, name: "Downloads" })
  ).toBeVisible();
  await context.setOffline(true);
  await page
    .getByRole("button", { exact: true, name: "Workspace actions" })
    .click();
  await page.getByRole("button", { exact: true, name: "Change icon…" }).click();
  const search = editor(page).getByRole("textbox", { name: "Search emoji" });
  await search.fill("panda");
  await expect(
    editor(page).getByRole("gridcell", { exact: true, name: "panda" })
  ).toBeVisible();
  await search.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(
    editor(page).getByRole("img", { name: "Icon preview: 🐼" })
  ).toBeVisible();
  await expect(
    editor(page).getByRole("button", { exact: true, name: "Save" })
  ).toBeInViewport();
  const bounds = await editor(page).boundingBox();
  expect(bounds).not.toBeNull();

  if (bounds) {
    expect(bounds.y).toBeGreaterThanOrEqual(0);
    expect(bounds.y + bounds.height).toBeLessThanOrEqual(520);
  }

  expect(external).toEqual([]);
  expect(errors).toEqual([]);

  const accessibility = await new AxeBuilder({ page })
    .include('[data-slot="popover-content"]')
    .disableRules(["button-name"])
    .analyze();

  expect(accessibility.violations).toEqual([]);

  const unnamed = await editor(page)
    .locator("button:not([aria-hidden])")
    .evaluateAll(
      (buttons) =>
        buttons.filter(
          (button) =>
            !button.getAttribute("aria-label") && !button.textContent?.trim()
        ).length
    );

  expect(unnamed).toBe(0);
  await page.screenshot({ path: "test-results/icon-picker-minimum.png" });
});

test("saved custom treatments and eligibility share the renderer across Unicode presentation variants", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  const emojis = ["⚪", "⚪︎", "⚪️", "☠️", "🐼", "👩🏽‍💻", "📁︎", "📁️"];
  library.workspaces = emojis.map((value, index) => ({
    ...library.workspaces[0],
    appearance: {
      automatic_folder: treatments[0],
      preference: { brightness: 95, hue: 120, kind: "Custom", saturation: 180 },
    },
    downloads: [],
    folders: [],
    icon: { kind: "Emoji", value },
    id: `appearance-${index}`,
    name: `Icon ${index}`,
  }));
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");

  for (const [index, value] of emojis.entries()) {
    const icon = page
      .getByRole("button", { exact: true, name: `Icon ${index} workspace` })
      .locator(artwork);

    await expect(icon).toHaveText(value);
    await expect(icon).toHaveCSS(
      "filter",
      index < 4 ? "none" : "hue-rotate(120deg) saturate(1.8) brightness(0.95)"
    );
  }
});

test("an unassigned native workspace freezes its first folder proposal", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  library.workspaces[0].icon = { kind: "Emoji", value: "🐼" };
  library.workspaces[0].appearance = {
    automatic_folder: null,
    preference: { kind: "Original" },
  };
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  await openPicker(page);
  await choose(page, "file folder", "file folder");
  await expect(editor(page).getByRole("img").locator(artwork)).toHaveCSS(
    "filter",
    "none"
  );
  await editor(page)
    .getByRole("button", { exact: true, name: "Automatic" })
    .click();
  await expect(editor(page).getByRole("img").locator(artwork)).toHaveAttribute(
    "data-preset",
    "coral"
  );
  await editor(page).getByRole("button", { exact: true, name: "Save" }).click();
  await expect(editor(page)).toHaveCount(0);
  expect(
    (await savedSnapshot(page)).workspaces[0].appearance.automatic_folder
  ).toEqual(treatments[1]);
});

test("save errors keep the actions reachable in the minimum window", async ({
  page,
}) => {
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
  await editor(page).getByRole("button", { exact: true, name: "Save" }).click();
  await expect(editor(page).getByRole("alert")).toBeVisible();
  const save = editor(page).getByRole("button", { exact: true, name: "Save" });
  await expect(save).toBeInViewport({ ratio: 1 });
  await save.click();
  await expect(editor(page)).toHaveCount(0);
});

test("bundled resolver filters version ceilings and unsupported country flags", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");

  const filtered = await page.evaluate(async () => {
    // Exercise the custom resolver boundary without depending on virtualized rows.
    const measure = CanvasRenderingContext2D.prototype.measureText;
    CanvasRenderingContext2D.prototype.measureText = function measureText(
      text
    ) {
      const metrics = measure.call(this, text);

      // Simulate a platform that renders country flags as separate letters.
      if (text === "🇪🇺") {
        Object.defineProperty(metrics, "width", { value: 4 });
      }
      return metrics;
    };

    const path = "/frontend/lib/emoji-data.ts";
    const { resolveEmojiData } = await import(path);
    const data = await resolveEmojiData("en", { emojiVersion: 1 });

    return {
      count: data.emojis.length,
      flags: data.emojis.filter(
        (emoji: { countryFlag?: boolean }) => emoji.countryFlag
      ).length,
      versions: data.emojis.map((emoji: { version: number }) => emoji.version),
    };
  });

  expect(filtered.count).toBeGreaterThan(100);
  expect(filtered.flags).toBe(0);
  expect(Math.max(...filtered.versions)).toBeLessThanOrEqual(1);
});

test("keyboard browsing keeps the active emoji visible beyond the first rows in both directions", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  const active = editor(page).getByRole("gridcell", { selected: true });

  for (let index = 0; index < 18; index++) {
    await page.keyboard.press("ArrowDown");
    await expect(active).toHaveCount(1);
    await expect(active).toBeInViewport({ ratio: 1 });
  }

  for (let index = 0; index < 12; index++) {
    await page.keyboard.press("ArrowUp");
    await expect(active).toHaveCount(1);
    await expect(active).toBeInViewport({ ratio: 1 });
  }

  const selected = await active.textContent();
  await page.keyboard.press("Enter");
  await expect(editor(page).getByRole("img").locator(artwork)).toHaveText(
    selected?.trim() ?? ""
  );
  expect((await savedSnapshot(page)).workspaces[0]).toEqual(
    snapshot.workspaces[0]
  );
});
