import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  showTrashOption?: boolean;
  onConfirm: (moveFilesToTrash: boolean) => void;
}

export function ConfirmDeleteDialog({
  open,
  onOpenChange,
  title,
  description,
  showTrashOption = true,
  onConfirm,
}: Props) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[440px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-destructive">
            <Trash2 className="size-5" />
            {title}
          </DialogTitle>
          <DialogDescription className="text-sm pt-1">
            {description}
          </DialogDescription>
        </DialogHeader>

        <DialogFooter className="flex-col sm:flex-row gap-2 pt-2">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            variant="secondary"
            onClick={() => {
              onConfirm(false);
              onOpenChange(false);
            }}
          >
            Remove from List
          </Button>
          {showTrashOption && (
            <Button
              variant="destructive"
              onClick={() => {
                onConfirm(true);
                onOpenChange(false);
              }}
            >
              Move to Trash
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
