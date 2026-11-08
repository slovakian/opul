import {
  Archive,
  ArrowDown,
  Check,
  CircleAlert,
  File,
  FileAudio,
  FileCode,
  FileImage,
  FileText,
  Film,
  HardDriveDownload,
  Pause,
} from "lucide-react";
import { createElement, useRef, useState } from "react";
import type { KeyboardEvent, MouseEvent } from "react";

import { DownloadActions } from "@/components/download-actions";
import { DownloadFolderIcon } from "@/components/download-folder-icon";
import {
  DownloadMenuBody,
  downloadMenuPopup,
  useActionAfterClose,
  useDownloadMenuGroups,
} from "@/components/download-menu";
import { Button } from "@/components/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
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
import { VideoToolsUpdateAdvice } from "@/components/video-tools-update-advice";
import type { Download, ListStyle, Workspace } from "@/lib/model";
import { bytes, sourceHost, stateNames, timeRemaining } from "@/lib/model";
import { cn } from "@/lib/utils";

export const fileIcon = (download: Download) => {
  const extension = download.name.split(".").findLast(Boolean)?.toLowerCase();

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
};

const StatusIcon = ({ state }: { state: Download["state"] }) => {
  if (state === "Finished") {
    return <Check className="size-3" />;
  }

  if (state === "Failed") {
    return <CircleAlert className="size-3" />;
  }

  if (state === "Paused") {
    return <Pause className="size-3" />;
  }

  return <ArrowDown className="size-3" />;
};

export const DownloadStatus = ({ download }: { download: Download }) => (
  <span
    className={cn(
      "text-muted-foreground inline-flex items-center gap-1.5 text-xs",
      download.state === "Failed" && "text-destructive"
    )}
  >
    <StatusIcon state={download.state} />
    {download.waiting_label ?? stateNames[download.state]}
  </span>
);

const progress = (download: Download) =>
  Math.min(100, Math.max(0, download.progress_millionths / 10_000));

// Cards mirror the outline button: a raised surface with its own border.
// Minimal keeps rows flat until they are hovered or selected.
// `menuTarget` outlines the row a right-click menu belongs to without selecting it.
const listItemStyles = {
  cards: {
    group: "gap-2",
    item: "border-border bg-background shadow-xs hover:bg-muted/50 dark:border-input dark:bg-input/30 dark:hover:bg-input/50 data-[selected=true]:border-ring data-[selected=true]:bg-muted/50 data-[selected=true]:ring-1 data-[selected=true]:ring-ring/30 dark:data-[selected=true]:border-ring dark:data-[selected=true]:bg-input/50",
    menuTarget:
      "data-popup-open:border-ring data-popup-open:ring-1 data-popup-open:ring-ring/30 dark:data-popup-open:border-ring",
    variant: "outline",
  },
  minimal: {
    group: "gap-1",
    item: "hover:bg-muted/30 data-[selected=true]:bg-muted/50 data-[selected=true]:ring-1 data-[selected=true]:ring-border data-[selected=true]:ring-inset",
    menuTarget:
      "data-popup-open:ring-1 data-popup-open:ring-ring data-popup-open:ring-inset",
    variant: "default",
  },
} as const satisfies Record<
  ListStyle,
  {
    group: string;
    item: string;
    menuTarget: string;
    variant: "default" | "outline";
  }
>;

const interactiveTarget =
  "a, button, input, select, textarea, [role='menuitem']";

/** Row clicks select the download, except when they land on its own controls. */
const selectsRow = (event: MouseEvent<HTMLElement>) => {
  const { currentTarget, target } = event;

  return (
    target instanceof Element &&
    // Clicks inside portaled menus still bubble through React's tree.
    currentTarget.contains(target) &&
    !target.closest(interactiveTarget)
  );
};

/** Shift+F10 and the Menu key are the keyboard equivalents of a right-click. */
const isMenuKey = (event: KeyboardEvent<HTMLElement>) =>
  event.key === "ContextMenu" || (event.shiftKey && event.key === "F10");

interface RowProps {
  download: Download;
  selected: boolean;
  onInspect: (download: Download) => void;
  onRemove: (download: Download) => void;
}

/**
 * State shared by list and table rows: one action list feeds both the "…"
 * menu and the right-click menu, and the menu keys open the "…" menu.
 */
const useDownloadRow = ({ download, onInspect, onRemove }: RowProps) => {
  const [menuOpen, setMenuOpen] = useState(false);
  const nameRef = useRef<HTMLButtonElement>(null);

  // The details panel returns focus to whatever was focused when it opened,
  // so a row click hands that role to the row's name.
  const handleInspect = () => {
    nameRef.current?.focus();
    onInspect(download);
  };

  // Both menus focus their return target (the "…" button, or the row's name)
  // before running this, via useActionAfterClose.
  const handleDetails = () => onInspect(download);
  const remove = () => onRemove(download);

  const groups = useDownloadMenuGroups({
    download,
    onInspect: handleDetails,
    onRemove: remove,
  });

  const contextMenuAction = useActionAfterClose(nameRef);

  const handleMenuKey = (event: KeyboardEvent<HTMLElement>) => {
    if (isMenuKey(event)) {
      event.preventDefault();
      setMenuOpen(true);
    }
  };

  return {
    actions: (
      <DownloadActions
        download={download}
        onInspect={handleDetails}
        onRemove={remove}
        menuOpen={menuOpen}
        onMenuOpenChange={setMenuOpen}
      />
    ),
    contextMenu: (
      <ContextMenuContent
        aria-label={`Actions for ${download.name}`}
        className={downloadMenuPopup}
      >
        <DownloadMenuBody
          groups={groups}
          onSelect={contextMenuAction.handleSelect}
        />
      </ContextMenuContent>
    ),
    handleContextMenuClosed: contextMenuAction.handleOpenChangeComplete,
    handleInspect,
    handleMenuKey,
    nameRef,
  };
};

