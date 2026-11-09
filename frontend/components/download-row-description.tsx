import { cn } from "cn";

import {
  RowLayer,
  Segment,
  rowLine,
  sizeLabel,
  useLinger,
  useRetained,
} from "@/components/download-row-parts";
import type { Completion } from "@/components/download-row-parts";
import { ItemDescription } from "@/components/ui/item";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { Download } from "@/lib/model";
import {
  compactTimeRemaining,
  liveBytes,
  sourceHost,
  timeRemaining,
} from "@/lib/model";
import { rowView } from "@/lib/row-view";
import type { DescriptionView } from "@/lib/row-view";

/**
 * Amount first, then either speed and time left or a word like "Paused".
 * Speed and time left appear only while they have a value. Speed drops first
 * as the list narrows, then the downloaded amount; time left stays longest.
 */
const LiveLine = ({ view }: { view: DescriptionView }) => {
  const { amount, note } = view;
  const speed = useLinger(note ? null : view.speed);
  const eta = useLinger(note ? null : view.eta);

  return (
    <>
      <Segment first>
        {amount.total ? (
          <>
            <span className="@max-[440px]:hidden">{amount.done} of </span>
            {amount.total}
          </>
        ) : (
          amount.done
        )}
      </Segment>
      {!note && speed.value !== null && (
        <Segment
          data-segment="speed"
          phase={speed.phase}
          className="@max-[640px]:hidden"
        >
          {liveBytes(speed.value)}/s
        </Segment>
      )}
      {!note && eta.value !== null && (
        <Segment data-segment="eta" phase={eta.phase}>
          <Tooltip>
            <TooltipTrigger render={<span />}>
              <span aria-hidden>{compactTimeRemaining(eta.value)}</span>
              <span className="sr-only">{timeRemaining(eta.value)}</span>
            </TooltipTrigger>
            <TooltipContent>{timeRemaining(eta.value)}</TooltipContent>
          </Tooltip>
        </Segment>
      )}
      {note && <Segment data-segment="note">{note}</Segment>}
    </>
  );
};

/**
 * The second line: the amount while transferring or held, a short message
 * while processing or failed, and otherwise where it came from and its size.
 */
export const DownloadRowDescription = ({
  download,
  completion,
}: {
  download: Download;
  completion: Completion;
}) => {
  // While the bar fills to 100%, the line keeps what it showed before.
  const shown = completion.from
    ? { ...download, state: completion.from }
    : download;

  const finishing = completion.phase === "check";
  const view = rowView(shown).description;
  const line = finishing ? "origin" : view.line;
  const current = line === "message" ? view.message : null;
  const text = useRetained(current?.text ?? null);
  const tone = useRetained(current?.tone ?? null);

  const messageText = (
    <span
      className={cn("block truncate", tone === "error" && "text-destructive")}
    >
      <span className={cn(tone === "shimmer" && "shimmer shimmer-spread-32")}>
        {text}
      </span>
    </span>
  );

  return (
    <ItemDescription
      data-line={line}
      className={cn(rowLine, "grid grid-cols-[minmax(0,1fr)]")}
    >
      <RowLayer active={line === "live"} className="truncate">
        <LiveLine view={view} />
      </RowLayer>
      <RowLayer active={line === "message"}>
        {tone === "error" && text ? (
          <Tooltip>
            <TooltipTrigger render={messageText} />
            <TooltipContent className="max-w-sm">{text}</TooltipContent>
          </Tooltip>
        ) : (
          messageText
        )}
      </RowLayer>
      <RowLayer active={line === "origin"} className="truncate">
        <Segment first>{sourceHost(download)}</Segment>
        <Segment>{sizeLabel(download)}</Segment>
      </RowLayer>
    </ItemDescription>
  );
};
