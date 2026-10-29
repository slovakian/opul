import { ChevronDown, FileUp } from "lucide-react";
import { useState } from "react";

import { Choice } from "@/components/sidebar-popover";
import { Button } from "@/components/ui/button";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { InputGroupButton } from "@/components/ui/input-group";
import {
  Popover,
  PopoverContent,
  PopoverTitle,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { downloadTypes } from "@/lib/download-options";
import type { DownloadType } from "@/lib/download-options";
import { useLibrary } from "@/lib/library";
import { formatNames, formatSchema } from "@/lib/model";
import type { Format, Workspace } from "@/lib/model";

export const DownloadOptions = ({
  workspace,
  type,
  format,
  onType,
  onFormat,
  onTorrent,
  disabled,
}: {
  workspace: Workspace | undefined;
  type: DownloadType;
  format: Format;
  onType: (type: DownloadType) => void;
  onFormat: (format: Format | undefined) => void;
  onTorrent: () => Promise<void>;
  disabled: boolean;
}) => {
  const { run } = useLibrary();
  const [open, setOpen] = useState(false);
  const active = type !== "Automatic" || format !== workspace?.default_format;

  const title = active
    ? `${downloadTypes.find((item) => item.value === type)?.label}; ${formatNames[format]}`
    : "Download options";

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <Tooltip>
        <TooltipTrigger
          render={
            <PopoverTrigger
              render={
                <InputGroupButton
                  type="button"
                  size="icon-sm"
                  variant={active ? "secondary" : "ghost"}
                  aria-label="Download options"
                  title={title}
                  data-active={active}
                  disabled={disabled}
                  className="rounded-l-none border-l"
                />
              }
            >
              <ChevronDown />
            </PopoverTrigger>
          }
        />
        <TooltipContent>{title}</TooltipContent>
      </Tooltip>
      <PopoverContent align="end">
        <PopoverTitle>Download options</PopoverTitle>
        <FieldGroup>
          <Field>
            <FieldLabel>Download type</FieldLabel>
            <Choice
              label="Download type"
              value={type}
              options={downloadTypes}
              onChange={onType}
              disabled={disabled}
            />
          </Field>
          <Field>
            <FieldLabel>Video format</FieldLabel>
            <Choice
              label="Video format"
              value={format}
              options={[
                { label: "Workspace default", value: "Default" },
                ...formatSchema.options.map((value) => ({
                  label: formatNames[value],
                  value,
                })),
              ]}
              onChange={(value) =>
                onFormat(value === "Default" ? undefined : value)
              }
              disabled={disabled}
            />
          </Field>
        </FieldGroup>
        <Button
          type="button"
          variant="outline"
          disabled={
            disabled || !workspace || format === workspace.default_format
          }
          onClick={async () => {
            if (
              workspace &&
              (await run({
                format,
                kind: "format",
                workspace_id: workspace.id,
              }))
            ) {
              onFormat(undefined);
            }
          }}
        >
          Set as workspace default
        </Button>
        <Button
          type="button"
          variant="ghost"
          disabled={disabled}
          onClick={async () => {
            setOpen(false);
            await onTorrent();
          }}
        >
          <FileUp />
          Open .torrent
        </Button>
      </PopoverContent>
    </Popover>
  );
};
