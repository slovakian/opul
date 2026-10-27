import { useState } from "react";
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
import { Choice } from "@/components/sidebar-popover";
import { useLibrary } from "@/lib/library";
import { bridge, errorMessage } from "@/lib/bridge";
import { toast } from "@/components/ui/toast";
import { workspaceEmoji, type Workspace } from "@/lib/model";

const emojis = [
  "📁",
  "🏠",
  "💼",
  "🎬",
  "🎵",
  "📚",
  "🎮",
  "🎨",
  "📷",
  "💻",
  "🌱",
  "✨",
];
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
    const id = workspace?.id ?? crypto.randomUUID();
    const folder = workspace?.folders.find((folder) => folder.is_default);
    const receipt = await run({
      kind: workspace ? "update_workspace" : "create_workspace",
      request: {
        request_id: crypto.randomUUID(),
        expected_revision: null,
        workspace_id: id,
        name: name.trim(),
        icon: { kind: "Emoji", value: emoji },
        folder_id: folder?.id ?? crypto.randomUUID(),
        folder_name: folder?.name ?? "Downloads",
        folder_path: path.trim(),
        default_format: workspace?.default_format ?? "Best",
        file_organization: workspace?.file_organization ?? "Automatic",
      },
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
                <Choice
                  label="Workspace emoji"
                  value={emoji}
                  options={emojis.map((value) => ({ value, label: value }))}
                  onChange={setEmoji}
                />
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
              disabled={busy || !name.trim() || !path.trim()}
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
