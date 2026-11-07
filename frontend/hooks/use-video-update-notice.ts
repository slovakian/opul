import { useEffect, useRef } from "react";

import { toast } from "@/components/ui/toast";
import { bridge } from "@/lib/bridge";
import { updateVideoTools } from "@/lib/update-video-tools";
import type { VideoToolsSnapshot } from "@/lib/video-tools";

export const useVideoUpdateNotice = (video: VideoToolsSnapshot | undefined) => {
  const current = useRef<string | null>(null);

  const version =
    video &&
    !video.keep_up_to_date &&
    video.state.kind === "ready" &&
    video.state.update_flag.kind === "available"
      ? video.state.update_flag.version
      : null;

  useEffect(() => {
    current.current = version;

    if (!video || !version) {
      toast.close("video-tools-update");

      return;
    }

    const notify = async () => {
      try {
        const notice = await bridge.takeVideoToolsUpdateNotice();

        if (notice && current.current === notice.version) {
          toast.add({
            actionProps: {
              children: "Update",
              onClick: () => {
                toast.close("video-tools-update");
                void updateVideoTools();
              },
            },
            description: `${notice.version} is available.`,
            id: "video-tools-update",
            title: "Video tools update available",
          });
        }
      } catch {
        // A failed notification claim stays quiet and remains eligible on the next check.
      }
    };

    void notify();
  }, [version, video]);
};
