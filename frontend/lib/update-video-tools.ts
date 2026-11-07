import { toast } from "@/components/ui/toast";
import { bridge, errorMessage } from "@/lib/bridge";

export const updateVideoTools = async () => {
  try {
    await bridge.videoToolsExecute({ kind: "update" });
  } catch (error) {
    toast.add({
      description: errorMessage(error),
      title: "Couldn't update video tools",
      type: "error",
    });
  }
};
