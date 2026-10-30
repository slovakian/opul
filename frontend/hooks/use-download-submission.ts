import { useRef, useState } from "react";
import type { Dispatch, SetStateAction } from "react";

import { toast } from "@/components/ui/toast";
import { bridge, errorMessage } from "@/lib/bridge";
import type { Destination } from "@/lib/destination";
import { downloadMode } from "@/lib/download-options";
import type { DownloadMode, DownloadType } from "@/lib/download-options";
import { useLibrary } from "@/lib/library";
import type { Format, Workspace } from "@/lib/model";

type Submission =
  | {
      kind: "url";
      query: string;
      mode: DownloadMode;
      format: Format | undefined;
    }
  | { kind: "torrent"; path: string };

export const useDownloadSubmission = ({
  destination,
  workspace,
  navigation,
  query,
  setQuery,
  disabled,
  onAdded,
  onUsed,
  selectDestination,
}: {
  destination: Destination | undefined;
  workspace: Workspace | undefined;
  navigation: number;
  query: string;
  setQuery: Dispatch<SetStateAction<string>>;
  disabled: boolean;
  onAdded: (ids: string[]) => void;
  onUsed: (id: string) => void;
  selectDestination: (next: Destination) => void;
}) => {
  const { run, preferences, savePreferences } = useLibrary();
  const [type, setType] = useState<DownloadType>("Automatic");

  const [formatOverride, setFormatOverride] = useState<{
    navigation: number;
    format: Format;
  }>();

  const format =
    formatOverride && formatOverride.navigation === navigation
      ? formatOverride.format
      : (workspace?.default_format ?? "Best");

  const setFormat = (next: Format | undefined) =>
    setFormatOverride(next ? { format: next, navigation } : undefined);

  const [adding, setAdding] = useState(false);
  const inFlight = useRef(false);
  const [uncertain, setUncertain] = useState<string>();

  const [confirmation, setConfirmation] = useState<{
    navigation: number;
    submission: Submission;
  }>();

  const locked = disabled || adding;

  const download = async (
    next: Destination,
    submission: Submission,
    automatic = false
  ) => {
    if (locked || inFlight.current) {
      return;
    }

    inFlight.current = true;
    setAdding(true);

    try {
      if (
        automatic &&
        (!preferences ||
          !(await savePreferences({
            ...preferences,
            aggregate_destination: {
              kind: "workspace",
              workspace_id: next.workspace.id,
            },
          })))
      ) {
        inFlight.current = false;
        setAdding(false);

        return;
      }

      const receipt = await run(
        submission.kind === "torrent"
          ? {
              folder_id: next.folder.id,
              kind: "add_torrent_file",
              path: submission.path,
              workspace_id: next.workspace.id,
            }
          : {
              folder_id: next.folder.id,
              format: submission.format ?? next.workspace.default_format,
              kind: "add_url",
              mode: submission.mode,
              url: submission.query.trim(),
              workspace_id: next.workspace.id,
            }
      );

      if (receipt) {
        selectDestination(next);
        onUsed(next.workspace.id);
        setConfirmation(undefined);

        if (submission.kind === "url") {
          setQuery((current) => (current === submission.query ? "" : current));
        }

        setType("Automatic");
        setFormatOverride(undefined);
        onAdded(receipt.created_download_ids);
      }
    } catch (error) {
      toast.add({
        description: errorMessage(error),
        title: "Couldn't add download",
        type: "error",
      });
    }

    inFlight.current = false;
    setAdding(false);
  };

  const dispatch = async (submission: Submission) => {
    if (destination) {
      await download(destination, submission);
    } else {
      setConfirmation({ navigation, submission });
    }
  };

  const createUrlSubmission = (
    mode: DownloadMode,
    targetQuery: string
  ): Submission => ({
    format:
      formatOverride?.navigation === navigation
        ? formatOverride.format
        : undefined,
    kind: "url",
    mode,
    query: targetQuery,
  });

  const submit = async () => {
    if (locked || inFlight.current) {
      return;
    }

    const mode = type === "Automatic" ? downloadMode(query.trim()) : type;

    if (!mode) {
      setUncertain(query);

      return;
    }

    await dispatch(createUrlSubmission(mode, query));
  };

  const importTorrent = async () => {
    if (locked || inFlight.current) {
      return;
    }

    inFlight.current = true;
    setAdding(true);
    let path: string | null = null;

    try {
      path = await bridge.chooseTorrent();
    } catch (error) {
      toast.add({
        description: errorMessage(error),
        title: "Couldn't open torrent",
        type: "error",
      });
    }

    inFlight.current = false;
    setAdding(false);

    if (path) {
      await dispatch({ kind: "torrent", path });
    }
  };

  return {
    adding,
    chooseType: async (mode: DownloadMode) => {
      const submitted = uncertain;
      setUncertain(undefined);

      if (submitted !== undefined) {
        await dispatch(createUrlSubmission(mode, submitted));
      }
    },
    closeConfirmation: () => setConfirmation(undefined),
    closeUncertain: () => setUncertain(undefined),
    confirm: async (next: Destination, automatic: boolean) => {
      if (confirmation) {
        await download(next, confirmation.submission, automatic);
      }
    },
    confirming: confirmation?.navigation === navigation,
    format,
    importTorrent,
    locked,
    setFormat,
    setType,
    submit,
    type,
    uncertain,
  };
};
