import type { Page } from "@playwright/test";

/** One React commit: which components rendered and how long rendering took. */
export interface CommitRecord {
  at: number;
  durationMs: number;
  rendered: Record<string, number>;
}

declare global {
  interface Window {
    renderProbe: { commits: CommitRecord[]; reset: () => void };
  }
}

/**
 * Installs a minimal React DevTools hook before React loads. React then
 * reports every commit and, because a hook is present, measures render time
 * for every fiber (`actualDuration`). The walk mirrors DevTools' own
 * "did this fiber render" rule: a subtree whose child pointer is shared with
 * the previous tree was skipped entirely, and a component rendered when React
 * flagged it with PerformedWork.
 */
export const installRenderProbe = async (page: Page) => {
  await page.addInitScript(() => {
    const performedWork = 1;
    const componentTags = new Set([0, 1, 11, 14, 15]);
    const commits: CommitRecord[] = [];

    interface ComponentType {
      displayName?: string;
      name?: string;
      render?: ComponentType;
      type?: ComponentType;
    }

    /** The fields of React's internal fiber that the walk reads. */
    interface Fiber {
      actualDuration?: number;
      alternate: Fiber | null;
      child: Fiber | null;
      elementType: ComponentType | null;
      flags: number;
      sibling: Fiber | null;
      tag: number;
      type: ComponentType | null;
    }

    // Memo and forwardRef wrap the component they name.
    const nameOf = (fiber: Fiber): string => {
      const type = fiber.elementType ?? fiber.type;
      const inner = type?.type ?? type?.render ?? type;

      return (
        type?.displayName ?? inner?.displayName ?? inner?.name ?? "Anonymous"
      );
    };

    // PerformedWork is React's lowest flag bit.
    const performed = (fiber: Fiber) => fiber.flags % 2 === performedWork;

    const walk = (fiber: Fiber | null, rendered: Record<string, number>) => {
      let current = fiber;

      while (current) {
        const previous = current.alternate;

        if (
          componentTags.has(current.tag) &&
          (!previous || performed(current))
        ) {
          const name = nameOf(current);

          rendered[name] = (rendered[name] ?? 0) + 1;
        }

        if (!previous || current.child !== previous.child) {
          walk(current.child, rendered);
        }

        current = current.sibling;
      }
    };

    let nextId = 0;

    Object.defineProperty(window, "__REACT_DEVTOOLS_GLOBAL_HOOK__", {
      value: {
        checkDCE: () => null,
        inject: () => {
          nextId += 1;

          return nextId;
        },
        isDisabled: false,
        onCommitFiberRoot: (_id: number, root: { current: Fiber }) => {
          const rendered: Record<string, number> = {};
          const top = root.current;

          if (!top.alternate || top.child !== top.alternate.child) {
            walk(top.child, rendered);
          }

          commits.push({
            at: performance.now(),
            durationMs: top.actualDuration ?? 0,
            rendered,
          });
        },
        onCommitFiberUnmount: () => null,
        onPostCommitFiberRoot: () => null,
        renderers: new Map(),
        supportsFiber: true,
      },
    });

    window.renderProbe = {
      commits,
      reset: () => {
        commits.length = 0;
      },
    };
  });
};
