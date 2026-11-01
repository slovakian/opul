import { expect, test } from "vite-plus/test";

import { preferencesSchema, workspaceExpansionSchema } from "./model";

test("older preference responses default to no remembered workspace choices", () => {
  const restored = preferencesSchema.parse({
    concurrent_downloads: 3,
    enable_dht: true,
    ffmpeg_path: null,
    listen_port: 6881,
    max_connections: 200,
    max_download_kbps: 0,
    max_upload_kbps: 0,
    randomize_port: true,
    theme: "system",
    yt_dlp_path: null,
  });

  expect(restored.workspace_expansion).toEqual({});
});

test("expansion IPC accepts independent boolean choices and rejects malformed values", () => {
  expect(
    workspaceExpansionSchema.parse({ media: false, research: true })
  ).toEqual({ media: false, research: true });
  expect(workspaceExpansionSchema.safeParse({ media: "true" }).success).toBe(
    false
  );
  expect(workspaceExpansionSchema.safeParse({ media: null }).success).toBe(
    false
  );
  expect(workspaceExpansionSchema.safeParse([]).success).toBe(false);
});
