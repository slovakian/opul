import { expect } from "@playwright/test";
import type { Page } from "@playwright/test";

import type { FolderRequest } from "../../frontend/lib/model";
import { library } from "./workspace-sidebar-fixture";

export type Entry = "sidebar" | "search";

export const additionLibrary = (count: number) => {
  const value = library();
  value.workspaces[0].name = "Research";
  value.workspaces[1].folders = value.workspaces[1].folders.slice(0, count);

  return value;
};

export const addFolder = async (page: Page, entry: Entry) => {
  const trigger =
    entry === "sidebar"
      ? page.getByRole("button", {
          exact: true,
          name: "Media actions",
        })
      : page.getByRole("button", { name: /^Save location:/u });

  await trigger.click();

  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", {
      exact: true,
      name: entry === "sidebar" ? "Add saved folder" : "Add new location…",
    })
    .click();
};

export const chooseMedia = async (page: Page) => {
  await page.getByRole("button", { name: /^Save location:/u }).click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", {
      exact: true,
      name: "Films (Default)",
    })
    .click();
};

export const folderRequest = async (page: Page): Promise<FolderRequest> => {
  await expect
    .poll(() =>
      page.evaluate(() => sessionStorage.getItem("test-folder-command"))
    )
    .not.toBeNull();

  return page.evaluate(
    () =>
      JSON.parse(sessionStorage.getItem("test-folder-command") ?? "null")
        .request
  );
};

export const confirmFolder = async (page: Page) => {
  const previous = await page.evaluate(() =>
    sessionStorage.getItem("test-snapshot-sequence")
  );

  await page.evaluate(() =>
    window.dispatchEvent(new Event("test:confirm-folder"))
  );
  await expect
    .poll(() =>
      page.evaluate(() => sessionStorage.getItem("test-snapshot-sequence"))
    )
    .not.toBe(previous);
};
