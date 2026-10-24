import { invoke } from '@tauri-apps/api/core'
import type {
  MachineCommandReceipt,
  MachineFormat,
  MachineGrouping,
  MachineLayout,
  MachineSnapshot,
  MachineSort,
  MachineTorrentFile,
  MediaProbe,
  MediaToolStatus,
  SystemPaths,
} from '@/types/opul'

export const api = {
  getSystemPaths: () => invoke<SystemPaths>('get_system_paths'),

  getSnapshot: () => invoke<MachineSnapshot>('get_snapshot'),

  waitForSnapshot: (afterBootId: string | null, afterSequence: number, timeoutMs?: number) =>
    invoke<MachineSnapshot>('wait_for_snapshot', {
      afterBootId,
      afterSequence,
      timeoutMs,
    }),

  acknowledgeRecovery: () => invoke<MachineCommandReceipt>('acknowledge_recovery'),

  createWorkspace: (params: {
    name: string
    iconKind?: 'symbol' | 'emoji'
    iconValue?: string
    folderName?: string
    folderPath?: string
    defaultFormat?: string
    fileOrganization?: string
  }) => invoke<MachineCommandReceipt>('create_workspace', params),

  updateWorkspace: (params: {
    workspaceId: string
    name: string
    iconKind?: 'symbol' | 'emoji'
    iconValue?: string
    fileOrganization?: string
  }) => invoke<MachineCommandReceipt>('update_workspace', params),

  deleteWorkspace: (workspaceId: string, moveFilesToTrash: boolean) =>
    invoke<MachineCommandReceipt>('delete_workspace', {
      workspaceId,
      moveFilesToTrash,
    }),

  addFolder: (params: {
    workspaceId: string
    name: string
    path: string
    makeDefault: boolean
  }) => invoke<MachineCommandReceipt>('add_folder', params),

  setDefaultFolder: (workspaceId: string, folderId: string) =>
    invoke<MachineCommandReceipt>('set_default_folder', {
      workspaceId,
      folderId,
    }),

  removeFolder: (workspaceId: string, folderId: string, moveFilesToTrash: boolean) =>
    invoke<MachineCommandReceipt>('remove_folder', {
      workspaceId,
      folderId,
      moveFilesToTrash,
    }),

  setWorkspaceView: (params: {
    workspaceId: string
    layout?: MachineLayout
    sort?: MachineSort
    grouping?: MachineGrouping
  }) => invoke<MachineCommandReceipt>('set_workspace_view', params),

  setDefaultFormat: (workspaceId: string, format: MachineFormat) =>
    invoke<MachineCommandReceipt>('set_default_format', {
      workspaceId,
      format,
    }),

  addTorrent: (params: {
    workspaceId: string
    folderId: string
    source: string
    metainfoBytes?: number[] | null
    seedRatioLimit?: number | null
  }) => invoke<MachineCommandReceipt>('add_torrent', params),

  probeMedia: (url: string) => invoke<MediaProbe>('probe_media', { url }),

  addMedia: (params: {
    workspaceId: string
    folderId: string
    format?: string
    items: Array<{
      url: string
      title: string
      page_url: string
      thumbnail_url?: string | null
      extractor?: string | null
      playlist_index?: number | null
    }>
  }) => invoke<MachineCommandReceipt>('add_media', params),

  pauseDownload: (downloadId: string) =>
    invoke<MachineCommandReceipt>('pause_download', { downloadId }),

  resumeDownload: (downloadId: string) =>
    invoke<MachineCommandReceipt>('resume_download', { downloadId }),

  retryDownload: (downloadId: string) =>
    invoke<MachineCommandReceipt>('retry_download', { downloadId }),

  cancelDownload: (downloadId: string) =>
    invoke<MachineCommandReceipt>('cancel_download', { downloadId }),

  stopSeeding: (downloadId: string) =>
    invoke<MachineCommandReceipt>('stop_seeding', { downloadId }),

  removeDownloads: (downloadIds: string[], moveFilesToTrash: boolean) =>
    invoke<MachineCommandReceipt>('remove_downloads', {
      downloadIds,
      moveFilesToTrash,
    }),

  setSeedRatio: (downloadId: string, ratio?: number | null) =>
    invoke<MachineCommandReceipt>('set_seed_ratio', {
      downloadId,
      ratio,
    }),

  setTorrentFiles: (downloadId: string, files: MachineTorrentFile[]) =>
    invoke<MachineCommandReceipt>('set_torrent_files', {
      downloadId,
      files,
    }),

  moveDownload: (params: {
    downloadId: string
    folderId: string
    outputPath: string
    contentFolder?: string | null
  }) => invoke<MachineCommandReceipt>('move_download', params),

  revealInFinder: (path: string) => invoke<void>('reveal_in_finder', { path }),

  getMediaToolStatus: () => invoke<MediaToolStatus>('get_media_tool_status'),

  setSpeedLimits: (downloadBps: number, uploadBps: number) =>
    invoke<void>('set_speed_limits', { downloadBps, uploadBps }),

  setMaxConcurrentMediaDownloads: (maximum: number) =>
    invoke<void>('set_max_concurrent_media_downloads', { maximum }),
}
