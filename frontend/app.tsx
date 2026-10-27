import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  CircleAlert,
  FolderOpen,
  MoreHorizontal,
  Plus,
  Settings2,
  Smile,
  Trash2,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

import { AddDownloadDialog } from "@/components/add-download-dialog";
import { DownloadInput } from "@/components/download-input";
import { DownloadInspector } from "@/components/download-inspector";
import { DownloadToolbar } from "@/components/download-toolbar";
import { Downloads } from "@/components/downloads";
import { SettingsDialog } from "@/components/settings-dialog";
import { SidebarPopover } from "@/components/sidebar-popover";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { SidebarInset, useSidebar } from "@/components/ui/sidebar";
import { Skeleton } from "@/components/ui/skeleton";
import { WorkspaceDialog } from "@/components/workspace-dialog";
import { WorkspaceHeader } from "@/components/workspace-header";
import { WorkspaceIconEditor } from "@/components/workspace-icon-picker";
import { WorkspaceSidebar } from "@/components/workspace-sidebar";
import type { Location, StatusFilter } from "@/components/workspace-sidebar";
import { useLibrary } from "@/lib/library";
import {
  aggregateNames,
  defaultDisplay,
  sourceNames,
  stateNames,
} from "@/lib/model";
import type { DisplayPreferences, Download, Workspace } from "@/lib/model";
import { cn } from "@/lib/utils";

type Modal =
  | { kind: "icon"; workspace: Workspace; anchor: HTMLButtonElement | null }
  | { kind: "add"; url?: string; folderId?: string; workspaceId: string }
  | { kind: "settings" }
  | { kind: "workspace"; workspace: Workspace | null }
  | { kind: "remove"; download: Download }
  | { kind: "delete-workspace"; workspace: Workspace };

