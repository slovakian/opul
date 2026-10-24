import type { MachineDownloadState, MachineSource } from '@/types/opul'

export function formatBytes(bytes: number, decimals = 1): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const dm = decimals < 0 ? 0 : decimals
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  if (i < 0) return `${bytes} B`
  const clamped = Math.min(i, sizes.length - 1)
  return `${parseFloat((bytes / Math.pow(k, clamped)).toFixed(dm))} ${sizes[clamped]}`
}

export function formatRate(bytesPerSec: number): string {
  if (bytesPerSec === 0) return '0 B/s'
  return `${formatBytes(bytesPerSec)}/s`
}

export function formatEta(seconds?: number | null): string {
  if (!seconds || seconds <= 0 || !isFinite(seconds)) return '—'
  if (seconds < 60) return `${seconds}s`
  const minutes = Math.floor(seconds / 60)
  const remainingSecs = seconds % 60
  if (minutes < 60) return `${minutes}m ${remainingSecs}s`
  const hours = Math.floor(minutes / 60)
  const remainingMins = minutes % 60
  return `${hours}h ${remainingMins}m`
}

export function getStateMeta(state: MachineDownloadState): {
  label: string
  color: string
  badgeVariant: 'default' | 'secondary' | 'destructive' | 'outline'
} {
  switch (state) {
    case 'Queued':
      return { label: 'Queued', color: 'text-muted-foreground', badgeVariant: 'outline' }
    case 'Starting':
      return { label: 'Starting', color: 'text-amber-500', badgeVariant: 'secondary' }
    case 'Fetching':
      return { label: 'Downloading', color: 'text-blue-500', badgeVariant: 'default' }
    case 'Processing':
      return { label: 'Processing', color: 'text-indigo-500', badgeVariant: 'secondary' }
    case 'Verifying':
      return { label: 'Verifying', color: 'text-purple-500', badgeVariant: 'secondary' }
    case 'Seeding':
      return { label: 'Seeding', color: 'text-emerald-500', badgeVariant: 'default' }
    case 'Pausing':
      return { label: 'Pausing...', color: 'text-amber-500', badgeVariant: 'outline' }
    case 'Paused':
      return { label: 'Paused', color: 'text-amber-500', badgeVariant: 'outline' }
    case 'Finished':
      return { label: 'Finished', color: 'text-emerald-600', badgeVariant: 'default' }
    case 'Failed':
      return { label: 'Failed', color: 'text-destructive', badgeVariant: 'destructive' }
    case 'Cancelled':
      return { label: 'Cancelled', color: 'text-muted-foreground', badgeVariant: 'outline' }
    default:
      return { label: state, color: 'text-muted-foreground', badgeVariant: 'outline' }
  }
}

export function getSourceLabel(source: MachineSource): string {
  switch (source) {
    case 'Torrent':
      return 'BitTorrent'
    case 'VideoPage':
      return 'Video'
    case 'Direct':
      return 'Direct Link'
    case 'Guided':
      return 'Guided Stream'
    default:
      return source
  }
}
