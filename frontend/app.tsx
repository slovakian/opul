import { useEffect, useRef, useState } from "react";
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
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbList,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Separator } from "@/components/ui/separator";
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { Skeleton } from "@/components/ui/skeleton";
import { DownloadInput } from "@/components/download-input";
import { DownloadToolbar } from "@/components/download-toolbar";
import { AddDownloadDialog } from "@/components/add-download-dialog";
import { DownloadInspector } from "@/components/download-inspector";
import { Downloads } from "@/components/downloads";
import { SettingsDialog } from "@/components/settings-dialog";
import { SidebarPopover } from "@/components/sidebar-popover";
import { WorkspaceIconEditor } from "@/components/workspace-icon-picker";
import { WorkspaceDialog } from "@/components/workspace-dialog";
import {
  WorkspaceSidebar,
  type Location,
  type StatusFilter,
} from "@/components/workspace-sidebar";
import { WorkspaceIcon } from "@/components/workspace-icon";
import { useLibrary } from "@/lib/library";
import { cn } from "@/lib/utils";
import {
  sourceNames,
  stateNames,
  type Download,
  type Workspace,
} from "@/lib/model";

type Modal =
  | { kind: "icon"; workspace: Workspace; anchor: HTMLButtonElement | null }
  | { kind: "add"; url?: string; folderId?: string }
  | { kind: "settings" }
  | { kind: "workspace"; workspace: Workspace | null }
  | { kind: "remove"; download: Download }
  | { kind: "delete-workspace"; workspace: Workspace };

