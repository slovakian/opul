import {
  FolderOpen,
  MoreHorizontal,
  Pause,
  Play,
  RefreshCw,
  RotateCcw,
  Wrench,
} from "lucide-react";
import { useRef } from "react";
import type { ReactNode } from "react";

import {
  DownloadMenuBody,
  downloadMenuPopup,
  reveal,
  useActionAfterClose,
  useDownloadMenuGroups,
} from "@/components/download-menu";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { toast } from "@/components/ui/toast";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { sendVideoTools, updateVideoTools } from "@/hooks/use-video-tools";
import { shortError } from "@/lib/bridge";
import { useLibrary } from "@/lib/library";
import type { Download, DownloadAction } from "@/lib/model";

/**
 * The row's one most useful action, always an icon button so the slot keeps
 * one width in every state. `hint` is the tooltip when it differs from the
 * accessible name, which names the download for screen readers.
 */
const PrimaryAction = ({
  label,
  hint = label,
  disabled,
  onClick,
  children,
}: {
  label: string;
  hint?: string;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
}) => (
  <Tooltip>
    <TooltipTrigger
      render={
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={label}
          disabled={disabled}
          onClick={onClick}
        />
      }
    >
      {children}
    </TooltipTrigger>
    <TooltipContent>{hint}</TooltipContent>
  </Tooltip>
);

export const DownloadActions = ({
  download,
  onInspect,
  onRemove,
  menuOpen,
  onMenuOpenChange,
}: {
  download: Download;
  onInspect: () => void;
  onRemove: () => void;
  /** Lets a row open the "…" menu from the keyboard (Shift+F10). */
  menuOpen?: boolean;
  onMenuOpenChange?: (open: boolean) => void;
}) => {
  const { run, busy } = useLibrary();
  const caps = download.capabilities;
  const groups = useDownloadMenuGroups({ download, onInspect, onRemove });
  const triggerRef = useRef<HTMLButtonElement>(null);

  const { handleOpenChangeComplete, handleSelect } =
    useActionAfterClose(triggerRef);

  // Pause and resume don't lock the app while Rust answers. Instead the
  // clicked button ignores further clicks until then, without greying out.
  const toggling = useRef(false);

  const act = (action: DownloadAction) => {
    run({ action, download_id: download.id, kind: "download" }).catch(
      () => null
    );
  };

  const toggle = async (action: "Pause" | "Resume") => {
    if (toggling.current) {
      return;
    }

    // `run` reports its own errors and never throws.
    toggling.current = true;
    await run({ action, download_id: download.id, kind: "download" });
    toggling.current = false;
  };

  const recoverVideoTools = async () => {
    try {
      await sendVideoTools({
        kind: caps.can_retry_video_tools ? "retry" : "set_up",
      });
    } catch (error) {
      toast.add({
        description: shortError(error),
        title: "Couldn't set up video tools",
        type: "error",
      });
    }
  };

  const renderPrimaryAction = () => {
    if (caps.can_set_up_video_tools || caps.can_retry_video_tools) {
      return (
        <PrimaryAction
          label={
            caps.can_retry_video_tools
              ? "Try setting up video tools again"
              : "Set up video tools"
          }
          disabled={busy}
          onClick={recoverVideoTools}
        >
          {caps.can_retry_video_tools ? <RotateCcw /> : <Wrench />}
        </PrimaryAction>
      );
    }

    if (caps.can_update_video_tools) {
      return (
        <PrimaryAction
          label="Update video tools"
          hint="An update may fix this"
          onClick={updateVideoTools}
        >
          <RefreshCw />
        </PrimaryAction>
      );
    }

    if (caps.can_pause) {
      return (
        <PrimaryAction
          label={`Pause ${download.name}`}
          hint="Pause download"
          onClick={() => toggle("Pause")}
        >
          <Pause />
        </PrimaryAction>
      );
    }

    if (caps.can_resume) {
      return (
        <PrimaryAction
          label={`Resume ${download.name}`}
          hint="Resume download"
          onClick={() => toggle("Resume")}
        >
          <Play />
        </PrimaryAction>
      );
    }

    if (caps.can_retry) {
      return (
        <PrimaryAction
          label={`Retry ${download.name}`}
          hint="Retry download"
          disabled={busy}
          onClick={() => act("Retry")}
        >
          <RotateCcw />
        </PrimaryAction>
      );
    }

    if (caps.can_reveal_output) {
      return (
        <PrimaryAction
          label={`Reveal ${download.name}`}
          hint="Show in Finder"
          onClick={() => {
            reveal(download.id).catch(() => null);
          }}
        >
          <FolderOpen />
        </PrimaryAction>
      );
    }

    // The slot keeps its width, so the "…" button never moves between states.
    return <span aria-hidden className="size-8 shrink-0" />;
  };

  return (
    <div className="flex shrink-0 items-center gap-1">
      {renderPrimaryAction()}
      <DropdownMenu
        open={menuOpen}
        onOpenChange={onMenuOpenChange}
        onOpenChangeComplete={handleOpenChangeComplete}
      >
        <DropdownMenuTrigger
          ref={triggerRef}
          render={
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={`Actions for ${download.name}`}
            >
              <MoreHorizontal />
            </Button>
          }
        />
        <DropdownMenuContent
          align="end"
          aria-label={`Actions for ${download.name}`}
          className={downloadMenuPopup}
        >
          <DownloadMenuBody groups={groups} onSelect={handleSelect} />
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
};
