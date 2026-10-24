import {
  AlertCircle,
  ArrowDownCircle,
  ArrowUpCircle,
  Clock,
  ExternalLink,
  FileBox,
  FolderOpen,
  Info,
  Pause,
  Play,
  RotateCw,
  Trash2,
  Video,
} from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import {
  formatBytes,
  formatEta,
  formatRate,
  getSourceLabel,
  getStateMeta,
} from '@/lib/format'
import type { MachineDownload } from '@/types/opul'

interface DownloadCardProps {
  download: MachineDownload
  onPause: (id: string) => void
  onResume: (id: string) => void
  onRetry: (id: string) => void
  onCancel: (id: string) => void
  onReveal: (path: string) => void
  onInspect: (download: MachineDownload) => void
  onRemove: (download: MachineDownload) => void
}

export function DownloadCard({
  download,
  onPause,
  onResume,
  onRetry,
  onReveal,
  onInspect,
  onRemove,
}: DownloadCardProps) {
  const percent = download.progress_millionths / 10_000
  const stateMeta = getStateMeta(download.state)
  const isTorrent = download.source === 'Torrent'
  const isVideo = download.source === 'VideoPage' || download.source === 'Guided'
  const isFailed = download.state === 'Failed'
  const isPaused = download.state === 'Paused'
  const isTransferring =
    download.state === 'Fetching' ||
    download.state === 'Starting' ||
    download.state === 'Processing' ||
    download.state === 'Verifying'

  return (
    <div className="group relative flex flex-col gap-2.5 rounded-lg border bg-card p-4 text-card-foreground shadow-xs transition-all hover:shadow-md">
      {/* Top row: Icon/thumbnail + Title + Status badge */}
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-start gap-3 min-w-0 flex-1">
          {/* Thumbnail or source icon */}
          {download.thumbnail_url ? (
            <div className="relative size-12 shrink-0 overflow-hidden rounded-md border bg-muted">
              <img
                src={download.thumbnail_url}
                alt=""
                className="h-full w-full object-cover"
              />
            </div>
          ) : (
            <div className="flex size-10 shrink-0 items-center justify-center rounded-lg border bg-muted/60 text-muted-foreground">
              {isVideo ? (
                <Video className="size-5 text-blue-500" />
              ) : isTorrent ? (
                <FileBox className="size-5 text-indigo-500" />
              ) : (
                <ExternalLink className="size-5 text-emerald-500" />
              )}
            </div>
          )}

          <div className="grid flex-1 gap-1 min-w-0">
            <div className="flex items-center gap-2">
              <span className="truncate font-semibold text-sm" title={download.name}>
                {download.name}
              </span>
              <Badge variant="outline" className="text-[10px] shrink-0 font-normal">
                {getSourceLabel(download.source)}
              </Badge>
            </div>
            <div className="flex items-center gap-2 text-xs text-muted-foreground font-mono">
              <span>{formatBytes(download.bytes_downloaded)}</span>
              {download.total_size > 0 && (
                <>
                  <span>/</span>
                  <span>{formatBytes(download.total_size)}</span>
                </>
              )}
              {isTorrent && download.peers > 0 && (
                <span className="text-[11px] text-muted-foreground">
                  • {download.peers} peers ({download.seeds} seeds)
                </span>
              )}
            </div>
          </div>
        </div>

        {/* Status Badge */}
        <Badge variant={stateMeta.badgeVariant} className="shrink-0 text-xs font-medium">
          {stateMeta.label}
        </Badge>
      </div>

      {/* Progress bar */}
      <div className="space-y-1">
        <Progress value={percent} className="h-2" />
        <div className="flex items-center justify-between text-[11px] font-mono text-muted-foreground">
          <span>{percent.toFixed(1)}%</span>
          {isTransferring && (
            <div className="flex items-center gap-3">
              {download.download_rate > 0 && (
                <span className="flex items-center gap-1 text-blue-500">
                  <ArrowDownCircle className="size-3" />
                  {formatRate(download.download_rate)}
                </span>
              )}
              {download.upload_rate > 0 && (
                <span className="flex items-center gap-1 text-emerald-500">
                  <ArrowUpCircle className="size-3" />
                  {formatRate(download.upload_rate)}
                </span>
              )}
              {download.eta_seconds ? (
                <span className="flex items-center gap-1">
                  <Clock className="size-3" />
                  ETA {formatEta(download.eta_seconds)}
                </span>
              ) : null}
            </div>
          )}
        </div>
      </div>

      {/* Error display if failed */}
      {isFailed && download.error_message && (
        <div className="flex items-center gap-1.5 rounded-md bg-destructive/10 px-2.5 py-1.5 text-xs text-destructive">
          <AlertCircle className="size-3.5 shrink-0" />
          <span className="truncate">{download.error_message}</span>
        </div>
      )}

      {/* Bottom controls & action cluster */}
      <div className="flex items-center justify-between pt-1 border-t border-border/50">
        <div className="text-[11px] text-muted-foreground truncate max-w-[50%]">
          {download.output_path ? (
            <span className="font-mono truncate" title={download.output_path}>
              {download.output_path}
            </span>
          ) : (
            <span>Ready</span>
          )}
        </div>

        <div className="flex items-center gap-1">
          {/* Pause / Resume */}
          {download.capabilities.can_pause && (
            <Button
              variant="ghost"
              size="icon"
              className="size-7 text-muted-foreground hover:text-foreground"
              onClick={() => onPause(download.id)}
              title="Pause"
            >
              <Pause className="size-3.5" />
            </Button>
          )}
          {download.capabilities.can_resume && isPaused && (
            <Button
              variant="ghost"
              size="icon"
              className="size-7 text-muted-foreground hover:text-foreground"
              onClick={() => onResume(download.id)}
              title="Resume"
            >
              <Play className="size-3.5" />
            </Button>
          )}

          {/* Retry */}
          {download.capabilities.can_retry && isFailed && (
            <Button
              variant="ghost"
              size="icon"
              className="size-7 text-muted-foreground hover:text-foreground"
              onClick={() => onRetry(download.id)}
              title="Retry download"
            >
              <RotateCw className="size-3.5" />
            </Button>
          )}

          {/* Reveal in Finder */}
          {download.output_path && (
            <Button
              variant="ghost"
              size="icon"
              className="size-7 text-muted-foreground hover:text-foreground"
              onClick={() => onReveal(download.output_path!)}
              title="Reveal in Finder"
            >
              <FolderOpen className="size-3.5" />
            </Button>
          )}

          {/* Inspect (torrents) */}
          {isTorrent && (
            <Button
              variant="ghost"
              size="icon"
              className="size-7 text-muted-foreground hover:text-foreground"
              onClick={() => onInspect(download)}
              title="Inspect Torrent Details"
            >
              <Info className="size-3.5" />
            </Button>
          )}

          {/* Remove */}
          {download.capabilities.can_remove && (
            <Button
              variant="ghost"
              size="icon"
              className="size-7 text-muted-foreground hover:text-destructive"
              onClick={() => onRemove(download)}
              title="Remove Download"
            >
              <Trash2 className="size-3.5" />
            </Button>
          )}
        </div>
      </div>
    </div>
  )
}
