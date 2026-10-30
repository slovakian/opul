import { CircleAlert } from "lucide-react";

import { Downloads } from "@/components/downloads";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import type { StatusFilter } from "@/components/workspace-sidebar";
import type { useLibrary } from "@/lib/library";
import type { DisplayPreferences, Download } from "@/lib/model";

const noop = () => null;

export interface AppContentProps {
  display: DisplayPreferences;
  effectiveFilter: StatusFilter;
  groups: Map<string, Download[]>;
  onInspect: (download: Download) => void;
  onNew: () => void;
  onRemove: (download: Download) => void;
  query: string;
  refresh: () => Promise<void>;
  selectedId: string | undefined;
  state: ReturnType<typeof useLibrary>["state"];
}

export const AppContent = ({
  display,
  effectiveFilter,
  groups,
  onInspect,
  onNew,
  onRemove,
  query,
  refresh,
  selectedId,
  state,
}: AppContentProps) => {
  if (state.kind === "loading") {
    return (
      <div className="flex flex-col gap-4" aria-label="Loading downloads">
        {[0, 1, 2].map((key) => (
          <Skeleton key={key} className="h-16 w-full" />
        ))}
      </div>
    );
  }

  if (state.kind === "failed") {
    return (
      <Alert>
        <CircleAlert />
        <AlertTitle>Couldn&apos;t open your library</AlertTitle>
        <AlertDescription>
          <p>{state.message}</p>
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              refresh().catch(() => null);
            }}
          >
            Try again
          </Button>
        </AlertDescription>
      </Alert>
    );
  }

  if (groups.size) {
    return (
      <div className="flex flex-col gap-7">
        {Array.from(groups, ([label, items]) => (
          <section key={label} className="flex flex-col gap-2">
            {label && (
              <h2 className="text-muted-foreground px-3 text-xs font-medium">
                {label} · {items.length}
              </h2>
            )}
            <Downloads
              downloads={items}
              layout={display.layout}
              selectedId={selectedId}
              onInspect={onInspect}
              onRemove={onRemove}
              onNew={onNew}
              filtered={Boolean(query) || effectiveFilter !== "all"}
            />
          </section>
        ))}
      </div>
    );
  }

  return (
    <Downloads
      downloads={[]}
      layout="List"
      onInspect={noop}
      onRemove={noop}
      onNew={onNew}
      filtered={Boolean(query) || effectiveFilter !== "all"}
    />
  );
};
