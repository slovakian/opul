import type { Workspace } from "@/lib/model";

// Curated for native artwork with little useful hue/chroma to adjust. Normalize
// presentation selectors only for matching; never alter the saved sequence.
const excluded = new Set([
  "⚪",
  "⚫",
  "⬜",
  "⬛",
  "◻",
  "◼",
  "◽",
  "◾",
  "▫",
  "▪",
  "🤍",
  "🖤",
  "🩶",
  "☠",
  "💀",
  "👻",
  "🕳",
  "🏴",
  "🏳",
  "♟",
  "🔲",
  "🔳",
  "➕",
  "➖",
  "➗",
  "✖",
  "✔",
  "☑",
  "➰",
  "➿",
  "〰",
  "❔",
  "❕",
  "🔘",
  "🔌",
]);

function presentationKey(emoji: string) {
  return emoji.replace(/[\uFE0E\uFE0F]/gu, "");
}

export function isFolderEmoji(emoji: string) {
  return presentationKey(emoji) === "📁";
}

export function canTreatEmoji(emoji: string) {
  return !excluded.has(presentationKey(emoji));
}

export function emojiTreatment(
  emoji: string,
  appearance: Workspace["appearance"],
) {
  if (!canTreatEmoji(emoji)) return null;
  const preference = appearance.preference;
  switch (preference.kind) {
    case "Automatic":
      return isFolderEmoji(emoji) ? appearance.automatic_folder : null;
    case "Original":
      return null;
    case "Preset":
      return preference.treatment;
    case "Custom":
      return preference;
    default: {
      const exhaustive: never = preference;
      return exhaustive;
    }
  }
}
