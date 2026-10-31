import { expect, test } from "@playwright/test";

import { mockDesktop, snapshot } from "./fixture";
import { baseDownload } from "./fixture-data";

test("torrent trackers and files render with trace, alignment, single-line truncation, and closed by default", async ({
  page,
}) => {
  const library = structuredClone(snapshot);

  const longTrackerUrl =
    "http://tracker.example.com:8080/announce?very_long_parameter=very_long_parameter_value_that_exceeds_normal_line_length_to_verify_single_line_truncation_behavior_and_does_not_wrap";

  const shortTrackerUrl = "udp://tracker.opentrackr.org:1337/announce";

  library.workspaces[0].downloads = [
    {
      ...baseDownload,
      files: [
        {
          index: 0,
          path: "Arch Linux 2026/archlinux-2026.01.01-x86_64.iso",
          priority: "normal",
          size: 943_718_400,
        },
        {
          index: 1,
          path: "Arch Linux 2026/sha256sums.txt",
          priority: "normal",
          size: 1024,
        },
      ],
      id: "download-torrent",
      name: "Arch Linux 2026.iso",
      source: "Torrent",
      source_url: "magnet:?xt=urn:btih:deadbeef&dn=Arch+Linux",
      trackers: [shortTrackerUrl, longTrackerUrl],
    },
  ];

  await mockDesktop(page, { snapshot: library });
  await page.goto("/");

  await expect(
    page.getByRole("button", { exact: true, name: "Arch Linux 2026.iso" })
  ).toBeVisible();

  // Open download inspector
  await page
    .getByRole("button", { exact: true, name: "Arch Linux 2026.iso" })
    .click();

  const details = page.getByRole("complementary", {
    name: "Download details",
  });

  await expect(details).toBeVisible();

  // Check the Files collapsible trigger
  const fileTrigger = details.getByRole("button", {
    exact: true,
    name: "2 files",
  });

  await expect(fileTrigger).toBeVisible();

  // Verify file trigger has a FileIcon
  const fileTriggerIcon = fileTrigger
    .locator('[data-slot="trace-icon"]')
    .first();

  await expect(fileTriggerIcon.locator("svg")).toBeVisible();

  // Check the Trackers collapsible trigger
  const trackerTrigger = details.getByRole("button", {
    exact: true,
    name: "2 trackers",
  });

  await expect(trackerTrigger).toBeVisible();

  // Verify tracker trigger has a GlobeIcon
  const trackerTriggerIcon = trackerTrigger
    .locator('[data-slot="trace-icon"]')
    .first();

  await expect(trackerTriggerIcon.locator("svg")).toBeVisible();

  // Verify both collapsible contents are CLOSED by default
  const collapsibles = details.locator('[data-slot="trace-collapsible"]');

  await expect(collapsibles).toHaveCount(2);

  const filesContent = collapsibles
    .nth(0)
    .locator('[data-slot="trace-collapsible-content"]');

  const trackersContent = collapsibles
    .nth(1)
    .locator('[data-slot="trace-collapsible-content"]');

  await expect(filesContent).toBeHidden();
  await expect(trackersContent).toBeHidden();

  // Expand trackers list
  await trackerTrigger.click();
  await expect(trackersContent).toBeVisible();

  // Verify each tracker row in the list
  const trackerLinks = trackersContent.locator("a");

  await expect(trackerLinks).toHaveCount(2);

  // Verify individual tracker rows do NOT have an icon on the left, but DO have the icon on the right
  const trackerLines = trackersContent.locator('[data-slot="trace-line"]');

  await expect(trackerLines).toHaveCount(2);

  const trackerLineCount = await trackerLines.count();

  await Promise.all(
    Array.from({ length: trackerLineCount }, async (_, index) => {
      const line = trackerLines.nth(index);
      const icons = line.locator('[data-slot="trace-icon"]');

      // Only one icon: the right arrow icon
      await expect(icons).toHaveCount(1);
      await expect(icons.first().locator("svg")).toBeVisible();
    })
  );

  // Verify horizontal alignment along the vertical line: right icons share the same right coordinate
  const firstIconRight = await trackerLines
    .nth(0)
    .locator('[data-slot="trace-icon"]')
    .evaluate((element) => element.getBoundingClientRect().right);

  const secondIconRight = await trackerLines
    .nth(1)
    .locator('[data-slot="trace-icon"]')
    .evaluate((element) => element.getBoundingClientRect().right);

  expect(Math.abs(firstIconRight - secondIconRight)).toBeLessThanOrEqual(1);

  // Verify single-line truncation of long tracker URL
  const longTrackerTitle = trackerLinks
    .nth(1)
    .locator('[data-slot="trace-title"]');

  await expect(longTrackerTitle).toHaveAttribute("title", longTrackerUrl);
  await expect(longTrackerTitle).toHaveClass(/truncate/u);

  const isTrackerTruncatedAndSingleLine = await longTrackerTitle.evaluate(
    (element) => {
      const computedStyle = window.getComputedStyle(element);

      // SAFETY: The trace title element is rendered as an HTMLSpanElement in the DOM.
      const htmlElement = element as HTMLElement;

      return {
        isNowrap: computedStyle.whiteSpace === "nowrap",
        isOverflowHidden: computedStyle.overflow === "hidden",
        isTextEllipsis: computedStyle.textOverflow === "ellipsis",
        singleLineHeight: htmlElement.offsetHeight <= 24,
      };
    }
  );

  expect(isTrackerTruncatedAndSingleLine.isNowrap).toBe(true);
  expect(isTrackerTruncatedAndSingleLine.isOverflowHidden).toBe(true);
  expect(isTrackerTruncatedAndSingleLine.isTextEllipsis).toBe(true);
  expect(isTrackerTruncatedAndSingleLine.singleLineHeight).toBe(true);

  // Expand files list
  await fileTrigger.click();
  await expect(filesContent).toBeVisible();

  // Verify file rows in the list: no left icon, truncated title, and inlined file size on right
  const fileLines = filesContent.locator('[data-slot="trace-line"]');

  await expect(fileLines).toHaveCount(2);

  const fileLineCount = await fileLines.count();

  await Promise.all(
    Array.from({ length: fileLineCount }, async (_, index) => {
      const line = fileLines.nth(index);
      const icons = line.locator('[data-slot="trace-icon"]');

      // No icons inside child file lines
      await expect(icons).toHaveCount(0);

      // Inlined file size exists
      const desc = line.locator('[data-slot="trace-description"]');

      await expect(desc).toBeVisible();
    })
  );

  // Verify file sizes share the same right alignment coordinate
  const firstSizeRight = await fileLines
    .nth(0)
    .locator('[data-slot="trace-description"]')
    .evaluate((element) => element.getBoundingClientRect().right);

  const secondSizeRight = await fileLines
    .nth(1)
    .locator('[data-slot="trace-description"]')
    .evaluate((element) => element.getBoundingClientRect().right);

  expect(Math.abs(firstSizeRight - secondSizeRight)).toBeLessThanOrEqual(1);
});

