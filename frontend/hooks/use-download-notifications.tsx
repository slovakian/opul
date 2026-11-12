import { useCallback, useEffect, useRef, useState } from "react";

import { toast } from "@/components/ui/toast";
import { WorkspaceIcon } from "@/components/workspace-icon";
import { useLibraryStore } from "@/lib/library";
import { truncateFilename } from "@/lib/truncate";

export const useDownloadNotifications = (onInspect: (id: string) => void) => {
  const store = useLibraryStore();
  const pending = useRef(new Set<string>());
  const [submission, setSubmission] = useState(0);

  useEffect(() => {
    if (submission === 0) {
      return;
    }

    // Announce each new download once the library shows it. Watching the store
    // directly keeps this from rendering the app on every library change.
    const announce = () => {
      const state = store.getState();

      if (state.kind !== "ready" || pending.current.size === 0) {
        return;
      }

      const { library } = state;

      for (const id of pending.current) {
        const download = library.downloadById.get(id);

        const workspace = library.workspaces.find(
          (item) => item.id === download?.workspace_id
        );

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
          description: `Added to ${truncateFilename(workspace.name, 32)} › ${truncateFilename(folder?.name ?? "Downloads", 32)}`,
          id: toastId,
          title: (
            <span className="flex min-w-0 items-center gap-2">
              <WorkspaceIcon workspace={workspace} />
              <span className="truncate" title={download.name}>
                {truncateFilename(download.name)}
              </span>
            </span>
          ),
        });
        pending.current.delete(id);
      }
    };

    announce();

    return store.subscribe(announce);
  }, [onInspect, store, submission]);

  return useCallback((ids: string[]) => {
    for (const id of ids) {
      pending.current.add(id);
    }

    setSubmission((current) => current + 1);
  }, []);
};
