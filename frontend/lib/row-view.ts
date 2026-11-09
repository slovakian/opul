import type { Download } from "@/lib/model";
import { liveBytes, stateNames } from "@/lib/model";

/**
 * How the bar (or ring) moves:
 * - `sheen`: Opul is working, so a soft highlight sweeps the empty track.
 * - `muted`: held (pausing or paused), so the fill greys out and stays still.
 * - `pulse`: resuming, so the fill keeps its value and gently pulses.
 * - `still`: full and waiting on a last step (processing).
 * - `searching`: a stalled torrent looking for peers, so the fill greys out
 *   and breathes slowly.
 */
export type BarMotion = "sheen" | "muted" | "pulse" | "still" | "searching";

export interface BarView {
  /** Percent done, or null for the no-progress look. */
  value: number | null;
  motion: BarMotion;
  /** The bar's tooltip and `aria-valuetext`. */
  label: string;
}

export type StatusText =
  | { kind: "seeding"; upload: number | null }
  | { kind: "waiting"; label: string }
  | { kind: "failed" }
  | { kind: "label"; text: string; tone: "plain" | "shimmer" };

export interface DescriptionMessage {
  text: string;
  tone: "plain" | "shimmer" | "error";
}

export interface DescriptionView {
  /** Which line shows: live numbers, a message, or where it came from. */
  line: "live" | "message" | "origin";
  amount: { done: string; total: string | null };
  /** Bytes per second, only while it's moving. */
  speed: number | null;
  /** Seconds left, only while it can be estimated. */
  eta: number | null;
  /** A word after the amount in place of speed and time left. */
  note: string | null;
  message: DescriptionMessage | null;
  /** The origin line leaves out the size while there's none to show yet. */
  size: boolean;
  /** Below 440px the status slot keeps only the ring and the check, so its text joins this line. */
  status: StatusText | null;
}

export interface RowView {
  status: "progress" | "text" | "empty";
  bar: BarView | null;
  statusText: StatusText | null;
  /**
   * The text took the bar's place, but the bar still has something to say:
   * below 440px the ring stays and the text moves under the name.
   */
  ring: boolean;
  description: DescriptionView;
}

export interface RowOptions {
  /**
   * A stalled torrent has gone a few seconds without data or peers (see
   * `useLookingForPeers`). Its bar stays with the `searching` look, and
   * "Looking for peers…" joins the line under the name.
   */
  lookingForPeers?: boolean;
}

/** Opul is actively working for you. */
export const workingStates = new Set<Download["state"]>([
  "Starting",
  "Fetching",
  "Processing",
  "Verifying",
]);

const heldStates = new Set<Download["state"]>(["Pausing", "Paused"]);

const progressStates = new Set<Download["state"]>([
  ...workingStates,
  ...heldStates,
]);

const FULL = 100;
const MILLIONTHS_PER_PERCENT = 10_000;

/** A resumed download goes back through Starting with its bytes still there. */
export const isResuming = (download: Download) =>
  download.state === "Starting" && download.bytes_downloaded > 0;

/** A magnet link waiting for its file list: nothing to measure yet. */
export const isGettingFileInfo = (download: Download) =>
  download.source === "Torrent" &&
  (download.state === "Starting" || download.state === "Fetching") &&
  download.bytes_downloaded === 0 &&
  download.total_size === 0;

/**
 * A torrent downloading at 0 B/s with nobody connected. The row waits a few
 * seconds before saying so, which `useLookingForPeers` decides.
 */
export const isStalled = (download: Download) =>
  download.source === "Torrent" &&
  download.state === "Fetching" &&
  download.total_size > 0 &&
  download.download_rate === 0 &&
  download.peers === 0;

const GETTING_FILE_INFO = "Getting file info…";
const LOOKING_FOR_PEERS = "Looking for peers…";

const positive = (value: number | null) =>
  value !== null && value > 0 ? value : null;

const percent = (download: Download) =>
  Math.min(
    FULL,
    Math.max(0, download.progress_millionths / MILLIONTHS_PER_PERCENT)
  );

const barValue = (download: Download) => {
  if (download.state === "Processing") {
    return FULL;
  }

  if (download.state === "Starting" && !isResuming(download)) {
    return null;
  }

  return download.total_size > 0 ? percent(download) : null;
};

const isLookingForPeers = (
  download: Download,
  { lookingForPeers = false }: RowOptions
) => lookingForPeers && isStalled(download);

