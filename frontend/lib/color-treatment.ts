import type { TreatmentParameters } from "@/lib/model";

// Recolors raster artwork with CSS filters. Each feature owns presets tuned to
// its own artwork, because the same parameters land on different colors when
// the base image differs.
export const colorTreatmentFilter = ({
  brightness,
  hue,
  saturation,
}: TreatmentParameters) =>
  `hue-rotate(${hue}deg) saturate(${saturation}%) brightness(${brightness}%)`;
