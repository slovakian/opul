import type { CSSProperties, ReactNode } from "react";

import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import { Separator } from "@/components/ui/separator";
import { SidebarTrigger, useSidebar } from "@/components/ui/sidebar";
import { WorkspaceIcon } from "@/components/workspace-icon";
import type { Workspace } from "@/lib/model";
import {
  TRAFFIC_LIGHT_CLEARANCE_PX,
  hasMacOverlayTrafficLights,
} from "@/lib/titlebar";

export const WorkspaceHeader = ({
  title,
  workspace,
  parentWorkspace,
  onParent,
  children,
}: {
  title: string;
  workspace?: Workspace;
  parentWorkspace?: Workspace;
  onParent: () => void;
  children: ReactNode;
}) => {
  const { open, isMobile } = useSidebar();

  // Overlay lights sit in the header row when the sidebar column is gone.
  // Clear that leading column so controls are not under the lights.
  const clearTrafficLights =
    hasMacOverlayTrafficLights() && (!open || isMobile);

  // SAFETY: Custom CSS property --traffic-light-inset is valid in CSSProperties.
  const trafficLightStyle = {
    "--traffic-light-inset": `${TRAFFIC_LIGHT_CLEARANCE_PX}px`,
  } as CSSProperties;

  return (
    <header
      className="workspace-header shrink-0"
      data-traffic-lights={clearTrafficLights}
      style={trafficLightStyle}
    >
      <div
        data-tauri-drag-region
        className="header-row flex h-14 min-w-0 items-center gap-2 px-4"
      >
        <SidebarTrigger />
        <Separator
          orientation="vertical"
          className="mr-1 data-vertical:h-4 data-vertical:self-center"
        />
        <Breadcrumb
          className="header-identity min-w-0"
          title={parentWorkspace ? `${parentWorkspace.name} › ${title}` : title}
        >
          <BreadcrumbList data-tauri-drag-region className="flex-nowrap">
            {parentWorkspace && (
              <>
                <BreadcrumbItem className="header-parent min-w-0 shrink">
                  <BreadcrumbLink
                    render={
                      <button
                        type="button"
                        aria-label={parentWorkspace.name}
                        onClick={onParent}
                      />
                    }
                    className="flex min-w-0 items-center gap-2 overflow-hidden"
                  >
                    <span aria-hidden className="shrink-0">
                      <WorkspaceIcon workspace={parentWorkspace} />
                    </span>
                    <span className="truncate">{parentWorkspace.name}</span>
                  </BreadcrumbLink>
                </BreadcrumbItem>
                <BreadcrumbSeparator />
              </>
            )}
            <BreadcrumbItem
              data-tauri-drag-region
              className="header-current min-w-0"
            >
              <h1
                data-tauri-drag-region
                aria-label={title}
                aria-current="page"
                className="text-foreground flex min-w-0 items-center gap-2 text-sm font-medium"
              >
                {workspace && !parentWorkspace && (
                  <span data-tauri-drag-region aria-hidden className="shrink-0">
                    <WorkspaceIcon workspace={workspace} />
                  </span>
                )}
                <span data-tauri-drag-region className="truncate">
                  {title}
                </span>
              </h1>
            </BreadcrumbItem>
          </BreadcrumbList>
        </Breadcrumb>
        {children}
      </div>
    </header>
  );
};
