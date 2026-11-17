# Native visual study for #101

**Status: the maintainer prefers the minimal direction; tighter spacing is under review.** The current request is to reduce visual space around the list items while continuing to treat this as a visual prototype. The minimal scene below opens that revision directly.

The question is: **which restrained Rune-kit surface, density and progress treatment suits Opul's existing layout?** The comparison preserves the layout and semantic colors. Production implementation, real transfers, persistence and full interaction parity remain separate tickets.

## Explore

```sh
Scripts/rune-visual.sh

# Preferred minimal direction with tighter spacing:
Scripts/rune-visual.sh --scene minimal --theme light
```

This builds and opens a native Rune window. Everything is in memory. The visible study strip is outside the product layout. Left/Right or its arrows wrap through the treatments. Those shortcuts leave a focused search field alone. Search filters the fixtures, and a title or ellipsis opens the modal sheet. Close, Escape and an outside click dismiss it; the outside click is consumed. Pause/resume updates the fixture in the sheet and list together.

System/Light/Dark, Motion/Reduced, Cards/Minimal, one/two Studio folders, and the State control expose the comparison states. The State control cycles downloading, paused, completed, failed and back. Personal is always a simple Workspace row; Studio becomes collapsible with two Saved Folders. Narrow windows use a modal sidebar. Settings, New workspace and filter/sort artwork are context, not implemented flows.

Minimal removes card/group surfaces and compacts wide rows from 58pt to 50pt. Its row inset is 12pt, artwork is 36pt, and the title/description gap is 3pt. Narrow rows keep room for inline progress and seeding text.

## Treatments under review

These are proposals, not selected tokens. All use Rune-kit's Inter, 14pt/500 Download names, 13pt/400 sidebar/body text, 12pt/16 captions, 15pt brand text, 20pt/27 sheet titles and a 232pt sidebar. The neutral light/dark palette follows Opul; existing raster artwork and colored folder strokes provide identity.

| Treatment | Rows and surfaces | Spacing and radii | Progress and sheet |
| --- | --- | --- | --- |
| A · Soft edge | Separate cards, thin semantic borders, subtle 1pt drop shadow | 76pt rows, 16pt horizontal inset, 8pt gaps, 10pt corners | Trailing horizontal progress; flush right sheet |
| B · Quiet line | Continuous divided rows without card shadows | 56pt rows, 12pt horizontal inset, no gaps, 4pt control corners | Trailing progress ring; flush right sheet |
| C · Inset panel | One shared surface with internal rules | 80pt rows, 16pt horizontal inset, 12pt group corners | Horizontal progress; sheet inset 12pt with 12pt corners |

The content inset is 32pt for the card/panel studies and 20pt for the minimal direction and narrow windows. The minimal list starts 16pt below the toolbar, reduced from 28pt. Controls use Rune-kit's focus ring and input behavior, with a 2pt blue ring for custom sidebar navigation. Reduced motion follows the OS and can also be enabled in the study. The proposed sheet uses Rune's modal stack for focus containment, background inertness, dismissal and focus restoration.

## Captures

Same fixtures and renderer across 1180×760 and 600×720 viewports, at scale 1. Native window controls are not composited into these headless captures.

The [tighter minimal preview](evidence/visual/minimal-light.png) opens with `--scene minimal`. The Quiet line captures below also use the reduced spacing; [its previous spacing](evidence/visual/list-before-density.png) is retained for comparison.

| Treatment | Light | Dark | Narrow light | Narrow dark | Light sheet | Dark sheet |
| --- | --- | --- | --- | --- | --- | --- |
| A | [View](evidence/visual/cards-light.png) | [View](evidence/visual/cards-dark.png) | [View](evidence/visual/cards-light-narrow.png) | [View](evidence/visual/cards-dark-narrow.png) | [View](evidence/visual/cards-light-sheet.png) | [View](evidence/visual/cards-dark-sheet.png) |
| B | [View](evidence/visual/list-light.png) | [View](evidence/visual/list-dark.png) | [View](evidence/visual/list-light-narrow.png) | [View](evidence/visual/list-dark-narrow.png) | [View](evidence/visual/list-light-sheet.png) | [View](evidence/visual/list-dark-sheet.png) |
| C | [View](evidence/visual/panels-light.png) | [View](evidence/visual/panels-dark.png) | [View](evidence/visual/panels-light-narrow.png) | [View](evidence/visual/panels-dark-narrow.png) | [View](evidence/visual/panels-light-sheet.png) | [View](evidence/visual/panels-dark-sheet.png) |

Other states: [single Studio folder](evidence/visual/single-folder.png), [narrow sidebar](evidence/visual/narrow-sidebar.png), [paused/minimal/reduced/dark controls](evidence/visual/controls.png).

The [keyboard focus capture](evidence/visual/focus.png) shows the proposed ring; the scripted modal journey also tabs through the sheet and checks containment.

## Motion

Recordings are seven seconds at 15 fps, from Rune's controlled clock and native renderer:

