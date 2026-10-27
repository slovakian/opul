import { AddDownloadDialog } from "@/components/add-download-dialog";
import { DownloadInspector } from "@/components/download-inspector";
import { SettingsDialog } from "@/components/settings-dialog";
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
import type { useLibrary } from "@/lib/library";
import type { Download, Workspace } from "@/lib/model";

export type Modal =
  | { kind: "icon"; workspace: Workspace; anchor: HTMLButtonElement | null }
  | { kind: "add"; url?: string; folderId?: string; workspaceId: string }
  | { kind: "settings" }
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
            {isRemove
              ? `Remove “${modal.download.name}” from your library? Any active transfer will stop. Your downloaded files stay on disk.`
              : `Delete “${modal.workspace.name}” and its download history? Downloaded files stay on disk.`}
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
  folderId?: string;
  modal: Modal | null;
  navigate: (next: Location) => void;
  onClose: () => void;
  onRemoveSelected: () => void;
  run: ReturnType<typeof useLibrary>["run"];
  selected: Download | undefined;
  selectedWorkspace: Workspace | undefined;
  setSelectedId: (id: string | undefined) => void;
  workspace: Workspace | undefined;
  workspaces: Workspace[];
}

export const AppModals = ({
  busy,
  folderId,
  modal,
  navigate,
  onClose,
  onRemoveSelected,
  run,
  selected,
  selectedWorkspace,
  setSelectedId,
  workspace,
  workspaces,
}: AppModalsProps) => (
  <>
    {selected && selectedWorkspace && (
      <DownloadInspector
        key={selected.id}
        download={selected}
        workspace={selectedWorkspace}
        onClose={() => setSelectedId(undefined)}
        onRemove={onRemoveSelected}
      />
    )}
    {modal?.kind === "settings" && <SettingsDialog onClose={onClose} />}
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
    {modal?.kind === "add" && workspace && (
      <AddDownloadDialog
        workspaces={workspaces}
        initialWorkspace={
          workspaces.find((item) => item.id === modal.workspaceId) ?? workspace
        }
        initialFolder={modal.folderId ?? folderId}
        initialUrl={modal.url}
        onClose={onClose}
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
