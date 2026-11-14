import { cn } from "cn";
import { Download, RefreshCw, RotateCw } from "lucide-react";
import type * as React from "react";
import { useState } from "react";

import opulIcon from "@/assets/opul-icon.png";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { CircularProgress } from "@/components/ui/circular-progress";
import { Field, FieldLabel } from "@/components/ui/field";
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item";
import {
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverHeader,
  PopoverTitle,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Switch } from "@/components/ui/switch";
import {
  cancelAppUpdateDownload,
  checkForAppUpdates,
  configureAppUpdates,
  downloadAppUpdate,
  requestAppUpdateRestart,
  useAppUpdates,
} from "@/hooks/use-app-updates";
import { appName, blockerText, downloadPercent } from "@/lib/app-updates";
import type { AppUpdatePhase, AppUpdates } from "@/lib/app-updates";
import { truncateError } from "@/lib/truncate";
import { checkedAgo } from "@/lib/video-tools";

const STAGGER_MS = 40;

/** Matches the footprint of "Check for Updates" so the ring doesn't shift the row. */
const ACTION_SLOT_CLASS = "min-w-[9.75rem] justify-end";

const stagger = (index: number): React.CSSProperties => ({
  animationDelay: `${index * STAGGER_MS}ms`,
});

/** One short line that says where things stand. */
const statusLine = (updates: AppUpdates): string => {
  const { phase } = updates;
  const name = appName(updates);

  switch (phase.kind) {
    case "checking": {
      return "Checking for updates…";
    }

    case "up_to_date": {
      return "You're up to date";
    }

    case "available": {
      return `${name} ${phase.version} is available`;
    }

    case "downloading": {
      const percent = downloadPercent(phase);

      return `Downloading ${phase.version}…${percent === null ? "" : ` ${percent}%`}`;
    }

    case "ready": {
      return `${name} ${phase.version} is ready. Restart to finish.`;
    }

    case "check_failed":
    case "download_failed": {
      return phase.message;
    }

    default: {
      return "Not checked yet";
    }
  }
};

/** Circular progress that opens Cancel / Continue — same ring as the download list. */
const DownloadProgressAction = ({
  phase,
}: {
  phase: Extract<AppUpdatePhase, { kind: "downloading" }>;
}) => {
  const [open, setOpen] = useState(false);
  const percent = downloadPercent(phase);

  const label =
    percent === null
      ? `Downloading ${phase.version}`
      : `Downloading ${phase.version}, ${percent}%`;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        render={
          <Button
            type="button"
            size="icon-sm"
            variant="ghost"
            aria-label={`${label}. Open download options.`}
            className="text-foreground"
          />
        }
      >
        <CircularProgress
          value={percent}
          size={20}
          thickness={3}
          aria-label={label}
          getAriaValueText={() =>
            percent === null ? "Download in progress" : `${percent} percent`
          }
        />
      </PopoverTrigger>
      <PopoverContent align="end" className="w-56 gap-3 p-3">
        <PopoverHeader className="gap-0.5">
          <PopoverTitle>Downloading {phase.version}</PopoverTitle>
          <PopoverDescription>
            {percent === null
              ? "Download in progress."
              : `${percent}% complete.`}
          </PopoverDescription>
        </PopoverHeader>
        <div className="flex justify-end gap-2">
          <Button
            type="button"
            size="xs"
            variant="outline"
            onClick={() => {
              void cancelAppUpdateDownload();
              setOpen(false);
            }}
          >
            Cancel download
          </Button>
          <Button type="button" size="xs" onClick={() => setOpen(false)}>
            Continue
          </Button>
        </div>
      </PopoverContent>
    </Popover>
  );
};

const UpdateAction = ({
  activeDownloads,
  updates,
}: {
  activeDownloads: number;
  updates: AppUpdates;
}) => {
  const { install_blocker: blocker, phase } = updates;

  switch (phase.kind) {
    case "available":
    case "download_failed": {
      const retry = phase.kind === "download_failed";

      return (
        <Button
          size="sm"
          disabled={blocker !== null}
          onClick={() => {
            void downloadAppUpdate();
          }}
        >
          {retry ? (
            <RotateCw data-icon="inline-start" aria-hidden="true" />
          ) : (
            <Download data-icon="inline-start" aria-hidden="true" />
          )}
          {retry ? "Try Again" : "Download"}
        </Button>
      );
    }

    case "ready": {
      return (
        <Button
          size="sm"
          onClick={() => {
            void requestAppUpdateRestart(activeDownloads);
          }}
        >
          Restart
        </Button>
      );
    }

    case "downloading": {
      return <DownloadProgressAction phase={phase} />;
    }

    default: {
      const checking = phase.kind === "checking";

      return (
        <Button
          size="sm"
          variant="outline"
          disabled={checking}
          onClick={() => {
            void checkForAppUpdates();
          }}
        >
          {checking ? (
            <Spinner data-icon="inline-start" />
          ) : (
            <RefreshCw data-icon="inline-start" aria-hidden="true" />
          )}
          {checking ? "Checking…" : "Check for Updates"}
        </Button>
      );
    }
  }
};

