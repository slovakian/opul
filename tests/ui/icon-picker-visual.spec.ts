import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { mockDesktop } from "./fixture";

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

for (const theme of ["light", "dark"]) {
  test(`emoji fade beneath sticky category labels in ${theme} mode`, async ({
    page,
  }) => {
    await mockDesktop(page);
    await page.goto("/");

    if (theme === "dark") {
      await page.getByRole("button", { exact: true, name: "Settings" }).click();

      const settings = page.getByRole("dialog", {
        exact: true,
        name: "Settings",
      });

      await settings.getByRole("button", { exact: true, name: "Dark" }).click();
      await settings.getByRole("button", { name: "Save changes" }).click();
      await expect(page.locator("html")).toHaveClass(/dark/);
    }

    await openPicker(page);
    const viewport = editor(page).locator("[frimousse-viewport]");

    const headers = viewport.locator(
      "[frimousse-list-sizer] > [frimousse-category] > [frimousse-category-header]"
    );

    await expect(headers.first()).toHaveText("Smileys & Emotion");
    await expect(headers.filter({ hasText: "Travel & Places" })).toHaveCount(1);

    const { rowHeight, headerHeight } = await viewport.evaluate((element) => {
      const style = getComputedStyle(element);

      return {
        headerHeight: parseFloat(
          style.getPropertyValue("--frimousse-category-header-height")
        ),
        rowHeight: parseFloat(style.getPropertyValue("--frimousse-row-height")),
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
      rowHeight * 2 + delta
    );

    const faded = await viewport.screenshot({
      path: `test-results/emoji-sticky-fade-${theme}.png`,
      scale: "css",
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

          if (!context) {
            throw new Error("Canvas unavailable");
          }
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
          ) {
            for (let x = 0; x < image.width; x++)
              for (let channel = 0; channel < 3; channel++)
                total += Math.abs(
                  image.data[(y * image.width + x) * 4 + channel] -
                    image.data[channel]
                );
          }
          return total;
        }

        return contrast(clipped, fadedY) / contrast(full, fullY);
      },
      {
        faded: faded.toString("base64"),
        fadedY: headerHeight + rowHeight / 2 - delta,
        fullY: headerHeight + rowHeight / 2,
        reference: reference.toString("base64"),
      }
    );

    expect(contrastRatio).toBeGreaterThan(0.15);
    expect(contrastRatio).toBeLessThan(0.85);
    const viewportBounds = await viewport.boundingBox();
    expect((await headers.first().boundingBox())?.y).toBeCloseTo(
      viewportBounds?.y ?? 0,
      1
    );
    const next = headers.nth(1);

    const nextTop = await next.evaluate(
      (element) => element.parentElement?.offsetTop ?? 0
    );

    await viewport.evaluate(
      (element, y) => {
        element.scrollTop = y;
      },
      nextTop + rowHeight * 2
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
    exact: true,
    name: "thumbs up",
  });

  const tone = editor(page).getByRole("button", {
    exact: true,
    name: "Default skin tone",
  });

  const color = editor(page)
    .getByRole("group", { exact: true, name: "Color suggestions" })
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
