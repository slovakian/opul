# Disposable Rune baseline preview

This is the minimal framework and agent-tooling check for [#100](https://github.com/slovakian/opul/issues/100). It is separate from the production Cargo workspace. Its in-memory Download is a fixture; no core, torrent engine, video tools or network transfer is started. The sidebar is a layout reference with static labels. This is not the visual comparison playground requested by #101.

From the repository root, build and open the native window:

```sh
Scripts/rune-preview.sh
```

Requires Rust/Cargo and the macOS command-line developer tools. The first run fetches the locked dependencies. Close the window to quit. Pause fixture, Resume fixture and Toggle appearance are interactive and expose stable keys and accessible names. Storage directories are always `.build/rune-preview/data` and `.build/rune-preview/Downloads`; they stay empty in this fixture. The production application directories are never opened. Build output stays under `target/rune-preview`.

Run the same view and handlers without a window, checking semantic output and writing screenshots:

```sh
python3 Scripts/check-rune-preview.py
```

For individual captures and native launch evidence, see [commands, tested targets, and limitations](../../docs/rune/preview.md). The [behavior baseline](../../docs/rune/baseline.md) records what production migration must preserve.

Rune and the five upstream dependency patches are pinned to `cca5df6fafb4721d96fe105f126ebfd2ec351bb6`. Keep every revision aligned when updating. Commit this workspace's `Cargo.lock`; the launch command uses `--locked`. Features enable embedded fonts, native accessibility, Metal and release tooling. The preview uses Rune's public APIs, with its [hello](https://github.com/anomalyco/rune/tree/cca5df6fafb4721d96fe105f126ebfd2ec351bb6/examples/hello) and [todo](https://github.com/anomalyco/rune/tree/cca5df6fafb4721d96fe105f126ebfd2ec351bb6/examples/todo) examples as references under Rune's [MIT license](https://github.com/anomalyco/rune/blob/cca5df6fafb4721d96fe105f126ebfd2ec351bb6/LICENSE). It has no Rune-kit dependency or copied kit components. No production styling decision is implied.
