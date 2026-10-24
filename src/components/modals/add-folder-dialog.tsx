import { useState } from 'react'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { FolderPlus } from 'lucide-react'
import { api } from '@/lib/api'
import { playSound } from '@/lib/sounds'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import type { MachineWorkspace } from '@/types/opul'
import { toast } from 'sonner'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  workspace: MachineWorkspace
  onSuccess: () => void
}

export function AddFolderDialog({ open, onOpenChange, workspace, onSuccess }: Props) {
  const [name, setName] = useState('')
  const [path, setPath] = useState('')
  const [makeDefault, setMakeDefault] = useState(false)
  const [isSubmitting, setIsSubmitting] = useState(false)

  const handlePickFolder = async () => {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: 'Select Folder',
      })
      if (selected && typeof selected === 'string') {
        setPath(selected)
        if (!name) {
          const parts = selected.split(/[\/\\]/)
          setName(parts[parts.length - 1] || 'Folder')
        }
      }
    } catch (e) {
      console.error(e)
    }
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!path.trim()) {
      toast.error('Please choose a folder path')
      return
    }

    setIsSubmitting(true)
    try {
      await api.addFolder({
        workspaceId: workspace.id,
        name: name.trim() || 'Folder',
        path: path.trim(),
        makeDefault,
      })
      playSound('tap')
      toast.success(`Added folder "${name || 'Folder'}"`)
      onSuccess()
      onOpenChange(false)
    } catch (err) {
      console.error(err)
      toast.error(`Failed to add folder: ${String(err)}`)
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[450px]">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <FolderPlus className="size-5 text-primary" />
              Add Saved Folder
            </DialogTitle>
            <DialogDescription>
              Add a folder in &quot;{workspace.name}&quot; to direct downloads into specific places.
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <Label htmlFor="folder-path">Path</Label>
              <div className="flex gap-2">
                <Input
                  id="folder-path"
                  value={path}
                  onChange={(e) => setPath(e.target.value)}
                  placeholder="Select a folder..."
                  className="font-mono text-xs"
                />
                <Button type="button" variant="outline" onClick={handlePickFolder}>
                  Browse...
                </Button>
              </div>
            </div>

            <div className="grid gap-2">
              <Label htmlFor="folder-name">Display Name</Label>
              <Input
                id="folder-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g. Movies, Music, Docs..."
              />
            </div>

            <div className="flex items-center space-x-2 pt-1">
              <Checkbox
                id="make-default"
                checked={makeDefault}
                onCheckedChange={(c) => setMakeDefault(!!c)}
              />
              <label
                htmlFor="make-default"
                className="text-xs font-medium cursor-pointer"
              >
                Set as default folder for this workspace
              </label>
            </div>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={isSubmitting}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              Add Folder
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
