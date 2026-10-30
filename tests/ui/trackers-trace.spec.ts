import { expect, test } from "@playwright/test";

import { mockDesktop, snapshot } from "./fixture";
import { baseDownload } from "./fixture-data";

test("torrent trackers render with shadcn trace, icons, single-line truncation, and toggle", async ({
  page,
}) => {
  const library = structuredClone(snapshot);

  const longTrackerUrl =
    "http://tracker.example.com:8080/announce?very_long_parameter=very_long_parameter_value_that_exceeds_normal_line_length_to_verify_single_line_truncation_behavior_and_does_not_wrap";

  const shortTrackerUrl = "udp://tracker.opentrackr.org:1337/announce";

  library.workspaces[0].downloads = [
    {
      ...baseDownload,
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

  // Check the TraceCollapsible trigger
  const trigger = details.locator('[data-slot="trace-collapsible-trigger"]');

  await expect(trigger).toBeVisible();
  await expect(trigger).toContainText("2 trackers");

  // Verify it is defaultOpen and trace container exists
  const trace = details.locator('[data-slot="trace"]');

  await expect(trace).toBeVisible();

  // Check both tracker links
  const links = trace.locator("a");

  await expect(links).toHaveCount(2);

  const firstLink = links.nth(0);

  await expect(firstLink).toHaveAttribute("href", shortTrackerUrl);
  await expect(firstLink).toHaveAttribute("target", "_blank");
  await expect(firstLink).toHaveAttribute("rel", "noopener noreferrer");

  const secondLink = links.nth(1);

  await expect(secondLink).toHaveAttribute("href", longTrackerUrl);
  await expect(secondLink).toHaveAttribute("target", "_blank");
  await expect(secondLink).toHaveAttribute("rel", "noopener noreferrer");

  // Verify left and right icons exist in each trace line
  const lines = trace.locator('[data-slot="trace-line"]');

  await expect(lines).toHaveCount(2);

  const lineCount = await lines.count();

  await Promise.all(
    Array.from({ length: lineCount }, async (_, index) => {
      const line = lines.nth(index);
      const icons = line.locator('[data-slot="trace-icon"]');

      await expect(icons).toHaveCount(2);
      await expect(icons.nth(0).locator("svg")).toBeVisible();
      await expect(icons.nth(1).locator("svg")).toBeVisible();
    })
  );

  // Verify title has truncation and title attribute
  const longTitle = secondLink.locator('[data-slot="trace-title"]');

  await expect(longTitle).toHaveAttribute("title", longTrackerUrl);
  await expect(longTitle).toHaveClass(/truncate/u);

  // Verify single-line behavior: nowrap, overflow hidden, text ellipsis, and height <= 24px
  const isTruncatedAndSingleLine = await longTitle.evaluate((element) => {
    const computedStyle = window.getComputedStyle(element);

    return {
      isNowrap: computedStyle.whiteSpace === "nowrap",
      isOverflowHidden: computedStyle.overflow === "hidden",
      isTextEllipsis: computedStyle.textOverflow === "ellipsis",
      singleLineHeight: element.offsetHeight <= 24,
    };
  });

  expect(isTruncatedAndSingleLine.isNowrap).toBe(true);
  expect(isTruncatedAndSingleLine.isOverflowHidden).toBe(true);
  expect(isTruncatedAndSingleLine.isTextEllipsis).toBe(true);
  expect(isTruncatedAndSingleLine.singleLineHeight).toBe(true);

  // Test collapsing and expanding
  await trigger.click();
  const content = details.locator('[data-slot="trace-collapsible-content"]');

  await expect(content).toBeHidden();

  await trigger.click();
  await expect(content).toBeVisible();
  await details.locator('[data-slot="sidebar-content"]').evaluate((el) => {
    el.scrollTop = el.scrollHeight;
  });
  await details.screenshot({ path: "test-results/trackers-trace.png" });
});
