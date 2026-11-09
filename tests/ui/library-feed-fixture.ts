import type { Page } from "@playwright/test";

import type { FeedMessageInput, Snapshot } from "../../frontend/lib/model";

export interface FeedChannel {
  onmessage: (message: FeedMessageInput) => void;
}

declare global {
  interface Window {
    testLibraryFeed: {
      /** Starts a feed the way `subscribe_library` does: the library, then changes. */
      subscribe: (
        feed: FeedChannel,
        read: () => Promise<Snapshot>
      ) => Promise<void>;
      /** Sends whatever changed since the last message, like the desktop after a commit. */
      push: () => void;
      /** Every message sent, newest last, for tests that inspect the protocol. */
      messages: FeedMessageInput[];
    };
  }
}

/**
 * Stands in for the desktop's library feed. It diffs the mock library exactly
 * as `library_feed.rs` does, so the interface always runs on real patches.
 */
export const mockLibraryFeed = async (page: Page) => {
  await page.addInitScript(() => {
    type Workspace = Snapshot["workspaces"][number];

    let channel: FeedChannel | undefined;
    let read: (() => Promise<Snapshot>) | undefined;
    let sent: Snapshot | undefined;
    const messages: FeedMessageInput[] = [];

    // Init scripts are serialized, so helpers must live inside them.
    // oxlint-disable-next-line unicorn/consistent-function-scoping
    const same = <T>(left: T, right: T) =>
      JSON.stringify(left) === JSON.stringify(right);

    const outline = ({ downloads, ...workspace }: Workspace) => ({
      ...workspace,
      download_ids: downloads.map((download) => download.id),
    });

    const changes = (previous: Snapshot, next: Snapshot): FeedMessageInput => {
      if (previous.boot_id !== next.boot_id) {
        return { kind: "snapshot", ...next };
      }

      const before = new Map(
        previous.workspaces
          .flatMap((workspace) => workspace.downloads)
          .map((download) => [download.id, JSON.stringify(download)])
      );

      const downloads = next.workspaces
        .flatMap((workspace) => workspace.downloads)
        .filter(
          (download) => before.get(download.id) !== JSON.stringify(download)
        );

      const outlines = next.workspaces.map(outline);

      return {
        kind: "patch",
        revision: next.revision,
        sequence: next.sequence,
        ...(downloads.length > 0 && { downloads }),
        ...(!same(previous.recent_emoji, next.recent_emoji) && {
          recent_emoji: next.recent_emoji,
        }),
        ...(!same(previous.recovery_notices, next.recovery_notices) && {
          recovery_notices: next.recovery_notices,
        }),
        ...(previous.session !== next.session && { session: next.session }),
        ...(!same(previous.workspaces.map(outline), outlines) && {
          workspaces: outlines,
        }),
      };
    };

    const deliver = (message: FeedMessageInput) => {
      messages.push(message);
      // The desktop serializes every message, so the page never shares objects with it.
      channel?.onmessage(structuredClone(message));
    };

    let reads = 0;
    let delivered = 0;

    // Reads may overlap, as a held read in one fixture can. A read that
    // finishes after a newer one was delivered is stale, so it is dropped:
    // the desktop never sends an older library after a newer one.
    const push = async () => {
      if (!read) {
        return;
      }

      reads += 1;
      const ticket = reads;

      try {
        const next = structuredClone(await read());

        if (sent && ticket > delivered && !same(sent, next)) {
          delivered = ticket;
          deliver(changes(sent, next));
          sent = next;
        }
      } catch {
        // An unreadable library sends nothing; the page keeps what it has.
      }
    };

    // Changes made by other fixtures (deferred removals, confirmations) arrive here.
    setInterval(() => push(), 100);

    window.testLibraryFeed = {
      messages,
      push: () => {
        push().catch(() => null);
      },
      subscribe: async (feed, reader) => {
        const first = structuredClone(await reader());

        channel = feed;
        read = reader;
        sent = first;
        delivered = reads;
        deliver({ kind: "snapshot", ...first });
      },
    };
  });
};
