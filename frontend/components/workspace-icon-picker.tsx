import { useEffect, useRef, useState } from "react";
import { EmojiPicker } from "frimousse";
import { Check, Loader2, Search } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
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
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { WorkspaceIcon } from "@/components/workspace-icon";
import { bridge, errorMessage } from "@/lib/bridge";
import { resolveEmojiData } from "@/lib/emoji-data";
import { useLibrary } from "@/lib/library";
import { workspaceEmoji, type Workspace } from "@/lib/model";
import { canTreatEmoji, isFolderEmoji } from "@/lib/workspace-appearance";

export type IconDraft = { emoji: string; appearance: Workspace["appearance"] };

// The browsing surface is controlled so creation can retain it in its own draft.
export function WorkspaceIconPicker({
  draft,
  onChange,
}: {
  draft: IconDraft;
  onChange: (draft: IconDraft) => void;
}) {
  const { emoji, appearance } = draft;
  const preference = appearance.preference;
  return (
    <>
      <div className="flex items-center gap-3">
        <div
          className="flex size-12 shrink-0 items-center justify-center text-3xl"
          role="img"
          aria-label={`Icon preview: ${emoji}`}
        >
          <WorkspaceIcon
            workspace={{ icon: { kind: "Emoji", value: emoji }, appearance }}
          />
        </div>
        <div className="flex min-w-0 flex-col gap-1">
          <span className="text-sm font-medium">Selected icon</span>
          <span className="text-xs text-muted-foreground" role="status">
            {canTreatEmoji(emoji)
              ? "Changes are previewed until you save."
              : "This emoji uses its original colors."}
          </span>
        </div>
      </div>
      <EmojiPicker.Root
        className="flex min-h-0 flex-1 flex-col gap-2"
        columns={8}
        locale="en"
        resolveEmojiData={resolveEmojiData}
        onEmojiSelect={({ emoji }) => onChange({ ...draft, emoji })}
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
          className="relative min-h-9 flex-1 overscroll-contain rounded-md outline-none focus-visible:ring-2 focus-visible:ring-ring"
          aria-label="Emoji categories"
        >
          <EmojiPicker.Loading className="p-4 text-sm text-muted-foreground">
            Loading emoji…
          </EmojiPicker.Loading>
          <EmojiPicker.Empty className="p-4 text-sm text-muted-foreground">
            No emoji found.
          </EmojiPicker.Empty>
          <EmojiPicker.List
            // Frimousse 0.4 emits zero-based ARIA indices; ARIA requires one-based indices.
            components={{
              CategoryHeader: ({ category, ...props }) => (
                <div
                  {...props}
                  className="bg-popover px-1 py-2 text-xs font-medium text-muted-foreground"
                >
                  {category.label}
                </div>
              ),
              Row: ({ children, ...props }) => (
                <div
                  {...props}
                  aria-rowindex={
                    props["aria-rowindex"] === undefined
                      ? undefined
                      : props["aria-rowindex"] + 1
                  }
                  className="scroll-my-1"
                >
                  {children}
                </div>
              ),
              Emoji: ({ emoji: item, ...props }) => (
                <button
                  {...props}
                  type="button"
                  aria-colindex={
                    props["aria-colindex"] === undefined
                      ? undefined
                      : props["aria-colindex"] + 1
                  }
                  aria-description={
                    item.emoji === emoji ? "Selected icon" : undefined
                  }
                  data-chosen={item.emoji === emoji}
                  className="relative flex size-9 items-center justify-center rounded-md text-2xl outline-none hover:bg-accent data-active:bg-accent data-[chosen=true]:ring-1 data-[chosen=true]:ring-ring focus-visible:ring-2 focus-visible:ring-ring"
                >
                  <span aria-hidden>{item.emoji}</span>
                  {item.emoji === emoji && (
                    <Check
                      aria-hidden
                      className="absolute bottom-0 right-0 size-3 rounded-sm bg-popover"
                    />
                  )}
                </button>
              ),
            }}
          />
        </EmojiPicker.Viewport>
      </EmojiPicker.Root>
      {canTreatEmoji(emoji) && (
        <ToggleGroup
          aria-label="Icon appearance"
          variant="outline"
          size="sm"
          value={[preference.kind]}
          onValueChange={(values) => {
            const kind = values[0];
            if (kind === "Original" || kind === "Automatic")
              onChange({
                ...draft,
                appearance: { ...appearance, preference: { kind } },
              });
          }}
        >
          <ToggleGroupItem value="Original">
            {preference.kind === "Original" && <Check />}Original
          </ToggleGroupItem>
          {isFolderEmoji(emoji) && (
            <ToggleGroupItem value="Automatic">
              {preference.kind === "Automatic" && <Check />}Automatic
            </ToggleGroupItem>
          )}
          {(preference.kind === "Preset" || preference.kind === "Custom") && (
            <ToggleGroupItem value={preference.kind}>
              <Check />
              Saved treatment
            </ToggleGroupItem>
          )}
        </ToggleGroup>
      )}
    </>
  );
}

