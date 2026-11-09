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

test("seeding shows the upload speed or nothing, never peers", () => {
  expect(seedingText({ peers: 12, upload_rate: MB })).toEqual({
    kind: "seeding",
    upload: MB,
  });
  // A connection alone doesn't change the label.
  expect(seedingText({ peers: 1 })).toEqual({ kind: "seeding", upload: null });
});

test("status labels", () => {
  expect(statusText({ state: "Queued" })).toEqual({
    kind: "label",
    text: "Queued",
    tone: "plain",
  });
  expect(statusText({ state: "Failed" })).toEqual({ kind: "failed" });
  expect(
    statusText({ state: "WaitingForVideoTools", waiting_label: "Setting up…" })
  ).toEqual({ kind: "waiting", label: "Setting up…" });
});

const torrent = (patch: Partial<Download>) =>
  download({ source: "Torrent", ...patch });

test("a complete torrent that isn't seeding collapses like a finished file", () => {
  const view = rowView(torrent({ state: "NotSeeding" }));
  expect(view.status).toBe("empty");
  expect(view.bar).toBeNull();
  expect(view.description.line).toBe("origin");
});

test("starting to seed again checks files with no bar", () => {
  const view = rowView(torrent({ state: "CheckingFiles" }));
  expect(view.bar).toBeNull();
  expect(view.status).toBe("text");
  expect(view.statusText).toEqual({
    kind: "label",
    text: "Checking files…",
    tone: "shimmer",
  });
  expect(view.description.line).toBe("origin");
});

test.each([
  ["Starting", "Starting"],
  ["Fetching", "Fetching"],
] satisfies [string, Download["state"]][])(
  "a magnet waiting for its file list (%s) says so in place of the bar",
  (_name, state) => {
    const view = rowView(
      torrent({ bytes_downloaded: 0, state, total_size: 0 })
    );

    expect(view.status).toBe("text");
    expect(view.statusText).toEqual({
      kind: "label",
      text: "Getting file info…",
      tone: "shimmer",
    });
    // The ring stays below 440px, with the hint under the name.
    expect(view.ring).toBe(true);
    expect(view.bar).toMatchObject({ motion: "sheen", value: null });
    expect(view.description).toMatchObject({ line: "origin", size: false });
  }
);

test("a stalled torrent keeps its bar until the row says it's looking for peers", () => {
  const stalled = torrent({ download_rate: 0, eta_seconds: null, peers: 0 });
  expect(rowView(stalled).status).toBe("progress");

  const looking = rowView(stalled, { lookingForPeers: true });
  expect(looking.status).toBe("text");
  expect(looking.statusText).toEqual({
    kind: "label",
    text: "Looking for peers…",
    tone: "plain",
  });
  expect(looking.ring).toBe(true);
  expect(looking.description.line).toBe("live");

  // Peers or data mean it isn't stalled, whatever the row last decided.
  expect(
    rowView({ ...stalled, peers: 2 }, { lookingForPeers: true }).status
  ).toBe("progress");
});

test("other states keep their status text in the narrow line too", () => {
  expect(rowView(download({ state: "Queued" })).description.status).toEqual({
    kind: "label",
    text: "Queued",
    tone: "plain",
  });
  expect(
    rowView(download({ state: "Fetching" })).description.status
  ).toBeNull();
});
