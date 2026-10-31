import bundled from "@/assets/emoji/data.json";

type SkinTone = "light" | "medium-light" | "medium" | "medium-dark" | "dark";

export type EmojiDetails = Omit<
  (typeof bundled.emojis)[number],
  "skins" | "countryFlag"
> & {
  countryFlag?: boolean;
  skins?: Record<SkinTone, string>;
};

export interface EmojiData {
  categories: typeof bundled.categories;
  emojis: EmojiDetails[];
  locale: string;
  skinTones: Record<SkinTone, string>;
}

export type EmojiDataResolver = (
  locale: string,
  options: { emojiVersion?: number; signal?: AbortSignal }
) => EmojiData | Promise<EmojiData>;

const data: EmojiData = {
  ...bundled,
  emojis: bundled.emojis.map((emoji) => ({
    ...emoji,
    countryFlag: emoji.countryFlag ? true : undefined,
    skins: emoji.skins ?? undefined,
  })),
};

const withoutPresentation = (emoji: string) =>
  emoji.replaceAll(/[\uFE0E\uFE0F]/gu, "");

const detailsBySequence = new Map<string, EmojiDetails>();

for (const item of data.emojis) {
  for (const sequence of [
    item.emoji,
    ...Object.values(item.skins ?? {}),
    ...(item.aliases ?? []),
  ]) {
    detailsBySequence.set(withoutPresentation(sequence), item);
  }
}

// Match metadata without changing the saved sequence, including mixed-tone aliases.
const details = (emoji: string) =>
  detailsBySequence.get(withoutPresentation(emoji));

const tones: (keyof EmojiData["skinTones"])[] = [
  "light",
  "medium-light",
  "medium",
  "medium-dark",
  "dark",
];

const toneLabels = new Map<string, string>(
  tones.map((tone, index) => [
    String.fromCodePoint(0x1_f3_fb + index),
    data.skinTones[tone].replace(/^./u, (letter) => letter.toUpperCase()),
  ])
);

// Frimousse's default resolver probes native color glyphs this way. Its custom
// resolver bypasses support filtering, so perform the probes on bundled data.
// See liveblocks/frimousse src/utils/is-emoji-supported.ts (MIT license bundled).
const nativeSupportProbe = () => {
  const context = document.createElement("canvas").getContext("2d", {
    willReadFrequently: true,
  });

  return (emoji: string) => {
    if (!context) {
      return false;
    }

    context.canvas.width = 2;
    context.canvas.height = 2;
    context.font =
      '2px "Apple Color Emoji", "Noto Color Emoji", "Segoe UI Emoji", sans-serif';
    context.textBaseline = "middle";

    if (context.measureText(emoji).width >= 4) {
      return false;
    }

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
};

let support: { flags: boolean; version: number } | undefined;

export const resolveEmojiData: EmojiDataResolver = (_locale, options) => {
  options.signal?.throwIfAborted();

  if (!support) {
    const probe = nativeSupportProbe();
    const versions = new Map<number, string>();

    for (const emoji of data.emojis) {
      if (!versions.has(emoji.version)) {
        versions.set(emoji.version, emoji.emoji);
      }
    }

    const supported = [...versions]
      .toSorted(([a], [b]) => b - a)
      .find(([, emoji]) => probe(emoji));

    support = { flags: probe("🇪🇺"), version: supported?.[0] ?? 0 };
  }

  const current = support;

  return {
    ...data,
    emojis: data.emojis.filter(
      (emoji) =>
        emoji.version <=
          Math.min(options.emojiVersion ?? current.version, current.version) &&
        (!emoji.countryFlag || current.flags)
    ),
  };
};

export const skinToneChoices = (emoji: string) => {
  const item = details(emoji);
  const skins = item?.skins;

  if (!item || !skins) {
    return [];
  }

  const probe = nativeSupportProbe();

  return [
    { emoji: item.emoji, label: "Default skin tone" },
    ...tones.map((tone) => ({
      emoji: skins[tone],
      label: data.skinTones[tone].replace(/^./u, (letter) =>
        letter.toUpperCase()
      ),
    })),
  ].filter((choice) => probe(choice.emoji));
};

// Freeze recent choices for one browsing session, so polling cannot reset search,
// keyboard position, or scroll. This resolver still uses the offline support filter.
export const emojiDataWithRecents =
  (recent: readonly string[]): EmojiDataResolver =>
  async (locale, options) => {
    const supported = await resolveEmojiData(locale, options);
    const supportedBase = new Set(supported.emojis.map((item) => item.emoji));
    const probe = nativeSupportProbe();
    const saved: EmojiDetails[] = [];

    for (const emoji of new Set(recent)) {
      const item = details(emoji);

      if (!item || !supportedBase.has(item.emoji) || !probe(emoji)) {
        continue;
      }

      const labels = [...emoji].flatMap((character) => {
        const label = toneLabels.get(character);

        return label ? [label] : [];
      });

      saved.push({
        ...item,
        category: -1,
        emoji,
        label:
          emoji === item.emoji
            ? item.label
            : [item.label, ...labels, emoji].join(", "),
        skins: undefined,
      });
    }

    const sequences = new Set(saved.map((item) => item.emoji));

    return {
      ...supported,
      categories: [
        { index: -1, label: "Recently used" },
        ...supported.categories,
      ],
      emojis: [
        ...saved,
        ...supported.emojis.filter((item) => !sequences.has(item.emoji)),
      ],
    };
  };
