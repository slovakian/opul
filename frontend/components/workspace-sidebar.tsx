import {
  ArrowDown,
  Check,
  ChevronRight,
  FolderClosed,
  FolderPlus,
  Inbox,
  MoreHorizontal,
  Pencil,
  Plus,
  Settings2,
  Trash2,
} from "lucide-react";
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
import { SidebarPopover } from "@/components/sidebar-popover";
import { useLibrary } from "@/lib/library";
import { bridge, errorMessage } from "@/lib/bridge";
import { toast } from "@/components/ui/toast";
import opulLogo from "@/assets/opul-logo.png";
import {
  bytes,
  workspaceEmoji,
  type Folder,
  type Workspace,
} from "@/lib/model";

export type Location =
  | { kind: "all" }
  | { kind: "workspace"; workspaceId: string; folderId?: string };
export type StatusFilter = "all" | "active" | "finished" | "paused" | "failed";

export function WorkspaceSidebar({
  workspaces,
  location,
  filter,
  navigate,
  setFilter,
  onCreate,
  onEdit,
  onSettings,
  onNew,
}: {
  workspaces: Workspace[];
  location: Location;
  filter: StatusFilter;
  navigate: (location: Location) => void;
  setFilter: (filter: StatusFilter) => void;
  onCreate: () => void;
  onEdit: (workspace: Workspace) => void;
  onSettings: () => void;
  onNew: () => void;
}) {
  const { run, busy, state } = useLibrary();
  const { isMobile, setOpenMobile } = useSidebar();
  const allDownloads = workspaces.flatMap((workspace) => workspace.downloads);
  const active = allDownloads.filter(
    (download) =>
      download.capabilities.can_pause || download.state === "Pausing",
  );
  const downloadRate = allDownloads.reduce(
    (sum, download) => sum + download.download_rate,
    0,
  );
  function go(next: Location, status: StatusFilter = "all") {
    navigate(next);
    setFilter(status);
    if (isMobile) setOpenMobile(false);
  }
  async function addFolder(workspace: Workspace) {
    try {
      const path = await bridge.chooseFolder();
      if (!path) return;
      await run({
        kind: "add_folder",
        request: {
          request_id: crypto.randomUUID(),
          expected_revision: null,
          workspace_id: workspace.id,
          folder_id: crypto.randomUUID(),
          name: path.split("/").filter(Boolean).pop() || "Folder",
          path,
        },
      });
    } catch (error) {
      toast.add({
        title: "Couldn't add folder",
        description: errorMessage(error),
        type: "error",
      });
    }
  }
  function folderRequest(workspace: Workspace, folder: Folder) {
    return {
      request_id: crypto.randomUUID(),
      expected_revision: null,
      workspace_id: workspace.id,
      folder_id: folder.id,
      name: folder.name,
      path: folder.path,
    };
  }
  return (
    <Sidebar
      className="border-r-0"
      role="navigation"
      aria-label="Workspace navigation"
    >
      <SidebarHeader className="gap-4 px-3 pb-3 pt-12">
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarPopover
              align="start"
              title="Opul"
              trigger={
                <SidebarMenuButton className="w-fit">
                  <img src={opulLogo} alt="" className="size-6 shrink-0" />
                  <span className="font-semibold">Opul</span>
                  <ChevronRight className="rotate-90 opacity-50" />
                </SidebarMenuButton>
              }
              groups={[
                {
                  actions: [
                    {
                      label: "New download",
                      icon: Plus,
                      onClick: onNew,
                      hint: "⌘N",
                    },
                    {
                      label: "New workspace",
                      icon: FolderPlus,
                      onClick: onCreate,
                    },
                  ],
                },
                {
                  actions: [
                    {
                      label: "Settings",
                      icon: Settings2,
                      onClick: onSettings,
                      hint: "⌘,",
                    },
                  ],
                },
              ]}
            />
          </SidebarMenuItem>
        </SidebarMenu>
        <SidebarSeparator />
        <SidebarMenu>
          {[
            {
              label: "All downloads",
              icon: Inbox,
              status: "all",
              count: allDownloads.length,
            },
            {
              label: "In progress",
              icon: ArrowDown,
              status: "active",
              count: active.length,
            },
            {
              label: "Completed",
              icon: Check,
              status: "finished",
              count: allDownloads.filter(
                (download) => download.state === "Finished",
              ).length,
            },
          ].map((item) => (
            <SidebarMenuItem key={item.label}>
              <SidebarMenuButton
                isActive={location.kind === "all" && filter === item.status}
                onClick={() =>
                  go(
                    { kind: "all" },
                    item.status === "active"
                      ? "active"
                      : item.status === "finished"
                        ? "finished"
                        : "all",
                  )
                }
              >
                <item.icon />
                <span>{item.label}</span>
              </SidebarMenuButton>
              {item.count > 0 && (
                <SidebarMenuBadge>{item.count}</SidebarMenuBadge>
              )}
            </SidebarMenuItem>
          ))}
        </SidebarMenu>
      </SidebarHeader>
      <SidebarSeparator />
      <SidebarContent>
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
                  className="group/workspace"
                  defaultOpen
                  render={<SidebarMenuItem />}
                >
                  <SidebarMenuButton
                    aria-label={`${workspace.name} workspace`}
                    isActive={
                      location.kind === "workspace" &&
                      location.workspaceId === workspace.id &&
                      !location.folderId
                    }
                    onClick={() =>
                      go({ kind: "workspace", workspaceId: workspace.id })
                    }
                  >
                    <span
                      aria-hidden
                      className="flex size-4 shrink-0 items-center justify-center group-hover/workspace:invisible group-focus-within/workspace:invisible"
                    >
                      {workspaceEmoji(workspace)}
                    </span>
                    <span>{workspace.name}</span>
                  </SidebarMenuButton>
                  <CollapsibleTrigger
                    render={
                      <SidebarMenuAction
                        className="left-1.5 right-auto opacity-0 group-hover/workspace:opacity-100 group-focus-within/workspace:opacity-100 data-panel-open:rotate-90"
                        aria-label={`Toggle ${workspace.name} folders`}
                      />
                    }
                  >
                    <ChevronRight />
                  </CollapsibleTrigger>
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
                            label: "Edit workspace",
                            icon: Pencil,
                            onClick: () => onEdit(workspace),
                          },
                          {
                            label: "Add saved folder",
                            icon: FolderPlus,
                            onClick: () => void addFolder(workspace),
                            disabled: busy,
                          },
                        ],
                      },
                    ]}
                  />
                  <CollapsibleContent>
                    <SidebarMenuSub>
                      {workspace.folders.map((folder) => (
                        <SidebarMenuSubItem
                          key={folder.id}
                          className="group/folder relative"
                        >
                          <SidebarMenuSubButton
                            render={<button type="button" />}
                            isActive={
                              location.kind === "workspace" &&
                              location.folderId === folder.id
                            }
                            onClick={() =>
                              go({
                                kind: "workspace",
                                workspaceId: workspace.id,
                                folderId: folder.id,
                              })
                            }
                            title={folder.path}
                          >
                            <FolderClosed />
                            <span>{folder.name}</span>
                          </SidebarMenuSubButton>
                          <SidebarPopover
                            title={`${folder.name} folder actions`}
                            trigger={
                              <SidebarMenuAction
                                className="top-0 opacity-0 group-hover/folder:opacity-100 focus-visible:opacity-100"
                                aria-label={`${folder.name} folder actions`}
                              >
                                <MoreHorizontal />
                              </SidebarMenuAction>
                            }
                            groups={[
                              {
                                label: folder.is_default
                                  ? "Default save location"
                                  : "Save location",
                                actions: [
                                  {
                                    label: "Set as default",
                                    icon: Check,
                                    disabled: folder.is_default || busy,
                                    onClick: () =>
                                      void run({
                                        kind: "default_folder",
                                        request: folderRequest(
                                          workspace,
                                          folder,
                                        ),
                                      }),
                                  },
                                  {
                                    label: "Remove saved folder",
                                    icon: Trash2,
                                    destructive: true,
                                    disabled:
                                      workspace.folders.length === 1 || busy,
                                    onClick: () =>
                                      void run({
                                        kind: "remove_folder",
                                        request: folderRequest(
                                          workspace,
                                          folder,
                                        ),
                                      }),
                                  },
                                ],
                              },
                            ]}
                          />
                        </SidebarMenuSubItem>
                      ))}
                    </SidebarMenuSub>
                  </CollapsibleContent>
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
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton onClick={onSettings}>
              <Settings2 />
              <span>Settings</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
        <div
          className="flex items-center gap-2 px-2 pb-1 text-xs text-muted-foreground"
          aria-live="polite"
        >
          <ArrowDown className="size-3" />
          <span className="tabular-nums">
            {downloadRate ? `${bytes(downloadRate)}/s` : "No active transfers"}
          </span>
          <span
            className="ml-auto"
            title={`Engine: ${state.kind === "ready" ? state.snapshot.session : "connecting"}`}
          >
            <span className="sr-only">Engine </span>
            {state.kind === "ready" && state.snapshot.session === "Running"
              ? "Ready"
              : "Connecting"}
          </span>
        </div>
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  );
}
