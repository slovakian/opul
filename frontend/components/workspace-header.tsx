import { isTauri } from "@tauri-apps/api/core";
import type { ReactNode } from "react";

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
  const iconWorkspace = workspace ?? parentWorkspace;

  const reserveTrafficLights =
    isTauri() && /Mac/u.test(navigator.userAgent) && (!open || isMobile);

  return (
    <header
      className="workspace-header shrink-0"
      data-traffic-lights={reserveTrafficLights}
    >
      <div
        data-tauri-drag-region
        className="header-row flex h-14 min-w-0 items-center gap-2 px-4"
      >
        <SidebarTrigger />
        <Separator
          orientation="vertical"
          className="data-vertical:h-4 data-vertical:self-center"
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
                    className="truncate"
                  >
                    {parentWorkspace.name}
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
                {iconWorkspace && (
                  <span data-tauri-drag-region aria-hidden className="shrink-0">
                    <WorkspaceIcon workspace={iconWorkspace} />
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
