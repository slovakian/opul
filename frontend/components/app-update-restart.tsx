import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  cancelAppUpdateRestart,
  restartForAppUpdate,
  useAppUpdates,
} from "@/hooks/use-app-updates";

/** Restart is the only way an update takes over now, so it asks before interrupting work. */
export const AppUpdateRestartDialog = ({
  activeDownloads,
}: {
  activeDownloads: number;
}) => {
  const { confirmingRestart } = useAppUpdates();
  const noun = activeDownloads === 1 ? "download" : "downloads";

  return (
    <Dialog
      open={confirmingRestart}
      onOpenChange={(open) => {
        if (!open) {
          cancelAppUpdateRestart();
        }
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Restart Opul now?</DialogTitle>
          <DialogDescription>
            {`Restarting interrupts ${activeDownloads} active ${noun}. You can also choose Later; the update finishes the next time you open Opul.`}
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={cancelAppUpdateRestart}>
            Later
          </Button>
          <Button
            onClick={() => {
              void restartForAppUpdate();
            }}
          >
            Restart
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
