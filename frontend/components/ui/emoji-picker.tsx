import { Search } from "lucide-react";
import { useId, useRef } from "react";
import type { KeyboardEvent } from "react";
import { useEffectReducer } from "use-effect-reducer";

import { Button } from "@/components/ui/button";
import { EmojiFrame, emojiOptionClassName } from "@/components/ui/emoji-frame";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from "@/components/ui/input-group";
import { errorMessage } from "@/lib/bridge";
import type { EmojiDataResolver } from "@/lib/emoji-data";
import {
  EMOJI_COLUMNS,
  emojiPickerReducer,
  initialEmojiPickerState,
  isEmojiNavigationKey,
} from "@/lib/emoji-picker";
import type { EmojiPosition } from "@/lib/emoji-picker";
import { cn } from "@/lib/utils";

export const EmojiPicker = ({
  chosenEmoji,
  resolveData,
  onSelect,
}: {
  chosenEmoji: string;
  resolveData: EmojiDataResolver;
  onSelect: (emoji: string) => void;
}) => {
  const id = useId();
  const viewport = useRef<HTMLDivElement>(null);
  const cells = useRef(new Map<string, HTMLButtonElement>());

  const [state, dispatch] = useEffectReducer(
    emojiPickerReducer,
    (exec) => {
      exec({ type: "load" });

      return initialEmojiPickerState;
    },
    {
      choose: (_state, effect) => onSelect(effect.emoji),
      load: async (_state, _effect, send, { signal }) => {
        try {
          const data = await resolveData("en", { signal });

          if (!signal.aborted) {
            send({ data, type: "loaded" });
          }
        } catch (error) {
          if (!signal.aborted) {
            send({ error: errorMessage(error), type: "failed" });
          }
        }
      },
      resetScroll: () => {
        viewport.current?.scrollTo({ behavior: "instant", top: 0 });
      },
      reveal: (_state, { position }) => {
        const cell = cells.current.get(`${position.row}:${position.column}`);
        cell?.focus({ preventScroll: true });
        cell?.scrollIntoView({
          behavior: "instant",
          block: "nearest",
          inline: "nearest",
        });
      },
    }
  );

  const handleKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.nativeEvent.isComposing || event.altKey) {
      return;
    }

    const inSearch = event.target instanceof HTMLInputElement;

    if (inSearch && !["ArrowDown", "ArrowUp"].includes(event.key)) {
      return;
    }

    if (isEmojiNavigationKey(event.key)) {
      event.preventDefault();
      dispatch({
        control: event.ctrlKey || event.metaKey,
        key: event.key,
        type: "navigate",
      });
    }
  };

  return (
    <div className="flex min-h-[7.25rem] flex-1 flex-col gap-2">
      <InputGroup>
        <InputGroupInput
          autoFocus
          aria-label="Search emoji"
          aria-controls={`${id}-grid`}
          placeholder="Search emoji…"
          value={state.query}
          onChange={(event) =>
            dispatch({ query: event.target.value, type: "search" })
          }
          onKeyDown={handleKeyDown}
        />
        <InputGroupAddon>
          <Search />
        </InputGroupAddon>
      </InputGroup>
      <div
        ref={viewport}
        data-slot="emoji-viewport"
        className="scroll-fade-b scroll-fade-2 focus-visible:ring-ring relative min-h-[4.5rem] flex-1 overflow-y-auto overscroll-contain rounded-md outline-none [--emoji-category-header-height:32px] [--emoji-row-height:36px] focus-visible:ring-2"
      >
        <span id={`${id}-chosen`} className="sr-only">
          Selected icon
        </span>
        {state.error ? (
          <div role="alert" className="text-muted-foreground p-4 text-sm">
            {state.error}
            <Button variant="ghost" onClick={() => dispatch("retry")}>
              Try again
            </Button>
          </div>
        ) : null}
        {!state.data && !state.error && (
          <output className="text-muted-foreground block p-4 text-sm">
            Loading emoji…
          </output>
        )}
        {state.data && state.rows.length === 0 && (
          <output className="text-muted-foreground block p-4 text-sm">
            No emoji found.
          </output>
        )}
        <div
          id={`${id}-grid`}
          data-slot="emoji-grid"
          role="grid"
          aria-label="Emoji categories"
          aria-colcount={EMOJI_COLUMNS}
          aria-rowcount={state.rows.length}
          tabIndex={-1}
          onKeyDown={handleKeyDown}
        >
          {state.data?.categories.map((category) => {
            const rows = state.rows.flatMap((row, index) =>
              row.category === category.index ? [{ ...row, index }] : []
            );

            if (rows.length === 0) {
              return null;
            }

            return (
              <div
                key={category.index}
                // Interactive layout grid, rather than tabular data.
                // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role
                role="rowgroup"
                aria-labelledby={`${id}-category-${category.index}`}
                data-slot="emoji-category"
                className="relative"
              >
                <div
                  id={`${id}-category-${category.index}`}
                  data-slot="emoji-category-header"
                  className="from-popover text-muted-foreground sticky top-0 z-10 h-[var(--emoji-category-header-height)] bg-linear-to-b from-[calc(100%_-_var(--scroll-fade-size))] to-transparent px-1 py-2 text-xs font-medium"
                >
                  {category.label.replaceAll(/\b[a-z]/gu, (letter) =>
                    letter.toUpperCase()
                  )}
                </div>
                {rows.map((row) => (
                  <div
                    key={row.index}
                    // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role
                    role="row"
                    aria-rowindex={row.index + 1}
                    className="grid h-[var(--emoji-row-height)] grid-cols-8"
                  >
                    {row.emojis.map((emoji, column) => {
                      const position: EmojiPosition = {
                        column,
                        row: row.index,
                      };

                      const active =
                        state.active?.row === row.index &&
                        state.active.column === column;

                      const chosen = emoji.emoji === chosenEmoji;

                      return (
                        <button
                          key={emoji.emoji}
                          ref={(cell) => {
                            const key = `${row.index}:${column}`;

                            if (cell) {
                              cells.current.set(key, cell);
                            } else {
                              cells.current.delete(key);
                            }
                          }}
                          type="button"
                          // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role
                          role="gridcell"
                          aria-label={emoji.label}
                          aria-colindex={column + 1}
                          aria-selected={active}
                          aria-describedby={chosen ? `${id}-chosen` : undefined}
                          tabIndex={
                            active ||
                            (!state.active && row.index === 0 && column === 0)
                              ? 0
                              : -1
                          }
                          data-slot="emoji-cell"
                          data-active={active ? "" : undefined}
                          data-chosen={chosen}
                          data-keyboard-active={active && state.keyboard}
                          className={cn(
                            emojiOptionClassName,
                            "data-[keyboard-active=true]:outline-ring scroll-mt-[var(--emoji-category-header-height)] scroll-mb-1 data-[keyboard-active=true]:outline-2"
                          )}
                          onPointerMove={() =>
                            dispatch({ position, type: "point" })
                          }
                          onFocus={() => dispatch({ position, type: "focus" })}
                          onClick={() =>
                            dispatch({ emoji: emoji.emoji, type: "select" })
                          }
                        >
                          <EmojiFrame selected={chosen}>
                            {emoji.emoji}
                          </EmojiFrame>
                        </button>
                      );
                    })}
                  </div>
                ))}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};
