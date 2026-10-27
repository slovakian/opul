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

import opulLogo from "@/assets/opul-logo.png";
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
import { toast } from "@/components/ui/toast";
import { WorkspaceIcon } from "@/components/workspace-icon";
import { bridge, errorMessage } from "@/lib/bridge";
import { useLibrary } from "@/lib/library";
import { aggregateNames, bytes } from "@/lib/model";
import type { AggregateView, Folder, Workspace } from "@/lib/model";

export type Location =
  | { kind: AggregateView }
  | { kind: "workspace"; workspaceId: string; folderId?: string };

export type StatusFilter = "all" | "active" | "finished" | "paused" | "failed";

export function WorkspaceSidebar({
  workspaces,
  location,
  navigate,
  onCreate,
  onEdit,
  onChangeIcon,
  onSettings,
  onNew,
}: {
  workspaces: Workspace[];
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
}) {
  const { run, busy, state } = useLibrary();
  const { isMobile, setOpenMobile } = useSidebar();
  const allDownloads = workspaces.flatMap((workspace) => workspace.downloads);

  const active = allDownloads.filter(
    (download) =>
      download.capabilities.can_pause || download.state === "Pausing"
  );

  const downloadRate = allDownloads.reduce(
    (sum, download) => sum + download.download_rate,
    0
  );

  function go(next: Location) {
    navigate(next);

    if (isMobile) {
      setOpenMobile(false);
    }
  }

  async function addFolder(workspace: Workspace) {
    try {
      const path = await bridge.chooseFolder();

      if (!path) {
        return;
      }

      await run({
        kind: "add_folder",
        request: {
          expected_revision: null,
          folder_id: crypto.randomUUID(),
          name: path.split("/").filter(Boolean).pop() || "Folder",
          path,
          request_id: crypto.randomUUID(),
          workspace_id: workspace.id,
        },
      });
    } catch (error) {
      toast.add({
        description: errorMessage(error),
        title: "Couldn't add folder",
        type: "error",
      });
    }
  }

  function folderRequest(workspace: Workspace, folder: Folder) {
    return {
      expected_revision: null,
      folder_id: folder.id,
      name: folder.name,
      path: folder.path,
      request_id: crypto.randomUUID(),
      workspace_id: workspace.id,
    };
  }

  return (
    <Sidebar
      className="border-r-0"
      role="navigation"
      aria-label="Workspace navigation"
    >
      <SidebarHeader className="gap-4 px-3 pt-12 pb-3">
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
                      hint: "⌘N",
                      icon: Plus,
                      label: "New download",
                      onClick: onNew,
                    },
                    {
                      icon: FolderPlus,
                      label: "New workspace",
                      onClick: onCreate,
                    },
                  ],
                },
                {
                  actions: [
                    {
                      hint: "⌘,",
                      icon: Settings2,
                      label: "Settings",
                      onClick: onSettings,
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
              count: allDownloads.length,
              icon: Inbox,
              label: aggregateNames.all,
              status: "all" as const,
            },
            {
              count: active.length,
              icon: ArrowDown,
              label: aggregateNames.active,
              status: "active" as const,
            },
            {
              count: allDownloads.filter(
                (download) => download.state === "Finished"
              ).length,
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
                      className="flex size-4 shrink-0 items-center justify-center group-focus-within/workspace:invisible group-hover/workspace:invisible"
                    >
                      <WorkspaceIcon workspace={workspace} />
                    </span>
                    <span>{workspace.name}</span>
                  </SidebarMenuButton>
                  <CollapsibleTrigger
                    render={
                      <SidebarMenuAction
                        className="right-auto left-1.5 opacity-0 group-focus-within/workspace:opacity-100 group-hover/workspace:opacity-100 data-panel-open:rotate-90"
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
                            disabled:
                              busy ||
                              state.kind !== "ready" ||
                              state.snapshot.session !== "Running",
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
                            onClick: () => void addFolder(workspace),
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
                                actions: [
                                  {
                                    disabled: folder.is_default || busy,
                                    icon: Check,
                                    label: "Set as default",
                                    onClick: () =>
                                      void run({
                                        kind: "default_folder",
                                        request: folderRequest(
                                          workspace,
                                          folder
                                        ),
                                      }),
                                  },
                                  {
                                    destructive: true,
                                    disabled:
                                      workspace.folders.length === 1 || busy,
                                    icon: Trash2,
                                    label: "Remove saved folder",
                                    onClick: () =>
                                      void run({
                                        kind: "remove_folder",
                                        request: folderRequest(
                                          workspace,
                                          folder
                                        ),
                                      }),
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
          className="text-muted-foreground flex items-center gap-2 px-2 pb-1 text-xs"
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