export default function App() {
  const { state, busy, preferences, refresh, run, saveAggregateView } =
    useLibrary();

  useEffect(() => {
    if (state.kind !== "loading" && isTauri()) {
      void invoke("frontend_ready").catch(() => {});
    }
  }, [state.kind]);
  const [location, setLocation] = useState<Location>({ kind: "all" });
  const [navigation, setNavigation] = useState(0);
  const [lastDestination, setLastDestination] = useState<string>();
  const [filter, setFilter] = useState<StatusFilter>("all");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string>();
  const [modal, setModal] = useState<Modal | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const { setOpenMobile } = useSidebar();
  const workspaces = state.kind === "ready" ? state.snapshot.workspaces : [];

  const workspace =
    (location.kind === "workspace"
      ? workspaces.find((workspace) => workspace.id === location.workspaceId)
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

  const folder = workspace?.folders.find((folder) => folder.id === folderId);

  const selectedWorkspace = workspaces.find((workspace) =>
    workspace.downloads.some((download) => download.id === selectedId)
  );

  const selected = selectedWorkspace?.downloads.find(
    (download) => download.id === selectedId
  );

  const canEdit =
    state.kind === "ready" && state.snapshot.session === "Running";

  const title =
    location.kind === "workspace"
      ? (folder?.name ?? workspace?.name ?? "Downloads")
      : aggregateNames[location.kind];

  const downloads = (
    location.kind === "workspace"
      ? (workspace?.downloads ?? [])
      : workspaces.flatMap((workspace) => workspace.downloads)
  ).filter((download) => {
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

  if (display.sort === "Name") {
    downloads.sort((a, b) => a.name.localeCompare(b.name));
  } else if (display.sort === "State") {
    downloads.sort((a, b) => a.state.localeCompare(b.state));
  } else if (display.sort === "Source") {
    downloads.sort((a, b) => a.source.localeCompare(b.source));
  }

  const groups = new Map<string, Download[]>();

  for (const download of downloads) {
    const group =
      display.grouping === "State"
        ? stateNames[download.state]
        : display.grouping === "Source"
          ? sourceNames[download.source]
          : display.grouping === "Kind"
            ? (download.name.split(".").pop()?.toUpperCase() ?? "Other")
            : display.grouping === "Folder"
              ? (workspaces
                  .flatMap((workspace) => workspace.folders)
                  .find((folder) => folder.id === download.folder_id)?.name ??
                "Other")
              : "";

    const existing = groups.get(group);

    if (existing) {
      existing.push(download);
    } else {
      groups.set(group, [download]);
    }
  }

  function navigate(next: Location) {
    setLocation(next);
    setNavigation((count) => count + 1);
    setFilter("all");
    setSelectedId(undefined);
  }

  function view(change: Partial<DisplayPreferences>) {
    if (location.kind !== "workspace") {
      void saveAggregateView(location.kind, { ...display, ...change });
    } else if (workspace) {
      void run({
        grouping: workspace.grouping,
        kind: "view",
        layout: workspace.layout,
        sort: workspace.sort,
        workspace_id: workspace.id,
        ...change,
      });
    }
  }

  const focusInput = useCallback(
    (select: boolean) => {
      setOpenMobile(false);
      // Let a closing menu restore focus before moving it to the shared input.
      requestAnimationFrame(() => {
        searchRef.current?.focus();

        if (select) {
          searchRef.current?.select();
        }
      });
    },
    [setOpenMobile]
  );

  const newDownload = useCallback(() => focusInput(true), [focusInput]);

  useEffect(() => {
    function keydown(event: KeyboardEvent) {
      if (!(event.metaKey || event.ctrlKey)) {
        return;
      }

      if (event.key === "n") {
        event.preventDefault();
        newDownload();
      }

      if (event.key === ",") {
        event.preventDefault();
        setModal({ kind: "settings" });
      }

      if (event.key === "f") {
        event.preventDefault();
        focusInput(false);
      }
    }

    window.addEventListener("keydown", keydown);

    if (!isTauri()) {
      return () => window.removeEventListener("keydown", keydown);
    }

    const subscriptions = [
      listen("settings", () => setModal({ kind: "settings" })),
      listen("new-download", () => {
        newDownload();
      }),
    ];

    return () => {
      window.removeEventListener("keydown", keydown);

      for (const subscription of subscriptions) {
        void subscription.then((unlisten) => unlisten());
      }
    };
  }, [focusInput, newDownload]);

  return (
    <>
      <WorkspaceSidebar
        workspaces={workspaces}
        location={location}
        navigate={navigate}
        onCreate={() => setModal({ kind: "workspace", workspace: null })}
        onEdit={(workspace) => setModal({ kind: "workspace", workspace })}
        onChangeIcon={(workspace, anchor) =>
          setModal({ anchor, kind: "icon", workspace })
        }
        onSettings={() => setModal({ kind: "settings" })}
        onNew={newDownload}
      />
      <SidebarInset className="h-svh min-w-0 overflow-hidden">
        <WorkspaceHeader
          title={title}
          workspace={location.kind === "workspace" ? workspace : undefined}
          parentWorkspace={folder ? workspace : undefined}
          onParent={() => {
            if (workspace) {
              navigate({ kind: "workspace", workspaceId: workspace.id });
            }
          }}
        >
          {workspace && (
            <DownloadInput
              key={navigation}
              workspaces={workspaces}
              location={location}
              lastUsed={lastDestination}
              onUsed={setLastDestination}
              disabled={!canEdit || busy}
              query={query}
              setQuery={setQuery}
              inputRef={searchRef}
              onOptions={(url, folderId, workspaceId) =>
                setModal({ folderId, kind: "add", url, workspaceId })
              }
            />
          )}
          <DownloadToolbar
            filter={filter}
            setFilter={setFilter}
            display={display}
            showFilter={
              location.kind === "all" || location.kind === "workspace"
            }
            view={view}
            disabled={
              !canEdit ||
              busy ||
              (location.kind !== "workspace" && !preferences)
            }
          />
          <SidebarPopover
            title="Workspace actions"
            trigger={
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label="Workspace actions"
              >
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
              ...(location.kind === "workspace" && workspace
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
                          onClick: () =>
                            setModal({ kind: "workspace", workspace }),
                        },
                        {
                          destructive: true,
                          disabled: workspaces.length === 1 || !canEdit,
                          icon: Trash2,
                          label: "Delete workspace",
                          onClick: () =>
                            setModal({ kind: "delete-workspace", workspace }),
                        },
                      ],
                    },
                  ]
                : []),
            ]}
          />
        </WorkspaceHeader>
        <div
          className={cn(
            "flex min-h-0 flex-1 flex-col overflow-y-auto",
            display.layout !== "Table" && "scroll-fade"
          )}
        >
          <div className="mx-auto flex w-full max-w-6xl flex-col gap-6 px-6 py-6 lg:px-10">
            {state.kind === "ready" && state.snapshot.session !== "Running" && (
              <Alert>
                <CircleAlert />
                <AlertTitle>
                  {state.snapshot.session === "RecoveryRequired"
                    ? "Your library needs attention"
                    : "Download engine is unavailable"}
                </AlertTitle>
                <AlertDescription>
                  <p>
                    {state.snapshot.recovery_notices.join(" ") ||
                      `Engine status: ${state.snapshot.session}`}
                  </p>
                  {state.snapshot.session === "RecoveryRequired" && (
                    <Button
                      variant="outline"
                      size="sm"
                      disabled={busy}
                      onClick={() => void run({ kind: "acknowledge_recovery" })}
                    >
                      Acknowledge recovery
                    </Button>
                  )}
                </AlertDescription>
              </Alert>
            )}
            {state.kind === "loading" ? (
              <div
                className="flex flex-col gap-4"
                aria-label="Loading downloads"
              >
                {[0, 1, 2].map((key) => (
                  <Skeleton key={key} className="h-16 w-full" />
                ))}
              </div>
            ) : state.kind === "failed" ? (
              <Alert>
                <CircleAlert />
                <AlertTitle>Couldn't open your library</AlertTitle>
                <AlertDescription>
                  <p>{state.message}</p>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => void refresh()}
                  >
                    Try again
                  </Button>
                </AlertDescription>
              </Alert>
            ) : groups.size ? (
              <div className="flex flex-col gap-7">
                {Array.from(groups, ([label, downloads]) => (
                  <section key={label} className="flex flex-col gap-2">
                    {label && (
                      <h2 className="text-muted-foreground px-3 text-xs font-medium">
                        {label} · {downloads.length}
                      </h2>
                    )}
                    <Downloads
                      downloads={downloads}
                      layout={display.layout}
                      selectedId={selectedId}
                      onInspect={(download) => setSelectedId(download.id)}
                      onRemove={(download) =>
                        setModal({ download, kind: "remove" })
                      }
                      onNew={newDownload}
                      filtered={Boolean(query) || effectiveFilter !== "all"}
                    />
                  </section>
                ))}
              </div>
            ) : (
              <Downloads
                downloads={[]}
                layout="List"
                onInspect={() => {}}
                onRemove={() => {}}
                onNew={newDownload}
                filtered={Boolean(query) || effectiveFilter !== "all"}
              />
            )}
            {downloads.length > 0 && (
              <p className="text-muted-foreground px-3 text-xs">
                {downloads.length}{" "}
                {downloads.length === 1 ? "download" : "downloads"}
              </p>
            )}
          </div>
        </div>
      </SidebarInset>
      {selected && selectedWorkspace && (
        <DownloadInspector
          key={selected.id}
          download={selected}
          workspace={selectedWorkspace}
          onClose={() => setSelectedId(undefined)}
          onRemove={() => setModal({ download: selected, kind: "remove" })}
        />
      )}
      {modal?.kind === "settings" && (
        <SettingsDialog onClose={() => setModal(null)} />
      )}
      {modal?.kind === "icon" && (
        <WorkspaceIconEditor
          workspace={modal.workspace}
          anchor={modal.anchor}
          onClose={() => setModal(null)}
        />
      )}
      {modal?.kind === "workspace" && (
        <WorkspaceDialog
          workspace={modal.workspace}
          onClose={() => setModal(null)}
          onCreated={(workspaceId) =>
            navigate({ kind: "workspace", workspaceId })
          }
        />
      )}
      {modal?.kind === "add" && workspace && (
        <AddDownloadDialog
          workspaces={workspaces}
          initialWorkspace={
            workspaces.find((item) => item.id === modal.workspaceId) ??
            workspace
          }
          initialFolder={modal.folderId ?? folderId}
          initialUrl={modal.url}
          onClose={() => setModal(null)}
        />
      )}
      {(modal?.kind === "remove" || modal?.kind === "delete-workspace") && (
        <Dialog
          open
          onOpenChange={(open) => {
            if (!open) {
              setModal(null);
            }
          }}
        >
          <DialogContent>
            <DialogHeader>
              <DialogTitle>
                {modal.kind === "remove"
                  ? "Remove this download?"
                  : "Delete this workspace?"}
              </DialogTitle>
              <DialogDescription>
                {modal.kind === "remove"
                  ? `Remove “${modal.download.name}” from your library? Any active transfer will stop. Your downloaded files stay on disk.`
                  : `Delete “${modal.workspace.name}” and its download history? Downloaded files stay on disk.`}
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="ghost" onClick={() => setModal(null)}>
                Keep it
              </Button>
              <Button
                variant="destructive"
                disabled={busy}
                onClick={async () => {
                  const receipt = await run(
                    modal.kind === "remove"
                      ? {
                          download_ids: [modal.download.id],
                          kind: "remove_downloads",
                        }
                      : {
                          kind: "delete_workspace",
                          workspace_id: modal.workspace.id,
                        }
                  );

                  if (receipt) {
                    if (modal.kind === "delete-workspace") {
                      navigate({ kind: "all" });
                    }

                    setModal(null);
                  }
                }}
              >
                {busy
                  ? "Removing…"
                  : modal.kind === "remove"
                    ? "Remove download"
                    : "Delete workspace"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      )}
    </>
  );
}
