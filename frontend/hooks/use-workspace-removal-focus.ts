import { useEffect, useLayoutEffect, useRef } from "react";

import type { Workspace } from "@/lib/model";

export const useWorkspaceRemovalFocus = (
  workspaces: Workspace[],
  visible: boolean
) => {
  const focused = useRef<{
    control: HTMLElement;
    navigation: HTMLButtonElement;
  } | null>(null);

  useEffect(() => {
    const rememberFocus = (event: FocusEvent) => {
      const control = event.target;

      if (
        !(control instanceof HTMLElement) ||
        !control.matches("[data-folder-id], [data-workspace-disclosure]")
      ) {
        focused.current = null;

        return;
      }

      const navigation = control
        .closest("[data-workspace-id]")
        ?.querySelector<HTMLButtonElement>("[data-workspace-navigation]");

      focused.current = navigation ? { control, navigation } : null;
    };

    document.addEventListener("focusin", rememberFocus);

    return () => document.removeEventListener("focusin", rememberFocus);
  }, []);

  useLayoutEffect(() => {
    const previous = focused.current;

    if (!visible) {
      focused.current = null;
    } else if (
      previous &&
      !previous.control.isConnected &&
      workspaces.some(
        (workspace) =>
          workspace.id ===
          previous.navigation.closest<HTMLElement>("[data-workspace-id]")
            ?.dataset.workspaceId
      )
    ) {
      focused.current = null;

      if (previous.navigation.isConnected) {
        previous.navigation.focus();
      }
    }
  }, [visible, workspaces]);

  const workspaceFocusTarget = (
    workspaceId: string
  ): HTMLButtonElement | null => {
    if (!visible) {
      return null;
    }

    return (
      [
        ...document.querySelectorAll<HTMLButtonElement>(
          "[data-workspace-navigation]"
        ),
      ].find(
        (button) =>
          button.closest<HTMLElement>("[data-workspace-id]")?.dataset
            .workspaceId === workspaceId
      ) ?? null
    );
  };

  return workspaceFocusTarget;
};
