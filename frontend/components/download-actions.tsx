import {
  FolderOpen,
  MoreHorizontal,
  Pause,
  Play,
  RotateCcw,
} from "lucide-react";
import { useRef } from "react";

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
import { sendVideoTools } from "@/hooks/use-video-tools";
import { errorMessage } from "@/lib/bridge";
import { useLibrary } from "@/lib/library";
import type { Download, DownloadAction } from "@/lib/model";

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

  const act = (action: DownloadAction) => {
    run({ action, download_id: download.id, kind: "download" }).catch(
      () => null
    );
  };

  const recoverVideoTools = async () => {
    try {
      await sendVideoTools({
        kind: caps.can_retry_video_tools ? "retry" : "set_up",
      });
    } catch (error) {
      toast.add({
        description: errorMessage(error),
        title: "Couldn't set up video tools",
        type: "error",
      });
    }
  };

  const renderPrimaryAction = () => {
    if (caps.can_set_up_video_tools || caps.can_retry_video_tools) {
      return (
        <Button
          variant="ghost"
          size="sm"
          disabled={busy}
          onClick={recoverVideoTools}
        >
          {caps.can_retry_video_tools ? "Try again" : "Set up"}
        </Button>
      );
    }

    if (caps.can_pause) {
      return (
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={`Pause ${download.name}`}
          title="Pause download"
          disabled={busy}
          onClick={() => act("Pause")}
        >
          <Pause />
        </Button>
      );
    }

    if (caps.can_resume) {
      return (
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={`Resume ${download.name}`}
          title="Resume download"
          disabled={busy}
          onClick={() => act("Resume")}
        >
          <Play />
        </Button>
      );
    }

    if (caps.can_retry) {
      return (
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={`Retry ${download.name}`}
          title="Retry download"
          disabled={busy}
          onClick={() => act("Retry")}
        >
          <RotateCcw />
        </Button>
      );
    }

    if (caps.can_reveal_output) {
      return (
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={`Reveal ${download.name}`}
          title="Show in Finder"
          onClick={() => {
            reveal(download.id).catch(() => null);
          }}
        >
          <FolderOpen />
        </Button>
      );
    }

    return null;
  };

  return (
    <div className="flex items-center gap-1">
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
