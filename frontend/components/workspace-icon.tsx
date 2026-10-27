import {
  workspaceEmoji,
  type Workspace,
  type TreatmentParameters,
} from "@/lib/model";

import { canTreatEmoji, emojiTreatment } from "@/lib/workspace-appearance";

type Treatment = TreatmentParameters & {
  preset_id?: string;
};

export function EmojiArtwork({
  emoji,
  treatment,
}: {
  emoji: string;
  treatment?: Treatment | null;
}) {
  const applied = canTreatEmoji(emoji) ? treatment : null;
  return (
    <span
      aria-hidden
      data-slot="emoji-artwork"
      data-preset={applied?.preset_id}
      className="inline-block shrink-0 leading-none"
      style={
        applied
          ? {
              filter: `hue-rotate(${applied.hue}deg) saturate(${applied.saturation}%) brightness(${applied.brightness}%)`,
            }
          : undefined
      }
    >
      {emoji}
    </span>
  );
}

export function WorkspaceIcon({
  workspace,
}: {
  workspace: Pick<Workspace, "icon" | "appearance">;
}) {
  const emoji = workspaceEmoji(workspace);
  return (
    <EmojiArtwork
      emoji={emoji}
      treatment={emojiTreatment(emoji, workspace.appearance)}
    />
  );
}