export default function App() {
  const { state, busy, refresh, run } = useLibrary();
  useEffect(() => {
    if (state.kind !== "loading" && isTauri())
      void invoke("frontend_ready").catch(() => {});
  }, [state.kind]);
  const [location, setLocation] = useState<Location>({ kind: "all" });
  const [filter, setFilter] = useState<StatusFilter>("all");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string>();
  const [modal, setModal] = useState<Modal | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const workspaces = state.kind === "ready" ? state.snapshot.workspaces : [];
  const workspace =
    (location.kind === "workspace"
      ? workspaces.find((workspace) => workspace.id === location.workspaceId)
      : undefined) ?? workspaces[0];
  const folderId =
    location.kind === "workspace" ? location.folderId : undefined;
  const folder = workspace?.folders.find((folder) => folder.id === folderId);
  const selectedWorkspace = workspaces.find((workspace) =>
    workspace.downloads.some((download) => download.id === selectedId),
  );
  const selected = selectedWorkspace?.downloads.find(
    (download) => download.id === selectedId,
  );
  const canEdit =
    state.kind === "ready" && state.snapshot.session === "Running";
  const title =
    location.kind === "all"
      ? filter === "active"
        ? "In progress"
        : filter === "finished"
          ? "Completed"
          : "All downloads"
      : (folder?.name ?? workspace?.name ?? "Downloads");
  const downloads = (
    location.kind === "all"
      ? workspaces.flatMap((workspace) => workspace.downloads)
      : (workspace?.downloads ?? [])
  ).filter((download) => {
    if (folderId && download.folder_id !== folderId) return false;
    if (
      !`${download.name} ${download.source_url}`
        .toLowerCase()
        .includes(query.toLowerCase())
    )
      return false;
    if (filter === "active")
      return download.capabilities.can_pause || download.state === "Pausing";
    if (filter === "finished") return download.state === "Finished";
    if (filter === "paused") return download.state === "Paused";
    if (filter === "failed") return download.state === "Failed";
    return true;
  });
  if (workspace?.sort === "Name")
    downloads.sort((a, b) => a.name.localeCompare(b.name));
  else if (workspace?.sort === "State")
    downloads.sort((a, b) => a.state.localeCompare(b.state));
  else if (workspace?.sort === "Source")
    downloads.sort((a, b) => a.source.localeCompare(b.source));
  const groups = new Map<string, Download[]>();
  for (const download of downloads) {
    const group =
      workspace?.grouping === "State"
        ? stateNames[download.state]
        : workspace?.grouping === "Source"
          ? sourceNames[download.source]
          : workspace?.grouping === "Kind"
            ? (download.name.split(".").pop()?.toUpperCase() ?? "Other")
            : workspace?.grouping === "Folder"
              ? (workspaces
                  .flatMap((workspace) => workspace.folders)
                  .find((folder) => folder.id === download.folder_id)?.name ??
                "Other")
              : "";
    const existing = groups.get(group);
    if (existing) existing.push(download);
    else groups.set(group, [download]);
  }
  function navigate(next: Location) {
    setLocation(next);
    setQuery("");
    setSelectedId(undefined);
  }
  function view(
    change: Partial<Pick<Workspace, "layout" | "sort" | "grouping">>,
  ) {
    if (workspace)
      void run({
        kind: "view",
        workspace_id: workspace.id,
        layout: workspace.layout,
        sort: workspace.sort,
        grouping: workspace.grouping,
        ...change,
      });
  }
  useEffect(() => {
    function keydown(event: KeyboardEvent) {
      if (!(event.metaKey || event.ctrlKey)) return;
      if (event.key === "n") {
        event.preventDefault();
        setModal({ kind: "add" });
      }
      if (event.key === ",") {
        event.preventDefault();
        setModal({ kind: "settings" });
      }
      if (event.key === "f") {
        event.preventDefault();
        searchRef.current?.focus();
      }
    }
    window.addEventListener("keydown", keydown);
    if (!isTauri()) return () => window.removeEventListener("keydown", keydown);
    const subscriptions = [
      listen("settings", () => setModal({ kind: "settings" })),
      listen("new-download", () => setModal({ kind: "add" })),
    ];
    return () => {
      window.removeEventListener("keydown", keydown);
      for (const subscription of subscriptions)
        void subscription.then((unlisten) => unlisten());
    };
  }, []);
  return (
    <SidebarProvider className="h-svh min-h-0 overflow-hidden">
      <WorkspaceSidebar
        workspaces={workspaces}
        location={location}
        filter={filter}
        navigate={navigate}
        setFilter={setFilter}
        onCreate={() => setModal({ kind: "workspace", workspace: null })}
        onEdit={(workspace) => setModal({ kind: "workspace", workspace })}
        onChangeIcon={(workspace, anchor) =>
          setModal({ kind: "icon", workspace, anchor })
        }
        onSettings={() => setModal({ kind: "settings" })}
        onNew={() => setModal({ kind: "add" })}
      />
      <SidebarInset className="h-svh min-w-0 overflow-hidden">
        <header className="shrink-0">
          <div
            data-tauri-drag-region
            className="flex min-h-14 flex-wrap items-center gap-x-2 gap-y-2 px-4 py-3"
          >
            <div className="flex min-w-0 flex-1 items-center gap-2 sm:max-w-56 sm:flex-initial">
              <SidebarTrigger />
              <Separator
                orientation="vertical"
                className="mx-1 data-vertical:h-4 data-vertical:self-center"
              />
              <Breadcrumb className="min-w-0">
                <BreadcrumbList className="flex-nowrap">
                  {folder && workspace && (
                    <>
                      <BreadcrumbItem className="min-w-0">
                        <span className="truncate">{workspace.name}</span>
                      </BreadcrumbItem>
                      <BreadcrumbSeparator />
                    </>
                  )}
                  <BreadcrumbItem className="min-w-0">
                    <h1
                      aria-label={title}
                      aria-current="page"
                      className="flex min-w-0 items-center gap-2 text-sm font-medium text-foreground"
                    >
                      {location.kind === "workspace" && workspace && (
                        <span aria-hidden>
                          <WorkspaceIcon workspace={workspace} />
                        </span>
                      )}
                      <span className="truncate">{title}</span>
                    </h1>
                  </BreadcrumbItem>
                </BreadcrumbList>
              </Breadcrumb>
            </div>
            <DownloadToolbar
              searchRef={searchRef}
              query={query}
              setQuery={setQuery}
              filter={filter}
              setFilter={setFilter}
              workspace={workspace}
              view={view}
              disabled={!canEdit || busy}
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
                      label: "New download",
                      icon: Plus,
                      onClick: () => setModal({ kind: "add" }),
                      disabled: !canEdit,
                    },
                    {
                      label: "Settings",
                      icon: Settings2,
                      onClick: () => setModal({ kind: "settings" }),
                    },
                  ],
                },
                ...(location.kind === "workspace" && workspace
                  ? [
                      {
                        actions: [
                          {
                            label: "Change icon…",
                            icon: Smile,
                            disabled: !canEdit || busy,
                            onClick: (anchor: HTMLButtonElement | null) =>
                              setModal({ kind: "icon", workspace, anchor }),
                          },
                          {
                            label: "Edit workspace",
                            icon: FolderOpen,
                            onClick: () =>
                              setModal({ kind: "workspace", workspace }),
                          },
                          {
                            label: "Delete workspace",
                            icon: Trash2,
                            destructive: true,
                            disabled: workspaces.length === 1 || !canEdit,
                            onClick: () =>
                              setModal({ kind: "delete-workspace", workspace }),
                          },
                        ],
                      },
                    ]
                  : []),
              ]}
            />
          </div>
          {workspace && (
            <DownloadInput
              key={workspace.id}
              workspace={workspace}
              disabled={!canEdit || busy}
              onOptions={(url, folderId) =>
                setModal({ kind: "add", url, folderId })
              }
            />
          )}
        </header>
        <div
          className={cn(
            "flex min-h-0 flex-1 flex-col overflow-y-auto",
            workspace?.layout !== "Table" && "scroll-fade",
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
                      <h2 className="px-3 text-xs font-medium text-muted-foreground">
                        {label} · {downloads.length}
                      </h2>
                    )}
                    <Downloads
                      downloads={downloads}
                      layout={workspace?.layout ?? "List"}
                      selectedId={selectedId}
                      onInspect={(download) => setSelectedId(download.id)}
                      onRemove={(download) =>
                        setModal({ kind: "remove", download })
                      }
                      onNew={() => setModal({ kind: "add" })}
                      filtered={Boolean(query) || filter !== "all"}
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
                onNew={() => setModal({ kind: "add" })}
                filtered={Boolean(query) || filter !== "all"}
              />
            )}
            {downloads.length > 0 && (
              <p className="px-3 text-xs text-muted-foreground">
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
          onRemove={() => setModal({ kind: "remove", download: selected })}
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
          initialWorkspace={workspace}
          initialFolder={modal.folderId ?? folderId}
          initialUrl={modal.url}
          onClose={() => setModal(null)}
        />
      )}
      {(modal?.kind === "remove" || modal?.kind === "delete-workspace") && (
        <Dialog
          open
          onOpenChange={(open) => {
            if (!open) setModal(null);
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
                          kind: "remove_downloads",
                          download_ids: [modal.download.id],
                        }
                      : {
                          kind: "delete_workspace",
                          workspace_id: modal.workspace.id,
                        },
                  );
                  if (receipt) {
                    if (modal.kind === "delete-workspace")
                      navigate({ kind: "all" });
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
    </SidebarProvider>
  );
}
