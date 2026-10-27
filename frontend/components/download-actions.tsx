import {
  CircleStop,
  FolderOpen,
  Info,
  MoreHorizontal,
  Pause,
  Play,
  RotateCcw,
  Trash2,
  X,
} from "lucide-react";

import { SidebarPopover } from "@/components/sidebar-popover";
import type { MenuAction } from "@/components/sidebar-popover";
import { Button } from "@/components/ui/button";
import { toast } from "@/components/ui/toast";
import { bridge, errorMessage } from "@/lib/bridge";
import { useLibrary } from "@/lib/library";
import type { Download, DownloadAction } from "@/lib/model";

export function DownloadActions({
  download,
  onInspect,
  onRemove,
}: {
  download: Download;
  onInspect: () => void;
  onRemove: () => void;
}) {
  const { run, busy } = useLibrary();
  const caps = download.capabilities;

  function act(action: DownloadAction) {
    void run({ action, download_id: download.id, kind: "download" });
  }

  const actions: MenuAction[] = [
    { icon: Info, label: "Show details", onClick: onInspect },
  ];

  if (caps.can_reveal_output) {
    actions.push({
      icon: FolderOpen,
      label: "Show in Finder",
      onClick: () => void reveal(download.id),
    });
  }

  if (caps.can_pause) {
    actions.push({
      disabled: busy,
      icon: Pause,
      label: "Pause download",
      onClick: () => act("Pause"),
    });
  }

  if (caps.can_resume) {
    actions.push({
      disabled: busy,
      icon: Play,
      label: "Resume download",
      onClick: () => act("Resume"),
    });
  }

  if (caps.can_retry) {
    actions.push({
      disabled: busy,
      icon: RotateCcw,
      label: "Retry download",
      onClick: () => act("Retry"),
    });
  }

  if (caps.can_stop_seeding) {
    actions.push({
      disabled: busy,
      icon: CircleStop,
      label: "Stop seeding",
      onClick: () => act("StopSeeding"),
    });
  }

  if (caps.can_cancel) {
    actions.push({
      disabled: busy,
      icon: X,
      label: "Cancel download",
      onClick: () => act("Cancel"),
    });
  }

  return (
    <div className="flex items-center gap-1">
      {caps.can_pause ? (
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
      ) : caps.can_resume ? (
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
      ) : caps.can_retry ? (
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
      ) : caps.can_reveal_output ? (
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={`Reveal ${download.name}`}
          title="Show in Finder"
          onClick={() => void reveal(download.id)}
        >
          <FolderOpen />
        </Button>
      ) : null}
      <SidebarPopover
        title="Download actions"
        trigger={
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={`Actions for ${download.name}`}
          >
            <MoreHorizontal />
          </Button>
        }
        groups={[
          { actions },
          {
            actions: [
              {
                disabled: busy || !caps.can_remove,
                icon: Trash2,
                label: "Remove from library",
                onClick: onRemove,
              },
            ],
          },
        ]}
      />
    </div>
  );
}

export async function reveal(id: string) {
  try {
    await bridge.reveal(id);
  } catch (error) {
    toast.add({
      description: errorMessage(error),
      title: "Couldn't reveal this download",
      type: "error",
    });
  }
}
