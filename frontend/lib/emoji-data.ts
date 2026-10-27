import type { EmojiData, EmojiDataResolver } from "frimousse";
import bundled from "@/assets/emoji/data.json";

const data: EmojiData = {
  ...bundled,
  emojis: bundled.emojis.map((emoji) => ({
    ...emoji,
    countryFlag: emoji.countryFlag ? true : undefined,
    skins: emoji.skins ?? undefined,
  })),
};

// Frimousse's default resolver probes native color glyphs this way. Its custom
// resolver bypasses support filtering, so perform the probes on bundled data.
// See liveblocks/frimousse src/utils/is-emoji-supported.ts (MIT license bundled).
function nativeSupportProbe() {
  const context = document.createElement("canvas").getContext("2d", {
    willReadFrequently: true,
  });
  return (emoji: string) => {
    if (!context) return false;
    context.canvas.width = 2;
    context.canvas.height = 2;
    context.font =
      '2px "Apple Color Emoji", "Noto Color Emoji", "Segoe UI Emoji", sans-serif';
    context.textBaseline = "middle";
    if (context.measureText(emoji).width >= 4) return false;
    context.fillStyle = "#00f";
    context.fillText(emoji, 0, 0);
    const blue = context.getImageData(0, 0, 2, 2).data;
    context.clearRect(0, 0, 2, 2);
    context.fillStyle = "#f00";
    context.fillText(emoji, 0, 0);
    const red = context.getImageData(0, 0, 2, 2).data;
    return (
      blue.some((value, index) => index % 4 === 3 && value > 0) &&
      blue.every((value, index) => value === red[index])
    );
  };
}

let support: { version: number; flags: boolean } | undefined;

export const resolveEmojiData: EmojiDataResolver = (_locale, options) => {
  options.signal?.throwIfAborted();
  if (!support) {
    const probe = nativeSupportProbe();
    const versions = new Map<number, string>();
    for (const emoji of data.emojis) {
      if (!versions.has(emoji.version))
        versions.set(emoji.version, emoji.emoji);
    }
    const supported = [...versions]
      .sort(([a], [b]) => b - a)
      .find(([, emoji]) => probe(emoji));
    support = { version: supported?.[0] ?? 0, flags: probe("🇪🇺") };
  }
  const current = support;
  return {
    ...data,
    emojis: data.emojis.filter(
      (emoji) =>
        emoji.version <=
          Math.min(options.emojiVersion ?? current.version, current.version) &&
        (!emoji.countryFlag || current.flags),
    ),
  };
};
