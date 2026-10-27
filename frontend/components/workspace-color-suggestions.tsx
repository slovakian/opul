import { useLayoutEffect, useRef, useState } from "react";
import { Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { EmojiArtwork } from "@/components/workspace-icon";
import type { IconDraft } from "@/components/workspace-icon-picker";
import { createColorSuggestions } from "@/lib/color-suggestions";
import {
  canTreatEmoji,
  emojiTreatment,
  isFolderEmoji,
} from "@/lib/workspace-appearance";

export function WorkspaceColorSuggestions({
  workspaceId,
  draft,
  onChange,
}: {
  workspaceId: string;
  draft: IconDraft;
  onChange: (draft: IconDraft) => void;
}) {
  const [{ folder, other }] = useState(() =>
    createColorSuggestions({ workspaceId, appearance: draft.appearance }),
  );
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
  const preference = appearance.preference;
  const choices = isFolderEmoji(emoji) ? folder : other;
  const visible = expanded ? choices : choices.slice(0, 6);
  const remaining = choices.length - visible.length;
  function select(preference: typeof appearance.preference) {
    onChange({ ...draft, appearance: { ...appearance, preference } });
  }
  if (!canTreatEmoji(emoji)) return null;
  return (
    <div
      className="flex h-44 min-h-20 shrink flex-col gap-2 overflow-y-auto overscroll-contain scroll-fade scroll-fade-2 p-1"
      role="group"
      aria-label="Icon appearance"
    >
      <div className="flex h-8 shrink-0 items-center justify-between gap-2">
        <span className="text-xs text-muted-foreground">Color suggestions</span>
        {remaining > 0 && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
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
          variant="outline"
          size="sm"
          className="grid w-full grid-cols-6 gap-1"
          value={
            preference.kind === "Preset" ? [preference.treatment.preset_id] : []
          }
          onValueChange={(values) => {
            const treatment = choices.find(
              (item) => item.preset_id === values[0],
            );
            if (treatment) select({ kind: "Preset", treatment });
          }}
        >
          {visible.map((treatment, index) => (
            <ToggleGroupItem
              key={treatment.preset_id}
              value={treatment.preset_id}
              ref={index === 6 ? firstRevealed : undefined}
              aria-label={`Preview ${treatment.preset_id} treatment`}
              title={`${treatment.preset_id} treatment`}
              className="relative size-9 p-0"
            >
              <span className="text-2xl">
                <EmojiArtwork emoji={emoji} treatment={treatment} />
              </span>
              {preference.kind === "Preset" &&
                preference.treatment.preset_id === treatment.preset_id && (
                  <Check
                    aria-hidden
                    className="absolute bottom-0 right-0 rounded-sm bg-popover"
                  />
                )}
            </ToggleGroupItem>
          ))}
        </ToggleGroup>
      </div>
      <ToggleGroup
        aria-label="Appearance options"
        variant="outline"
        size="sm"
        className="w-full shrink-0 flex-wrap justify-start"
        value={[preference.kind]}
        onValueChange={(values) => {
          const kind = values[0];
          if (kind === "Original" || kind === "Automatic") select({ kind });
          if (kind === "Custom") {
            const current = emojiTreatment(emoji, appearance);
            select({
              kind: "Custom",
              hue: current?.hue ?? 0,
              saturation: current?.saturation ?? 100,
              brightness: current?.brightness ?? 100,
            });
          }
        }}
      >
        <ToggleGroupItem value="Original">
          {preference.kind === "Original" && <Check aria-hidden />}Original
        </ToggleGroupItem>
        {isFolderEmoji(emoji) && (
          <ToggleGroupItem value="Automatic">
            {preference.kind === "Automatic" && <Check aria-hidden />}Automatic
          </ToggleGroupItem>
        )}
        <ToggleGroupItem value="Custom">
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
}
