import { ArrowLeft } from "lucide-react";
import { useId, useEffect, useRef } from "react";
import type { Dispatch, SetStateAction } from "react";

import { Button } from "@/components/ui/button";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Slider } from "@/components/ui/slider";
import { EmojiArtwork } from "@/components/workspace-icon";
import type { IconDraft } from "@/components/workspace-icon-picker";

const controls = [
  { key: "hue", label: "Hue", max: 359, min: 0 },
  { key: "saturation", label: "Saturation", max: 1000, min: 0 },
  { key: "brightness", label: "Brightness", max: 150, min: 50 },
] as const;

export function WorkspaceCustomTreatment({
  emoji,
  treatment,
  onChange,
  onBack,
}: {
  emoji: string;
  treatment: Extract<IconDraft["appearance"]["preference"], { kind: "Custom" }>;
  onChange: Dispatch<SetStateAction<IconDraft>>;
  onBack: () => void;
}) {
  const id = useId();
  const container = useRef<HTMLDivElement>(null);
  useEffect(() => {
    // Edge-aligned thumbs become visible after Base UI measures the control.
    const frame = requestAnimationFrame(() => {
      container.current
        ?.querySelector<HTMLInputElement>('input[type="range"]')
        ?.focus({ preventScroll: true });
    });

    return () => cancelAnimationFrame(frame);
  }, []);

  return (
    <div
      ref={container}
      className="scroll-fade scroll-fade-2 col-start-1 row-start-1 flex min-h-0 flex-col gap-3 overflow-y-auto overscroll-contain p-1"
    >
      <div className="flex shrink-0 items-center gap-2">
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label="Back to emoji browsing"
          onClick={onBack}
        >
          <ArrowLeft aria-hidden data-icon="inline-start" />
        </Button>
        <span className="text-sm font-medium">Custom treatment</span>
      </div>
      <div className="flex min-h-8 flex-1 items-center justify-center py-1 text-5xl">
        <EmojiArtwork emoji={emoji} treatment={treatment} />
      </div>
      <FieldGroup className="shrink-0 gap-3 pb-2">
        {controls.map(({ key, label, min, max }) => (
          <Field key={key} className="gap-2">
            <FieldLabel id={`${id}-${key}`}>{label}</FieldLabel>
            <Slider
              aria-labelledby={`${id}-${key}`}
              min={min}
              max={max}
              step={1}
              value={[treatment[key]]}
              onValueChange={(values) => {
                const value = Array.isArray(values) ? values[0] : values;
                onChange((draft) => {
                  const { preference } = draft.appearance;

                  return preference.kind === "Custom"
                    ? {
                        ...draft,
                        appearance: {
                          ...draft.appearance,
                          preference: { ...preference, [key]: value },
                        },
                      }
                    : draft;
                });
              }}
            />
          </Field>
        ))}
      </FieldGroup>
    </div>
  );
}
