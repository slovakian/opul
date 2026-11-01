import { useEffect, useRef, useState } from "react";

import { toast } from "@/components/ui/toast";
import { bridge, errorMessage } from "@/lib/bridge";
import { useLibrary } from "@/lib/library";

export type AddSavedFolder = (
  workspaceId: string,
  onConfirmed: (folderId: string) => void,
  errorTitle: string
) => Promise<void>;

interface PendingAddition {
  workspaceId: string;
  folderId: string;
  accepted: boolean;
  onConfirmed: (folderId: string) => void;
}

export const useSavedFolderAddition = (
  onExpansionChange: (workspaceId: string, expanded: boolean) => void
): AddSavedFolder => {
  const { run, state } = useLibrary();
  const [pending, setPending] = useState<PendingAddition[]>([]);
  const previousFolderCounts = useRef(new Map<string, number>());
  const awaitingExpansion = useRef(new Set<string>());

  useEffect(() => {
    if (state.kind !== "ready") {
      return;
    }

    const counts = new Map<string, number>();

    for (const workspace of state.snapshot.workspaces) {
      counts.set(workspace.id, workspace.folders.length);

      if (
        previousFolderCounts.current.get(workspace.id) === 1 &&
        workspace.folders.length > 1
      ) {
        const addition = pending.find(
          (item) =>
            item.workspaceId === workspace.id &&
            workspace.folders.some((folder) => folder.id === item.folderId)
        );

        if (addition) {
          // A snapshot can arrive before this command's accepted receipt.
          awaitingExpansion.current.add(addition.folderId);
        } else {
          onExpansionChange(workspace.id, true);
        }
      }
    }

    previousFolderCounts.current = counts;

    const confirmed = pending.filter(
      (addition) =>
        addition.accepted &&
        state.snapshot.workspaces.some(
          (workspace) =>
            workspace.id === addition.workspaceId &&
            workspace.folders.some((folder) => folder.id === addition.folderId)
        )
    );

    if (!confirmed.length) {
      return;
    }

    // Authoritative desktop snapshots acknowledge accepted command intents.
    // oxlint-disable-next-line react/set-state-in-effect
    setPending((previous) =>
      previous.filter((addition) => !confirmed.includes(addition))
    );

    for (const addition of confirmed) {
      if (awaitingExpansion.current.delete(addition.folderId)) {
        onExpansionChange(addition.workspaceId, true);
      }

      addition.onConfirmed(addition.folderId);
    }
  }, [onExpansionChange, pending, state]);

  return async (workspaceId, onConfirmed, errorTitle) => {
    try {
      const path = await bridge.chooseFolder();

      if (!path) {
        return;
      }

      const folderId = crypto.randomUUID();
      setPending((previous) => [
        ...previous,
        { accepted: false, folderId, onConfirmed, workspaceId },
      ]);

      const receipt = await run(
        {
          kind: "add_folder",
          request: {
            expected_revision: null,
            folder_id: folderId,
            name: path.split("/").findLast(Boolean) || "Folder",
            path,
            request_id: crypto.randomUUID(),
            workspace_id: workspaceId,
          },
        },
        () => {
          setPending((previous) =>
            previous.map((addition) =>
              addition.folderId === folderId
                ? { ...addition, accepted: true }
                : addition
            )
          );
        }
      );

      if (receipt === null) {
        awaitingExpansion.current.delete(folderId);
        setPending((previous) =>
          previous.filter((addition) => addition.folderId !== folderId)
        );
      }
    } catch (error) {
      toast.add({
        description: errorMessage(error),
        title: errorTitle,
        type: "error",
      });
    }
  };
};
