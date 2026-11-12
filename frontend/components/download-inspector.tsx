import {
  ArrowUpRightIcon,
  Check,
  Copy,
  FileIcon,
  FolderOpen,
  GlobeIcon,
  Link,
  Magnet,
  X,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { DownloadActions } from "@/components/download-actions";
import { DownloadFolderIcon } from "@/components/download-folder-icon";
import { reveal } from "@/components/download-menu";
import { DownloadStatus } from "@/components/downloads";
import { InspectorPanel } from "@/components/inspector-panel";
import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { Separator } from "@/components/ui/separator";
import { SidebarContent, SidebarHeader } from "@/components/ui/sidebar";
import { toast } from "@/components/ui/toast";
import {
  Trace,
  TraceCollapsible,
  TraceCollapsibleContent,
  TraceCollapsibleTrigger,
  TraceDescription,
  TraceIcon,
  TraceLine,
  TraceTitle,
} from "@/components/ui/trace";
import { VideoToolsUpdateAdvice } from "@/components/video-tools-update-advice";
import { shortError } from "@/lib/bridge";
import { useLibrary } from "@/lib/library";
import { bytes, formatNames, sourceNames, timeRemaining } from "@/lib/model";
import type { Download, Workspace } from "@/lib/model";
import { truncateError, truncateFilename, truncateUrl } from "@/lib/truncate";

const noop = () => null;

const CopyError = ({ text }: { text: string }) => {
  const [copied, setCopied] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    },
    []
  );

  return (
    <Button
      type="button"
      variant="ghost"
      size="icon-sm"
      className="shrink-0"
      aria-label={copied ? "Copied error" : "Copy error"}
      onClick={async () => {
        try {
          await navigator.clipboard.writeText(text);
          setCopied(true);

          if (timeoutRef.current) {
            clearTimeout(timeoutRef.current);
          }

          timeoutRef.current = setTimeout(() => {
            setCopied(false);
          }, 2000);
        } catch (error) {
          toast.add({
            description: shortError(error),
            title: "Couldn't copy error",
            type: "error",
          });
        }
      }}
    >
      {copied ? <Check /> : <Copy />}
    </Button>
  );
};

export const DownloadSourceLink = ({ download }: { download: Download }) => {
  const [copied, setCopied] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    },
    []
  );

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(download.source_url);
      setCopied(true);

      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }

      timeoutRef.current = setTimeout(() => {
        setCopied(false);
      }, 2000);
    } catch (error) {
      toast.add({
        description: shortError(error),
        title: "Couldn't copy link",
        type: "error",
      });
    }
  };

  if (!download.source_url) {
    return null;
  }

  const isTorrent = download.source === "Torrent";
  const sourceLabel = truncateUrl(download.source_url);

  return (
    <Trace>
      <button
        type="button"
        className="group/trace"
        onClick={handleCopy}
        aria-label={copied ? "Copied to clipboard" : download.source_url}
      >
        <TraceLine>
          <TraceIcon>{isTorrent ? <Magnet /> : <Link />}</TraceIcon>
          <TraceTitle
            title={copied ? "Copied to clipboard" : download.source_url}
          >
            {copied ? "Copied to clipboard" : sourceLabel}
          </TraceTitle>
          <TraceIcon
            className={
              copied ? "text-foreground" : "invisible group-hover/trace:visible"
            }
          >
            {copied ? <Check /> : <Copy />}
          </TraceIcon>
        </TraceLine>
      </button>
    </Trace>
  );
};

