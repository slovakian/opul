import { describe, expect, it } from "vite-plus/test";

import {
  naturalDestination,
  recommendedWorkspace,
  workspaceDestination,
} from "./destination";
import type { Preferences, Workspace } from "./model";

const makeWorkspace = (id: string, name: string): Workspace => ({
  appearance: {
    automatic_folder: null,
    preference: { kind: "Automatic" },
  },
  default_format: "Best",
  downloads: [],
  file_organization: "Automatic",
  folders: [
    {
      id: `${id}-default`,
      is_default: true,
      name: "Default Folder",
      path: `/path/${id}/default`,
    },
    {
      id: `${id}-other`,
      is_default: false,
      name: "Other Folder",
      path: `/path/${id}/other`,
    },
  ],
  grouping: "None",
  icon: { kind: "Emoji", value: "📁" },
  id,
  layout: "List",
  name,
  sort: "Order",
});

describe("destination helpers", () => {
  const ws1 = makeWorkspace("ws-1", "Workspace 1");
  const ws2 = makeWorkspace("ws-2", "Workspace 2");
  const workspaces = [ws1, ws2];

  describe("workspaceDestination", () => {
    it("returns explicit folder when found", () => {
      const dest = workspaceDestination(ws1, "ws-1-other");
      expect(dest).toEqual({
        folder: ws1.folders[1],
        workspace: ws1,
      });
    });

    it("falls back to default folder when folderId not specified or not found", () => {
      const dest = workspaceDestination(ws1);
      expect(dest).toEqual({
        folder: ws1.folders[0],
        workspace: ws1,
      });
    });

    it("returns undefined if workspace is undefined", () => {
      expect(workspaceDestination(undefined)).toBeUndefined();
    });
  });

  describe("naturalDestination", () => {
    it("resolves workspace location explicitly", () => {
      const dest = naturalDestination(
        workspaces,
        { folderId: "ws-2-other", kind: "workspace", workspaceId: "ws-2" },
        null
      );

      expect(dest?.workspace.id).toBe("ws-2");
      expect(dest?.folder.id).toBe("ws-2-other");
    });

    it("resolves single workspace automatically for aggregate locations", () => {
      const dest = naturalDestination([ws1], { kind: "all" }, null);
      expect(dest?.workspace.id).toBe("ws-1");
      expect(dest?.folder.id).toBe("ws-1-default");
    });

    it("uses automatic workspace preference for aggregate locations when multiple workspaces exist", () => {
      const prefs: Preferences = {
        aggregate_destination: { kind: "workspace", workspace_id: "ws-2" },
        aggregate_views: {
          active: { grouping: "None", layout: "List", sort: "Order" },
          all: { grouping: "None", layout: "List", sort: "Order" },
          finished: { grouping: "None", layout: "List", sort: "Order" },
        },
        concurrent_downloads: 3,
        enable_dht: true,
        ffmpeg_path: null,
        listen_port: 6881,
        max_connections: 200,
        max_download_kbps: 0,
        max_upload_kbps: 0,
        randomize_port: true,
        theme: "light",
        yt_dlp_path: null,
      };

      const dest = naturalDestination(workspaces, { kind: "all" }, prefs);
      expect(dest?.workspace.id).toBe("ws-2");
    });

    it("returns undefined for prompt preference when multiple workspaces exist", () => {
      const prefs: Preferences = {
        aggregate_destination: { kind: "prompt" },
        aggregate_views: {
          active: { grouping: "None", layout: "List", sort: "Order" },
          all: { grouping: "None", layout: "List", sort: "Order" },
          finished: { grouping: "None", layout: "List", sort: "Order" },
        },
        concurrent_downloads: 3,
        enable_dht: true,
        ffmpeg_path: null,
        listen_port: 6881,
        max_connections: 200,
        max_download_kbps: 0,
        max_upload_kbps: 0,
        randomize_port: true,
        theme: "light",
        yt_dlp_path: null,
      };

      const dest = naturalDestination(workspaces, { kind: "all" }, prefs);
      expect(dest).toBeUndefined();
    });
  });

  describe("recommendedWorkspace", () => {
    it("prefers configured automatic workspace", () => {
      const prefs: Preferences = {
        aggregate_destination: { kind: "workspace", workspace_id: "ws-2" },
        aggregate_views: {
          active: { grouping: "None", layout: "List", sort: "Order" },
          all: { grouping: "None", layout: "List", sort: "Order" },
          finished: { grouping: "None", layout: "List", sort: "Order" },
        },
        concurrent_downloads: 3,
        enable_dht: true,
        ffmpeg_path: null,
        listen_port: 6881,
        max_connections: 200,
        max_download_kbps: 0,
        max_upload_kbps: 0,
        randomize_port: true,
        theme: "light",
        yt_dlp_path: null,
      };

      expect(recommendedWorkspace(workspaces, prefs, "ws-1").id).toBe("ws-2");
    });

    it("falls back to lastUsed if no configured workspace", () => {
      expect(recommendedWorkspace(workspaces, null, "ws-2").id).toBe("ws-2");
    });

    it("falls back to first workspace if neither configured nor lastUsed match", () => {
      expect(recommendedWorkspace(workspaces, null).id).toBe("ws-1");
    });
  });
});
