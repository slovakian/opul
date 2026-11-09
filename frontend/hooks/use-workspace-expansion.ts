import { useCallback, useEffect, useRef, useState } from "react";

import { errorMessage } from "@/lib/bridge";
import { useLibrary } from "@/lib/library";
import type { Preferences, Workspace } from "@/lib/model";

export const useWorkspaceExpansion = (
  preferences: Preferences | null,
  /** The workspaces once the library has loaded. */
  workspaces: readonly Workspace[] | null
) => {
  const { saveWorkspaceExpansion } = useLibrary();

  const [expansion, setExpansion] = useState<ReadonlyMap<string, boolean>>(
    () => new Map()
  );

  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const session = useRef(new Map<string, boolean>());
  const persisted = useRef<Map<string, boolean> | null>(null);
  const pending = useRef(new Map<string, boolean | null>());
  const writing = useRef(false);
  const initialized = useRef(false);

  const save = useCallback(async () => {
    if (writing.current || !persisted.current) {
      return;
    }

    writing.current = true;
    setSaving(true);

    try {
      while (true) {
        for (const [id, expanded] of pending.current) {
          if (
            expanded === null
              ? !persisted.current.has(id)
              : (persisted.current.get(id) ?? false) === expanded
          ) {
            pending.current.delete(id);
          }
        }

        if (pending.current.size === 0) {
          setError(null);
          break;
        }

        const choices = new Map(pending.current);

        // One request at a time; responses acknowledge persistence, never session state.
        // oxlint-disable-next-line no-await-in-loop
        const saved = await saveWorkspaceExpansion(Object.fromEntries(choices));

        persisted.current = new Map(Object.entries(saved));

        for (const [id, expanded] of choices) {
          if (pending.current.get(id) === expanded) {
            pending.current.delete(id);
          }
        }
      }
    } catch (error_) {
      setError(errorMessage(error_));
    }

    writing.current = false;
    setSaving(false);
  }, [saveWorkspaceExpansion]);

  useEffect(() => {
    if (!preferences || !workspaces) {
      return;
    }

    const firstSnapshot = !initialized.current;

    if (firstSnapshot) {
      initialized.current = true;
      persisted.current = new Map(
        Object.entries(preferences.workspace_expansion)
      );

      const initial =
        preferences.workspace_expansion_startup === "collapsed"
          ? new Map<string, boolean>()
          : persisted.current;

      session.current = new Map([...initial, ...session.current]);
    }

    const surviving = new Set(workspaces.map((workspace) => workspace.id));

    const known = new Set([
      ...(persisted.current?.keys() ?? []),
      ...session.current.keys(),
      ...pending.current.keys(),
    ]);

    let removed = false;

    for (const id of known) {
      if (!surviving.has(id) && pending.current.get(id) !== null) {
        session.current.delete(id);
        // A tombstone supersedes queued intent and survives an older acknowledgement.
        pending.current.set(id, null);
        removed = true;
      }
    }

    if (firstSnapshot || removed) {
      setExpansion(new Map(session.current));
      void save();
    }
  }, [workspaces, preferences, save]);

  const handleExpansionChange = useCallback(
    (workspaceId: string, expanded: boolean) => {
      if ((session.current.get(workspaceId) ?? false) === expanded) {
        return;
      }

      session.current.set(workspaceId, expanded);
      pending.current.set(workspaceId, expanded);
      setExpansion(new Map(session.current));
      void save();
    },
    [save]
  );

  return { error, expansion, handleExpansionChange, retry: save, saving };
};
