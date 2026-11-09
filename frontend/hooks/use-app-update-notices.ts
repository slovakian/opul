import { useEffect, useRef } from "react";

import { toast } from "@/components/ui/toast";
import {
  downloadAppUpdate,
  requestAppUpdateRestart,
  useAppUpdates,
} from "@/hooks/use-app-updates";
import {
  appName,
  downloadPercent,
  installsAutomatically,
} from "@/lib/app-updates";
import type { AppUpdatePhase, AppUpdates } from "@/lib/app-updates";
import { bridge } from "@/lib/bridge";

/** One toast follows an update from offer to progress to ready, changing in place. */
const TOAST_ID = "opul-update";

type Notice = Parameters<typeof toast.add<{ dismissLabel?: string }>>[0];

const download = async () => {
  const failure = await downloadAppUpdate();

  if (failure) {
    toast.add({
      data: {},
      description: failure,
      id: TOAST_ID,
      timeout: 0,
      title: "Couldn't download the update",
      type: "error",
    });
  }
};

/** The progress, ready and failure toasts a phase should show, keyed so each shows once. */
const followUp = (
  phase: AppUpdatePhase,
  name: string,
  restart: () => void
): { key: string; notice: Notice } | null => {
  if (phase.kind === "downloading" && !phase.automatic) {
    const percent = downloadPercent(phase);

    return {
      key: `downloading:${phase.version}:${percent}`,
      notice: {
        // Upserts merge, so clear the offer's buttons explicitly.
        actionProps: undefined,
        data: {},
        description: percent === null ? undefined : `${percent}%`,
        timeout: 0,
        title: `Downloading ${name} ${phase.version}…`,
        type: "loading",
      },
    };
  }

  if (phase.kind === "ready") {
    return {
      key: `ready:${phase.version}`,
      notice: {
        actionProps: { children: "Restart", onClick: restart },
        data: { dismissLabel: "Later" },
        description: "Restart to finish updating.",
        timeout: 0,
        title: `${name} ${phase.version} is ready`,
        type: undefined,
      },
    };
  }

  if (phase.kind === "download_failed" && !phase.automatic) {
    return {
      key: `failed:${phase.version}:${phase.message}`,
      notice: {
        actionProps: {
          children: "Try again",
          onClick: () => {
            void download();
          },
        },
        data: {},
        description: phase.message,
        timeout: 0,
        title: `Couldn't download ${name} ${phase.version}`,
        type: "error",
      },
    };
  }

  return null;
};

const offeredVersion = (updates: AppUpdates | null): string | null => {
  if (
    !updates ||
    updates.phase.kind !== "available" ||
    updates.install_blocker ||
    installsAutomatically(updates)
  ) {
    return null;
  }

  return updates.phase.version;
};

/** Update notifications for the whole app. Opul never restarts without a click. */
export const useAppUpdateNotices = (activeDownloads: number) => {
  const { updates } = useAppUpdates();
  const active = useRef(activeDownloads);
  const shown = useRef<string | null>(null);
  const offered = offeredVersion(updates);
  const name = updates ? appName(updates) : "Opul";
  const phase = updates?.phase;

  useEffect(() => {
    active.current = activeDownloads;
  }, [activeDownloads]);

  // Rust claims the notice, so a version offers itself once, even across launches.
  useEffect(() => {
    if (!offered) {
      return;
    }

    let stopped = false;

    const offer = async () => {
      try {
        const claimed = await bridge.takeAppUpdateNotice();

        if (stopped || claimed !== offered) {
          return;
        }

        shown.current = `offer:${claimed}`;
        toast.add({
          actionProps: {
            children: "Download",
            onClick: () => {
              void download();
            },
          },
          data: { dismissLabel: "Not now" },
          id: TOAST_ID,
          timeout: 0,
          title: `${name} ${claimed} is available`,
        });
      } catch {
        // A failed claim stays quiet; Settings still offers the update.
      }
    };

    void offer();

    return () => {
      stopped = true;
    };
  }, [offered, name]);

  useEffect(() => {
    if (!phase) {
      return;
    }

    const next = followUp(phase, name, () => {
      void requestAppUpdateRestart(active.current);
    });

    if (next) {
      if (next.key !== shown.current) {
        shown.current = next.key;
        toast.add({ ...next.notice, id: TOAST_ID });
      }
    } else if (shown.current?.startsWith("downloading:")) {
      shown.current = null;
      toast.close(TOAST_ID);
    }
  }, [phase, name]);
};
