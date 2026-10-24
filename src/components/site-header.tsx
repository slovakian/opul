import * as React from "react";
import {
  ArrowDownUp,
  Download,
  FileCode,
  Folder,
  Layers,
  LayoutList,
  Search,
  Sparkles,
  Table as TableIcon,
  X,
} from "lucide-react";
import { SidebarTrigger } from "@/components/ui/sidebar";
import { Separator } from "@/components/ui/separator";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type {
  MachineFolder,
  MachineGrouping,
  MachineLayout,
  MachineSort,
  MachineWorkspace,
} from "@/types/opul";

interface SiteHeaderProps {
  workspace?: MachineWorkspace;
  selectedFolder?: MachineFolder;
  searchQuery: string;
  onSearchChange: (query: string) => void;
  onSubmitUrl: (url: string, folderId: string) => void;
  onPickTorrentFile: (folderId: string) => void;
  layout: MachineLayout;
  onLayoutChange: (layout: MachineLayout) => void;
  sort: MachineSort;
  onSortChange: (sort: MachineSort) => void;
  grouping: MachineGrouping;
  onGroupingChange: (grouping: MachineGrouping) => void;
}

export function SiteHeader({
  workspace,
  selectedFolder,
  searchQuery: _searchQuery,
  onSearchChange,
  onSubmitUrl,
  onPickTorrentFile,
  layout,
  onLayoutChange,
  sort,
  onSortChange,
  grouping,
  onGroupingChange,
}: SiteHeaderProps) {
  const [inputVal, setInputVal] = React.useState("");
  const defaultFolderId =
    selectedFolder?.id ??
    workspace?.folders.find((f) => f.is_default)?.id ??
    workspace?.folders[0]?.id ??
    "";
  const [selectedFolderOverride, setSelectedFolderOverride] = React.useState<string | null>(null);
  const targetFolderId = selectedFolderOverride ?? defaultFolderId;

  const isUrl =
    inputVal.trim().startsWith("http://") ||
    inputVal.trim().startsWith("https://") ||
    inputVal.trim().startsWith("magnet:") ||
    inputVal.trim().endsWith(".torrent");

  const handleAction = () => {
    const trimmed = inputVal.trim();
    if (!trimmed) return;

    if (isUrl) {
      onSubmitUrl(trimmed, targetFolderId);
      setInputVal("");
      onSearchChange("");
    } else {
      onSearchChange(trimmed);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      handleAction();
    } else if (e.key === "Escape") {
      setInputVal("");
      onSearchChange("");
    }
  };

  return (
    <header className="sticky top-0 z-10 flex flex-col border-b border-border/50 bg-background/90 backdrop-blur-md select-none">
      {/* Top Bar with native macOS drag region */}
      <div
        data-tauri-drag-region
        className="flex items-center justify-between gap-3 px-4 pt-2.5 pb-2 min-h-12"
      >
        {/* Left: Window drag region + Sidebar trigger + Breadcrumb */}
        <div
          className="flex items-center gap-2.5 min-w-0"
          data-tauri-drag-region
        >
          <SidebarTrigger className="size-7" />
          <Separator orientation="vertical" className="h-4 opacity-50" />
          <div className="flex items-center gap-1.5 min-w-0 text-[13px] font-semibold text-foreground">
            {workspace && (
              <span className="flex items-center gap-1.5 truncate">
                {workspace.icon.kind === "Emoji" ? (
                  <span>{workspace.icon.value}</span>
                ) : workspace.icon.value === "folder" ? (
                  <Folder className="size-3.5 text-muted-foreground" />
                ) : (
                  <Sparkles className="size-3.5 text-primary" />
                )}
                <span className="truncate">{workspace.name}</span>
              </span>
            )}
            {selectedFolder && (
              <>
                <span className="text-muted-foreground/60 font-normal">/</span>
                <span className="truncate text-xs font-normal text-muted-foreground">
                  {selectedFolder.name}
                </span>
              </>
            )}
          </div>
        </div>

        {/* Right view & sort controls */}
        <div className="flex items-center gap-1.5">
          {/* Layout Toggle (Cards vs Table) */}
          <div className="flex items-center rounded-lg border border-border/60 p-0.5 bg-muted/40 shadow-2xs">
            <Tooltip>
              <TooltipTrigger
                render={
                  <Button
                    variant={layout === "List" ? "secondary" : "ghost"}
                    size="icon"
                    className="size-6.5 rounded-md"
                    onClick={() => onLayoutChange("List")}
                  >
                    <LayoutList className="size-3.5" />
                  </Button>
                }
              />
              <TooltipContent side="bottom">Card View</TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger
                render={
                  <Button
                    variant={layout === "Table" ? "secondary" : "ghost"}
                    size="icon"
                    className="size-6.5 rounded-md"
                    onClick={() => onLayoutChange("Table")}
                  >
                    <TableIcon className="size-3.5" />
                  </Button>
                }
              />
              <TooltipContent side="bottom">Table View</TooltipContent>
            </Tooltip>
          </div>

          {/* Grouping Dropdown */}
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button
                  variant="outline"
                  size="sm"
                  className="h-7.5 gap-1.5 text-xs px-2.5 rounded-lg border-border/60"
                >
                  <Layers className="size-3.5 text-muted-foreground" />
                  <span className="font-normal">Group</span>
                </Button>
              }
            />
            <DropdownMenuContent align="end" className="w-36 text-xs">
              <DropdownMenuLabel className="text-[11px] uppercase tracking-wider text-muted-foreground">
                Group By
              </DropdownMenuLabel>
              <DropdownMenuSeparator />
              <DropdownMenuRadioGroup
                value={grouping}
                onValueChange={(val) =>
                  onGroupingChange(val as MachineGrouping)
                }
              >
                <DropdownMenuRadioItem value="None">None</DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="State">
                  Status
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="Folder">
                  Folder
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="Source">
                  Source
                </DropdownMenuRadioItem>
              </DropdownMenuRadioGroup>
            </DropdownMenuContent>
          </DropdownMenu>

          {/* Sort Dropdown */}
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button
                  variant="outline"
                  size="sm"
                  className="h-7.5 gap-1.5 text-xs px-2.5 rounded-lg border-border/60"
                >
                  <ArrowDownUp className="size-3.5 text-muted-foreground" />
                  <span className="font-normal">Sort</span>
                </Button>
              }
            />
            <DropdownMenuContent align="end" className="w-36 text-xs">
              <DropdownMenuLabel className="text-[11px] uppercase tracking-wider text-muted-foreground">
                Sort By
              </DropdownMenuLabel>
              <DropdownMenuSeparator />
              <DropdownMenuRadioGroup
                value={sort}
                onValueChange={(val) => onSortChange(val as MachineSort)}
              >
                <DropdownMenuRadioItem value="Order">
                  Added Date
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="Name">Name</DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="State">
                  Status
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="Source">
                  Source
                </DropdownMenuRadioItem>
              </DropdownMenuRadioGroup>
            </DropdownMenuContent>
          </DropdownMenu>

          {/* Open Torrent File */}
          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  variant="outline"
                  size="sm"
                  className="h-7.5 gap-1.5 text-xs px-2.5 rounded-lg border-border/60"
                  onClick={() => onPickTorrentFile(targetFolderId)}
                >
                  <FileCode className="size-3.5 text-muted-foreground" />
                  <span>+ Torrent</span>
                </Button>
              }
            />
            <TooltipContent side="bottom">Open .torrent file</TooltipContent>
          </Tooltip>
        </div>
      </div>

      {/* Unified URL Input & Search Toolbar */}
      <div className="flex items-center gap-2 px-4 pb-2.5 pt-0.5">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-2.5 size-3.5 text-muted-foreground/70" />
          <Input
            value={inputVal}
            onChange={(e) => {
              const val = e.target.value;
              setInputVal(val);
              if (
                !val.trim().startsWith("http") &&
                !val.trim().startsWith("magnet:")
              ) {
                onSearchChange(val.trim());
              }
            }}
            onKeyDown={handleKeyDown}
            placeholder="Paste URL, magnet link, or type to search..."
            className="h-8.5 pl-8.5 pr-24 text-xs rounded-lg border-border/60 bg-muted/20 focus-visible:bg-background transition-colors"
          />

          <div className="absolute right-1.5 top-1 flex items-center gap-1">
            {inputVal && !isUrl && (
              <Button
                variant="ghost"
                size="icon"
                className="size-6 text-muted-foreground hover:text-foreground"
                onClick={() => {
                  setInputVal("");
                  onSearchChange("");
                }}
              >
                <X className="size-3" />
              </Button>
            )}

            {isUrl && (
              <Button
                size="sm"
                className="h-6.5 px-2.5 text-xs gap-1 rounded-md shadow-2xs font-medium"
                onClick={handleAction}
              >
                <Download className="size-3" />
                <span>Add</span>
                <span className="text-[10px] opacity-70 ml-0.5">↵</span>
              </Button>
            )}
          </div>
        </div>

        {/* Destination Folder Selector (if multiple folders exist) */}
        {workspace && workspace.folders.length > 1 && (
          <div className="w-40 shrink-0">
            <Select
              value={targetFolderId}
              onValueChange={(val) => {
                if (val) setSelectedFolderOverride(val);
              }}
            >
              <SelectTrigger className="h-8.5 text-xs rounded-lg border-border/60 bg-muted/20">
                <SelectValue placeholder="Folder" />
              </SelectTrigger>
              <SelectContent className="text-xs">
                {workspace.folders.map((f) => (
                  <SelectItem key={f.id} value={f.id} className="text-xs">
                    {f.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        )}
      </div>
    </header>
  );
}
