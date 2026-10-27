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

const presentationKey = (emoji: string) =>
  emoji.replaceAll(/[\uFE0E\uFE0F]/gu, "");

export const isFolderEmoji = (emoji: string) => presentationKey(emoji) === "📁";

export const canTreatEmoji = (emoji: string) =>
  !excluded.has(presentationKey(emoji));

export const emojiTreatment = (
  emoji: string,
  appearance: Workspace["appearance"]
) => {
  if (!canTreatEmoji(emoji)) {
    return null;
  }

  const { preference } = appearance;

  switch (preference.kind) {
    case "Automatic": {
      return isFolderEmoji(emoji) ? appearance.automatic_folder : null;
    }

    case "Original": {
      return null;
    }

    case "Preset": {
      return preference.treatment;
    }

    case "Custom": {
      return preference;
    }

    default: {
      const exhaustive: never = preference;

      return exhaustive;
    }
  }
};
