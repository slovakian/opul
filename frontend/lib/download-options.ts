import type { Command } from "@/lib/model";

export type DownloadMode = Extract<Command, { kind: "add_url" }>["mode"];

export type DownloadType = "Automatic" | DownloadMode;

export const downloadTypes = [
  { label: "Automatic", value: "Automatic" },
  { label: "File", value: "Direct" },
  { label: "Video or playlist", value: "Page" },
  { label: "Torrent", value: "Torrent" },
] as const;

// Only explicit file suffixes and torrent links bypass the uncertainty prompt.
export const downloadMode = (link: string): DownloadMode | undefined => {
  if (/^magnet:/iu.test(link)) {
    return "Torrent";
  }

  let pathname = "";

  try {
    ({ pathname } = new URL(link));
  } catch {
    return undefined;
  }

  if (/\.torrent$/iu.test(pathname)) {
    return "Torrent";
  }

  if (
    /\.(?:zip|gz|bz2|xz|7z|rar|tar|pdf|dmg|pkg|exe|msi|iso|bin|txt|csv|json|png|jpe?g|gif|webp|svg|mp4|mkv|webm|mov|mp3|m4a|wav|flac)$/iu.test(
      pathname
    )
  ) {
    return "Direct";
  }

  return undefined;
};
