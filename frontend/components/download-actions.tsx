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

export const reveal = async (id: string) => {
  try {
    await bridge.reveal(id);
  } catch (error) {
    toast.add({
      description: errorMessage(error),
      title: "Couldn't reveal this download",
      type: "error",
    });
  }
};

export const DownloadActions = ({
  download,
  onInspect,
  onRemove,
}: {
  download: Download;
  onInspect: () => void;
  onRemove: () => void;
}) => {
  const { run, busy } = useLibrary();
  const caps = download.capabilities;

  const act = (action: DownloadAction) => {
    run({ action, download_id: download.id, kind: "download" }).catch(
      () => null
    );
  };

  const actions: MenuAction[] = [
    { icon: Info, label: "Show details", onClick: onInspect },
  ];

  if (caps.can_reveal_output) {
    actions.push({
      icon: FolderOpen,
      label: "Show in Finder",
      onClick: () => {
        reveal(download.id).catch(() => null);
      },
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

  const renderPrimaryAction = () => {
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
};
