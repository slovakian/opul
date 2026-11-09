import { expect, test } from "vite-plus/test";

import { baseDownload } from "../../tests/ui/fixture-data";
import type { Download } from "./model";
import { rowView } from "./row-view";

const MB = 1024 * 1024;

const download = (patch: Partial<Download>): Download => ({
  ...baseDownload,
  bytes_downloaded: 25.9 * MB,
  download_rate: 1.2 * MB,
  eta_seconds: 300,
  peers: 0,
  progress_millionths: 60_000,
  total_size: 432 * MB,
  upload_rate: 0,
  ...patch,
});

/** The description as a row would read it, without the grace-period hooks. */
const line = (patch: Partial<Download>) => {
  const { description } = rowView(download(patch));

  if (description.line === "message") {
    return description.message?.text;
  }

  if (description.line === "origin") {
    return "origin";
  }

  const { amount, eta, note, speed } = description;

  return [
    amount.total ? `${amount.done} of ${amount.total}` : amount.done,
    speed === null ? null : `${speed} B/s`,
    eta === null ? null : `${eta}s`,
    note,
  ]
    .filter(Boolean)
    .join(" · ");
};

const bar = (patch: Partial<Download>) => rowView(download(patch)).bar;

const statusOf = (state: Download["state"]) =>
  rowView(download({ state })).status;

const seedingText = (patch: Partial<Download>) =>
  rowView(download({ state: "Seeding", ...patch })).statusText;

const statusText = (patch: Partial<Download>) =>
  rowView(download(patch)).statusText;

test.each([
  ["Fetching", {}, "25.9 MB of 432 MB · 1258291.2 B/s · 300s"],
  ["Pausing", { state: "Pausing" }, "25.9 MB of 432 MB · Pausing…"],
  ["Paused", { state: "Paused" }, "25.9 MB of 432 MB · Paused"],
  ["Resuming", { state: "Starting" }, "25.9 MB of 432 MB · Resuming…"],
  ["Starting from zero", { bytes_downloaded: 0, state: "Starting" }, "origin"],
  ["Processing", { state: "Processing" }, "Processing…"],
  ["Verifying", { state: "Verifying" }, "Verifying…"],
  ["Seeding", { state: "Seeding" }, "origin"],
  ["Queued", { state: "Queued" }, "origin"],
  ["Finished", { state: "Finished" }, "origin"],
  ["Cancelled", { state: "Cancelled" }, "origin"],
  ["Failed", { error_message: "Timed out", state: "Failed" }, "Timed out"],
  ["Failed without a message", { state: "Failed" }, "origin"],
] satisfies [string, Partial<Download>, string][])(
  "%s: the description puts the amount first",
  (_name, patch, expected) => {
    expect(line(patch)).toBe(expected);
  }
);

test("speed and time left are left out while they're empty", () => {
  expect(line({ download_rate: 0, eta_seconds: null })).toBe(
    "25.9 MB of 432 MB"
  );
  expect(line({ eta_seconds: null })).toBe("25.9 MB of 432 MB · 1258291.2 B/s");
});

test("an unknown total shows the amount alone and the no-progress bar", () => {
  expect(line({ eta_seconds: null, total_size: 0 })).toBe(
    "25.9 MB · 1258291.2 B/s"
  );
  expect(bar({ total_size: 0 })).toEqual({
    label: "Downloading…",
    motion: "sheen",
    value: null,
  });
});

test.each([
  ["Fetching", {}, { label: "6%", motion: "sheen", value: 6 }],
  [
    "Starting from zero",
    { bytes_downloaded: 0, progress_millionths: 0, state: "Starting" },
    { label: "Starting…", motion: "sheen", value: null },
  ],
  [
    "Resuming keeps its value",
    { state: "Starting" },
    { label: "Resuming at 6%", motion: "pulse", value: 6 },
  ],
  [
    "Resuming without a total",
    { state: "Starting", total_size: 0 },
    { label: "Resuming…", motion: "sheen", value: null },
  ],
  [
    "Pausing",
    { state: "Pausing" },
    { label: "Paused at 6%", motion: "muted", value: 6 },
  ],
  [
    "Paused",
    { state: "Paused" },
    { label: "Paused at 6%", motion: "muted", value: 6 },
  ],
  [
    "Paused without a total",
    { state: "Paused", total_size: 0 },
    { label: "Paused", motion: "muted", value: null },
  ],
  [
    "Processing stays full and still",
    { state: "Processing" },
    { label: "Processing…", motion: "still", value: 100 },
  ],
  [
    "Verifying",
    { state: "Verifying" },
    { label: "6%", motion: "sheen", value: 6 },
  ],
] satisfies [string, Partial<Download>, ReturnType<typeof bar>][])(
  "%s: bar",
  (_name, patch, expected) => {
    expect(bar(patch)).toEqual(expected);
  }
);

test.each([
  ["Seeding", "Seeding"],
  ["WaitingForVideoTools", "WaitingForVideoTools"],
  ["Queued", "Queued"],
  ["Failed", "Failed"],
  ["Cancelled", "Cancelled"],
  ["Finished", "Finished"],
] satisfies [string, Download["state"]][])("%s: has no bar", (_name, state) => {
  expect(rowView(download({ state })).bar).toBeNull();
});

test("status slot: bar while transferring, text while waiting, nothing when finished", () => {
  expect(statusOf("Fetching")).toBe("progress");
  expect(statusOf("Paused")).toBe("progress");
  expect(statusOf("Seeding")).toBe("text");
  expect(statusOf("Queued")).toBe("text");
  expect(statusOf("Failed")).toBe("text");
  expect(statusOf("WaitingForVideoTools")).toBe("text");
  expect(statusOf("Finished")).toBe("empty");
});

test("seeding leaves out an idle upload and zero peers", () => {
  expect(seedingText({ peers: 12, upload_rate: MB })).toEqual({
    kind: "seeding",
    peers: 12,
    upload: MB,
  });
  expect(seedingText({})).toEqual({
    kind: "seeding",
    peers: null,
    upload: null,
  });
});

test("status labels", () => {
  expect(statusText({ state: "Queued" })).toEqual({
    kind: "label",
    text: "Queued",
  });
  expect(statusText({ state: "Failed" })).toEqual({ kind: "failed" });
  expect(
    statusText({ state: "WaitingForVideoTools", waiting_label: "Setting up…" })
  ).toEqual({ kind: "waiting", label: "Setting up…" });
});
