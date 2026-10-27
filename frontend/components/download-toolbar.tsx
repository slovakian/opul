import { ArrowUpDown, List, ListFilter, Search, Table2 } from "lucide-react";
import type { Ref } from "react";

import { SidebarPopover } from "@/components/sidebar-popover";
import { Button } from "@/components/ui/button";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from "@/components/ui/input-group";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import type { StatusFilter } from "@/components/workspace-sidebar";
import { groupingSchema, sortSchema } from "@/lib/model";
import type { Workspace } from "@/lib/model";

const filterOptions: { value: StatusFilter; label: string }[] = [
  { label: "All downloads", value: "all" },
  { label: "In progress", value: "active" },
  { label: "Completed", value: "finished" },
  { label: "Paused", value: "paused" },
  { label: "Failed", value: "failed" },
];

const sortNames: Record<Workspace["sort"], string> = {
  Name: "Name",
  Order: "Date added",
  Source: "Source",
  State: "Status",
};

const groupNames: Record<Workspace["grouping"], string> = {
  Folder: "Save location",
  Kind: "File type",
  None: "No grouping",
  Source: "Source",
  State: "Status",
};

export function DownloadToolbar({
  searchRef,
  query,
  setQuery,
  filter,
  setFilter,
  workspace,
  view,
  disabled,
}: {
  searchRef: Ref<HTMLInputElement>;
  query: string;
  setQuery: (query: string) => void;
  filter: StatusFilter;
  setFilter: (filter: StatusFilter) => void;
  workspace?: Workspace;
  view: (
    change: Partial<Pick<Workspace, "layout" | "sort" | "grouping">>
  ) => void;
  disabled: boolean;
}) {
  return (
    <div className="order-last flex w-full min-w-0 items-center gap-1 sm:order-none sm:ml-auto sm:w-auto sm:max-w-md sm:flex-1">
      <InputGroup className="min-w-20 flex-1">
        <InputGroupInput
          ref={searchRef}
          type="search"
          aria-label="Search downloads"
          placeholder="Search downloads…"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
        <InputGroupAddon>
          <Search />
        </InputGroupAddon>
      </InputGroup>
      <SidebarPopover
        title="Filter downloads"
        trigger={
          <Button
            variant={filter === "all" ? "ghost" : "secondary"}
            size="icon-sm"
            aria-label={`${filter === "all" ? "Filter" : filterOptions.find((option) => option.value === filter)?.label} downloads`}
            title={
              filter === "all"
                ? "Filter downloads"
                : filterOptions.find((option) => option.value === filter)?.label
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
      <SidebarPopover
        title="Sort and group"
        trigger={
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={`${workspace ? sortNames[workspace.sort] : "Sort"}: sort and group`}
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
              selected: workspace?.sort === value,
            })),
            label: "Sort by",
          },
          {
            actions: groupingSchema.options.map((value) => ({
              disabled,
              label: groupNames[value],
              onClick: () => view({ grouping: value }),
              selected: workspace?.grouping === value,
            })),
            label: "Group by",
          },
        ]}
      />
      <ToggleGroup
        value={[workspace?.layout ?? "List"]}
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
