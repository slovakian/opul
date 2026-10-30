import { FolderOpen, Loader2 } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import type { Dispatch, RefObject, SetStateAction } from "react";

import { Choice } from "@/components/sidebar-popover";
import { Alert, AlertDescription } from "@/components/ui/alert";
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
import {
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverHeader,
  PopoverTitle,
  PopoverTrigger,
} from "@/components/ui/popover";
import { toast } from "@/components/ui/toast";
import { WorkspaceIcon } from "@/components/workspace-icon";
import { WorkspaceIconPicker } from "@/components/workspace-icon-picker";
import { bridge, errorMessage } from "@/lib/bridge";
import { useLibrary } from "@/lib/library";
import { workspaceEmoji } from "@/lib/model";
import type { Workspace, WorkspaceDraft, WorkspaceRequest } from "@/lib/model";

const emojis = [
  "📁",
  "📦",
  "💼",
  "📚",
  "🎨",
  "🚀",
  "💡",
  "🎯",
  "⚡️",
  "🔥",
  "✨",
];

const WorkspaceIconField = ({
  workspace,
  draft,
  emoji,
  busy,
  pickerOpen,
  setPickerOpen,
  setDraft,
}: {
  workspace: Workspace | null;
  draft: (WorkspaceDraft & { emoji: string }) | null;
  emoji: string;
  busy: boolean;
  pickerOpen: boolean;
  setPickerOpen: (open: boolean) => void;
  setDraft: Dispatch<
    SetStateAction<(WorkspaceDraft & { emoji: string }) | null>
  >;
}) => {
  if (workspace) {
    return (
      <Choice
        label="Workspace emoji"
        value={emoji}
        options={emojis.map((value) => ({
          content: (
            <WorkspaceIcon
              workspace={{
                appearance: workspace.appearance,
                icon: { kind: "Emoji", value },
              }}
            />
          ),
          label: value,
          value,
        }))}
        onChange={(nextEmoji) =>
          setDraft((prev) => prev && { ...prev, emoji: nextEmoji })
        }
      />
    );
  }

  return (
    <Popover open={pickerOpen} onOpenChange={setPickerOpen}>
      <PopoverTrigger
        render={
          <Button
            id="workspace-emoji"
            type="button"
            variant="outline"
            disabled={!draft || busy}
            aria-label={`Workspace emoji: ${emoji}`}
          />
        }
      >
        <WorkspaceIcon
          workspace={{
            appearance: draft?.appearance ?? {
              automatic_folder: null,
              preference: { kind: "Automatic" },
            },
            icon: { kind: "Emoji", value: emoji },
          }}
        />
      </PopoverTrigger>
      <PopoverContent
        side="right"
        align="start"
        className="h-[min(34rem,var(--available-height))] max-h-[calc(100svh-1rem)] w-80 max-w-[calc(100vw-1rem)] gap-2 overflow-hidden p-3 motion-reduce:transition-none motion-reduce:data-closed:animate-none motion-reduce:data-open:animate-none"
      >
        <PopoverHeader className="shrink-0">
          <PopoverTitle>Choose workspace icon</PopoverTitle>
          <PopoverDescription>
            Saved when you create the workspace.
          </PopoverDescription>
        </PopoverHeader>
        {draft && (
          <WorkspaceIconPicker
            workspaceId={draft.workspace_id}
            draft={{ appearance: draft.appearance, emoji }}
            onChange={(update) =>
              setDraft((prevDraft) => {
                if (!prevDraft) {
                  return null;
                }

                const resolved =
                  "call" in update
                    ? update({
                        appearance: prevDraft.appearance,
                        emoji: prevDraft.emoji,
                      })
                    : update;

                return { ...prevDraft, ...resolved };
              })
            }
          />
        )}
        <div className="flex shrink-0 justify-end">
          <Button type="button" onClick={() => setPickerOpen(false)}>
            Done
          </Button>
        </div>
      </PopoverContent>
    </Popover>
  );
};

const WorkspaceFormFields = ({
  workspace,
  draft,
  emoji,
  busy,
  pickerOpen,
  setPickerOpen,
  setDraft,
  name,
  setName,
  nameInput,
  path,
  setPath,
  chooseFolder,
}: {
  workspace: Workspace | null;
  draft: (WorkspaceDraft & { emoji: string }) | null;
  emoji: string;
  busy: boolean;
  pickerOpen: boolean;
  setPickerOpen: (open: boolean) => void;
  setDraft: Dispatch<
    SetStateAction<(WorkspaceDraft & { emoji: string }) | null>
  >;
  name: string;
  setName: (name: string) => void;
  nameInput: RefObject<HTMLInputElement | null>;
  path: string;
  setPath: (path: string) => void;
  chooseFolder: () => Promise<void>;
}) => (
  <FieldGroup>
    <div className="flex items-end gap-3">
      <Field className="w-24 shrink-0">
        <FieldLabel htmlFor={workspace ? undefined : "workspace-emoji"}>
          Icon
        </FieldLabel>
        <WorkspaceIconField
          workspace={workspace}
          draft={draft}
          emoji={emoji}
          busy={busy}
          pickerOpen={pickerOpen}
          setPickerOpen={setPickerOpen}
          setDraft={setDraft}
        />
      </Field>
      <Field>
        <FieldLabel htmlFor="workspace-name">Name</FieldLabel>
        <Input
          id="workspace-name"
          ref={nameInput}
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
        <FieldLabel htmlFor="workspace-path">Save downloads to</FieldLabel>
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
            onClick={() => {
              chooseFolder().catch(() => null);
            }}
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
);