const barMotion = (
  download: Download,
  value: number | null,
  options: RowOptions
): BarMotion => {
  if (isLookingForPeers(download, options)) {
    return "searching";
  }

  if (heldStates.has(download.state)) {
    return "muted";
  }

  if (download.state === "Processing") {
    return "still";
  }

  // Without a value there's no fill to pulse, so it keeps the no-progress look.
  return isResuming(download) && value !== null ? "pulse" : "sheen";
};

const barLabel = (
  download: Download,
  value: number | null,
  options: RowOptions
) => {
  if (download.state === "Processing") {
    return "Processing…";
  }

  if (isLookingForPeers(download, options)) {
    return value === null
      ? LOOKING_FOR_PEERS
      : `${Math.round(value)}% · ${LOOKING_FOR_PEERS}`;
  }

  const held = heldStates.has(download.state);
  const resuming = isResuming(download);

  if (isGettingFileInfo(download)) {
    return GETTING_FILE_INFO;
  }

  if (value === null) {
    if (held) {
      return "Paused";
    }

    return resuming ? "Resuming…" : `${stateNames[download.state]}…`;
  }

  const shown = `${Math.round(value)}%`;

  if (held) {
    return `Paused at ${shown}`;
  }

  return resuming ? `Resuming at ${shown}` : shown;
};

const barView = (download: Download, options: RowOptions): BarView | null => {
  if (!progressStates.has(download.state)) {
    return null;
  }

  const value = barValue(download);

  return {
    label: barLabel(download, value, options),
    motion: barMotion(download, value, options),
    value,
  };
};

const statusTextView = (download: Download): StatusText | null => {
  if (isGettingFileInfo(download)) {
    return { kind: "label", text: GETTING_FILE_INFO, tone: "shimmer" };
  }

  switch (download.state) {
    case "Seeding": {
      // Peers live in the details panel; a connection alone changes nothing here.
      return { kind: "seeding", upload: positive(download.upload_rate) };
    }

    case "CheckingFiles": {
      return { kind: "label", text: "Checking files…", tone: "shimmer" };
    }

    case "WaitingForVideoTools": {
      return {
        kind: "waiting",
        label: download.waiting_label ?? stateNames.WaitingForVideoTools,
      };
    }

    case "Failed": {
      return { kind: "failed" };
    }

    case "Queued":
    case "Cancelled": {
      return { kind: "label", text: stateNames[download.state], tone: "plain" };
    }

    default: {
      return null;
    }
  }
};

const noteFor = (download: Download, options: RowOptions) => {
  if (isLookingForPeers(download, options)) {
    return LOOKING_FOR_PEERS;
  }

  switch (download.state) {
    case "Pausing": {
      return "Pausing…";
    }

    case "Paused": {
      return "Paused";
    }

    default: {
      return isResuming(download) ? "Resuming…" : null;
    }
  }
};

const messageFor = (download: Download): DescriptionMessage | null => {
  switch (download.state) {
    case "Processing": {
      return { text: "Processing…", tone: "shimmer" };
    }

    case "Verifying": {
      return { text: "Verifying…", tone: "plain" };
    }

    case "Failed": {
      return download.error_message
        ? { text: download.error_message, tone: "error" }
        : null;
    }

    default: {
      return null;
    }
  }
};

const descriptionView = (
  download: Download,
  status: StatusText | null,
  options: RowOptions
): DescriptionView => {
  const note = noteFor(download, options);
  const message = messageFor(download);
  const gettingFileInfo = isGettingFileInfo(download);

  const live =
    !gettingFileInfo && (download.state === "Fetching" || note !== null);

  const fetching = download.state === "Fetching";

  let line: DescriptionView["line"] = "origin";

  if (message) {
    line = "message";
  } else if (live) {
    line = "live";
  }

  return {
    amount: {
      done: liveBytes(download.bytes_downloaded),
      total: download.total_size > 0 ? liveBytes(download.total_size) : null,
    },
    eta: fetching ? download.eta_seconds : null,
    line,
    message,
    note,
    size: !gettingFileInfo,
    speed: fetching ? positive(download.download_rate) : null,
    status,
  };
};

/**
 * Everything a list row shows for one snapshot of a download. Purely visual
 * timing (the finish check, the speed grace period, fade-outs) lives in hooks.
 */
export const rowView = (
  download: Download,
  options: RowOptions = {}
): RowView => {
  const bar = barView(download, options);
  const statusText = statusTextView(download);

  // "Getting file info…" takes the bar's place; anything else with a bar shows it.
  const hint = bar !== null && statusText !== null;

  let status: RowView["status"] = "empty";

  if (bar && !hint) {
    status = "progress";
  } else if (statusText) {
    status = "text";
  }

  return {
    bar,
    description: descriptionView(download, statusText, options),
    ring: hint,
    status,
    statusText,
  };
};
