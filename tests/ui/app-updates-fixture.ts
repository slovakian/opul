import type { Page } from "@playwright/test";

import type { AppUpdates } from "../../frontend/lib/app-updates";
import type { DesktopInvokeArgs, DesktopOptions } from "./fixture-options";

declare global {
  interface Window {
    testAppUpdates: {
      invoke: (
        name: string,
        args: DesktopInvokeArgs
      ) => AppUpdates | string | null;
    };
  }
}

export const appUpdates: AppUpdates = {
  channel: "stable",
  check_automatically: true,
  current_version: "0.3.0",
  install_automatically: false,
  install_blocker: null,
  last_checked: null,
  phase: { kind: "idle" },
  release_notes_url: null,
  sequence: 1,
};

/** Each simulated network step takes this long, so tests can see the states between. */
const STEP_MS = 250;

/**
 * Mirrors `AppUpdates` in Rust: single-flight checks, a per-version notice claim that
 * survives reloads, silent automatic installs, and blocked copies that can't install.
 */
export const mockAppUpdates = async (page: Page, options: DesktopOptions) => {
  await page.addInitScript(
    ({ initial, mockOpts, step }) => {
      const saved = sessionStorage.getItem("test-app-updates");

      const current: AppUpdates = saved
        ? JSON.parse(saved)
        : structuredClone(initial);

      const publish = () => {
        current.sequence += 1;
        const { phase } = current;
        current.release_notes_url =
          "version" in phase
            ? `https://github.com/slovakian/opul/releases/tag/v${phase.version}`
            : null;
        sessionStorage.setItem("test-app-updates", JSON.stringify(current));
        window.testVideoTools.emit("app-updates", current);
      };

      const installsAutomatically = () =>
        current.check_automatically &&
        current.install_automatically &&
        !current.install_blocker;

      const download = (automatic: boolean) => {
        const { phase } = current;

        if (phase.kind !== "available" && phase.kind !== "download_failed") {
          return;
        }

        const { version } = phase;
        const total = 40_000_000;
        current.phase = {
          automatic,
          downloaded: 0,
          kind: "downloading",
          total,
          version,
        };
        publish();
        setTimeout(() => {
          current.phase = {
            automatic,
            downloaded: total / 2,
            kind: "downloading",
            total,
            version,
          };
          publish();
        }, step);
        setTimeout(() => {
          current.phase = mockOpts.failAppUpdateDownload
            ? {
                automatic,
                kind: "download_failed",
                message:
                  "The update didn't pass signature verification, so Opul didn't install it.",
                version,
              }
            : { kind: "ready", version };
          publish();
        }, step * 2);
      };

      const check = () => {
        if (["checking", "downloading", "ready"].includes(current.phase.kind)) {
          return;
        }

        current.phase = { kind: "checking" };
        publish();
        setTimeout(() => {
          const found = sessionStorage.getItem("test-app-update-release");
          current.last_checked = Math.floor(Date.now() / 1000);
          current.phase = found
            ? { kind: "available", version: found }
            : { kind: "up_to_date" };
          publish();

          if (found && installsAutomatically()) {
            download(true);
          }
        }, step);
      };

      // Tests replace the snapshot to simulate a scheduled check or a new release.
      window.addEventListener("test:app-updates", () => {
        const replacement = sessionStorage.getItem("test-app-updates");

        if (replacement) {
          Object.assign(current, JSON.parse(replacement));
          publish();

          if (current.phase.kind === "available" && installsAutomatically()) {
            download(true);
          }
        }
      });

      window.addEventListener("test:menu-check-for-updates", () => {
        window.testVideoTools.emit("check-for-updates", null);
      });

      window.testAppUpdates = {
        invoke: (name, args) => {
          sessionStorage.setItem(
            "test-app-update-command",
            JSON.stringify({ args, name })
          );

          if (name === "app_updates_check") {
            check();
          } else if (name === "app_updates_download") {
            if (current.install_blocker) {
              throw new Error("Development builds can't install updates.");
            }

            download(false);
          } else if (name === "app_updates_configure") {
            current.check_automatically = Boolean(args.checkAutomatically);
            current.install_automatically = Boolean(args.installAutomatically);
            publish();

            if (current.phase.kind === "available" && installsAutomatically()) {
              download(true);
            }
          } else if (name === "app_updates_take_notice") {
            const { phase } = current;

            if (
              phase.kind !== "available" ||
              installsAutomatically() ||
              current.install_blocker
            ) {
              return null;
            }

            const key = `test-app-update-notified-${phase.version}`;

            if (sessionStorage.getItem(key)) {
              return null;
            }

            sessionStorage.setItem(key, "true");

            return phase.version;
          } else if (name === "app_updates_restart") {
            sessionStorage.setItem("test-app-update-restarted", "true");

            return null;
          }

          return structuredClone(current);
        },
      };
    },
    {
      initial: options.appUpdates ?? appUpdates,
      mockOpts: options,
      step: STEP_MS,
    }
  );
};
