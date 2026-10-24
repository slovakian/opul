import * as React from 'react'
import {
  AlertTriangle,
  Download,
  FileCode,
  Layers,
  X,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { DownloadCard } from '@/components/download-card'
import { DownloadTable } from '@/components/download-table'
import type {
  MachineDownload,
  MachineGrouping,
  MachineLayout,
  MachineSort,
  MachineWorkspace,
} from '@/types/opul'

interface WorkspaceViewProps {
  workspace: MachineWorkspace
  downloads: MachineDownload[]
  layout: MachineLayout
  sort: MachineSort
  grouping: MachineGrouping
  recoveryNotices: string[]
  onDismissRecovery: () => void
  onAcknowledgeRecovery: () => void
  onPause: (id: string) => void
  onResume: (id: string) => void
  onRetry: (id: string) => void
  onCancel: (id: string) => void
  onReveal: (path: string) => void
  onInspect: (download: MachineDownload) => void
  onRemove: (download: MachineDownload) => void
  onPickTorrentFile: () => void
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
  onPause,
  onResume,
  onRetry,
  onCancel,
  onReveal,
  onInspect,
  onRemove,
  onPickTorrentFile,
}: WorkspaceViewProps) {
  // Sort downloads
  const sortedDownloads = React.useMemo(() => {
    const list = [...downloads]
    switch (sort) {
      case 'Name':
        return list.sort((a, b) => a.name.localeCompare(b.name))
      case 'State':
        return list.sort((a, b) => a.state.localeCompare(b.state))
      case 'Source':
        return list.sort((a, b) => a.source.localeCompare(b.source))
      case 'Order':
      default:
        return list
    }
  }, [downloads, sort])

  // Group downloads
  const groupedDownloads = React.useMemo(() => {
    if (grouping === 'None') {
      return [{ key: 'all', label: '', items: sortedDownloads }]
    }

    const groups: Record<string, MachineDownload[]> = {}
    for (const d of sortedDownloads) {
      let key = 'Other'
      if (grouping === 'State') {
        key = d.state
      } else if (grouping === 'Source') {
        key = d.source
      } else if (grouping === 'Folder') {
        const f = workspace.folders.find((f) => f.id === d.folder_id)
        key = f?.name ?? 'Default Folder'
      }
      if (!groups[key]) groups[key] = []
      groups[key].push(d)
    }

    return Object.entries(groups).map(([key, items]) => ({
      key,
      label: key,
      items,
    }))
  }, [sortedDownloads, grouping, workspace.folders])

  return (
    <div className="flex-1 flex flex-col p-4 md:p-6 space-y-6">
      {/* Recovery notice banner */}
      {recoveryNotices.length > 0 && (
        <div className="flex items-center justify-between rounded-lg border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-700 dark:text-amber-400">
          <div className="flex items-center gap-2">
            <AlertTriangle className="size-4 shrink-0 text-amber-500" />
            <span>{recoveryNotices.join(' ')}</span>
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
        <div className="flex flex-1 flex-col items-center justify-center rounded-xl border border-dashed p-12 text-center min-h-[360px]">
          <div className="flex size-14 items-center justify-center rounded-2xl bg-muted/60 text-muted-foreground mb-4">
            <Download className="size-7" />
          </div>
          <h3 className="text-base font-semibold">No downloads in this workspace</h3>
          <p className="text-xs text-muted-foreground max-w-sm mt-1 mb-6">
            Paste a video URL, torrent magnet link, direct web link above, or open a .torrent file to start downloading.
          </p>
          <div className="flex flex-wrap items-center justify-center gap-3">
            <Button variant="outline" size="sm" onClick={onPickTorrentFile} className="gap-2">
              <FileCode className="size-3.5" />
              Open .torrent File
            </Button>
          </div>
        </div>
      ) : (
        <div className="space-y-6">
          {groupedDownloads.map((group) => (
            <div key={group.key} className="space-y-3">
              {group.label && (
                <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground pb-1 border-b">
                  <Layers className="size-3.5" />
                  <span>{group.label}</span>
                  <span className="text-[10px] font-mono text-muted-foreground">
                    ({group.items.length})
                  </span>
                </div>
              )}

              {layout === 'Table' ? (
                <DownloadTable
                  downloads={group.items}
                  onPause={onPause}
                  onResume={onResume}
                  onRetry={onRetry}
                  onReveal={onReveal}
                  onInspect={onInspect}
                  onRemove={onRemove}
                />
              ) : (
                <div className="grid gap-3 sm:grid-cols-1 lg:grid-cols-2 xl:grid-cols-3">
                  {group.items.map((download) => (
                    <DownloadCard
                      key={download.id}
                      download={download}
                      onPause={onPause}
                      onResume={onResume}
                      onRetry={onRetry}
                      onCancel={onCancel}
                      onReveal={onReveal}
                      onInspect={onInspect}
                      onRemove={onRemove}
                    />
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
