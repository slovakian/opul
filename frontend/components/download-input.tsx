import {
  ChevronDown,
  Download,
  FolderClosed,
  FolderPlus,
  Link,
  Loader2,
  Search,
  Settings2,
  X,
} from "lucide-react";
import { useState } from "react";
import type { FormEvent, RefObject } from "react";

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
import { isDownloadLink } from "@/lib/download-link";
import { useLibrary } from "@/lib/library";
import type { Workspace } from "@/lib/model";

export function DownloadInput({
  workspace,
  disabled,
  onOptions,
  query,
  setQuery,
  inputRef,
}: {
  workspace: Workspace;
  disabled: boolean;
  onOptions: (url: string, folderId: string) => void;
  query: string;
  setQuery: (query: string) => void;
  inputRef: RefObject<HTMLInputElement | null>;
}) {
  const { run } = useLibrary();
  const [selectedFolderId, setSelectedFolderId] = useState<string>();
  const [adding, setAdding] = useState(false);

  const folder =
    workspace.folders.find((option) => option.id === selectedFolderId) ??
    workspace.folders.find((option) => option.is_default);

  const isLink = isDownloadLink(query);

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

    if (!folder || disabled || adding || !isLink) {
      return;
    }

    setAdding(true);
    const link = query.trim();

    const receipt = await run({
      folder_id: folder.id,
      format: workspace.default_format,
      kind: "add_url",
      mode:
        /^magnet:/iu.test(link) || /\.torrent(?:\?|$)/iu.test(link)
          ? "Torrent"
          : "Direct",
      url: link,
      workspace_id: workspace.id,
    });

    if (receipt) {
      setQuery("");
    }

    setAdding(false);
  }

  return (
    <form
      onSubmit={submit}
      aria-label="Search or add a download"
      className="flex min-w-0 flex-1 items-center gap-1"
    >
      <InputGroup>
        <InputGroupAddon>
          {isLink ? <Link aria-hidden /> : <Search aria-hidden />}
          <span className="sr-only">
            {isLink ? "Download link recognized" : "Search query"}
          </span>
        </InputGroupAddon>
        <InputGroupInput
          ref={inputRef}
          type="search"
          aria-label="Search downloads or paste a link"
          placeholder="Search or paste a link…"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (
              event.key === "Escape" &&
              !event.defaultPrevented &&
              !document.querySelector(
                '[data-slot="popover-content"]:not([data-closed]), [role="dialog"]:not([data-closed])'
              )
            ) {
              event.preventDefault();
              setQuery("");
            }
          }}
        />
        <InputGroupAddon align="inline-end">
          {query && (
            <InputGroupButton
              size="icon-xs"
              aria-label="Clear input"
              onClick={() => {
                setQuery("");
                inputRef.current?.focus();
              }}
            >
              <X data-icon="inline-start" />
            </InputGroupButton>
          )}
          <SidebarPopover
            title="Save location"
            trigger={
              <InputGroupButton
                aria-label={`Save location: ${folder?.name ?? "Choose"}`}
                title={folder?.path}
                disabled={disabled || adding}
              >
                <FolderClosed data-icon="inline-start" />
                <span className="header-location max-w-24 truncate">
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
            variant={isLink ? "default" : "ghost"}
            disabled={disabled || adding || !isLink || !folder}
            aria-label="Download link"
          >
            {adding ? (
              <Loader2 className="animate-spin" data-icon="inline-start" />
            ) : (
              <Download data-icon="inline-start" />
            )}
            <span className="header-download-label">
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
            onOptions(query, folder.id);
          }
        }}
      >
        <Settings2 />
      </Button>
    </form>
  );
}
