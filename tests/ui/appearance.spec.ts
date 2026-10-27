import { expect, test } from "@playwright/test";
import { mockDesktop, snapshot } from "./fixture";
import { snapshotSchema } from "../../frontend/lib/model";
import treatments from "../../src/application/folder-treatments.json" with { type: "json" };

const artwork = '[data-slot="emoji-artwork"]';

test("folder appearance is identical across navigation, heading, and download choices", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  const workspace = page.getByRole("button", {
    name: "Downloads workspace",
    exact: true,
  });
  const icon = workspace.locator(artwork);
  await expect(icon).toHaveAttribute("data-preset", "gold");
  const filter = await icon.evaluate(
    (element) => getComputedStyle(element).filter,
  );
  await workspace.click();
  await expect(
    page
      .getByRole("heading", { name: "Downloads", exact: true })
      .locator(artwork),
  ).toHaveCSS("filter", filter);
  await page.getByRole("button", { name: "Toggle Downloads folders" }).click();
  await expect(
    page.getByRole("button", { name: "Toggle Downloads folders" }),
  ).toHaveAttribute("aria-expanded", "false");
  await expect(icon).toHaveCSS("filter", filter);
  await page.getByRole("button", { name: "Toggle Downloads folders" }).click();
  await page
    .getByRole("button", { name: "Download options", exact: true })
    .click();
  const selector = page.getByRole("button", {
    name: "Download workspace: Downloads",
    exact: true,
  });
  await expect(selector.locator(artwork)).toHaveCSS("filter", filter);
  await expect(selector).toHaveCSS("filter", "none");
  await selector.click();
  const option = page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { name: "Downloads", exact: true });
  await expect(option.locator(artwork)).toHaveCSS("filter", filter);
  await expect(option).toHaveCSS("filter", "none");
});

for (const loseResponse of [false, true]) {
  test(`creation retains its proposed treatment and identity through polling and ${loseResponse ? "a lost response" : "a failed save"}`, async ({
    page,
  }) => {
    await mockDesktop(page, {
      failFirstCreate: !loseResponse,
      loseFirstCreateResponse: loseResponse,
      changeSnapshotDuringDraft: true,
    });
    await page.goto("/");
    await page
      .getByRole("button", { name: "New workspace", exact: true })
      .last()
      .click();
    const name = page.getByRole("textbox", { name: "Name", exact: true });
    await expect(name).toBeFocused();
    await name.fill("Research");
    const preview = page
      .getByRole("button", { name: "Workspace emoji: 📁", exact: true })
      .locator(artwork);
    await expect(preview).toHaveAttribute("data-preset", "coral");
    const filter = await preview.evaluate(
      (element) => getComputedStyle(element).filter,
    );
    // Observe a later polled snapshot without using a timing-based sleep.
    await expect(
      page
        .getByRole("button", {
          name: "Downloads workspace",
          exact: true,
          includeHidden: true,
        })
        .locator(artwork),
    ).toHaveAttribute("data-preset", "rose");
    await expect(preview).toHaveCSS("filter", filter);
    await page
      .getByRole("button", { name: "Workspace emoji: 📁", exact: true })
      .click();
    await page.getByRole("textbox", { name: "Search emoji" }).fill("books");
    await page
      .locator('[data-slot="popover-content"]')
      .getByRole("gridcell", { name: "books", exact: true })
      .click();
    await page.getByRole("button", { name: "Done", exact: true }).click();
    await expect(
      page
        .getByRole("button", { name: "Workspace emoji: 📚", exact: true })
        .locator(artwork),
    ).toHaveCSS("filter", "none");
    await page
      .getByRole("button", { name: "Workspace emoji: 📚", exact: true })
      .click();
    await page
      .getByRole("textbox", { name: "Search emoji" })
      .fill("file folder");
    await page
      .locator('[data-slot="popover-content"]')
      .getByRole("gridcell", { name: "file folder", exact: true })
      .click();
    await page.getByRole("button", { name: "Done", exact: true }).click();
    await expect(preview).toHaveCSS("filter", filter);
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await expect(
      page.getByText(
        loseResponse
          ? "The response was lost after saving. Try again."
          : "Couldn't save the checkpoint. Try again.",
      ),
    ).toBeVisible();
    await expect(preview).toHaveCSS("filter", filter);
    await expect(name).toHaveValue("Research");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await expect(
      page
        .getByRole("heading", { name: "Research", exact: true })
        .locator(artwork),
    ).toHaveCSS("filter", filter);
    await expect(
      page
        .getByRole("button", { name: "Research workspace", exact: true })
        .locator(artwork),
    ).toHaveCSS("filter", filter);
    await expect(
      page.getByRole("button", { name: "Research workspace", exact: true }),
    ).toHaveCount(1);
    const saved = snapshotSchema.parse(
      await page.evaluate(async () => {
        const modulePath = "/frontend/lib/bridge.ts";
        const { bridge } = await import(modulePath);
        return bridge.snapshot();
      }),
    );
    const created = saved.workspaces.find(
      (workspace) => workspace.name === "Research",
    );
    expect(created?.id).toBe("draft-workspace-1");
    expect(created?.folders[0].id).toBe("draft-folder-1");
    expect(created?.appearance.automatic_folder).toEqual(treatments[1]);
  });
}

test("all twelve treatments render at sidebar size in light and dark themes", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1220, height: 1040 });
  const library = structuredClone(snapshot);
  library.workspaces = treatments.map((treatment, index) => ({
    ...library.workspaces[0],
    id: `palette-${index}`,
    name: treatment.preset_id,
    appearance: {
      automatic_folder: treatment,
      preference: { kind: "Automatic" },
    },
    folders: [],
    downloads: [],
  }));
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  for (const treatment of treatments) {
    const button = page.getByRole("button", {
      name: `${treatment.preset_id} workspace`,
      exact: true,
    });
    await expect(button.locator(artwork)).toHaveAttribute(
      "data-preset",
      treatment.preset_id,
    );
    await expect(button).toHaveCSS("filter", "none");
  }
  await page.screenshot({ path: "test-results/folders-light.png" });
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  const settings = page.getByRole("dialog", { name: "Settings", exact: true });
  await settings.getByRole("button", { name: "Dark", exact: true }).click();
  await settings.getByRole("button", { name: "Save changes" }).click();
  await expect(page.locator("html")).toHaveClass(/dark/);
  await page.screenshot({ path: "test-results/folders-dark.png" });
});

test("native emoji sequences retain their original artwork even with an inactive folder assignment", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  const emojis = ["🐼", "👀", "🌈", "👩🏽‍💻", "👨‍👩‍👧‍👦", "⚪️"];
  library.workspaces = emojis.map((emoji, index) => ({
    ...library.workspaces[0],
    id: `native-${index}`,
    name: `Native ${index}`,
    icon: { kind: "Emoji", value: emoji },
    folders: [],
    downloads: [],
  }));
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  for (const [index, emoji] of emojis.entries()) {
    const icon = page
      .getByRole("button", { name: `Native ${index} workspace`, exact: true })
      .locator(artwork);
    await expect(icon).toHaveText(emoji);
    await expect(icon).toHaveCSS("filter", "none");
  }
  await page.screenshot({ path: "test-results/native-emoji.png" });
});
