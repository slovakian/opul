import { useRef } from "react";
import type { ReactNode } from "react";

import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetTitle,
} from "@/components/ui/sheet";
import { Sidebar } from "@/components/ui/sidebar";
import { useMediaQuery } from "@/hooks/use-mobile";

export const InspectorPanel = ({
  children,
  onClose,
}: {
  children: ReactNode;
  onClose: () => void;
}) => {
  const overlay = useMediaQuery("(max-width: 1100px)");
  const anchor = useRef(document.activeElement);

  if (overlay) {
    return (
      <Sheet
        open
        onOpenChange={(open) => {
          if (!open) {
            onClose();
          }
        }}
      >
        <SheetContent
          aria-modal="true"
          side="right"
          showCloseButton={false}
          className="w-80 gap-0"
          finalFocus={() =>
            anchor.current instanceof HTMLElement && anchor.current.isConnected
              ? anchor.current
              : document.querySelector<HTMLInputElement>('input[type="search"]')
          }
        >
          <SheetTitle className="sr-only">Download details</SheetTitle>
          <SheetDescription className="sr-only">
            Inspect and manage the selected download.
          </SheetDescription>
          {children}
        </SheetContent>
      </Sheet>
    );
  }

  return (
    <Sidebar
      side="right"
      collapsible="none"
      className="inspector bg-background w-80 shrink-0 border-l"
      // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role
      role="complementary"
      aria-label="Download details"
    >
      {children}
    </Sidebar>
  );
};
