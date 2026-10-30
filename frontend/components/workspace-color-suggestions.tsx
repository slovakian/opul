import { Toggle as TogglePrimitive } from "@base-ui/react/toggle";
import { Check } from "lucide-react";
import { useLayoutEffect, useRef, useState } from "react";
import type { Dispatch, RefObject, SetStateAction } from "react";

import { Button } from "@/components/ui/button";
import { EmojiFrame, emojiOptionClassName } from "@/components/ui/emoji-frame";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { EmojiArtwork } from "@/components/workspace-icon";
import type { IconDraft } from "@/components/workspace-icon-picker";
import { createColorSuggestions } from "@/lib/color-suggestions";
import { cn } from "@/lib/utils";
import { canTreatEmoji, isFolderEmoji } from "@/lib/workspace-appearance";

export const WorkspaceColorSuggestions = ({
  workspaceId,
  draft,
  onChange,
  onCustomize,
  customButton,
}: {
  workspaceId: string;
  draft: IconDraft;
  onChange: Dispatch<SetStateAction<IconDraft>>;
  onCustomize: () => void;
  customButton: RefObject<HTMLButtonElement | null>;
}) => {
  // oxlint-disable-next-line react/hook-use-state
  const [{ folder, other }] = useState(() =>
    createColorSuggestions({ appearance: draft.appearance, workspaceId })
  );

  // oxlint-disable-next-line react/hook-use-state
  const [savedPreference] = useState(draft.appearance.preference);

  const [expanded, setExpanded] = useState(false);
  const firstRevealed = useRef<HTMLButtonElement>(null);
  const restoreFocus = useRef(false);

  useLayoutEffect(() => {
    if (expanded && restoreFocus.current) {
      firstRevealed.current?.focus({ preventScroll: true });
      restoreFocus.current = false;
    }
  }, [expanded]);

  const { emoji, appearance } = draft;
  const { preference } = appearance;
  const folderEmoji = isFolderEmoji(emoji);
  const choices = folderEmoji ? folder : other;
  const visible = expanded ? choices : choices.slice(0, 6);
  const remaining = choices.length - visible.length;

  const select = (nextPreference: typeof appearance.preference) => {
    onChange((currentDraft) => ({
      ...currentDraft,
      appearance: { ...currentDraft.appearance, preference: nextPreference },
    }));
  };

  if (!canTreatEmoji(emoji)) {
    return null;
  }

  return (
    <div
      className="scroll-fade scroll-fade-2 flex h-44 min-h-20 shrink flex-col gap-2 overflow-y-auto overscroll-contain p-1"
      // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role
      role="group"
      aria-label="Icon appearance"
    >
      <div className="-mb-1 flex h-6 shrink-0 items-center justify-between gap-2">
        <span className="text-muted-foreground text-xs font-medium">
          Color suggestions
        </span>
        {remaining > 0 && (
          <Button
            type="button"
            variant="ghost"
            size="xs"
            aria-label={`Show ${remaining} more color suggestions`}
            onClick={(event) => {
              restoreFocus.current =
                event.currentTarget === document.activeElement;
              setExpanded(true);
            }}
          >
            +{remaining}
          </Button>
        )}
      </div>
      <div className="shrink-0">
        <ToggleGroup
          aria-label="Color suggestions"
          size="sm"
          className="grid w-full grid-cols-6 gap-2"
          value={
            preference.kind === "Preset" ? [preference.treatment.preset_id] : []
          }
          onValueChange={(values) => {
            const [presetId] = values;

            const treatment = choices.find(
              (item) => item.preset_id === presetId
            );

            if (treatment) {
              select({ kind: "Preset", treatment });
            }
          }}
        >
          {visible.map((treatment, index) => {
            const label = folderEmoji
              ? `${treatment.preset_id.charAt(0).toUpperCase()}${treatment.preset_id.slice(1)} Treatment`
              : `color suggestion ${index + 1}`;

            return (
              <TogglePrimitive
                key={treatment.preset_id}
                value={treatment.preset_id}
                ref={index === 6 ? firstRevealed : undefined}
                aria-label={`Preview ${label}`}
                title={folderEmoji ? label : undefined}
                className={cn(
                  emojiOptionClassName,
                  "aspect-square size-auto w-full"
                )}
              >
                <EmojiFrame
                  selected={
                    preference.kind === "Preset" &&
                    preference.treatment.preset_id === treatment.preset_id
                  }
                >
                  <EmojiArtwork emoji={emoji} treatment={treatment} />
                </EmojiFrame>
              </TogglePrimitive>
            );
          })}
        </ToggleGroup>
      </div>
      <ToggleGroup
        aria-label="Appearance options"
        variant="outline"
        size="sm"
        className="w-full shrink-0 flex-wrap justify-start"
        value={[preference.kind]}
        onValueChange={(values) => {
          const [kind] = values;

          if (kind === "Original" || kind === "Automatic") {
            select({ kind });
          }
        }}
      >
        <ToggleGroupItem value="Original">
          {preference.kind === "Original" && <Check aria-hidden />}Original
        </ToggleGroupItem>
        {folderEmoji && (
          <ToggleGroupItem value="Automatic">
            {preference.kind === "Automatic" && <Check aria-hidden />}Automatic
          </ToggleGroupItem>
        )}
        <ToggleGroupItem
          value="Custom"
          ref={customButton}
          onClick={onCustomize}
        >
          {preference.kind === "Custom" && <Check aria-hidden />}Custom
        </ToggleGroupItem>
      </ToggleGroup>
      {savedPreference.kind === "Custom" && (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="self-start"
          onClick={() => select(savedPreference)}
        >
          <EmojiArtwork emoji={emoji} treatment={savedPreference} />
          Saved custom treatment
        </Button>
      )}
    </div>
  );
};
