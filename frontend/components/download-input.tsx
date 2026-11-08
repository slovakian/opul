import { Download, Link, Loader2, Search, X } from "lucide-react";
import { useLayoutEffect, useRef, useState } from "react";
import type { Dispatch, FormEvent, RefObject, SetStateAction } from "react";

import { DestinationDialog } from "@/components/destination-dialog";
import { DestinationPicker } from "@/components/destination-picker";
import { DownloadOptions } from "@/components/download-options";
import { DownloadTypeDialog } from "@/components/download-type-dialog";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from "@/components/ui/input-group";
import { VideoToolsConsent } from "@/components/video-tools-consent";
import type { Location } from "@/components/workspace-sidebar";
import { useDownloadSubmission } from "@/hooks/use-download-submission";
import type { AddSavedFolder } from "@/hooks/use-saved-folder-addition";
import { naturalDestination, recommendedWorkspace } from "@/lib/destination";
import type { Destination } from "@/lib/destination";
import { isDownloadLink } from "@/lib/download-link";
import { useLibrary } from "@/lib/library";
import type { Folder, Preferences, Workspace } from "@/lib/model";

interface DestinationOverride {
  workspaceId: string;
  folderId: string;
  navigation: number;
}

const resolveDestination = (
  workspaces: Workspace[],
  override: DestinationOverride | undefined,
  navigation: number,
  location: Location,
  preferences: Preferences | null
): Destination | undefined => {
  const overrideWorkspace = workspaces.find(
    (item) =>
      override?.navigation === navigation && item.id === override.workspaceId
  );

  const overrideFolder = overrideWorkspace?.folders.find(
    (item) => item.id === override?.folderId
  );

  if (overrideWorkspace && overrideFolder) {
    return { folder: overrideFolder, workspace: overrideWorkspace };
  }

  return naturalDestination(workspaces, location, preferences);
};

const resolveOptionsFolder = (
  destination: Destination | undefined,
  workspace: Workspace | undefined
): Folder | undefined =>
  destination?.folder ??
  workspace?.folders.find((folder) => folder.is_default) ??
  workspace?.folders[0];

export const DownloadInput = ({
  workspaces,
  location,
  navigation,
  onAdded,
  onAddFolder,
  lastUsed,
  onUsed,
  disabled,
  query,
  setQuery,
  inputRef,
}: {
  workspaces: Workspace[];
  location: Location;
  navigation: number;
  onAdded: (ids: string[]) => void;
  onAddFolder: AddSavedFolder;
  lastUsed?: string;
  onUsed: (workspaceId: string) => void;
  disabled: boolean;
  query: string;
  setQuery: Dispatch<SetStateAction<string>>;
  inputRef: RefObject<HTMLInputElement | null>;
}) => {
  const { preferences } = useLibrary();
  const currentNavigation = useRef(navigation);

  useLayoutEffect(() => {
    currentNavigation.current = navigation;
  }, [navigation]);

  const [override, setOverride] = useState<DestinationOverride>();

  const destination = resolveDestination(
    workspaces,
    override,
    navigation,
    location,
    preferences
  );

  const recommended = recommendedWorkspace(workspaces, preferences, lastUsed);

  const selectDestination = (next: Destination) => {
    setOverride({
      folderId: next.folder.id,
      navigation,
      workspaceId: next.workspace.id,
    });
  };

  const isLink = isDownloadLink(query);

  const optionsWorkspace = destination?.workspace ?? recommended;
  const optionsFolder = resolveOptionsFolder(destination, optionsWorkspace);

  const submission = useDownloadSubmission({
    destination,
    disabled,
    navigation,
    onAdded,
    onUsed,
    query,
    selectDestination,
    setQuery,
    workspace: optionsWorkspace,
  });

  const { adding, locked } = submission;
  const { videoTurnedOff } = submission;

  const addLocation = async () => {
    const workspace = destination?.workspace ?? recommended;

    if (!workspace) {
      return;
    }

    await onAddFolder(
      workspace.id,
      (folderId) => {
        if (currentNavigation.current === navigation) {
          setOverride({ folderId, navigation, workspaceId: workspace.id });
        }
      },
      "Couldn't add location"
    );
  };

  const submit = async (event: FormEvent) => {
    event.preventDefault();

    if (isLink) {
      await submission.submit();
    }
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
              onAdd={() => {
                addLocation().catch(() => null);
              }}
            />
            <div className="inline-flex items-center">
              <InputGroupButton
                type="submit"
                variant={isLink ? "default" : "ghost"}
                className="rounded-r-none"
                disabled={locked || !isLink || !workspaces.length}
                aria-label="Download link"
                aria-busy={adding}
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
              <DownloadOptions
                workspace={optionsWorkspace}
                folder={optionsFolder}
                type={submission.type}
                format={submission.format}
                onType={(...args) => submission.setType(...args)}
                onFormat={(...args) => submission.setFormat(...args)}
                onTorrent={(...args) => submission.importTorrent(...args)}
                videoTurnedOff={videoTurnedOff}
                disabled={locked || !workspaces.length}
              />
            </div>
          </InputGroupAddon>
        </InputGroup>
      </form>
      {submission.videoConsent && (
        <VideoToolsConsent
          size={submission.videoToolsSize}
          busy={adding}
          error={submission.consentError}
          onDownload={() => submission.consent()}
          onDecline={(turnOff) => submission.declineVideoTools(turnOff)}
          inputRef={inputRef}
        />
      )}
      {submission.uncertain !== undefined && (
        <DownloadTypeDialog
          videoTurnedOff={videoTurnedOff}
          onChoose={(...args) => submission.chooseType(...args)}
          onClose={(...args) => submission.closeUncertain(...args)}
          inputRef={inputRef}
        />
      )}
      {submission.confirming && recommended && (
        <DestinationDialog
          workspaces={workspaces}
          recommended={recommended}
          canSaveDefault={preferences !== null}
          disabled={locked}
          onConfirm={(...args) => submission.confirm(...args)}
          onClose={(...args) => submission.closeConfirmation(...args)}
          inputRef={inputRef}
        />
      )}
    </>
  );
};
