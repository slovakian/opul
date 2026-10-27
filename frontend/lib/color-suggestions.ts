import type { Workspace } from "@/lib/model";

import palette from "../../src/application/folder-treatments.json";

type Treatment = NonNullable<Workspace["appearance"]["automatic_folder"]>;

function identitySeed(identity: string) {
  let hash = 2_166_136_261;

  for (const character of identity) {
    hash = Math.imul(hash ^ character.charCodeAt(0), 16_777_619);
  }

  return hash >>> 0;
}

function sample({ seed, saved }: { seed: number; saved: Treatment | null }) {
  const remaining = palette.filter(
    (item) => item.preset_id !== saved?.preset_id
  );

  // Fisher-Yates samples identities without replacement. The seed belongs to
  // the workspace for folders, and to one picker opening for other emoji.
  let state = seed;

  for (let index = remaining.length - 1; index > 0; index--) {
    state = (Math.imul(state, 1_664_525) + 1_013_904_223) >>> 0;
    const target = Math.floor((state / 4_294_967_296) * (index + 1));
    [remaining[index], remaining[target]] = [
      remaining[target],
      remaining[index],
    ];
  }

  return saved ? [saved, ...remaining] : remaining;
}

export function createColorSuggestions({
  workspaceId,
  appearance,
}: {
  workspaceId: string;
  appearance: Workspace["appearance"];
}) {
  const saved =
    appearance.preference.kind === "Preset"
      ? appearance.preference.treatment
      : null;

  // Reuse resolved saved values even if a later palette version changes them.
  const builtIn =
    saved && palette.some((item) => item.preset_id === saved.preset_id)
      ? saved
      : null;

  const automatic = appearance.automatic_folder;

  const folder =
    builtIn ??
    (automatic && palette.some((item) => item.preset_id === automatic.preset_id)
      ? automatic
      : null);

  return {
    folder: sample({ saved: folder, seed: identitySeed(workspaceId) }),
    other: sample({
      saved: builtIn,
      seed: crypto.getRandomValues(new Uint32Array(1))[0],
    }),
  };
}
