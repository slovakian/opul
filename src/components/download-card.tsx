import { AlertCircle, ArrowDown, ArrowUp, Clock, Radio } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import {
  DownloadActions,
  type DownloadActionHandlers,
} from "@/components/download-actions";
import {
  formatBytes,
  formatEta,
  formatRate,
  getMediaArtwork,
  getSourceLabel,
  getStateMeta,
} from "@/lib/format";
import type { MachineDownload } from "@/types/opul";

interface DownloadCardProps {
  download: MachineDownload;
  handlers: DownloadActionHandlers;
}

export function DownloadCard({ download, handlers }: DownloadCardProps) {
  const percent = download.progress_millionths / 10_000;
  const stateMeta = getStateMeta(download.state);
  const isTorrent = download.source === "Torrent";
  const isFailed = download.state === "Failed";
  const isTransferring =
    download.state === "Fetching" ||
    download.state === "Starting" ||
    download.state === "Processing" ||
    download.state === "Verifying";

  const artworkSrc = getMediaArtwork(
    download.name,
    download.source,
    download.thumbnail_url,
  );

  return (
    <div
      onDoubleClick={() => {
        if (download.output_path) {
          handlers.onReveal(download.output_path);
        }
      }}
      className="group relative flex flex-col gap-2.5 rounded-xl border border-border/60 bg-card p-3.5 text-card-foreground shadow-2xs hover:shadow-xs hover:border-border transition-all"
    >
      {/* Top row: Artwork/icon + Title & Meta + Status badge */}
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-start gap-3 min-w-0 flex-1">
          {/* Media artwork icon / thumbnail */}
          <div className="relative size-11 shrink-0 overflow-hidden rounded-lg border border-border/40 bg-muted/30 p-1 flex items-center justify-center">
            <img
              src={artworkSrc}
              alt=""
              className="h-full w-full object-contain select-none"
              loading="lazy"
            />
          </div>

          <div className="grid flex-1 gap-0.5 min-w-0">
            <div className="flex items-center gap-2">
              <span
                className="truncate font-medium text-xs text-foreground leading-snug select-text"
                title={download.name}
              >
                {download.name}
              </span>
            </div>

            <div className="flex flex-wrap items-center gap-1.5 text-[11px] text-muted-foreground font-mono">
              <span className="font-sans font-normal text-[10px] text-muted-foreground/80 bg-muted/50 px-1.5 py-0.2 rounded border border-border/40">
                {getSourceLabel(download.source)}
              </span>

              <span>{formatBytes(download.bytes_downloaded)}</span>
              {download.total_size > 0 && (
                <>
                  <span>/</span>
                  <span>{formatBytes(download.total_size)}</span>
                </>
              )}

              {isTorrent && download.peers > 0 && (
                <span className="flex items-center gap-1 text-[11px] text-muted-foreground/80">
                  <Radio className="size-2.5 text-blue-500" />
                  {download.peers} peers ({download.seeds} seeds)
                </span>
              )}
            </div>
          </div>
        </div>

        {/* Status Badge */}
        <Badge
          variant={stateMeta.badgeVariant}
          className="shrink-0 text-[10px] px-2 py-0.5 h-5 font-medium rounded-full"
        >
          {stateMeta.label}
        </Badge>
      </div>

      {/* Progress Bar & Rate Indicators */}
      <div className="space-y-1">
        <Progress
          value={percent}
          className={`h-1.5 rounded-full ${
            download.state === "Finished"
              ? "[&>div]:bg-emerald-500"
              : download.state === "Failed"
                ? "[&>div]:bg-destructive"
                : download.state === "Paused"
                  ? "[&>div]:bg-amber-500"
                  : "[&>div]:bg-primary"
          }`}
        />

        <div className="flex items-center justify-between text-[11px] font-mono text-muted-foreground">
          <div className="flex items-center gap-2">
            <span className="font-medium text-foreground/80">
              {percent.toFixed(1)}%
            </span>
            {isTransferring &&
              download.eta_seconds != null &&
              download.eta_seconds > 0 && (
                <span className="flex items-center gap-1 text-[10px]">
                  <Clock className="size-2.5 opacity-60" />
                  <span>ETA {formatEta(download.eta_seconds)}</span>
                </span>
              )}
          </div>

          {isTransferring && (
            <div className="flex items-center gap-2 text-[11px]">
              {download.download_rate > 0 && (
                <span className="flex items-center gap-0.5 text-blue-500 font-medium">
                  <ArrowDown className="size-2.5" />
                  {formatRate(download.download_rate)}
                </span>
              )}
              {download.upload_rate > 0 && (
                <span className="flex items-center gap-0.5 text-emerald-500 font-medium">
                  <ArrowUp className="size-2.5" />
                  {formatRate(download.upload_rate)}
                </span>
              )}
            </div>
          )}

          {download.state === "Seeding" && (
            <div className="flex items-center gap-1.5 text-emerald-500 font-medium text-[10px]">
              <ArrowUp className="size-2.5" />
              <span>{formatRate(download.upload_rate)}</span>
              <span className="text-muted-foreground">
                Ratio: {download.seed_ratio.toFixed(2)}
              </span>
            </div>
          )}
        </div>
      </div>

      {/* Error Notice */}
      {isFailed && download.error_message && (
        <div className="flex items-start gap-1.5 rounded-md bg-destructive/10 p-2 text-[11px] text-destructive">
          <AlertCircle className="size-3.5 shrink-0 mt-0.5" />
          <p className="line-clamp-2 select-text">{download.error_message}</p>
        </div>
      )}

      {/* Bottom Footer: Output Path Info & Action Buttons */}
      <div className="flex items-center justify-between pt-1 border-t border-border/40 text-xs">
        <div className="truncate text-[11px] text-muted-foreground/70 font-mono max-w-[65%] select-text">
          {download.output_path ? (
            <span title={download.output_path}>
              {download.output_path.split("/").pop()}
            </span>
          ) : (
            <span className="italic">Downloading...</span>
          )}
        </div>

        {/* Action Buttons */}
        <DownloadActions
          download={download}
          handlers={handlers}
          size="default"
        />
      </div>
    </div>
  );
}
