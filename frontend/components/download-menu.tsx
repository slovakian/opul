import { Menu } from "@base-ui/react/menu";
import {
  CircleStop,
  FolderOpen,
  Info,
  Pause,
  Play,
  RotateCcw,
  Trash2,
  X,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Fragment, useRef } from "react";
import type { RefObject } from "react";

import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarSeparator,
} from "@/components/ui/sidebar";
import { toast } from "@/components/ui/toast";
import { bridge, errorMessage } from "@/lib/bridge";
import { useLibrary } from "@/lib/library";
import type { Download, DownloadAction } from "@/lib/model";

export const reveal = async (id: string) => {
  try {
    await bridge.reveal(id);
  } catch (error) {
    toast.add({
      description: errorMessage(error),
      title: "Couldn't reveal this download",
      type: "error",
    });
  }
};

export interface DownloadMenuAction {
  label: string;
  icon: LucideIcon;
  onClick: () => void;
  disabled?: boolean;
}

/** Each inner list is one group; groups render with a separator between them. */
export type DownloadMenuGroups = DownloadMenuAction[][];

/** Builds the actions shared by a download's "…" menu and its right-click menu. */
export const useDownloadMenuGroups = ({
  download,
  onInspect,
  onRemove,
}: {
  download: Download;
  onInspect: () => void;
  onRemove: () => void;
}): DownloadMenuGroups => {
  const { run, busy } = useLibrary();
  const caps = download.capabilities;

  const act = (action: DownloadAction) => {
    run({ action, download_id: download.id, kind: "download" }).catch(
      () => null
    );
  };

  const actions: DownloadMenuAction[] = [
    { icon: Info, label: "Show details", onClick: onInspect },
  ];

  if (caps.can_reveal_output) {
    actions.push({
      icon: FolderOpen,
      label: "Show in Finder",
      onClick: () => {
        reveal(download.id).catch(() => null);
      },
    });
  }

  const transfers: [boolean, LucideIcon, string, DownloadAction][] = [
    [caps.can_pause, Pause, "Pause download", "Pause"],
    [caps.can_resume, Play, "Resume download", "Resume"],
    [caps.can_retry, RotateCcw, "Retry download", "Retry"],
    [caps.can_stop_seeding, CircleStop, "Stop seeding", "StopSeeding"],
    [caps.can_cancel, X, "Cancel download", "Cancel"],
  ];

  for (const [allowed, icon, label, action] of transfers) {
    if (allowed) {
      actions.push({ disabled: busy, icon, label, onClick: () => act(action) });
    }
  }

  return [
    actions,
    [
      {
        disabled: busy || !caps.can_remove,
        icon: Trash2,
        label: "Remove from library",
        onClick: onRemove,
      },
    ],
  ];
};

/**
 * Holds the chosen action until its menu has finished closing, then focuses
 * `returnFocus` before running it. Anything the action opens (the details
 * panel, the remove dialog) records the focused element to return to later,
 * and while the menu is open its focus trap would refuse the move.
 */
export const useActionAfterClose = (
  returnFocus: RefObject<HTMLElement | null>
) => {
  const pending = useRef<(() => void) | null>(null);

  return {
    handleOpenChangeComplete: (open: boolean) => {
      const action = pending.current;
      pending.current = null;

      if (!open && action) {
        returnFocus.current?.focus();
        action();
      }
    },
    handleSelect: (action: () => void) => {
      pending.current = action;
    },
  };
};

// Base UI marks the keyboard/pointer target with data-highlighted rather than focus.
const highlighted =
  "data-highlighted:bg-sidebar-accent data-highlighted:text-sidebar-accent-foreground";

/** Popup classes shared by both menus so they look identical. */
export const downloadMenuPopup = "w-60 overflow-hidden p-0";

/**
 * Sidebar-styled menu body. Items render as Base UI menu items, so this works
 * inside both a dropdown menu and a context menu (they share Menu parts).
 */
export const DownloadMenuBody = ({
  groups,
  onSelect,
}: {
  groups: DownloadMenuGroups;
  /** Receives the chosen action; pair with `useActionAfterClose`. */
  onSelect: (action: () => void) => void;
}) => (
  <Sidebar collapsible="none" className="w-auto bg-transparent">
    <SidebarContent className="max-h-96 gap-0">
      {groups.map((actions, index) => (
        <Fragment key={actions[0]?.label ?? index}>
          {index > 0 && <SidebarSeparator />}
          <Menu.Group render={<SidebarGroup />}>
            <SidebarMenu role="none">
              {actions.map((action) => (
                <SidebarMenuItem key={action.label} role="none">
                  <SidebarMenuButton
                    render={
                      <Menu.Item
                        disabled={action.disabled}
                        onClick={() => onSelect(action.onClick)}
                      />
                    }
                    className={highlighted}
                  >
                    <action.icon />
                    <span>{action.label}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </Menu.Group>
        </Fragment>
      ))}
    </SidebarContent>
  </Sidebar>
);
