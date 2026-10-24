import { useState } from 'react'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'
import { api } from '@/lib/api'
import { formatBytes } from '@/lib/format'
import { playSound } from '@/lib/sounds'
import type { MachineDownload } from '@/types/opul'
import { toast } from 'sonner'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  download: MachineDownload | null
}

export function TorrentInspectorDialog({ open, onOpenChange, download }: Props) {
  if (!download) return null

  const [seedLimit, setSeedLimit] = useState<string>(
    download.seed_ratio_limit ? String(download.seed_ratio_limit) : '',
  )
  const [selectedFileIndices, setSelectedFileIndices] = useState<Set<number>>(
    () => new Set(download.files.map((f) => f.index)),
  )
  const [isSaving, setIsSaving] = useState(false)

  const handleToggleFile = (index: number) => {
    setSelectedFileIndices((prev) => {
      const next = new Set(prev)
      if (next.has(index)) {
        if (next.size > 1) {
          next.delete(index)
        } else {
          toast.warning('At least one file must remain selected')
        }
      } else {
        next.add(index)
      }
      return next
    })
  }

  const handleSave = async () => {
    setIsSaving(true)
    try {
      // Update seed ratio
      const limitVal = seedLimit.trim() ? parseFloat(seedLimit) : null
      await api.setSeedRatio(download.id, limitVal)

      // Update files if applicable
      if (download.files.length > 0) {
        const updatedFiles = download.files.map((f) => ({
          ...f,
          priority: selectedFileIndices.has(f.index) ? 'normal' : 'skip',
        }))
        await api.setTorrentFiles(download.id, updatedFiles)
      }

      playSound('tap')
      toast.success('Updated torrent settings')
      onOpenChange(false)
    } catch (err) {
      console.error(err)
      toast.error(`Failed to update: ${String(err)}`)
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[600px] max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="truncate">{download.name}</DialogTitle>
          <DialogDescription className="font-mono text-xs">
            Hash: {download.info_hash || 'N/A'}
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto space-y-4 py-2 pr-1">
          <div className="grid grid-cols-2 gap-3 text-xs bg-muted/40 p-3 rounded-lg border">
            <div>
              <span className="text-muted-foreground">Total Size:</span>{' '}
              <span className="font-medium">{formatBytes(download.total_size)}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Pieces:</span>{' '}
              <span className="font-medium">
                {download.piece_count} × {formatBytes(download.piece_length)}
              </span>
            </div>
            <div>
              <span className="text-muted-foreground">Peers / Seeds:</span>{' '}
              <span className="font-medium">
                {download.peers} peers / {download.seeds} seeds
              </span>
            </div>
            <div>
              <span className="text-muted-foreground">Ratio:</span>{' '}
              <span className="font-medium">{download.seed_ratio.toFixed(2)}</span>
            </div>
          </div>

          <div className="grid gap-2">
            <Label htmlFor="seed-limit">Stop Seeding at Ratio (leave blank for unlimited)</Label>
            <Input
              id="seed-limit"
              type="number"
              step="0.1"
              min="0"
              placeholder="e.g. 1.0 or 2.0"
              value={seedLimit}
              onChange={(e) => setSeedLimit(e.target.value)}
            />
          </div>

          {download.files.length > 0 && (
            <div className="space-y-2">
              <Label>Files ({download.files.length})</Label>
              <div className="max-h-48 overflow-y-auto border rounded-md divide-y text-xs">
                {download.files.map((file) => (
                  <div
                    key={file.index}
                    className="flex items-center justify-between p-2 hover:bg-muted/50 transition-colors"
                  >
                    <div className="flex items-center gap-2 truncate pr-2">
                      <Checkbox
                        id={`file-${file.index}`}
                        checked={selectedFileIndices.has(file.index)}
                        onCheckedChange={() => handleToggleFile(file.index)}
                      />
                      <label
                        htmlFor={`file-${file.index}`}
                        className="truncate cursor-pointer font-mono"
                      >
                        {file.path}
                      </label>
                    </div>
                    <span className="text-muted-foreground whitespace-nowrap">
                      {formatBytes(file.size)}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {download.trackers.length > 0 && (
            <div className="space-y-2">
              <Label>Trackers ({download.trackers.length})</Label>
              <div className="max-h-24 overflow-y-auto border rounded-md p-2 text-xs font-mono text-muted-foreground space-y-1">
                {download.trackers.map((tracker, i) => (
                  <div key={i} className="truncate">
                    {tracker}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isSaving}>
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={isSaving}>
            Save Changes
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
