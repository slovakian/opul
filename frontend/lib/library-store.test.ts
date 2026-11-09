import { expect, test } from "vite-plus/test";

import { baseDownload, snapshot } from "../../tests/ui/fixture-data";
import { applyFeedMessage, createLibraryStore } from "./library-store";
import type { Library } from "./library-store";
import { feedMessageSchema } from "./model";
import type { Download, FeedMessageInput } from "./model";

const [workspace] = snapshot.workspaces;

const second = {
  ...workspace,
  downloads: [{ ...baseDownload, id: "download-4", name: "Notes.txt" }],
  id: "workspace-2",
  name: "Notes",
};

const library = (): Library =>
  applyFeedMessage(
    null,
    feedMessageSchema.parse({
      ...snapshot,
      kind: "snapshot",
      workspaces: [workspace, second],
    })
  );

const apply = (current: Library, message: FeedMessageInput) =>
  applyFeedMessage(current, feedMessageSchema.parse(message));

const outlines = (current: Library, ids: Record<string, string[]>) =>
  current.workspaces.map((item) => ({
    ...item,
    download_ids: ids[item.id] ?? [],
  }));

test("a snapshot separates workspaces from the downloads they hold", () => {
  const current = library();

  expect(current.workspaces.map((item) => item.id)).toEqual([
    "workspace-1",
    "workspace-2",
  ]);
  expect(current.workspaces[0]).not.toHaveProperty("downloads");
  expect(
    current.downloads.map((download) => [download.id, download.workspace_id])
  ).toEqual([
    ["download-1", "workspace-1"],
    ["download-2", "workspace-1"],
    ["download-3", "workspace-1"],
    ["download-4", "workspace-2"],
  ]);
  expect(current.downloadById.get("download-4")?.name).toBe("Notes.txt");
});

test("a patch replaces only the downloads it carries", () => {
  const before = library();
  const changed: Download = { ...baseDownload, download_rate: 1 };

  const after = apply(before, {
    downloads: [changed],
    kind: "patch",
    revision: before.revision,
    sequence: before.sequence + 1,
  });

  expect(after.downloads[0]).toMatchObject({
    download_rate: 1,
    workspace_id: "workspace-1",
  });
  expect(after.downloadById.get("download-1")).toBe(after.downloads[0]);
  expect(after.downloads.slice(1)).toEqual(before.downloads.slice(1));

  for (const [index, download] of after.downloads.slice(1).entries()) {
    expect(download).toBe(before.downloads[index + 1]);
  }

  expect(after.workspaces).toBe(before.workspaces);
  expect(after.sequence).toBe(before.sequence + 1);
});

test("a patch that changes nothing visible keeps every collection", () => {
  const before = library();

  const after = apply(before, {
    kind: "patch",
    revision: before.revision + 1,
    sequence: before.sequence + 1,
  });

  expect(after.revision).toBe(before.revision + 1);
  expect(after.downloads).toBe(before.downloads);
  expect(after.downloadById).toBe(before.downloadById);
  expect(after.workspaces).toBe(before.workspaces);
  expect(after.recent_emoji).toBe(before.recent_emoji);
});

test("outlines remove, reorder, and move downloads while keeping the rest", () => {
  const before = library();

  const after = apply(before, {
    kind: "patch",
    revision: before.revision + 1,
    sequence: before.sequence + 1,
    workspaces: outlines(before, {
      "workspace-1": ["download-3", "download-1"],
      "workspace-2": ["download-4", "download-2"],
    }),
  });

  expect(
    after.downloads.map((download) => [download.id, download.workspace_id])
  ).toEqual([
    ["download-3", "workspace-1"],
    ["download-1", "workspace-1"],
    ["download-4", "workspace-2"],
    ["download-2", "workspace-2"],
  ]);
  expect(after.downloads[0]).toBe(before.downloadById.get("download-3"));
  expect(after.downloadById.size).toBe(4);
  expect(after.workspaces).toBe(before.workspaces);

  const removed = apply(after, {
    kind: "patch",
    revision: after.revision + 1,
    sequence: after.sequence + 1,
    workspaces: outlines(after, {
      "workspace-1": ["download-1"],
      "workspace-2": ["download-4"],
    }),
  });

  expect(removed.downloads.map((download) => download.id)).toEqual([
    "download-1",
    "download-4",
  ]);
  expect(removed.downloadById.has("download-3")).toBe(false);
});

test("a renamed workspace is the only workspace object that changes", () => {
  const before = library();

  const after = apply(before, {
    kind: "patch",
    revision: before.revision + 1,
    sequence: before.sequence + 1,
    workspaces: outlines(before, {
      "workspace-1": ["download-1", "download-2", "download-3"],
      "workspace-2": ["download-4"],
    }).map((item) =>
      item.id === "workspace-2" ? { ...item, name: "Reading" } : item
    ),
  });

  expect(after.workspaces[0]).toBe(before.workspaces[0]);
  expect(after.workspaces[1]?.name).toBe("Reading");
  expect(after.downloads).toBe(before.downloads);
});

test("an identical snapshot keeps every object", () => {
  const before = library();

  const after = apply(before, {
    ...snapshot,
    kind: "snapshot",
    sequence: before.sequence + 1,
    workspaces: [workspace, second],
  });

  expect(after.downloads).toBe(before.downloads);
  expect(after.downloadById).toBe(before.downloadById);
  expect(after.workspaces).toBe(before.workspaces);
});

test("an older message never undoes a newer one", () => {
  const before = library();

  const newer = apply(before, {
    kind: "patch",
    revision: before.revision + 1,
    sequence: before.sequence + 2,
  });

  expect(
    apply(newer, {
      kind: "patch",
      revision: before.revision,
      sequence: before.sequence + 1,
    })
  ).toBe(newer);
  expect(
    apply(newer, {
      ...snapshot,
      kind: "snapshot",
      workspaces: [],
    })
  ).toBe(newer);
});

test("a change before the library is a broken feed", () => {
  expect(() =>
    applyFeedMessage(
      null,
      feedMessageSchema.parse({ kind: "patch", revision: 1, sequence: 1 })
    )
  ).toThrow("before the library");
});

test("a command settles once the library reaches its revision", async () => {
  const store = createLibraryStore();
  const settled: string[] = [];

  const reached = (async () => {
    await store.reach(3, 10_000);
    settled.push("reached");
  })();

  store.receive(
    feedMessageSchema.parse({ ...snapshot, kind: "snapshot", revision: 2 })
  );
  await Promise.resolve();
  expect(settled).toEqual([]);

  store.receive(
    feedMessageSchema.parse({ kind: "patch", revision: 3, sequence: 2 })
  );
  await reached;
  expect(settled).toEqual(["reached"]);
});

test("a command settles after its timeout if the feed never shows it", async () => {
  const store = createLibraryStore();
  const started = performance.now();

  await store.reach(1, 20);

  expect(performance.now() - started).toBeGreaterThanOrEqual(15);
});

test("a failure is shown only until the library first loads", () => {
  const store = createLibraryStore();

  store.fail("The state directory is already in use");
  expect(store.getState()).toEqual({
    kind: "failed",
    message: "The state directory is already in use",
  });

  store.receive(feedMessageSchema.parse({ ...snapshot, kind: "snapshot" }));
  store.fail("Lost connection");
  expect(store.getState().kind).toBe("ready");
});
