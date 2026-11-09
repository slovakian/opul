import { cn } from "cn";
import type { ReactNode } from "react";

import {
  RowLayer,
  Segment,
  rowLine,
  sizeLabel,
  useLinger,
  useRetained,
} from "@/components/download-row-parts";
import type { Completion, LingerPhase } from "@/components/download-row-parts";
import {
  LabelText,
  UploadSpeed,
  WaitingLabel,
} from "@/components/download-row-status";
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
import type { DescriptionView, StatusText } from "@/lib/row-view";

/** Only below 440px, where the status slot hands its text to this line. */
const narrowOnly = "hidden @max-[440px]:inline";

/**
 * The status slot's text, for the narrowest lists: Queued, Cancelled, Failed,
 * the waiting labels and the torrent hints. Seeding shows only its upload
 * speed, without the word.
 */
const NarrowStatus = ({
  text,
  upload,
}: {
  text: StatusText | null;
  upload: { value: number | null; phase: LingerPhase };
}) => {
  switch (text?.kind) {
    case "seeding": {
      return upload.value === null ? null : (
        <Segment className={narrowOnly} phase={upload.phase}>
          <UploadSpeed value={upload.value} />
        </Segment>
      );
    }

    case "waiting": {
      return (
        <Segment className={narrowOnly}>
          <WaitingLabel label={text.label} />
        </Segment>
      );
    }

    case "failed": {
      return (
        <Segment className={cn(narrowOnly, "text-destructive")}>Failed</Segment>
      );
    }

    case "label": {
      return (
        <Segment className={narrowOnly}>
          <LabelText text={text} />
        </Segment>
      );
    }

    default: {
      return null;
    }
  }
};

/**
 * Amount first, then either speed and time left or a word like "Paused".
 * Speed and time left appear only while they have a value. Speed drops first
 * as the list narrows, then the downloaded amount; time left stays longest.
 */
const LiveLine = ({
  view,
  children,
}: {
  view: DescriptionView;
  children: ReactNode;
}) => {
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
      {children}
    </>
  );
};

/** A short message, or the error in red with its full text in a tooltip. */
const MessageLine = ({
  message,
  failed,
}: {
  message: DescriptionView["message"];
  failed: boolean;
}) => {
  const text = useRetained(message?.text ?? null);
  const tone = useRetained(message?.tone ?? null);
  const error = tone === "error";

  const messageText = (
    <span className={cn("block truncate", error && "text-destructive")}>
      {error && failed && <span className={narrowOnly}>Failed · </span>}
      <span className={cn(tone === "shimmer" && "shimmer shimmer-spread-32")}>
        {text}
      </span>
    </span>
  );

  if (!error || !text) {
    return messageText;
  }

  return (
    <Tooltip>
      <TooltipTrigger render={messageText} />
      <TooltipContent className="max-w-sm">{text}</TooltipContent>
    </Tooltip>
  );
};

/**
 * Where it came from and its size. Below 440px an uploading seed reads
 * `5.7 GB · ↑ 820 KB/s`, without its source.
 */
const OriginLine = ({
  download,
  size,
  uploading,
  children,
}: {
  download: Download;
  size: boolean;
  uploading: boolean;
  children: ReactNode;
}) => (
  <>
    <Segment first className={cn(uploading && "@max-[440px]:hidden")}>
      {sourceHost(download)}
    </Segment>
    {size && (
      <Segment
        className={cn(
          uploading && "@max-[440px]:[&>[data-slot=download-separator]]:hidden"
        )}
      >
        {sizeLabel(download)}
      </Segment>
    )}
    {children}
  </>
);

/**
 * The second line: the amount while transferring or held, a short message
 * while processing or failed, and otherwise where it came from and its size.
 */
export const DownloadRowDescription = ({
  download,
  completion,
  lookingForPeers,
}: {
  download: Download;
  completion: Completion;
  lookingForPeers: boolean;
}) => {
  // While the bar fills to 100%, the line keeps what it showed before.
  const shown = completion.from
    ? { ...download, state: completion.from }
    : download;

  const finishing = completion.phase === "check";
  const view = rowView(shown, { lookingForPeers }).description;
  const seeding = view.status?.kind === "seeding" ? view.status : null;
  const upload = useLinger(seeding?.upload ?? null);
  const line = finishing ? "origin" : view.line;

  const narrow = finishing ? null : (
    <NarrowStatus text={view.status} upload={upload} />
  );

  return (
    <ItemDescription
      data-line={line}
      className={cn(rowLine, "grid grid-cols-[minmax(0,1fr)]")}
    >
      <RowLayer active={line === "live"} className="truncate">
        <LiveLine view={view}>{narrow}</LiveLine>
      </RowLayer>
      <RowLayer active={line === "message"}>
        <MessageLine
          message={line === "message" ? view.message : null}
          failed={view.status?.kind === "failed"}
        />
      </RowLayer>
      <RowLayer active={line === "origin"} className="truncate">
        <OriginLine
          download={download}
          size={view.size}
          uploading={upload.value !== null}
        >
          {narrow}
        </OriginLine>
      </RowLayer>
    </ItemDescription>
  );
};
