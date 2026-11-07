import {
  Check,
  Download,
  FileDown,
  ListVideo,
  Magnet,
  Music,
} from "lucide-react";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Trace, TraceIcon, TraceLine, TraceTitle } from "@/components/ui/trace";
import { VideoToolsProgress } from "@/components/video-tools-progress";
import type { useVideoSetup } from "@/hooks/use-video-setup";
import { downloadSize } from "@/lib/video-tools";
import type { SetupStep } from "@/lib/video-tools";

const features = [
  { icon: FileDown, label: "Direct files", video: false },
  { icon: Magnet, label: "Torrents", video: false },
  { icon: ListVideo, label: "Video pages and playlists", video: true },
  { icon: Music, label: "Audio from video pages", video: true },
];

const finishedSteps: SetupStep[] = ["python", "yt_dlp"].map((component) => ({
  component: component === "python" ? "python" : "yt_dlp",
  downloaded_bytes: 0,
  status: "done",
  total_bytes: null,
}));

export const VideoSetup = ({
  setup,
}: {
  setup: ReturnType<typeof useVideoSetup>;
}) => {
  const { busy, close, error, execute, open, tools } = setup;
  const video = tools?.video;
  const state = video?.state;
  const installing = state?.kind === "setting_up";
  const ready = state?.kind === "ready";
  const failed = state?.kind === "not_working";

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) {
          close();
        }
      }}
    >
      <DialogContent className="inset-0 flex h-svh max-w-none translate-x-0 translate-y-0 flex-col overflow-y-auto rounded-none p-8 sm:max-w-none">
        <div className="m-auto flex w-full max-w-md flex-col gap-8 py-6">
          <DialogHeader>
            <DialogTitle>Set Up Video Tools</DialogTitle>
            <DialogDescription>
              {installing
                ? "Setup can finish while you use Opul."
                : "Add video and audio downloads to Opul. Direct files and torrents work on their own."}
            </DialogDescription>
          </DialogHeader>
          {installing || ready ? (
            <>
              <VideoToolsProgress
                steps={installing ? state.steps : finishedSteps}
              />
              {ready && (
                <p className="flex items-center gap-2">
                  <Check aria-hidden="true" />
                  Video tools are ready
                </p>
              )}
              <Button onClick={close}>Continue to Opul</Button>
            </>
          ) : (
            <>
              <Trace aria-label="Opul features">
                {features.map(({ icon: Icon, label, video: needsVideo }) => (
                  <TraceLine
                    key={label}
                    status={needsVideo ? undefined : "available"}
                  >
                    <TraceIcon>
                      <Icon />
                    </TraceIcon>
                    <TraceTitle data-unavailable={needsVideo}>
                      {label}
                    </TraceTitle>
                    {needsVideo && (
                      <span className="text-muted-foreground text-xs">
                        Needs video tools
                      </span>
                    )}
                  </TraceLine>
                ))}
              </Trace>
              {failed && (
                <Alert variant="destructive">
                  <AlertDescription>
                    Video tools aren&apos;t working. {state.details}
                  </AlertDescription>
                </Alert>
              )}
              {video && (
                <FieldGroup>
                  <Field orientation="horizontal">
                    <Checkbox
                      id="setup-keep-up-to-date"
                      checked={video.keep_up_to_date}
                      disabled={busy}
                      onCheckedChange={(enabled) =>
                        execute({ enabled, kind: "set_keep_up_to_date" })
                      }
                    />
                    <FieldLabel htmlFor="setup-keep-up-to-date">
                      Keep video tools up to date
                    </FieldLabel>
                  </Field>
                </FieldGroup>
              )}
              <div className="flex flex-col gap-3">
                <Button
                  disabled={busy || !video}
                  onClick={() => execute({ kind: failed ? "retry" : "set_up" })}
                >
                  <Download data-icon="inline-start" />
                  {failed
                    ? "Try again"
                    : `Download video tools (${downloadSize(video?.published_size ?? null)})`}
                </Button>
                <Button variant="ghost" onClick={close}>
                  Not now
                </Button>
                <Button
                  variant="link"
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
            </>
          )}
          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
};
