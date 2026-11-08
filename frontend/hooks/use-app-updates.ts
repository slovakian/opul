import { useSyncExternalStore } from "react";

import { toast } from "@/components/ui/toast";
import type { AppUpdates } from "@/lib/app-updates";
import { bridge, errorMessage } from "@/lib/bridge";

interface AppUpdatesStore {
  updates: AppUpdates | null;
  /** The last failed action, shown where Settings can explain it. */
  error: string;
  /** Restart waits for confirmation while downloads are active. */
  confirmingRestart: boolean;
}

/** Holds an event that arrives before the first fetch finishes. */
interface EarlyUpdates {
  updates: AppUpdates | null;
}

let store: AppUpdatesStore = {
  confirmingRestart: false,
  error: "",
  updates: null,
};

let started = false;
const listeners = new Set<() => void>();

const update = (patch: Partial<AppUpdatesStore>) => {
  store = { ...store, ...patch };

  for (const listener of listeners) {
    listener();
  }
};

/** Snapshots arrive from events and from replies; the newest sequence wins. */
const accept = (updates: AppUpdates) => {
  if (!store.updates || updates.sequence >= store.updates.sequence) {
    update({ updates });
  }
};

/** Listens before fetching, so no change between the two is lost. Lives as long as the app. */
const load = async () => {
  const early: EarlyUpdates = { updates: null };

  const stopListening = await bridge.onAppUpdates((updates) => {
    if (store.updates) {
      accept(updates);
    } else if (!early.updates || updates.sequence > early.updates.sequence) {
      early.updates = updates;
    }
  });

  try {
    const fetched = await bridge.appUpdates();
    const latest = early.updates;

    accept(latest && latest.sequence > fetched.sequence ? latest : fetched);
  } catch (error) {
    stopListening();
    // The next screen that subscribes tries again.
    started = false;
    update({ error: errorMessage(error) });
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

/** Resolves to the failure message, or an empty string when the action was accepted. */
const act = async (work: () => Promise<AppUpdates>): Promise<string> => {
  update({ error: "" });

  try {
    accept(await work());

    return "";
  } catch (error) {
    const message = errorMessage(error);

    update({ error: message });

    return message;
  }
};

export const checkForAppUpdates = () => act(() => bridge.checkForAppUpdates());

export const downloadAppUpdate = () => act(() => bridge.downloadAppUpdate());

export const configureAppUpdates = (
  checkAutomatically: boolean,
  installAutomatically: boolean
) =>
  act(() =>
    bridge.configureAppUpdates({ checkAutomatically, installAutomatically })
  );

export const restartForAppUpdate = async () => {
  update({ confirmingRestart: false });

  try {
    await bridge.restartForAppUpdate();
  } catch (error) {
    toast.add({
      description: errorMessage(error),
      title: "Couldn't restart Opul",
      type: "error",
    });
  }
};

/** Restart right away, or ask first when it would interrupt downloads. */
export const requestAppUpdateRestart = async (activeDownloads: number) => {
  if (activeDownloads > 0) {
    update({ confirmingRestart: true });

    return;
  }

  await restartForAppUpdate();
};

export const cancelAppUpdateRestart = () =>
  update({ confirmingRestart: false });

/** The one updates snapshot the whole app shares. */
export const useAppUpdates = () => useSyncExternalStore(subscribe, getSnapshot);
