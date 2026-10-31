import { Toggle as TogglePrimitive } from "@base-ui/react/toggle";
import { useMemo, useRef } from "react";
import type { Dispatch, SetStateAction } from "react";
import { useEffectReducer } from "use-effect-reducer";
import type { EffectReducer } from "use-effect-reducer";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { EmojiFrame, emojiOptionClassName } from "@/components/ui/emoji-frame";
import { EmojiPicker } from "@/components/ui/emoji-picker";
import {
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverTitle,
} from "@/components/ui/popover";
import { Spinner } from "@/components/ui/spinner";
import { ToggleGroup } from "@/components/ui/toggle-group";
import { WorkspaceColorSuggestions } from "@/components/workspace-color-suggestions";
import { WorkspaceCustomTreatment } from "@/components/workspace-custom-treatment";
import { WorkspaceIcon } from "@/components/workspace-icon";
import { bridge, errorMessage } from "@/lib/bridge";
import { emojiDataWithRecents, skinToneChoices } from "@/lib/emoji-data";
import { useLibrary } from "@/lib/library";
import { workspaceEmoji } from "@/lib/model";
import type { Workspace } from "@/lib/model";
import { cn } from "@/lib/utils";
import { canTreatEmoji, emojiTreatment } from "@/lib/workspace-appearance";

export interface IconDraft {
  appearance: Workspace["appearance"];
  emoji: string;
}

type BrowsingEvent =
  | { type: "customize" }
  | { type: "back" }
  | { type: "change"; update: SetStateAction<IconDraft> };

type BrowsingEffect =
  | { type: "customize" }
  | { type: "focusCustom" }
  | { type: "change"; update: SetStateAction<IconDraft> };

const browsingReducer: EffectReducer<boolean, BrowsingEvent, BrowsingEffect> = (
  state,
  event,
  exec
) => {
  switch (event.type) {
    case "customize": {
      exec({ type: "customize" });

      return true;
    }

    case "back": {
      exec({ type: "focusCustom" });

      return false;
    }

    case "change": {
      exec(event);

      return state;
    }

    default: {
      throw new Error(`Unhandled browsing event: ${event satisfies never}`);
    }
  }
};

// The browsing surface is controlled so creation can retain it in its own draft.
export const WorkspaceIconPicker = ({
  workspaceId,
  draft,
  onChange,
}: {
  workspaceId: string;
  draft: IconDraft;
  onChange: Dispatch<SetStateAction<IconDraft>>;
}) => {
  const { emoji, appearance } = draft;
  const { state } = useLibrary();

  const resolveEmojiData = useMemo(
    () =>
      emojiDataWithRecents(
        state.kind === "ready" ? state.snapshot.recent_emoji : []
      ),
    [state]
  );

  const tones = useMemo(() => skinToneChoices(emoji), [emoji]);
  const customButton = useRef<HTMLButtonElement>(null);

  const [custom, dispatch] = useEffectReducer(browsingReducer, false, {
    change: (_state, event) => onChange(event.update),
    customize: () => {
      onChange((prevDraft) => {
        const current = emojiTreatment(prevDraft.emoji, prevDraft.appearance);

        return {
          ...prevDraft,
          appearance: {
            ...prevDraft.appearance,
            preference: {
              brightness: current?.brightness ?? 100,
              hue: current?.hue ?? 0,
              kind: "Custom",
              saturation: current?.saturation ?? 100,
            },
          },
        };
      });
    },
    focusCustom: () => customButton.current?.focus({ preventScroll: true }),
  });

  return (
    <>
      <div className="flex shrink-0 items-center gap-3">
        <div
          className="flex size-12 shrink-0 items-center justify-center text-3xl"
          // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role
          role="img"
          aria-label={`Icon preview: ${emoji}`}
        >
          <WorkspaceIcon
            workspace={{ appearance, icon: { kind: "Emoji", value: emoji } }}
          />
        </div>
        <div className="flex min-w-0 flex-col gap-1">
          <span className="text-sm font-medium">Selected icon</span>
          <output className="text-muted-foreground text-xs">
            {canTreatEmoji(emoji)
              ? "Changes are previewed until you save."
              : "This emoji uses its original colors."}
          </output>
        </div>
      </div>
      <div className="grid min-h-0 flex-1">
        <div
          inert={custom}
          className={cn(
            "col-start-1 row-start-1 flex min-h-0 flex-col gap-2",
            custom && "invisible"
          )}
        >
          {tones.length > 1 && (
            <ToggleGroup
              aria-label="Skin tone"
              size="sm"
              value={[emoji]}
              onValueChange={(values) => {
                const [selected] = values;

                if (selected) {
                  dispatch({
                    type: "change",
                    update: (prevDraft) => ({ ...prevDraft, emoji: selected }),
                  });
                }
              }}
            >
              {tones.map((tone) => (
                <TogglePrimitive
                  key={tone.emoji}
                  value={tone.emoji}
                  aria-label={tone.label}
                  title={tone.label}
                  className={emojiOptionClassName}
                >
                  <EmojiFrame selected={tone.emoji === emoji}>
                    {tone.emoji}
                  </EmojiFrame>
                </TogglePrimitive>
              ))}
            </ToggleGroup>
          )}
          <EmojiPicker
            chosenEmoji={emoji}
            resolveData={resolveEmojiData}
            onSelect={(selected) =>
              dispatch({
                type: "change",
                update: (prevDraft) => ({ ...prevDraft, emoji: selected }),
              })
            }
          />
          <WorkspaceColorSuggestions
            workspaceId={workspaceId}
            draft={draft}
            onChange={onChange}
            customButton={customButton}
            onCustomize={() => dispatch("customize")}
          />
        </div>
        {custom &&
          canTreatEmoji(emoji) &&
          appearance.preference.kind === "Custom" && (
            <WorkspaceCustomTreatment
              emoji={emoji}
              treatment={appearance.preference}
              onChange={onChange}
              onBack={() => dispatch("back")}
            />
          )}
      </div>
    </>
  );
};

