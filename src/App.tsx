import { useState } from 'react'
import { open as openFileDialog } from '@tauri-apps/plugin-dialog'
import { Toaster } from '@/components/ui/sonner'
import { TooltipProvider } from '@/components/ui/tooltip'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { AppSidebar } from '@/components/app-sidebar'
import { SiteHeader } from '@/components/site-header'
import { WorkspaceView } from '@/components/workspace-view'
import { CreateWorkspaceDialog } from '@/components/modals/create-workspace-dialog'
import { MediaProbeDialog } from '@/components/modals/media-probe-dialog'
import { TorrentInspectorDialog } from '@/components/modals/torrent-inspector-dialog'
import { PreferencesDialog } from '@/components/modals/preferences-dialog'
import { AddFolderDialog } from '@/components/modals/add-folder-dialog'
import { ConfirmDeleteDialog } from '@/components/modals/confirm-delete-dialog'
import { useOpul } from '@/hooks/use-opul'
import { api } from '@/lib/api'
import { playSound } from '@/lib/sounds'
import type {
  MachineDownload,
  MachineGrouping,
  MachineLayout,
  MachineSort,
  MachineWorkspace,
} from '@/types/opul'
import { toast } from 'sonner'

export default function App() {
  const {
    snapshot,
    paths,
    isLoading,
    selectedWorkspaceId,
    setSelectedWorkspaceId,
    selectedFolderId,
    setSelectedFolderId,
    selectedWorkspace,
    selectedFolder,
    displayedDownloads,
    searchQuery,
    setSearchQuery,
    totalDownloadRate,
    totalUploadRate,
    activeTransfersCount,
    refresh,
  } = useOpul()

  // Dialog state
  const [createWorkspaceOpen, setCreateWorkspaceOpen] = useState(false)
  const [workspaceToEdit, setWorkspaceToEdit] = useState<MachineWorkspace | null>(null)
  const [addFolderWorkspace, setAddFolderWorkspace] = useState<MachineWorkspace | null>(null)
  const [workspaceToDelete, setWorkspaceToDelete] = useState<MachineWorkspace | null>(null)
  const [downloadToRemove, setDownloadToRemove] = useState<MachineDownload | null>(null)
  const [inspectorDownload, setInspectorDownload] = useState<MachineDownload | null>(null)
  const [preferencesOpen, setPreferencesOpen] = useState(false)
  const [probeUrl, setProbeUrl] = useState<string | null>(null)
  const [recoveryNotices, setRecoveryNotices] = useState<string[]>([])

  // Update recovery notices from snapshot
  useState(() => {
    if (snapshot?.recovery_notices && snapshot.recovery_notices.length > 0) {
      setRecoveryNotices(snapshot.recovery_notices)
    }
  })

  // Handle URL or Magnet submission
  const handleSubmitUrl = async (url: string, folderId: string) => {
    if (!selectedWorkspace) return

    const targetFolder = folderId || selectedWorkspace.folders[0]?.id || ''
    const trimmed = url.trim()

    if (trimmed.startsWith('magnet:')) {
      try {
        await api.addTorrent({
          workspaceId: selectedWorkspace.id,
          folderId: targetFolder,
          source: trimmed,
        })
        playSound('tap')
        toast.success('Added BitTorrent download')
        refresh()
      } catch (err) {
        console.error(err)
        toast.error(`Failed to add magnet link: ${String(err)}`)
      }
    } else {
      // Probing video / web / direct link
      setProbeUrl(trimmed)
    }
  }

  // Handle opening .torrent file via native dialog
  const handlePickTorrentFile = async (folderId?: string) => {
    if (!selectedWorkspace) return
    const targetFolder = folderId || selectedWorkspace.folders[0]?.id || ''

    try {
      const selected = await openFileDialog({
        multiple: false,
        filters: [{ name: 'BitTorrent File', extensions: ['torrent'] }],
        title: 'Open .torrent File',
      })

      if (selected && typeof selected === 'string') {
        await api.addTorrent({
          workspaceId: selectedWorkspace.id,
          folderId: targetFolder,
          source: selected,
        })
        playSound('tap')
        toast.success('Added torrent download')
        refresh()
      }
    } catch (err) {
      console.error(err)
      toast.error(`Failed to add torrent file: ${String(err)}`)
    }
  }

  // Transfer actions
  const handlePause = async (id: string) => {
    try {
      await api.pauseDownload(id)
      playSound('tap')
      refresh()
    } catch (err) {
      toast.error(`Failed to pause: ${String(err)}`)
    }
  }

  const handleResume = async (id: string) => {
    try {
      await api.resumeDownload(id)
      playSound('tap')
      refresh()
    } catch (err) {
      toast.error(`Failed to resume: ${String(err)}`)
    }
  }

  const handleRetry = async (id: string) => {
    try {
      await api.retryDownload(id)
      playSound('tap')
      refresh()
    } catch (err) {
      toast.error(`Failed to retry: ${String(err)}`)
    }
  }

  const handleCancel = async (id: string) => {
    try {
      await api.cancelDownload(id)
      playSound('tap')
      refresh()
    } catch (err) {
      toast.error(`Failed to cancel: ${String(err)}`)
    }
  }

  const handleReveal = async (path: string) => {
    try {
      await api.revealInFinder(path)
    } catch (err) {
      toast.error(`Cannot reveal file: ${String(err)}`)
    }
  }

  // View settings update
  const handleLayoutChange = async (layout: MachineLayout) => {
    if (!selectedWorkspace) return
    try {
      await api.setWorkspaceView({
        workspaceId: selectedWorkspace.id,
        layout,
        sort: selectedWorkspace.sort,
        grouping: selectedWorkspace.grouping,
      })
      refresh()
    } catch (e) {
      console.error(e)
    }
  }

  const handleSortChange = async (sort: MachineSort) => {
    if (!selectedWorkspace) return
    try {
      await api.setWorkspaceView({
        workspaceId: selectedWorkspace.id,
        layout: selectedWorkspace.layout,
        sort,
        grouping: selectedWorkspace.grouping,
      })
      refresh()
    } catch (e) {
      console.error(e)
    }
  }

  const handleGroupingChange = async (grouping: MachineGrouping) => {
    if (!selectedWorkspace) return
    try {
      await api.setWorkspaceView({
        workspaceId: selectedWorkspace.id,
        layout: selectedWorkspace.layout,
        sort: selectedWorkspace.sort,
        grouping,
      })
      refresh()
    } catch (e) {
      console.error(e)
    }
  }

  // Deletion confirm handlers
  const handleConfirmDeleteWorkspace = async (moveFilesToTrash: boolean) => {
    if (!workspaceToDelete) return
    try {
      await api.deleteWorkspace(workspaceToDelete.id, moveFilesToTrash)
      playSound('tap')
      toast.success(`Deleted workspace "${workspaceToDelete.name}"`)
      setWorkspaceToDelete(null)
      refresh()
    } catch (err) {
      toast.error(`Failed to delete workspace: ${String(err)}`)
    }
  }

  const handleConfirmRemoveDownload = async (moveFilesToTrash: boolean) => {
    if (!downloadToRemove) return
    try {
      await api.removeDownloads([downloadToRemove.id], moveFilesToTrash)
      playSound('tap')
      toast.success(
        moveFilesToTrash
          ? `Moved "${downloadToRemove.name}" to Trash`
          : `Removed "${downloadToRemove.name}" from list`,
      )
      setDownloadToRemove(null)
      refresh()
    } catch (err) {
      toast.error(`Failed to remove: ${String(err)}`)
    }
  }

  const handleAcknowledgeRecovery = async () => {
    try {
      await api.acknowledgeRecovery()
      setRecoveryNotices([])
      toast.success('App session recovered')
      refresh()
    } catch (err) {
      toast.error(`Failed recovery: ${String(err)}`)
    }
  }

  if (isLoading) {
    return (
      <div className="flex h-screen w-screen items-center justify-center bg-background text-foreground">
        <div className="flex flex-col items-center gap-3">
          <div className="size-8 rounded-full border-2 border-primary border-t-transparent animate-spin" />
          <p className="text-xs text-muted-foreground font-mono">Launching Opul...</p>
        </div>
      </div>
    )
  }

  return (
    <TooltipProvider>
      <SidebarProvider>
        <AppSidebar
          workspaces={snapshot?.workspaces ?? []}
          selectedWorkspaceId={selectedWorkspaceId}
          selectedFolderId={selectedFolderId}
          onSelectWorkspace={setSelectedWorkspaceId}
          onSelectFolder={setSelectedFolderId}
          onCreateWorkspace={() => {
            setWorkspaceToEdit(null)
            setCreateWorkspaceOpen(true)
          }}
          onEditWorkspace={(ws) => {
            setWorkspaceToEdit(ws)
            setCreateWorkspaceOpen(true)
          }}
          onDeleteWorkspace={setWorkspaceToDelete}
          onAddFolder={setAddFolderWorkspace}
          onOpenPreferences={() => setPreferencesOpen(true)}
          totalDownloadRate={totalDownloadRate}
          totalUploadRate={totalUploadRate}
          activeTransfersCount={activeTransfersCount}
        />

        <SidebarInset>
          <SiteHeader
            workspace={selectedWorkspace}
            selectedFolder={selectedFolder}
            searchQuery={searchQuery}
            onSearchChange={setSearchQuery}
            onSubmitUrl={handleSubmitUrl}
            onPickTorrentFile={handlePickTorrentFile}
            layout={selectedWorkspace?.layout ?? 'List'}
            onLayoutChange={handleLayoutChange}
            sort={selectedWorkspace?.sort ?? 'Order'}
            onSortChange={handleSortChange}
            grouping={selectedWorkspace?.grouping ?? 'None'}
            onGroupingChange={handleGroupingChange}
          />

          {selectedWorkspace ? (
            <WorkspaceView
              workspace={selectedWorkspace}
              downloads={displayedDownloads}
              layout={selectedWorkspace.layout ?? 'List'}
              sort={selectedWorkspace.sort ?? 'Order'}
              grouping={selectedWorkspace.grouping ?? 'None'}
              recoveryNotices={recoveryNotices}
              onDismissRecovery={() => setRecoveryNotices([])}
              onAcknowledgeRecovery={handleAcknowledgeRecovery}
              onPause={handlePause}
              onResume={handleResume}
              onRetry={handleRetry}
              onCancel={handleCancel}
              onReveal={handleReveal}
              onInspect={setInspectorDownload}
              onRemove={setDownloadToRemove}
              onPickTorrentFile={() => handlePickTorrentFile()}
            />
          ) : (
            <div className="flex flex-1 items-center justify-center p-8 text-center">
              <p className="text-sm text-muted-foreground">
                No workspace selected. Create one using the sidebar!
              </p>
            </div>
          )}
        </SidebarInset>
      </SidebarProvider>

      {/* Dialogs */}
      <CreateWorkspaceDialog
        open={createWorkspaceOpen}
        onOpenChange={setCreateWorkspaceOpen}
        workspaceToEdit={workspaceToEdit}
        defaultPath={paths?.downloads_folder}
        onSuccess={(id) => {
          refresh()
          if (id) setSelectedWorkspaceId(id)
        }}
      />

      {probeUrl && selectedWorkspace && (
        <MediaProbeDialog
          open={!!probeUrl}
          onOpenChange={(open) => !open && setProbeUrl(null)}
          url={probeUrl}
          workspace={selectedWorkspace}
          onSuccess={refresh}
        />
      )}

      <TorrentInspectorDialog
        open={!!inspectorDownload}
        onOpenChange={(open) => !open && setInspectorDownload(null)}
        download={inspectorDownload}
      />

      <PreferencesDialog
        open={preferencesOpen}
        onOpenChange={setPreferencesOpen}
        paths={paths}
      />

      {addFolderWorkspace && (
        <AddFolderDialog
          open={!!addFolderWorkspace}
          onOpenChange={(open) => !open && setAddFolderWorkspace(null)}
          workspace={addFolderWorkspace}
          onSuccess={refresh}
        />
      )}

      {workspaceToDelete && (
        <ConfirmDeleteDialog
          open={!!workspaceToDelete}
          onOpenChange={(open) => !open && setWorkspaceToDelete(null)}
          title={`Delete "${workspaceToDelete.name}"?`}
          description="Deleting this workspace will remove it from Opul. You can choose to keep downloaded files on disk, or move them to the Trash."
          showTrashOption={workspaceToDelete.downloads.length > 0}
          onConfirm={handleConfirmDeleteWorkspace}
        />
      )}

      {downloadToRemove && (
        <ConfirmDeleteDialog
          open={!!downloadToRemove}
          onOpenChange={(open) => !open && setDownloadToRemove(null)}
          title={`Remove "${downloadToRemove.name}"?`}
          description="Removing stops any transfer in progress. You can remove it from the list only, or move downloaded files to the Trash."
          showTrashOption={true}
          onConfirm={handleConfirmRemoveDownload}
        />
      )}

      <Toaster position="bottom-right" richColors />
    </TooltipProvider>
  )
}
