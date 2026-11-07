import { Button } from "@/components/ui/button";
import type { Download } from "@/lib/model";
import { updateVideoTools } from "@/lib/update-video-tools";

export const VideoToolsUpdateAdvice = ({ download }: { download: Download }) =>
  download.capabilities.can_update_video_tools ? (
    <div
      data-slot="video-tools-update-advice"
      className="flex flex-wrap items-center gap-2 text-xs"
    >
      <span>An update may fix this</span>
      <Button type="button" variant="link" size="sm" onClick={updateVideoTools}>
        Update
      </Button>
    </div>
  ) : null;
