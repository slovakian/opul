import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import type { Modal } from "@/components/app-modals";
import { checkForAppUpdates } from "@/hooks/use-app-updates";

interface UseAppKeyboardProps {
  focusInput: (select: boolean) => void;
  newDownload: () => void;
  setModal: (modal: Modal | null) => void;
}

export const useAppKeyboard = ({
  focusInput,
  newDownload,
  setModal,
}: UseAppKeyboardProps) => {
  useEffect(() => {
    const keydown = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey)) {
        return;
      }

      if (event.key === "n") {
        event.preventDefault();
        newDownload();
      }

      if (event.key === ",") {
        event.preventDefault();
        setModal({ kind: "settings" });
      }

      if (event.key === "f") {
        event.preventDefault();
        focusInput(false);
      }
    };

    window.addEventListener("keydown", keydown);

    if (!isTauri()) {
      return () => window.removeEventListener("keydown", keydown);
    }

    const subscriptions = [
      listen("settings", () => setModal({ kind: "settings" })),
      listen("new-download", () => {
        newDownload();
      }),
      // The app menu's Check for Updates… opens the Updates section with a check running.
      listen("check-for-updates", () => {
        setModal({ kind: "settings", section: "Updates" });
        void checkForAppUpdates();
      }),
    ];

    return () => {
      window.removeEventListener("keydown", keydown);

      for (const subscription of subscriptions) {
        void (async () => {
          try {
            const unlisten = await subscription;

            unlisten();
          } catch {
            // Ignore unlisten errors
          }
        })();
      }
    };
  }, [focusInput, newDownload, setModal]);
};
