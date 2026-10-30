import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
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
} from "@/lib/model";

type LibraryState =
  | { kind: "loading" }
  | { kind: "failed"; message: string }
  | { kind: "ready"; snapshot: Snapshot };

interface Library {
  state: LibraryState;
  preferences: Preferences | null;
  busy: boolean;
  refresh: () => Promise<void>;
  run: (command: Command) => Promise<Receipt | null>;
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
      setState({ kind: "failed", message: errorMessage(error) });
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
      try {
        const initialPreferences = await bridge.preferences();

        if (!stopped) {
          setPreferences(initialPreferences);
        }
      } catch {
        // Ignore initial preference read errors
      }
    };

    void poll();
    void loadPreferences();

    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  }, [refresh]);

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
    async (command: Command) => {
      const affectsBusy = command.kind !== "view";

      if (affectsBusy) {
        setPending((count) => count + 1);
      }

      try {
        const receipt = await bridge.execute(command);
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
      try {
        setPreferences(await bridge.saveAggregateView(view, display));
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

  const savePreferences = useCallback(async (next: Preferences) => {
    try {
      setPreferences(await bridge.savePreferences(next));
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
      preferences,
      refresh,
      run,
      saveAggregateView,
      savePreferences,
      state,
    }),
    [
      pending,
      preferences,
      refresh,
      run,
      saveAggregateView,
      savePreferences,
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