/** "Stop seeding at ratio": empty seeds until stopped by hand. */
const SeedRatioField = ({ download }: { download: Download }) => {
  const { run, busy } = useLibrary();
  const caps = download.capabilities;
  const saved = download.seed_ratio_limit?.toString() ?? "";
  const [ratio, setRatio] = useState(saved);
  const [seen, setSeen] = useState(saved);

  // Rust can change the limit too: Start seeding after the limit was reached clears it.
  if (seen !== saved) {
    setSeen(saved);
    setRatio(saved);
  }

  const editable =
    caps.can_stop_seeding ||
    caps.can_start_seeding ||
    caps.can_pause ||
    caps.can_resume;

  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();

        run({
          download_id: download.id,
          kind: "seed_ratio",
          limit: ratio.trim() ? Number(ratio) : null,
        }).catch(() => null);
      }}
    >
      <FieldGroup>
        <Field>
          <FieldLabel htmlFor="seed-ratio">Stop seeding at ratio</FieldLabel>
          <div className="flex gap-2">
            <Input
              id="seed-ratio"
              type="number"
              min="0"
              step="0.1"
              placeholder="Unlimited"
              value={ratio}
              onChange={(event) => setRatio(event.target.value)}
              disabled={busy || !editable}
            />
            <Button type="submit" variant="outline" disabled={busy}>
              Save
            </Button>
          </div>
          <FieldDescription>Leave empty to keep seeding.</FieldDescription>
        </Field>
      </FieldGroup>
    </form>
  );
};

