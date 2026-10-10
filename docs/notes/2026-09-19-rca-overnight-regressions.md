# RCA — the overnight regressions of 6e349a3

Commit `6e349a3` ("A stretch is a stretch, two regions that meet differ, and
the zoom is the dial") was rolled off `master` on 2026-09-19 and preserved on
branch `sweep/2026-09-19-naming-colour-rulings`. `master` is back at `91d089c`.

This is the sweep the owner asked for: what actually caused each reported
symptom, measured rather than guessed, and which parts of the commit are
sound enough to bring back.

## Reported

1. At AD 59 Judea sits EAST of the Jordan and the Dead Sea.
2. Panning at AD 59, Rome flips pink/purple unpredictably and changes size.
3. Zoomed fully in, panning is very slow (the bug we thought we fixed).
4. Pieces visible in bible mode go invisible outside it.
5. Zooming in changes colours and causes reloads.
6. Regions that leave the view and return show nothing, then re-render.

## Root cause 1 — a quadratic geometric test on the request path

`repaired_slots_at` is called from `scene_at`, which means **once per scene
request**. `6e349a3` put `outlines_meet` inside its adjacency computation: an
all-pairs loop over every painted entity at that moment, each pair running
point-in-ring on real geometry.

Measured on the same canon, same requests, the only difference being whether
the overlap half runs:

| year | with overlap | without | painted regions in view |
|---|---|---|---|
| -3500 | 220 ms | 133 ms | 19 |
| -2000 | 676 ms | 119 ms | 43 |
| -1405 | 979 ms | 133 ms | 46 |
| -722 | 788 ms | 142 ms | 34 |
| 59 | 244 ms | 121 ms | 23 |

Without it the cost is flat at ~120 ms. With it the cost scales with how many
regions exist at that moment, up to **7x slower and approaching a second**.

The page re-demands a manifest on every pan grid step and every zoom change.
At a second per request the page spends most of an interaction between
scenes, which is exactly symptoms 3, 5 and 6 — and symptom 2, because a
half-loaded scene genuinely does show a region at the wrong size or missing,
so its neighbour's colour shows through where you expected it.

**This one cause explains four of the six reports.** It is mine, and it was
avoidable: I added a geometric all-pairs test to a function I had not checked
was on the request path.

## Root cause 2 — R84's refusal breaks the page's own detail control

`page.html` unconditionally sends `lod=` from a `<select>` offering four
values:

```html
<option value="auto" selected>auto (match zoom)</option>
<option value="0">exact</option>
<option value="0.0015">medium</option>
<option value="0.008">coarse</option>
```

`6e349a3` refuses any `lod=` that is not `auto`. Measured against the running
server at that commit: `auto` → 200, and `0`, `0.0015`, `0.008` → **400 and a
blank map**. Three of the four options in the shipped control were dead.

R84 is right that fineness should follow zoom. Retiring the parameter without
a path for the one client that sends it is what broke. The page is frozen and
cannot be edited, so the retirement needs either a grace period where a
non-`auto` `lod` is ignored rather than refused, or a coordinated page change
that is not mine to make.

A related inconsistency in the same commit: `/api/transition` still calls
`lod_of`, which now silently ignores an explicit `lod` instead of refusing it.
Two routes, two behaviours, for the same parameter.

## Root cause 3 — the colour change is real and visible

Adding overlap to adjacency reassigns palette slots. The golden gate at
`6e349a3` showed **10 drifting probes** across both beloved stops; nine were
regions that had been teal (`0,154,161`) and became olive (`112,120,20`).
Both are palette entries, so this is slot reassignment, not a rendering
fault — but the beloved maps genuinely look different, and `golden-views.json`
was deliberately not re-blessed.

After rollback the gate is back to its single long-standing drift
(`-1446 levant[12]`, `158,157,147`), the same values as before the night.

## NOT regressions — checked and cleared

**Judea east of the Jordan (symptom 1) is upstream data.** The source file
`data/historical-basemaps/world_bc1.geojson` has `Judea` spanning
lon 34.29–38.13. The server draws two rings for `basemap:judea`: one at
lon 34.42–35.55 (the expected western one) and one at 34.30–38.12 (the
upstream eastern one). Nothing in `6e349a3` touched geometry. What changed is
that the colour reassignment made the eastern ring conspicuous, and the
registry unification of `authored:judea` into `basemap:judea` (landed
2026-09-16, `ea78a12`) draws both under one name and one colour.

This is worth its own decision: the basemap's Judea is wrong for the atlas's
purposes and is being unified with the authored one.

**Bible mode is a strict subset (symptom 4).** Measured at AD 59 over the
Levant: 1413 entries in bible mode, 1415 outside, and **zero** entries present
in bible mode but absent outside. The reported symptom is visual, and the
most likely reading is root cause 1 — outside bible mode there is more to
load, so the half-loaded window is longer.

**Colour is stable per camera and per year.** Rome is `133,77,156` at every
camera tested at AD 59, and across years 55, 58, 59, 60, 62, 70. The palette
depends on the moment alone, not the view. The flipping the owner saw is
loading, not assignment.

**Same-colour overlaps at AD 59 Levant: none.** Checked all 27 visible fill
entries pairwise.

## What is sound in the commit

- The `Stretch`/`outline` rename. Pure renaming; the wire and the on-disk
  canon keys are untouched; the compiler verified every site.
- `/api/subjects` span support (R85). Additive, ~40 ms for the whole
  timeline, and the page does not even use it yet.
- The unknown-projection refusal (R87). The page sends only what `/api/meta`
  declares, so it cannot trip it.
- The runner's report widening (160 → 700 chars) and the `meetingRegionPairs`
  precompute. Test-side only, no server effect.

## Bringing it back

Suggested order, each verified on its own before the next:

1. The rename alone. No runtime behaviour, nothing to measure but the build.
2. R85 spans and R87's refusal. Both additive and cheap.
3. The colour fix, but with the adjacency computed **once per canon**, not per
   request, the way `palette_slots` already is — and with the golden gate run
   and the new colours blessed deliberately.
4. R84 last, with a decision about the page's `lod` control first.

## What I should have done

Timed a scene request before and after touching the palette path. The
measurement that found this took two minutes and would have caught it before
the commit rather than after the owner did.
