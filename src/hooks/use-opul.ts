import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "@/lib/api";
import { playSound } from "@/lib/sounds";
import type {
  MachineDownload,
  MachineFolder,
  MachineSnapshot,
  MachineWorkspace,
  SystemPaths,
} from "@/types/opul";
import { toast } from "sonner";

export function useOpul() {
  const [snapshot, setSnapshot] = useState<MachineSnapshot | null>(null);
  const [paths, setPaths] = useState<SystemPaths | null>(null);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string | null>(
    null,
  );
  const [selectedFolderId, setSelectedFolderId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [isLoading, setIsLoading] = useState(true);

  const prevDownloadsRef = useRef<Record<string, string>>({});
  const snapshotRef = useRef<MachineSnapshot | null>(null);

  useEffect(() => {
    snapshotRef.current = snapshot;
  }, [snapshot]);

  // Fetch initial paths and snapshot
  useEffect(() => {
    let mounted = true;

    async function init() {
      try {
        const [sysPaths, initialSnap] = await Promise.all([
          api.getSystemPaths(),
          api.getSnapshot(),
        ]);
        if (!mounted) return;
        setPaths(sysPaths);
        setSnapshot(initialSnap);
        snapshotRef.current = initialSnap;
        if (initialSnap.workspaces.length > 0) {
          setSelectedWorkspaceId(initialSnap.workspaces[0].id);
        }
      } catch (err) {
        console.error("Failed to initialize Opul state:", err);
        toast.error("Failed to connect to Opul backend");
      } finally {
        if (mounted) setIsLoading(false);
      }
    }

    init();
    return () => {
      mounted = false;
    };
  }, []);

  // Audio cues on state transitions
  useEffect(() => {
    if (!snapshot) return;
    const prev = prevDownloadsRef.current;
    const curr: Record<string, string> = {};

    for (const ws of snapshot.workspaces) {
      for (const dl of ws.downloads) {
        curr[dl.id] = dl.state;
        const prevState = prev[dl.id];
        if (prevState && prevState !== dl.state) {
          if (dl.state === "Finished" || dl.state === "Seeding") {
            playSound("success");
            toast.success(`Completed: ${dl.name}`);
          } else if (dl.state === "Failed") {
            playSound("error");
            toast.error(`Failed: ${dl.name}`);
          }
        }
      }
    }
    prevDownloadsRef.current = curr;
  }, [snapshot]);

  // Reactive wait-for-snapshot loop
  useEffect(() => {
    let unmounted = false;

    async function poll() {
      while (!unmounted) {
        try {
          const current = snapshotRef.current;
          const currentSeq = current?.sequence ?? 0;
          const currentBoot = current?.boot_id ?? null;
          const nextSnap = await api.waitForSnapshot(
            currentBoot,
            currentSeq,
            2000,
          );
          if (!unmounted && nextSnap) {
            setSnapshot(nextSnap);
            snapshotRef.current = nextSnap;
          }
        } catch {
          if (!unmounted) {
            await new Promise((r) => setTimeout(r, 1000));
          }
        }
      }
    }

    poll();
    return () => {
      unmounted = true;
    };
  }, []);

  const refresh = useCallback(async () => {
    try {
      const snap = await api.getSnapshot();
      setSnapshot(snap);
    } catch (e) {
      console.error("Failed to refresh snapshot:", e);
    }
  }, []);

  const selectedWorkspace: MachineWorkspace | undefined =
    snapshot?.workspaces.find((w) => w.id === selectedWorkspaceId) ??
    snapshot?.workspaces[0];

  const selectedFolder: MachineFolder | undefined =
    selectedWorkspace?.folders.find((f) => f.id === selectedFolderId);

  const displayedDownloads: MachineDownload[] = (
    selectedWorkspace?.downloads ?? []
  ).filter((d) => {
    if (selectedFolderId && d.folder_id !== selectedFolderId) {
      return false;
    }
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase().trim();
      return (
        d.name.toLowerCase().includes(q) ||
        d.source_url.toLowerCase().includes(q)
      );
    }
    return true;
  });

  // Calculate network totals
  let totalDownloadRate = 0;
  let totalUploadRate = 0;
  let activeTransfersCount = 0;

  if (snapshot) {
    for (const ws of snapshot.workspaces) {
      for (const dl of ws.downloads) {
        totalDownloadRate += dl.download_rate;
        totalUploadRate += dl.upload_rate;
        if (
          dl.state === "Fetching" ||
          dl.state === "Processing" ||
          dl.state === "Verifying" ||
          dl.state === "Seeding"
        ) {
          activeTransfersCount++;
        }
      }
    }
  }

  return {
    snapshot,
    paths,
    isLoading,
    selectedWorkspaceId,
    setSelectedWorkspaceId,
    selectedFolderId,
    setSelectedFolderId,
    selectedWorkspace,
    selectedFolder,
    displayedDownloads,
    searchQuery,
    setSearchQuery,
    totalDownloadRate,
    totalUploadRate,
    activeTransfersCount,
    refresh,
  };
}
