import { invoke, isTauri } from "@tauri-apps/api/core";
import { CircleAlert } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

import { AppContent } from "@/components/app-content";
import { AppHeader } from "@/components/app-header";
import { AppModals } from "@/components/app-modals";
import type { Modal } from "@/components/app-modals";
import { AppUpdateRestartDialog } from "@/components/app-update-restart";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { SidebarInset, useSidebar } from "@/components/ui/sidebar";
import { VideoSetup } from "@/components/video-setup";
import { WorkspaceSidebar } from "@/components/workspace-sidebar";
import type { Location, StatusFilter } from "@/components/workspace-sidebar";
import { useAppKeyboard } from "@/hooks/use-app-keyboard";
import { useAppUpdateNotices } from "@/hooks/use-app-update-notices";
import { useDownloadNotifications } from "@/hooks/use-download-notifications";
import { useSavedFolderAddition } from "@/hooks/use-saved-folder-addition";
import { useVideoSetup } from "@/hooks/use-video-setup";
import { useWorkspaceExpansion } from "@/hooks/use-workspace-expansion";
import {
  normalizeWorkspaceLocation,
  useWorkspaceView,
} from "@/hooks/use-workspace-view";
import { isActiveDownload } from "@/lib/download-activity";
import { useLibrary, useLibrarySelector } from "@/lib/library";
import {
  countDownloads,
  selectSession,
  selectWorkspaces,
} from "@/lib/library-store";
import type { DisplayPreferences, Download } from "@/lib/model";
import { cn } from "@/lib/utils";

const noNotices: readonly string[] = [];

const EngineAlert = () => {
  const { busy, run } = useLibrary();
  const session = useLibrarySelector(selectSession);

  const notices = useLibrarySelector((state) =>
    state.kind === "ready" ? state.library.recovery_notices : noNotices
  );

  if (session === null || session === "Running") {
    return null;
  }

  const isRecovery = session === "RecoveryRequired";

  return (
    <Alert>
      <CircleAlert />
      <AlertTitle>
        {isRecovery
          ? "Your library needs attention"
          : "Download engine is unavailable"}
      </AlertTitle>
      <AlertDescription>
        <p>{notices.join(" ") || `Engine status: ${session}`}</p>
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
  const {
    busy,
    preferences,
    preferenceStatus,
    retryPreferences,
    run,
    saveAggregateView,
  } = useLibrary();

  // App reads only what changes the layout; download changes reach the rows directly.
  const status = useLibrarySelector((state) => state.kind);
  const session = useLibrarySelector(selectSession);
  const workspaces = useLibrarySelector(selectWorkspaces);

  const activeDownloads = useLibrarySelector((state) =>
    countDownloads(state, isActiveDownload)
  );

  useEffect(() => {
    if (status !== "loading" && isTauri()) {
      invoke("frontend_ready").catch(() => null);
    }
  }, [status]);

  const videoSetup = useVideoSetup();

  const [location, setLocation] = useState<Location>({ kind: "all" });

  const workspaceExpansion = useWorkspaceExpansion(
    preferences,
    status === "ready" ? workspaces : null
  );

  const [navigation, setNavigation] = useState(0);
  const [lastDestination, setLastDestination] = useState<string>();
  const [filter, setFilter] = useState<StatusFilter>("all");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string>();
  const notifyAdded = useDownloadNotifications(setSelectedId);
  const [modal, setModal] = useState<Modal | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const { setOpenMobile } = useSidebar();

  const addFolder = useSavedFolderAddition(
    workspaceExpansion.handleExpansionChange
  );

  useAppUpdateNotices(activeDownloads);

  const { display, effectiveFilter, folder, folderId, title, workspace } =
    useWorkspaceView({
      filter,
      location,
      preferences,
      workspaces,
    });

  const canEdit = session === "Running";

  const navigate = useCallback((next: Location) => {
    setLocation(next);
    setNavigation((count) => count + 1);
    setFilter("all");
    setSelectedId(undefined);
  }, []);

  const inspect = useCallback(
    (download: Download) => setSelectedId(download.id),
    []
  );

  const remove = useCallback(
    (download: Download) => setModal({ download, kind: "remove" }),
    []
  );

  const canonicalLocation = normalizeWorkspaceLocation(location, workspaces);

  if (canonicalLocation !== location) {
    navigate(canonicalLocation);
  }

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
        workspaceExpansion={workspaceExpansion.expansion}
        onExpansionChange={workspaceExpansion.handleExpansionChange}
        onAddFolder={(item) => {
          addFolder(
            item.id,
            (newFolderId) => {
              workspaceExpansion.handleExpansionChange(item.id, true);
              navigate({
                folderId: newFolderId,
                kind: "workspace",
                workspaceId: item.id,
              });
              setOpenMobile(false);
            },
            "Couldn't add folder"
          ).catch(() => null);
        }}
        location={location}
        navigate={navigate}
        onCreate={() => setModal({ kind: "workspace", workspace: null })}
        onEdit={(item) => setModal({ kind: "workspace", workspace: item })}
        onChangeIcon={(item, anchor) =>
          setModal({ anchor, kind: "icon", workspace: item })
        }
        onSettings={() => setModal({ kind: "settings" })}
        onNew={newDownload}
        videoTools={videoSetup.tools?.video ?? null}
        onVideoSetup={videoSetup.handleShow}
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
          onAddFolder={addFolder}
          onNavigateWorkspace={(id) =>
            navigate({ kind: "workspace", workspaceId: id })
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
            <EngineAlert />
            {preferenceStatus.kind === "failed" && (
              <Alert variant="destructive">
                <CircleAlert />
                <AlertTitle>Couldn&apos;t load preferences</AlertTitle>
                <AlertDescription>
                  <p>{preferenceStatus.message}</p>
                  <Button
                    variant="outline"
                    onClick={() => {
                      void retryPreferences();
                    }}
                  >
                    Retry
                  </Button>
                </AlertDescription>
              </Alert>
            )}
            {workspaceExpansion.error && (
              <Alert variant="destructive">
                <CircleAlert />
                <AlertTitle>Couldn&apos;t save workspace expansion</AlertTitle>
                <AlertDescription>
                  <p>{workspaceExpansion.error}</p>
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={workspaceExpansion.saving}
                    onClick={() => {
                      void workspaceExpansion.retry();
                    }}
                  >
                    Retry
                  </Button>
                </AlertDescription>
              </Alert>
            )}
            <AppContent
              display={display}
              effectiveFilter={effectiveFilter}
              folderId={folderId}
              listStyle={preferences?.list_style ?? "cards"}
              onInspect={inspect}
              onRemove={remove}
              onNew={newDownload}
              query={query}
              selectedId={selectedId}
              workspaceId={
                location.kind === "workspace" ? workspace?.id : undefined
              }
            />
          </div>
        </div>
      </SidebarInset>
      <VideoSetup setup={videoSetup} />
      <AppUpdateRestartDialog activeDownloads={activeDownloads} />
      <AppModals
        onVideoSetup={() => {
          setModal(null);
          videoSetup.handleShow();
        }}
        busy={busy}
        modal={modal}
        navigate={navigate}
        onClose={() => setModal(null)}
        onRemove={remove}
        run={run}
        selectedId={selectedId}
        setSelectedId={setSelectedId}
      />
    </>
  );
};

export default App;
