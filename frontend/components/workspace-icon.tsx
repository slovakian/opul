import { workspaceEmoji, type Workspace } from "@/lib/model";

type Appearance = Workspace["appearance"];

export function EmojiArtwork({
  emoji,
  treatment,
}: {
  emoji: string;
  treatment?: Appearance["automatic_folder"];
}) {
  return (
    <span
      aria-hidden
      data-slot="emoji-artwork"
      data-preset={treatment?.preset_id}
      className="inline-block shrink-0 leading-none"
      style={
        treatment
          ? {
              filter: `hue-rotate(${treatment.hue}deg) saturate(${treatment.saturation}%) brightness(${treatment.brightness}%)`,
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
  const folder = emoji === "📁" || emoji === "📁\uFE0F";
  return (
    <EmojiArtwork
      emoji={emoji}
      treatment={folder ? workspace.appearance.automatic_folder : null}
    />
  );
}
