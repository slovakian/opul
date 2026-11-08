import type { TreatmentParameters } from "@/lib/model";

export interface DownloadFolderColor {
  id: string;
  // `null` shows the artwork's own blue without a filter.
  treatment: TreatmentParameters | null;
}

// Tuned in WebKit against `frontend/assets/download-folder.png`, whose face is
// roughly hsl(203 92% 69%). They don't carry over to other artwork.
export const downloadFolderColors: readonly DownloadFolderColor[] = [
  { id: "blue", treatment: null },
  { id: "red", treatment: { brightness: 100, hue: 154, saturation: 300 } },
  { id: "orange", treatment: { brightness: 105, hue: 183, saturation: 150 } },
  { id: "yellow", treatment: { brightness: 121, hue: 203, saturation: 125 } },
  { id: "green", treatment: { brightness: 97, hue: 282, saturation: 105 } },
  { id: "purple", treatment: { brightness: 94, hue: 49, saturation: 400 } },
];

// A polynomial string hash kept below 2^31, so every step stays an exact
// integer in floating point.
const HASH_MULTIPLIER = 31;
const HASH_MODULUS = 2_147_483_647;

const hash = (value: string) => {
  let result = 0;

  for (const character of value) {
    result =
      (result * HASH_MULTIPLIER + (character.codePointAt(0) ?? 0)) %
      HASH_MODULUS;
  }

  return result;
};

// Looks random but stays stable for a download. Changing the palette's length
// reshuffles existing downloads, which is fine because nothing is persisted.
export const downloadFolderColor = (downloadId: string): DownloadFolderColor =>
  downloadFolderColors[hash(downloadId) % downloadFolderColors.length] ??
  downloadFolderColors[0];
