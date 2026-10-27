import type { Location, StatusFilter } from "@/components/workspace-sidebar";
import type { useLibrary } from "@/lib/library";
import {
  aggregateNames,
  defaultDisplay,
  sourceNames,
  stateNames,
} from "@/lib/model";
import type { DisplayPreferences, Download, Workspace } from "@/lib/model";

interface FilterAndSortOptions {
  downloads: Download[];
  effectiveFilter: StatusFilter;
  folderId?: string;
  query: string;
  sort: DisplayPreferences["sort"];
}

export const filterAndSortDownloads = ({
  downloads,
  effectiveFilter,
  folderId,
  query,
  sort,
}: FilterAndSortOptions): Download[] => {
  const result = downloads.filter((download) => {
    if (folderId && download.folder_id !== folderId) {
      return false;
    }

    if (
      !`${download.name} ${download.source_url}`
        .toLowerCase()
        .includes(query.toLowerCase())
    ) {
      return false;
    }

    if (effectiveFilter === "active") {
      return download.capabilities.can_pause || download.state === "Pausing";
    }

    if (effectiveFilter === "finished") {
      return download.state === "Finished";
    }

    if (effectiveFilter === "paused") {
      return download.state === "Paused";
    }

    if (effectiveFilter === "failed") {
      return download.state === "Failed";
    }

    return true;
  });

  if (sort === "Name") {
    result.sort((a, b) => a.name.localeCompare(b.name));
  } else if (sort === "State") {
    result.sort((a, b) => a.state.localeCompare(b.state));
  } else if (sort === "Source") {
    result.sort((a, b) => a.source.localeCompare(b.source));
  }

  return result;
};

export const groupDownloads = (
  downloads: Download[],
  grouping: DisplayPreferences["grouping"],
  workspaces: Workspace[]
): Map<string, Download[]> => {
  const groups = new Map<string, Download[]>();

  for (const download of downloads) {
    let group = "";

    if (grouping === "State") {
      group = stateNames[download.state];
    } else if (grouping === "Source") {
      group = sourceNames[download.source];
    } else if (grouping === "Kind") {
      group =
        download.name.split(".").findLast(Boolean)?.toUpperCase() ?? "Other";
    } else if (grouping === "Folder") {
      group =
        workspaces
          .flatMap((item) => item.folders)
          .find((item) => item.id === download.folder_id)?.name ?? "Other";
    }

    const existing = groups.get(group);

    if (existing) {
      existing.push(download);
    } else {
      groups.set(group, [download]);
    }
  }

  return groups;
};

export interface WorkspaceViewState {
  display: DisplayPreferences;
  effectiveFilter: StatusFilter;
  folder: Workspace["folders"][number] | undefined;
  folderId?: string;
  rawDownloads: Download[];
  title: string;
  workspace: Workspace | undefined;
}

export const useWorkspaceView = ({
  filter,
  location,
  preferences,
  workspaces,
}: {
  filter: StatusFilter;
  location: Location;
  preferences: ReturnType<typeof useLibrary>["preferences"];
  workspaces: Workspace[];
}): WorkspaceViewState => {
  const workspace =
    (location.kind === "workspace"
      ? workspaces.find((item) => item.id === location.workspaceId)
      : undefined) ?? workspaces[0];

  const display =
    location.kind === "workspace"
      ? (workspace ?? defaultDisplay)
      : (preferences?.aggregate_views[location.kind] ?? defaultDisplay);

  const effectiveFilter =
    location.kind === "active" || location.kind === "finished"
      ? location.kind
      : filter;

  const folderId =
    location.kind === "workspace" ? location.folderId : undefined;

  const folder = workspace?.folders.find((item) => item.id === folderId);

  const title =
    location.kind === "workspace"
      ? (folder?.name ?? workspace?.name ?? "Downloads")
      : aggregateNames[location.kind];

  const rawDownloads =
    location.kind === "workspace"
      ? (workspace?.downloads ?? [])
      : workspaces.flatMap((item) => item.downloads);

  return {
    display,
    effectiveFilter,
    folder,
    folderId,
    rawDownloads,
    title,
    workspace,
  };
};
