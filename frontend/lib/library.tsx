import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";
import type { ReactNode } from "react";

import { toast } from "@/components/ui/toast";
import { bridge, errorMessage } from "@/lib/bridge";
import { createLibraryStore } from "@/lib/library-store";
import type { LibraryState, LibraryStore } from "@/lib/library-store";
import type {
  AggregateView,
  Command,
  DisplayPreferences,
  Preferences,
  Receipt,
  WorkspaceExpansion,
  WorkspaceExpansionChanges,
} from "@/lib/model";

type PreferenceStatus =
  | { kind: "unread" }
  | { kind: "failed"; message: string }
  | { kind: "ready" };

/** Library commands and preferences. The library itself is read with `useLibrarySelector`. */
interface LibraryActions {
  preferences: Preferences | null;
  preferenceStatus: PreferenceStatus;
  busy: boolean;
  /** Opens a new feed, which starts again from the whole library. */
  reconnect: () => void;
  retryPreferences: () => Promise<void>;
  saveWorkspaceExpansion: (
    choices: WorkspaceExpansionChanges
  ) => Promise<WorkspaceExpansion>;
  run: (
    command: Command,
    onAccepted?: (receipt: Receipt) => void
  ) => Promise<Receipt | null>;
  savePreferences: (preferences: Preferences) => Promise<boolean>;
  saveAggregateView: (
    view: AggregateView,
    preferences: DisplayPreferences
  ) => Promise<void>;
}

/** How long to wait before trying again when the feed can't open. */
const reconnectDelay = 1000;

/**
 * How long a command waits for the feed to show its result. The desktop
 * publishes before it replies, so this only matters if the feed broke.
 */
const acknowledgementTimeout = 2000;

const LibraryContext = createContext<LibraryActions | null>(null);
const LibraryStoreContext = createContext<LibraryStore | null>(null);

