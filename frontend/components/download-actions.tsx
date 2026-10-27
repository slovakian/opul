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
import { Button } from "@/components/ui/button";
import { SidebarPopover, type MenuAction } from "@/components/sidebar-popover";
import { useLibrary } from "@/lib/library";
import { bridge, errorMessage } from "@/lib/bridge";
import { toast } from "@/components/ui/toast";
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
    void run({ kind: "download", download_id: download.id, action });
  }
  const actions: MenuAction[] = [
    { label: "Show details", icon: Info, onClick: onInspect },
  ];
  if (caps.can_reveal_output)
    actions.push({
      label: "Show in Finder",
      icon: FolderOpen,
      onClick: () => void reveal(download.id),
    });
  if (caps.can_pause)
    actions.push({
      label: "Pause download",
      icon: Pause,
      onClick: () => act("Pause"),
      disabled: busy,
    });
  if (caps.can_resume)
    actions.push({
      label: "Resume download",
      icon: Play,
      onClick: () => act("Resume"),
      disabled: busy,
    });
  if (caps.can_retry)
    actions.push({
      label: "Retry download",
      icon: RotateCcw,
      onClick: () => act("Retry"),
      disabled: busy,
    });
  if (caps.can_stop_seeding)
    actions.push({
      label: "Stop seeding",
      icon: CircleStop,
      onClick: () => act("StopSeeding"),
      disabled: busy,
    });
  if (caps.can_cancel)
    actions.push({
      label: "Cancel download",
      icon: X,
      onClick: () => act("Cancel"),
      disabled: busy,
    });
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
                label: "Remove from library",
                icon: Trash2,
                onClick: onRemove,
                disabled: busy || !caps.can_remove,
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
      title: "Couldn't reveal this download",
      description: errorMessage(error),
      type: "error",
    });
  }
}
