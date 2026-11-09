import { cn } from "cn";
import { Check } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";

import {
  RowLayer,
  Segment,
  rowLine,
  useLinger,
  useRetained,
} from "@/components/download-row-parts";
import type { Completion, StatusKind } from "@/components/download-row-parts";
import { CircularProgress } from "@/components/ui/circular-progress";
import { ItemDescription } from "@/components/ui/item";
import { Progress } from "@/components/ui/progress";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useVideoTools } from "@/hooks/use-video-tools";
import type { Download } from "@/lib/model";
import { liveBytes } from "@/lib/model";
import { rowView } from "@/lib/row-view";
import type { BarView, StatusText } from "@/lib/row-view";

/** Shimmers only while Opul is setting video tools up; waiting on you stays still. */
const WaitingLabel = ({ label }: { label: string }) => {
  const { tools } = useVideoTools();
  const settingUp = tools?.video.state.kind === "setting_up";

  return (
    <span className={cn(settingUp && "shimmer shimmer-spread-32")}>
      {label}
    </span>
  );
};

/** `↑ speed · N peers`, each only while it has a value, or just "Seeding". */
const SeedingLabel = ({
  text,
}: {
  text: Extract<StatusText, { kind: "seeding" }>;
}) => {
  const upload = useLinger(text.upload);
  const peers = useLinger(text.peers);
  const idle = upload.value === null && peers.value === null;

  return (
    <span className="shimmer-up">
      {idle && "Seeding"}
      {upload.value !== null && (
        <Segment first phase={upload.phase}>
          <span className="sr-only">Uploading at </span>
          <span aria-hidden>↑ </span>
          {liveBytes(upload.value)}/s
        </Segment>
      )}
      {peers.value !== null && (
        <Segment
          first={upload.value === null}
          phase={peers.phase}
          // Narrow lists keep the speed; peers show alone when there's no speed.
          className={cn(upload.value !== null && "@max-[640px]:hidden")}
        >
          {peers.value} {peers.value === 1 ? "peer" : "peers"}
        </Segment>
      )}
    </span>
  );
};

const StatusLabel = ({ text }: { text: StatusText | null }) => {
  switch (text?.kind) {
    case "seeding": {
      return <SeedingLabel text={text} />;
    }

    case "waiting": {
      return <WaitingLabel label={text.label} />;
    }

    case "failed": {
      return <span className="text-destructive">Failed</span>;
    }

    case "label": {
      return text.text;
    }

    default: {
      return null;
    }
  }
};

const FULL = 100;

const sameBar = (a: BarView | null, b: BarView | null) =>
  a?.value === b?.value && a?.motion === b?.motion && a?.label === b?.label;

/** Keeps the last bar while its layer fades out, so it doesn't flip to empty. */
const useRetainedBar = (bar: BarView | null) => {
  const [retained, setRetained] = useState(bar);

  if (bar && !sameBar(bar, retained)) {
    setRetained(bar);
  }

  return bar ?? retained;
};

/**
 * Labels are sized to their text, so the slot reads the label's natural width
 * into a CSS variable that the slot's width transitions to (see styles.css).
 * Only changes are written, and transitions start after the first measure,
 * so rows don't animate in.
 */
const measureLabel = (slot: HTMLElement | null, label: HTMLElement | null) => {
  if (!slot || !label) {
    return;
  }

  // scrollWidth rounds, so one spare pixel keeps a fitting label from truncating.
  const width = `${label.scrollWidth + 1}px`;

  if (slot.style.getPropertyValue("--download-status-label") !== width) {
    slot.style.setProperty("--download-status-label", width);
  }
};

const useLabelWidth = () => {
  const slot = useRef<HTMLDivElement>(null);
  const label = useRef<HTMLParagraphElement>(null);

  const measure = () => measureLabel(slot.current, label.current);

  // Labels change only when the row renders, so each render measures again.
  useLayoutEffect(measure);

  // The first measure can run before the app's font loads.
  useEffect(() => {
    let current = true;
    const slotElement = slot.current;

    const ready = async () => {
      await document.fonts.ready;

      if (!current) {
        return;
      }

      measureLabel(slotElement, label.current);
      requestAnimationFrame(() => {
        if (current && slotElement) {
          slotElement.dataset.ready = "true";
        }
      });
    };

    ready().catch(() => null);

    return () => {
      current = false;
    };
  }, []);

  return { label, slot };
};

/**
 * Exactly one of the bar (or ring), the transient check, status text, or
 * nothing, stacked so they crossfade in one spot.
 */
export const DownloadRowStatus = ({
  download,
  completion,
  kind,
}: {
  download: Download;
  completion: Completion;
  kind: StatusKind;
}) => {
  const filling = completion.phase === "filling";

  const source =
    filling && completion.from
      ? { ...download, state: completion.from }
      : download;

  const bar = useRetainedBar(rowView(source).bar);
  const value = filling ? FULL : (bar?.value ?? null);
  const motion = bar?.motion ?? "still";
  const label = bar?.label ?? "";
  const textState = useRetained(kind === "text" ? download.state : null);

  const text = textState
    ? rowView({ ...download, state: textState }).statusText
    : null;

  const { label: labelRef, slot } = useLabelWidth();

  return (
    <div
      ref={slot}
      data-slot="download-status"
      className="grid min-w-0 grid-cols-[minmax(0,1fr)] items-center justify-items-end overflow-hidden"
    >
      <RowLayer active={kind === "progress"} className="flex">
        <Tooltip>
          <TooltipTrigger
            render={
              <span className="flex h-8 w-8 items-center justify-end @min-[520px]:w-28 @min-[640px]:w-48" />
            }
          >
            <Progress
              className="w-full @max-[520px]:hidden"
              value={value}
              motion={motion}
              aria-label={`${download.name} progress`}
              getAriaValueText={() => label}
            />
            <span className="flex size-8 items-center justify-center @min-[520px]:hidden">
              <CircularProgress
                value={value}
                size={20}
                thickness={3}
                motion={motion}
                aria-label={`${download.name} progress`}
                getAriaValueText={() => label}
              />
            </span>
          </TooltipTrigger>
          <TooltipContent>{label}</TooltipContent>
        </Tooltip>
      </RowLayer>
      <RowLayer
        active={kind === "check"}
        pop
        className="text-muted-foreground flex size-8 items-center justify-center"
      >
        <Check className="size-4" aria-hidden />
        <span className="sr-only">Download complete</span>
      </RowLayer>
      <RowLayer
        active={kind === "text"}
        className="max-w-(--download-status-text-max)"
      >
        <ItemDescription ref={labelRef} className={cn(rowLine, "text-right")}>
          <StatusLabel text={text} />
        </ItemDescription>
      </RowLayer>
    </div>
  );
};
