import { Toggle as TogglePrimitive } from "@base-ui/react/toggle";
import { EmojiPicker } from "frimousse";
import type {
  EmojiPickerListCategoryHeaderProps,
  EmojiPickerListComponents,
  EmojiPickerListEmojiProps,
  EmojiPickerListRowProps,
} from "frimousse";
import { Search } from "lucide-react";
import {
  createContext,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { Dispatch, RefObject, SetStateAction } from "react";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { EmojiFrame, emojiOptionClassName } from "@/components/ui/emoji-frame";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from "@/components/ui/input-group";
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

interface EmojiPickerContextValue {
  chosenEmoji: string;
  keyboardNavigation: RefObject<boolean>;
}

const EmojiPickerContext = createContext<EmojiPickerContextValue>({
  chosenEmoji: "",
  keyboardNavigation: { current: false },
});

const EmojiCategoryHeader = ({
  category,
  ...headerProps
}: EmojiPickerListCategoryHeaderProps) => (
  <div
    {...headerProps}
    // Fade through the sticky header's bottom padding at the same depth as the viewport's bottom fade.
    className="from-popover text-muted-foreground bg-linear-to-b from-[calc(100%_-_var(--scroll-fade-size))] to-transparent px-1 py-2 text-xs font-medium"
  >
    {category.label.replaceAll(/\b[a-z]/gu, (letter) => letter.toUpperCase())}
  </div>
);

const EmojiRow = ({ children, ...rowProps }: EmojiPickerListRowProps) => (
  <div
    {...rowProps}
    aria-rowindex={
      rowProps["aria-rowindex"] === undefined
        ? undefined
        : rowProps["aria-rowindex"] + 1
    }
    className="scroll-my-1"
  >
    {children}
  </div>
);

const EmojiGridCell = (props: EmojiPickerListEmojiProps) => {
  const { chosenEmoji, keyboardNavigation } = useContext(EmojiPickerContext);
  const { emoji } = props;
  const chosen = emoji.emoji === chosenEmoji;
  const ref = useRef<HTMLButtonElement>(null);

  useLayoutEffect(() => {
    if (!emoji.isActive || !keyboardNavigation.current) {
      return;
    }

    const cell = ref.current;
    const viewport = cell?.closest<HTMLElement>("[frimousse-viewport]");

    if (!cell || !viewport) {
      return;
    }
    // Frimousse 0.4 also uses its zero-based ARIA row index as a scroll target.
    // After correcting indices, keep the actual active cell visible, without
    // scrolling on hover or depending on the library's private row numbering.

    const bounds = cell.getBoundingClientRect();
    const frame = viewport.getBoundingClientRect();

    const header = viewport.querySelector<HTMLElement>(
      "[frimousse-category-header]"
    );

    const top =
      frame.top +
      Math.max(
        0,
        Math.min(header?.offsetHeight ?? 0, frame.height - bounds.height)
      );

    let delta = 0;

    if (bounds.bottom > frame.bottom) {
      delta = bounds.bottom - frame.bottom;
    } else if (bounds.top < top) {
      delta = bounds.top - top;
    }

    if (delta) {
      viewport.scrollBy({ behavior: "instant", top: delta });
    }
  }, [emoji.isActive, keyboardNavigation]);

  return (
    <button
      {...props}
      ref={ref}
      type="button"
      // oxlint-disable-next-line jsx-a11y/role-supports-aria-props
      aria-colindex={
        props["aria-colindex"] === undefined
          ? undefined
          : props["aria-colindex"] + 1
      }
      // oxlint-disable-next-line jsx-a11y/role-supports-aria-props
      aria-description={chosen ? "Selected icon" : undefined}
      data-chosen={chosen}
      data-keyboard-active={emoji.isActive && keyboardNavigation.current}
      className={cn(
        emojiOptionClassName,
        "data-[keyboard-active=true]:outline-ring data-[keyboard-active=true]:outline-2"
      )}
    >
      <EmojiFrame selected={chosen}>{emoji.emoji}</EmojiFrame>
    </button>
  );
};

const listComponents: EmojiPickerListComponents = {
  CategoryHeader: EmojiCategoryHeader,
  Emoji: EmojiGridCell,
  Row: EmojiRow,
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
  const keyboardNavigation = useRef(false);
  const [custom, setCustom] = useState(false);
  const customButton = useRef<HTMLButtonElement>(null);
  const restoreFocus = useRef(false);

  useLayoutEffect(() => {
    if (!custom && restoreFocus.current) {
      customButton.current?.focus({ preventScroll: true });
      restoreFocus.current = false;
    }
  }, [custom]);

  const pickerContextValue = useMemo(
    () => ({ chosenEmoji: emoji, keyboardNavigation }),
    [emoji]
  );

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
                  onChange((prevDraft) => ({ ...prevDraft, emoji: selected }));
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
          <EmojiPicker.Root
            className="flex min-h-[7.25rem] flex-1 flex-col gap-2"
            columns={8}
            locale="en"
            resolveEmojiData={resolveEmojiData}
            onKeyDownCapture={(event) => {
              if (event.key.startsWith("Arrow")) {
                keyboardNavigation.current = true;
              }
            }}
            onPointerMoveCapture={() => {
              keyboardNavigation.current = false;
            }}
            onEmojiSelect={({ emoji: selectedEmoji }) =>
              onChange((prevDraft) => ({ ...prevDraft, emoji: selectedEmoji }))
            }
          >
            <InputGroup>
              <InputGroupInput
                render={
                  <EmojiPicker.Search
                    autoFocus
                    aria-label="Search emoji"
                    placeholder="Search emoji…"
                  />
                }
              />
              <InputGroupAddon>
                <Search />
              </InputGroupAddon>
            </InputGroup>
            <EmojiPicker.Viewport
              className="scroll-fade-b scroll-fade-2 focus-visible:ring-ring relative min-h-[4.5rem] flex-1 overscroll-contain rounded-md outline-none focus-visible:ring-2"
              aria-label="Emoji categories"
            >
              <EmojiPicker.Loading className="text-muted-foreground p-4 text-sm">
                Loading emoji…
              </EmojiPicker.Loading>
              <EmojiPicker.Empty className="text-muted-foreground p-4 text-sm">
                No emoji found.
              </EmojiPicker.Empty>
              <EmojiPickerContext.Provider value={pickerContextValue}>
                <EmojiPicker.List
                  // Frimousse 0.4 emits zero-based ARIA indices; ARIA requires one-based indices.
                  components={listComponents}
                />
              </EmojiPickerContext.Provider>
            </EmojiPicker.Viewport>
          </EmojiPicker.Root>
          <WorkspaceColorSuggestions
            workspaceId={workspaceId}
            draft={draft}
            onChange={onChange}
            customButton={customButton}
            onCustomize={() => {
              onChange((prevDraft) => {
                const current = emojiTreatment(
                  prevDraft.emoji,
                  prevDraft.appearance
                );

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
              setCustom(true);
            }}
          />
        </div>
        {custom &&
          canTreatEmoji(emoji) &&
          appearance.preference.kind === "Custom" && (
            <WorkspaceCustomTreatment
              emoji={emoji}
              treatment={appearance.preference}
              onChange={onChange}
              onBack={() => {
                restoreFocus.current = true;
                setCustom(false);
              }}
            />
          )}
      </div>
    </>
  );
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

  const [draft, setDraft] = useState<IconDraft>({
    appearance: workspace.appearance,
    emoji: workspaceEmoji(workspace),
  });

  const [ready, setReady] = useState(!!workspace.appearance.automatic_folder);
  const [error, setError] = useState<string>();
  const [saving, setSaving] = useState(false);
  const [loadAttempt, setLoadAttempt] = useState(0);
  const submission = useRef<{ input: string; requestId: string } | null>(null);

  useEffect(() => {
    if (workspace.appearance.automatic_folder || loadAttempt < 0) {
      return;
    }

    let canceled = false;

    const loadAppearance = async () => {
      try {
        const appearance = await bridge.workspaceAppearanceDraft(workspace.id);

        if (!canceled) {
          setDraft((prevDraft) => ({ ...prevDraft, appearance }));
          setReady(true);
        }
      } catch (error_) {
        if (!canceled) {
          setError(errorMessage(error_));
        }
      }
    };

    loadAppearance().catch(() => null);

    return () => {
      canceled = true;
    };
  }, [workspace.id, workspace.appearance.automatic_folder, loadAttempt]);

  const save = async () => {
    const input = JSON.stringify(draft);

    if (submission.current?.input !== input) {
      submission.current = { input, requestId: crypto.randomUUID() };
    }

    setSaving(true);
    setError(undefined);

    const receipt = await run({
      kind: "set_workspace_appearance",
      request: {
        appearance: draft.appearance,
        emoji: draft.emoji,
        request_id: submission.current.requestId,
        workspace_id: workspace.id,
      },
    });

    setSaving(false);

    if (receipt) {
      onClose();
    } else {
      setError(
        "Couldn't save the icon. Your changes are kept here; try again."
      );
    }
  };

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
            onChange={setDraft}
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
            <Button
              variant="outline"
              onClick={() => {
                setError(undefined);
                setLoadAttempt((value) => value + 1);
              }}
            >
              Try again
            </Button>
          )}
          <Button variant="ghost" disabled={saving} onClick={onClose}>
            Cancel
          </Button>
          <Button
            disabled={!ready || saving}
            onClick={() => {
              save().catch(() => null);
            }}
          >
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
