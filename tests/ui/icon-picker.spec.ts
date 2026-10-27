import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { mockDesktop, snapshot } from "./fixture";
import { snapshotSchema } from "../../frontend/lib/model";
import treatments from "../../src/application/folder-treatments.json" with { type: "json" };

const artwork = '[data-slot="emoji-artwork"]';
const editor = (page: Page) =>
  page.getByRole("dialog", { name: "Change icon", exact: true });

async function openPicker(page: Page) {
  await page
    .getByRole("button", { name: "Downloads actions", exact: true })
    .click();
  await page.getByRole("button", { name: "Change icon…", exact: true }).click();
  await expect(
    editor(page).getByRole("textbox", { name: "Search emoji" }),
  ).toBeFocused();
}

async function choose(page: Page, search: string, label: string) {
  await editor(page)
    .getByRole("textbox", { name: "Search emoji" })
    .fill(search);
  await editor(page)
    .getByRole("gridcell", { name: label, exact: true })
    .click();
  await expect(editor(page)).toBeVisible();
}

async function savedSnapshot(page: Page) {
  return snapshotSchema.parse(
    await page.evaluate(async () => {
      const path = "/frontend/lib/bridge.ts";
      const { bridge } = await import(path);
      return bridge.snapshot();
    }),
  );
}

for (const theme of ["light", "dark"]) {
  test(`emoji fade beneath sticky category labels in ${theme} mode`, async ({
    page,
  }) => {
    await mockDesktop(page);
    await page.goto("/");
    if (theme === "dark") {
      await page.getByRole("button", { name: "Settings", exact: true }).click();
      const settings = page.getByRole("dialog", {
        name: "Settings",
        exact: true,
      });
      await settings.getByRole("button", { name: "Dark", exact: true }).click();
      await settings.getByRole("button", { name: "Save changes" }).click();
      await expect(page.locator("html")).toHaveClass(/dark/);
    }
    await openPicker(page);
    const viewport = editor(page).locator("[frimousse-viewport]");
    const headers = viewport.locator(
      "[frimousse-list-sizer] > [frimousse-category] > [frimousse-category-header]",
    );
    await expect(headers.first()).toHaveText("Smileys & Emotion");
    await expect(headers.filter({ hasText: "Travel & Places" })).toHaveCount(1);
    const { rowHeight, headerHeight } = await viewport.evaluate((element) => {
      const style = getComputedStyle(element);
      return {
        rowHeight: parseFloat(style.getPropertyValue("--frimousse-row-height")),
        headerHeight: parseFloat(
          style.getPropertyValue("--frimousse-category-header-height"),
        ),
      };
    });
    await viewport.evaluate((element, y) => {
      element.scrollTop = y;
    }, rowHeight * 2);
    const reference = await viewport.screenshot({ scale: "css" });
    const delta = rowHeight / 2 + 4;
    await viewport.evaluate(
      (element, y) => {
        element.scrollTop = y;
      },
      rowHeight * 2 + delta,
    );
    const faded = await viewport.screenshot({
      scale: "css",
      path: `test-results/emoji-sticky-fade-${theme}.png`,
    });
    // Compare the same glyph pixels before and after they move behind the label.
    const contrastRatio = await page.evaluate(
      async ({ reference, faded, fullY, fadedY }) => {
        async function pixels(encoded: string) {
          const image = new Image();
          image.src = `data:image/png;base64,${encoded}`;
          await image.decode();
          const canvas = document.createElement("canvas");
          canvas.width = image.width;
          canvas.height = image.height;
          const context = canvas.getContext("2d");
          if (!context) throw new Error("Canvas unavailable");
          context.drawImage(image, 0, 0);
          return context.getImageData(0, 0, image.width, image.height);
        }
        const full = await pixels(reference);
        const clipped = await pixels(faded);
        function contrast(image: ImageData, centerY: number) {
          let total = 0;
          for (
            let y = Math.round(centerY) - 1;
            y <= Math.round(centerY) + 1;
            y++
          )
            for (let x = 0; x < image.width; x++)
              for (let channel = 0; channel < 3; channel++)
                total += Math.abs(
                  image.data[(y * image.width + x) * 4 + channel] -
                    image.data[channel],
                );
          return total;
        }
        return contrast(clipped, fadedY) / contrast(full, fullY);
      },
      {
        reference: reference.toString("base64"),
        faded: faded.toString("base64"),
        fullY: headerHeight + rowHeight / 2,
        fadedY: headerHeight + rowHeight / 2 - delta,
      },
    );
    expect(contrastRatio).toBeGreaterThan(0.15);
    expect(contrastRatio).toBeLessThan(0.85);
    const viewportBounds = await viewport.boundingBox();
    expect((await headers.first().boundingBox())?.y).toBeCloseTo(
      viewportBounds?.y ?? 0,
      1,
    );
    const next = headers.nth(1);
    const nextTop = await next.evaluate(
      (element) => element.parentElement?.offsetTop ?? 0,
    );
    await viewport.evaluate(
      (element, y) => {
        element.scrollTop = y;
      },
      nextTop + rowHeight * 2,
    );
    await expect(next).toHaveText("People & Body");
    await expect
      .poll(async () => (await next.boundingBox())?.y)
      .toBeCloseTo(viewportBounds?.y ?? 0, 1);
    await viewport.screenshot({
      path: `test-results/emoji-next-category-${theme}.png`,
    });
  });
}

