import { useState } from "react";
import type { RefObject } from "react";

import { Choice } from "@/components/sidebar-popover";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { downloadTypes } from "@/lib/download-options";
import type { DownloadMode } from "@/lib/download-options";

export const DownloadTypeDialog = ({
  onChoose,
  onClose,
  inputRef,
}: {
  onChoose: (mode: DownloadMode) => void;
  onClose: () => void;
  inputRef: RefObject<HTMLInputElement | null>;
}) => {
  const [mode, setMode] = useState<DownloadMode | "">("");

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
    >
      <DialogContent finalFocus={inputRef}>
        <DialogHeader>
          <DialogTitle>Choose download type</DialogTitle>
          <DialogDescription>
            This link could be a file, video, or torrent. Choose how to download
            it.
          </DialogDescription>
        </DialogHeader>
        <Choice
          label="Download type"
          value={mode}
          options={downloadTypes.filter((item) => item.value !== "Automatic")}
          onChange={setMode}
        />
        <DialogFooter>
          <Button type="button" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button
            type="button"
            disabled={!mode}
            onClick={() => {
              if (mode) {
                onChoose(mode);
              }
            }}
          >
            Download
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
