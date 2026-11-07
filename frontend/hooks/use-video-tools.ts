import { useCallback, useEffect, useState } from "react";

import { bridge, errorMessage } from "@/lib/bridge";
import type {
  ToolsSnapshot,
  VideoToolsOperation,
  VideoToolsSnapshot,
} from "@/lib/video-tools";

export const useVideoTools = () => {
  const [tools, setTools] = useState<ToolsSnapshot | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const accept = useCallback((video: VideoToolsSnapshot) => {
    setTools((current) =>
      current && video.sequence >= current.video.sequence
        ? { ...current, video }
        : current
    );
  }, []);

  useEffect(() => {
    let stopped = false;
    let unlisten: (() => void) | undefined;
    let latest: VideoToolsSnapshot | null = null;

    const load = async () => {
      unlisten = await bridge.onVideoTools((video) => {
        latest = video;

        if (!stopped) {
          accept(video);
        }
      });

      if (stopped) {
        unlisten();

        return;
      }

      const snapshot = await bridge.toolsSnapshot();

      if (stopped) {
        return;
      }

      setTools(
        latest && latest.sequence > snapshot.video.sequence
          ? { ...snapshot, video: latest }
          : snapshot
      );

      if (snapshot.video.state.kind === "not_set_up") {
        await bridge.videoToolsExecute({ kind: "refresh_published_size" });
      }
    };

    load().catch((error_) => {
      if (!stopped) {
        setError(errorMessage(error_));
      }
    });

    return () => {
      stopped = true;
      unlisten?.();
    };
  }, [accept]);

  const execute = async (operation: VideoToolsOperation) => {
    setBusy(true);
    setError("");

    try {
      accept(await bridge.videoToolsExecute(operation));
    } catch (error_) {
      setError(errorMessage(error_));
    }

    setBusy(false);
  };

  return { busy, error, execute, tools };
};
