import { useEffect, useState } from "react";
import { CheckCircle2, Sliders, Volume2, XCircle } from "lucide-react";
import { api } from "@/lib/api";
import { areSoundsEnabled, setSoundsEnabled } from "@/lib/sounds";
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
import type { MediaToolStatus, SystemPaths } from "@/types/opul";
import { toast } from "sonner";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  paths: SystemPaths | null;
}

export function PreferencesDialog({ open, onOpenChange, paths }: Props) {
  const [downloadLimitKB, setDownloadLimitKB] = useState("");
  const [uploadLimitKB, setUploadLimitKB] = useState("");
  const [maxMediaDownloads, setMaxMediaDownloads] = useState("3");
  const [soundEffects, setSoundEffects] = useState(areSoundsEnabled());
  const [toolStatus, setToolStatus] = useState<MediaToolStatus | null>(null);

  useEffect(() => {
    if (open) {
      api.getMediaToolStatus().then(setToolStatus).catch(console.error);
      api
        .getEnginePreferences()
        .then((prefs) => {
          if (prefs) {
            setDownloadLimitKB(
              prefs.max_download_bps > 0
                ? String(Math.round(prefs.max_download_bps / 1024))
                : "",
            );
            setUploadLimitKB(
              prefs.max_upload_bps > 0
                ? String(Math.round(prefs.max_upload_bps / 1024))
                : "",
            );
            setMaxMediaDownloads(
              String(prefs.max_concurrent_media_downloads || 3),
            );
          }
        })
        .catch(console.error);
    }
  }, [open]);

  const handleSave = async () => {
    try {
      const dlBps = downloadLimitKB.trim()
        ? parseFloat(downloadLimitKB) * 1024
        : 0;
      const ulBps = uploadLimitKB.trim() ? parseFloat(uploadLimitKB) * 1024 : 0;
      await api.setSpeedLimits(dlBps, ulBps);

      const maxMedia = parseInt(maxMediaDownloads, 10);
      if (!isNaN(maxMedia) && maxMedia > 0) {
        await api.setMaxConcurrentMediaDownloads(maxMedia);
      }

      setSoundsEnabled(soundEffects);
      toast.success("Preferences saved");
      onOpenChange(false);
    } catch (err) {
      console.error(err);
      toast.error(`Failed to save preferences: ${String(err)}`);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Sliders className="size-5 text-primary" />
            Preferences
          </DialogTitle>
          <DialogDescription>
            Configure system, transfer and engine settings.
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-5 py-3">
          <div className="space-y-2">
            <Label className="text-xs uppercase font-semibold tracking-wider text-muted-foreground">
              Storage Locations
            </Label>
            <div className="rounded-md border p-3 space-y-2 text-xs font-mono bg-muted/30">
              <div>
                <span className="text-muted-foreground">Downloads Folder:</span>
                <p className="truncate text-foreground font-medium">
                  {paths?.downloads_folder || "—"}
                </p>
              </div>
              <div>
                <span className="text-muted-foreground">
                  Application Support:
                </span>
                <p className="truncate text-foreground font-medium">
                  {paths?.support_folder || "—"}
                </p>
              </div>
            </div>
          </div>

          <div className="space-y-3">
            <Label className="text-xs uppercase font-semibold tracking-wider text-muted-foreground">
              Speed & Concurrency
            </Label>
            <div className="grid grid-cols-2 gap-3">
              <div className="grid gap-1.5">
                <Label htmlFor="dl-limit" className="text-xs">
                  Max Download (KB/s, 0=unlimited)
                </Label>
                <Input
                  id="dl-limit"
                  type="number"
                  min="0"
                  placeholder="Unlimited"
                  value={downloadLimitKB}
                  onChange={(e) => setDownloadLimitKB(e.target.value)}
                />
              </div>
              <div className="grid gap-1.5">
                <Label htmlFor="ul-limit" className="text-xs">
                  Max Upload (KB/s, 0=unlimited)
                </Label>
                <Input
                  id="ul-limit"
                  type="number"
                  min="0"
                  placeholder="Unlimited"
                  value={uploadLimitKB}
                  onChange={(e) => setUploadLimitKB(e.target.value)}
                />
              </div>
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="concurrent-media" className="text-xs">
                Concurrent Video / Media Downloads
              </Label>
              <Input
                id="concurrent-media"
                type="number"
                min="1"
                max="10"
                value={maxMediaDownloads}
                onChange={(e) => setMaxMediaDownloads(e.target.value)}
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label className="text-xs uppercase font-semibold tracking-wider text-muted-foreground">
              Media Engine Tools
            </Label>
            <div className="rounded-md border p-3 text-xs space-y-2 bg-muted/30">
              <div className="flex items-center justify-between">
                <span className="font-medium">yt-dlp:</span>
                <span className="flex items-center gap-1.5">
                  {toolStatus?.yt_dlp_path ? (
                    <>
                      <CheckCircle2 className="size-3.5 text-emerald-500" />
                      <span className="font-mono text-muted-foreground">
                        {toolStatus.yt_dlp_version || "Installed"}
                      </span>
                    </>
                  ) : (
                    <>
                      <XCircle className="size-3.5 text-amber-500" />
                      <span className="text-muted-foreground">
                        Not found in PATH
                      </span>
                    </>
                  )}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="font-medium">ffmpeg:</span>
                <span className="flex items-center gap-1.5">
                  {toolStatus?.ffmpeg_path ? (
                    <>
                      <CheckCircle2 className="size-3.5 text-emerald-500" />
                      <span className="font-mono text-muted-foreground">
                        {toolStatus.ffmpeg_version || "Installed"}
                      </span>
                    </>
                  ) : (
                    <>
                      <XCircle className="size-3.5 text-amber-500" />
                      <span className="text-muted-foreground">
                        Not found in PATH
                      </span>
                    </>
                  )}
                </span>
              </div>
            </div>
          </div>

          <div className="flex items-center space-x-2 pt-1">
            <Checkbox
              id="sound-fx"
              checked={soundEffects}
              onCheckedChange={(checked) => setSoundEffects(!!checked)}
            />
            <label
              htmlFor="sound-fx"
              className="text-xs font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 flex items-center gap-1.5 cursor-pointer"
            >
              <Volume2 className="size-3.5" />
              Enable interaction sound effects
            </label>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Close
          </Button>
          <Button onClick={handleSave}>Save Preferences</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
