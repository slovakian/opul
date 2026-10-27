import type { Location } from "@/components/workspace-sidebar";
import type { Preferences, Workspace } from "@/lib/model";

export interface Destination {
  workspace: Workspace;
  folder: Workspace["folders"][number];
}

export const workspaceDestination = (
  workspace?: Workspace,
  folderId?: string
): Destination | undefined => {
  const folder =
    workspace?.folders.find((item) => item.id === folderId) ??
    workspace?.folders.find((item) => item.is_default);

  return workspace && folder ? { folder, workspace } : undefined;
};

export const naturalDestination = (
  workspaces: Workspace[],
  location: Location,
  preferences: Preferences | null
) => {
  if (location.kind === "workspace") {
    return workspaceDestination(
      workspaces.find((item) => item.id === location.workspaceId),
      location.folderId
    );
  }

  if (workspaces.length === 1) {
    return workspaceDestination(workspaces[0]);
  }

  const automatic = preferences?.aggregate_destination;

  return automatic?.kind === "workspace"
    ? workspaceDestination(
        workspaces.find((item) => item.id === automatic.workspace_id)
      )
    : undefined;
};

export const recommendedWorkspace = (
  workspaces: Workspace[],
  preferences: Preferences | null,
  lastUsed?: string
) => {
  const automatic = preferences?.aggregate_destination;

  const configured =
    automatic?.kind === "workspace" ? automatic.workspace_id : undefined;

  return (
    workspaces.find((item) => item.id === configured) ??
    workspaces.find((item) => item.id === lastUsed) ??
    workspaces[0]
  );
};
