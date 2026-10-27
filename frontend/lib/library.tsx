import {
  createContext,
  useCallback,
  useContext,
  useEffect,
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

export function LibraryProvider({ children }: { children: ReactNode }) {
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

    async function poll() {
      await refresh();

      if (!stopped) {
        timer = setTimeout(poll, 800);
      }
    }

    void poll();
    void bridge
      .preferences()
      .then(setPreferences)
      .catch(() => {});

    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  }, [refresh]);
  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)");

    function apply() {
      document.documentElement.classList.toggle(
        "dark",
        preferences?.theme === "dark" ||
          ((preferences?.theme ?? "system") === "system" && media.matches)
      );
    }

    apply();
    media.addEventListener("change", apply);

    return () => media.removeEventListener("change", apply);
  }, [preferences?.theme]);

  const run = useCallback(
    async (command: Command) => {
      setPending((count) => count + 1);

      try {
        const receipt = await bridge.execute(command);
        await refresh();

        return receipt;
      } catch (error) {
        toast.add({
          description: errorMessage(error),
          title: "Couldn't complete that action",
          type: "error",
        });

        return null;
      } finally {
        setPending((count) => count - 1);
      }
    },
    [refresh]
  );

  const saveAggregateView = useCallback(
    async (view: AggregateView, display: DisplayPreferences) => {
      setPending((count) => count + 1);

      try {
        setPreferences(await bridge.saveAggregateView(view, display));
      } catch (error) {
        toast.add({
          description: errorMessage(error),
          title: "Couldn't save display preferences",
          type: "error",
        });
      }

      setPending((count) => count - 1);
    },
    []
  );

  async function savePreferences(next: Preferences) {
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
  }

  return (
    <LibraryContext.Provider
      value={{
        busy: pending > 0,
        preferences,
        refresh,
        run,
        saveAggregateView,
        savePreferences,
        state,
      }}
    >
      {children}
    </LibraryContext.Provider>
  );
}

export function useLibrary() {
  const library = useContext(LibraryContext);

  if (!library) {
    throw new Error("LibraryProvider is required");
  }

  return library;
}
