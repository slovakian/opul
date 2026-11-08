import {
  Download,
  Info,
  Monitor,
  Network,
  RefreshCw,
  Settings2,
  Wrench,
} from "lucide-react";
import { useState } from "react";

import opulLogo from "@/assets/opul-logo.png";
import { Choice } from "@/components/sidebar-popover";
import { ToolsSettings } from "@/components/tools-settings";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import { Switch } from "@/components/ui/switch";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { UpdatesSettings } from "@/components/updates-settings";
import { useAppUpdates } from "@/hooks/use-app-updates";
import { useVideoTools } from "@/hooks/use-video-tools";
import { appName, needsAttention } from "@/lib/app-updates";
import { isActiveDownload } from "@/lib/download-activity";
import { useLibrary } from "@/lib/library";
import type { Preferences } from "@/lib/model";

const sections = [
  { icon: Monitor, label: "Appearance" },
  { icon: Download, label: "Downloads" },
  { icon: Network, label: "BitTorrent" },
  { icon: Settings2, label: "Workspaces" },
  { icon: Wrench, label: "Tools" },
  { icon: RefreshCw, label: "Updates" },
  { icon: Info, label: "About" },
] as const;

export type SettingsSection = (typeof sections)[number]["label"];

/** These sections apply each change at once, so they close with Done instead of Save. */
const immediateSections = new Set<SettingsSection>(["Tools", "Updates"]);

const WorkspaceSettings = ({
  draft,
  setDraft,
}: {
  draft: Preferences;
  setDraft: (draft: Preferences) => void;
}) => (
  <>
    <p className="text-muted-foreground text-sm">
      Choose how saved-folder branches appear when Opul starts.
    </p>
    <FieldGroup>
      <Field>
        <FieldLabel>Saved folders on startup</FieldLabel>
        <Choice
          label="Saved folders on startup"
          value={draft.workspace_expansion_startup}
          options={[
            { label: "Remember previous state", value: "remember" },
            { label: "Start collapsed", value: "collapsed" },
          ]}
          onChange={(value) => {
            if (value === "remember" || value === "collapsed") {
              setDraft({ ...draft, workspace_expansion_startup: value });
            }
          }}
        />
        <FieldDescription>
          This setting applies the next time Opul launches. You can still expand
          and collapse folders during this session.
        </FieldDescription>
      </Field>
    </FieldGroup>
  </>
);

