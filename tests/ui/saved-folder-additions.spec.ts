/* oxlint-disable no-await-in-loop */
import { expect, test } from "@playwright/test";

import { mockDesktop } from "./fixture";
import type { DesktopOptions } from "./fixture-options";
import {
  addFolder,
  additionLibrary,
  chooseMedia,
  confirmFolder,
  folderRequest,
} from "./saved-folder-fixture";
import {
  breadcrumb,
  input,
  replaceSnapshot,
  sidebar,
} from "./workspace-sidebar-fixture";

for (const entry of ["sidebar", "search"] as const) {
  for (const count of [1, 2]) {
    for (const selected of [false, true]) {
      test(`${entry} addition to ${selected ? "selected" : "unselected"} workspace with ${count} folders applies its confirmed destination`, async ({
        page,
      }) => {
        const saved = additionLibrary(count);
        await mockDesktop(page, {
          folderPickerPath: "/Users/test/New reports",
          persist: true,
          snapshot: saved,
        });
        await page.goto("/");
        const active = selected ? "Media" : "Research";
        await sidebar(page)
          .getByRole("button", { exact: true, name: `${active} workspace` })
          .click();
        await input(page).fill("Report");

        if (entry === "search" && !selected) {
          await chooseMedia(page);
        }

        await addFolder(page, entry);
        const request = await folderRequest(page);
        expect(request.workspace_id).toBe("media");

        const toggle = sidebar(page).getByRole("button", {
          name: "Toggle Media folders",
        });

        await expect(toggle).toHaveAttribute(
          "aria-expanded",
          entry === "sidebar" || count === 1 ? "true" : "false"
        );
        await expect(input(page)).toHaveValue("Report");

        if (entry === "sidebar") {
          await expect(
            breadcrumb(page).getByRole("heading", {
              exact: true,
              name: "New reports",
            })
          ).toBeVisible();
          await expect(
            breadcrumb(page).getByRole("button", { exact: true, name: "Media" })
          ).toBeVisible();
          await expect(
            sidebar(page).getByRole("button", {
              exact: true,
              name: "New reports",
            })
          ).toHaveAttribute("data-active");
          await expect(
            page.getByRole("button", {
              exact: true,
              name: "Report designs.zip",
            })
          ).toHaveCount(0);
        } else {
          await expect(
            breadcrumb(page).getByRole("heading", { exact: true, name: active })
          ).toBeVisible();
          await expect(
            sidebar(page).getByRole("button", {
              exact: true,
              name: `${active} workspace`,
            })
          ).toHaveAttribute("data-active");
          await expect(
            page.getByRole("button", {
              exact: true,
              name: "Report designs.zip",
            })
          ).toHaveCount(selected ? 0 : 1);
        }

        await expect(
          page.getByRole("button", { name: /^Save location:/u })
        ).toHaveAttribute(
          "title",
          `${entry === "search" && !selected ? "Media › " : ""}New reports\n/Users/test/New reports`
        );
        await input(page).fill("https://example.com/report.zip");
        await input(page).press("Enter");
        await expect
          .poll(() =>
            page.evaluate(() =>
              JSON.parse(sessionStorage.getItem("test-command") ?? "null")
            )
          )
          .toMatchObject({
            folder_id: request.folder_id,
            kind: "add_url",
            workspace_id: "media",
          });
        await expect
          .poll(() =>
            page.evaluate(() =>
              JSON.parse(sessionStorage.getItem("test-snapshot") ?? "null")
                .workspaces.find((item: { id: string }) => item.id === "media")
                .folders.filter(
                  (item: { is_default: boolean }) => item.is_default
                )
                .map((item: { id: string }) => item.id)
            )
          )
          .toEqual(["films"]);
        await sidebar(page)
          .getByRole("button", { exact: true, name: "Media workspace" })
          .click();
        await expect(
          page.getByRole("button", {
            exact: true,
            name: "Save location: Films",
          })
        ).toBeVisible();
      });
    }
  }

  for (const failure of ["cancel", "picker", "rejected", "null"] as const) {
    test(`${entry} ${failure} preserves view, query, temporary destination, and expansion`, async ({
      page,
    }) => {
      const saved = additionLibrary(2);

      const options: DesktopOptions = {
        folderPickerPath: "/Users/test/New reports",
        snapshot: saved,
      };

      if (failure === "cancel") {
        options.folderPickerPath = null;
      } else if (failure === "picker") {
        options.folderPickerError = "Folder picker unavailable";
      } else {
        options.folderAddition = failure;
      }

      await mockDesktop(page, options);
      await page.goto("/");
      await sidebar(page)
        .getByRole("button", { exact: true, name: "Research workspace" })
        .click();
      await chooseMedia(page);
      await input(page).fill("Report");
      await addFolder(page, entry);

      if (failure !== "cancel") {
        await expect(
          page.getByText(
            failure === "picker"
              ? "Folder picker unavailable"
              : "Couldn't complete that action",
            { exact: true }
          )
        ).toBeVisible();
      }

      await expect(input(page)).toHaveValue("Report");
      await expect(
        breadcrumb(page).getByRole("heading", { exact: true, name: "Research" })
      ).toBeVisible();
      await expect(
        sidebar(page).getByRole("button", {
          exact: true,
          name: "Research workspace",
        })
      ).toHaveAttribute("data-active");
      await expect(
        page.getByRole("button", { exact: true, name: "Report designs.zip" })
      ).toBeVisible();
      await expect(
        page.getByRole("button", {
          exact: true,
          name: "Save location: Media › Films",
        })
      ).toBeVisible();
      await expect(
        sidebar(page).getByRole("button", { name: "Toggle Media folders" })
      ).toHaveAttribute("aria-expanded", "false");
      await expect(
        sidebar(page).getByRole("button", { exact: true, name: "New reports" })
      ).toHaveCount(0);
    });
  }

  for (const delay of ["deferred", "refresh-failure"] as const) {
    test(`${entry} retains accepted intent through ${delay} and applies it once on a later snapshot`, async ({
      page,
    }) => {
      await mockDesktop(page, {
        folderAddition: delay,
        folderPickerPath: "/Users/test/New reports",
        snapshot: additionLibrary(1),
      });
      await page.goto("/");
      await sidebar(page)
        .getByRole("button", { exact: true, name: "Research workspace" })
        .click();
      await chooseMedia(page);
      await input(page).fill("Report");
      await addFolder(page, entry);
      await folderRequest(page);
      await expect(
        page.getByRole("button", { exact: true, name: "Filter downloads" })
      ).toBeEnabled();
      await expect(
        breadcrumb(page).getByRole("heading", { exact: true, name: "Research" })
      ).toBeVisible();
      await expect(input(page)).toHaveValue("Report");
      await expect(
        page.getByRole("button", {
          exact: true,
          name: "Save location: Media › Films",
        })
      ).toBeVisible();
      await expect(
        sidebar(page).getByRole("button", { name: "Toggle Media folders" })
      ).toHaveCount(0);
      await input(page).focus();
      await confirmFolder(page);

      const toggle = sidebar(page).getByRole("button", {
        name: "Toggle Media folders",
      });

      await expect(toggle).toHaveAttribute("aria-expanded", "true");
      await expect(
        breadcrumb(page).getByRole("heading", {
          exact: true,
          name: entry === "sidebar" ? "New reports" : "Research",
        })
      ).toBeVisible();
      await expect(input(page)).toBeFocused();
      await expect(input(page)).toHaveValue("Report");
      await expect(
        page.getByRole("button", { name: /^Save location:/u })
      ).toHaveAttribute(
        "title",
        `${entry === "search" ? "Media › " : ""}New reports\n/Users/test/New reports`
      );
      await toggle.click();
      await expect(toggle).toHaveAttribute("aria-expanded", "false");
      await sidebar(page)
        .getByRole("button", { exact: true, name: "Research workspace" })
        .click();
      await expect(
        page.getByRole("button", {
          exact: true,
          name: "Save location: Saved reports",
        })
      ).toBeVisible();
      await expect
        .poll(() =>
          page.evaluate(() => sessionStorage.getItem("test-folder-attempts"))
        )
        .toBe("1");
      // A second authoritative snapshot must not replay the completed intent.
      await confirmFolder(page);
      await expect(toggle).toHaveAttribute("aria-expanded", "false");
      await expect(
        breadcrumb(page).getByRole("heading", { exact: true, name: "Research" })
      ).toBeVisible();
    });
  }
}

test("structural second-folder transitions expand only the affected existing workspace without navigation", async ({
  page,
}) => {
  const saved = additionLibrary(1);
  await mockDesktop(page, { snapshot: saved });
  await page.goto("/");
  await sidebar(page)
    .getByRole("button", { exact: true, name: "Research workspace" })
    .click();
  await input(page).fill("Report");
  saved.workspaces[1].folders.push({
    id: "later",
    is_default: false,
    name: "Later",
    path: "/Users/test/Later",
  });
  saved.sequence += 1;
  await replaceSnapshot(page, saved);

  const toggle = sidebar(page).getByRole("button", {
    name: "Toggle Media folders",
  });

  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(
    breadcrumb(page).getByRole("heading", { exact: true, name: "Research" })
  ).toBeVisible();
  await expect(input(page)).toHaveValue("Report");
  await expect(
    page.getByRole("button", {
      exact: true,
      name: "Save location: Saved reports",
    })
  ).toBeVisible();
  await toggle.click();
  saved.sequence += 1;
  await replaceSnapshot(page, saved);
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
});
