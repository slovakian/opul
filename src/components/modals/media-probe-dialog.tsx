import { useEffect, useState } from "react";
import { Download, Loader2, Video } from "lucide-react";
import { api } from "@/lib/api";
import { playSound } from "@/lib/sounds";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { MachineWorkspace, MediaProbe } from "@/types/opul";
import { toast } from "sonner";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  url: string;
  workspace: MachineWorkspace;
  onSuccess: () => void;
}

export function MediaProbeDialog({
  open,
  onOpenChange,
  url,
  workspace,
  onSuccess,
}: Props) {
  const [loading, setLoading] = useState(false);
  const [probe, setProbe] = useState<MediaProbe | null>(null);
  const [format, setFormat] = useState<string>(
    workspace.default_format || "Best",
  );
  const [folderId, setFolderId] = useState<string>(
    workspace.folders.find((f) => f.is_default)?.id ??
      workspace.folders[0]?.id ??
      "",
  );
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (!open || !url) return;
    let active = true;
    Promise.resolve().then(() => {
      if (active) {
        setLoading(true);
        setProbe(null);
      }
    });

    api
      .probeMedia(url)
      .then((res) => {
        if (!active) return;
        setProbe(res);
      })
      .catch((err) => {
        if (!active) return;
        console.error("Probe error:", err);
        toast.error(`Unable to probe media: ${String(err)}`);
        onOpenChange(false);
      })
      .finally(() => {
        if (active) setLoading(false);
      });

    return () => {
      active = false;
    };
  }, [open, url, onOpenChange]);

  const handleStartDownload = async () => {
    if (!probe) return;
    setSubmitting(true);
    try {
      const items = probe.items.map((item) => ({
        url: item.webpage_url,
        title: item.title,
        page_url: item.webpage_url,
        thumbnail_url: item.thumbnail_url,
        extractor: item.extractor,
        playlist_index: item.playlist_index,
      }));

      await api.addMedia({
        workspaceId: workspace.id,
        folderId,
        format,
        items,
      });

      playSound("tap");
      toast.success(
        probe.is_playlist
          ? `Added playlist with ${items.length} items to download`
          : `Started download: ${probe.title}`,
      );
      onSuccess();
      onOpenChange(false);
    } catch (err) {
      console.error(err);
      toast.error(`Failed to start download: ${String(err)}`);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[520px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Video className="size-5 text-primary" />
            Download Media
          </DialogTitle>
          <DialogDescription className="truncate font-mono text-xs">
            {url}
          </DialogDescription>
        </DialogHeader>

        {loading ? (
          <div className="flex flex-col items-center justify-center py-12 gap-3 text-muted-foreground">
            <Loader2 className="size-8 animate-spin text-primary" />
            <p className="text-sm">
              Inspecting stream metadata and available formats...
            </p>
          </div>
        ) : probe ? (
          <div className="grid gap-4 py-2">
            {probe.thumbnail_url && (
              <div className="relative aspect-video w-full overflow-hidden rounded-md bg-muted border">
                <img
                  src={probe.thumbnail_url}
                  alt={probe.title}
                  className="h-full w-full object-cover"
                />
              </div>
            )}

            <div className="space-y-1">
              <h4 className="font-semibold text-sm line-clamp-2">
                {probe.title}
              </h4>
              <p className="text-xs text-muted-foreground">
                Source: {probe.extractor || "Direct / Web"}
                {probe.is_playlist &&
                  ` • Playlist (${probe.items.length} items)`}
              </p>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="grid gap-2">
                <Label>Format</Label>
                <Select
                  value={format}
                  onValueChange={(val) => {
                    if (val) setFormat(val);
                  }}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="Best">Best Available</SelectItem>
                    <SelectItem value="BestMp4">Best MP4</SelectItem>
                    <SelectItem value="P1080">1080p Full HD</SelectItem>
                    <SelectItem value="P720">720p HD</SelectItem>
                    <SelectItem value="AudioMp3">Audio MP3</SelectItem>
                    <SelectItem value="AudioM4a">Audio M4A</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="grid gap-2">
                <Label>Destination Folder</Label>
                <Select
                  value={folderId}
                  onValueChange={(val) => {
                    if (val) setFolderId(val);
                  }}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {workspace.folders.map((f) => (
                      <SelectItem key={f.id} value={f.id}>
                        {f.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
          </div>
        ) : null}

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={submitting}
          >
            Cancel
          </Button>
          <Button
            onClick={handleStartDownload}
            disabled={loading || !probe || submitting}
            className="gap-2"
          >
            {submitting ? (
              <Loader2 className="size-4 animate-spin" />
            ) : (
              <Download className="size-4" />
            )}
            Download
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
