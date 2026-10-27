import {
  FolderOpen,
  MoreHorizontal,
  Plus,
  Settings2,
  Smile,
  Trash2,
} from "lucide-react";
import type { RefObject } from "react";

import type { Modal } from "@/components/app-modals";
import { DownloadInput } from "@/components/download-input";
import { DownloadToolbar } from "@/components/download-toolbar";
import { SidebarPopover } from "@/components/sidebar-popover";
import { Button } from "@/components/ui/button";
import { WorkspaceHeader } from "@/components/workspace-header";
import type { Location, StatusFilter } from "@/components/workspace-sidebar";
import type { useLibrary } from "@/lib/library";
import type { DisplayPreferences, Workspace } from "@/lib/model";

interface WorkspaceActionsProps {
  busy: boolean;
  canEdit: boolean;
  location: Location;
  newDownload: () => void;
  setModal: (modal: Modal | null) => void;
  workspace: Workspace | undefined;
  workspacesCount: number;
}

const WorkspaceActions = ({
  busy,
  canEdit,
  location,
  newDownload,
  setModal,
  workspace,
  workspacesCount,
}: WorkspaceActionsProps) => {
  const workspaceGroup =
    location.kind === "workspace" && workspace
      ? [
          {
            actions: [
              {
                disabled: !canEdit || busy,
                icon: Smile,
                label: "Change icon…",
                onClick: (anchor: HTMLButtonElement | null) =>
                  setModal({ anchor, kind: "icon", workspace }),
              },
              {
                icon: FolderOpen,
                label: "Edit workspace",
                onClick: () => setModal({ kind: "workspace", workspace }),
              },
              {
                destructive: true,
                disabled: workspacesCount === 1 || !canEdit,
                icon: Trash2,
                label: "Delete workspace",
                onClick: () =>
                  setModal({ kind: "delete-workspace", workspace }),
              },
            ],
          },
        ]
      : [];

  return (
    <SidebarPopover
      title="Workspace actions"
      trigger={
        <Button variant="ghost" size="icon-sm" aria-label="Workspace actions">
          <MoreHorizontal />
        </Button>
      }
      groups={[
        {
          actions: [
            {
              disabled: !canEdit,
              icon: Plus,
              label: "New download",
              onClick: newDownload,
            },
            {
              icon: Settings2,
              label: "Settings",
              onClick: () => setModal({ kind: "settings" }),
            },
          ],
        },
        ...workspaceGroup,
      ]}
    />
  );
};

export interface AppHeaderProps {
  busy: boolean;
  canEdit: boolean;
  display: DisplayPreferences;
  filter: StatusFilter;
  folder: Workspace["folders"][number] | undefined;
  lastDestination: string | undefined;
  location: Location;
  navigation: number;
  newDownload: () => void;
  onNavigateWorkspace: (id: string) => void;
  onOpenAddModal: (
    url?: string,
    folderId?: string,
    workspaceId?: string
  ) => void;
  onSetLastDestination: (dest: string | undefined) => void;
  onViewChange: (change: Partial<DisplayPreferences>) => void;
  preferences: ReturnType<typeof useLibrary>["preferences"];
  query: string;
  searchRef: RefObject<HTMLInputElement | null>;
  setFilter: (filter: StatusFilter) => void;
  setModal: (modal: Modal | null) => void;
  setQuery: (query: string) => void;
  title: string;
  workspace: Workspace | undefined;
  workspaces: Workspace[];
}

export const AppHeader = ({
  busy,
  canEdit,
  display,
  filter,
  folder,
  lastDestination,
  location,
  navigation,
  newDownload,
  onNavigateWorkspace,
  onOpenAddModal,
  onSetLastDestination,
  onViewChange,
  preferences,
  query,
  searchRef,
  setFilter,
  setModal,
  setQuery,
  title,
  workspace,
  workspaces,
}: AppHeaderProps) => (
  <WorkspaceHeader
    title={title}
    workspace={location.kind === "workspace" ? workspace : undefined}
    parentWorkspace={folder ? workspace : undefined}
    onParent={() => {
      if (workspace) {
        onNavigateWorkspace(workspace.id);
      }
    }}
  >
    {workspace && (
      <DownloadInput
        key={navigation}
        workspaces={workspaces}
        location={location}
        lastUsed={lastDestination}
        onUsed={onSetLastDestination}
        disabled={!canEdit || busy}
        query={query}
        setQuery={setQuery}
        inputRef={searchRef}
        onOptions={onOpenAddModal}
      />
    )}
    <DownloadToolbar
      filter={filter}
      setFilter={setFilter}
      display={display}
      showFilter={location.kind === "all" || location.kind === "workspace"}
      view={onViewChange}
      disabled={
        !canEdit || busy || (location.kind !== "workspace" && !preferences)
      }
    />
    <WorkspaceActions
      busy={busy}
      canEdit={canEdit}
      location={location}
      newDownload={newDownload}
      setModal={setModal}
      workspace={workspace}
      workspacesCount={workspaces.length}
    />
  </WorkspaceHeader>
);
