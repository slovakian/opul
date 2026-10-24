import { useState } from "react";
import { HardDrive, Network } from "lucide-react";
import { api } from "@/lib/api";
import { formatBytes } from "@/lib/format";
import { playSound } from "@/lib/sounds";
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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { MachineDownload } from "@/types/opul";
import { toast } from "sonner";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  download: MachineDownload | null;
}

export function TorrentInspectorDialog({
  open,
  onOpenChange,
  download,
}: Props) {
  if (!download) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <TorrentInspectorDialogContent
        key={download.id}
        download={download}
        onOpenChange={onOpenChange}
      />
    </Dialog>
  );
}

function TorrentInspectorDialogContent({
  download,
  onOpenChange,
}: {
  download: MachineDownload;
  onOpenChange: (open: boolean) => void;
}) {
  const [seedLimit, setSeedLimit] = useState<string>(
    download.seed_ratio_limit ? String(download.seed_ratio_limit) : "",
  );
  const [selectedFileIndices, setSelectedFileIndices] = useState<Set<number>>(
    () => new Set(download.files.map((f) => f.index)),
  );
  const [isSaving, setIsSaving] = useState(false);

  const handleToggleFile = (index: number) => {
    setSelectedFileIndices((prev) => {
      const next = new Set(prev);
      if (next.has(index)) {
        if (next.size > 1) {
          next.delete(index);
        } else {
          toast.warning("At least one file must remain selected");
        }
      } else {
        next.add(index);
      }
      return next;
    });
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      // Update seed ratio
      const limitVal = seedLimit.trim() ? parseFloat(seedLimit) : null;
      await api.setSeedRatio(download.id, limitVal);

      // Update files if applicable
      if (download.files.length > 0) {
        const updatedFiles = download.files.map((f) => ({
          ...f,
          priority: selectedFileIndices.has(f.index) ? "normal" : "skip",
        }));
        await api.setTorrentFiles(download.id, updatedFiles);
      }

      playSound("tap");
      toast.success("Updated torrent settings");
      onOpenChange(false);
    } catch (err) {
      console.error(err);
      toast.error(`Failed to update: ${String(err)}`);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <DialogContent className="sm:max-w-[600px] max-h-[85vh] flex flex-col">
      <DialogHeader>
        <DialogTitle className="truncate">{download.name}</DialogTitle>
        <DialogDescription className="font-mono text-xs">
          Hash: {download.info_hash || "N/A"}
        </DialogDescription>
      </DialogHeader>

      <div className="flex-1 overflow-y-auto space-y-4 py-2 pr-1">
        <div className="grid grid-cols-2 gap-3 text-xs bg-muted/40 p-3 rounded-lg border">
          <div>
            <span className="text-muted-foreground">Total Size:</span>{" "}
            <span className="font-mono font-medium">
              {formatBytes(download.total_size)}
            </span>
          </div>
          <div>
            <span className="text-muted-foreground">Downloaded:</span>{" "}
            <span className="font-mono font-medium">
              {formatBytes(download.bytes_downloaded)}
            </span>
          </div>
          <div>
            <span className="text-muted-foreground">Uploaded:</span>{" "}
            <span className="font-mono font-medium">
              {formatBytes(download.bytes_uploaded)}
            </span>
          </div>
          <div>
            <span className="text-muted-foreground">Current Ratio:</span>{" "}
            <span className="font-mono font-medium">
              {download.seed_ratio.toFixed(2)}
            </span>
          </div>
          <div>
            <span className="text-muted-foreground">Peers / Seeds:</span>{" "}
            <span className="font-mono font-medium">
              {download.peers} / {download.seeds}
            </span>
          </div>
          <div>
            <span className="text-muted-foreground">Pieces:</span>{" "}
            <span className="font-mono font-medium">
              {download.piece_count} ({formatBytes(download.piece_length)}/piece)
            </span>
          </div>
        </div>

        {/* Seed ratio setting */}
        <div className="space-y-1.5">
          <Label htmlFor="seed-limit" className="text-xs">
            Seed Ratio Limit (leave blank for default)
          </Label>
          <Input
            id="seed-limit"
            type="number"
            step="0.1"
            min="0"
            placeholder="e.g. 2.0"
            value={seedLimit}
            onChange={(e) => setSeedLimit(e.target.value)}
            className="h-8 text-xs font-mono"
          />
        </div>

        {/* File selection */}
        {download.files.length > 0 && (
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-xs font-semibold flex items-center gap-1.5">
                <HardDrive className="size-3.5" />
                Files ({selectedFileIndices.size}/{download.files.length} selected)
              </Label>
              <div className="flex gap-2 text-[11px]">
                <button
                  type="button"
                  className="text-primary hover:underline"
                  onClick={() =>
                    setSelectedFileIndices(
                      new Set(download.files.map((f) => f.index)),
                    )
                  }
                >
                  Select All
                </button>
              </div>
            </div>

            <div className="border rounded-md divide-y max-h-56 overflow-y-auto text-xs">
              {download.files.map((file) => {
                const isSelected = selectedFileIndices.has(file.index);
                return (
                  <label
                    key={file.index}
                    className="flex items-center gap-2 p-2 hover:bg-muted/50 cursor-pointer"
                  >
                    <Checkbox
                      checked={isSelected}
                      onCheckedChange={() => handleToggleFile(file.index)}
                    />
                    <span className="truncate flex-1 font-mono text-[11px]" title={file.path}>
                      {file.path}
                    </span>
                    <span className="text-[11px] font-mono text-muted-foreground shrink-0">
                      {formatBytes(file.size)}
                    </span>
                  </label>
                );
              })}
            </div>
          </div>
        )}

        {/* Trackers */}
        {download.trackers.length > 0 && (
          <div className="space-y-1.5">
            <Label className="text-xs font-semibold flex items-center gap-1.5">
              <Network className="size-3.5" />
              Trackers ({download.trackers.length})
            </Label>
            <div className="border rounded-md p-2 bg-muted/20 max-h-28 overflow-y-auto font-mono text-[11px] space-y-1 text-muted-foreground">
              {download.trackers.map((tracker, i) => (
                <div key={i} className="truncate" title={tracker}>
                  {tracker}
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <DialogFooter className="pt-2 border-t">
        <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button size="sm" onClick={handleSave} disabled={isSaving}>
          {isSaving ? "Saving..." : "Save Settings"}
        </Button>
      </DialogFooter>
    </DialogContent>
  );
}