- [Effects](evidence/visual/effects.mp4): determinate sheen travels right to left toward the fill on a 6s loop, indeterminate sheen left to right on a 2.6s loop, and seeding's text mask rises on a 4s loop. Only the empty track shimmers. The bright seeding copy counter-moves to remain registered with its base glyphs.
- [Sheet](evidence/visual/sheet-motion.mp4): opens at 0.4s, pauses at 1.4s and closes at 2.2s. The background blurs and the list retains its width. Entry/exit use Rune-kit's overlay spring with a right-edge transition.
- [Reduced motion](evidence/visual/reduced.mp4): enabled at 0.1s; shimmer nodes disappear and the renderer settles. Text/status changes remain readable.

The [measured samples](evidence/visual/motion-summary.json) record rendered positions at 0.6s, 1.2s and 2s. The capture recipe asserts sweep directions, unchanged fill geometry, stationary copied glyphs, and settled reduced motion. These timings start from [the CSS baseline](baseline.md); the prototype uses Rune's loop easing and spring transitions, not pixel-equivalent CSS curves. Circular sheen and newly witnessed completion choreography remain production-effect work, not claims of this prototype.

## Reproduce evidence

```sh
python3 prototypes/rune-visual/capture.py
cargo check --locked --manifest-path prototypes/rune-visual/Cargo.toml --target-dir target/rune-preview -j 2
cargo fmt --manifest-path prototypes/rune-visual/Cargo.toml --check

# Native submissions; exits after 120 frames.
Scripts/rune-visual.sh --native --trace .build/rune-visual/native.jsonl --frames 120
```

The recipe checks every treatment in both appearances and widths, sheet geometry/background inertness, consumed outside dismissal, Escape/focus restoration, search/keyboard behavior, transfer-state cycling, and single/multiple-folder transitions. Generated trees, scripts and traces remain in `.build/rune-visual/evidence`; selected PNGs, MP4s and motion samples are retained here. This disposable recipe is not the production test suite.

## Verification and limits

Checked on macOS 26.2 arm64 with Rust 1.99.0. The revised native window completed 120 submissions with zero failed surface acquisitions. This run used system traffic-light placement. Explicit placement at `(16, 22)` reproducibly asserted in AppKit's `NSDP_getComputedPropertyValue`; reverting only that option allowed the same view and dependencies to run. The prototype leaves the pinned framework unchanged. Native titlebar placement needs its own review before production use.

The native debug run shared the machine with the full browser suite. Presentation interval p50 was 16.67ms and p95 66.67ms; this is launch/submission evidence, not a performance result. New prototype Intel, macOS 13, packaging, signing, VoiceOver and actual Download journeys have not been checked. #100's separate architecture evidence does not establish this prototype's coverage.

`vp check`, TypeScript checking, the production frontend build, all 107 frontend unit tests, and all 248 Rust tests passed, with two existing ignored desktop tests. The first Rust build exhausted disk space; the failed cache was cleared and the subsequent complete run passed. No production source or existing assertions changed.

The single-worker full WebKit run passed 272/276 in 8.8 minutes. Four cases failed with missing/detached controls or navigation interruptions: custom-treatment dragging, icon-picker search/save, custom workspace directory naming, and creation after a lost response. All four passed together in an isolated 14.8s rerun. This is not a fully green full-suite result. No browser test or production code was changed to obtain the rerun result; #118 retains the suite-stability gate.

```sh
CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 \
  CARGO_PROFILE_TEST_DEBUG=0 cargo test --workspace -- --test-threads=2
vp check
vp exec tsc --noEmit
vp test
vp run build
OPUL_TEST_PORT=1561 vp run test --project=webkit --workers=1 \
  --output=.build/rune-visual/webkit
OPUL_TEST_PORT=1562 vp run test --project=webkit \
  tests/ui/custom-treatment.spec.ts tests/ui/icon-picker.spec.ts \
  tests/ui/workspace-creation.spec.ts \
  --grep='dragging updates only|search previews locally|a custom workspace folder|creation saves emoji' \
  --workers=1 --output=.build/rune-visual/webkit-focused
```

Independent Standards and Spec reviews found no implementation issues. The Spec review records one intentional pending item: the maintainer's design choice. The prototype's own renderer recipe, motion assertions, modal keyboard checks, and native launch passed.

## Decision record

The initial options were rejected as ugly and insufficient for a choice. The replacement restores Opul's toolbar, artwork, sidebar and details hierarchy. The maintainer now prefers the minimal direction as a visual direction and requested less space around list items. This revision tightens row height, horizontal padding, artwork size, text gaps and the list's outer inset. The spacing revision and remaining typography, radii, focus and motion decisions still await review; #101 stays open.

Once a treatment or a mix is selected, record the exact decisions and rationale here and on #101. Critical unresolved designs return to this native playground. Keep all alternatives disposable on `integration/99-rune-redesign`; rebuild the selected treatment as attributed, owned, tested production components.
