import {
  Archive,
  ArrowDown,
  Check,
  File,
  FileAudio,
  FileCode,
  FileImage,
  FileText,
  Film,
  HardDriveDownload,
  Pause,
  CircleAlert,
} from "lucide-react";

import { DownloadActions } from "@/components/download-actions";
import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemGroup,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item";
import { Progress } from "@/components/ui/progress";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { bytes, sourceHost, stateNames, timeRemaining } from "@/lib/model";
import type { Download, Workspace } from "@/lib/model";
import { cn } from "@/lib/utils";

export function fileIcon(download: Download) {
  const extension = download.name.split(".").pop()?.toLowerCase();

  if (extension && ["zip", "rar", "7z", "gz", "tar"].includes(extension)) {
    return Archive;
  }

  if (
    (extension && ["mp4", "mkv", "mov", "webm"].includes(extension)) ||
    download.source === "VideoPage"
  ) {
    return Film;
  }

  if (extension && ["mp3", "m4a", "wav", "flac"].includes(extension)) {
    return FileAudio;
  }

  if (
    extension &&
    ["png", "jpg", "jpeg", "gif", "webp", "svg"].includes(extension)
  ) {
    return FileImage;
  }

  if (
    extension &&
    ["js", "ts", "rs", "json", "html", "css"].includes(extension)
  ) {
    return FileCode;
  }

  if (extension && ["pdf", "txt", "md", "epub", "docx"].includes(extension)) {
    return FileText;
  }

  if (download.source === "Torrent") {
    return HardDriveDownload;
  }

  return File;
}

export function DownloadStatus({ download }: { download: Download }) {
  const Icon =
    download.state === "Finished"
      ? Check
      : download.state === "Failed"
        ? CircleAlert
        : download.state === "Paused"
          ? Pause
          : ArrowDown;

  return (
    <span
      className={cn(
        "text-muted-foreground inline-flex items-center gap-1.5 text-xs",
        download.state === "Failed" && "text-destructive"
      )}
    >
      <Icon className="size-3" />
      {stateNames[download.state]}
    </span>
  );
}

function progress(download: Download) {
  return Math.min(100, Math.max(0, download.progress_millionths / 10_000));
}

export function Downloads({
  downloads,
  layout,
  selectedId,
  onInspect,
  onRemove,
  onNew,
  filtered,
}: {
  downloads: Download[];
  layout: Workspace["layout"];
  selectedId?: string;
  onInspect: (download: Download) => void;
  onRemove: (download: Download) => void;
  onNew: () => void;
  filtered: boolean;
}) {
  if (!downloads.length) {
    return (
      <Empty className="min-h-80 border-0">
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <ArrowDown />
          </EmptyMedia>
          <EmptyTitle>
            {filtered ? "No matching downloads" : "No downloads yet"}
          </EmptyTitle>
          <EmptyDescription>
            {filtered
              ? "Try another search or change your filters."
              : "Add a link or open a torrent to get started."}
          </EmptyDescription>
        </EmptyHeader>
        {!filtered && (
          <EmptyContent>
            <Button onClick={onNew}>Add your first download</Button>
            <p className="text-muted-foreground text-xs">or press ⌘N</p>
          </EmptyContent>
        )}
      </Empty>
    );
  }

  if (layout === "Table") {
    return (
      <Table className="[&_tr]:border-0">
        <TableHeader>
          <TableRow>
            <TableHead>Name</TableHead>
            <TableHead>Status</TableHead>
            <TableHead className="hidden lg:table-cell">Source</TableHead>
            <TableHead className="text-right">Size</TableHead>
            <TableHead className="text-right">Speed</TableHead>
            <TableHead>
              <span className="sr-only">Actions</span>
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {downloads.map((download) => {
            const Icon = fileIcon(download);

            return (
              <TableRow
                key={download.id}
                data-state={selectedId === download.id ? "selected" : undefined}
              >
                <TableCell className="max-w-60">
                  <button
                    type="button"
                    className="flex max-w-full items-center gap-3 py-2 text-left outline-none hover:underline focus-visible:underline"
                    onClick={() => onInspect(download)}
                  >
                    <Icon className="text-muted-foreground size-4 shrink-0" />
                    <span className="truncate">{download.name}</span>
                  </button>
                </TableCell>
                <TableCell>
                  <DownloadStatus download={download} />
                </TableCell>
                <TableCell className="text-muted-foreground hidden lg:table-cell">
                  {sourceHost(download)}
                </TableCell>
                <TableCell className="text-muted-foreground text-right tabular-nums">
                  {download.total_size ? bytes(download.total_size) : "—"}
                </TableCell>
                <TableCell className="text-muted-foreground text-right tabular-nums">
                  {download.download_rate
                    ? `${bytes(download.download_rate)}/s`
                    : "—"}
                </TableCell>
                <TableCell>
                  <DownloadActions
                    download={download}
                    onInspect={() => onInspect(download)}
                    onRemove={() => onRemove(download)}
                  />
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    );
  }

  return (
    <ItemGroup className="gap-1">
      {downloads.map((download) => {
        const Icon = fileIcon(download);

        const transferring = [
          "Fetching",
          "Starting",
          "Verifying",
          "Processing",
          "Paused",
          "Pausing",
        ].includes(download.state);

        return (
          <Item
            key={download.id}
            role="listitem"
            data-selected={selectedId === download.id}
            className="download-item"
            size="sm"
          >
            <ItemMedia
              variant="icon"
              className="bg-muted flex size-10 items-center justify-center rounded-lg"
            >
              <Icon />
            </ItemMedia>
            <ItemContent className="min-w-0">
              <ItemTitle className="max-w-full">
                <button
                  type="button"
                  className="truncate text-left outline-none hover:underline focus-visible:underline"
                  onClick={() => onInspect(download)}
                >
                  {download.name}
                </button>
              </ItemTitle>
              <ItemDescription className="flex flex-wrap items-center gap-x-2 gap-y-0.5 text-xs">
                <span>{sourceHost(download)}</span>
                <span aria-hidden>·</span>
                <span className="tabular-nums">
                  {download.total_size
                    ? bytes(download.total_size)
                    : "Size unknown"}
                </span>
                {download.error_message && (
                  <span
                    className="text-destructive truncate"
                    title={download.error_message}
                  >
                    {download.error_message}
                  </span>
                )}
              </ItemDescription>
              {transferring && (
                <Progress
                  className="mt-1 max-w-96"
                  value={progress(download)}
                  aria-label={`${download.name} progress`}
                />
              )}
            </ItemContent>
            <ItemActions className="gap-4">
              <div className="hidden min-w-28 flex-col items-end gap-1 sm:flex">
                <DownloadStatus download={download} />
                <span className="text-muted-foreground text-xs tabular-nums">
                  {download.download_rate
                    ? `${bytes(download.download_rate)}/s · ${timeRemaining(download.eta_seconds)}`
                    : transferring
                      ? `${Math.round(progress(download))}%`
                      : ""}
                </span>
              </div>
              <DownloadActions
                download={download}
                onInspect={() => onInspect(download)}
                onRemove={() => onRemove(download)}
              />
            </ItemActions>
          </Item>
        );
      })}
    </ItemGroup>
  );
}