const WorkspaceDialogAlerts = ({
  draftError,
  saveError,
  onRetry,
}: {
  draftError?: string;
  saveError?: string;
  onRetry: () => void;
}) => (
  <>
    {draftError && (
      <Alert>
        <AlertDescription>
          <p>{draftError}</p>
          <Button type="button" variant="outline" size="sm" onClick={onRetry}>
            Try again
          </Button>
        </AlertDescription>
      </Alert>
    )}
    {saveError && (
      <Alert role="alert">
        <AlertDescription>{saveError}</AlertDescription>
      </Alert>
    )}
  </>
);

export const WorkspaceDialog = ({
  workspace,
  onClose,
  onCreated,
}: {
  workspace: Workspace | null;
  onClose: () => void;
  onCreated: (id: string) => void;
}) => {
  const { state, run, busy } = useLibrary();
  const nameInput = useRef<HTMLInputElement>(null);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [saveError, setSaveError] = useState<string>();

  const [draft, setDraft] = useState<
    (WorkspaceDraft & { emoji: string }) | null
  >(() =>
    workspace
      ? {
          appearance: workspace.appearance,
          emoji: workspaceEmoji(workspace),
          folder_id:
            workspace.folders.find((folder) => folder.is_default)?.id ?? "",
          workspace_id: workspace.id,
        }
      : null
  );

  const [draftError, setDraftError] = useState<string>();
  const pendingDraft = useRef<Promise<WorkspaceDraft> | null>(null);

  const lastSubmission = useRef<{ input: string; requestId: string } | null>(
    null
  );

  const loadDraft = useCallback(async () => {
    try {
      if (!pendingDraft.current) {
        pendingDraft.current = bridge.workspaceDraft();
      }

      const loaded = await pendingDraft.current;
      setDraft({ ...loaded, emoji: "📁" });
      setDraftError(undefined);
    } catch (error) {
      pendingDraft.current = null;
      setDraftError(errorMessage(error));
    }
  }, []);

  useEffect(() => {
    if (!workspace) {
      loadDraft().catch(() => null);
    }
  }, [workspace, loadDraft]);

  const [name, setName] = useState(workspace?.name ?? "");
  const emoji = draft?.emoji ?? "📁";

  const [path, setPath] = useState(
    workspace?.folders.find((folder) => folder.is_default)?.path ??
      (state.kind === "ready"
        ? (state.snapshot.workspaces[0]?.folders[0]?.path ?? "")
        : "")
  );

  const chooseFolder = async () => {
    try {
      const selected = await bridge.chooseFolder();

      if (selected) {
        setPath(selected);
      }
    } catch (error) {
      toast.add({
        description: errorMessage(error),
        title: "Couldn't open folder picker",
        type: "error",
      });
    }
  };

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();

    if (!draft) {
      return;
    }

    setSaveError(undefined);
    const id = draft.workspace_id;
    const defaultFolder = workspace?.folders.find((f) => f.is_default);

    const input = {
      appearance: draft.appearance,
      default_format: workspace?.default_format ?? "Best",
      expected_revision: null,
      file_organization: workspace?.file_organization ?? "Automatic",
      folder_id: draft.folder_id,
      folder_name: defaultFolder?.name ?? "Downloads",
      folder_path: path.trim(),
      icon: { kind: "Emoji", value: emoji },
      name: name.trim(),
      workspace_id: id,
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
    } else {
      setSaveError(
        "Couldn't save the workspace. Your changes are kept here; try again."
      );
    }
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy) {
          onClose();
        }
      }}
    >
      <DialogContent
        initialFocus={nameInput}
        className="max-h-[calc(100svh-2rem)] overflow-y-auto motion-reduce:animate-none motion-reduce:transition-none sm:max-w-md"
      >
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
          <WorkspaceFormFields
            workspace={workspace}
            draft={draft}
            emoji={emoji}
            busy={busy}
            pickerOpen={pickerOpen}
            setPickerOpen={setPickerOpen}
            setDraft={setDraft}
            name={name}
            setName={setName}
            nameInput={nameInput}
            path={path}
            setPath={setPath}
            chooseFolder={chooseFolder}
          />
          <WorkspaceDialogAlerts
            draftError={draftError}
            saveError={saveError}
            onRetry={() => {
              loadDraft().catch(() => null);
            }}
          />
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
};
