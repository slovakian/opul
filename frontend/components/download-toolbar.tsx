import { ArrowUpDown, List, ListFilter, Table2 } from "lucide-react";
import { useLayoutEffect, useRef, useState } from "react";

import { SidebarPopover } from "@/components/sidebar-popover";
import { Button } from "@/components/ui/button";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import type { StatusFilter } from "@/components/workspace-sidebar";
import { groupingSchema, sortSchema } from "@/lib/model";
import type { DisplayPreferences } from "@/lib/model";

const filterOptions: { value: StatusFilter; label: string }[] = [
  { label: "All downloads", value: "all" },
  { label: "In progress", value: "active" },
  { label: "Completed", value: "finished" },
  { label: "Paused", value: "paused" },
  { label: "Failed", value: "failed" },
];

const sortNames: Record<DisplayPreferences["sort"], string> = {
  Name: "Name",
  Order: "Date added",
  Source: "Source",
  State: "Status",
};

const groupNames: Record<DisplayPreferences["grouping"], string> = {
  Folder: "Save location",
  Kind: "File type",
  None: "No grouping",
  Source: "Source",
  State: "Status",
};

export const DownloadToolbar = ({
  filter,
  setFilter,
  display,
  showFilter,
  view,
  disabled,
}: {
  filter: StatusFilter;
  setFilter: (filter: StatusFilter) => void;
  display: DisplayPreferences;
  showFilter: boolean;
  view: (change: Partial<DisplayPreferences>) => void;
  disabled: boolean;
}) => {
  const toolbarRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<HTMLButtonElement>(null);
  const [compact, setCompact] = useState(false);

  useLayoutEffect(() => {
    const header = toolbarRef.current?.closest("header");

    if (!header) {
      return;
    }

    let frame = 0;

    const observer = new ResizeObserver(([entry]) => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const nextCompact = entry.contentRect.width <= 900;

        if (
          compact &&
          !nextCompact &&
          viewRef.current?.getAttribute("aria-expanded") === "true"
        ) {
          viewRef.current.click();
        }

        if (
          nextCompact &&
          toolbarRef.current
            ?.querySelector('[data-slot="toggle-group"]')
            ?.contains(document.activeElement)
        ) {
          viewRef.current?.focus();
        }

        setCompact(nextCompact);
      });
    });

    observer.observe(header);

    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, [compact]);

  return (
    <div ref={toolbarRef} className="flex shrink-0 items-center gap-1">
      {showFilter && (
        <SidebarPopover
          title="Filter downloads"
          trigger={
            <Button
              variant={filter === "all" ? "ghost" : "secondary"}
              size="icon-sm"
              aria-pressed={filter !== "all"}
              aria-label={`${filter === "all" ? "Filter" : filterOptions.find((option) => option.value === filter)?.label} downloads`}
              title={
                filter === "all"
                  ? "Filter downloads"
                  : filterOptions.find((option) => option.value === filter)
                      ?.label
              }
            >
              <ListFilter />
            </Button>
          }
          groups={[
            {
              actions: filterOptions.map((option) => ({
                label: option.label,
                onClick: () => setFilter(option.value),
                selected: filter === option.value,
              })),
              label: "Status",
            },
          ]}
        />
      )}
      <SidebarPopover
        title={compact ? "View" : "Sort and group"}
        trigger={
          <Button
            ref={viewRef}
            variant="ghost"
            size="icon-sm"
            aria-label={
              compact ? "View" : `${sortNames[display.sort]}: sort and group`
            }
            title={compact ? "View: layout, sort and group" : "Sort and group"}
          >
            <ArrowUpDown />
          </Button>
        }
        groups={[
          ...(compact
            ? [
                {
                  actions: [
                    {
                      disabled,
                      icon: List,
                      label: "List view",
                      onClick: () => view({ layout: "List" }),
                      selected: display.layout === "List",
                    },
                    {
                      disabled,
                      icon: Table2,
                      label: "Table view",
                      onClick: () => view({ layout: "Table" }),
                      selected: display.layout === "Table",
                    },
                  ],
                  label: "Layout",
                },
              ]
            : []),
          {
            actions: sortSchema.options.map((value) => ({
              disabled,
              label: sortNames[value],
              onClick: () => view({ sort: value }),
              selected: display.sort === value,
            })),
            label: "Sort by",
          },
          {
            actions: groupingSchema.options.map((value) => ({
              disabled,
              label: groupNames[value],
              onClick: () => view({ grouping: value }),
              selected: display.grouping === value,
            })),
            label: "Group by",
          },
        ]}
      />
      {!compact && (
        <ToggleGroup
          value={[display.layout]}
          onValueChange={(values) => {
            const [layout] = values;

            if (layout === "List" || layout === "Table") {
              view({ layout });
            }
          }}
          spacing={0}
          size="sm"
          aria-label="Download view"
          disabled={disabled}
        >
          <ToggleGroupItem
            value="List"
            aria-label="List view"
            title="List view"
          >
            <List />
          </ToggleGroupItem>
          <ToggleGroupItem
            value="Table"
            aria-label="Table view"
            title="Table view"
          >
            <Table2 />
          </ToggleGroupItem>
        </ToggleGroup>
      )}
    </div>
  );
};
