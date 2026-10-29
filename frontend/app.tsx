import { invoke, isTauri } from "@tauri-apps/api/core";
import { CircleAlert } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

import { AppContent } from "@/components/app-content";
import { AppHeader } from "@/components/app-header";
import { AppModals } from "@/components/app-modals";
import type { Modal } from "@/components/app-modals";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { SidebarInset, useSidebar } from "@/components/ui/sidebar";
import { WorkspaceSidebar } from "@/components/workspace-sidebar";
import type { Location, StatusFilter } from "@/components/workspace-sidebar";
import { useAppKeyboard } from "@/hooks/use-app-keyboard";
import { useDownloadNotifications } from "@/hooks/use-download-notifications";
import {
  filterAndSortDownloads,
  groupDownloads,
  useWorkspaceView,
} from "@/hooks/use-workspace-view";
import { useLibrary } from "@/lib/library";
import type { DisplayPreferences } from "@/lib/model";
import { cn } from "@/lib/utils";

interface EngineAlertProps {
  busy: boolean;
  run: ReturnType<typeof useLibrary>["run"];
  state: ReturnType<typeof useLibrary>["state"];
}

const EngineAlert = ({ busy, run, state }: EngineAlertProps) => {
  if (state.kind !== "ready" || state.snapshot.session === "Running") {
    return null;
  }

  const isRecovery = state.snapshot.session === "RecoveryRequired";

  return (
    <Alert>
      <CircleAlert />
      <AlertTitle>
        {isRecovery
          ? "Your library needs attention"
          : "Download engine is unavailable"}
      </AlertTitle>
      <AlertDescription>
        <p>
          {state.snapshot.recovery_notices.join(" ") ||
            `Engine status: ${state.snapshot.session}`}
        </p>
        {isRecovery && (
          <Button
            variant="outline"
            size="sm"
            disabled={busy}
            onClick={() => {
              run({ kind: "acknowledge_recovery" }).catch(() => null);
            }}
          >
            Acknowledge recovery
          </Button>
        )}
      </AlertDescription>
    </Alert>
  );
};

const App = () => {
  const { busy, preferences, refresh, run, saveAggregateView, state } =
    useLibrary();

  useEffect(() => {
    if (state.kind !== "loading" && isTauri()) {
      invoke("frontend_ready").catch(() => null);
    }
  }, [state.kind]);

  const [location, setLocation] = useState<Location>({ kind: "all" });
  const [navigation, setNavigation] = useState(0);
  const [lastDestination, setLastDestination] = useState<string>();
  const [filter, setFilter] = useState<StatusFilter>("all");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string>();
  const notifyAdded = useDownloadNotifications(setSelectedId);
  const [modal, setModal] = useState<Modal | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const { setOpenMobile } = useSidebar();
  const workspaces = state.kind === "ready" ? state.snapshot.workspaces : [];

  const {
    display,
    effectiveFilter,
    folder,
    folderId,
    rawDownloads,
    title,
    workspace,
  } = useWorkspaceView({
    filter,
    location,
    preferences,
    workspaces,
  });

  const selectedWorkspace = workspaces.find((item) =>
    item.downloads.some((download) => download.id === selectedId)
  );

  const selected = selectedWorkspace?.downloads.find(
    (download) => download.id === selectedId
  );

  const canEdit =
    state.kind === "ready" && state.snapshot.session === "Running";

  const downloads = filterAndSortDownloads({
    downloads: rawDownloads,
    effectiveFilter,
    folderId,
    query,
    sort: display.sort,
  });

  const groups = groupDownloads(downloads, display.grouping, workspaces);

  const navigate = (next: Location) => {
    setLocation(next);
    setNavigation((count) => count + 1);
    setFilter("all");
    setSelectedId(undefined);
  };

  const view = (change: Partial<DisplayPreferences>) => {
    if (location.kind !== "workspace") {
      saveAggregateView(location.kind, { ...display, ...change }).catch(
        () => null
      );
    } else if (workspace) {
      run({
        grouping: workspace.grouping,
        kind: "view",
        layout: workspace.layout,
        sort: workspace.sort,
        workspace_id: workspace.id,
        ...change,
      }).catch(() => null);
    }
  };

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

  useAppKeyboard({ focusInput, newDownload, setModal });

  return (
    <>
      <WorkspaceSidebar
        workspaces={workspaces}
        location={location}
        navigate={navigate}
        onCreate={() => setModal({ kind: "workspace", workspace: null })}
        onEdit={(item) => setModal({ kind: "workspace", workspace: item })}
        onChangeIcon={(item, anchor) =>
          setModal({ anchor, kind: "icon", workspace: item })
        }
        onSettings={() => setModal({ kind: "settings" })}
        onNew={newDownload}
      />
      <SidebarInset className="h-svh min-w-0 overflow-hidden">
        <AppHeader
          busy={busy}
          canEdit={canEdit}
          display={display}
          filter={filter}
          folder={folder}
          lastDestination={lastDestination}
          location={location}
          navigation={navigation}
          newDownload={newDownload}
          onAdded={notifyAdded}
          onNavigateWorkspace={(id) =>
            navigate({ kind: "workspace", workspaceId: id })
          }
          onOpenAddModal={(url, optionFolderId, optionWorkspaceId) =>
            setModal({
              folderId: optionFolderId,
              kind: "add",
              url,
              workspaceId: optionWorkspaceId ?? workspace?.id ?? "",
            })
          }
          onSetLastDestination={setLastDestination}
          onViewChange={view}
          preferences={preferences}
          query={query}
          searchRef={searchRef}
          setFilter={setFilter}
          setModal={setModal}
          setQuery={setQuery}
          title={title}
          workspace={workspace}
          workspaces={workspaces}
        />
        <div
          className={cn(
            "flex min-h-0 flex-1 flex-col overflow-y-auto",
            display.layout !== "Table" && "scroll-fade"
          )}
        >
          <div className="mx-auto flex w-full max-w-6xl flex-col gap-6 px-6 py-6 lg:px-10">
            <EngineAlert busy={busy} run={run} state={state} />
            <AppContent
              display={display}
              effectiveFilter={effectiveFilter}
              groups={groups}
              onInspect={(download) => setSelectedId(download.id)}
              onRemove={(download) => setModal({ download, kind: "remove" })}
              onNew={newDownload}
              query={query}
              refresh={refresh}
              selectedId={selectedId}
              state={state}
            />
            {downloads.length > 0 && (
              <p className="text-muted-foreground px-3 text-xs">
                {downloads.length}{" "}
                {downloads.length === 1 ? "download" : "downloads"}
              </p>
            )}
          </div>
        </div>
      </SidebarInset>
      <AppModals
        busy={busy}
        folderId={folderId}
        modal={modal}
        navigate={navigate}
        onClose={() => setModal(null)}
        onRemoveSelected={() => {
          if (selected) {
            setModal({ download: selected, kind: "remove" });
          }
        }}
        run={run}
        selected={selected}
        selectedWorkspace={selectedWorkspace}
        setSelectedId={setSelectedId}
        workspace={workspace}
        workspaces={workspaces}
      />
    </>
  );
};

export default App;
