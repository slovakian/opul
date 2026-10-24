import * as React from 'react'
import {
  ArrowDownUp,
  Download,
  FileCode,
  Folder,
  Layers,
  LayoutGrid,
  List as ListIcon,
  Search,
  Sparkles,
} from 'lucide-react'
import { SidebarTrigger } from '@/components/ui/sidebar'
import { Separator } from '@/components/ui/separator'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import type {
  MachineFolder,
  MachineGrouping,
  MachineLayout,
  MachineSort,
  MachineWorkspace,
} from '@/types/opul'

interface SiteHeaderProps {
  workspace?: MachineWorkspace
  selectedFolder?: MachineFolder
  searchQuery: string
  onSearchChange: (query: string) => void
  onSubmitUrl: (url: string, folderId: string) => void
  onPickTorrentFile: (folderId: string) => void
  layout: MachineLayout
  onLayoutChange: (layout: MachineLayout) => void
  sort: MachineSort
  onSortChange: (sort: MachineSort) => void
  grouping: MachineGrouping
  onGroupingChange: (grouping: MachineGrouping) => void
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
  const [inputVal, setInputVal] = React.useState('')
  const defaultFolderId =
    selectedFolder?.id ??
    workspace?.folders.find((f) => f.is_default)?.id ??
    workspace?.folders[0]?.id ??
    ''
  const [targetFolderId, setTargetFolderId] = React.useState(defaultFolderId)

  React.useEffect(() => {
    if (defaultFolderId) setTargetFolderId(defaultFolderId)
  }, [defaultFolderId])

  const handleAction = () => {
    const trimmed = inputVal.trim()
    if (!trimmed) return
    const isUrl =
      trimmed.startsWith('http://') ||
      trimmed.startsWith('https://') ||
      trimmed.startsWith('magnet:') ||
      trimmed.endsWith('.torrent')

    if (isUrl) {
      onSubmitUrl(trimmed, targetFolderId)
      setInputVal('')
      onSearchChange('')
    } else {
      // Use as search filter
      onSearchChange(trimmed)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      handleAction()
    }
  }

  const isUrlInput =
    inputVal.trim().startsWith('http://') ||
    inputVal.trim().startsWith('https://') ||
    inputVal.trim().startsWith('magnet:')

  return (
    <header className="sticky top-0 z-10 flex flex-col gap-2 border-b bg-background/95 backdrop-blur-md px-4 py-2.5">
      <div className="flex items-center justify-between gap-3">
        {/* Left: Sidebar trigger + Breadcrumb */}
        <div className="flex items-center gap-2 min-w-0">
          <SidebarTrigger />
          <Separator orientation="vertical" className="h-4" />
          <div className="flex items-center gap-1.5 min-w-0 text-sm font-semibold">
            {workspace && (
              <span className="flex items-center gap-1.5 truncate">
                {workspace.icon.kind === 'Emoji' ? (
                  workspace.icon.value
                ) : workspace.icon.value === 'folder' ? (
                  <Folder className="size-4 text-muted-foreground" />
                ) : (
                  <Sparkles className="size-4 text-primary" />
                )}
                {workspace.name}
              </span>
            )}
            {selectedFolder && (
              <>
                <span className="text-muted-foreground">/</span>
                <span className="truncate text-xs font-normal text-muted-foreground">
                  {selectedFolder.name}
                </span>
              </>
            )}
          </div>
        </div>

        {/* Right view & sort controls */}
        <div className="flex items-center gap-1.5">
          {/* Layout Toggle */}
          <div className="flex items-center border rounded-md p-0.5 bg-muted/30">
            <Button
              variant={layout === 'List' ? 'secondary' : 'ghost'}
              size="icon"
              className="size-7"
              onClick={() => onLayoutChange('List')}
              title="List View"
            >
              <ListIcon className="size-3.5" />
            </Button>
            <Button
              variant={layout === 'Table' ? 'secondary' : 'ghost'}
              size="icon"
              className="size-7"
              onClick={() => onLayoutChange('Table')}
              title="Table View"
            >
              <LayoutGrid className="size-3.5" />
            </Button>
          </div>

          {/* Grouping Dropdown */}
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button variant="outline" size="sm" className="h-8 gap-1.5 text-xs px-2.5">
                  <Layers className="size-3.5 text-muted-foreground" />
                  <span>Group</span>
                </Button>
              }
            />
            <DropdownMenuContent align="end" className="w-36">
              <DropdownMenuLabel className="text-xs">Group By</DropdownMenuLabel>
              <DropdownMenuSeparator />
              <DropdownMenuRadioGroup
                value={grouping}
                onValueChange={(val) => onGroupingChange(val as MachineGrouping)}
              >
                <DropdownMenuRadioItem value="None">None</DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="State">Status</DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="Folder">Folder</DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="Source">Source</DropdownMenuRadioItem>
              </DropdownMenuRadioGroup>
            </DropdownMenuContent>
          </DropdownMenu>

          {/* Sort Dropdown */}
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button variant="outline" size="sm" className="h-8 gap-1.5 text-xs px-2.5">
                  <ArrowDownUp className="size-3.5 text-muted-foreground" />
                  <span>Sort</span>
                </Button>
              }
            />
            <DropdownMenuContent align="end" className="w-36">
              <DropdownMenuLabel className="text-xs">Sort By</DropdownMenuLabel>
              <DropdownMenuSeparator />
              <DropdownMenuRadioGroup
                value={sort}
                onValueChange={(val) => onSortChange(val as MachineSort)}
              >
                <DropdownMenuRadioItem value="Order">Added Date</DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="Name">Name</DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="State">Status</DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="Source">Source</DropdownMenuRadioItem>
              </DropdownMenuRadioGroup>
            </DropdownMenuContent>
          </DropdownMenu>

          {/* Open Torrent File */}
          <Button
            variant="outline"
            size="sm"
            className="h-8 gap-1.5 text-xs px-2.5"
            onClick={() => onPickTorrentFile(targetFolderId)}
            title="Open .torrent file"
          >
            <FileCode className="size-3.5" />
            <span>+ Torrent</span>
          </Button>
        </div>
      </div>

      {/* Unified Search & URL Input Bar */}
      <div className="flex items-center gap-2">
        <div className="relative flex-1">
          <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
          <Input
            value={inputVal}
            onChange={(e) => {
              setInputVal(e.target.value)
              if (!e.target.value.startsWith('http') && !e.target.value.startsWith('magnet:')) {
                onSearchChange(e.target.value)
              }
            }}
            onKeyDown={handleKeyDown}
            placeholder="Paste URL, magnet link, video page, or type to search..."
            className="h-9 pl-9 pr-24 font-mono text-xs"
          />
          {inputVal.trim() && (
            <div className="absolute right-1.5 top-1">
              <Button
                size="sm"
                className="h-7 px-2.5 text-xs gap-1.5"
                onClick={handleAction}
              >
                {isUrlInput ? (
                  <>
                    <Download className="size-3" />
                    Download
                  </>
                ) : (
                  'Filter'
                )}
              </Button>
            </div>
          )}
        </div>

        {/* Folder Destination Selector */}
        {workspace && workspace.folders.length > 0 && (
          <div className="w-44">
            <Select
              value={targetFolderId}
              onValueChange={(val) => {
                if (val) setTargetFolderId(val)
              }}
            >
              <SelectTrigger className="h-9 text-xs">
                <SelectValue placeholder="Save Folder" />
              </SelectTrigger>
              <SelectContent>
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
  )
}
