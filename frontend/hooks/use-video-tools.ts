import { useCallback, useState, useSyncExternalStore } from "react";

import { toast } from "@/components/ui/toast";
import { bridge, errorMessage, shortError } from "@/lib/bridge";
import type {
  ToolsSnapshot,
  VideoToolsOperation,
  VideoToolsSnapshot,
} from "@/lib/video-tools";

interface VideoToolsStore {
  /** Operations still waiting for their reply, from any screen. */
  pending: number;
  loadError: string;
  tools: ToolsSnapshot | null;
}

/** Holds an event that arrives before the first fetch finishes. */
interface EarlySnapshot {
  video: VideoToolsSnapshot | null;
}

let store: VideoToolsStore = { loadError: "", pending: 0, tools: null };
let started = false;
const listeners = new Set<() => void>();

const update = (patch: Partial<VideoToolsStore>) => {
  store = { ...store, ...patch };

  for (const listener of listeners) {
    listener();
  }
};

let sizeRequested = false;

/** The offer shows the download size, so ask for it whenever it's unknown. Metadata only. */
const requestSizeIfUnknown = async (video: VideoToolsSnapshot) => {
  if (video.state.kind !== "not_set_up" || video.published_size !== null) {
    sizeRequested = false;

    return;
  }

  if (sizeRequested) {
    return;
  }

  sizeRequested = true;

  try {
    await bridge.videoToolsExecute({ kind: "refresh_published_size" });
  } catch {
    // The offer falls back to an estimate; the next unknown snapshot asks again.
    sizeRequested = false;
  }
};

/** Snapshots arrive from events and from replies; the newest sequence wins. */
const accept = (video: VideoToolsSnapshot) => {
  if (store.tools && video.sequence >= store.tools.video.sequence) {
    update({ tools: { ...store.tools, video } });
    void requestSizeIfUnknown(video);
  }
};

/** Listens before fetching, so no change between the two is lost. Lives as long as the app. */
const load = async () => {
  const early: EarlySnapshot = { video: null };

  const stopListening = await bridge.onVideoTools((video) => {
    if (store.tools) {
      accept(video);
    } else if (!early.video || video.sequence > early.video.sequence) {
      early.video = video;
    }
  });

  try {
    const snapshot = await bridge.toolsSnapshot();
    const latest = early.video;

    const tools =
      latest && latest.sequence > snapshot.video.sequence
        ? { ...snapshot, video: latest }
        : snapshot;

    update({ loadError: "", tools });
    void requestSizeIfUnknown(tools.video);
  } catch (error) {
    stopListening();
    // The next screen that subscribes tries again.
    started = false;
    update({ loadError: errorMessage(error) });
  }
};

const subscribe = (listener: () => void) => {
  listeners.add(listener);

  if (!started) {
    started = true;
    void load();
  }

  return () => {
    listeners.delete(listener);
  };
};

const getSnapshot = () => store;

/** Send an operation and keep its reply. Throws, so callers report errors their own way. */
const send = async (work: () => Promise<VideoToolsSnapshot>) => {
  update({ pending: store.pending + 1 });

  try {
    const video = await work();
    accept(video);

    return video;
  } finally {
    update({ pending: store.pending - 1 });
  }
};

export const sendVideoTools = (operation: VideoToolsOperation) =>
  send(() => bridge.videoToolsExecute(operation));

/** Starts an update and reports a failure as a toast, for surfaces with no error area. */
export const updateVideoTools = async () => {
  try {
    await sendVideoTools({ kind: "update" });
  } catch (error) {
    toast.add({
      description: shortError(error),
      title: "Couldn't update video tools",
      type: "error",
    });
  }
};

/**
 * The one tools snapshot the whole app shares. Errors stay with the screen whose operation
 * failed, so one surface never shows another's failure.
 */
export const useVideoTools = () => {
  const { loadError, pending, tools } = useSyncExternalStore(
    subscribe,
    getSnapshot
  );

  const [error, setError] = useState("");

  const run = useCallback(async (work: () => Promise<VideoToolsSnapshot>) => {
    setError("");

    try {
      await send(work);

      return true;
    } catch (error_) {
      setError(errorMessage(error_));

      return false;
    }
  }, []);

  /** Resolves false and keeps the error on this screen when the operation fails. */
  const execute = useCallback(
    (operation: VideoToolsOperation) =>
      run(() => bridge.videoToolsExecute(operation)),
    [run]
  );

  const remove = useCallback(
    (turnOff: boolean) => run(() => bridge.videoToolsRemove(turnOff)),
    [run]
  );

  return {
    busy: pending > 0,
    error: error || loadError,
    execute,
    remove,
    tools,
  };
};
