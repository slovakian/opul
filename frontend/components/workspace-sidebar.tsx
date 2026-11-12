import {
  ArrowDown,
  Check,
  ChevronRight,
  FolderClosed,
  FolderPlus,
  Inbox,
  MoreHorizontal,
  Pencil,
  Smile,
  Plus,
  Settings2,
  Trash2,
} from "lucide-react";
import { useEffect, useRef } from "react";

import { OpulLogoRow } from "@/components/opul-logo-row";
import { SidebarPopover } from "@/components/sidebar-popover";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupAction,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuAction,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  SidebarRail,
  SidebarSeparator,
  useSidebar,
} from "@/components/ui/sidebar";
import { WorkspaceIcon } from "@/components/workspace-icon";
import { useWorkspaceRemovalFocus } from "@/hooks/use-workspace-removal-focus";
import { isActiveDownload } from "@/lib/download-activity";
import { useLibrary, useLibrarySelector } from "@/lib/library";
import { selectDownloads, selectSession } from "@/lib/library-store";
import type { LibraryState } from "@/lib/library-store";
import { aggregateNames, bytes, completeStates } from "@/lib/model";
import type { AggregateView, Folder, Workspace } from "@/lib/model";
import { cn } from "@/lib/utils";
import type { VideoToolsSnapshot } from "@/lib/video-tools";
import { setupProgress } from "@/lib/video-tools";

export type Location =
  | { kind: AggregateView }
  | { kind: "workspace"; workspaceId: string; folderId?: string };

export type StatusFilter = "all" | "active" | "finished" | "paused" | "failed";

const folderRequest = (workspace: Workspace, folder: Folder) => ({
  expected_revision: null,
  folder_id: folder.id,
  name: folder.name,
  path: folder.path,
  request_id: crypto.randomUUID(),
  workspace_id: workspace.id,
});

interface DownloadCounts {
  active: number;
  all: number;
  /** Whether anything is transferring, which animates the In progress label. */
  downloading: boolean;
  finished: number;
}

const selectDownloadCounts = (state: LibraryState): DownloadCounts => {
  const downloads = selectDownloads(state);

  return {
    active: downloads.filter(isActiveDownload).length,
    all: downloads.length,
    downloading: downloads.some((download) => download.download_rate > 0),
    finished: downloads.filter((download) => completeStates.has(download.state))
      .length,
  };
};

const sameCounts = (previous: DownloadCounts, next: DownloadCounts) =>
  previous.active === next.active &&
  previous.all === next.all &&
  previous.downloading === next.downloading &&
  previous.finished === next.finished;

/** The combined download rate, which changes with every transfer update. */
const TransferRate = () => {
  const rate = useLibrarySelector((state) =>
    selectDownloads(state).reduce(
      (sum, download) => sum + download.download_rate,
      0
    )
  );

  return (
    <span className="tabular-nums">
      {rate ? `${bytes(rate)}/s` : "No active transfers"}
    </span>
  );
};

