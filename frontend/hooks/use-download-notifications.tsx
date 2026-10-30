import { useCallback, useEffect, useRef, useState } from "react";

import { toast } from "@/components/ui/toast";
import { WorkspaceIcon } from "@/components/workspace-icon";
import { useLibrary } from "@/lib/library";

export const useDownloadNotifications = (onInspect: (id: string) => void) => {
  const { state } = useLibrary();
  const pending = useRef(new Set<string>());
  const [submission, setSubmission] = useState(0);

  useEffect(() => {
    if (
      submission === 0 ||
      state.kind !== "ready" ||
      pending.current.size === 0
    ) {
      return;
    }

    for (const id of pending.current) {
      const workspace = state.snapshot.workspaces.find((item) =>
        item.downloads.some((download) => download.id === id)
      );

      const download = workspace?.downloads.find((item) => item.id === id);

      if (!workspace || !download) {
        continue;
      }

      const folder = workspace.folders.find(
        (item) => item.id === download.folder_id
      );

      const toastId = `download-added-${id}`;

      toast.add({
        actionProps: {
          children: "View",
          onClick: () => {
            onInspect(id);
            toast.close(toastId);
          },
        },
        description: `Added to ${workspace.name} › ${folder?.name ?? "Downloads"}`,
        id: toastId,
        title: (
          <span className="flex min-w-0 items-center gap-2">
            <WorkspaceIcon workspace={workspace} />
            <span className="truncate">{download.name}</span>
          </span>
        ),
      });
      pending.current.delete(id);
    }
  }, [onInspect, state, submission]);

  return useCallback((ids: string[]) => {
    for (const id of ids) {
      pending.current.add(id);
    }

    setSubmission((current) => current + 1);
  }, []);
};