test("single tracker and single file render without collapsible parent-child wrapper", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  const singleTracker = "udp://tracker.single.org:1337/announce";

  library.workspaces[0].downloads = [
    {
      ...baseDownload,
      files: [
        {
          index: 0,
          path: "single-archive.tar.gz",
          priority: "normal",
          size: 52_428_800,
        },
      ],
      id: "download-single-torrent",
      name: "Single Archive Torrent",
      source: "Torrent",
      source_url: "magnet:?xt=urn:btih:feedface&dn=Archive",
      trackers: [singleTracker],
    },
  ];

  await mockDesktop(page, { snapshot: library });
  await page.goto("/");

  await page
    .getByRole("button", { exact: true, name: "Single Archive Torrent" })
    .click();

  const details = page.getByRole("complementary", {
    name: "Download details",
  });

  await expect(details).toBeVisible();

  // No collapsible triggers should exist since each list has only 1 item
  await expect(
    details.locator('[data-slot="trace-collapsible-trigger"]')
  ).toHaveCount(0);

  // Check the single file item: has left FileIcon, path, and inlined size
  const singleFileTitle = details
    .locator('[data-slot="trace-title"]')
    .filter({ hasText: "single-archive.tar.gz" });

  await expect(singleFileTitle).toBeVisible();

  const singleFileRow = singleFileTitle.locator("..");
  const singleFileIcon = singleFileRow.locator('[data-slot="trace-icon"]');

  await expect(singleFileIcon).toHaveCount(1);
  await expect(
    singleFileRow.locator('[data-slot="trace-description"]')
  ).toContainText("50 MB");

  // Check the single tracker item: has left GlobeIcon, URL, and right ArrowUpRightIcon
  const singleTrackerLink = details.locator(`a[href="${singleTracker}"]`);

  await expect(singleTrackerLink).toBeVisible();

  const singleTrackerRow = singleTrackerLink.locator(
    '[data-slot="trace-line"]'
  );

  const singleTrackerIcons = singleTrackerRow.locator(
    '[data-slot="trace-icon"]'
  );

  // Has both left GlobeIcon and right ArrowUpRightIcon
  await expect(singleTrackerIcons).toHaveCount(2);
  await expect(singleTrackerIcons.nth(0).locator("svg")).toBeVisible();
  await expect(singleTrackerIcons.nth(1).locator("svg")).toBeVisible();
});

