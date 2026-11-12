import { isTauri } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";

import { toast } from "@/components/ui/toast";
import { useVideoTools } from "@/hooks/use-video-tools";
import { useVideoUpdateNotice } from "@/hooks/use-video-update-notice";
import { bridge, shortError } from "@/lib/bridge";
import { useLibrary } from "@/lib/library";

export const useVideoSetup = () => {
  const videoTools = useVideoTools();
  useVideoUpdateNotice(videoTools.tools?.video);
  const { preferences } = useLibrary();
  const [open, setOpen] = useState(false);
  const offer = useRef<Promise<boolean> | null>(null);
  const handled = useRef(false);
  const previous = useRef<string | undefined>(undefined);
  const kind = videoTools.tools?.video.state.kind;
  const shown = preferences?.video_tools_setup_shown;
  const handleShow = useCallback(() => setOpen(true), []);

  useEffect(() => {
    if (shown !== false || !kind || handled.current) {
      return;
    }

    let stopped = false;

    const remember = async () => {
      try {
        if (!offer.current) {
          offer.current = bridge.markVideoToolsSetupShown();
        }

        const first = await offer.current;

        if (!stopped) {
          handled.current = true;

          if (first && kind !== "turned_off" && kind !== "ready") {
            setOpen(true);
          }
        }
      } catch (error) {
        if (!stopped) {
          toast.add({
            description: shortError(error),
            title: "Couldn't remember Setup",
          });
        }
      }
    };

    void remember();

    return () => {
      stopped = true;
    };
  }, [shown, kind]);

  // Setup shows its own finish, so the toast is only for people who left it.
  useEffect(() => {
    if (previous.current === "setting_up" && kind === "ready" && !open) {
      toast.add({ id: "video-tools-ready", title: "Video tools are ready" });
    }

    previous.current = kind;
  }, [kind, open]);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    const subscription = bridge.onSetUpVideoToolsMenu(handleShow);

    return () => {
      void (async () => {
        const unlisten = await subscription;
        unlisten();
      })();
    };
  }, [handleShow]);

  return { ...videoTools, close: () => setOpen(false), handleShow, open };
};