export const LibraryProvider = ({ children }: { children: ReactNode }) => {
  // One store for the provider's lifetime; it never needs to be recreated.
  const store = useMemo(() => createLibraryStore(), []);
  const [preferences, setPreferences] = useState<Preferences | null>(null);

  const [preferenceStatus, setPreferenceStatus] = useState<PreferenceStatus>({
    kind: "unread",
  });

  const preferenceRead = useRef(0);

  const preferenceSave = useRef(0);
  const aggregateViewSave = useRef(new Map<AggregateView, number>());
  const [pending, setPending] = useState(0);

  const connectFeed = useRef(() => null);
  const reconnect = useCallback(() => connectFeed.current(), []);

  useEffect(() => {
    let mounted = true;
    let connection = 0;
    let retry: ReturnType<typeof setTimeout> | undefined;

    // Each connection starts from the whole library, so a newer connection
    // makes everything an older one might still deliver irrelevant.
    const connect = async () => {
      clearTimeout(retry);
      connection += 1;
      const attempt = connection;
      const isCurrent = () => mounted && attempt === connection;

      try {
        await bridge.subscribeLibrary(
          (message) => {
            if (isCurrent()) {
              store.receive(message);
            }
          },
          () => {
            if (isCurrent()) {
              void connect();
            }
          }
        );
      } catch (error) {
        if (isCurrent()) {
          store.fail(errorMessage(error));
          retry = setTimeout(connect, reconnectDelay);
        }
      }
    };

    connectFeed.current = () => {
      void connect();

      return null;
    };

    void connect();

    return () => {
      mounted = false;
      clearTimeout(retry);
    };
  }, [store]);

  useEffect(() => {
    let stopped = false;

    const loadPreferences = async () => {
      const read = preferenceRead.current + 1;
      preferenceRead.current = read;
      setPreferenceStatus({ kind: "unread" });

      try {
        const initialPreferences = await bridge.preferences();

        if (!stopped && preferenceRead.current === read) {
          setPreferences(initialPreferences);
          setPreferenceStatus({ kind: "ready" });
        }
      } catch (error) {
        if (!stopped && preferenceRead.current === read) {
          setPreferenceStatus({
            kind: "failed",
            message: errorMessage(error),
          });
        }
      }
    };

    void loadPreferences();

    return () => {
      stopped = true;
    };
  }, []);

  const retryPreferences = useCallback(async () => {
    const read = preferenceRead.current + 1;
    preferenceRead.current = read;
    setPreferenceStatus({ kind: "unread" });

    try {
      const next = await bridge.preferences();

      if (preferenceRead.current !== read) {
        return;
      }

      setPreferences(next);
      setPreferenceStatus({ kind: "ready" });
    } catch (error) {
      if (preferenceRead.current === read) {
        setPreferenceStatus({ kind: "failed", message: errorMessage(error) });
      }
    }
  }, []);

  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)");

    const apply = () => {
      document.documentElement.classList.toggle(
        "dark",
        preferences?.theme === "dark" ||
          ((preferences?.theme ?? "system") === "system" && media.matches)
      );
    };

    apply();
    media.addEventListener("change", apply);

    return () => media.removeEventListener("change", apply);
  }, [preferences?.theme]);

  const run = useCallback(
    async (command: Command, onAccepted?: (receipt: Receipt) => void) => {
      const affectsBusy = command.kind !== "view";

      if (affectsBusy) {
        setPending((count) => count + 1);
      }

      try {
        const receipt = await bridge.execute(command);
        onAccepted?.(receipt);
        // Settle once the result is on screen, so controls never act on stale state.
        await store.reach(receipt.revision, acknowledgementTimeout);

        if (affectsBusy) {
          setPending((count) => count - 1);
        }

        return receipt;
      } catch (error) {
        if (affectsBusy) {
          setPending((count) => count - 1);
        }

        toast.add({
          description: errorMessage(error),
          title: "Couldn't complete that action",
          type: "error",
        });

        return null;
      }
    },
    [store]
  );

  const saveAggregateView = useCallback(
    async (view: AggregateView, display: DisplayPreferences) => {
      const request = (aggregateViewSave.current.get(view) ?? 0) + 1;
      aggregateViewSave.current.set(view, request);

      try {
        const saved = await bridge.saveAggregateView(view, display);

        if (aggregateViewSave.current.get(view) !== request) {
          return;
        }

        setPreferences((current) => {
          const next = {
            ...(current ?? saved),
            aggregate_views: {
              ...(current?.aggregate_views ?? saved.aggregate_views),
              [view]: saved.aggregate_views[view],
            },
          };

          return next;
        });
      } catch (error) {
        toast.add({
          description: errorMessage(error),
          title: "Couldn't save display preferences",
          type: "error",
        });
      }
    },
    []
  );

  const saveWorkspaceExpansion = useCallback(
    async (choices: WorkspaceExpansionChanges) => {
      const saved = await bridge.saveWorkspaceExpansion(choices);

      setPreferences((current) => {
        if (!current) {
          return current;
        }

        const next = {
          ...current,
          workspace_expansion: saved,
        };

        return next;
      });

      return saved;
    },
    []
  );

  const savePreferences = useCallback(async (next: Preferences) => {
    const request = preferenceSave.current + 1;
    preferenceSave.current = request;

    try {
      const saved = await bridge.savePreferences(next);

      if (preferenceSave.current !== request) {
        return true;
      }

      setPreferences((current) => {
        const nextPreferences = {
          ...(current ?? saved),
          ...saved,
          aggregate_views: current?.aggregate_views ?? saved.aggregate_views,
          workspace_expansion:
            current?.workspace_expansion ?? saved.workspace_expansion,
        };

        return nextPreferences;
      });
      toast.add({ title: "Settings saved" });

      return true;
    } catch (error) {
      toast.add({
        description: errorMessage(error),
        title: "Couldn't save settings",
        type: "error",
      });

      return false;
    }
  }, []);

  const value = useMemo(
    () => ({
      busy: pending > 0,
      preferenceStatus,
      preferences,
      reconnect,
      retryPreferences,
      run,
      saveAggregateView,
      savePreferences,
      saveWorkspaceExpansion,
    }),
    [
      pending,
      preferences,
      preferenceStatus,
      reconnect,
      retryPreferences,
      run,
      saveAggregateView,
      savePreferences,
      saveWorkspaceExpansion,
    ]
  );

  return (
    <LibraryStoreContext.Provider value={store}>
      <LibraryContext.Provider value={value}>
        {children}
      </LibraryContext.Provider>
    </LibraryStoreContext.Provider>
  );
};

export const useLibrary = () => {
  const library = useContext(LibraryContext);

  if (!library) {
    throw new Error("LibraryProvider is required");
  }

  return library;
};

export const useLibraryStore = () => {
  const store = useContext(LibraryStoreContext);

  if (!store) {
    throw new Error("LibraryProvider is required");
  }

  return store;
};

/**
 * Reads part of the library and renders again only when that part changes.
 * `isEqual` lets a selector that builds a new value each time (a count, a
 * pair) keep the previous one when it is equivalent.
 */
export const useLibrarySelector = <T,>(
  select: (state: LibraryState) => T,
  isEqual: (previous: T, next: T) => boolean = Object.is
): T => {
  const store = useLibraryStore();

  const selection = useRef<{
    select: typeof select;
    state: LibraryState;
    value: T;
  } | null>(null);

  const getSelection = () => {
    const state = store.getState();
    const previous = selection.current;

    if (previous?.state === state && previous.select === select) {
      return previous.value;
    }

    const next = select(state);

    const value =
      previous && isEqual(previous.value, next) ? previous.value : next;

    selection.current = { select, state, value };

    return value;
  };

  return useSyncExternalStore(store.subscribe, getSelection);
};
