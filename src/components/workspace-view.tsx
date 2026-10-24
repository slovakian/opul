import * as React from "react";
import {
  AlertTriangle,
  ArrowDownCircle,
  FileCode,
  Layers,
  X,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { DownloadCard } from "@/components/download-card";
import { DownloadTable } from "@/components/download-table";
import type { DownloadActionHandlers } from "@/components/download-actions";
import type {
  MachineDownload,
  MachineGrouping,
  MachineLayout,
  MachineSort,
  MachineWorkspace,
} from "@/types/opul";

interface WorkspaceViewProps {
  workspace: MachineWorkspace;
  downloads: MachineDownload[];
  layout: MachineLayout;
  sort: MachineSort;
  grouping: MachineGrouping;
  recoveryNotices: string[];
  onDismissRecovery: () => void;
  onAcknowledgeRecovery: () => void;
  handlers: DownloadActionHandlers;
  onPickTorrentFile: () => void;
}

export function WorkspaceView({
  workspace,
  downloads,
  layout,
  sort,
  grouping,
  recoveryNotices,
  onDismissRecovery,
  onAcknowledgeRecovery,
  handlers,
  onPickTorrentFile,
}: WorkspaceViewProps) {
  // Sort downloads
  const sortedDownloads = React.useMemo(() => {
    const list = [...downloads];
    switch (sort) {
      case "Name":
        return list.sort((a, b) => a.name.localeCompare(b.name));
      case "State":
        return list.sort((a, b) => a.state.localeCompare(b.state));
      case "Source":
        return list.sort((a, b) => a.source.localeCompare(b.source));
      case "Order":
      default:
        return list;
    }
  }, [downloads, sort]);

  // Group downloads
  const groupedDownloads = React.useMemo(() => {
    if (grouping === "None") {
      return [{ key: "all", label: "", items: sortedDownloads }];
    }

    const groups: Record<string, MachineDownload[]> = {};
    for (const d of sortedDownloads) {
      let key = "Other";
      if (grouping === "State") {
        key = d.state;
      } else if (grouping === "Source") {
        key = d.source;
      } else if (grouping === "Folder") {
        const f = workspace.folders.find((f) => f.id === d.folder_id);
        key = f?.name ?? "Default Folder";
      }
      if (!groups[key]) groups[key] = [];
      groups[key].push(d);
    }

    return Object.entries(groups).map(([key, items]) => ({
      key,
      label: key,
      items,
    }));
  }, [sortedDownloads, grouping, workspace.folders]);

  return (
    <div className="flex-1 flex flex-col p-4 md:p-6 space-y-5 overflow-y-auto">
      {/* Recovery notice banner */}
      {recoveryNotices.length > 0 && (
        <div className="flex items-center justify-between rounded-lg border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-700 dark:text-amber-400">
          <div className="flex items-center gap-2">
            <AlertTriangle className="size-4 shrink-0 text-amber-500" />
            <span>{recoveryNotices.join(" ")}</span>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              className="h-7 text-xs border-amber-500/40 hover:bg-amber-500/20"
              onClick={onAcknowledgeRecovery}
            >
              Resume
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className="size-6 text-muted-foreground"
              onClick={onDismissRecovery}
            >
              <X className="size-3.5" />
            </Button>
          </div>
        </div>
      )}

      {/* Downloads list / table or empty state */}
      {downloads.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center rounded-2xl border border-dashed border-border/70 p-12 text-center min-h-[380px] bg-card/40">
          <div className="flex size-14 items-center justify-center rounded-2xl bg-muted/50 text-muted-foreground/70 mb-4 shadow-2xs">
            <ArrowDownCircle className="size-7" />
          </div>
          <h3 className="text-sm font-semibold tracking-tight text-foreground">
            No downloads in this workspace
          </h3>
          <p className="text-xs text-muted-foreground/80 max-w-sm mt-1.5 mb-5 leading-relaxed">
            Paste a video page, direct file URL, or magnet link in the toolbar
            above, or open a BitTorrent file to begin.
          </p>
          <Button
            variant="outline"
            size="sm"
            onClick={onPickTorrentFile}
            className="gap-2 h-8 text-xs rounded-lg border-border/60 shadow-2xs"
          >
            <FileCode className="size-3.5 text-muted-foreground" />
            <span>Open .torrent File</span>
          </Button>
        </div>
      ) : (
        <div className="space-y-6">
          {groupedDownloads.map((group) => (
            <div key={group.key} className="space-y-3">
              {group.label && (
                <div className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/80 pb-1.5 border-b border-border/40">
                  <Layers className="size-3.5 text-muted-foreground/60" />
                  <span>{group.label}</span>
                  <span className="text-[10px] font-mono opacity-60">
                    ({group.items.length})
                  </span>
                </div>
              )}

              {layout === "Table" ? (
                <DownloadTable downloads={group.items} handlers={handlers} />
              ) : (
                <div className="grid gap-3 sm:grid-cols-1 md:grid-cols-2 xl:grid-cols-3">
                  {group.items.map((download) => (
                    <DownloadCard
                      key={download.id}
                      download={download}
                      handlers={handlers}
                    />
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
