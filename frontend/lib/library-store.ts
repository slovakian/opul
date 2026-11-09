import type {
  Download,
  FeedMessage,
  Patch,
  Session,
  Snapshot,
  Workspace,
} from "@/lib/model";

/** A download together with the workspace that holds it. */
export type LibraryDownload = Download & { readonly workspace_id: string };

/**
 * The library as the interface reads it. Every update keeps the objects that
 * did not change, so a component that reads only unchanged values can skip
 * rendering by comparing references.
 */
export interface Library {
  readonly boot_id: string | null;
  readonly sequence: number;
  readonly revision: number;
  readonly session: Session;
  readonly recent_emoji: readonly string[];
  readonly recovery_notices: readonly string[];
  readonly workspaces: readonly Workspace[];
  /** Every download, ordered by workspace and then by each workspace's order. */
  readonly downloads: readonly LibraryDownload[];
  readonly downloadById: ReadonlyMap<string, LibraryDownload>;
}

export type LibraryState =
  | { readonly kind: "loading" }
  | { readonly kind: "failed"; readonly message: string }
  | { readonly kind: "ready"; readonly library: Library };

/**
 * Whether two values the desktop sent are the same. Both sides serialize
 * fields in declaration order, so equal values serialize identically.
 */
const isSameValue = <T>(left: T, right: T) =>
  left === right || JSON.stringify(left) === JSON.stringify(right);

/** The previous list when it holds the same items, so readers see no change. */
const keepList = <T>(previous: readonly T[], next: readonly T[]) =>
  previous.length === next.length &&
  previous.every((item, index) => item === next[index])
    ? previous
    : next;

/** Reuses each previous item that is structurally equal to its replacement. */
const reconcileById = <T extends { readonly id: string }>(
  previous: readonly T[],
  next: readonly T[]
): readonly T[] => {
  const known = new Map(previous.map((item) => [item.id, item]));

  return keepList(
    previous,
    next.map((item) => {
      const before = known.get(item.id);

      return before && isSameValue(before, item) ? before : item;
    })
  );
};

const keepValue = <T>(previous: T, next: T) =>
  isSameValue(previous, next) ? previous : next;

const indexDownloads = (downloads: readonly LibraryDownload[]) =>
  new Map(downloads.map((download) => [download.id, download]));

const fromSnapshot = (
  snapshot: Snapshot,
  previous: Library | null
): Library => {
  const workspaces = snapshot.workspaces.map(
    ({ downloads: _downloads, ...workspace }) => workspace
  );

  const downloads = snapshot.workspaces.flatMap((workspace) =>
    workspace.downloads.map((download): LibraryDownload => ({
      ...download,
      workspace_id: workspace.id,
    }))
  );

  if (!previous) {
    return {
      boot_id: snapshot.boot_id,
      downloadById: indexDownloads(downloads),
      downloads,
      recent_emoji: snapshot.recent_emoji,
      recovery_notices: snapshot.recovery_notices,
      revision: snapshot.revision,
      sequence: snapshot.sequence,
      session: snapshot.session,
      workspaces,
    };
  }

  const reconciled = reconcileById(previous.downloads, downloads);

  return {
    boot_id: snapshot.boot_id,
    downloadById:
      reconciled === previous.downloads
        ? previous.downloadById
        : indexDownloads(reconciled),
    downloads: reconciled,
    recent_emoji: keepValue(previous.recent_emoji, snapshot.recent_emoji),
    recovery_notices: keepValue(
      previous.recovery_notices,
      snapshot.recovery_notices
    ),
    revision: snapshot.revision,
    sequence: snapshot.sequence,
    session: snapshot.session,
    workspaces: reconcileById(previous.workspaces, workspaces),
  };
};

