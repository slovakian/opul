import downloadFolder from "@/assets/download-folder.png";
import { colorTreatmentFilter } from "@/lib/color-treatment";
import { downloadFolderColor } from "@/lib/download-folder";
import type { Download } from "@/lib/model";
import { cn } from "@/lib/utils";

// Decorative: the download's name always labels the row or panel beside it.
export const DownloadFolderIcon = ({
  className,
  download,
}: {
  className?: string;
  download: Pick<Download, "id">;
}) => {
  const { id, treatment } = downloadFolderColor(download.id);

  return (
    <img
      src={downloadFolder}
      alt=""
      draggable={false}
      data-slot="download-folder-icon"
      data-color={id}
      className={cn("pointer-events-none shrink-0 select-none", className)}
      style={
        treatment ? { filter: colorTreatmentFilter(treatment) } : undefined
      }
    />
  );
};