// oxlint-disable-next-line complexity -- Settings intentionally keeps all sections in one draft form.
const SettingsContent = ({
  initial,
  initialSection,
  onClose,
  onVideoSetup,
}: {
  initial: Preferences;
  initialSection: SettingsSection;
  onClose: () => void;
  onVideoSetup: () => void;
}) => {
  const { savePreferences, state } = useLibrary();
  const { tools } = useVideoTools();
  const { updates } = useAppUpdates();

  const marked: Partial<Record<SettingsSection, boolean>> = {
    Tools: tools?.video.update_available,
    Updates: needsAttention(updates),
  };

  const workspaces = state.kind === "ready" ? state.snapshot.workspaces : [];

  const activeDownloads = workspaces
    .flatMap((workspace) => workspace.downloads)
    .filter(isActiveDownload).length;

  const [section, setSection] = useState<SettingsSection>(initialSection);
  const [draft, setDraft] = useState(initial);
  const [saving, setSaving] = useState(false);

  const save = async (event: React.FormEvent) => {
    event.preventDefault();
    setSaving(true);

    if (await savePreferences(draft)) {
      onClose();
    }

    setSaving(false);
  };

  const numberField = (
    key:
      | "max_download_kbps"
      | "max_upload_kbps"
      | "listen_port"
      | "max_connections",
    label: string,
    min = 0,
    max = 1_000_000
  ) => (
    <Field>
      <FieldLabel htmlFor={`setting-${key}`}>{label}</FieldLabel>
      <Input
        id={`setting-${key}`}
        type="number"
        min={min}
        max={max}
        required
        value={draft[key]}
        onChange={(event) =>
          setDraft({ ...draft, [key]: event.target.valueAsNumber || 0 })
        }
      />
    </Field>
  );

  return (
    <form onSubmit={save} className="flex h-[32rem] max-h-[85svh] min-w-0">
      <Sidebar
        collapsible="none"
        className="border-sidebar-border w-40 shrink-0 border-r sm:w-48"
      >
        <SidebarContent>
          <SidebarGroup className="pt-5">
            <SidebarGroupLabel>Settings</SidebarGroupLabel>
            <SidebarMenu>
              {sections.map((item) => (
                <SidebarMenuItem key={item.label}>
                  <SidebarMenuButton
                    type="button"
                    isActive={section === item.label}
                    onClick={() => setSection(item.label)}
                  >
                    <item.icon />
                    <span>{item.label}</span>
                  </SidebarMenuButton>
                  {marked[item.label] && (
                    <SidebarMenuBadge className="animate-pop">
                      <span
                        aria-hidden="true"
                        className="bg-primary size-1.5 rounded-full"
                      />
                      <span className="sr-only">Update available</span>
                    </SidebarMenuBadge>
                  )}
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroup>
        </SidebarContent>
      </Sidebar>
      <main className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-16 shrink-0 items-center px-6">
          <h2 className="font-medium">{section}</h2>
        </header>
        <div
          key={section}
          className="scroll-fade min-h-0 flex-1 overflow-y-auto px-6 pt-1 pb-6"
        >
          <div className="animate-settle flex flex-col gap-6">
            {section === "Appearance" && (
              <>
                <p className="text-muted-foreground text-sm">
                  Make yourself at home.
                </p>
                <FieldGroup>
                  <Field>
                    <FieldLabel>Theme</FieldLabel>
                    <ToggleGroup
                      aria-label="Color theme"
                      value={[draft.theme]}
                      onValueChange={(values) => {
                        const [theme] = values;

                        if (
                          theme === "system" ||
                          theme === "light" ||
                          theme === "dark"
                        ) {
                          setDraft({ ...draft, theme });
                        }
                      }}
                    >
                      <ToggleGroupItem value="system">System</ToggleGroupItem>
                      <ToggleGroupItem value="light">Light</ToggleGroupItem>
                      <ToggleGroupItem value="dark">Dark</ToggleGroupItem>
                    </ToggleGroup>
                  </Field>
                </FieldGroup>
              </>
            )}
            {section === "Downloads" && (
              <>
                <p className="text-muted-foreground text-sm">
                  Limits apply as soon as you save.
                </p>
                <FieldGroup>
                  <Field>
                    <FieldLabel>Destination in aggregate views</FieldLabel>
                    <Choice
                      label="Aggregate destination"
                      value={draft.aggregate_destination.kind}
                      options={[
                        { label: "Ask each time", value: "prompt" },
                        {
                          label: "Use a default workspace",
                          value: "workspace",
                        },
                      ]}
                      onChange={(kind) => {
                        if (kind === "prompt") {
                          setDraft({
                            ...draft,
                            aggregate_destination: { kind: "prompt" },
                          });
                        } else if (workspaces[0]) {
                          setDraft({
                            ...draft,
                            aggregate_destination: {
                              kind: "workspace",
                              workspace_id: workspaces[0].id,
                            },
                          });
                        }
                      }}
                    />
                    <FieldDescription>
                      Applies to All downloads, In progress, and Completed when
                      you have multiple workspaces.
                    </FieldDescription>
                  </Field>
                  {draft.aggregate_destination.kind === "workspace" && (
                    <Field>
                      <FieldLabel>Default workspace</FieldLabel>
                      <Choice
                        label="Default workspace"
                        value={draft.aggregate_destination.workspace_id}
                        options={workspaces.map((workspace) => ({
                          label: workspace.name,
                          value: workspace.id,
                        }))}
                        onChange={(workspace_id) =>
                          setDraft({
                            ...draft,
                            aggregate_destination: {
                              kind: "workspace",
                              workspace_id,
                            },
                          })
                        }
                      />
                      {!workspaces.some(
                        (workspace) =>
                          draft.aggregate_destination.kind === "workspace" &&
                          workspace.id ===
                            draft.aggregate_destination.workspace_id
                      ) && (
                        <FieldDescription>
                          Choose a workspace. The previous default is
                          unavailable; downloads will ask for a destination.
                        </FieldDescription>
                      )}
                    </Field>
                  )}
                  <Field>
                    <FieldLabel>Concurrent media downloads</FieldLabel>
                    <Choice
                      label="Concurrent downloads"
                      value={String(draft.concurrent_downloads)}
                      options={["1", "2", "3", "4", "5", "8"].map((value) => ({
                        label: `${value} at a time`,
                        value,
                      }))}
                      onChange={(value) =>
                        setDraft({
                          ...draft,
                          concurrent_downloads: Number(value),
                        })
                      }
                    />
                    <FieldDescription>
                      Applies to video pages and direct file downloads.
                    </FieldDescription>
                  </Field>
                  {numberField(
                    "max_download_kbps",
                    "Max download speed (KB/s, 0 for unlimited)"
                  )}
                  {numberField(
                    "max_upload_kbps",
                    "Max upload speed (KB/s, 0 for unlimited)"
                  )}
                </FieldGroup>
              </>
            )}
            {section === "BitTorrent" && (
              <>
                <p className="text-muted-foreground text-sm">
                  Connection changes take effect when you restart Opul.
                </p>
                <FieldGroup>
                  <Field orientation="horizontal">
                    <FieldContent>
                      <FieldLabel htmlFor="dht">Peer discovery</FieldLabel>
                      <FieldDescription>
                        Find peers using the DHT network.
                      </FieldDescription>
                    </FieldContent>
                    <Switch
                      id="dht"
                      checked={draft.enable_dht}
                      onCheckedChange={(checked) =>
                        setDraft({ ...draft, enable_dht: checked })
                      }
                    />
                  </Field>
                  <Field orientation="horizontal">
                    <FieldContent>
                      <FieldLabel htmlFor="random-port">
                        Randomize listen port
                      </FieldLabel>
                    </FieldContent>
                    <Switch
                      id="random-port"
                      checked={draft.randomize_port}
                      onCheckedChange={(checked) =>
                        setDraft({ ...draft, randomize_port: checked })
                      }
                    />
                  </Field>
                  {!draft.randomize_port &&
                    numberField("listen_port", "Listen port", 1, 65_535)}
                  {numberField(
                    "max_connections",
                    "Maximum connections",
                    1,
                    10_000
                  )}
                </FieldGroup>
              </>
            )}
            {section === "Workspaces" && (
              <WorkspaceSettings draft={draft} setDraft={setDraft} />
            )}
            {section === "Tools" && (
              <ToolsSettings onVideoSetup={onVideoSetup} />
            )}
            {section === "Updates" && (
              <UpdatesSettings activeDownloads={activeDownloads} />
            )}
            {section === "About" && (
              <div className="flex flex-col items-start gap-4 py-4">
                <img src={opulLogo} alt="" className="size-12" />
                <div>
                  <h3 className="text-lg font-semibold">Opul</h3>
                  <p className="text-muted-foreground text-sm">
                    A quiet home for your downloads.
                  </p>
                </div>
                <Separator />
                <p className="text-muted-foreground text-sm">
                  Built with Tauri, Base UI, and the Opul Rust core. Free and
                  open source under the MIT license.
                </p>
                {updates && (
                  <p className="text-muted-foreground text-xs tabular-nums">
                    {`${appName(updates)} ${updates.current_version}`}
                  </p>
                )}
              </div>
            )}
          </div>
        </div>
        <footer className="flex justify-end gap-2 px-6 py-4">
          {immediateSections.has(section) ? (
            <Button type="button" variant="ghost" onClick={onClose}>
              Done
            </Button>
          ) : (
            <>
              <Button type="button" variant="ghost" onClick={onClose}>
                Cancel
              </Button>
              <Button type="submit" disabled={saving}>
                {saving ? "Saving…" : "Save changes"}
              </Button>
            </>
          )}
        </footer>
      </main>
    </form>
  );
};

export const SettingsDialog = ({
  onClose,
  onVideoSetup,
  section = "Appearance",
}: {
  onClose: () => void;
  onVideoSetup: () => void;
  section?: SettingsSection;
}) => {
  const { preferences } = useLibrary();

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
    >
      <DialogContent className="overflow-hidden p-0 sm:max-w-3xl">
        <DialogTitle className="sr-only">Settings</DialogTitle>
        <DialogDescription className="sr-only">
          Customize Opul&apos;s appearance, downloads, and connections.
        </DialogDescription>
        {preferences ? (
          <SettingsContent
            initial={preferences}
            initialSection={section}
            onClose={onClose}
            onVideoSetup={onVideoSetup}
          />
        ) : (
          <p className="p-8">
            Settings are unavailable until the download engine connects.
          </p>
        )}
      </DialogContent>
    </Dialog>
  );
};
