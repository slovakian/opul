import { useState, type FormEvent } from "react";
import {
  ChevronDown,
  Download,
  FolderClosed,
  Link,
  Loader2,
  Settings2,
} from "lucide-react";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from "@/components/ui/input-group";
import { Button } from "@/components/ui/button";
import { SidebarPopover } from "@/components/sidebar-popover";
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

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!folder || disabled || adding || !url.trim()) return;
    setAdding(true);
    const link = url.trim();
    const receipt = await run({
      kind: "add_url",
      workspace_id: workspace.id,
      folder_id: folder.id,
      url: link,
      mode:
        link.startsWith("magnet:") || /\.torrent(\?|$)/i.test(link)
          ? "Torrent"
          : "Direct",
      format: workspace.default_format,
    });
    if (receipt) setUrl("");
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
                label: `${workspace.name} · Save to`,
                actions: [...workspace.folders]
                  .sort((a, b) => Number(b.is_default) - Number(a.is_default))
                  .map((option) => ({
                    label: `${option.name}${option.is_default ? " (Default)" : ""}`,
                    selected: option.id === folder?.id,
                    icon: FolderClosed,
                    onClick: () => setSelectedFolderId(option.id),
                  })),
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
          if (folder) onOptions(url, folder.id);
        }}
      >
        <Settings2 />
      </Button>
    </form>
  );
}
