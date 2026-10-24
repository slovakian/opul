import { ArrowDown, ArrowUp } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  DownloadActions,
  type DownloadActionHandlers,
} from "@/components/download-actions";
import {
  formatBytes,
  formatRate,
  getMediaArtwork,
  getSourceLabel,
  getStateMeta,
} from "@/lib/format";
import type { MachineDownload } from "@/types/opul";

interface DownloadTableProps {
  downloads: MachineDownload[];
  handlers: DownloadActionHandlers;
}

export function DownloadTable({ downloads, handlers }: DownloadTableProps) {
  return (
    <div className="rounded-xl border border-border/60 bg-card overflow-hidden shadow-2xs">
      <Table>
        <TableHeader>
          <TableRow className="hover:bg-transparent border-b border-border/60 bg-muted/30">
            <TableHead className="w-[40%] text-xs font-medium">Name</TableHead>
            <TableHead className="w-[10%] text-xs font-medium">
              Source
            </TableHead>
            <TableHead className="w-[12%] text-xs font-medium">
              Status
            </TableHead>
            <TableHead className="w-[18%] text-xs font-medium">
              Progress
            </TableHead>
            <TableHead className="w-[10%] text-xs font-medium">Size</TableHead>
            <TableHead className="w-[10%] text-right text-xs font-medium pr-4">
              Actions
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {downloads.map((download) => {
            const percent = download.progress_millionths / 10_000;
            const stateMeta = getStateMeta(download.state);
            const artworkSrc = getMediaArtwork(
              download.name,
              download.source,
              download.thumbnail_url,
            );

            return (
              <TableRow
                key={download.id}
                className="text-xs group hover:bg-muted/40 transition-colors"
                onDoubleClick={() => {
                  if (download.output_path) {
                    handlers.onReveal(download.output_path);
                  }
                }}
              >
                {/* Name & Artwork */}
                <TableCell className="font-medium max-w-0 py-2.5">
                  <div className="flex items-center gap-2.5 min-w-0">
                    <img
                      src={artworkSrc}
                      alt=""
                      className="size-5 shrink-0 object-contain select-none"
                    />
                    <div className="min-w-0 flex-1">
                      <div
                        className="truncate font-medium text-xs text-foreground select-text"
                        title={download.name}
                      >
                        {download.name}
                      </div>
                      {download.download_rate > 0 && (
                        <div className="flex items-center gap-1 font-mono text-[10px] text-blue-500">
                          <ArrowDown className="size-2.5" />
                          {formatRate(download.download_rate)}
                        </div>
                      )}
                      {download.upload_rate > 0 &&
                        download.download_rate === 0 && (
                          <div className="flex items-center gap-1 font-mono text-[10px] text-emerald-500">
                            <ArrowUp className="size-2.5" />
                            {formatRate(download.upload_rate)}
                          </div>
                        )}
                    </div>
                  </div>
                </TableCell>

                {/* Source */}
                <TableCell className="py-2.5">
                  <span className="font-sans text-[10px] text-muted-foreground/80 bg-muted/50 px-1.5 py-0.5 rounded border border-border/40">
                    {getSourceLabel(download.source)}
                  </span>
                </TableCell>

                {/* Status */}
                <TableCell className="py-2.5">
                  <Badge
                    variant={stateMeta.badgeVariant}
                    className="text-[10px] font-medium h-5 px-1.5 rounded-full"
                  >
                    {stateMeta.label}
                  </Badge>
                </TableCell>

                {/* Progress */}
                <TableCell className="py-2.5">
                  <div className="space-y-1 w-36">
                    <div className="flex items-center justify-between text-[10px] font-mono text-muted-foreground">
                      <span>{percent.toFixed(1)}%</span>
                    </div>
                    <Progress
                      value={percent}
                      className={`h-1.5 ${
                        download.state === "Finished"
                          ? "[&>div]:bg-emerald-500"
                          : download.state === "Failed"
                            ? "[&>div]:bg-destructive"
                            : download.state === "Paused"
                              ? "[&>div]:bg-amber-500"
                              : "[&>div]:bg-primary"
                      }`}
                    />
                  </div>
                </TableCell>

                {/* Size */}
                <TableCell className="font-mono text-[11px] text-muted-foreground py-2.5">
                  {download.total_size > 0
                    ? formatBytes(download.total_size)
                    : formatBytes(download.bytes_downloaded)}
                </TableCell>

                {/* Actions */}
                <TableCell className="text-right py-2.5 pr-3">
                  <div className="flex justify-end">
                    <DownloadActions
                      download={download}
                      handlers={handlers}
                      size="sm"
                    />
                  </div>
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </div>
  );
}
