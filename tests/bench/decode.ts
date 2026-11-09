/**
 * What the page spends turning one engine update into library state, before
 * React renders anything: parsing and validating the message, then applying it.
 *
 *   vpx tests/bench/decode.ts
 */
import { applyFeedMessage } from "../../frontend/lib/library-store";
import { feedMessageSchema, snapshotSchema } from "../../frontend/lib/model";
import { baseDownload, snapshot } from "../ui/fixture-data";

const rounds = 50;

const median = (samples: number[]) =>
  samples.toSorted((a, b) => a - b)[Math.floor(samples.length / 2)] ?? 0;

const time = (work: () => void) => {
  const samples: number[] = [];

  for (let round = 0; round < rounds; round += 1) {
    const started = performance.now();

    work();
    samples.push(performance.now() - started);
  }

  return Math.round(median(samples) * 100) / 100;
};

for (const count of [500, 2000]) {
  const [workspace] = snapshot.workspaces;

  const library = {
    ...snapshot,
    workspaces: [
      {
        ...workspace,
        downloads: Array.from({ length: count }, (_, index) => ({
          ...baseDownload,
          id: `download-${index}`,
        })),
      },
    ],
  };

  const snapshotJson = JSON.stringify(library);

  const patchJson = JSON.stringify({
    downloads: [{ ...baseDownload, download_rate: 1, id: "download-0" }],
    kind: "patch",
    revision: library.revision,
    sequence: library.sequence + 1,
  });

  const current = applyFeedMessage(
    null,
    feedMessageSchema.parse({ ...library, kind: "snapshot" })
  );

  const polled = time(() => snapshotSchema.parse(JSON.parse(snapshotJson)));

  const patched = time(() =>
    applyFeedMessage(current, feedMessageSchema.parse(JSON.parse(patchJson)))
  );

  // oxlint-disable-next-line no-console
  console.log(
    `${count} downloads: poll ${snapshotJson.length} B, ${polled} ms; patch ${patchJson.length} B, ${patched} ms`
  );
}