export const WorkspaceSidebar = ({
  workspaces,
  workspaceExpansion,
  onExpansionChange,
  onAddFolder,
  location,
  navigate,
  onCreate,
  onEdit,
  onChangeIcon,
  onSettings,
  onNew,
  onVideoSetup,
  videoTools,
}: {
  workspaces: readonly Workspace[];
  workspaceExpansion: ReadonlyMap<string, boolean>;
  onExpansionChange: (workspaceId: string, expanded: boolean) => void;
  onAddFolder: (workspace: Workspace) => void;
  location: Location;
  navigate: (location: Location) => void;
  onCreate: () => void;
  onEdit: (workspace: Workspace) => void;
  onChangeIcon: (
    workspace: Workspace,
    anchor: HTMLButtonElement | null
  ) => void;
  onSettings: () => void;
  onNew: () => void;
  onVideoSetup: () => void;
  videoTools: VideoToolsSnapshot | null;
}) => {
  const { run, busy } = useLibrary();
  const session = useLibrarySelector(selectSession);
  const counts = useLibrarySelector(selectDownloadCounts, sameCounts);
  const { isMobile, open, openMobile, setOpenMobile } = useSidebar();
  const selectedFolderRef = useRef<HTMLButtonElement>(null);

  const selectedWorkspaceId =
    location.kind === "workspace" ? location.workspaceId : undefined;

  const selectedFolderId =
    location.kind === "workspace" ? location.folderId : undefined;

  const selectedExpanded =
    selectedWorkspaceId !== undefined &&
    workspaceExpansion.get(selectedWorkspaceId);

  useEffect(() => {
    if (
      selectedFolderId &&
      selectedWorkspaceId &&
      selectedExpanded &&
      (isMobile ? openMobile : open)
    ) {
      selectedFolderRef.current?.scrollIntoView({ block: "nearest" });
    }
  }, [
    isMobile,
    open,
    openMobile,
    selectedExpanded,
    selectedFolderId,
    selectedWorkspaceId,
  ]);

  const workspaceFocusTarget = useWorkspaceRemovalFocus(
    workspaces,
    !isMobile || openMobile
  );

  const { downloading } = counts;

  const go = (next: Location) => {
    navigate(next);

    if (isMobile) {
      setOpenMobile(false);
    }
  };

  return (
    <Sidebar
      className="border-r-0"
      // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role
      role="navigation"
      aria-label="Workspace navigation"
    >
      <SidebarHeader className="gap-0 p-0">
        <OpulLogoRow
          onNew={onNew}
          onCreate={onCreate}
          onSettings={onSettings}
        />
        <div className="flex flex-col gap-4 px-3 pb-3">
          <SidebarMenu>
            {[
              {
                count: counts.all,
                icon: Inbox,
                label: aggregateNames.all,
                status: "all" as const,
              },
              {
                count: counts.active,
                icon: ArrowDown,
                label: aggregateNames.active,
                status: "active" as const,
              },
              {
                count: counts.finished,
                icon: Check,
                label: aggregateNames.finished,
                status: "finished" as const,
              },
            ].map((item) => (
              <SidebarMenuItem key={item.label}>
                <SidebarMenuButton
                  isActive={location.kind === item.status}
                  onClick={() => go({ kind: item.status })}
                >
                  <item.icon />
                  {/* The shimmer stays mounted and fades via its text fill, so
                      starting or stopping never snaps the sweep mid-frame. */}
                  <span
                    className={cn(
                      item.status === "active" &&
                        "shimmer transition-[-webkit-text-fill-color] duration-500 ease-out",
                      item.status === "active" &&
                        !downloading &&
                        "[--shimmer-text-fill:currentColor] [animation-play-state:paused]"
                    )}
                  >
                    {item.label}
                  </span>
                </SidebarMenuButton>
                {item.count > 0 && (
                  <SidebarMenuBadge>{item.count}</SidebarMenuBadge>
                )}
              </SidebarMenuItem>
            ))}
          </SidebarMenu>
        </div>
      </SidebarHeader>
      <SidebarSeparator />
      <SidebarContent className="scroll-fade">
        <SidebarGroup>
          <SidebarGroupLabel>Workspaces</SidebarGroupLabel>
          <SidebarGroupAction
            aria-label="New workspace"
            title="New workspace"
            onClick={onCreate}
          >
            <Plus />
          </SidebarGroupAction>
          <SidebarGroupContent>
            <SidebarMenu>
              {workspaces.map((workspace) => (
                <Collapsible
                  key={workspace.id}
                  data-workspace-id={workspace.id}
                  className="group/workspace"
                  open={workspaceExpansion.get(workspace.id) ?? false}
                  onOpenChange={(expanded) =>
                    onExpansionChange(workspace.id, expanded)
                  }
                  render={<SidebarMenuItem />}
                >
                  <SidebarMenuButton
                    data-workspace-navigation
                    aria-label={`${workspace.name} workspace`}
                    isActive={
                      location.kind === "workspace" &&
                      location.workspaceId === workspace.id &&
                      (!location.folderId ||
                        !workspaceExpansion.get(workspace.id))
                    }
                    onClick={() =>
                      go({ kind: "workspace", workspaceId: workspace.id })
                    }
                  >
                    <span
                      aria-hidden
                      className={cn(
                        "flex size-4 shrink-0 items-center justify-center",
                        workspace.folders.length > 1 &&
                          "group-focus-within/workspace:invisible group-hover/workspace:invisible"
                      )}
                    >
                      <WorkspaceIcon workspace={workspace} />
                    </span>
                    <span>{workspace.name}</span>
                  </SidebarMenuButton>
                  {workspace.folders.length > 1 && (
                    <CollapsibleTrigger
                      render={
                        <SidebarMenuAction
                          className="right-auto left-1.5 opacity-0 group-focus-within/workspace:opacity-100 group-hover/workspace:opacity-100 data-panel-open:rotate-90"
                          aria-label={`Toggle ${workspace.name} folders`}
                          data-workspace-disclosure
                        />
                      }
                    >
                      <ChevronRight />
                    </CollapsibleTrigger>
                  )}
                  <SidebarPopover
                    title={`${workspace.name} actions`}
                    trigger={
                      <SidebarMenuAction
                        showOnHover
                        aria-label={`${workspace.name} actions`}
                      >
                        <MoreHorizontal />
                      </SidebarMenuAction>
                    }
                    groups={[
                      {
                        actions: [
                          {
                            disabled: busy || session !== "Running",
                            icon: Smile,
                            label: "Change icon…",
                            onClick: (anchor) =>
                              onChangeIcon(workspace, anchor),
                          },
                          {
                            icon: Pencil,
                            label: "Edit workspace",
                            onClick: () => onEdit(workspace),
                          },
                          {
                            disabled: busy,
                            icon: FolderPlus,
                            label: "Add saved folder",
                            onClick: () => {
                              onAddFolder(workspace);
                            },
                          },
                        ],
                      },
                    ]}
                  />
                  {workspace.folders.length > 1 && (
                    <CollapsibleContent>
                      <SidebarMenuSub>
                        {workspace.folders.map((folder) => (
                          <SidebarMenuSubItem
                            key={folder.id}
                            className="group/folder relative"
                          >
                            <SidebarMenuSubButton
                              render={
                                <button
                                  ref={
                                    selectedWorkspaceId === workspace.id &&
                                    selectedFolderId === folder.id
                                      ? selectedFolderRef
                                      : undefined
                                  }
                                  type="button"
                                  tabIndex={0}
                                  aria-label={folder.name}
                                  data-folder-id={folder.id}
                                />
                              }
                              isActive={
                                location.kind === "workspace" &&
                                location.workspaceId === workspace.id &&
                                location.folderId === folder.id
                              }
                              onClick={() =>
                                go({
                                  folderId: folder.id,
                                  kind: "workspace",
                                  workspaceId: workspace.id,
                                })
                              }
                              title={folder.path}
                            >
                              <FolderClosed />
                              <span>{folder.name}</span>
                            </SidebarMenuSubButton>
                            <SidebarPopover
                              title={`${folder.name} folder actions`}
                              fallbackFocus={() =>
                                workspaceFocusTarget(workspace.id)
                              }
                              trigger={
                                <SidebarMenuAction
                                  className="top-0 opacity-0 group-hover/folder:opacity-100 focus-visible:opacity-100"
                                  aria-label={`${folder.name} folder actions`}
                                  data-folder-id={folder.id}
                                >
                                  <MoreHorizontal />
                                </SidebarMenuAction>
                              }
                              groups={[
                                {
                                  actions: [
                                    {
                                      disabled: folder.is_default || busy,
                                      icon: Check,
                                      label: "Set as default",
                                      onClick: () => {
                                        run({
                                          kind: "default_folder",
                                          request: folderRequest(
                                            workspace,
                                            folder
                                          ),
                                        }).catch(() => null);
                                      },
                                    },
                                    {
                                      destructive: true,
                                      disabled:
                                        workspace.folders.length === 1 || busy,
                                      icon: Trash2,
                                      label: "Remove saved folder",
                                      onClick: () => {
                                        run({
                                          kind: "remove_folder",
                                          request: folderRequest(
                                            workspace,
                                            folder
                                          ),
                                        }).catch(() => null);
                                      },
                                    },
                                  ],
                                  label: folder.is_default
                                    ? "Default save location"
                                    : "Save location",
                                },
                              ]}
                            />
                          </SidebarMenuSubItem>
                        ))}
                      </SidebarMenuSub>
                    </CollapsibleContent>
                  )}
                </Collapsible>
              ))}
              <SidebarMenuItem>
                <SidebarMenuButton onClick={onCreate}>
                  <Plus />
                  <span>New workspace</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
      <SidebarSeparator />
      <SidebarFooter className="gap-3 p-3">
        {videoTools?.state.kind === "setting_up" && (
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton
                onClick={() => {
                  setOpenMobile(false);
                  onVideoSetup();
                }}
              >
                <ArrowDown />
                <span>
                  Setting up video tools…{" "}
                  {setupProgress(videoTools.state.steps)}
                </span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        )}
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton onClick={onSettings}>
              <Settings2 />
              <span>Settings</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
        <div
          className="text-muted-foreground flex items-center gap-2 px-2 pb-1 text-xs"
          aria-live="polite"
        >
          <ArrowDown className="size-3" />
          <TransferRate />
          <span
            className="ml-auto"
            title={`Engine: ${session ?? "connecting"}`}
          >
            <span className="sr-only">Engine </span>
            {session === "Running" ? "Ready" : "Connecting"}
          </span>
        </div>
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  );
};
