import { useMemo } from "react";

import type { Location, StatusFilter } from "@/components/workspace-sidebar";
import type { useLibrary } from "@/lib/library";
import { useLibrarySelector } from "@/lib/library";
import { selectDownloads, selectWorkspaces } from "@/lib/library-store";
import type { LibraryDownload } from "@/lib/library-store";
import {
  aggregateNames,
  defaultDisplay,
  sourceNames,
  stateNames,
  completeStates,
} from "@/lib/model";
import type { DisplayPreferences, Download, Workspace } from "@/lib/model";

interface FilterAndSortOptions<T extends Download> {
  downloads: readonly T[];
  effectiveFilter: StatusFilter;
  folderId?: string;
  query: string;
  sort: DisplayPreferences["sort"];
}

export const filterAndSortDownloads = <T extends Download>({
  downloads,
  effectiveFilter,
  folderId,
  query,
  sort,
}: FilterAndSortOptions<T>): T[] => {
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
      return (
        download.capabilities.can_pause ||
        download.state === "Pausing" ||
        download.state === "WaitingForVideoTools"
      );
    }

    if (effectiveFilter === "finished") {
      return completeStates.has(download.state);
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

export const groupDownloads = <T extends Download>(
  downloads: readonly T[],
  grouping: DisplayPreferences["grouping"],
  workspaces: readonly Workspace[]
): Map<string, T[]> => {
  const groups = new Map<string, T[]>();

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
  title: string;
  workspace: Workspace | undefined;
}

export const normalizeWorkspaceLocation = (
  location: Location,
  workspaces: readonly Workspace[]
): Location => {
  if (location.kind !== "workspace" || !location.folderId) {
    return location;
  }

  const workspace = workspaces.find((item) => item.id === location.workspaceId);

  if (
    workspace &&
    (workspace.folders.length === 1 ||
      !workspace.folders.some((item) => item.id === location.folderId))
  ) {
    return { kind: "workspace", workspaceId: location.workspaceId };
  }

  return location;
};

export const useWorkspaceView = ({
  filter,
  location: selectedLocation,
  preferences,
  workspaces,
}: {
  filter: StatusFilter;
  location: Location;
  preferences: ReturnType<typeof useLibrary>["preferences"];
  workspaces: readonly Workspace[];
}): WorkspaceViewState => {
  const location = normalizeWorkspaceLocation(selectedLocation, workspaces);

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

  return {
    display,
    effectiveFilter,
    folder,
    folderId,
    title,
    workspace,
  };
};

export interface VisibleDownloadsOptions {
  display: DisplayPreferences;
  effectiveFilter: StatusFilter;
  folderId?: string;
  query: string;
  /** The workspace being viewed, or undefined for every workspace. */
  workspaceId?: string;
}

/** The downloads a view shows, grouped as its display preferences ask. */
export const useVisibleDownloads = ({
  display,
  effectiveFilter,
  folderId,
  query,
  workspaceId,
}: VisibleDownloadsOptions) => {
  const downloads = useLibrarySelector(selectDownloads);
  const workspaces = useLibrarySelector(selectWorkspaces);

  return useMemo(() => {
    const visible = filterAndSortDownloads({
      downloads: workspaceId
        ? downloads.filter((download) => download.workspace_id === workspaceId)
        : downloads,
      effectiveFilter,
      folderId,
      query,
      sort: display.sort,
    });

    return {
      count: visible.length,
      groups: groupDownloads<LibraryDownload>(
        visible,
        display.grouping,
        workspaces
      ),
    };
  }, [
    display.grouping,
    display.sort,
    downloads,
    effectiveFilter,
    folderId,
    query,
    workspaceId,
    workspaces,
  ]);
};
