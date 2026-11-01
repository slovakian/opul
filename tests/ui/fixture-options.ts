import type {
  AggregateView,
  Command,
  DisplayPreferences,
  Preferences,
  Snapshot,
} from "../../frontend/lib/model";

export interface DesktopInvokeArgs {
  command?: Command;
  preferences?: Preferences | DisplayPreferences;
  view?: AggregateView;
  workspaceId?: string;
  choices?: Preferences["workspace_expansion"];
}

export interface DesktopOptions {
  changeSnapshotDuringDraft?: boolean;
  empty?: boolean;
  fail?: boolean;
  failFirstAdd?: boolean;
  failFirstAppearance?: boolean;
  failFirstCreate?: boolean;
  failFirstPreferences?: boolean;
  failFirstExpansion?: boolean;
  holdFirstExpansion?: boolean;
  failFirstView?: boolean;
  failPreferencesRead?: boolean;
  folderPickerError?: string;
  folderPickerPath?: string | null;
  torrentPickerPath?: string | null;
  holdAdd?: boolean;
  folderRemoval?: "immediate" | "deferred" | "rejected";
  loseFirstAppearanceResponse?: boolean;
  loseFirstCreateResponse?: boolean;
  persist?: boolean;
  preferences?: Preferences;
  recovery?: boolean;
  snapshot?: Snapshot;
}
