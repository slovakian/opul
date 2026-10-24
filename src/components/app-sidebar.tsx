import * as React from 'react'
import {
  ArrowDownCircle,
  ArrowUpCircle,
  Folder,
  MoreVertical,
  Plus,
  Settings,
  Sparkles,
  Trash2,
} from 'lucide-react'
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuAction,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
} from '@/components/ui/sidebar'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { formatRate } from '@/lib/format'
import type { MachineWorkspace } from '@/types/opul'

interface AppSidebarProps extends React.ComponentProps<typeof Sidebar> {
  workspaces: MachineWorkspace[]
  selectedWorkspaceId: string | null
  selectedFolderId: string | null
  onSelectWorkspace: (id: string) => void
  onSelectFolder: (id: string | null) => void
  onCreateWorkspace: () => void
  onEditWorkspace: (workspace: MachineWorkspace) => void
  onDeleteWorkspace: (workspace: MachineWorkspace) => void
  onAddFolder: (workspace: MachineWorkspace) => void
  onOpenPreferences: () => void
  totalDownloadRate: number
  totalUploadRate: number
  activeTransfersCount: number
}

export function AppSidebar({
  workspaces,
  selectedWorkspaceId,
  selectedFolderId,
  onSelectWorkspace,
  onSelectFolder,
  onCreateWorkspace,
  onEditWorkspace,
  onDeleteWorkspace,
  onAddFolder,
  onOpenPreferences,
  totalDownloadRate,
  totalUploadRate,
  activeTransfersCount,
  ...props
}: AppSidebarProps) {
  return (
    <Sidebar collapsible="icon" {...props}>
      <SidebarHeader className="border-b border-sidebar-border px-3 py-2.5">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <div className="flex size-7 items-center justify-center rounded-lg bg-primary text-primary-foreground font-semibold shadow-xs">
              <span className="text-sm">⚡</span>
            </div>
            <div className="grid flex-1 text-left leading-tight">
              <span className="truncate font-semibold tracking-tight text-sm">Opul</span>
              <span className="truncate text-[10px] text-muted-foreground">Download Manager</span>
            </div>
          </div>
          <Badge variant="outline" className="text-[10px] font-mono py-0 px-1.5 h-4">
            v0.3.0
          </Badge>
        </div>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <div className="flex items-center justify-between px-2 py-1">
            <SidebarGroupLabel className="text-xs font-semibold">Workspaces</SidebarGroupLabel>
            <Button
              variant="ghost"
              size="icon"
              className="size-6 text-muted-foreground hover:text-foreground"
              onClick={onCreateWorkspace}
              title="Create Workspace"
            >
              <Plus className="size-3.5" />
            </Button>
          </div>

          <SidebarGroupContent>
            <SidebarMenu>
              {workspaces.map((ws) => {
                const isSelected = ws.id === selectedWorkspaceId
                const hasSubfolders = ws.folders.length > 1
                const totalDls = ws.downloads.length

                return (
                  <SidebarMenuItem key={ws.id}>
                    <SidebarMenuButton
                      isActive={isSelected && !selectedFolderId}
                      onClick={() => {
                        onSelectWorkspace(ws.id)
                        onSelectFolder(null)
                      }}
                      className="gap-2.5"
                    >
                      <span className="text-base flex size-4 items-center justify-center">
                        {ws.icon.kind === 'Emoji' ? (
                          ws.icon.value
                        ) : ws.icon.value === 'folder' ? (
                          <Folder className="size-4" />
                        ) : (
                          <Sparkles className="size-4" />
                        )}
                      </span>
                      <span className="truncate font-medium text-xs">{ws.name}</span>
                      {totalDls > 0 && (
                        <span className="ml-auto text-[10px] font-mono text-muted-foreground">
                          {totalDls}
                        </span>
                      )}
                    </SidebarMenuButton>

                    <DropdownMenu>
                      <DropdownMenuTrigger
                        render={
                          <SidebarMenuAction showOnHover>
                            <MoreVertical className="size-3.5" />
                            <span className="sr-only">Workspace options</span>
                          </SidebarMenuAction>
                        }
                      />
                      <DropdownMenuContent align="end" className="w-44">
                        <DropdownMenuItem onClick={() => onEditWorkspace(ws)}>
                          <Settings className="size-3.5 mr-2" />
                          Edit Workspace
                        </DropdownMenuItem>
                        <DropdownMenuItem onClick={() => onAddFolder(ws)}>
                          <Folder className="size-3.5 mr-2" />
                          Add Saved Folder
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                          onClick={() => onDeleteWorkspace(ws)}
                          className="text-destructive focus:text-destructive"
                        >
                          <Trash2 className="size-3.5 mr-2" />
                          Delete Workspace
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>

                    {isSelected && hasSubfolders && (
                      <SidebarMenuSub>
                        <SidebarMenuSubItem>
                          <SidebarMenuSubButton
                            isActive={isSelected && selectedFolderId === null}
                            onClick={() => onSelectFolder(null)}
                          >
                            <span className="text-xs">All Folders</span>
                          </SidebarMenuSubButton>
                        </SidebarMenuSubItem>
                        {ws.folders.map((folder) => (
                          <SidebarMenuSubItem key={folder.id}>
                            <SidebarMenuSubButton
                              isActive={isSelected && selectedFolderId === folder.id}
                              onClick={() => onSelectFolder(folder.id)}
                              className="justify-between"
                            >
                              <span className="truncate text-xs">{folder.name}</span>
                              {folder.is_default && (
                                <span className="text-[9px] text-muted-foreground uppercase tracking-wider">
                                  Default
                                </span>
                              )}
                            </SidebarMenuSubButton>
                          </SidebarMenuSubItem>
                        ))}
                      </SidebarMenuSub>
                    )}
                  </SidebarMenuItem>
                )
              })}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      <SidebarFooter className="border-t border-sidebar-border p-3 space-y-2">
        {/* Network speeds panel */}
        {(totalDownloadRate > 0 || totalUploadRate > 0 || activeTransfersCount > 0) && (
          <div className="rounded-md border bg-muted/40 p-2 text-xs space-y-1">
            <div className="flex items-center justify-between text-[11px]">
              <span className="text-muted-foreground">Active Transfers</span>
              <Badge variant="secondary" className="h-4 px-1 text-[10px] font-mono">
                {activeTransfersCount}
              </Badge>
            </div>
            <div className="flex items-center justify-between text-xs font-mono">
              <span className="flex items-center gap-1 text-blue-500">
                <ArrowDownCircle className="size-3.5" />
                {formatRate(totalDownloadRate)}
              </span>
              <span className="flex items-center gap-1 text-emerald-500">
                <ArrowUpCircle className="size-3.5" />
                {formatRate(totalUploadRate)}
              </span>
            </div>
          </div>
        )}

        <div className="flex items-center justify-between pt-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={onOpenPreferences}
            className="h-8 gap-2 px-2 text-xs text-muted-foreground hover:text-foreground"
          >
            <Settings className="size-4" />
            Preferences
          </Button>
        </div>
      </SidebarFooter>
    </Sidebar>
  )
}
