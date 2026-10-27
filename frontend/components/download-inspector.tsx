import { useState } from "react";
import { Copy, FolderOpen, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { Separator } from "@/components/ui/separator";
import {
  Sidebar,
  SidebarContent,
  SidebarHeader,
} from "@/components/ui/sidebar";
import {
  Item,
  ItemContent,
  ItemDescription,
  ItemGroup,
  ItemTitle,
} from "@/components/ui/item";
import { DownloadActions, reveal } from "@/components/download-actions";
import { DownloadStatus, fileIcon } from "@/components/downloads";
import { useLibrary } from "@/lib/library";
import {
  bytes,
  formatNames,
  sourceNames,
  timeRemaining,
  type Download,
  type Workspace,
} from "@/lib/model";
import { toast } from "@/components/ui/toast";
import { errorMessage } from "@/lib/bridge";

export function DownloadInspector({
  download,
  workspace,
  onClose,
  onRemove,
}: {
  download: Download;
  workspace: Workspace;
  onClose: () => void;
  onRemove: () => void;
}) {
  const { run, busy } = useLibrary();
  const [ratio, setRatio] = useState(
    download.seed_ratio_limit?.toString() ?? "",
  );
  const Icon = fileIcon(download);
  const percent = Math.min(100, download.progress_millionths / 10_000);
  const folder = workspace.folders.find(
    (folder) => folder.id === download.folder_id,
  );
  return (
    <Sidebar
      side="right"
      collapsible="none"
      className="inspector w-80 shrink-0 border-l bg-background"
      role="complementary"
      aria-label="Download details"
    >
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
      <SidebarContent className="gap-6 px-5 pb-6">
        <div className="flex flex-col items-start gap-4 pt-4">
          <div className="flex size-14 items-center justify-center rounded-xl bg-muted">
            <Icon className="size-6 text-muted-foreground" />
          </div>
          <h3 className="max-w-full break-words text-base font-medium">
            {download.name}
          </h3>
          <div className="flex w-full items-center justify-between">
            <DownloadStatus download={download} />
            <DownloadActions
              download={download}
              onInspect={() => {}}
              onRemove={onRemove}
            />
          </div>
          <Progress
            className="w-full"
            value={percent}
            aria-label="Download progress"
          />
          <p className="text-xs tabular-nums text-muted-foreground">
            {bytes(download.bytes_downloaded)} of{" "}
            {download.total_size ? bytes(download.total_size) : "unknown size"}{" "}
            · {Math.round(percent)}%
          </p>
        </div>
        {download.error_message && (
          <p
            className="rounded-lg bg-destructive/10 p-3 text-sm text-destructive"
            role="alert"
          >
            {download.error_message}
          </p>
        )}
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
          <dd className="select-text break-all" title={folder?.path}>
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
          <Button variant="outline" onClick={() => void reveal(download.id)}>
            <FolderOpen data-icon="inline-start" />
            Show in Finder
          </Button>
        )}
        <FieldGroup>
          <Field>
            <FieldLabel>Original link</FieldLabel>
            <p className="select-text break-all text-xs leading-relaxed text-muted-foreground">
              {download.source_url}
            </p>
            <Button
              variant="ghost"
              size="sm"
              className="self-start"
              onClick={() => {
                void navigator.clipboard
                  .writeText(download.source_url)
                  .then(() => toast.add({ title: "Link copied" }))
                  .catch((error) =>
                    toast.add({
                      title: "Couldn't copy link",
                      description: errorMessage(error),
                      type: "error",
                    }),
                  );
              }}
            >
              <Copy data-icon="inline-start" />
              Copy link
            </Button>
          </Field>
        </FieldGroup>
        {download.source === "Torrent" && (
          <>
            <Separator />
            <form
              onSubmit={(event) => {
                event.preventDefault();
                void run({
                  kind: "seed_ratio",
                  download_id: download.id,
                  limit: ratio.trim() ? Number(ratio) : null,
                });
              }}
            >
              <FieldGroup>
                <Field>
                  <FieldLabel htmlFor="seed-ratio">
                    Stop seeding at ratio
                  </FieldLabel>
                  <div className="flex gap-2">
                    <Input
                      id="seed-ratio"
                      type="number"
                      min="0"
                      step="0.1"
                      placeholder="Unlimited"
                      value={ratio}
                      onChange={(event) => setRatio(event.target.value)}
                      disabled={
                        busy ||
                        (!download.capabilities.can_stop_seeding &&
                          !download.capabilities.can_pause &&
                          !download.capabilities.can_resume)
                      }
                    />
                    <Button type="submit" variant="outline" disabled={busy}>
                      Save
                    </Button>
                  </div>
                  <FieldDescription>
                    Leave empty to keep seeding.
                  </FieldDescription>
                </Field>
              </FieldGroup>
            </form>
          </>
        )}
        {download.files.length > 0 && (
          <>
            <Separator />
            <section className="flex flex-col gap-3">
              <h3 className="text-sm font-medium">
                Files{" "}
                <span className="text-muted-foreground">
                  {download.files.length}
                </span>
              </h3>
              <ItemGroup className="gap-1">
                {download.files.map((file) => (
                  <Item key={file.index} size="xs" className="px-0">
                    <Checkbox
                      aria-label={`Download ${file.path}`}
                      checked={file.priority !== "skip"}
                      disabled={
                        busy || !download.capabilities.can_change_file_selection
                      }
                      onCheckedChange={(checked) =>
                        void run({
                          kind: "torrent_files",
                          download_id: download.id,
                          files: download.files.map((current) =>
                            current.index === file.index
                              ? {
                                  ...current,
                                  priority: checked ? "normal" : "skip",
                                }
                              : current,
                          ),
                        })
                      }
                    />
                    <ItemContent className="min-w-0">
                      <ItemTitle
                        className="max-w-full truncate text-xs"
                        title={file.path}
                      >
                        {file.path}
                      </ItemTitle>
                      <ItemDescription className="text-xs">
                        {bytes(file.size)}
                      </ItemDescription>
                    </ItemContent>
                  </Item>
                ))}
              </ItemGroup>
            </section>
          </>
        )}
        {download.trackers.length > 0 && (
          <>
            <Separator />
            <section className="flex flex-col gap-2">
              <h3 className="text-sm font-medium">Trackers</h3>
              {download.trackers.map((tracker) => (
                <p
                  key={tracker}
                  className="select-text break-all text-xs text-muted-foreground"
                >
                  {tracker}
                </p>
              ))}
            </section>
          </>
        )}
      </SidebarContent>
    </Sidebar>
  );
}
