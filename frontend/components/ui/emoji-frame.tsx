import type { ReactNode } from "react";
import { Check } from "lucide-react";

export const emojiOptionClassName =
  "group/emoji-option relative flex size-9 shrink-0 items-center justify-center rounded-md outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring";

// Fork of shadcn's Avatar + AvatarBadge for transparent, square emoji options.
export function EmojiFrame({
  selected,
  children,
}: {
  selected: boolean;
  children: ReactNode;
}) {
  return (
    <span
      aria-hidden
      data-slot="emoji-frame"
      className="pointer-events-none relative flex size-full items-center justify-center rounded-[inherit] text-2xl group-hover/emoji-option:bg-muted"
    >
      <span
        data-slot="emoji-frame-artwork"
        className="inline-flex leading-none transition-transform duration-150 group-hover/emoji-option:scale-110 motion-reduce:transform-none motion-reduce:transition-none"
      >
        {children}
      </span>
      {selected && (
        <span
          data-slot="emoji-frame-badge"
          className="absolute right-0 bottom-0 inline-flex size-3 items-center justify-center rounded-full bg-primary text-primary-foreground ring-2 ring-popover [&>svg]:size-2"
        >
          <Check strokeWidth={3} />
        </span>
      )}
    </span>
  );
}