export const DownloadInspector = ({
  download,
  workspace,
  onClose,
  onRemove,
}: {
  download: Download;
  workspace: Workspace;
  onClose: () => void;
  onRemove: () => void;
}) => {
  const percent = Math.min(100, download.progress_millionths / 10_000);

  const folder = workspace.folders.find(
    (item) => item.id === download.folder_id
  );

  return (
    <InspectorPanel onClose={onClose}>
      <SidebarHeader className="h-14 flex-row items-center justify-between px-4">
        <h2 className="text-sm font-medium">Download details</h2>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Close details"
          onClick={onClose}
        >
          <X />
        </Button>
      </SidebarHeader>
      <SidebarContent className="scroll-fade gap-6 px-5 pb-6">
        <div className="flex flex-col items-start gap-4 pt-4">
          <DownloadFolderIcon download={download} className="size-12" />
          <h3
            className="max-w-full text-base font-medium wrap-break-word"
            title={download.name}
          >
            {truncateFilename(download.name, 72)}
          </h3>
          <div className="flex w-full items-center justify-between">
            <DownloadStatus download={download} />
            <DownloadActions
              download={download}
              onInspect={noop}
              onRemove={onRemove}
            />
          </div>
          <Progress
            className="w-full"
            value={percent}
            aria-label="Download progress"
          />
          <p className="text-muted-foreground text-xs tabular-nums">
            {bytes(download.bytes_downloaded)} of{" "}
            {download.total_size ? bytes(download.total_size) : "unknown size"}{" "}
            · {Math.round(percent)}%
          </p>
        </div>
        {download.error_message && (
          <div className="flex flex-col gap-2">
            <Alert variant="destructive" title={download.error_message}>
              {truncateError(download.error_message)}
            </Alert>
            <div className="bg-muted/40 flex items-start gap-2 rounded-lg border px-3 py-2">
              <pre className="text-muted-foreground scroll-fade max-h-32 min-w-0 flex-1 overflow-auto font-mono text-xs whitespace-pre-wrap select-text">
                {download.error_message}
              </pre>
              <CopyError text={download.error_message} />
            </div>
          </div>
        )}
        <VideoToolsUpdateAdvice download={download} />
        <Separator />
        <dl className="details-grid text-xs">
          <dt>Workspace</dt>
          <dd>{workspace.name}</dd>
          <dt>Source</dt>
          <dd>{sourceNames[download.source]}</dd>
          {download.format && (
            <>
              <dt>Format</dt>
              <dd>{formatNames[download.format]}</dd>
            </>
          )}
          <dt>Save location</dt>
          <dd className="break-all select-text" title={folder?.path}>
            {folder?.name ?? "Unknown"}
          </dd>
          {download.download_rate > 0 && (
            <>
              <dt>Download speed</dt>
              <dd>{bytes(download.download_rate)}/s</dd>
              <dt>Time remaining</dt>
              <dd>{timeRemaining(download.eta_seconds)}</dd>
            </>
          )}
          {download.source === "Torrent" && (
            <>
              <dt>Uploaded</dt>
              <dd>{bytes(download.bytes_uploaded)}</dd>
              <dt>Upload speed</dt>
              <dd>{bytes(download.upload_rate)}/s</dd>
              <dt>Share ratio</dt>
              <dd>{download.seed_ratio.toFixed(2)}</dd>
              <dt>Peers / seeds</dt>
              <dd>
                {download.peers} / {download.seeds}
              </dd>
            </>
          )}
        </dl>
        {download.capabilities.can_reveal_output && (
          <Button
            variant="outline"
            onClick={() => {
              reveal(download.id).catch(() => null);
            }}
          >
            <FolderOpen data-icon="inline-start" />
            Show in Finder
          </Button>
        )}
        <DownloadSourceLink key={download.id} download={download} />
        {download.source === "Torrent" && (
          <>
            <Separator />
            <SeedRatioField download={download} />
          </>
        )}
        {download.files.length > 0 && (
          <>
            <Separator />
            {download.files.length === 1 ? (
              <Trace>
                <TraceLine>
                  <TraceIcon>
                    <FileIcon />
                  </TraceIcon>
                  <TraceTitle title={download.files[0].path}>
                    {truncateFilename(download.files[0].path)}
                  </TraceTitle>
                  <TraceDescription>
                    {bytes(download.files[0].size)}
                  </TraceDescription>
                </TraceLine>
              </Trace>
            ) : (
              <TraceCollapsible>
                <TraceCollapsibleTrigger>
                  <TraceLine>
                    <TraceIcon>
                      <FileIcon />
                    </TraceIcon>
                    <TraceTitle>{`${download.files.length} files`}</TraceTitle>
                  </TraceLine>
                </TraceCollapsibleTrigger>
                <TraceCollapsibleContent>
                  <Trace>
                    {download.files.map((file) => (
                      <TraceLine key={file.index}>
                        <TraceTitle title={file.path}>
                          {truncateFilename(file.path)}
                        </TraceTitle>
                        <TraceDescription>{bytes(file.size)}</TraceDescription>
                      </TraceLine>
                    ))}
                  </Trace>
                </TraceCollapsibleContent>
              </TraceCollapsible>
            )}
          </>
        )}
        {download.trackers.length > 0 &&
          (download.trackers.length === 1 ? (
            <Trace>
              <a
                href={download.trackers[0]}
                aria-label={download.trackers[0]}
                target="_blank"
                rel="noopener noreferrer"
              >
                <TraceLine>
                  <TraceIcon>
                    <GlobeIcon />
                  </TraceIcon>
                  <TraceTitle title={download.trackers[0]}>
                    {truncateUrl(download.trackers[0])}
                    <span className="sr-only"> (opens in a new tab)</span>
                  </TraceTitle>
                  <TraceIcon>
                    <ArrowUpRightIcon />
                  </TraceIcon>
                </TraceLine>
              </a>
            </Trace>
          ) : (
            <TraceCollapsible>
              <TraceCollapsibleTrigger>
                <TraceLine>
                  <TraceIcon>
                    <GlobeIcon />
                  </TraceIcon>
                  <TraceTitle>{`${download.trackers.length} trackers`}</TraceTitle>
                </TraceLine>
              </TraceCollapsibleTrigger>
              <TraceCollapsibleContent>
                <Trace>
                  {download.trackers.map((tracker) => (
                    <a
                      key={tracker}
                      href={tracker}
                      aria-label={tracker}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      <TraceLine>
                        <TraceTitle title={tracker}>
                          {truncateUrl(tracker)}
                          <span className="sr-only"> (opens in a new tab)</span>
                        </TraceTitle>
                        <TraceIcon>
                          <ArrowUpRightIcon />
                        </TraceIcon>
                      </TraceLine>
                    </a>
                  ))}
                </Trace>
              </TraceCollapsibleContent>
            </TraceCollapsible>
          ))}
      </SidebarContent>
    </InspectorPanel>
  );
};
