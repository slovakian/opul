import { DownloadInspector } from "@/components/download-inspector";
import { SettingsDialog } from "@/components/settings-dialog";
import type { SettingsSection } from "@/components/settings-dialog";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { WorkspaceDialog } from "@/components/workspace-dialog";
import { WorkspaceIconEditor } from "@/components/workspace-icon-picker";
import type { Location } from "@/components/workspace-sidebar";
import { useLibrarySelector } from "@/lib/library";
import type { useLibrary } from "@/lib/library";
import { selectWorkspaces } from "@/lib/library-store";
import type { Download, Workspace } from "@/lib/model";
import { truncateFilename } from "@/lib/truncate";

export type Modal =
  | { kind: "icon"; workspace: Workspace; anchor: HTMLButtonElement | null }
  | { kind: "settings"; section?: SettingsSection }
  | { kind: "workspace"; workspace: Workspace | null }
  | { kind: "remove"; download: Download }
  | { kind: "delete-workspace"; workspace: Workspace };

interface DeleteConfirmDialogProps {
  busy: boolean;
  modal: Extract<Modal, { kind: "delete-workspace" | "remove" }>;
  navigate: (next: Location) => void;
  onClose: () => void;
  run: ReturnType<typeof useLibrary>["run"];
}

const DeleteConfirmDialog = ({
  busy,
  modal,
  navigate,
  onClose,
  run,
}: DeleteConfirmDialogProps) => {
  let deleteActionLabel = "Delete workspace";

  if (busy) {
    deleteActionLabel = "Removing…";
  } else if (modal.kind === "remove") {
    deleteActionLabel = "Remove download";
  }

  const isRemove = modal.kind === "remove";

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {isRemove ? "Remove this download?" : "Delete this workspace?"}
          </DialogTitle>
          <DialogDescription>
            {isRemove ? (
              <>
                Remove “
                <span title={modal.download.name}>
                  {truncateFilename(modal.download.name, 72)}
                </span>
                ” from your library? Any active transfer will stop. Your
                downloaded files stay on disk.
              </>
            ) : (
              <>
                Delete “
                <span title={modal.workspace.name}>
                  {truncateFilename(modal.workspace.name, 72)}
                </span>
                ” and its download history? Downloaded files stay on disk.
              </>
            )}
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
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

                onClose();
              }
            }}
          >
            {deleteActionLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export interface AppModalsProps {
  busy: boolean;
  modal: Modal | null;
  navigate: (next: Location) => void;
  onClose: () => void;
  onVideoSetup: () => void;
  onRemove: (download: Download) => void;
  run: ReturnType<typeof useLibrary>["run"];
  selectedId: string | undefined;
  setSelectedId: (id: string | undefined) => void;
}

/** The selected download's details. Only this panel follows its every change. */
const SelectedDownload = ({
  onRemove,
  selectedId,
  setSelectedId,
}: Pick<AppModalsProps, "onRemove" | "selectedId" | "setSelectedId">) => {
  const download = useLibrarySelector((state) =>
    selectedId && state.kind === "ready"
      ? state.library.downloadById.get(selectedId)
      : undefined
  );

  const workspace = useLibrarySelector((state) =>
    selectWorkspaces(state).find((item) => item.id === download?.workspace_id)
  );

  if (!download || !workspace) {
    return null;
  }

  return (
    <DownloadInspector
      key={download.id}
      download={download}
      workspace={workspace}
      onClose={() => setSelectedId(undefined)}
      onRemove={() => onRemove(download)}
    />
  );
};

export const AppModals = ({
  busy,
  modal,
  navigate,
  onClose,
  onVideoSetup,
  onRemove,
  run,
  selectedId,
  setSelectedId,
}: AppModalsProps) => (
  <>
    <SelectedDownload
      onRemove={onRemove}
      selectedId={selectedId}
      setSelectedId={setSelectedId}
    />
    {modal?.kind === "settings" && (
      <SettingsDialog
        onClose={onClose}
        onVideoSetup={onVideoSetup}
        section={modal.section}
      />
    )}
    {modal?.kind === "icon" && (
      <WorkspaceIconEditor
        workspace={modal.workspace}
        anchor={modal.anchor}
        onClose={onClose}
      />
    )}
    {modal?.kind === "workspace" && (
      <WorkspaceDialog
        workspace={modal.workspace}
        onClose={onClose}
        onCreated={(workspaceId) =>
          navigate({ kind: "workspace", workspaceId })
        }
      />
    )}
    {(modal?.kind === "remove" || modal?.kind === "delete-workspace") && (
      <DeleteConfirmDialog
        busy={busy}
        modal={modal}
        navigate={navigate}
        onClose={onClose}
        run={run}
      />
    )}
  </>
);
