import { useState } from "react";
import type { RefObject } from "react";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { downloadSize } from "@/lib/video-tools";

export const VideoToolsConsent = ({
  size,
  busy,
  error,
  onDownload,
  onDecline,
  inputRef,
}: {
  size: number | null;
  busy: boolean;
  error: string;
  onDownload: () => Promise<void>;
  onDecline: (turnOff: boolean) => Promise<void>;
  inputRef: RefObject<HTMLInputElement | null>;
}) => {
  const [turnOff, setTurnOff] = useState(false);

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy) {
          void onDecline(turnOff);
        }
      }}
    >
      <DialogContent finalFocus={inputRef}>
        <DialogHeader>
          <DialogTitle>Video downloads need video tools</DialogTitle>
          <DialogDescription>
            Download video tools ({downloadSize(size)}). Your download will wait
            while they finish setting up.
          </DialogDescription>
        </DialogHeader>
        <label className="flex items-center gap-2 text-sm">
          <Checkbox
            checked={turnOff}
            onCheckedChange={(checked) => setTurnOff(checked === true)}
            disabled={busy}
          />
          Don&apos;t show this again
        </label>
        {error && (
          <p role="alert" className="text-destructive text-sm">
            {error}
          </p>
        )}
        <DialogFooter>
          <Button
            variant="ghost"
            disabled={busy}
            onClick={() => onDecline(turnOff)}
          >
            Not now
          </Button>
          <Button disabled={busy || turnOff} onClick={onDownload}>
            Download
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
