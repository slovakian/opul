import { useCallback, useEffect, useRef, useState } from "react";
import { WorkspaceIcon } from "@/components/workspace-icon";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { FolderOpen, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { WorkspaceIconPicker } from "@/components/workspace-icon-picker";
import {
  Popover,
  PopoverContent,
  PopoverTitle,
  PopoverTrigger,
} from "@/components/ui/popover";
import { useLibrary } from "@/lib/library";
import { bridge, errorMessage } from "@/lib/bridge";
import { toast } from "@/components/ui/toast";
import {
  workspaceEmoji,
  type Workspace,
  type WorkspaceDraft,
  type WorkspaceRequest,
} from "@/lib/model";

export function WorkspaceDialog({
  workspace,
  onClose,
  onCreated,
}: {
  workspace: Workspace | null;
  onClose: () => void;
  onCreated: (id: string) => void;
}) {
  const { state, run, busy } = useLibrary();
  const [draft, setDraft] = useState<WorkspaceDraft | null>(() =>
    workspace
      ? {
          workspace_id: workspace.id,
          folder_id:
            workspace.folders.find((folder) => folder.is_default)?.id ?? "",
          appearance: workspace.appearance,
        }
      : null,
  );
  const [draftError, setDraftError] = useState<string>();
  const pendingDraft = useRef<Promise<WorkspaceDraft> | null>(null);
  const lastSubmission = useRef<{ input: string; requestId: string } | null>(
    null,
  );
  const loadDraft = useCallback(async () => {
    setDraftError(undefined);
    try {
      pendingDraft.current ??= bridge.workspaceDraft();
      setDraft(await pendingDraft.current);
    } catch (error) {
      pendingDraft.current = null;
      setDraftError(errorMessage(error));
    }
  }, []);
  useEffect(() => {
    if (!workspace) void loadDraft();
  }, [workspace, loadDraft]);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [name, setName] = useState(workspace?.name ?? "");
  const [emoji, setEmoji] = useState(
    workspace ? workspaceEmoji(workspace) : "📁",
  );
  const [path, setPath] = useState(
    workspace?.folders.find((folder) => folder.is_default)?.path ??
      (state.kind === "ready"
        ? (state.snapshot.workspaces[0]?.folders[0]?.path ?? "")
        : ""),
  );
  async function chooseFolder() {
    try {
      const selected = await bridge.chooseFolder();
      if (selected) setPath(selected);
    } catch (error) {
      toast.add({
        title: "Couldn't open folder picker",
        description: errorMessage(error),
        type: "error",
      });
    }
  }
  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!draft) return;
    const id = draft.workspace_id;
    const folder = workspace?.folders.find((folder) => folder.is_default);
    const input = {
      expected_revision: null,
      workspace_id: id,
      name: name.trim(),
      icon: { kind: "Emoji", value: emoji },
      appearance: draft.appearance,
      folder_id: draft.folder_id,
      folder_name: folder?.name ?? "Downloads",
      folder_path: path.trim(),
      default_format: workspace?.default_format ?? "Best",
      file_organization: workspace?.file_organization ?? "Automatic",
    } satisfies Omit<WorkspaceRequest, "request_id">;
    const serialized = JSON.stringify(input);
    if (lastSubmission.current?.input !== serialized) {
      lastSubmission.current = {
        input: serialized,
        requestId: crypto.randomUUID(),
      };
    }
    const receipt = await run({
      kind: workspace ? "update_workspace" : "create_workspace",
      request: { request_id: lastSubmission.current.requestId, ...input },
    });
    if (receipt) {
      onCreated(id);
      onClose();
    }
  }
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy) onClose();
      }}
    >
      <DialogContent className="sm:max-w-md">
        <form className="flex flex-col gap-6" onSubmit={submit}>
          <DialogHeader>
            <DialogTitle>
              {workspace ? "Edit workspace" : "A place for your downloads"}
            </DialogTitle>
            <DialogDescription>
              {workspace
                ? "Give this workspace a name and an emoji."
                : "Keep a project, a collection, or just your everyday files together."}
            </DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <div className="flex items-end gap-3">
              <Field className="w-24 shrink-0">
                <FieldLabel>Emoji</FieldLabel>
                <Popover open={pickerOpen} onOpenChange={setPickerOpen} modal>
                  <PopoverTrigger
                    render={
                      <Button
                        type="button"
                        variant="outline"
                        disabled={!draft}
                        aria-label={`Workspace emoji: ${emoji}`}
                      />
                    }
                  >
                    <WorkspaceIcon
                      workspace={{
                        icon: { kind: "Emoji", value: emoji },
                        appearance: draft?.appearance ?? {
                          automatic_folder: null,
                          preference: { kind: "Automatic" },
                        },
                      }}
                    />
                  </PopoverTrigger>
                  <PopoverContent
                    side="right"
                    align="start"
                    className="h-[min(34rem,var(--available-height))] max-h-[calc(100svh-1rem)] w-80 max-w-[calc(100vw-1rem)] gap-2 overflow-hidden p-3 motion-reduce:animate-none motion-reduce:transition-none"
                  >
                    <PopoverTitle>Change icon</PopoverTitle>
                    {draft && (
                      <WorkspaceIconPicker
                        workspaceId={draft.workspace_id}
                        draft={{ emoji, appearance: draft.appearance }}
                        onChange={(next) => {
                          setEmoji(next.emoji);
                          setDraft({ ...draft, appearance: next.appearance });
                        }}
                      />
                    )}
                    <Button
                      type="button"
                      variant="outline"
                      className="shrink-0 self-end"
                      onClick={() => setPickerOpen(false)}
                    >
                      Done
                    </Button>
                  </PopoverContent>
                </Popover>
              </Field>
              <Field>
                <FieldLabel htmlFor="workspace-name">Name</FieldLabel>
                <Input
                  id="workspace-name"
                  autoFocus
                  value={name}
                  onChange={(event) => setName(event.target.value)}
                  placeholder="e.g. Personal"
                  required
                  maxLength={80}
                />
              </Field>
            </div>
            {!workspace && (
              <Field>
                <FieldLabel htmlFor="workspace-path">
                  Save downloads to
                </FieldLabel>
                <div className="flex gap-2">
                  <Input
                    id="workspace-path"
                    value={path}
                    onChange={(event) => setPath(event.target.value)}
                    required
                  />
                  <Button
                    type="button"
                    variant="outline"
                    size="icon"
                    aria-label="Choose workspace folder"
                    onClick={() => void chooseFolder()}
                  >
                    <FolderOpen />
                  </Button>
                </div>
                <FieldDescription>
                  You can add more saved folders later.
                </FieldDescription>
              </Field>
            )}
          </FieldGroup>
          {draftError && (
            <Alert>
              <AlertDescription>
                <p>{draftError}</p>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => void loadDraft()}
                >
                  Try again
                </Button>
              </AlertDescription>
            </Alert>
          )}
          <DialogFooter>
            <Button
              type="button"
              variant="ghost"
              disabled={busy}
              onClick={onClose}
            >
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={busy || !draft || !name.trim() || !path.trim()}
            >
              {busy && (
                <Loader2 className="animate-spin" data-icon="inline-start" />
              )}
              {workspace ? "Save changes" : "Create workspace"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