test("source link renders as trace line with hover copy and feedback without layout shift", async ({
  page,
}) => {
  await page.addInitScript(() => {
    let clipboard = "";

    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        readText: () => Promise.resolve(clipboard),
        writeText: (text: string) => {
          clipboard = text;

          return Promise.resolve();
        },
      },
      writable: true,
    });
  });

  const library = structuredClone(snapshot);

  const magnetUrl =
    "magnet:?xt=urn:btih:deadbeef&dn=Arch+Linux+2026+x86_64+Complete+System+Installation+Media+ISO+With+Extra+Long+Parameters+To+Test+Truncation";

  library.workspaces[0].downloads = [
    {
      ...baseDownload,
      files: [],
      id: "download-torrent",
      name: "Arch Linux 2026.iso",
      source: "Torrent",
      source_url: magnetUrl,
      trackers: [],
    },
  ];

  await mockDesktop(page, { snapshot: library });
  await page.goto("/");

  await page
    .getByRole("button", { exact: true, name: "Arch Linux 2026.iso" })
    .click();

  const details = page.getByRole("complementary", {
    name: "Download details",
  });

  await expect(details).toBeVisible();

  // Verify the old "Original link" field label is gone
  await expect(details.getByText("Original link")).toHaveCount(0);

  // Locate the Trace source link button
  const linkTrace = details.locator('[data-slot="trace"]').first();
  const linkButton = linkTrace.locator("button");

  await expect(linkButton).toBeVisible();
  await expect(linkButton).toContainText(magnetUrl);

  const title = linkButton.locator('[data-slot="trace-title"]');

  await expect(title).toHaveAttribute("title", magnetUrl);
  await expect(title).toHaveClass(/truncate/u);

  // Verify single-line truncation
  const isTruncatedAndSingleLine = await title.evaluate((element) => {
    const computedStyle = window.getComputedStyle(element);

    // SAFETY: The trace title element is rendered as an HTMLSpanElement in the DOM.
    const htmlElement = element as HTMLElement;

    return {
      isNowrap: computedStyle.whiteSpace === "nowrap",
      isOverflowHidden: computedStyle.overflow === "hidden",
      isTextEllipsis: computedStyle.textOverflow === "ellipsis",
      singleLineHeight: htmlElement.offsetHeight <= 24,
    };
  });

  expect(isTruncatedAndSingleLine.isNowrap).toBe(true);
  expect(isTruncatedAndSingleLine.isOverflowHidden).toBe(true);
  expect(isTruncatedAndSingleLine.isTextEllipsis).toBe(true);
  expect(isTruncatedAndSingleLine.singleLineHeight).toBe(true);

  // Verify single trace icon container with resting icon and hover copy icon
  const icon = linkButton.locator('[data-slot="trace-icon"]');

  await expect(icon).toHaveCount(1);
  await expect(icon.locator("svg")).toHaveCount(2);
  await expect(icon.locator("svg").first()).toBeVisible();

  // Record height before click to check for layout shift
  const heightBefore = await linkButton.evaluate(
    (element) => element.getBoundingClientRect().height
  );

  // Click to copy
  await linkButton.click();

  // After click: title temporarily changes to "Copied" and icon swaps to Check
  await expect(title).toHaveText("Copied");
  await expect(linkButton).toHaveText("Copied");
  await expect(icon.locator("svg")).toHaveCount(1);
  await expect(icon.locator("svg")).toBeVisible();

  // Verify clipboard received the magnet URL
  const clipboardText = await page.evaluate(() =>
    navigator.clipboard.readText()
  );

  expect(clipboardText).toBe(magnetUrl);

  // Verify no layout shift occurred
  const heightAfter = await linkButton.evaluate(
    (element) => element.getBoundingClientRect().height
  );

  expect(heightAfter).toBe(heightBefore);
});

test("direct download link renders with Link icon in trace", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  const directUrl = "https://example.com/downloads/package.tar.gz";

  library.workspaces[0].downloads = [
    {
      ...baseDownload,
      files: [],
      id: "download-direct",
      name: "package.tar.gz",
      source: "Direct",
      source_url: directUrl,
      trackers: [],
    },
  ];

  await mockDesktop(page, { snapshot: library });
  await page.goto("/");

  await page
    .getByRole("button", { exact: true, name: "package.tar.gz" })
    .click();

  const details = page.getByRole("complementary", {
    name: "Download details",
  });

  await expect(details).toBeVisible();

  const linkTrace = details.locator('[data-slot="trace"]').first();
  const linkButton = linkTrace.locator("button");

  await expect(linkButton).toBeVisible();

  const title = linkButton.locator('[data-slot="trace-title"]');

  await expect(title).toHaveAttribute("title", directUrl);
});
