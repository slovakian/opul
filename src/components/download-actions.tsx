import { FolderOpen, Info, Pause, Play, RotateCw, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { MachineDownload } from "@/types/opul";

export interface DownloadActionHandlers {
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onRetry: (id: string) => void;
  onReveal: (path: string) => void;
  onInspect: (download: MachineDownload) => void;
  onRemove: (download: MachineDownload) => void;
}

interface DownloadActionsProps {
  download: MachineDownload;
  handlers: DownloadActionHandlers;
  size?: "sm" | "default";
}

export function DownloadActions({
  download,
  handlers,
  size = "default",
}: DownloadActionsProps) {
  const isPaused = download.state === "Paused";
  const isFailed = download.state === "Failed";
  const isTorrent = download.source === "Torrent";
  const btnSize = size === "sm" ? "size-6.5 p-0" : "size-7.5 p-0";
  const iconSize = size === "sm" ? "size-3" : "size-3.5";

  return (
    <div
      className="flex items-center gap-0.5"
      onClick={(e) => e.stopPropagation()}
    >
      {/* Resume */}
      {download.capabilities.can_resume && isPaused && (
        <Tooltip>
          <TooltipTrigger
            render={
              <Button
                variant="ghost"
                size="icon"
                className={`${btnSize} text-muted-foreground hover:text-foreground`}
                onClick={() => handlers.onResume(download.id)}
              >
                <Play className={iconSize} />
              </Button>
            }
          />
          <TooltipContent side="top">Resume</TooltipContent>
        </Tooltip>
      )}

      {/* Pause */}
      {download.capabilities.can_pause && !isPaused && (
        <Tooltip>
          <TooltipTrigger
            render={
              <Button
                variant="ghost"
                size="icon"
                className={`${btnSize} text-muted-foreground hover:text-foreground`}
                onClick={() => handlers.onPause(download.id)}
              >
                <Pause className={iconSize} />
              </Button>
            }
          />
          <TooltipContent side="top">Pause</TooltipContent>
        </Tooltip>
      )}

      {/* Retry */}
      {download.capabilities.can_retry && isFailed && (
        <Tooltip>
          <TooltipTrigger
            render={
              <Button
                variant="ghost"
                size="icon"
                className={`${btnSize} text-muted-foreground hover:text-foreground`}
                onClick={() => handlers.onRetry(download.id)}
              >
                <RotateCw className={iconSize} />
              </Button>
            }
          />
          <TooltipContent side="top">Retry</TooltipContent>
        </Tooltip>
      )}

      {/* Reveal in Finder */}
      {download.output_path && (
        <Tooltip>
          <TooltipTrigger
            render={
              <Button
                variant="ghost"
                size="icon"
                className={`${btnSize} text-muted-foreground hover:text-foreground`}
                onClick={() => handlers.onReveal(download.output_path!)}
              >
                <FolderOpen className={iconSize} />
              </Button>
            }
          />
          <TooltipContent side="top">Reveal in Finder</TooltipContent>
        </Tooltip>
      )}

      {/* Inspect (torrents) */}
      {isTorrent && (
        <Tooltip>
          <TooltipTrigger
            render={
              <Button
                variant="ghost"
                size="icon"
                className={`${btnSize} text-muted-foreground hover:text-foreground`}
                onClick={() => handlers.onInspect(download)}
              >
                <Info className={iconSize} />
              </Button>
            }
          />
          <TooltipContent side="top">Torrent Details</TooltipContent>
        </Tooltip>
      )}

      {/* Remove */}
      {download.capabilities.can_remove && (
        <Tooltip>
          <TooltipTrigger
            render={
              <Button
                variant="ghost"
                size="icon"
                className={`${btnSize} text-muted-foreground hover:text-destructive hover:bg-destructive/10`}
                onClick={() => handlers.onRemove(download)}
              >
                <Trash2 className={iconSize} />
              </Button>
            }
          />
          <TooltipContent side="top">Remove...</TooltipContent>
        </Tooltip>
      )}
    </div>
  );
}
