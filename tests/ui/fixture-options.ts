import type {
  AggregateView,
  Command,
  DisplayPreferences,
  Preferences,
  Snapshot,
  WorkspaceExpansionChanges,
} from "../../frontend/lib/model";
import type {
  VideoToolsOperation,
  VideoToolsSnapshot,
} from "../../frontend/lib/video-tools";

export interface DesktopInvokeArgs {
  command?: Command;
  operation?: VideoToolsOperation;
  turnOff?: boolean;
  event?: string;
  handler?: number;
  eventId?: number;
  preferences?: Preferences | DisplayPreferences;
  view?: AggregateView;
  workspaceId?: string;
  choices?: WorkspaceExpansionChanges;
}

export interface DesktopOptions {
  videoTools?: VideoToolsSnapshot;
  failVideoToolsPreference?: boolean;
  holdPreferenceResponses?: PreferenceResponse[];
  changeSnapshotDuringDraft?: boolean;
  empty?: boolean;
  fail?: boolean;
  failFirstAdd?: boolean;
  failFirstAppearance?: boolean;
  failFirstCreate?: boolean;
  failFirstPreferences?: boolean;
  failFirstExpansion?: boolean;
  failExpansionAttempts?: number;
  holdFirstExpansion?: boolean;
  failFirstView?: boolean;
  failPreferencesRead?: boolean;
  folderPickerError?: string;
  folderPickerPath?: string | null;
  folderAddition?: "rejected" | "null" | "deferred" | "refresh-failure";
  holdFolderRefresh?: boolean;
  holdFolderReceipt?: boolean;
  torrentPickerPath?: string | null;
  holdAdd?: boolean;
  workspaceDeletion?: "immediate" | "deferred" | "rejected";
  folderRemoval?: "immediate" | "deferred" | "rejected";
  loseFirstAppearanceResponse?: boolean;
  loseFirstCreateResponse?: boolean;
  persist?: boolean;
  preferences?: Preferences;
  recovery?: boolean;
  snapshot?: Snapshot;
}

export type PreferenceResponse = "read" | "settings" | "display" | "expansion";
