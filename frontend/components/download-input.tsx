import { Download, Link, Loader2, Search, Settings2, X } from "lucide-react";
import { useState } from "react";
import type { FormEvent, RefObject } from "react";

import { DestinationDialog } from "@/components/destination-dialog";
import { DestinationPicker } from "@/components/destination-picker";
import { Button } from "@/components/ui/button";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from "@/components/ui/input-group";
import { toast } from "@/components/ui/toast";
import type { Location } from "@/components/workspace-sidebar";
import { bridge, errorMessage } from "@/lib/bridge";
import { naturalDestination, recommendedWorkspace } from "@/lib/destination";
import type { Destination } from "@/lib/destination";
import { isDownloadLink } from "@/lib/download-link";
import { useLibrary } from "@/lib/library";
import type { Workspace } from "@/lib/model";

export const DownloadInput = ({
  workspaces,
  location,
  lastUsed,
  onUsed,
  disabled,
  onOptions,
  query,
  setQuery,
  inputRef,
}: {
  workspaces: Workspace[];
  location: Location;
  lastUsed?: string;
  onUsed: (workspaceId: string) => void;
  disabled: boolean;
  onOptions: (url: string, folderId: string, workspaceId: string) => void;
  query: string;
  setQuery: (query: string) => void;
  inputRef: RefObject<HTMLInputElement | null>;
}) => {
  const { run, preferences, savePreferences } = useLibrary();

  const [override, setOverride] = useState<{
    workspaceId: string;
    folderId: string;
  }>();

  const [adding, setAdding] = useState(false);
  const [confirming, setConfirming] = useState(false);

  const overrideWorkspace = workspaces.find(
    (item) => item.id === override?.workspaceId
  );

  const overrideFolder = overrideWorkspace?.folders.find(
    (item) => item.id === override?.folderId
  );

  const destination =
    overrideWorkspace && overrideFolder
      ? { folder: overrideFolder, workspace: overrideWorkspace }
      : naturalDestination(workspaces, location, preferences);

  const recommended = recommendedWorkspace(workspaces, preferences, lastUsed);

  const selectDestination = (next: Destination) => {
    setOverride({ folderId: next.folder.id, workspaceId: next.workspace.id });
  };

  const isLink = isDownloadLink(query);
  const locked = disabled || adding;

  const addLocation = async () => {
    const workspace = destination?.workspace ?? recommended;

    if (!workspace) {
      return;
    }

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
        setOverride({ folderId, workspaceId: workspace.id });
      }
    } catch (error) {
      toast.add({
        description: errorMessage(error),
        title: "Couldn't add location",
        type: "error",
      });
    }
  };

  const download = async (next: Destination, automatic = false) => {
    if (locked || !isLink) {
      return;
    }

    setAdding(true);

    if (
      automatic &&
      (!preferences ||
        !(await savePreferences({
          ...preferences,
          aggregate_destination: {
            kind: "workspace",
            workspace_id: next.workspace.id,
          },
        })))
    ) {
      setAdding(false);

      return;
    }

    const { workspace, folder } = next;
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
      selectDestination(next);
      onUsed(workspace.id);
      setConfirming(false);
      setQuery("");
    }

    setAdding(false);
  };

  const submit = async (event: FormEvent) => {
    event.preventDefault();

    if (locked || !isLink) {
      return;
    }

    if (!destination) {
      setConfirming(true);

      return;
    }

    await download(destination);
  };

  return (
    <>
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
            <DestinationPicker
              workspaces={workspaces}
              destination={destination}
              viewedWorkspaceId={
                location.kind === "workspace" ? location.workspaceId : undefined
              }
              disabled={locked}
              onSelect={selectDestination}
              onAdd={() => void addLocation()}
            />
            <InputGroupButton
              type="submit"
              variant={isLink ? "default" : "ghost"}
              disabled={locked || !isLink || !workspaces.length}
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
          disabled={locked || !workspaces.length}
          onClick={() => {
            const target =
              destination ??
              naturalDestination(
                workspaces,
                { kind: "workspace", workspaceId: recommended?.id ?? "" },
                preferences
              );

            if (target) {
              onOptions(query, target.folder.id, target.workspace.id);
            }
          }}
        >
          <Settings2 />
        </Button>
      </form>
      {confirming && recommended && (
        <DestinationDialog
          workspaces={workspaces}
          recommended={recommended}
          canSaveDefault={preferences !== null}
          disabled={locked}
          onConfirm={download}
          onClose={() => setConfirming(false)}
          inputRef={inputRef}
        />
      )}
    </>
  );
};
