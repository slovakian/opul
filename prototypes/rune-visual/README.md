# Disposable Opul visual study

This is the native playground for [#101](https://github.com/slovakian/opul/issues/101), adapted from `mattpocock-skills:prototype`. It is separate from production, uses in-memory fixtures, and must be rewritten as owned, tested components after a design is chosen.

From the repository root:

```sh
Scripts/rune-visual.sh
```

Use the bottom arrows or Left/Right keys to compare treatments. The controls cycle appearance, motion, card/minimal presentation, one/two Studio folders, and Downloading/Paused/Completed/Failed states. Click a Download title or its ellipsis for details. Pause/resume, search, disclosure, sidebar navigation, close, Escape, and outside dismissal operate on fixtures. Search never submits a URL or starts a Download. New workspace, Settings and filter/sort artwork provide layout context only.

Start directly with a treatment:

```sh
Scripts/rune-visual.sh --scene cards --theme light
Scripts/rune-visual.sh --scene list --theme dark
Scripts/rune-visual.sh --scene panels --size 600x720
```

State resets on every launch. There is no application-state connection, persistence, network request, transfer engine, or real Download destination.

[Design question, review status, screenshots, motion and limitations](../../docs/rune/visual-treatment.md).

Recreate renderer evidence, including actual keyed input, semantic assertions and three seven-second recordings:

```sh
python3 prototypes/rune-visual/capture.py
```

This writes `.build/rune-visual/evidence`. Cargo, Python 3 and `ffmpeg` on PATH are needed for the evidence recipe. The native exploration command needs Cargo and macOS; it does not need FFmpeg. A first build fetches the pinned Rune sources and crates automatically.

Rune and Rune-kit are both pinned to `cca5df6fafb4721d96fe105f126ebfd2ec351bb6`, with the same five patches as #100. The separate manifest/lockfile keeps these dependencies out of the production workspace. Buttons, icon buttons, text input, circular progress and theme structure come from that revision's Rune-kit. The simple sidebar/rows/effect masks/sheet are disposable compositions over Rune, not an owned production kit. Artwork comes from Opul's existing assets; the small stroke glyphs in `src/icons.rs` are local prototype artwork.
