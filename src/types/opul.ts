export type MachineSessionState =
  | 'Booting'
  | 'Reconciling'
  | 'Running'
  | 'Quiescing'
  | 'Stopped'
  | 'RecoveryRequired'
  | 'ReadOnly'

export type MachineWorkspaceIconKind = 'Symbol' | 'Emoji'

export interface MachineWorkspaceIcon {
  kind: MachineWorkspaceIconKind
  value: string
}

export type MachineFormat =
  | 'Best'
  | 'BestMp4'
  | 'P1080'
  | 'P720'
  | 'Original'
  | 'AudioMp3'
  | 'AudioM4a'

export type MachineFileOrganization =
  | 'Automatic'
  | 'AlwaysCreateFolder'
  | 'KeepInSaveLocation'

export type MachineLayout = 'List' | 'Table'

export type MachineSort = 'Order' | 'Name' | 'State' | 'Source'

export type MachineGrouping = 'None' | 'State' | 'Folder' | 'Kind' | 'Source'

export type MachineSource = 'Torrent' | 'VideoPage' | 'Direct' | 'Guided'

export type MachineDownloadState =
  | 'Queued'
  | 'Starting'
  | 'Fetching'
  | 'Processing'
  | 'Verifying'
  | 'Seeding'
  | 'Pausing'
  | 'Paused'
  | 'Finished'
  | 'Failed'
  | 'Cancelled'

export type MachineHoldCause = 'Person' | 'Shutdown' | 'Recovery'

export interface MachineCapabilities {
  can_pause: boolean
  can_resume: boolean
  can_retry: boolean
  can_cancel: boolean
  can_stop_seeding: boolean
  can_change_file_selection: boolean
  can_remove: boolean
  can_reveal_output: boolean
  can_move: boolean
}

export interface MachineFolder {
  id: string
  name: string
  path: string
  is_default: boolean
}

export interface MachineTorrentFile {
  index: number
  path: string
  size: number
  priority: string
}

export interface MachineDownload {
  id: string
  folder_id: string
  name: string
  source: MachineSource
  source_url: string
  page_url?: string | null
  format?: MachineFormat | null
  state: MachineDownloadState
  hold_cause?: MachineHoldCause | null
  info_hash?: string | null
  total_size: number
  bytes_downloaded: number
  bytes_uploaded: number
  seed_ratio: number
  seed_ratio_limit?: number | null
  download_rate: number
  upload_rate: number
  eta_seconds?: number | null
  peers: number
  seeds: number
  progress_millionths: number
  piece_length: number
  piece_count: number
  comment?: string | null
  trackers: string[]
  output_path?: string | null
  content_folder?: string | null
  thumbnail_url?: string | null
  extractor?: string | null
  playlist_index?: number | null
  error_message?: string | null
  files: MachineTorrentFile[]
  capabilities: MachineCapabilities
}

export interface MachineWorkspace {
  id: string
  name: string
  icon: MachineWorkspaceIcon
  folders: MachineFolder[]
  default_format: MachineFormat
  file_organization: MachineFileOrganization
  layout: MachineLayout
  sort: MachineSort
  grouping: MachineGrouping
  downloads: MachineDownload[]
}

export interface MachineSnapshot {
  revision: number
  boot_id?: string | null
  sequence: number
  session: MachineSessionState
  workspaces: MachineWorkspace[]
  recovery_notices: string[]
}

export interface MachineCommandReceipt {
  revision: number
  created_download_ids: string[]
}

export interface MediaProbeItem {
  webpage_url: string
  title: string
  thumbnail_url?: string | null
  extractor?: string | null
  duration_seconds?: number | null
  playlist_index?: number | null
}

export interface MediaProbe {
  title: string
  webpage_url: string
  extractor?: string | null
  thumbnail_url?: string | null
  is_playlist: boolean
  items: MediaProbeItem[]
}

export interface MediaToolStatus {
  yt_dlp_path?: string | null
  yt_dlp_version?: string | null
  ffmpeg_path?: string | null
  ffmpeg_version?: string | null
  ready: boolean
  message: string
}

export interface SystemPaths {
  downloads_folder: string
  support_folder: string
}
