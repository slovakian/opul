import { cn } from "cn";
import {
  Download,
  FileDown,
  ListVideo,
  Magnet,
  Music,
  RotateCw,
} from "lucide-react";
import { useRef } from "react";
import type * as React from "react";

import opulIcon from "@/assets/opul-icon.png";
import { InfoTip } from "@/components/tools-primitives";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { CircularProgress } from "@/components/ui/circular-progress";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldLabel,
  FieldTitle,
} from "@/components/ui/field";
import { Trace, TraceIcon, TraceLine, TraceTitle } from "@/components/ui/trace";
import { VideoToolsProgress } from "@/components/video-tools-progress";
import type { useVideoSetup } from "@/hooks/use-video-setup";
import { downloadSize, setupFraction } from "@/lib/video-tools";
import type { SetupStep, VideoToolsSnapshot } from "@/lib/video-tools";

type Setup = ReturnType<typeof useVideoSetup>;

type Execute = Setup["execute"];

type Phase = "offer" | "installing" | "ready" | "failed";

type NotWorking = Extract<VideoToolsSnapshot["state"], { kind: "not_working" }>;

/** Entrance stagger. Short gaps so the screen reads as one arrival. */
const RISE_GAP_MS = 60;

/** The video features light up one after another when setup finishes. */
const LIGHT_UP_GAP_MS = 90;

const PERCENT = 100;

const UNFINISHED_RING = 0.96;

const copy: Record<Phase, { description: string; title: string }> = {
  failed: {
    description: "Setup couldn't finish. Files and torrents still work.",
    title: "Set Up Video Tools",
  },
  installing: {
    description: "Setup keeps going while you use Opul.",
    title: "Set Up Video Tools",
  },
  offer: {
    description:
      "Files and torrents work right away. Video tools add downloads from video pages.",
    title: "Set Up Video Tools",
  },
  ready: {
    description: "Opul can now download from video pages.",
    title: "Video tools are ready",
  },
};

const failureTitles: Record<NotWorking["reason"], string> = {
  download: "The download couldn't finish",
  health_check: "The video tools didn't pass their health check",
  stopped_working: "The video tools stopped working",
  verification: "The download couldn't be verified",
};

const featureGroups = [
  {
    features: [
      { icon: FileDown, label: "Direct files" },
      { icon: Magnet, label: "Torrents" },
    ],
    hint: undefined,
    id: "setup-included",
    label: "Included",
    video: false,
  },
  {
    features: [
      { icon: ListVideo, label: "Videos and playlists" },
      { icon: Music, label: "Audio from videos" },
    ],
    hint: "Python and yt-dlp, downloaded once and kept separate from your files.",
    id: "setup-with-video-tools",
    label: "With video tools",
    video: true,
  },
] as const;

const finishedSteps: SetupStep[] = (["python", "yt_dlp"] as const).map(
  (component) => ({
    component,
    downloaded_bytes: 0,
    status: "done",
    total_bytes: null,
  })
);

const phaseOf = (state: VideoToolsSnapshot["state"] | undefined): Phase => {
  switch (state?.kind) {
    case "setting_up": {
      return "installing";
    }

    case "ready": {
      return "ready";
    }

    case "not_working": {
      return "failed";
    }

    default: {
      return "offer";
    }
  }
};

const rise = (order: number): React.CSSProperties => ({
  animationDelay: `${order * RISE_GAP_MS}ms`,
});

/**
 * The app icon on a soft glow drawn from its own colours. While setup runs, a
 * thin ring around it fills with overall progress; once ready the ring
 * finishes, then fades so the icon is left on its own.
 */
const Hero = ({ phase, progress }: { phase: Phase; progress: number }) => {
  const working = phase === "installing";
  const ringVisible = working || phase === "ready";

  return (
    <div className="animate-rise relative grid size-24 place-items-center">
      <img
        src={opulIcon}
        alt=""
        aria-hidden="true"
        className={cn(
          "absolute size-16 scale-110 blur-xl transition-opacity duration-700 ease-(--ease-out)",
          working && "animate-breathe",
          phase === "ready" ? "opacity-80" : "opacity-45"
        )}
      />
      <img
        src={opulIcon}
        alt=""
        aria-hidden="true"
        className="relative size-16"
        draggable={false}
      />
      <CircularProgress
        aria-hidden="true"
        value={ringVisible ? progress * PERCENT : 0}
        size={96}
        thickness={2}
        className={cn(
          "absolute inset-0 transition-opacity ease-(--ease-out)",
          ringVisible ? "opacity-100" : "opacity-0",
          phase === "ready"
            ? "opacity-0 delay-500 duration-500"
            : "duration-300"
        )}
      />
    </div>
  );
};

