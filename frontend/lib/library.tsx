import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { ReactNode } from "react";

import { toast } from "@/components/ui/toast";
import { bridge, errorMessage } from "@/lib/bridge";
import type {
  AggregateView,
  Command,
  DisplayPreferences,
  Preferences,
  Receipt,
  Snapshot,
  WorkspaceExpansion,
  WorkspaceExpansionChanges,
} from "@/lib/model";

type LibraryState =
  | { kind: "loading" }
  | { kind: "failed"; message: string }
  | { kind: "ready"; snapshot: Snapshot };

type PreferenceStatus =
  | { kind: "unread" }
  | { kind: "failed"; message: string }
  | { kind: "ready" };

interface Library {
  state: LibraryState;
  preferences: Preferences | null;
  preferenceStatus: PreferenceStatus;
  busy: boolean;
  refresh: () => Promise<void>;
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

const LibraryContext = createContext<Library | null>(null);

export const LibraryProvider = ({ children }: { children: ReactNode }) => {
  const [state, setState] = useState<LibraryState>({ kind: "loading" });
  const [preferences, setPreferences] = useState<Preferences | null>(null);

  const [preferenceStatus, setPreferenceStatus] = useState<PreferenceStatus>({
    kind: "unread",
  });

  const preferenceRead = useRef(0);

  const preferenceSave = useRef(0);
  const aggregateViewSave = useRef(new Map<AggregateView, number>());
  const [pending, setPending] = useState(0);

  const refresh = useCallback(async () => {
    try {
      const snapshot = await bridge.snapshot();
      setState((previous) =>
        previous.kind === "ready" &&
        previous.snapshot.boot_id === snapshot.boot_id &&
        previous.snapshot.sequence > snapshot.sequence
          ? previous
          : { kind: "ready", snapshot }
      );
    } catch (error) {
      // A failed read is not evidence that library containers disappeared.
      setState((previous) =>
        previous.kind === "ready"
          ? previous
          : { kind: "failed", message: errorMessage(error) }
      );
    }
  }, []);

  useEffect(() => {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;

    const poll = async () => {
      try {
        await refresh();
      } catch {
        // Ignore poll refresh errors
      }

      if (!stopped) {
        timer = setTimeout(poll, 800);
      }
    };

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

    void poll();
    void loadPreferences();

    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  }, [refresh]);

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
        await refresh();

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
    [refresh]
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
      refresh,
      retryPreferences,
      run,
      saveAggregateView,
      savePreferences,
      saveWorkspaceExpansion,
      state,
    }),
    [
      pending,
      preferences,
      preferenceStatus,
      refresh,
      retryPreferences,
      run,
      saveAggregateView,
      savePreferences,
      saveWorkspaceExpansion,
      state,
    ]
  );

  return (
    <LibraryContext.Provider value={value}>{children}</LibraryContext.Provider>
  );
};

export const useLibrary = () => {
  const library = useContext(LibraryContext);

  if (!library) {
    throw new Error("LibraryProvider is required");
  }

  return library;
};
