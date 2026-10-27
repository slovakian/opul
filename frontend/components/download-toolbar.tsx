import type { Ref } from "react";
import { ArrowUpDown, List, ListFilter, Search, Table2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from "@/components/ui/input-group";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { SidebarPopover } from "@/components/sidebar-popover";
import { groupingSchema, sortSchema, type Workspace } from "@/lib/model";
import type { StatusFilter } from "@/components/workspace-sidebar";
const filterOptions: { value: StatusFilter; label: string }[] = [
  { value: "all", label: "All downloads" },
  { value: "active", label: "In progress" },
  { value: "finished", label: "Completed" },
  { value: "paused", label: "Paused" },
  { value: "failed", label: "Failed" },
];
const sortNames: Record<Workspace["sort"], string> = {
  Order: "Date added",
  Name: "Name",
  State: "Status",
  Source: "Source",
};
const groupNames: Record<Workspace["grouping"], string> = {
  None: "No grouping",
  State: "Status",
  Folder: "Save location",
  Kind: "File type",
  Source: "Source",
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
    change: Partial<Pick<Workspace, "layout" | "sort" | "grouping">>,
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
            label: "Status",
            actions: filterOptions.map((option) => ({
              label: option.label,
              selected: filter === option.value,
              onClick: () => setFilter(option.value),
            })),
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
            label: "Sort by",
            actions: sortSchema.options.map((value) => ({
              label: sortNames[value],
              selected: workspace?.sort === value,
              disabled: disabled,
              onClick: () => view({ sort: value }),
            })),
          },
          {
            label: "Group by",
            actions: groupingSchema.options.map((value) => ({
              label: groupNames[value],
              selected: workspace?.grouping === value,
              disabled: disabled,
              onClick: () => view({ grouping: value }),
            })),
          },
        ]}
      />
      <ToggleGroup
        value={[workspace?.layout ?? "List"]}
        onValueChange={(values) => {
          const layout = values[0];
          if (layout === "List" || layout === "Table") view({ layout });
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
