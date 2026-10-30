import { Download, Info, Monitor, Network, Wrench } from "lucide-react";
import { useEffect, useState } from "react";

import opulLogo from "@/assets/opul-logo.png";
import { Choice } from "@/components/sidebar-popover";
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
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import { Switch } from "@/components/ui/switch";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { bridge, errorMessage } from "@/lib/bridge";
import { useLibrary } from "@/lib/library";
import type { Preferences, ToolStatus } from "@/lib/model";

const sections = [
  { icon: Monitor, label: "Appearance" },
  { icon: Download, label: "Downloads" },
  { icon: Network, label: "BitTorrent" },
  { icon: Wrench, label: "Tools" },
  { icon: Info, label: "About" },
];

const SettingsContent = ({
  initial,
  onClose,
}: {
  initial: Preferences;
  onClose: () => void;
}) => {
  const { savePreferences, state } = useLibrary();
  const workspaces = state.kind === "ready" ? state.snapshot.workspaces : [];
  const [section, setSection] = useState("Appearance");
  const [draft, setDraft] = useState(initial);
  const [saving, setSaving] = useState(false);
  const [tools, setTools] = useState<ToolStatus | null>(null);
  const [toolError, setToolError] = useState("");

  useEffect(() => {
    const loadTools = async () => {
      try {
        const status = await bridge.toolStatus();

        setTools(status);
      } catch (error) {
        setToolError(errorMessage(error));
      }
    };

    loadTools().catch(() => null);
  }, []);

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
        <div className="flex flex-1 flex-col gap-6 overflow-y-auto px-6 pb-6">
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
                      { label: "Use a default workspace", value: "workspace" },
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
                        Choose a workspace. The previous default is unavailable;
                        downloads will ask for a destination.
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
          {section === "Tools" && (
            <>
              <p className="text-muted-foreground text-sm">
                Direct files and torrents work on their own. Video pages use
                yt-dlp; audio conversion uses FFmpeg.
              </p>
              <output className="bg-muted block rounded-lg p-3 text-sm">
                {toolError || tools?.message || "Checking installed tools…"}
              </output>
              <FieldGroup>
                <Field>
                  <FieldLabel htmlFor="yt-dlp">yt-dlp path</FieldLabel>
                  <Input
                    id="yt-dlp"
                    placeholder="Auto-detect"
                    value={draft.yt_dlp_path ?? ""}
                    onChange={(event) =>
                      setDraft({
                        ...draft,
                        yt_dlp_path: event.target.value.trim() || null,
                      })
                    }
                  />
                  <FieldDescription>
                    {tools?.yt_dlp_version ??
                      "Install with: brew install yt-dlp"}
                  </FieldDescription>
                </Field>
                <Field>
                  <FieldLabel htmlFor="ffmpeg">FFmpeg path</FieldLabel>
                  <Input
                    id="ffmpeg"
                    placeholder="Auto-detect"
                    value={draft.ffmpeg_path ?? ""}
                    onChange={(event) =>
                      setDraft({
                        ...draft,
                        ffmpeg_path: event.target.value.trim() || null,
                      })
                    }
                  />
                  <FieldDescription>
                    {tools?.ffmpeg_path ?? "Install with: brew install ffmpeg"}
                  </FieldDescription>
                </Field>
              </FieldGroup>
              <p className="text-muted-foreground text-xs">
                Restart Opul after changing tool paths.
              </p>
            </>
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
                Built with Tauri, Base UI, and the Opul Rust core. Free and open
                source under the MIT license.
              </p>
              <p className="text-muted-foreground text-xs">
                Version 0.1.0 · Updates are available on GitHub.
              </p>
            </div>
          )}
        </div>
        <footer className="flex justify-end gap-2 px-6 py-4">
          <Button type="button" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={saving}>
            {saving ? "Saving…" : "Save changes"}
          </Button>
        </footer>
      </main>
    </form>
  );
};

export const SettingsDialog = ({ onClose }: { onClose: () => void }) => {
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
          <SettingsContent initial={preferences} onClose={onClose} />
        ) : (
          <p className="p-8">
            Settings are unavailable until the download engine connects.
          </p>
        )}
      </DialogContent>
    </Dialog>
  );
};
