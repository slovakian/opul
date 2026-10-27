import { ArrowUpDown, List, ListFilter, Table2 } from "lucide-react";

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

export function DownloadToolbar({
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
}) {
  return (
    <div className="flex shrink-0 items-center gap-1">
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
        title="Sort and group"
        trigger={
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={`${sortNames[display.sort]}: sort and group`}
            title="Sort and group"
          >
            <ArrowUpDown />
          </Button>
        }
        groups={[
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
      <ToggleGroup
        value={[display.layout]}
        onValueChange={(values) => {
          const layout = values[0];

          if (layout === "List" || layout === "Table") {
            view({ layout });
          }
        }}
        spacing={0}
        size="sm"
        aria-label="Download view"
        disabled={disabled}
      >
        <ToggleGroupItem value="List" aria-label="List view" title="List view">
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
    </div>
  );
}