const VersionCard = ({
  activeDownloads,
  updates,
}: {
  activeDownloads: number;
  updates: AppUpdates;
}) => {
  const { last_checked: lastChecked, phase } = updates;
  const status = statusLine(updates);

  const failed =
    phase.kind === "check_failed" || phase.kind === "download_failed";

  const showLastChecked =
    lastChecked !== null &&
    (phase.kind === "idle" || phase.kind === "up_to_date");

  return (
    <Item
      variant="muted"
      size="sm"
      aria-label="Version"
      className="animate-settle flex-nowrap px-4 py-3"
      style={stagger(0)}
    >
      <ItemMedia className="translate-y-0! self-center!">
        <img src={opulIcon} alt="" className="size-9" />
      </ItemMedia>
      <ItemContent className="min-w-0 gap-0.5">
        <ItemTitle className="tabular-nums">
          {appName(updates)} {updates.current_version}
          <Badge variant="outline" className="capitalize">
            {updates.channel}
          </Badge>
        </ItemTitle>
        <ItemDescription
          aria-live="polite"
          className={cn("line-clamp-2 text-xs", failed && "text-destructive")}
        >
          <span className={cn(phase.kind === "checking" && "shimmer")}>
            {status}
          </span>
          {showLastChecked && ` · ${checkedAgo(lastChecked)}`}
          {updates.release_notes_url && (
            <>
              {" · "}
              <a
                href={updates.release_notes_url}
                target="_blank"
                rel="noopener noreferrer"
              >
                What&apos;s new
              </a>
            </>
          )}
        </ItemDescription>
      </ItemContent>
      <ItemActions className={ACTION_SLOT_CLASS}>
        <UpdateAction activeDownloads={activeDownloads} updates={updates} />
      </ItemActions>
    </Item>
  );
};

const Preferences = ({ updates }: { updates: AppUpdates }) => {
  const { check_automatically: check, install_automatically: install } =
    updates;

  return (
    <section
      aria-label="Update preferences"
      className="animate-settle bg-card divide-y overflow-hidden rounded-xl border"
      style={stagger(1)}
    >
      <Field orientation="horizontal" className="min-h-12 px-4 py-2.5">
        <FieldLabel
          htmlFor="check-updates-automatically"
          className="font-normal"
        >
          Check for updates automatically
        </FieldLabel>
        <Switch
          id="check-updates-automatically"
          checked={check}
          onCheckedChange={(enabled) => {
            void configureAppUpdates(enabled, install);
          }}
        />
      </Field>
      <Field
        orientation="horizontal"
        data-disabled={!check || undefined}
        className="min-h-12 px-4 py-2.5"
      >
        <FieldLabel
          htmlFor="install-updates-automatically"
          className="font-normal"
        >
          Install updates automatically
        </FieldLabel>
        <Switch
          id="install-updates-automatically"
          checked={check && install}
          disabled={!check}
          onCheckedChange={(enabled) => {
            void configureAppUpdates(check, enabled);
          }}
        />
      </Field>
    </section>
  );
};

const UpdatesSkeleton = ({ error }: { error: string }) => (
  <div className="flex flex-col gap-3">
    {error ? (
      <p
        role="alert"
        className="text-destructive text-sm wrap-break-word"
        title={error}
      >
        {truncateError(error)}
      </p>
    ) : (
      <output className="sr-only">Loading updates…</output>
    )}
    <Skeleton className="h-16 w-full rounded-xl" />
    <Skeleton className="h-24 w-full rounded-xl" />
  </div>
);

export const UpdatesSettings = ({
  activeDownloads,
}: {
  activeDownloads: number;
}) => {
  const { error, updates } = useAppUpdates();

  if (!updates) {
    return <UpdatesSkeleton error={error} />;
  }

  const blocked = updates.install_blocker;

  return (
    <div className="flex flex-col gap-3">
      <VersionCard activeDownloads={activeDownloads} updates={updates} />
      {blocked && (
        <p className="text-muted-foreground animate-settle px-1 text-xs">
          {blockerText[blocked]}
        </p>
      )}
      {error && (
        <p
          role="alert"
          className="text-destructive animate-settle px-1 text-xs wrap-break-word"
          title={error}
        >
          {truncateError(error)}
        </p>
      )}
      <Preferences updates={updates} />
    </div>
  );
};
