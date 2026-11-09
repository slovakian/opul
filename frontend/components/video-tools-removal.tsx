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
import { useVideoTools } from "@/hooks/use-video-tools";
import { useLibrarySelector } from "@/lib/library";
import { countDownloads } from "@/lib/library-store";

export const VideoToolsRemoval = () => {
  const { busy, error, remove } = useVideoTools();

  const count = useLibrarySelector((state) =>
    countDownloads(state, (download) => download.relies_on_video_tools)
  );

  const [removing, setRemoving] = useState(false);
  const [turnOff, setTurnOff] = useState(false);

  return (
    <>
      <Button
        type="button"
        variant="outline"
        size="sm"
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
              {count > 0 &&
                `${count} video ${count === 1 ? "download is" : "downloads are"} in progress. Removing the video tools stops them. `}
              Partial downloads and finished files are kept.
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
          {error && (
            <p role="alert" className="text-destructive text-xs">
              {error}
            </p>
          )}
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
                if (await remove(turnOff)) {
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
