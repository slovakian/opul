import type { Page } from "@playwright/test";

import type {
  ToolsSnapshot,
  VideoToolsSnapshot,
} from "../../frontend/lib/video-tools";
import { toolsSnapshot } from "./fixture-data";
import type { DesktopInvokeArgs, DesktopOptions } from "./fixture-options";

interface ToolsEvent {
  event: string;
  id: number;
  payload: VideoToolsSnapshot;
}

declare global {
  interface Window {
    testVideoTools: {
      snapshot: () => VideoToolsSnapshot;
      invoke: (
        name: string,
        args: DesktopInvokeArgs
      ) => ToolsSnapshot | VideoToolsSnapshot | string | number | null;
      register: (handler: (event: ToolsEvent) => void) => number;
    };
  }
}

export const mockVideoTools = async (page: Page, options: DesktopOptions) => {
  await page.addInitScript(
    ({ mockOpts, tools }) => {
      const saved = mockOpts.persist
        ? sessionStorage.getItem("test-video-tools")
        : null;

      const currentTools = {
        ...tools,
        video: saved
          ? JSON.parse(saved)
          : structuredClone(mockOpts.videoTools ?? tools.video),
      };

      const toolCallbacks = new Map<
        number,
        (event: {
          event: string;
          id: number;
          payload: VideoToolsSnapshot;
        }) => void
      >();

      const toolListeners = new Map<number, string>();

      const pushTools = () => {
        sessionStorage.setItem(
          "test-video-tools",
          JSON.stringify(currentTools.video)
        );

        for (const [id, event] of toolListeners) {
          if (event !== "video-tools") {
            continue;
          }

          toolCallbacks.get(id)?.({
            event: "video-tools",
            id,
            payload: structuredClone(currentTools.video),
          });
        }
      };

      window.addEventListener("test:video-tools", () => {
        const replacement = sessionStorage.getItem("test-video-tools");

        if (replacement) {
          currentTools.video = JSON.parse(replacement);
          pushTools();
        }
      });

      window.addEventListener("test:open-video-setup", () => {
        for (const [id, event] of toolListeners) {
          if (event === "set-up-video-tools") {
            toolCallbacks.get(id)?.({
              event,
              id,
              payload: structuredClone(currentTools.video),
            });
          }
        }
      });

      let callbackId = 0;
      window.testVideoTools = {
        invoke: (name, args) => {
          if (name === "tools_snapshot") {
            return structuredClone(currentTools);
          }

          if (name === "video_tools_details") {
            const failure = currentTools.video.state;

            return `Video tools: ${failure.kind}${failure.kind === "not_working" ? `\n${failure.reason}: ${failure.details}` : ""}\nPython runtime 1, yt-dlp 2026.10.26, chip arm64, Opul 0.2.2.`;
          }

          if (name === "plugin:event|listen" && args.event && args.handler) {
            toolListeners.set(args.handler, args.event);

            return args.handler;
          }

          if (name === "plugin:event|unlisten" && args.eventId) {
            toolListeners.delete(args.eventId);

            return null;
          }

          if (name === "video_tools_execute" && args.operation) {
            const { operation } = args;
            sessionStorage.setItem(
              "test-video-tools-operation",
              JSON.stringify(operation)
            );

            if (operation.kind === "set_keep_up_to_date") {
              if (mockOpts.failVideoToolsPreference) {
                throw new Error("Couldn't save the video tools preference.");
              }

              currentTools.video.keep_up_to_date = operation.enabled;
            } else if (
              operation.kind === "set_up" ||
              operation.kind === "retry"
            ) {
              currentTools.video.state = {
                kind: "setting_up",
                steps: [
                  {
                    component: "python",
                    downloaded_bytes: 0,
                    status: "downloading",
                    total_bytes: 17_143_107,
                  },
                  {
                    component: "yt_dlp",
                    downloaded_bytes: 0,
                    status: "pending",
                    total_bytes: null,
                  },
                ],
              };
            } else if (operation.kind === "cancel_setup") {
              currentTools.video.state = { kind: "not_set_up" };
            } else if (operation.kind === "turn_off") {
              currentTools.video.state = { kind: "turned_off" };
            }

            currentTools.video.sequence += 1;
            pushTools();

            return structuredClone(currentTools.video);
          }

          return null;
        },
        register: (handler) => {
          callbackId += 1;
          toolCallbacks.set(callbackId, handler);

          return callbackId;
        },
        snapshot: () => structuredClone(currentTools.video),
      };
    },
    { mockOpts: options, tools: toolsSnapshot }
  );
};