export function WorkspaceIconEditor({
  workspace,
  anchor,
  onClose,
}: {
  workspace: Workspace;
  anchor: HTMLButtonElement | null;
  onClose: () => void;
}) {
  const { run } = useLibrary();
  const [draft, setDraft] = useState<IconDraft>({
    emoji: workspaceEmoji(workspace),
    appearance: workspace.appearance,
  });
  const [ready, setReady] = useState(!!workspace.appearance.automatic_folder);
  const [error, setError] = useState<string>();
  const [saving, setSaving] = useState(false);
  const [loadAttempt, setLoadAttempt] = useState(0);
  const submission = useRef<{ input: string; requestId: string } | null>(null);
  useEffect(() => {
    if (workspace.appearance.automatic_folder) return;
    let canceled = false;
    void bridge
      .workspaceAppearanceDraft(workspace.id)
      .then((appearance) => {
        if (!canceled) {
          setDraft((draft) => ({ ...draft, appearance }));
          setReady(true);
        }
      })
      .catch((error) => {
        if (!canceled) setError(errorMessage(error));
      });
    return () => {
      canceled = true;
    };
  }, [workspace.id, workspace.appearance.automatic_folder, loadAttempt]);

  async function save() {
    const input = JSON.stringify(draft);
    if (submission.current?.input !== input)
      submission.current = { input, requestId: crypto.randomUUID() };
    setSaving(true);
    setError(undefined);
    const receipt = await run({
      kind: "set_workspace_appearance",
      request: {
        request_id: submission.current.requestId,
        workspace_id: workspace.id,
        emoji: draft.emoji,
        appearance: draft.appearance,
      },
    });
    setSaving(false);
    if (receipt) onClose();
    else
      setError(
        "Couldn't save the icon. Your changes are kept here; try again.",
      );
  }

  return (
    <Popover
      open
      modal
      onOpenChange={(open) => {
        if (!open && !saving) onClose();
      }}
    >
      <PopoverContent
        anchor={anchor}
        align="start"
        finalFocus={anchor ? () => anchor : true}
        className="h-[min(30rem,var(--available-height))] max-h-[calc(100svh-1rem)] w-80 max-w-[calc(100vw-1rem)] gap-3 overflow-hidden motion-reduce:animate-none motion-reduce:transition-none"
      >
        <div className="flex shrink-0 flex-col gap-1">
          <PopoverTitle>Change icon</PopoverTitle>
          <PopoverDescription>{workspace.name}</PopoverDescription>
        </div>
        {ready ? (
          <WorkspaceIconPicker draft={draft} onChange={setDraft} />
        ) : (
          <div role="status" className="flex-1 text-sm text-muted-foreground">
            Preparing icon…
          </div>
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
          <Button disabled={!ready || saving} onClick={() => void save()}>
            {saving && (
              <Loader2
                className="animate-spin motion-reduce:animate-none"
                data-icon="inline-start"
              />
            )}
            Save
          </Button>
        </div>
      </PopoverContent>
    </Popover>
  );
}
