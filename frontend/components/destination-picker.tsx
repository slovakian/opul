import { ChevronDown, FolderClosed, FolderPlus } from "lucide-react";

import { SidebarPopover } from "@/components/sidebar-popover";
import { InputGroupButton } from "@/components/ui/input-group";
import { WorkspaceIcon } from "@/components/workspace-icon";
import type { Destination } from "@/lib/destination";
import type { Workspace } from "@/lib/model";

export const DestinationPicker = ({
  workspaces,
  destination,
  viewedWorkspaceId,
  disabled,
  onSelect,
  onAdd,
}: {
  workspaces: Workspace[];
  destination?: Destination;
  viewedWorkspaceId?: string;
  disabled: boolean;
  onSelect: (destination: Destination) => void;
  onAdd: () => void;
}) => {
  let label = "Choose…";

  if (destination) {
    label = destination.folder.name;

    if (destination.workspace.id !== viewedWorkspaceId) {
      label = `${destination.workspace.name} › ${destination.folder.name}`;
    }
  }

  return (
    <SidebarPopover
      title="Save location"
      trigger={
        <InputGroupButton
          aria-label={`Save location: ${label}`}
          title={
            destination
              ? `${label}\n${destination.folder.path}`
              : "Choose a download destination"
          }
          disabled={disabled}
        >
          {destination ? (
            <WorkspaceIcon workspace={destination.workspace} />
          ) : (
            <FolderClosed data-icon="inline-start" />
          )}
          <span
            className="header-location max-w-36 truncate"
            data-choose={!destination}
          >
            {label}
          </span>
          <ChevronDown data-icon="inline-end" />
        </InputGroupButton>
      }
      groups={[
        ...workspaces.map((workspace) => ({
          actions: [
            ...workspace.folders.filter((folder) => folder.is_default),
            ...workspace.folders.filter((folder) => !folder.is_default),
          ].map((folder) => ({
            icon: FolderClosed,
            label: `${folder.name}${folder.is_default ? " (Default)" : ""}`,
            onClick: () => onSelect({ folder, workspace }),
            selected:
              workspace.id === destination?.workspace.id &&
              folder.id === destination.folder.id,
          })),
          label: workspace.name,
        })),
        {
          actions: [
            { icon: FolderPlus, label: "Add new location…", onClick: onAdd },
          ],
        },
      ]}
    />
  );
};