const applyPatch = (library: Library, patch: Patch): Library => {
  const outlines = patch.workspaces;
  let { downloadById, downloads, workspaces } = library;

  if (outlines || patch.downloads.length) {
    const placement = new Map(
      (outlines ?? []).flatMap((outline) =>
        outline.download_ids.map((id) => [id, outline.id] as const)
      )
    );

    const changed = new Map<string, LibraryDownload>();

    for (const download of patch.downloads) {
      const workspaceId =
        placement.get(download.id) ??
        downloadById.get(download.id)?.workspace_id;

      if (workspaceId) {
        changed.set(download.id, { ...download, workspace_id: workspaceId });
      }
    }

    if (outlines) {
      workspaces = reconcileById(
        workspaces,
        outlines.map(({ download_ids: _ids, ...workspace }) => workspace)
      );

      const ordered = outlines.flatMap((outline) =>
        outline.download_ids.flatMap((id) => {
          const download = changed.get(id) ?? downloadById.get(id);

          if (!download) {
            return [];
          }

          return download.workspace_id === outline.id
            ? [download]
            : [{ ...download, workspace_id: outline.id }];
        })
      );

      downloads = keepList(downloads, ordered);
      downloadById =
        downloads === library.downloads
          ? downloadById
          : indexDownloads(downloads);
    } else if (changed.size) {
      downloads = downloads.map((item) => changed.get(item.id) ?? item);
      const index = new Map(downloadById);

      for (const download of changed.values()) {
        index.set(download.id, download);
      }

      downloadById = index;
    }
  }

  return {
    ...library,
    downloadById,
    downloads,
    recent_emoji: patch.recent_emoji ?? library.recent_emoji,
    recovery_notices: patch.recovery_notices ?? library.recovery_notices,
    revision: patch.revision,
    sequence: patch.sequence,
    session: patch.session ?? library.session,
    workspaces,
  };
};

/**
 * Applies one feed message. A snapshot can always be applied; a patch needs
 * the library its predecessor produced. A message older than the library
 * changes nothing, so a late reply can never undo a newer change.
 */
export const applyFeedMessage = (
  library: Library | null,
  message: FeedMessage
): Library => {
  const stale =
    library !== null &&
    (message.kind === "patch" || message.boot_id === library.boot_id) &&
    message.sequence < library.sequence;

  if (stale) {
    return library;
  }

  if (message.kind === "snapshot") {
    return fromSnapshot(message, library);
  }

  if (!library) {
    throw new Error("The library feed sent a change before the library.");
  }

  return applyPatch(library, message);
};

export interface LibraryStore {
  getState: () => LibraryState;
  subscribe: (listener: () => void) => () => void;
  /** Applies a feed message from a connection that started with a snapshot. */
  receive: (message: FeedMessage) => void;
  /** Reports a connection failure; a library that already loaded stays visible. */
  fail: (message: string) => void;
  /** Resolves once the library includes `revision`, or after `timeout` ms. */
  reach: (revision: number, timeout: number) => Promise<void>;
}

export const createLibraryStore = (): LibraryStore => {
  let state: LibraryState = { kind: "loading" };
  const listeners = new Set<() => void>();
  const waiters = new Set<{ resolve: () => void; revision: number }>();

  const reached = (revision: number) =>
    state.kind === "ready" && state.library.revision >= revision;

  const setState = (next: LibraryState) => {
    if (next === state) {
      return;
    }

    state = next;

    for (const listener of listeners) {
      listener();
    }

    for (const waiter of waiters) {
      if (reached(waiter.revision)) {
        waiters.delete(waiter);
        waiter.resolve();
      }
    }
  };

  const subscribe = (listener: () => void) => {
    listeners.add(listener);

    return () => {
      listeners.delete(listener);
    };
  };

  return {
    fail: (message) => {
      if (state.kind !== "ready") {
        setState({ kind: "failed", message });
      }
    },
    getState: () => state,
    reach: async (revision, timeout) => {
      if (reached(revision)) {
        return;
      }

      // oxlint-disable-next-line promise/avoid-new -- Resolved from store updates.
      await new Promise<void>((resolve) => {
        const waiter = { resolve, revision };
        waiters.add(waiter);
        setTimeout(() => {
          waiters.delete(waiter);
          resolve();
        }, timeout);
      });
    },
    receive: (message) => {
      const library = state.kind === "ready" ? state.library : null;

      setState({ kind: "ready", library: applyFeedMessage(library, message) });
    },
    subscribe,
  };
};

const noDownloads: readonly LibraryDownload[] = [];
const noWorkspaces: readonly Workspace[] = [];

export const selectWorkspaces = (state: LibraryState) =>
  state.kind === "ready" ? state.library.workspaces : noWorkspaces;

export const selectDownloads = (state: LibraryState) =>
  state.kind === "ready" ? state.library.downloads : noDownloads;

export const selectSession = (state: LibraryState) =>
  state.kind === "ready" ? state.library.session : null;

/** Counts downloads matching `include`, for selectors that should change only with the count. */
export const countDownloads = (
  state: LibraryState,
  include: (download: LibraryDownload) => boolean
) => selectDownloads(state).filter(include).length;
