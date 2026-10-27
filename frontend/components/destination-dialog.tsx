import { useState } from "react";

import { Choice } from "@/components/sidebar-popover";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { WorkspaceIcon } from "@/components/workspace-icon";
import { workspaceDestination } from "@/lib/destination";
import type { Destination } from "@/lib/destination";
import type { Workspace } from "@/lib/model";

export const DestinationDialog = ({
  workspaces,
  recommended,
  canSaveDefault,
  disabled,
  onConfirm,
  onClose,
  inputRef,
}: {
  workspaces: Workspace[];
  recommended: Workspace;
  canSaveDefault: boolean;
  disabled: boolean;
  onConfirm: (destination: Destination, automatic: boolean) => Promise<void>;
  onClose: () => void;
  inputRef: React.RefObject<HTMLInputElement | null>;
}) => {
  const [workspaceId, setWorkspaceId] = useState(recommended.id);
  const [folderId, setFolderId] = useState<string>();
  const [automatic, setAutomatic] = useState(false);

  const selected = workspaceDestination(
    workspaces.find((item) => item.id === workspaceId),
    folderId
  );

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !disabled) {
          onClose();
        }
      }}
    >
      <DialogContent finalFocus={inputRef}>
        <form
          className="flex flex-col gap-6"
          onSubmit={async (event) => {
            event.preventDefault();

            if (selected && !disabled) {
              await onConfirm(selected, automatic);
            }
          }}
        >
          <DialogHeader>
            <DialogTitle>Choose download destination</DialogTitle>
            <DialogDescription>
              Choose where to save this download.
            </DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <Field data-disabled={disabled}>
              <FieldLabel>Workspace</FieldLabel>
              <Choice
                label="Download workspace"
                value={selected?.workspace.id ?? ""}
                disabled={disabled}
                options={workspaces.map((workspace) => ({
                  content: (
                    <>
                      <WorkspaceIcon workspace={workspace} />
                      <span className="truncate">{workspace.name}</span>
                    </>
                  ),
                  label: workspace.name,
                  value: workspace.id,
                }))}
                onChange={(id) => {
                  setWorkspaceId(id);
                  setFolderId(undefined);
                }}
              />
            </Field>
            <Field data-disabled={disabled}>
              <FieldLabel>Save to</FieldLabel>
              <Choice
                label="Save location"
                value={selected?.folder.id ?? ""}
                disabled={disabled}
                options={(selected?.workspace.folders ?? []).map((folder) => ({
                  label: folder.name,
                  value: folder.id,
                }))}
                onChange={setFolderId}
              />
            </Field>
            <Field
              orientation="horizontal"
              data-disabled={disabled || !canSaveDefault}
            >
              <Checkbox
                id="automatic-destination"
                checked={automatic}
                onCheckedChange={setAutomatic}
                disabled={disabled || !canSaveDefault}
              />
              <FieldLabel htmlFor="automatic-destination">
                Use this workspace automatically next time
              </FieldLabel>
            </Field>
          </FieldGroup>
          <DialogFooter>
            <Button
              type="button"
              variant="ghost"
              onClick={onClose}
              disabled={disabled}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={disabled || !selected}>
              {disabled ? "Adding…" : "Download"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};
