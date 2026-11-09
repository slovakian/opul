import type { Download } from "@/lib/model";
import { liveBytes, stateNames } from "@/lib/model";

/**
 * How the bar (or ring) moves:
 * - `sheen`: Opul is working, so a soft highlight sweeps the empty track.
 * - `muted`: held (pausing or paused), so the fill greys out and stays still.
 * - `pulse`: resuming, so the fill keeps its value and gently pulses.
 * - `still`: full and waiting on a last step (processing).
 */
export type BarMotion = "sheen" | "muted" | "pulse" | "still";

export interface BarView {
  /** Percent done, or null for the no-progress look. */
  value: number | null;
  motion: BarMotion;
  /** The bar's tooltip and `aria-valuetext`. */
  label: string;
}

export type StatusText =
  | { kind: "seeding"; upload: number | null; peers: number | null }
  | { kind: "waiting"; label: string }
  | { kind: "failed" }
  | { kind: "label"; text: string };

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
}

export interface RowView {
  status: "progress" | "text" | "empty";
  bar: BarView | null;
  statusText: StatusText | null;
  description: DescriptionView;
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

const barMotion = (download: Download, value: number | null): BarMotion => {
  if (heldStates.has(download.state)) {
    return "muted";
  }

  if (download.state === "Processing") {
    return "still";
  }

  // Without a value there's no fill to pulse, so it keeps the no-progress look.
  return isResuming(download) && value !== null ? "pulse" : "sheen";
};

const barLabel = (download: Download, value: number | null) => {
  if (download.state === "Processing") {
    return "Processing…";
  }

  const held = heldStates.has(download.state);
  const resuming = isResuming(download);

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

const barView = (download: Download): BarView | null => {
  if (!progressStates.has(download.state)) {
    return null;
  }

  const value = barValue(download);

  return {
    label: barLabel(download, value),
    motion: barMotion(download, value),
    value,
  };
};

const statusTextView = (download: Download): StatusText | null => {
  switch (download.state) {
    case "Seeding": {
      return {
        kind: "seeding",
        peers: positive(download.peers),
        upload: positive(download.upload_rate),
      };
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
      return { kind: "label", text: stateNames[download.state] };
    }

    default: {
      return null;
    }
  }
};

const noteFor = (download: Download) => {
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

const descriptionView = (download: Download): DescriptionView => {
  const note = noteFor(download);
  const message = messageFor(download);
  const live = download.state === "Fetching" || note !== null;
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
    speed: fetching ? positive(download.download_rate) : null,
  };
};

/**
 * Everything a list row shows for one snapshot of a download. Purely visual
 * timing (the finish check, the speed grace period, fade-outs) lives in hooks.
 */
export const rowView = (download: Download): RowView => {
  const bar = barView(download);
  const statusText = statusTextView(download);

  let status: RowView["status"] = "empty";

  if (bar) {
    status = "progress";
  } else if (statusText) {
    status = "text";
  }

  return {
    bar,
    description: descriptionView(download),
    status,
    statusText,
  };
};
