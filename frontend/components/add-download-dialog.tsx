import { FileUp, Loader2 } from "lucide-react";
import { useState } from "react";

import { Choice } from "@/components/sidebar-popover";
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
import { toast } from "@/components/ui/toast";
import { WorkspaceIcon } from "@/components/workspace-icon";
import { bridge, errorMessage } from "@/lib/bridge";
import { useLibrary } from "@/lib/library";
import { formatNames, formatSchema } from "@/lib/model";
import type { Command, Format, Workspace } from "@/lib/model";

type Mode = Extract<Command, { kind: "add_url" }>["mode"];

export const AddDownloadDialog = ({
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
}) => {
  const { run, busy } = useLibrary();
  const [workspaceId, setWorkspaceId] = useState(initialWorkspace.id);

  const workspace =
    workspaces.find((w) => w.id === workspaceId) ?? initialWorkspace;

  const [folderId, setFolderId] = useState(
    initialFolder ??
      workspace.folders.find((folder) => folder.is_default)?.id ??
      ""
  );

  const [url, setUrl] = useState(initialUrl);

  const [mode, setMode] = useState<Mode>(
    initialUrl.startsWith("magnet:") ? "Torrent" : "Direct"
  );

  const [format, setFormat] = useState<Format>(workspace.default_format);

  const importTorrent = async () => {
    try {
      const path = await bridge.chooseTorrent();

      if (!path) {
        return;
      }

      if (
        await run({
          folder_id: folderId,
          kind: "add_torrent_file",
          path,
          workspace_id: workspace.id,
        })
      ) {
        onClose();
      }
    } catch (error) {
      toast.add({
        description: errorMessage(error),
        title: "Couldn't open torrent",
        type: "error",
      });
    }
  };

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();

    const receipt = await run({
      folder_id: folderId,
      format,
      kind: "add_url",
      mode: url.trim().startsWith("magnet:") ? "Torrent" : mode,
      url: url.trim(),
      workspace_id: workspace.id,
    });

    if (receipt) {
      onClose();
    }
  };

  let submitLabel = "Add download";

  if (busy) {
    submitLabel = mode === "Page" ? "Inspecting page…" : "Adding…";
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy) {
          onClose();
        }
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
                    /\.torrent(?:\?|$)/iu.test(event.target.value)
                  ) {
                    setMode("Torrent");
                  }
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
                  { label: "Original file", value: "Direct" },
                  { label: "Video page or playlist", value: "Page" },
                  { label: "Torrent or magnet link", value: "Torrent" },
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
                    label: formatNames[value],
                    value,
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
                  options={workspaces.map((ws) => ({
                    content: (
                      <>
                        <WorkspaceIcon workspace={ws} />
                        <span className="truncate">{ws.name}</span>
                      </>
                    ),
                    label: ws.name,
                    value: ws.id,
                  }))}
                  onChange={(id) => {
                    setWorkspaceId(id);
                    setFolderId(
                      workspaces
                        .find((ws) => ws.id === id)
                        ?.folders.find((folder) => folder.is_default)?.id ?? ""
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
                    label: folder.name,
                    value: folder.id,
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
              onClick={() => {
                importTorrent().catch(() => null);
              }}
              disabled={busy}
            >
              <FileUp data-icon="inline-start" />
              Open .torrent
            </Button>
            <Button type="submit" disabled={busy || !url.trim() || !folderId}>
              {busy && (
                <Loader2 className="animate-spin" data-icon="inline-start" />
              )}
              {submitLabel}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};