const Features = ({ ready }: { ready: boolean }) => (
  <div className="bg-card/60 grid w-full grid-cols-2 divide-x overflow-hidden rounded-xl border backdrop-blur-sm">
    {featureGroups.map((group) => (
      <section
        key={group.id}
        aria-labelledby={group.id}
        className="flex min-w-0 flex-col gap-3 p-4"
      >
        <div className="flex h-4 items-center gap-1.5">
          <h3 id={group.id} className="text-muted-foreground text-xs">
            {group.label}
          </h3>
          {group.hint && (
            <InfoTip label={`About ${group.label.toLowerCase()}`}>
              {group.hint}
            </InfoTip>
          )}
        </div>
        <Trace aria-label={group.label} className="gap-2.5">
          {group.features.map(({ icon: Icon, label }, index) => {
            const available = !group.video || ready;

            return (
              <TraceLine
                key={label}
                status={available ? "available" : undefined}
                className="duration-500 ease-(--ease-out)"
                style={{
                  transitionDelay: group.video
                    ? `${index * LIGHT_UP_GAP_MS}ms`
                    : undefined,
                }}
              >
                <TraceIcon>
                  <Icon />
                </TraceIcon>
                <TraceTitle data-unavailable={!available}>{label}</TraceTitle>
              </TraceLine>
            );
          })}
        </Trace>
      </section>
    ))}
  </div>
);

/** The one preference worth deciding up front, as a choice card. */
const KeepUpToDate = ({
  busy,
  execute,
  video,
}: {
  busy: boolean;
  execute: Execute;
  video: VideoToolsSnapshot | undefined;
}) => (
  <FieldLabel
    htmlFor="setup-keep-up-to-date"
    // The border marks it as chosen; a tinted fill would cost the description its contrast.
    className="bg-card has-data-checked:bg-card dark:has-data-checked:bg-card"
  >
    <Field orientation="horizontal" data-disabled={busy || !video}>
      <FieldContent>
        <FieldTitle>Keep video tools up to date</FieldTitle>
        <FieldDescription className="text-xs">
          Video sites change often. Opul tests each update first.
        </FieldDescription>
      </FieldContent>
      <Checkbox
        id="setup-keep-up-to-date"
        checked={video?.keep_up_to_date ?? true}
        disabled={busy || !video}
        onCheckedChange={(enabled) =>
          execute({ enabled, kind: "set_keep_up_to_date" })
        }
      />
    </Field>
  </FieldLabel>
);

const OfferActions = ({
  busy,
  close,
  execute,
  failure,
  primary,
  video,
}: {
  busy: boolean;
  close: () => void;
  execute: Execute;
  failure: NotWorking | null;
  primary: React.RefObject<HTMLButtonElement | null>;
  video: VideoToolsSnapshot | undefined;
}) => {
  const size = downloadSize(video?.published_size ?? null);

  return (
    <>
      {failure ? (
        <Alert variant="destructive">
          <AlertTitle>{failureTitles[failure.reason]}</AlertTitle>
          <AlertDescription className="select-text">
            {failure.details}
          </AlertDescription>
        </Alert>
      ) : (
        <KeepUpToDate busy={busy} execute={execute} video={video} />
      )}
      <div className="flex flex-col gap-1.5">
        <Button
          ref={primary}
          size="lg"
          disabled={busy || !video}
          className="h-11 rounded-lg transition-[transform,background-color] duration-150 ease-(--ease-out) active:scale-[0.98]"
          onClick={() => execute({ kind: failure ? "retry" : "set_up" })}
        >
          {failure ? (
            <RotateCw data-icon="inline-start" aria-hidden="true" />
          ) : (
            <Download data-icon="inline-start" aria-hidden="true" />
          )}
          {failure ? (
            "Try again"
          ) : (
            <>
              Download video tools{" "}
              <span className="text-primary-foreground/60 font-normal tabular-nums">
                ({size})
              </span>
            </>
          )}
        </Button>
        <Button
          variant="ghost"
          size="lg"
          className="text-muted-foreground h-10 rounded-lg"
          onClick={close}
        >
          Not now
        </Button>
      </div>
    </>
  );
};

