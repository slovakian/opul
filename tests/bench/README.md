# Library sync benchmarks

These measure what the interface spends staying in sync with the desktop. None of them run with `vp test` or `vp run test`.

## Renders, render time and latency

`vp run bench` loads the app in WebKit against the mock desktop with a large library: one transferring download and the rest finished. A React DevTools hook (`render-probe.ts`) records every commit, which components rendered, and React's measured render time. The run reports:

- `idlePerMinute`: commits, component renders and render time while nothing changes.
- `perEngineUpdate`: the same for one transfer update, sent at 4 Hz for 10 s and divided by the number of updates.
- `latencyMs`: time from the mock engine changing a download to the new rate appearing in that row, sampled at random moments.

`BENCH_DOWNLOADS=2000` changes the library size, `BENCH_IDLE_SECONDS` the idle window, and `BENCH_OUT=file.json` writes the results. The mock desktop computes patches in the page, so latency includes work the real desktop does on its own thread.

`vpx tests/bench/decode.ts` times, in Node, the JavaScript cost of turning one update into library state: parsing and validating the whole library for a poll, against parsing, validating and applying a patch.

The desktop side is `cargo test --release -p Opul feed_cost -- --ignored --nocapture`, which serializes the whole library against diffing and serializing a patch.

## The app at rest

1. Seed an idle library: `OPUL_SEED_DIR=/tmp/opul-500 OPUL_SEED_COUNT=500 cargo test -p Opul seed_benchmark_library -- --ignored`.
2. Build the frontend and an app that embeds it: `vp run build && cargo build -p Opul --features tauri/custom-protocol`, then copy `target/debug/Opul` aside. Repeat on the other revision.
3. For each binary, run `tests/bench/idle-cpu.sh <binary> /tmp/opul-500 <label> 40 60`. It starts the app on a copy of the library, waits for startup to settle, then reports the CPU time and memory of the app and its WebContent process over 60 s.
