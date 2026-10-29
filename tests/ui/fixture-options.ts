import type { Preferences, Snapshot } from "../../frontend/lib/model";

export interface DesktopOptions {
  changeSnapshotDuringDraft?: boolean;
  empty?: boolean;
  fail?: boolean;
  failFirstAdd?: boolean;
  failFirstAppearance?: boolean;
  failFirstCreate?: boolean;
  failFirstPreferences?: boolean;
  failFirstView?: boolean;
  failPreferencesRead?: boolean;
  folderPickerError?: string;
  folderPickerPath?: string | null;
  torrentPickerPath?: string | null;
  holdAdd?: boolean;
  loseFirstAppearanceResponse?: boolean;
  loseFirstCreateResponse?: boolean;
  persist?: boolean;
  preferences?: Preferences;
  recovery?: boolean;
  snapshot?: Snapshot;
}
