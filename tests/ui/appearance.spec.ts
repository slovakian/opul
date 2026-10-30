import { expect, test } from "@playwright/test";

import { snapshotSchema } from "../../frontend/lib/model";
import treatments from "../../src/application/folder-treatments.json" with { type: "json" };
import { mockDesktop, snapshot } from "./fixture";

const artwork = '[data-slot="emoji-artwork"]';

test("folder appearance is identical across navigation, heading, and download choices", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");

  const workspace = page.getByRole("button", {
    exact: true,
    name: "Downloads workspace",
  });

  const icon = workspace.locator(artwork);
  await expect(icon).toHaveAttribute("data-preset", "gold");

  const filter = await icon.evaluate(
    (element) => getComputedStyle(element).filter
  );

  await workspace.click();
  await expect(
    page
      .getByRole("heading", { exact: true, name: "Downloads" })
      .locator(artwork)
  ).toHaveCSS("filter", filter);
  await page.getByRole("button", { name: "Toggle Downloads folders" }).click();
  await expect(
    page.getByRole("button", { name: "Toggle Downloads folders" })
  ).toHaveAttribute("aria-expanded", "false");
  await expect(icon).toHaveCSS("filter", filter);
  await page.getByRole("button", { name: "Toggle Downloads folders" }).click();
  await page
    .getByRole("button", {
      exact: true,
      name: "Save location: Downloads",
    })
    .click();

  const selector = page.getByRole("button", {
    exact: true,
    name: "Save location: Downloads",
  });

  await expect(selector.locator(artwork)).toHaveCSS("filter", filter);
  await expect(selector).toHaveCSS("filter", "none");

  await expect(
    page.getByRole("button", { exact: true, name: "Downloads (Default)" })
  ).toBeVisible();
});

for (const loseResponse of [false, true]) {
  test(`creation retains its proposed treatment and identity through polling and ${loseResponse ? "a lost response" : "a failed save"}`, async ({
    page,
  }) => {
    await mockDesktop(page, {
      changeSnapshotDuringDraft: true,
      failFirstCreate: !loseResponse,
      loseFirstCreateResponse: loseResponse,
    });
    await page.goto("/");
    await page
      .getByRole("button", { exact: true, name: "New workspace" })
      .last()
      .click();
    const name = page.getByRole("textbox", { exact: true, name: "Name" });
    await expect(name).toBeFocused();
    await name.fill("Research");

    const preview = page
      .getByRole("button", { exact: true, name: "Workspace emoji: 📁" })
      .locator(artwork);

    await expect(preview).toHaveAttribute("data-preset", "coral");

    const filter = await preview.evaluate(
      (element) => getComputedStyle(element).filter
    );

    // Observe a later polled snapshot without using a timing-based sleep.
    await expect(
      page
        .getByRole("button", {
          exact: true,
          includeHidden: true,
          name: "Downloads workspace",
        })
        .locator(artwork)
    ).toHaveAttribute("data-preset", "rose");
    await expect(preview).toHaveCSS("filter", filter);
    await page
      .getByRole("button", { exact: true, name: "Workspace emoji: 📁" })
      .click();
    await page.getByRole("textbox", { name: "Search emoji" }).fill("books");
    await page
      .locator('[data-slot="popover-content"]')
      .getByRole("gridcell", { exact: true, name: "books" })
      .click();
    await page.getByRole("button", { exact: true, name: "Done" }).click();
    await expect(
      page
        .getByRole("button", { exact: true, name: "Workspace emoji: 📚" })
        .locator(artwork)
    ).toHaveCSS("filter", "none");
    await page
      .getByRole("button", { exact: true, name: "Workspace emoji: 📚" })
      .click();
    await page
      .getByRole("textbox", { name: "Search emoji" })
      .fill("file folder");
    await page
      .locator('[data-slot="popover-content"]')
      .getByRole("gridcell", { exact: true, name: "file folder" })
      .click();
    await page.getByRole("button", { exact: true, name: "Done" }).click();
    await expect(preview).toHaveCSS("filter", filter);
    await page
      .getByRole("button", { exact: true, name: "Create workspace" })
      .click();
    await expect(
      page.getByText(
        loseResponse
          ? "The response was lost after saving. Try again."
          : "Couldn't save the checkpoint. Try again."
      )
    ).toBeVisible();
    await expect(preview).toHaveCSS("filter", filter);
    await expect(name).toHaveValue("Research");
    await page
      .getByRole("button", { exact: true, name: "Create workspace" })
      .click();
    await expect(
      page
        .getByRole("heading", { exact: true, name: "Research" })
        .locator(artwork)
    ).toHaveCSS("filter", filter);
    await expect(
      page
        .getByRole("button", { exact: true, name: "Research workspace" })
        .locator(artwork)
    ).toHaveCSS("filter", filter);
    await expect(
      page.getByRole("button", { exact: true, name: "Research workspace" })
    ).toHaveCount(1);

    const saved = snapshotSchema.parse(
      await page.evaluate(async () => {
        const modulePath = "/frontend/lib/bridge.ts";
        const { bridge } = await import(modulePath);

        return bridge.snapshot();
      })
    );

    const created = saved.workspaces.find(
      (workspace) => workspace.name === "Research"
    );

    expect(created?.id).toBe("draft-workspace-1");
    expect(created?.folders[0].id).toBe("draft-folder-1");
    expect(created?.appearance.automatic_folder).toEqual(treatments[1]);
  });
}

test("all twelve treatments render at sidebar size in light and dark themes", async ({
  page,
}) => {
  await page.setViewportSize({ height: 1040, width: 1220 });
  const library = structuredClone(snapshot);
  library.workspaces = treatments.map((treatment, index) => ({
    ...library.workspaces[0],
    appearance: {
      automatic_folder: treatment,
      preference: { kind: "Automatic" },
    },
    downloads: [],
    folders: [],
    id: `palette-${index}`,
    name: treatment.preset_id,
  }));
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");

  await Promise.all(
    treatments.map(async (treatment) => {
      const button = page.getByRole("button", {
        exact: true,
        name: `${treatment.preset_id} workspace`,
      });

      await expect(button.locator(artwork)).toHaveAttribute(
        "data-preset",
        treatment.preset_id
      );
      await expect(button).toHaveCSS("filter", "none");
    })
  );

  await page.screenshot({ path: "test-results/folders-light.png" });
  await page.getByRole("button", { exact: true, name: "Settings" }).click();
  const settings = page.getByRole("dialog", { exact: true, name: "Settings" });
  await settings.getByRole("button", { exact: true, name: "Dark" }).click();
  await settings.getByRole("button", { name: "Save changes" }).click();
  await expect(page.locator("html")).toHaveClass(/dark/u);
  await page.screenshot({ path: "test-results/folders-dark.png" });
});

test("native emoji sequences retain their original artwork even with an inactive folder assignment", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  const emojis = ["🐼", "👀", "🌈", "👩🏽‍💻", "👨‍👩‍👧‍👦", "⚪️"];
  library.workspaces = emojis.map((emoji, index) => ({
    ...library.workspaces[0],
    downloads: [],
    folders: [],
    icon: { kind: "Emoji", value: emoji },
    id: `native-${index}`,
    name: `Native ${index}`,
  }));
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");

  await Promise.all(
    emojis.map(async (emoji, index) => {
      const icon = page
        .getByRole("button", { exact: true, name: `Native ${index} workspace` })
        .locator(artwork);

      await expect(icon).toHaveText(emoji);
      await expect(icon).toHaveCSS("filter", "none");
    })
  );

  await page.screenshot({ path: "test-results/native-emoji.png" });
});
