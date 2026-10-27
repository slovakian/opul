// Regenerate the offline Frimousse dataset from the locked Emojibase dependency.
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const emojis = require("emojibase-data/en/data.json");
const messages = require("emojibase-data/en/messages.json");

const tones = [
  "none",
  "light",
  "medium-light",
  "medium",
  "medium-dark",
  "dark",
];

const flags = new Set(
  messages.subgroups
    .filter(({ key }) => ["country-flag", "subdivision-flag"].includes(key))
    .map(({ order }) => order)
);

const data = {
  categories: messages.groups
    .filter(({ key }) => key !== "component")
    .map(({ order, message }) => ({ index: order, label: message })),
  emojis: emojis
    .filter((emoji) => "group" in emoji)
    .map((emoji) => ({
      emoji: emoji.emoji,
      label: emoji.label,
      tags: emoji.tags ?? [],
      category: emoji.group,
      version: emoji.version,
      countryFlag: flags.has(emoji.subgroup),
      skins: emoji.skins?.some(({ tone }) => typeof tone === "number")
        ? Object.fromEntries(
            emoji.skins
              .filter(({ tone }) => typeof tone === "number")
              .map(({ tone, emoji }) => [tones[tone], emoji])
          )
        : null,
      aliases:
        emoji.skins
          ?.filter(({ tone }) => Array.isArray(tone))
          .map(({ emoji }) => emoji) ?? [],
    })),
  locale: "en",
  skinTones: Object.fromEntries(
    messages.skinTones.map(({ key, message }) => [key, message])
  ),
};

writeFileSync(
  new URL("../frontend/assets/emoji/data.json", import.meta.url),
  `${JSON.stringify(data)}\n`
);

// Include the source license alongside the dataset.
writeFileSync(
  new URL("../frontend/assets/emoji/LICENSE-emojibase", import.meta.url),
  readFileSync(require.resolve("emojibase-data/LICENSE"))
);