test("emoji options stay transparent when chosen and only grow their artwork on hover", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  await choose(page, "thumbs up", "thumbs up");
  const cell = editor(page).getByRole("gridcell", {
    name: "thumbs up",
    exact: true,
  });
  const tone = editor(page).getByRole("button", {
    name: "Default skin tone",
    exact: true,
  });
  const color = editor(page)
    .getByRole("group", { name: "Color suggestions", exact: true })
    .getByRole("button")
    .first();
  await color.click();
  await page.mouse.move(0, 0);
  const badge = '[data-slot="emoji-frame-badge"]';
  for (const option of [cell, tone, color]) {
    await expect(option.locator(badge)).toHaveCount(1);
    await expect(option).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
    await expect(option).toHaveCSS("border-top-width", "0px");
    await expect(option).toHaveCSS("box-shadow", "none");
    const frame = option.locator('[data-slot="emoji-frame"]');
    await expect(frame).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
    const glyph = option.locator('[data-slot="emoji-frame-artwork"]');
    const bounds = await option.boundingBox();
    const before = await glyph.boundingBox();
    const checkBefore = await option.locator(badge).boundingBox();
    await option.hover();
    await expect(frame).not.toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
    await expect
      .poll(async () => {
        const after = await glyph.boundingBox();
        return after && before ? after.width / before.width : 0;
      })
      .toBeCloseTo(1.1, 2);
    expect(await option.boundingBox()).toEqual(bounds);
    expect(await option.locator(badge).boundingBox()).toEqual(checkBefore);
    await page.mouse.move(0, 0);
    await expect(frame).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  }
  await page.screenshot({ path: "test-results/emoji-check-badges.png" });
});

