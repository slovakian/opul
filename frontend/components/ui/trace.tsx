import { cn } from "cn";
import { ChevronRightIcon } from "lucide-react";
import type * as React from "react";

import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";

export const Trace = ({ className, ...props }: React.ComponentProps<"div">) => (
  <div
    data-slot="trace"
    className={cn(
      "*:has-[>[data-slot=trace-line]]:focus-visible:ring-ring/50 flex w-full min-w-0 flex-col gap-2 *:has-[>[data-slot=trace-line]]:flex *:has-[>[data-slot=trace-line]]:w-full *:has-[>[data-slot=trace-line]]:max-w-full *:has-[>[data-slot=trace-line]]:min-w-0 *:has-[>[data-slot=trace-line]]:rounded-md *:has-[>[data-slot=trace-line]]:text-left *:has-[>[data-slot=trace-line]]:outline-none *:has-[>[data-slot=trace-line]]:focus-visible:ring-3",
      className
    )}
    {...props}
  />
);

export const TraceLine = ({
  className,
  status,
  ...props
}: React.ComponentProps<"span"> & {
  status?: "running" | "error" | "pending";
}) => (
  <span
    data-slot="trace-line"
    data-status={status}
    className={cn(
      "text-muted-foreground in-[a:hover,button:hover]:text-foreground data-[status=error]:text-destructive data-[status=error]:in-[a:hover,button:hover]:text-destructive data-[status=running]:*:data-[slot=trace-title]:shimmer flex w-full min-w-0 items-center gap-2 text-sm transition-colors in-[a,button]:cursor-pointer data-[status=pending]:opacity-60",
      className
    )}
    {...props}
  />
);

export const TraceIcon = ({
  className,
  ...props
}: React.ComponentProps<"span">) => (
  <span
    data-slot="trace-icon"
    aria-hidden="true"
    className={cn(
      "flex size-4 shrink-0 items-center justify-center [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4",
      className
    )}
    {...props}
  />
);

export const TraceTitle = ({
  className,
  ...props
}: React.ComponentProps<"span">) => (
  <span
    data-slot="trace-title"
    className={cn(
      "motion-reduce:shimmer-none min-w-0 flex-1 truncate",
      className
    )}
    {...props}
  />
);

export const TraceDescription = ({
  className,
  ...props
}: React.ComponentProps<"span">) => (
  <span
    data-slot="trace-description"
    className={cn(
      "text-muted-foreground/70 shrink-0 text-xs tabular-nums",
      className
    )}
    {...props}
  />
);

export const TraceCollapsible = ({
  className,
  ...props
}: Omit<React.ComponentProps<typeof Collapsible>, "open"> &
  (
    | { open?: undefined }
    | {
        open: boolean;
        onOpenChange: NonNullable<
          React.ComponentProps<typeof Collapsible>["onOpenChange"]
        >;
      }
  )) => (
  <Collapsible
    data-slot="trace-collapsible"
    className={cn("flex w-full min-w-0 flex-col", className)}
    {...props}
  />
);

export const TraceCollapsibleTrigger = ({
  className,
  children,
  ...props
}: React.ComponentProps<typeof CollapsibleTrigger>) => (
  <CollapsibleTrigger
    data-slot="trace-collapsible-trigger"
    className={cn(
      "group/trace-collapsible-trigger text-muted-foreground hover:text-foreground focus-visible:ring-ring/50 has-[[data-status=error]]:text-destructive flex w-fit max-w-full min-w-0 cursor-pointer items-center gap-1 rounded-md text-left transition-colors outline-none focus-visible:ring-3",
      className
    )}
    {...props}
  >
    {children}
    <TraceIcon className="transition-transform group-aria-expanded/trace-collapsible-trigger:rotate-90">
      <ChevronRightIcon />
    </TraceIcon>
  </CollapsibleTrigger>
);

export const TraceCollapsibleContent = ({
  className,
  children,
  ...props
}: React.ComponentProps<typeof CollapsibleContent>) => (
  <CollapsibleContent
    data-slot="trace-collapsible-content"
    className="h-(--collapsible-panel-height) min-w-0 overflow-hidden transition-[height] duration-200 data-ending-style:h-0 data-starting-style:h-0"
    {...props}
  >
    <div
      data-slot="trace-collapsible-content-inner"
      className={cn(
        "text-muted-foreground mt-2 mb-1 ml-[calc(--spacing(2)-1px)] min-w-0 border-l pr-1 pl-4 leading-relaxed wrap-break-word",
        className
      )}
    >
      {children}
    </div>
  </CollapsibleContent>
);