const DownloadTableRow = (props: RowProps) => {
  const { download, selected } = props;

  const {
    actions,
    contextMenu,
    handleContextMenuClosed,
    handleInspect,
    handleMenuKey,
    nameRef,
  } = useDownloadRow(props);

  return (
    <ContextMenu onOpenChangeComplete={handleContextMenuClosed}>
      <ContextMenuTrigger
        // The trigger is the row itself, so it keeps the row's slot name.
        data-slot="table-row"
        render={
          <TableRow
            data-state={selected ? "selected" : undefined}
            className="data-popup-open:bg-muted/50"
          />
        }
        onKeyDown={handleMenuKey}
      >
        <TableCell className="max-w-60">
          <button
            ref={nameRef}
            type="button"
            className="flex max-w-full items-center gap-3 py-2 text-left outline-none hover:underline focus-visible:underline"
            onClick={handleInspect}
          >
            {createElement(fileIcon(download), {
              className: "text-muted-foreground size-4 shrink-0",
            })}
            <span className="truncate">{download.name}</span>
          </button>
        </TableCell>
        <TableCell>
          <DownloadStatus download={download} />
          {download.error_message && (
            <p
              className="text-destructive truncate text-xs"
              title={download.error_message}
            >
              {download.error_message}
            </p>
          )}
          <VideoToolsUpdateAdvice download={download} />
        </TableCell>
        <TableCell className="text-muted-foreground hidden lg:table-cell">
          {sourceHost(download)}
        </TableCell>
        <TableCell className="text-muted-foreground text-right tabular-nums">
          {download.total_size ? bytes(download.total_size) : "—"}
        </TableCell>
        <TableCell className="text-muted-foreground text-right tabular-nums">
          {download.download_rate ? `${bytes(download.download_rate)}/s` : "—"}
        </TableCell>
        <TableCell>{actions}</TableCell>
      </ContextMenuTrigger>
      {contextMenu}
    </ContextMenu>
  );
};

const transferringStates = new Set<Download["state"]>([
  "Fetching",
  "Starting",
  "Verifying",
  "Seeding",
]);

const DownloadListItem = ({
  listStyle,
  ...props
}: RowProps & { listStyle: ListStyle }) => {
  const { download, selected } = props;

  const {
    actions,
    contextMenu,
    handleContextMenuClosed,
    handleInspect,
    handleMenuKey,
    nameRef,
  } = useDownloadRow(props);

  const styles = listItemStyles[listStyle];
  const transferring = transferringStates.has(download.state);

  let rateOrProgress = "";

  if (download.download_rate) {
    rateOrProgress = `${bytes(download.download_rate)}/s · ${timeRemaining(download.eta_seconds)}`;
  } else if (transferring) {
    rateOrProgress = `${Math.round(progress(download))}%`;
  }

  return (
    <ContextMenu onOpenChangeComplete={handleContextMenuClosed}>
      <ContextMenuTrigger
        // The trigger is the row itself, so it keeps the row's slot name.
        data-slot="item"
        render={
          <Item
            // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role
            role="listitem"
            variant={styles.variant}
            data-selected={selected}
            className={cn(styles.item, styles.menuTarget)}
          />
        }
        // Keyboard users select through the focusable name button below.
        onClick={(event) => {
          if (selectsRow(event)) {
            handleInspect();
          }
        }}
        onKeyDown={handleMenuKey}
      >
        <ItemMedia className="size-9">
          <DownloadFolderIcon download={download} className="size-8" />
        </ItemMedia>
        <ItemContent className="min-w-0">
          <ItemTitle className="max-w-full">
            <button
              ref={nameRef}
              type="button"
              className="truncate text-left outline-none hover:underline focus-visible:underline"
              onClick={handleInspect}
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
          <VideoToolsUpdateAdvice download={download} />
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
              {rateOrProgress}
            </span>
          </div>
          {actions}
        </ItemActions>
      </ContextMenuTrigger>
      {contextMenu}
    </ContextMenu>
  );
};

export const Downloads = ({
  downloads,
  layout,
  listStyle = "cards",
  selectedId,
  onInspect,
  onRemove,
  onNew,
  filtered,
}: {
  downloads: Download[];
  layout: Workspace["layout"];
  listStyle?: ListStyle;
  selectedId?: string;
  onInspect: (download: Download) => void;
  onRemove: (download: Download) => void;
  onNew: () => void;
  filtered: boolean;
}) => {
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
              ? "Try a different search query or change your filters."
              : "Paste a video link, direct download URL, or torrent magnet to get started."}
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
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-1/2">Name</TableHead>
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
          {downloads.map((download) => (
            <DownloadTableRow
              key={download.id}
              download={download}
              selected={selectedId === download.id}
              onInspect={onInspect}
              onRemove={onRemove}
            />
          ))}
        </TableBody>
      </Table>
    );
  }

  return (
    <ItemGroup
      className={listItemStyles[listStyle].group}
      data-list-style={listStyle}
    >
      {downloads.map((download) => (
        <DownloadListItem
          key={download.id}
          download={download}
          listStyle={listStyle}
          selected={selectedId === download.id}
          onInspect={onInspect}
          onRemove={onRemove}
        />
      ))}
    </ItemGroup>
  );
};
