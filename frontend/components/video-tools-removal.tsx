import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { useVideoTools } from "@/hooks/use-video-tools";
import { useLibrary } from "@/lib/library";

export const VideoToolsRemoval = ({
  busy,
  execute,
}: {
  busy: boolean;
  execute: ReturnType<typeof useVideoTools>["execute"];
}) => {
  const { state: library } = useLibrary();
  const [removing, setRemoving] = useState(false);
  const [turnOff, setTurnOff] = useState(false);

  const count =
    library.kind === "ready"
      ? library.snapshot.workspaces
          .flatMap((workspace) => workspace.downloads)
          .filter(
            (download) =>
              (download.needs_video_tools || download.source === "VideoPage") &&
              [
                "WaitingForVideoTools",
                "Queued",
                "Starting",
                "Fetching",
                "Processing",
                "Pausing",
              ].includes(download.state)
          ).length
      : 0;

  return (
    <>
      <Button
        type="button"
        variant="outline"
        disabled={busy}
        onClick={() => {
          setTurnOff(false);
          setRemoving(true);
        }}
      >
        Remove…
      </Button>
      <Dialog open={removing} onOpenChange={setRemoving}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Remove video tools?</DialogTitle>
            <DialogDescription>
              {count} video {count === 1 ? "download is" : "downloads are"} in
              progress. Removing the video tools stops them. Partial downloads
              and finished files are kept.
            </DialogDescription>
          </DialogHeader>
          <label className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={turnOff}
              disabled={busy}
              onCheckedChange={(checked) => setTurnOff(checked === true)}
            />
            Don&apos;t offer video tools again
          </label>
          <DialogFooter>
            <Button
              type="button"
              variant="ghost"
              disabled={busy}
              onClick={() => setRemoving(false)}
            >
              Cancel
            </Button>
            <Button
              type="button"
              disabled={busy}
              onClick={async () => {
                if (await execute({ kind: "remove", turn_off: turnOff })) {
                  setRemoving(false);
                }
              }}
            >
              Remove
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
};
