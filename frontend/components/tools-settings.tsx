import { useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { VideoToolsProgress } from "@/components/video-tools-progress";
import { useVideoTools } from "@/hooks/use-video-tools";
import { bridge, errorMessage } from "@/lib/bridge";
import type { VideoToolsSnapshot } from "@/lib/video-tools";
import { downloadSize, updateMessage } from "@/lib/video-tools";

const failureMessages = {
  download: "The download couldn't finish.",
  health_check: "The video tools didn't pass their health check.",
  stopped_working:
    "The video tools stopped working. Files may be missing or blocked by macOS.",
  verification: "The download couldn't be verified.",
};

const ReadyVideoToolsSettings = ({
  state,
  busy,
  execute,
}: {
  state: Extract<VideoToolsSnapshot["state"], { kind: "ready" }>;
  busy: boolean;
  execute: ReturnType<typeof useVideoTools>["execute"];
}) => (
  <>
    <output className="text-sm">Ready</output>
    <p className="text-muted-foreground text-sm">
      Python runtime {state.installed.runtime} · Python{" "}
      {state.installed.python_version}
      <br />
      yt-dlp {state.installed.yt_dlp}
    </p>
    {state.last_checked !== null && (
      <p className="text-muted-foreground text-xs">
        Last checked {new Date(state.last_checked * 1000).toLocaleString()}
      </p>
    )}
    <Button
      type="button"
      variant="outline"
      disabled={busy || state.update_flag.kind === "updating"}
      onClick={() => execute({ kind: "check_for_updates" })}
    >
      Check for updates
    </Button>
    {updateMessage(state.update_flag) && (
      <div className="flex flex-wrap items-center gap-2">
        <output className="text-muted-foreground text-sm">
          {updateMessage(state.update_flag)}
        </output>
        {state.update_flag.kind === "available" && (
          <Button
            type="button"
            size="sm"
            disabled={busy}
            onClick={() => execute({ kind: "update" })}
          >
            Update
          </Button>
        )}
      </div>
    )}
  </>
);

export const ToolsSettings = ({
  onVideoSetup,
}: {
  onVideoSetup: () => void;
}) => {
  const { busy, error, execute, tools } = useVideoTools();
  const [copyError, setCopyError] = useState("");
  const [copied, setCopied] = useState(false);

  const copyDetails = async () => {
    try {
      await navigator.clipboard.writeText(await bridge.videoToolsDetails());
      setCopied(true);
    } catch (error_) {
      setCopyError(errorMessage(error_));
    }
  };

  if (!tools) {
    return (
      <div className="flex flex-col gap-4">
        <output>{error || "Loading tools…"}</output>
        {!error && <Skeleton className="h-20 w-full" />}
      </div>
    );
  }

  const { video, built_in: builtIn } = tools;
  const { state } = video;
  const initial = state.kind === "not_set_up" || state.kind === "turned_off";

  return (
    <>
      <p className="text-muted-foreground text-sm">
        Direct files and torrents work on their own.
      </p>
      <FieldGroup>
        <Field>
          <FieldLabel>FFmpeg</FieldLabel>
          <FieldDescription>
            Built in · {builtIn.ffmpeg ?? "Unavailable"} · with ffprobe{" "}
            {builtIn.ffprobe ?? "unavailable"}
          </FieldDescription>
        </Field>
      </FieldGroup>
      <Separator />
      <section aria-label="Video tools" className="flex flex-col gap-4">
        <h3 className="font-medium">Video tools</h3>
        <Button type="button" variant="outline" onClick={onVideoSetup}>
          Set Up Video Tools…
        </Button>
        {initial && (
          <>
            <p className="text-muted-foreground text-sm">
              {state.kind === "turned_off"
                ? "Video tools are turned off. Opul won't offer them."
                : "Download videos and audio from video pages."}
            </p>
            <Button
              type="button"
              disabled={busy}
              onClick={() => execute({ kind: "set_up" })}
            >
              Download video tools ({downloadSize(video.published_size)})
            </Button>
          </>
        )}
        {state.kind === "setting_up" && (
          <>
            <VideoToolsProgress steps={state.steps} />
            <Button
              type="button"
              variant="outline"
              disabled={busy}
              onClick={() => execute({ kind: "cancel_setup" })}
            >
              Cancel setup
            </Button>
          </>
        )}
        {state.kind === "ready" && (
          <ReadyVideoToolsSettings
            state={state}
            busy={busy}
            execute={execute}
          />
        )}
        {state.kind === "not_working" && (
          <>
            <output className="text-sm">{failureMessages[state.reason]}</output>
            <details className="text-muted-foreground text-xs">
              <summary>Details</summary>
              <pre className="mt-2 max-h-40 overflow-auto whitespace-pre-wrap">
                {state.details}
              </pre>
            </details>
            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                disabled={busy}
                onClick={() => execute({ kind: "retry" })}
              >
                Try again
              </Button>
              <Button type="button" variant="outline" onClick={copyDetails}>
                {copied ? "Copied" : "Copy details"}
              </Button>
            </div>
          </>
        )}
        <FieldGroup>
          <Field orientation="horizontal">
            <FieldContent>
              <FieldLabel htmlFor="keep-video-tools-up-to-date">
                Keep video tools up to date
              </FieldLabel>
              <FieldDescription>
                Changes take effect immediately.
              </FieldDescription>
            </FieldContent>
            <Switch
              id="keep-video-tools-up-to-date"
              checked={video.keep_up_to_date}
              disabled={busy}
              onCheckedChange={(enabled) =>
                execute({ enabled, kind: "set_keep_up_to_date" })
              }
            />
          </Field>
        </FieldGroup>
        {state.kind === "not_set_up" && (
          <Button
            type="button"
            variant="link"
            disabled={busy}
            onClick={() => execute({ kind: "turn_off" })}
          >
            Don&apos;t offer video tools again
          </Button>
        )}
        {(error || copyError) && (
          <p role="alert" className="text-destructive text-sm">
            {error || copyError}
          </p>
        )}
      </section>
      <p className="text-muted-foreground text-xs">
        Video downloads use{" "}
        <a
          className="underline"
          href="https://github.com/yt-dlp/yt-dlp"
          target="_blank"
          rel="noopener noreferrer"
        >
          yt-dlp
        </a>
        , a free and open source project.{" "}
        <a
          className="underline"
          href="https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md"
          target="_blank"
          rel="noopener noreferrer"
        >
          Supported sites
        </a>
      </p>
    </>
  );
};