test("search previews locally, keeps the picker open, and saves the full sequence", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  await expect(
    editor(page).locator("[frimousse-category-header]:visible").first(),
  ).toHaveText("Smileys & Emotion");
  await choose(page, "family", "family: man, woman, girl, boy");
  await expect(
    editor(page).getByRole("img", { name: "Icon preview: 👨‍👩‍👧‍👦" }),
  ).toBeVisible();
  expect((await savedSnapshot(page)).workspaces[0].icon).toEqual(
    snapshot.workspaces[0].icon,
  );
  await editor(page).getByRole("button", { name: "Save", exact: true }).click();
  await expect(editor(page)).toHaveCount(0);
  await expect(
    page
      .getByRole("button", { name: "Downloads workspace", exact: true })
      .locator(artwork),
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
    if (close === "Cancel")
      await editor(page)
        .getByRole("button", { name: "Cancel", exact: true })
        .click();
    else if (close === "Escape") await page.keyboard.press("Escape");
    else await page.mouse.click(650, 450);
    await expect(editor(page)).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Downloads actions", exact: true }),
    ).toBeFocused();
    expect((await savedSnapshot(page)).workspaces[0]).toEqual(
      snapshot.workspaces[0],
    );
    await openPicker(page);
    await expect(
      editor(page).getByRole("img", { name: "Icon preview: 📁" }),
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
    await openPicker(page);
    await choose(page, "panda", "panda");
    // An unrelated setting changes while the picker owns a local draft.
    await page.evaluate(async () => {
      const path = "/frontend/lib/bridge.ts";
      const { bridge } = await import(path);
      await bridge.execute({
        kind: "format",
        workspace_id: "workspace-1",
        format: "AudioMp3",
      });
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
    await expect(
      editor(page).getByRole("img", { name: "Icon preview: 🐼" }),
    ).toBeVisible();
    await editor(page)
      .getByRole("button", { name: "Save", exact: true })
      .click();
    await expect(editor(page).getByRole("alert")).toContainText(
      "Your changes are kept",
    );
    await expect(
      editor(page).getByRole("textbox", { name: "Search emoji" }),
    ).toHaveValue("panda");
    await expect(
      editor(page).getByRole("img", { name: "Icon preview: 🐼" }),
    ).toBeVisible();
    await editor(page)
      .getByRole("button", { name: "Save", exact: true })
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
    .getByRole("button", { name: "Original", exact: true })
    .click();
  await expect(preview).toHaveCSS("filter", "none");
  await editor(page).getByRole("button", { name: "Save", exact: true }).click();
  await expect(editor(page)).toHaveCount(0);
  await openPicker(page);
  await expect(
    editor(page).getByRole("button", { name: "Original", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await expect(editor(page).getByRole("img").locator(artwork)).toHaveCSS(
    "filter",
    "none",
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
    editor(page).getByRole("group", { name: "Icon appearance", exact: true }),
  ).toHaveCount(0);
  await editor(page).getByRole("button", { name: "Save", exact: true }).click();
  await expect(editor(page)).toHaveCount(0);
  expect(
    (await savedSnapshot(page)).workspaces[0].appearance.preference,
  ).toEqual({ kind: "Preset", treatment: treatments[1] });
  await openPicker(page);
  await choose(page, "panda", "panda");
  await expect(editor(page).getByRole("img").locator(artwork)).toHaveAttribute(
    "data-preset",
    "coral",
  );
  await editor(page).getByRole("button", { name: "Save", exact: true }).click();
  await expect(editor(page)).toHaveCount(0);
  await expect(
    page
      .getByRole("button", { name: "Downloads workspace", exact: true })
      .locator(artwork),
  ).toHaveAttribute("data-preset", "coral");
});

test("first use has no emoji-data requests and supports keyboard selection at minimum size", async ({
  page,
  context,
  baseURL,
}) => {
  const external: string[] = [];
  page.on("request", (request) => {
    if (!request.url().startsWith(baseURL ?? "http://localhost:1420"))
      external.push(request.url());
  });
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.setViewportSize({ width: 720, height: 520 });
  await mockDesktop(page);
  await page.goto("/");
  await page
    .getByRole("button", { name: "Toggle Sidebar", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Downloads workspace", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Downloads", exact: true }),
  ).toBeVisible();
  await context.setOffline(true);
  await page
    .getByRole("button", { name: "Workspace actions", exact: true })
    .click();
  await page.getByRole("button", { name: "Change icon…", exact: true }).click();
  const search = editor(page).getByRole("textbox", { name: "Search emoji" });
  await search.fill("panda");
  await expect(
    editor(page).getByRole("gridcell", { name: "panda", exact: true }),
  ).toBeVisible();
  await search.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(
    editor(page).getByRole("img", { name: "Icon preview: 🐼" }),
  ).toBeVisible();
  await expect(
    editor(page).getByRole("button", { name: "Save", exact: true }),
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
            !button.getAttribute("aria-label") && !button.textContent?.trim(),
        ).length,
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
    id: `appearance-${index}`,
    name: `Icon ${index}`,
    icon: { kind: "Emoji", value },
    appearance: {
      automatic_folder: treatments[0],
      preference: { kind: "Custom", hue: 120, saturation: 180, brightness: 95 },
    },
    folders: [],
    downloads: [],
  }));
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  for (const [index, value] of emojis.entries()) {
    const icon = page
      .getByRole("button", { name: `Icon ${index} workspace`, exact: true })
      .locator(artwork);
    await expect(icon).toHaveText(value);
    await expect(icon).toHaveCSS(
      "filter",
      index < 4 ? "none" : "hue-rotate(120deg) saturate(1.8) brightness(0.95)",
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
    "none",
  );
  await editor(page)
    .getByRole("button", { name: "Automatic", exact: true })
    .click();
  await expect(editor(page).getByRole("img").locator(artwork)).toHaveAttribute(
    "data-preset",
    "coral",
  );
  await editor(page).getByRole("button", { name: "Save", exact: true }).click();
  await expect(editor(page)).toHaveCount(0);
  expect(
    (await savedSnapshot(page)).workspaces[0].appearance.automatic_folder,
  ).toEqual(treatments[1]);
});

test("save errors keep the actions reachable in the minimum window", async ({
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
  await editor(page).getByRole("button", { name: "Save", exact: true }).click();
  await expect(editor(page).getByRole("alert")).toBeVisible();
  const save = editor(page).getByRole("button", { name: "Save", exact: true });
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
    CanvasRenderingContext2D.prototype.measureText = function (text) {
      const metrics = measure.call(this, text);
      // Simulate a platform that renders country flags as separate letters.
      if (text === "🇪🇺") Object.defineProperty(metrics, "width", { value: 4 });
      return metrics;
    };
    const path = "/frontend/lib/emoji-data.ts";
    const { resolveEmojiData } = await import(path);
    const data = await resolveEmojiData("en", { emojiVersion: 1 });
    return {
      versions: data.emojis.map((emoji: { version: number }) => emoji.version),
      count: data.emojis.length,
      flags: data.emojis.filter(
        (emoji: { countryFlag?: boolean }) => emoji.countryFlag,
      ).length,
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
    selected?.trim() ?? "",
  );
  expect((await savedSnapshot(page)).workspaces[0]).toEqual(
    snapshot.workspaces[0],
  );
});
