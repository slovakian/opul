import { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Folder, Sparkles } from "lucide-react";
import { api } from "@/lib/api";
import { playSound } from "@/lib/sounds";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { MachineWorkspace } from "@/types/opul";
import { toast } from "sonner";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess: (newWorkspaceId?: string) => void;
  defaultPath?: string;
  workspaceToEdit?: MachineWorkspace | null;
}

const EMOJI_OPTIONS = [
  "📦",
  "🎬",
  "🎵",
  "📚",
  "⚡️",
  "🚀",
  "📥",
  "💎",
  "🔥",
  "💻",
];

export function CreateWorkspaceDialog({
  open,
  onOpenChange,
  onSuccess,
  defaultPath = "",
  workspaceToEdit,
}: Props) {
  const isEditing = !!workspaceToEdit;
  const [name, setName] = useState(workspaceToEdit?.name ?? "");
  const [iconKind, setIconKind] = useState<"symbol" | "emoji">(
    workspaceToEdit?.icon.kind === "Emoji" ? "emoji" : "symbol",
  );
  const [iconValue, setIconValue] = useState(
    workspaceToEdit?.icon.value ?? "sparkles",
  );
  const [folderPath, setFolderPath] = useState(
    workspaceToEdit?.folders[0]?.path ?? defaultPath,
  );
  const [defaultFormat, setDefaultFormat] = useState(
    workspaceToEdit?.default_format ?? "Best",
  );
  const [fileOrg, setFileOrg] = useState(
    workspaceToEdit?.file_organization ?? "Automatic",
  );
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handlePickFolder = async () => {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        defaultPath: folderPath || defaultPath,
        title: "Choose Save Folder for Workspace",
      });
      if (selected && typeof selected === "string") {
        setFolderPath(selected);
      }
    } catch (e) {
      console.error("Failed to open folder picker:", e);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      toast.error("Please enter a workspace name");
      return;
    }

    setIsSubmitting(true);
    try {
      if (isEditing && workspaceToEdit) {
        await api.updateWorkspace({
          workspaceId: workspaceToEdit.id,
          name: name.trim(),
          iconKind,
          iconValue,
          fileOrganization: fileOrg,
        });
        playSound("tap");
        toast.success(`Updated workspace "${name}"`);
        onSuccess(workspaceToEdit.id);
      } else {
        await api.createWorkspace({
          name: name.trim(),
          iconKind,
          iconValue,
          folderPath: folderPath || defaultPath,
          defaultFormat,
          fileOrganization: fileOrg,
        });
        playSound("tap");
        toast.success(`Created workspace "${name}"`);
        onSuccess();
      }
      onOpenChange(false);
    } catch (err) {
      console.error(err);
      toast.error(`Error: ${String(err)}`);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[480px]">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>
              {isEditing ? "Edit Workspace" : "Create Workspace"}
            </DialogTitle>
            <DialogDescription>
              {isEditing
                ? "Update workspace settings and icon."
                : "Workspaces help you organize different types of downloads and saved folders."}
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <Label htmlFor="ws-name">Name</Label>
              <Input
                id="ws-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g. Media, Software, Torrents..."
                autoFocus
              />
            </div>

            <div className="grid gap-2">
              <Label>Icon</Label>
              <div className="flex flex-wrap gap-2 pt-1">
                <Button
                  type="button"
                  variant={
                    iconKind === "symbol" && iconValue === "sparkles"
                      ? "default"
                      : "outline"
                  }
                  size="sm"
                  onClick={() => {
                    setIconKind("symbol");
                    setIconValue("sparkles");
                  }}
                  className="gap-1.5"
                >
                  <Sparkles className="size-4" />
                  Sparkles
                </Button>
                <Button
                  type="button"
                  variant={
                    iconKind === "symbol" && iconValue === "folder"
                      ? "default"
                      : "outline"
                  }
                  size="sm"
                  onClick={() => {
                    setIconKind("symbol");
                    setIconValue("folder");
                  }}
                  className="gap-1.5"
                >
                  <Folder className="size-4" />
                  Folder
                </Button>
                {EMOJI_OPTIONS.map((emoji) => (
                  <Button
                    key={emoji}
                    type="button"
                    variant={
                      iconKind === "emoji" && iconValue === emoji
                        ? "default"
                        : "outline"
                    }
                    size="sm"
                    onClick={() => {
                      setIconKind("emoji");
                      setIconValue(emoji);
                    }}
                    className="text-base px-2.5"
                  >
                    {emoji}
                  </Button>
                ))}
              </div>
            </div>

            {!isEditing && (
              <div className="grid gap-2">
                <Label htmlFor="ws-folder">Save Location</Label>
                <div className="flex gap-2">
                  <Input
                    id="ws-folder"
                    value={folderPath}
                    onChange={(e) => setFolderPath(e.target.value)}
                    placeholder="Path to downloads folder"
                    className="font-mono text-xs"
                  />
                  <Button
                    type="button"
                    variant="outline"
                    onClick={handlePickFolder}
                  >
                    Browse...
                  </Button>
                </div>
              </div>
            )}

            <div className="grid grid-cols-2 gap-4">
              <div className="grid gap-2">
                <Label>Default Video Format</Label>
                <Select
                  value={defaultFormat}
                  onValueChange={(val) => {
                    if (val) setDefaultFormat(val as typeof defaultFormat);
                  }}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="Best">Best Quality</SelectItem>
                    <SelectItem value="BestMp4">Best MP4</SelectItem>
                    <SelectItem value="P1080">1080p Full HD</SelectItem>
                    <SelectItem value="P720">720p HD</SelectItem>
                    <SelectItem value="AudioMp3">Audio MP3</SelectItem>
                    <SelectItem value="AudioM4a">Audio M4A</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="grid gap-2">
                <Label>File Organization</Label>
                <Select
                  value={fileOrg}
                  onValueChange={(val) => {
                    if (val) setFileOrg(val as typeof fileOrg);
                  }}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="Automatic">Automatic</SelectItem>
                    <SelectItem value="AlwaysCreateFolder">
                      Always Folder
                    </SelectItem>
                    <SelectItem value="KeepInSaveLocation">
                      Flat / In Place
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
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
              {isEditing ? "Save Changes" : "Create Workspace"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
