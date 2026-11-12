import { ChevronRight, FolderPlus, Plus, Settings2 } from "lucide-react";
import type { CSSProperties } from "react";

import opulLogo from "@/assets/opul-logo.png";
import { SidebarPopover } from "@/components/sidebar-popover";
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import {
  TRAFFIC_LIGHT_CLEARANCE_PX,
  hasMacOverlayTrafficLights,
} from "@/lib/titlebar";

/**
 * Opul logo/dropdown on the Overlay chrome midY with traffic lights and the
 * content header. Clears the light column instead of stacking under it.
 */
export const OpulLogoRow = ({
  onNew,
  onCreate,
  onSettings,
}: {
  onNew: () => void;
  onCreate: () => void;
  onSettings: () => void;
}) => {
  const clearTrafficLights = hasMacOverlayTrafficLights();

  // SAFETY: Custom CSS property --traffic-light-inset is valid in CSSProperties.
  const trafficLightStyle = {
    "--traffic-light-inset": `${TRAFFIC_LIGHT_CLEARANCE_PX}px`,
  } as CSSProperties;

  return (
    <div
      data-tauri-drag-region
      data-traffic-lights={clearTrafficLights}
      className="opul-logo-row flex h-14 items-center px-3"
      style={trafficLightStyle}
    >
      <SidebarMenu>
        <SidebarMenuItem>
          <SidebarPopover
            align="start"
            title="Opul"
            trigger={
              <SidebarMenuButton className="w-fit">
                <img src={opulLogo} alt="" className="size-6 shrink-0" />
                <span className="font-semibold">Opul</span>
                <ChevronRight className="rotate-90 opacity-50" />
              </SidebarMenuButton>
            }
            groups={[
              {
                actions: [
                  {
                    hint: "⌘N",
                    icon: Plus,
                    label: "New download",
                    onClick: onNew,
                  },
                  {
                    icon: FolderPlus,
                    label: "New workspace",
                    onClick: onCreate,
                  },
                ],
              },
              {
                actions: [
                  {
                    hint: "⌘,",
                    icon: Settings2,
                    label: "Settings",
                    onClick: onSettings,
                  },
                ],
              },
            ]}
          />
        </SidebarMenuItem>
      </SidebarMenu>
    </div>
  );
};
