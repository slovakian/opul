import { useState } from "react";
import { FileUp, Loader2 } from "lucide-react";
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
import { bridge, errorMessage } from "@/lib/bridge";
import { useLibrary } from "@/lib/library";
import {
  formatNames,
  formatSchema,
  workspaceEmoji,
  type Command,
  type Format,
  type Workspace,
} from "@/lib/model";
import { toast } from "@/components/ui/toast";

type Mode = Extract<Command, { kind: "add_url" }>["mode"];
export function AddDownloadDialog({
  workspaces,
  initialWorkspace,
  initialFolder,
  initialUrl = "",
  onClose,
}: {
  workspaces: Workspace[];
  initialWorkspace: Workspace;
  initialFolder?: string;
  initialUrl?: string;
  onClose: () => void;
}) {
  const { run, busy } = useLibrary();
  const [workspaceId, setWorkspaceId] = useState(initialWorkspace.id);
  const workspace =
    workspaces.find((workspace) => workspace.id === workspaceId) ??
    initialWorkspace;
  const [folderId, setFolderId] = useState(
    initialFolder ??
      workspace.folders.find((folder) => folder.is_default)?.id ??
      "",
  );
  const [url, setUrl] = useState(initialUrl);
  const [mode, setMode] = useState<Mode>(
    initialUrl.startsWith("magnet:") ? "Torrent" : "Direct",
  );
  const [format, setFormat] = useState<Format>(workspace.default_format);
  async function importTorrent() {
    try {
      const path = await bridge.chooseTorrent();
      if (!path) return;
      if (
        await run({
          kind: "add_torrent_file",
          workspace_id: workspace.id,
          folder_id: folderId,
          path,
        })
      )
        onClose();
    } catch (error) {
      toast.add({
        title: "Couldn't open torrent",
        description: errorMessage(error),
        type: "error",
      });
    }
  }
  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const receipt = await run({
      kind: "add_url",
      workspace_id: workspace.id,
      folder_id: folderId,
      url: url.trim(),
      mode: url.trim().startsWith("magnet:") ? "Torrent" : mode,
      format,
    });
    if (receipt) onClose();
  }
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy) onClose();
      }}
    >
      <DialogContent className="sm:max-w-lg">
        <form onSubmit={submit} className="flex flex-col gap-6">
          <DialogHeader>
            <DialogTitle>New download</DialogTitle>
            <DialogDescription>
              A file link, a video page, or a torrent. All in one place.
            </DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="download-url">Link</FieldLabel>
              <Input
                id="download-url"
                autoFocus
                placeholder="https://… or magnet:?…"
                value={url}
                onChange={(event) => {
                  setUrl(event.target.value);
                  if (
                    event.target.value.startsWith("magnet:") ||
                    /\.torrent(\?|$)/i.test(event.target.value)
                  )
                    setMode("Torrent");
                }}
                required
              />
              <FieldDescription>
                Video pages and playlists use yt-dlp.
              </FieldDescription>
            </Field>
            <Field>
              <FieldLabel>Download as</FieldLabel>
              <Choice<Mode>
                label="Download type"
                value={mode}
                options={[
                  { value: "Direct", label: "Original file" },
                  { value: "Page", label: "Video page or playlist" },
                  { value: "Torrent", label: "Torrent or magnet link" },
                ]}
                onChange={setMode}
                disabled={busy}
              />
            </Field>
            {mode === "Page" && (
              <Field>
                <FieldLabel>Format</FieldLabel>
                <Choice
                  label="Video format"
                  value={format}
                  options={formatSchema.options.map((value) => ({
                    value,
                    label: formatNames[value],
                  }))}
                  onChange={setFormat}
                />
              </Field>
            )}
            <div className="grid grid-cols-2 gap-3">
              <Field>
                <FieldLabel>Workspace</FieldLabel>
                <Choice
                  label="Download workspace"
                  value={workspace.id}
                  options={workspaces.map((workspace) => ({
                    value: workspace.id,
                    label: `${workspaceEmoji(workspace)} ${workspace.name}`,
                  }))}
                  onChange={(id) => {
                    setWorkspaceId(id);
                    setFolderId(
                      workspaces
                        .find((workspace) => workspace.id === id)
                        ?.folders.find((folder) => folder.is_default)?.id ?? "",
                    );
                  }}
                />
              </Field>
              <Field>
                <FieldLabel>Save to</FieldLabel>
                <Choice
                  label="Save location"
                  value={folderId}
                  options={workspace.folders.map((folder) => ({
                    value: folder.id,
                    label: folder.name,
                  }))}
                  onChange={setFolderId}
                />
              </Field>
            </div>
          </FieldGroup>
          <DialogFooter className="sm:justify-between">
            <Button
              type="button"
              variant="ghost"
              onClick={() => void importTorrent()}
              disabled={busy}
            >
              <FileUp data-icon="inline-start" />
              Open .torrent
            </Button>
            <Button type="submit" disabled={busy || !url.trim() || !folderId}>
              {busy && (
                <Loader2 className="animate-spin" data-icon="inline-start" />
              )}
              {busy
                ? mode === "Page"
                  ? "Inspecting page…"
                  : "Adding…"
                : "Add download"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
