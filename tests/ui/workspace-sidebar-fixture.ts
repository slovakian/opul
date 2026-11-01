import type { Page } from "@playwright/test";

import type { Snapshot } from "../../frontend/lib/model";
import { snapshot } from "./fixture";

export const sidebar = (page: Page) =>
  page.getByRole("navigation", { name: "Workspace navigation" });

export const breadcrumb = (page: Page) =>
  page.locator("main > header").getByRole("navigation", { name: "breadcrumb" });

export const input = (page: Page) =>
  page.getByRole("searchbox", { name: "Search downloads or paste a link" });

export const library = () => {
  const value = structuredClone(snapshot);
  const [workspace] = value.workspaces;
  workspace.name = "Research and reference material with a long workspace name";
  workspace.folders[0].name = "Saved reports";
  workspace.folders[0].path = "/Users/test/Research/Saved reports";
  workspace.downloads[0].name = "Report designs.zip";
  workspace.downloads[1].name = "Report interview.mp4";
  workspace.downloads[2].name = "Unrelated notes.pdf";
  value.workspaces.push({
    ...structuredClone(workspace),
    downloads: [],
    folders: [
      {
        id: "films",
        is_default: true,
        name: "Films",
        path: "/Users/test/Films",
      },
      {
        id: "archive",
        is_default: false,
        name: "Archive",
        path: "/Users/test/Archive",
      },
    ],
    icon: { kind: "Emoji", value: "🍿" },
    id: "media",
    name: "Media",
  });

  return value;
};

export const replaceSnapshot = async (page: Page, value: Snapshot) => {
  await page.evaluate((next) => {
    sessionStorage.setItem("test-snapshot", JSON.stringify(next));
    window.dispatchEvent(new Event("test:replace-snapshot"));
  }, value);
};
