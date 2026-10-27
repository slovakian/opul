import {
  useLayoutEffect,
  useRef,
  useState,
  type Dispatch,
  type RefObject,
  type SetStateAction,
} from "react";
import { Check } from "lucide-react";
import { Toggle as TogglePrimitive } from "@base-ui/react/toggle";
import { Button } from "@/components/ui/button";
import { EmojiFrame, emojiOptionClassName } from "@/components/ui/emoji-frame";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { EmojiArtwork } from "@/components/workspace-icon";
import type { IconDraft } from "@/components/workspace-icon-picker";
import { createColorSuggestions } from "@/lib/color-suggestions";
import { canTreatEmoji, isFolderEmoji } from "@/lib/workspace-appearance";

export function WorkspaceColorSuggestions({
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
    onChange((draft) => ({
      ...draft,
      appearance: { ...draft.appearance, preference },
    }));
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
            <TogglePrimitive
              key={treatment.preset_id}
              value={treatment.preset_id}
              ref={index === 6 ? firstRevealed : undefined}
              aria-label={`Preview ${treatment.preset_id} treatment`}
              title={`${treatment.preset_id} treatment`}
              className={emojiOptionClassName}
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
}