interface EditorState {
  draft: IconDraft;
  status: "loading" | "ready" | "saving";
  error: string | null;
}

type EditorEvent =
  | { type: "change"; update: SetStateAction<IconDraft> }
  | { type: "loaded"; appearance: Workspace["appearance"] }
  | { type: "loadFailed"; error: string }
  | { type: "retry" }
  | { type: "save" }
  | { type: "saved" }
  | { type: "saveFailed" };

type EditorEffect = { type: "load" } | { type: "save" } | { type: "close" };

const editorReducer: EffectReducer<EditorState, EditorEvent, EditorEffect> = (
  state,
  event,
  exec
) => {
  switch (event.type) {
    case "change": {
      return {
        ...state,
        draft:
          "call" in event.update ? event.update(state.draft) : event.update,
      };
    }

    case "loaded": {
      return {
        ...state,
        draft: { ...state.draft, appearance: event.appearance },
        error: null,
        status: "ready",
      };
    }

    case "loadFailed": {
      return { ...state, error: event.error };
    }

    case "retry": {
      exec({ type: "load" });

      return { ...state, error: null };
    }

    case "save": {
      if (state.status !== "ready") {
        return state;
      }

      exec({ type: "save" });

      return { ...state, error: null, status: "saving" };
    }

    case "saved": {
      exec({ type: "close" });

      return { ...state, status: "ready" };
    }

    case "saveFailed": {
      return {
        ...state,
        error: "Couldn't save the icon. Your changes are kept here; try again.",
        status: "ready",
      };
    }

    default: {
      throw new Error(`Unhandled editor event: ${event satisfies never}`);
    }
  }
};

export const WorkspaceIconEditor = ({
  workspace,
  anchor,
  onClose,
}: {
  workspace: Workspace;
  anchor: HTMLButtonElement | null;
  onClose: () => void;
}) => {
  const { run } = useLibrary();

  const submission = useRef<{ input: string; requestId: string } | null>(null);

  const [state, dispatch] = useEffectReducer(
    editorReducer,
    (exec) => {
      if (!workspace.appearance.automatic_folder) {
        exec({ type: "load" });
      }

      return {
        draft: {
          appearance: workspace.appearance,
          emoji: workspaceEmoji(workspace),
        },
        error: null,
        status: workspace.appearance.automatic_folder ? "ready" : "loading",
      } satisfies EditorState;
    },
    {
      close: () => onClose(),
      load: async (_state, _effect, send, { signal }) => {
        try {
          const appearance = await bridge.workspaceAppearanceDraft(
            workspace.id
          );

          if (!signal.aborted) {
            send({ appearance, type: "loaded" });
          }
        } catch (error) {
          if (!signal.aborted) {
            send({ error: errorMessage(error), type: "loadFailed" });
          }
        }
      },
      save: async ({ draft }, _effect, send, { signal }) => {
        const input = JSON.stringify(draft);

        if (submission.current?.input !== input) {
          submission.current = { input, requestId: crypto.randomUUID() };
        }

        const receipt = await run({
          kind: "set_workspace_appearance",
          request: {
            appearance: draft.appearance,
            emoji: draft.emoji,
            request_id: submission.current.requestId,
            workspace_id: workspace.id,
          },
        });

        if (!signal.aborted) {
          send(receipt ? "saved" : "saveFailed");
        }
      },
    }
  );

  const { draft, error, status } = state;
  const ready = status !== "loading";
  const saving = status === "saving";

  return (
    <Popover
      open
      modal
      onOpenChange={(open) => {
        if (!open && !saving) {
          onClose();
        }
      }}
    >
      <PopoverContent
        anchor={anchor}
        align="start"
        finalFocus={anchor ? () => anchor : true}
        className="h-[min(34rem,var(--available-height))] max-h-[calc(100svh-1rem)] w-80 max-w-[calc(100vw-1rem)] gap-2 overflow-hidden p-3 motion-reduce:transition-none motion-reduce:data-closed:animate-none motion-reduce:data-open:animate-none"
      >
        <div className="flex shrink-0 flex-col gap-1">
          <PopoverTitle>Change icon</PopoverTitle>
          <PopoverDescription>{workspace.name}</PopoverDescription>
        </div>
        {ready ? (
          <WorkspaceIconPicker
            workspaceId={workspace.id}
            draft={draft}
            onChange={(update) => dispatch({ type: "change", update })}
          />
        ) : (
          <output className="text-muted-foreground flex-1 text-sm">
            Preparing icon…
          </output>
        )}
        {error && (
          <Alert role="alert">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
        <div className="flex shrink-0 justify-end gap-2">
          {!ready && error && (
            <Button variant="outline" onClick={() => dispatch("retry")}>
              Try again
            </Button>
          )}
          <Button variant="ghost" disabled={saving} onClick={onClose}>
            Cancel
          </Button>
          <Button disabled={!ready || saving} onClick={() => dispatch("save")}>
            {saving && (
              <Spinner
                className="motion-reduce:animate-none"
                data-icon="inline-start"
              />
            )}
            Save
          </Button>
        </div>
      </PopoverContent>
    </Popover>
  );
};
