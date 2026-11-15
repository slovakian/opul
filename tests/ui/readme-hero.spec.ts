import { mkdirSync } from "node:fs";
import path from "node:path";

import { expect, test } from "@playwright/test";

import folderTreatments from "../../src/application/folder-treatments.json" with { type: "json" };
import { mockDesktop } from "./fixture";
import { baseDownload, preferences, snapshot } from "./fixture-data";

const caps = (overrides: Partial<typeof baseDownload.capabilities> = {}) => ({
  ...baseDownload.capabilities,
  ...overrides,
});

const heroSnapshot = () => {
  const value = structuredClone(snapshot);
  const [coral, rose, orchid] = folderTreatments;

  value.workspaces = [
    {
      appearance: {
        automatic_folder: coral,
        preference: { kind: "Automatic" },
      },
      default_format: "Best",
      downloads: [
        {
          ...baseDownload,
          capabilities: caps({
            can_change_file_selection: true,
            can_stop_seeding: false,
          }),
          download_rate: 3_145_728,
          eta_seconds: 42,
          files: [
            {
              index: 0,
              path: "Harbor Lights Collection/readme.txt",
              priority: "Normal",
              size: 1200,
            },
            {
              index: 1,
              path: "Harbor Lights Collection/disc1.mkv",
              priority: "Normal",
              size: 4_294_967_296,
            },
          ],
          folder_id: "projects-default",
          id: "dl-torrent-active",
          info_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          name: "Harbor Lights Collection",
          peers: 18,
          piece_count: 2048,
          piece_length: 262_144,
          progress_millionths: 620_000,
          seeds: 6,
          source: "Torrent",
          source_url:
            "magnet:?xt=urn:btih:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          state: "Fetching",
          total_size: 4_294_968_496,
        },
        {
          ...baseDownload,
          bytes_downloaded: 41_943_040,
          capabilities: caps(),
          download_rate: 1_572_864,
          eta_seconds: 68,
          extractor: "generic",
          folder_id: "projects-default",
          format: "BestMp4",
          id: "dl-video-active",
          name: "Evening Walkthrough — Video page",
          page_url: "https://media.example/watch/evening-walkthrough",
          progress_millionths: 380_000,
          relies_on_video_tools: true,
          source: "VideoPage",
          source_url: "https://media.example/watch/evening-walkthrough",
          state: "Fetching",
          total_size: 110_100_480,
        },
        {
          ...baseDownload,
          bytes_downloaded: 8_388_608,
          capabilities: caps({ can_pause: false, can_resume: true }),
          download_rate: 0,
          eta_seconds: null,
          extractor: "generic",
          folder_id: "projects-audio",
          format: "AudioMp3",
          id: "dl-music-paused",
          name: "Soft Focus Demo — Music track",
          page_url: "https://audio.example/track/soft-focus-demo",
          progress_millionths: 720_000,
          relies_on_video_tools: true,
          source: "VideoPage",
          source_url: "https://audio.example/track/soft-focus-demo",
          state: "Paused",
          total_size: 11_534_336,
        },
      ],
      file_organization: "Automatic",
      folders: [
        {
          id: "projects-default",
          is_default: true,
          name: "Inbox",
          path: "/Users/alex/Opul/Projects",
        },
        {
          id: "projects-audio",
          is_default: false,
          name: "Audio references",
          path: "/Users/alex/Opul/Projects/Audio",
        },
      ],
      grouping: "None",
      icon: { kind: "Emoji", value: "📁" },
      id: "ws-projects",
      layout: "List",
      name: "Projects",
      sort: "Order",
    },
    {
      appearance: {
        automatic_folder: rose,
        preference: { kind: "Automatic" },
      },
      default_format: "Best",
      downloads: [
        {
          ...baseDownload,
          bytes_downloaded: 52_428_800,
          capabilities: caps({
            can_cancel: false,
            can_move: true,
            can_pause: false,
            can_reveal_output: true,
          }),
          download_rate: 0,
          eta_seconds: null,
          extractor: "generic",
          folder_id: "media-default",
          format: "BestMp4",
          id: "dl-video-done",
          name: "Studio Session Notes — Video page",
          output_path: "/Users/alex/Opul/Media/Studio Session Notes.mp4",
          page_url: "https://media.example/watch/studio-session-notes",
          progress_millionths: 1_000_000,
          relies_on_video_tools: true,
          source: "VideoPage",
          source_url: "https://media.example/watch/studio-session-notes",
          state: "Finished",
          total_size: 52_428_800,
        },
        {
          ...baseDownload,
          bytes_downloaded: 2_147_483_648,
          bytes_uploaded: 524_288_000,
          capabilities: caps({
            can_cancel: false,
            can_change_file_selection: true,
            can_pause: false,
            can_reveal_output: true,
            can_start_seeding: true,
            can_stop_seeding: false,
          }),
          download_rate: 0,
          eta_seconds: null,
          folder_id: "media-default",
          id: "dl-torrent-done",
          info_hash: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
          name: "Open Dataset Archive",
          output_path: "/Users/alex/Opul/Media/Open Dataset Archive",
          peers: 0,
          piece_count: 4096,
          piece_length: 524_288,
          progress_millionths: 1_000_000,
          seed_ratio: 0.24,
          seeds: 0,
          source: "Torrent",
          source_url:
            "magnet:?xt=urn:btih:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
          state: "NotSeeding",
          total_size: 2_147_483_648,
          upload_rate: 0,
        },
        {
          ...baseDownload,
          bytes_downloaded: 6_291_456,
          capabilities: caps({
            can_cancel: false,
            can_move: true,
            can_pause: false,
            can_reveal_output: true,
          }),
          download_rate: 0,
          eta_seconds: null,
          extractor: "generic",
          folder_id: "media-default",
          format: "AudioM4a",
          id: "dl-music-done",
          name: "Riverbank Sketch — Music track",
          output_path: "/Users/alex/Opul/Media/Riverbank Sketch.m4a",
          page_url: "https://audio.example/track/riverbank-sketch",
          progress_millionths: 1_000_000,
          relies_on_video_tools: true,
          source: "VideoPage",
          source_url: "https://audio.example/track/riverbank-sketch",
          state: "Finished",
          total_size: 6_291_456,
        },
      ],
      file_organization: "Automatic",
      folders: [
        {
          id: "media-default",
          is_default: true,
          name: "Library",
          path: "/Users/alex/Opul/Media",
        },
      ],
      grouping: "None",
      icon: { kind: "Emoji", value: "🎬" },
      id: "ws-media",
      layout: "List",
      name: "Media",
      sort: "Order",
    },
    {
      appearance: {
        automatic_folder: orchid,
        preference: { kind: "Automatic" },
      },
      default_format: "Original",
      downloads: [],
      file_organization: "KeepInSaveLocation",
      folders: [
        {
          id: "research-default",
          is_default: true,
          name: "Downloads",
          path: "/Users/alex/Opul/Research",
        },
      ],
      grouping: "None",
      icon: { kind: "Emoji", value: "📚" },
      id: "ws-research",
      layout: "List",
      name: "Research",
      sort: "Order",
    },
  ];

  return value;
};

test("capture README hero screenshot", async ({ page }) => {
  const outPath = path.resolve("assets/readme/opul-hero.png");
  mkdirSync(path.dirname(outPath), { recursive: true });

  await mockDesktop(page, {
    preferences: {
      ...preferences,
      theme: "dark",
      workspace_expansion: {
        "ws-media": true,
        "ws-projects": true,
        "ws-research": false,
      },
    },
    snapshot: heroSnapshot(),
  });
  await page.setViewportSize({ height: 820, width: 1280 });
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "All downloads" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Harbor Lights Collection" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Projects workspace" })
  ).toBeVisible();
  await page.waitForTimeout(500);
  await page.screenshot({ path: outPath, type: "png" });
});
