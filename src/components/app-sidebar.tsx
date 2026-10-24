import * as React from "react";
import {
  ArrowDown,
  ArrowUp,
  Folder,
  FolderPlus,
  MoreHorizontal,
  Pencil,
  Plus,
  Settings,
  Sparkles,
  Trash2,
} from "lucide-react";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuAction,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
} from "@/components/ui/sidebar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { formatRate } from "@/lib/format";
import type { MachineWorkspace } from "@/types/opul";

interface AppSidebarProps extends React.ComponentProps<typeof Sidebar> {
  workspaces: MachineWorkspace[];
  selectedWorkspaceId: string | null;
  selectedFolderId: string | null;
  onSelectWorkspace: (id: string) => void;
  onSelectFolder: (id: string | null) => void;
  onCreateWorkspace: () => void;
  onEditWorkspace: (workspace: MachineWorkspace) => void;
  onDeleteWorkspace: (workspace: MachineWorkspace) => void;
  onAddFolder: (workspace: MachineWorkspace) => void;
  onOpenPreferences: () => void;
  totalDownloadRate: number;
  totalUploadRate: number;
  activeTransfersCount: number;
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
    <Sidebar
      collapsible="icon"
      className="border-r border-sidebar-border bg-sidebar/95 backdrop-blur-sm select-none"
      {...props}
    >
      {/* Native macOS titlebar drag region & clean section header */}
      <SidebarHeader
        data-tauri-drag-region
        className="pt-7 pb-1.5 px-3 border-b border-sidebar-border/40"
      >
        <div
          className="flex items-center justify-between h-7 px-1"
          data-tauri-drag-region
        >
          <span className="text-[11px] font-semibold tracking-wider uppercase text-muted-foreground/80">
            Workspaces
          </span>
          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  variant="ghost"
                  size="icon"
                  className="size-6 text-muted-foreground/80 hover:text-foreground hover:bg-sidebar-accent/80 rounded-md"
                  onClick={onCreateWorkspace}
                >
                  <Plus className="size-3.5" />
                </Button>
              }
            />
            <TooltipContent side="right">New Workspace</TooltipContent>
          </Tooltip>
        </div>
      </SidebarHeader>

      <SidebarContent className="px-1.5 py-2">
        <SidebarGroup className="p-0">
          <SidebarGroupContent>
            <SidebarMenu className="gap-0.5">
              {workspaces.map((ws) => {
                const isSelected = ws.id === selectedWorkspaceId;
                const hasSubfolders = ws.folders.length > 1;
                const totalDls = ws.downloads.length;
                const activeDls = ws.downloads.filter(
                  (d) =>
                    d.state === "Fetching" ||
                    d.state === "Processing" ||
                    d.state === "Verifying" ||
                    d.state === "Starting" ||
                    d.state === "Seeding",
                ).length;

                return (
                  <SidebarMenuItem key={ws.id}>
                    <SidebarMenuButton
                      isActive={isSelected && !selectedFolderId}
                      onClick={() => {
                        onSelectWorkspace(ws.id);
                        onSelectFolder(null);
                      }}
                      className={`gap-2 h-8 px-2 text-[13px] rounded-md transition-colors ${
                        isSelected && !selectedFolderId
                          ? "bg-sidebar-accent text-sidebar-accent-foreground font-semibold shadow-2xs"
                          : "text-sidebar-foreground/85 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
                      }`}
                    >
                      <span className="flex size-4.5 shrink-0 items-center justify-center text-sm">
                        {ws.icon.kind === "Emoji" ? (
                          ws.icon.value
                        ) : ws.icon.value === "folder" ? (
                          <Folder className="size-4 text-muted-foreground/90" />
                        ) : (
                          <Sparkles className="size-4 text-muted-foreground/90" />
                        )}
                      </span>
                      <span className="truncate flex-1 text-left">
                        {ws.name}
                      </span>

                      {/* Active or Total Counts */}
                      {activeDls > 0 ? (
                        <span className="ml-auto flex items-center gap-1 rounded-full bg-blue-500/15 px-1.5 py-0.5 font-mono text-[10px] font-medium text-blue-600 dark:text-blue-400">
                          <span className="size-1.5 rounded-full bg-blue-500 animate-pulse" />
                          {activeDls}
                        </span>
                      ) : totalDls > 0 ? (
                        <span className="ml-auto font-mono text-[11px] text-muted-foreground/60">
                          {totalDls}
                        </span>
                      ) : null}
                    </SidebarMenuButton>

                    <DropdownMenu>
                      <DropdownMenuTrigger
                        render={
                          <SidebarMenuAction
                            showOnHover
                            className="size-6 text-muted-foreground hover:text-foreground"
                          >
                            <MoreHorizontal className="size-3.5" />
                            <span className="sr-only">Workspace options</span>
                          </SidebarMenuAction>
                        }
                      />
                      <DropdownMenuContent align="end" className="w-44 text-xs">
                        <DropdownMenuItem
                          onClick={() => onEditWorkspace(ws)}
                          className="gap-2"
                        >
                          <Pencil className="size-3.5" />
                          <span>Edit Workspace</span>
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={() => onAddFolder(ws)}
                          className="gap-2"
                        >
                          <FolderPlus className="size-3.5" />
                          <span>Add Saved Folder</span>
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                          onClick={() => onDeleteWorkspace(ws)}
                          className="gap-2 text-destructive focus:text-destructive"
                        >
                          <Trash2 className="size-3.5" />
                          <span>Delete Workspace</span>
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>

                    {/* Subfolders Navigation */}
                    {hasSubfolders && isSelected && (
                      <SidebarMenuSub className="my-0.5 ml-4 border-l border-sidebar-border/60 pl-2">
                        {ws.folders.map((folder) => {
                          const isFolderSelected =
                            isSelected && selectedFolderId === folder.id;
                          const folderDlCount = ws.downloads.filter(
                            (d) => d.folder_id === folder.id,
                          ).length;

                          return (
                            <SidebarMenuSubItem key={folder.id}>
                              <SidebarMenuSubButton
                                isActive={isFolderSelected}
                                onClick={() => {
                                  onSelectWorkspace(ws.id);
                                  onSelectFolder(folder.id);
                                }}
                                className={`h-7 px-2 text-xs rounded-md ${
                                  isFolderSelected
                                    ? "bg-sidebar-accent font-medium text-sidebar-accent-foreground"
                                    : "text-muted-foreground hover:text-foreground hover:bg-sidebar-accent/50"
                                }`}
                              >
                                <Folder className="size-3.5 shrink-0 opacity-70" />
                                <span className="truncate flex-1">
                                  {folder.name}
                                </span>
                                {folder.is_default && (
                                  <span className="text-[10px] text-muted-foreground/60 uppercase tracking-wider">
                                    Default
                                  </span>
                                )}
                                {folderDlCount > 0 && !folder.is_default && (
                                  <span className="font-mono text-[10px] text-muted-foreground/60">
                                    {folderDlCount}
                                  </span>
                                )}
                              </SidebarMenuSubButton>
                            </SidebarMenuSubItem>
                          );
                        })}
                      </SidebarMenuSub>
                    )}
                  </SidebarMenuItem>
                );
              })}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      {/* Sleek Native macOS Footer with speed meters and preferences */}
      <SidebarFooter className="border-t border-sidebar-border/50 p-2.5 bg-sidebar/60">
        <div className="flex items-center justify-between gap-2 px-1">
          {/* Network Throughput Meters */}
          <div className="flex flex-col gap-0.5 min-w-0">
            <div className="flex items-center gap-2 font-mono text-[11px] leading-none text-muted-foreground/80">
              <span className="flex items-center gap-1">
                <ArrowDown
                  className={`size-3 ${totalDownloadRate > 0 ? "text-blue-500" : "opacity-40"}`}
                />
                <span
                  className={
                    totalDownloadRate > 0 ? "text-foreground font-medium" : ""
                  }
                >
                  {formatRate(totalDownloadRate)}
                </span>
              </span>
              <span className="flex items-center gap-1">
                <ArrowUp
                  className={`size-3 ${totalUploadRate > 0 ? "text-emerald-500" : "opacity-40"}`}
                />
                <span
                  className={
                    totalUploadRate > 0 ? "text-foreground font-medium" : ""
                  }
                >
                  {formatRate(totalUploadRate)}
                </span>
              </span>
            </div>
            {activeTransfersCount > 0 ? (
              <span className="text-[10px] text-muted-foreground">
                {activeTransfersCount} active{" "}
                {activeTransfersCount === 1 ? "transfer" : "transfers"}
              </span>
            ) : (
              <span className="text-[10px] text-muted-foreground/50">Idle</span>
            )}
          </div>

          {/* Preferences Button */}
          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  variant="ghost"
                  size="icon"
                  className="size-7 text-muted-foreground/80 hover:text-foreground rounded-md hover:bg-sidebar-accent"
                  onClick={onOpenPreferences}
                >
                  <Settings className="size-4" />
                </Button>
              }
            />
            <TooltipContent side="top">Preferences</TooltipContent>
          </Tooltip>
        </div>
      </SidebarFooter>
    </Sidebar>
  );
}