const ProgressActions = ({
  close,
  primary,
  steps,
}: {
  close: () => void;
  primary: React.RefObject<HTMLButtonElement | null>;
  steps: SetupStep[];
}) => (
  <>
    <div className="bg-card/60 rounded-xl border px-4 py-3.5 backdrop-blur-sm">
      <VideoToolsProgress steps={steps} />
    </div>
    <Button
      ref={primary}
      size="lg"
      className="h-11 rounded-lg transition-[transform,background-color] duration-150 ease-(--ease-out) active:scale-[0.98]"
      onClick={close}
    >
      Continue to Opul
    </Button>
  </>
);

export const VideoSetup = ({ setup }: { setup: Setup }) => {
  const { busy, close, error, execute, open, tools } = setup;
  const primary = useRef<HTMLButtonElement>(null);
  const video = tools?.video;
  const state = video?.state;
  const phase = phaseOf(state);
  const steps = state?.kind === "setting_up" ? state.steps : finishedSteps;
  const failure = state?.kind === "not_working" ? state : null;

  // Downloads can finish before verifying and installing do, so the ring only
  // closes once the tools are actually ready.
  const progress =
    phase === "ready" ? 1 : Math.min(UNFINISHED_RING, setupFraction(steps));

  const { description, title } = copy[phase];

  const offerTurnOff =
    (phase === "offer" || phase === "failed") && state?.kind !== "turned_off";

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) {
          close();
        }
      }}
    >
      <DialogContent
        initialFocus={primary}
        className="bg-background data-open:fade-in-0 data-closed:fade-out-0 data-closed:zoom-out-[1.02] data-open:zoom-in-100 inset-0 flex h-svh max-w-none translate-x-0 translate-y-0 flex-col gap-0 overflow-hidden rounded-none p-0 ring-0 data-closed:duration-200 data-open:duration-300 sm:max-w-none"
      >
        {/* A wash of the icon's colours, far behind everything. */}
        <img
          src={opulIcon}
          alt=""
          aria-hidden="true"
          className="pointer-events-none absolute -top-90 left-1/2 size-140 max-w-none -translate-x-1/2 opacity-20 blur-[100px] select-none dark:opacity-15"
        />
        {/* The window has no title bar here, so keep it draggable. */}
        <div data-tauri-drag-region className="absolute inset-x-0 top-0 h-12" />
        <div className="relative flex min-h-0 flex-1 flex-col overflow-y-auto">
          {/* Anchored to the top, not centred, so the hero and features stay put
              while the actions below change height between phases. */}
          <div className="mx-auto mt-[max(3.5rem,11vh)] mb-auto flex w-full max-w-md flex-col items-center gap-8 px-6 pb-14">
            <Hero phase={phase} progress={progress} />
            <DialogHeader
              className="animate-rise items-center gap-2 text-center"
              style={rise(1)}
            >
              <DialogTitle
                key={title}
                className="animate-settle text-2xl font-semibold tracking-tight"
              >
                {title}
              </DialogTitle>
              <DialogDescription
                key={description}
                className="animate-settle max-w-xs text-balance"
              >
                {description}
              </DialogDescription>
            </DialogHeader>
            <div className="animate-rise w-full" style={rise(2)}>
              <Features ready={phase === "ready"} />
            </div>
            <div className="animate-rise w-full" style={rise(3)}>
              <div
                key={phase === "ready" ? "installing" : phase}
                className="animate-settle flex flex-col gap-4"
              >
                {phase === "installing" || phase === "ready" ? (
                  <ProgressActions
                    close={close}
                    primary={primary}
                    steps={steps}
                  />
                ) : (
                  <OfferActions
                    busy={busy}
                    close={close}
                    execute={execute}
                    failure={failure}
                    primary={primary}
                    video={video}
                  />
                )}
                {error && (
                  <p
                    role="alert"
                    className="text-destructive animate-settle text-center text-xs"
                  >
                    {error}
                  </p>
                )}
              </div>
            </div>
          </div>
          {offerTurnOff && (
            <div
              className="animate-rise flex justify-center pb-6"
              style={rise(4)}
            >
              <Button
                variant="link"
                size="xs"
                className="text-muted-foreground hover:text-foreground"
                disabled={busy || !video}
                onClick={async () => {
                  if (await execute({ kind: "turn_off" })) {
                    close();
                  }
                }}
              >
                Don&apos;t show this again
              </Button>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
};
