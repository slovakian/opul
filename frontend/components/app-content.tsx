import { CircleAlert } from "lucide-react";

import { Downloads } from "@/components/downloads";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useVisibleDownloads } from "@/hooks/use-workspace-view";
import type { VisibleDownloadsOptions } from "@/hooks/use-workspace-view";
import { useLibrary, useLibrarySelector } from "@/lib/library";
import type { Download } from "@/lib/model";

const noop = () => null;

export interface AppContentProps extends VisibleDownloadsOptions {
  onInspect: (download: Download) => void;
  onNew: () => void;
  onRemove: (download: Download) => void;
  selectedId: string | undefined;
}

export const AppContent = ({
  onInspect,
  onNew,
  onRemove,
  selectedId,
  ...view
}: AppContentProps) => {
  const { reconnect } = useLibrary();
  const state = useLibrarySelector((library) => library.kind);

  const failure = useLibrarySelector((library) =>
    library.kind === "failed" ? library.message : null
  );

  const { count, groups } = useVisibleDownloads(view);
  const { display, effectiveFilter, query } = view;

  if (state === "loading") {
    return (
      <div className="flex flex-col gap-4" aria-label="Loading downloads">
        {[0, 1, 2].map((key) => (
          <Skeleton key={key} className="h-16 w-full" />
        ))}
      </div>
    );
  }

  if (failure !== null) {
    return (
      <Alert>
        <CircleAlert />
        <AlertTitle>Couldn&apos;t open your library</AlertTitle>
        <AlertDescription>
          <p>{failure}</p>
          <Button variant="outline" size="sm" onClick={reconnect}>
            Try again
          </Button>
        </AlertDescription>
      </Alert>
    );
  }

  if (groups.size) {
    return (
      <>
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
        <p className="text-muted-foreground px-3 text-xs">
          {count} {count === 1 ? "download" : "downloads"}
        </p>
      </>
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
