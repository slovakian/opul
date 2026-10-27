import {
  ChevronDown,
  Download,
  FolderClosed,
  FolderPlus,
  Link,
  Loader2,
  Settings2,
} from "lucide-react";
import { useState } from "react";
import type { FormEvent } from "react";

import { SidebarPopover } from "@/components/sidebar-popover";
import { Button } from "@/components/ui/button";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from "@/components/ui/input-group";
import { toast } from "@/components/ui/toast";
import { bridge, errorMessage } from "@/lib/bridge";
import { useLibrary } from "@/lib/library";
import type { Workspace } from "@/lib/model";

export function DownloadInput({
  workspace,
  disabled,
  onOptions,
}: {
  workspace: Workspace;
  disabled: boolean;
  onOptions: (url: string, folderId: string) => void;
}) {
  const { run } = useLibrary();
  const [url, setUrl] = useState("");
  const [selectedFolderId, setSelectedFolderId] = useState<string>();
  const [adding, setAdding] = useState(false);

  const folder =
    workspace.folders.find((folder) => folder.id === selectedFolderId) ??
    workspace.folders.find((folder) => folder.is_default);

  async function addLocation() {
    try {
      const path = await bridge.chooseFolder();

      if (!path) {
        return;
      }

      const folderId = crypto.randomUUID();

      const receipt = await run({
        kind: "add_folder",
        request: {
          expected_revision: null,
          folder_id: folderId,
          name: path.split("/").filter(Boolean).pop() || "Folder",
          path,
          request_id: crypto.randomUUID(),
          workspace_id: workspace.id,
        },
      });

      if (receipt) {
        setSelectedFolderId(folderId);
      }
    } catch (error) {
      toast.add({
        description: errorMessage(error),
        title: "Couldn't add location",
        type: "error",
      });
    }
  }

  async function submit(event: FormEvent) {
    event.preventDefault();

    if (!folder || disabled || adding || !url.trim()) {
      return;
    }

    setAdding(true);
    const link = url.trim();

    const receipt = await run({
      folder_id: folder.id,
      format: workspace.default_format,
      kind: "add_url",
      mode:
        link.startsWith("magnet:") || /\.torrent(\?|$)/i.test(link)
          ? "Torrent"
          : "Direct",
      url: link,
      workspace_id: workspace.id,
    });

    if (receipt) {
      setUrl("");
    }

    setAdding(false);
  }

  return (
    <form
      onSubmit={submit}
      aria-label="Add a download"
      className="flex min-w-0 items-center gap-2 px-4 pb-3"
    >
      <InputGroup>
        <InputGroupAddon>
          <Link />
        </InputGroupAddon>
        <InputGroupInput
          aria-label="Download link"
          placeholder="Paste a download link…"
          value={url}
          onChange={(event) => setUrl(event.target.value)}
          disabled={disabled || adding}
          required
        />
        <InputGroupAddon align="inline-end">
          <SidebarPopover
            title="Save location"
            trigger={
              <InputGroupButton
                aria-label={`Save location: ${folder?.name ?? "Choose"}`}
                title={folder?.path}
                disabled={disabled || adding}
              >
                <FolderClosed data-icon="inline-start" />
                <span className="hidden max-w-40 truncate sm:inline">
                  {folder?.name ?? "Save to"}
                </span>
                <ChevronDown data-icon="inline-end" />
              </InputGroupButton>
            }
            groups={[
              {
                actions: [...workspace.folders]
                  .sort((a, b) => Number(b.is_default) - Number(a.is_default))
                  .map((option) => ({
                    icon: FolderClosed,
                    label: `${option.name}${option.is_default ? " (Default)" : ""}`,
                    onClick: () => setSelectedFolderId(option.id),
                    selected: option.id === folder?.id,
                  })),
              },
              {
                actions: [
                  {
                    icon: FolderPlus,
                    label: "Add new location…",
                    onClick: () => void addLocation(),
                  },
                ],
              },
            ]}
          />
          <InputGroupButton
            type="submit"
            variant="default"
            disabled={disabled || adding || !url.trim() || !folder}
            aria-label="Download link"
          >
            {adding ? (
              <Loader2 className="animate-spin" data-icon="inline-start" />
            ) : (
              <Download data-icon="inline-start" />
            )}
            <span className="hidden sm:inline">
              {adding ? "Adding…" : "Download"}
            </span>
          </InputGroupButton>
        </InputGroupAddon>
      </InputGroup>
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        aria-label="Download options"
        title="Video pages, formats, and torrent files"
        disabled={disabled || adding || !folder}
        onClick={() => {
          if (folder) {
            onOptions(url, folder.id);
          }
        }}
      >
        <Settings2 />
      </Button>
    </form>
  );
}
