import type { EffectReducer } from "use-effect-reducer";

import type { EmojiData, EmojiDetails } from "@/lib/emoji-data";

export const EMOJI_COLUMNS = 8;

const navigationKeys = [
  "ArrowDown",
  "ArrowUp",
  "ArrowLeft",
  "ArrowRight",
  "Home",
  "End",
  "PageDown",
  "PageUp",
] as const;

type NavigationKey = (typeof navigationKeys)[number];

export const isEmojiNavigationKey = (key: string): key is NavigationKey =>
  navigationKeys.some((candidate) => candidate === key);

interface EmojiRow {
  category: number;
  emojis: EmojiDetails[];
}

export interface EmojiPosition {
  row: number;
  column: number;
}

interface SearchEntry {
  emoji: EmojiDetails;
  text: string;
}

export interface EmojiPickerState {
  data: EmojiData | null;
  index: SearchEntry[];
  rows: EmojiRow[];
  query: string;
  active: EmojiPosition | null;
  keyboard: boolean;
  error: string | null;
}

export type EmojiPickerEvent =
  | { type: "loaded"; data: EmojiData }
  | { type: "failed"; error: string }
  | { type: "retry" }
  | { type: "search"; query: string }
  | { type: "navigate"; key: NavigationKey; control: boolean }
  | { type: "focus"; position: EmojiPosition }
  | { type: "point"; position: EmojiPosition }
  | { type: "select"; emoji: string };

export type EmojiPickerEffect =
  | { type: "load" }
  | { type: "resetScroll" }
  | { type: "reveal"; position: EmojiPosition }
  | { type: "choose"; emoji: string };

export const initialEmojiPickerState: EmojiPickerState = {
  active: null,
  data: null,
  error: null,
  index: [],
  keyboard: false,
  query: "",
  rows: [],
};

const normalize = (text: string) =>
  text.normalize("NFKD").replaceAll(/\p{M}/gu, "").toLowerCase();

const filterRows = (
  data: EmojiData,
  index: SearchEntry[],
  query: string
): EmojiRow[] => {
  const terms = normalize(query).trim().split(/\s+/u).filter(Boolean);

  const matches = index.filter(({ text }) =>
    terms.every((term) => text.includes(term))
  );

  const rows: EmojiRow[] = [];

  for (const category of data.categories) {
    const emojis: EmojiDetails[] = [];

    for (const entry of matches) {
      if (entry.emoji.category === category.index) {
        emojis.push(entry.emoji);
      }
    }

    for (let start = 0; start < emojis.length; start += EMOJI_COLUMNS) {
      rows.push({
        category: category.index,
        emojis: emojis.slice(start, start + EMOJI_COLUMNS),
      });
    }
  }

  return rows;
};

const navigate = (
  state: EmojiPickerState,
  key: NavigationKey,
  control: boolean
): EmojiPosition | null => {
  if (state.rows.length === 0) {
    return null;
  }

  if (!state.active) {
    return { column: 0, row: 0 };
  }

  let { row, column } = state.active;
  const current = state.rows[row];

  switch (key) {
    case "ArrowDown": {
      row += 1;
      break;
    }

    case "ArrowUp": {
      row -= 1;
      break;
    }

    case "ArrowRight": {
      column += 1;

      if (column >= current.emojis.length && row < state.rows.length - 1) {
        row += 1;
        column = 0;
      }

      break;
    }

    case "ArrowLeft": {
      column -= 1;

      if (column < 0 && row > 0) {
        row -= 1;
        column = EMOJI_COLUMNS - 1;
      }

      break;
    }

    case "Home": {
      row = control ? 0 : row;
      column = 0;
      break;
    }

    case "End": {
      row = control ? state.rows.length - 1 : row;
      column = EMOJI_COLUMNS - 1;
      break;
    }

    case "PageDown": {
      const next = state.rows.findIndex(
        (candidate, index) =>
          index > row && candidate.category !== current.category
      );

      row = next === -1 ? state.rows.length - 1 : next;
      column = 0;
      break;
    }

    case "PageUp": {
      const { category } = state.rows[Math.max(0, row - 1)];
      row = state.rows.findIndex(
        (candidate) => candidate.category === category
      );
      column = 0;
      break;
    }

    default: {
      throw new Error(`Unhandled navigation key: ${key satisfies never}`);
    }
  }

  row = Math.max(0, Math.min(row, state.rows.length - 1));
  column = Math.max(0, Math.min(column, state.rows[row].emojis.length - 1));

  return { column, row };
};

export const emojiPickerReducer: EffectReducer<
  EmojiPickerState,
  EmojiPickerEvent,
  EmojiPickerEffect
> = (state, event, exec) => {
  switch (event.type) {
    case "loaded": {
      const index = event.data.emojis.map((emoji) => ({
        emoji,
        text: normalize(
          [emoji.emoji, emoji.label, ...emoji.tags, ...emoji.aliases].join(" ")
        ),
      }));

      return {
        ...state,
        data: event.data,
        error: null,
        index,
        rows: filterRows(event.data, index, state.query),
      };
    }

    case "failed": {
      return { ...state, error: event.error };
    }

    case "retry": {
      exec({ type: "load" });

      return { ...state, error: null };
    }

    case "search": {
      exec({ type: "resetScroll" });

      return {
        ...state,
        active: null,
        keyboard: false,
        query: event.query,
        rows: state.data
          ? filterRows(state.data, state.index, event.query)
          : [],
      };
    }

    case "navigate": {
      const active = navigate(state, event.key, event.control);

      if (active) {
        exec({ position: active, type: "reveal" });
      }

      return { ...state, active, keyboard: true };
    }

    case "point": {
      if (
        !state.keyboard &&
        state.active?.row === event.position.row &&
        state.active.column === event.position.column
      ) {
        return state;
      }

      return { ...state, active: event.position, keyboard: false };
    }

    case "focus": {
      return { ...state, active: event.position };
    }

    case "select": {
      exec({ emoji: event.emoji, type: "choose" });

      return state;
    }

    default: {
      throw new Error(`Unhandled picker event: ${event satisfies never}`);
    }
  }
};
