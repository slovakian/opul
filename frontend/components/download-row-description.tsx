import { cn } from "cn";
import { Fragment } from "react";

import {
  LiveValue,
  RowLayer,
  Separator,
  rowLine,
  sizeLabel,
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
  stateNames,
  timeRemaining,
} from "@/lib/model";

/**
 * Downloaded of total · speed · time left. Speed drops first as the list
 * narrows, then the downloaded amount; time left stays the longest.
 */
const LiveLine = ({ download }: { download: Download }) => {
  const total = download.total_size;
  const eta = timeRemaining(download.eta_seconds);

  return (
    <>
      <span className={cn(total > 0 && "@max-[440px]:hidden")}>
        <LiveValue className="min-w-[6.6ch]">
          {liveBytes(download.bytes_downloaded)}
        </LiveValue>
        {total > 0 && " of "}
      </span>
      {total > 0 && liveBytes(total)}
      <span className="@max-[640px]:hidden">
        <Separator />
        <LiveValue className="min-w-[8ch] text-right">
          {liveBytes(download.download_rate)}/s
        </LiveValue>
      </span>
      <Separator />
      <Tooltip>
        <TooltipTrigger render={<LiveValue className="min-w-[7ch]" />}>
          <span aria-hidden>{compactTimeRemaining(download.eta_seconds)}</span>
          <span className="sr-only">{eta}</span>
        </TooltipTrigger>
        <TooltipContent>{eta}</TooltipContent>
      </Tooltip>
    </>
  );
};

const SEGMENT_BREAK = " · ";

/** Spaces a message's segments like every other row line. */
const withSeparators = (text: string | null) =>
  text?.split(SEGMENT_BREAK).map((segment, index) => (
    // Segments repeat ("50.0 MB of 50.0 MB"), so their position is the key.
    // oxlint-disable-next-line react/no-array-index-key
    <Fragment key={index}>
      {index > 0 && <Separator />}
      {segment}
    </Fragment>
  ));

/** A sentence in place of the live numbers, or null when the summary fits. */
const descriptionMessage = (download: Download) => {
  switch (download.state) {
    case "Processing": {
      return "Processing…";
    }

    case "Verifying": {
      return "Verifying…";
    }

    case "Pausing":
    case "Paused": {
      const done = liveBytes(download.bytes_downloaded);

      const amount = download.total_size
        ? `${done} of ${liveBytes(download.total_size)}`
        : done;

      return `${stateNames[download.state]}${SEGMENT_BREAK}${amount}`;
    }

    case "Failed": {
      return download.error_message;
    }

    default: {
      return null;
    }
  }
};

/**
 * The second line: live numbers while fetching, a short message while
 * processing, paused or failed, and otherwise where it came from and its size.
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
  const live = !finishing && shown.state === "Fetching";
  const currentMessage = finishing ? null : descriptionMessage(shown);
  const message = useRetained(currentMessage);
  const failed = useRetained(currentMessage ? shown.state === "Failed" : null);

  const messageText = (
    <span className={cn("block truncate", failed && "text-destructive")}>
      {failed ? message : withSeparators(message)}
    </span>
  );

  return (
    <ItemDescription className={cn(rowLine, "grid grid-cols-[minmax(0,1fr)]")}>
      <RowLayer active={live} className="truncate">
        <LiveLine download={shown} />
      </RowLayer>
      <RowLayer active={currentMessage !== null}>
        {failed && message ? (
          <Tooltip>
            <TooltipTrigger render={messageText} />
            <TooltipContent className="max-w-sm">{message}</TooltipContent>
          </Tooltip>
        ) : (
          messageText
        )}
      </RowLayer>
      <RowLayer active={!live && currentMessage === null} className="truncate">
        {sourceHost(download)}
        <Separator />
        <span className="tabular-nums">{sizeLabel(download)}</span>
      </RowLayer>
    </ItemDescription>
  );
};
