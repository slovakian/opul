import {
  ArrowDownCircle,
  FolderOpen,
  Info,
  Pause,
  Play,
  RotateCw,
  Trash2,
} from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import {
  formatBytes,
  formatRate,
  getSourceLabel,
  getStateMeta,
} from '@/lib/format'
import type { MachineDownload } from '@/types/opul'

interface DownloadTableProps {
  downloads: MachineDownload[]
  onPause: (id: string) => void
  onResume: (id: string) => void
  onRetry: (id: string) => void
  onReveal: (path: string) => void
  onInspect: (download: MachineDownload) => void
  onRemove: (download: MachineDownload) => void
}

export function DownloadTable({
  downloads,
  onPause,
  onResume,
  onRetry,
  onReveal,
  onInspect,
  onRemove,
}: DownloadTableProps) {
  return (
    <div className="rounded-lg border bg-card overflow-hidden">
      <Table>
        <TableHeader>
          <TableRow className="hover:bg-transparent">
            <TableHead className="w-[35%]">Name</TableHead>
            <TableHead className="w-[12%]">Source</TableHead>
            <TableHead className="w-[12%]">Status</TableHead>
            <TableHead className="w-[16%]">Progress</TableHead>
            <TableHead className="w-[13%]">Size</TableHead>
            <TableHead className="w-[12%] text-right">Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {downloads.map((download) => {
            const percent = download.progress_millionths / 10_000
            const stateMeta = getStateMeta(download.state)
            const isTorrent = download.source === 'Torrent'
            const isPaused = download.state === 'Paused'
            const isFailed = download.state === 'Failed'

            return (
              <TableRow key={download.id} className="text-xs">
                {/* Name */}
                <TableCell className="font-medium max-w-0">
                  <div className="truncate font-semibold text-xs" title={download.name}>
                    {download.name}
                  </div>
                  {download.download_rate > 0 && (
                    <div className="flex items-center gap-1 font-mono text-[11px] text-blue-500">
                      <ArrowDownCircle className="size-3" />
                      {formatRate(download.download_rate)}
                    </div>
                  )}
                </TableCell>

                {/* Source */}
                <TableCell>
                  <Badge variant="outline" className="text-[10px] font-normal">
                    {getSourceLabel(download.source)}
                  </Badge>
                </TableCell>

                {/* Status */}
                <TableCell>
                  <Badge variant={stateMeta.badgeVariant} className="text-[10px] font-medium">
                    {stateMeta.label}
                  </Badge>
                </TableCell>

                {/* Progress */}
                <TableCell>
                  <div className="space-y-1">
                    <Progress value={percent} className="h-1.5" />
                    <span className="text-[10px] font-mono text-muted-foreground">
                      {percent.toFixed(1)}%
                    </span>
                  </div>
                </TableCell>

                {/* Size */}
                <TableCell className="font-mono text-muted-foreground">
                  <div>{formatBytes(download.bytes_downloaded)}</div>
                  {download.total_size > 0 && (
                    <div className="text-[10px] text-muted-foreground/70">
                      of {formatBytes(download.total_size)}
                    </div>
                  )}
                </TableCell>

                {/* Actions */}
                <TableCell className="text-right">
                  <div className="flex items-center justify-end gap-1">
                    {download.capabilities.can_pause && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-6 text-muted-foreground hover:text-foreground"
                        onClick={() => onPause(download.id)}
                        title="Pause"
                      >
                        <Pause className="size-3" />
                      </Button>
                    )}
                    {download.capabilities.can_resume && isPaused && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-6 text-muted-foreground hover:text-foreground"
                        onClick={() => onResume(download.id)}
                        title="Resume"
                      >
                        <Play className="size-3" />
                      </Button>
                    )}
                    {download.capabilities.can_retry && isFailed && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-6 text-muted-foreground hover:text-foreground"
                        onClick={() => onRetry(download.id)}
                        title="Retry"
                      >
                        <RotateCw className="size-3" />
                      </Button>
                    )}
                    {download.output_path && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-6 text-muted-foreground hover:text-foreground"
                        onClick={() => onReveal(download.output_path!)}
                        title="Reveal in Finder"
                      >
                        <FolderOpen className="size-3" />
                      </Button>
                    )}
                    {isTorrent && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-6 text-muted-foreground hover:text-foreground"
                        onClick={() => onInspect(download)}
                        title="Inspect Torrent"
                      >
                        <Info className="size-3" />
                      </Button>
                    )}
                    {download.capabilities.can_remove && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-6 text-muted-foreground hover:text-destructive"
                        onClick={() => onRemove(download)}
                        title="Remove"
                      >
                        <Trash2 className="size-3" />
                      </Button>
                    )}
                  </div>
                </TableCell>
              </TableRow>
            )
          })}
        </TableBody>
      </Table>
    </div>
  )
}
