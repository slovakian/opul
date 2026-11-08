import { cn } from "cn";
import { Info } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type * as React from "react";

import { Button } from "@/components/ui/button";
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

const BYTES_PER_MB = 1_000_000;

export const megabytes = (bytes: number) => (bytes / BYTES_PER_MB).toFixed(1);

export const InfoTip = ({
  children,
  label,
}: {
  children: React.ReactNode;
  label: string;
}) => (
  <Tooltip>
    <TooltipTrigger
      type="button"
      aria-label={label}
      className="text-muted-foreground/60 hover:text-foreground focus-visible:text-foreground focus-visible:ring-ring/50 inline-flex size-4 shrink-0 items-center justify-center rounded-full transition-colors duration-150 outline-none focus-visible:ring-3"
    >
      <Info className="size-3.5" />
    </TooltipTrigger>
    <TooltipContent className="max-w-60 leading-relaxed">
      {children}
    </TooltipContent>
  </Tooltip>
);

export const IconAction = ({
  children,
  disabled,
  icon: Icon,
  label,
  onClick,
  size = "icon-sm",
  spinning = false,
}: {
  children: string;
  disabled?: boolean;
  icon: LucideIcon;
  label: string;
  onClick: () => void;
  size?: "icon-sm" | "icon-xs";
  spinning?: boolean;
}) => (
  <Tooltip>
    <TooltipTrigger
      render={
        <Button
          type="button"
          variant="ghost"
          size={size}
          disabled={disabled}
          onClick={onClick}
        />
      }
    >
      <Icon
        aria-hidden="true"
        className={cn(spinning && "animate-spin [animation-duration:600ms]")}
      />
      {/* Real text keeps the accessible name and visible-text lookups aligned. */}
      <span className="sr-only">{children}</span>
    </TooltipTrigger>
    <TooltipContent>{label}</TooltipContent>
  </Tooltip>
);

export const SectionHeading = ({
  children,
  hint,
  id,
  trailing,
}: {
  children: string;
  hint: string;
  id: string;
  trailing?: React.ReactNode;
}) => (
  <div className="flex h-7 items-center gap-1.5 px-1">
    <h3 id={id} className="text-muted-foreground text-xs font-medium">
      {children}
    </h3>
    <InfoTip label={`About ${children.toLowerCase()}`}>{hint}</InfoTip>
    <div className="ml-auto flex items-center gap-1.5">{trailing}</div>
  </div>
);

export const ToolRow = ({
  active = false,
  description,
  dimmed = false,
  hint,
  icon: Icon,
  name,
  trailing,
}: {
  active?: boolean;
  description: string;
  dimmed?: boolean;
  hint?: string;
  icon: LucideIcon;
  name: string;
  trailing?: React.ReactNode;
}) => (
  <Item size="sm" className="flex-nowrap rounded-none px-4 py-3">
    <ItemMedia
      variant="icon"
      className={cn(
        "bg-muted size-8 translate-y-0! self-center! rounded-lg transition-opacity duration-200",
        dimmed && "text-muted-foreground opacity-60"
      )}
    >
      <Icon />
    </ItemMedia>
    <ItemContent className="min-w-0 gap-0.5">
      <ItemTitle className="line-clamp-none flex! gap-1.5">
        {name}
        {hint && <InfoTip label={`About ${name}`}>{hint}</InfoTip>}
      </ItemTitle>
      <ItemDescription
        key={description}
        className="animate-settle line-clamp-1 text-xs tabular-nums"
      >
        {/* shimmer and settle both drive `animation`, so they need separate elements. */}
        <span className={cn(active && "shimmer")}>{description}</span>
      </ItemDescription>
    </ItemContent>
    <ItemActions className="min-w-5 justify-end">{trailing}</ItemActions>
  </Item>
);

/** Hover detail for a ring: the exact percent and a linear bar. */
export const ProgressDetails = ({
  detail,
  label,
  percent,
}: {
  detail?: string;
  label: string;
  percent: number | null;
}) => (
  <div className="flex w-40 flex-col gap-1.5 py-0.5">
    <div className="flex items-center justify-between gap-2">
      <span>{label}</span>
      {percent !== null && <span className="tabular-nums">{percent}%</span>}
    </div>
    {percent !== null && (
      <div className="bg-background/20 h-1 overflow-hidden rounded-full">
        <div
          className="bg-background h-full rounded-full transition-[width] duration-300 ease-(--ease-out)"
          style={{ width: `${percent}%` }}
        />
      </div>
    )}
    {detail && (
      <span className="text-background/60 tabular-nums">{detail}</span>
    )}
  </div>
);
