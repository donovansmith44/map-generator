# Retained Renderer (Spec Stages 2–5) — State & Execution Plan

## Preservation of the Canaan / 12-tribes work (2026-08-30)

- **Every input is committed**: `git status data` is clean (only `data/canon/`
  is gitignored, as an artifact).
- **The canon reproduces byte-identically**: `cargo run --release -p
  map-compile -- build --out <tmp>` produced an 81,331,146-byte canon.json
  exactly equal (`cmp`) to the live `data/canon/canon.json`. Loss of the
  artifact costs one rebuild command, nothing more.
- **The 16 canonical plates are regression-pinned**: rendered via `map-cli
  plates` before and after the stage-5 changes — byte-identical (the SVG
  path is untouched by the retained-renderer work).
- The loose repo-root PNG "10 Canaan Before the Conquest of Joshua.png" is a
  © 2020 Knowing the Bible *reference* image (third-party), not project
  output — left on disk, deliberately not committed.

## Stage 5 (fills) — decision record

Fills are realized on the GPU with the stencil even-odd parity technique
over the already-retained ring resources, NOT server-side triangulation:
even-odd is the fill law the SVG encoder already declares
(`fill-rule="evenodd"`), so both renderers share one semantics, and
LOD-simplified real-world rings (which can self-intersect) render
identically instead of feeding a fragile ear-clipper. The manifest now
carries honest `fill` styles per region (one feature key across a region's
rings) and marks the whole-sphere sentinel via the existing `covers_sphere`
law. A region's fill draws only when every ring is resident (§40).
Server-side triangulation remains the stage-7 route if refinement
correspondences need it.

> Implements `docs/superpowers/specs/2026-08-30-refactor.md` (stages 2–4 of its
> own migration strategy). Written mid-session to checkpoint state.

**Goal:** The retained-scene protocol end-to-end: `GpuSceneEncoder` (manifest +
content-addressed binary geometry), demand-driven `/api/scene` + `/api/resource`
routes, and a client WebGL2 layer with a ResourceCache that projects resident
unit-sphere geometry via camera uniforms every frame.

## Done (verified)

- **Stage 2 — `crates/map-encoders/src/gpu.rs`** (new): `GpuSceneEncoder`,
  `SceneManifest`/`FeatureInstance`/`ResourceDescriptor`/`GeometryResource`,
  `MGR1` binary packet (§63: LE header, f32 xyz unit-sphere verts, spherical-cap
  bounds). Content dedup (§I5), geometry id independent of style/time (§R8).
  6 new tests in `crates/map-encoders/src/tests.rs` — all 27 pass.
- **Stage 3 server — `crates/map-viewer/src/lib.rs`**: extracted
  `composed_scene()` (pieces/multi-subject/bible/ghost pipeline, shared by
  `/api/render` and `/api/scene`); `route()` now returns `Vec<u8>` bodies
  (binary transport); `/api/scene` returns manifest JSON + publishes payloads
  into `App.resources` (Mutex<BTreeMap<u64, Vec<u8>>>); `/api/resource?id=HEX`
  serves them. `map-cli.rs` updated for byte bodies. Viewer tests pass.
  Verified live on :8090 with curl: manifest (948 features / 909 resources /
  1.3 MB), binary packet decodes (magic, ids, unit-norm verts), 404 on unknown.
- **Stages 3–4 client — `crates/map-viewer/src/page.html`**: `gpu` module —
  ResourceCache (§11/§14 states), bounded-concurrency acquisition loop (§26),
  WebGL2 renderer (vertex-shader orthographic + equirect projection mirroring
  the server math, limb discard, uniforms-only camera updates §R1/§R2),
  `state.cameraNow()` exposing the continuous camera, "webgl borders
  (retained)" checkbox, `gpuSync()` after every full render. Draw list =
  boundary strokes + markers (stage 4 representative layer); ring resources
  ship but are not fetched (demand-driven §12).
- Viewer running detached on :8090 (release build with all the above).

## Remaining

1. **Browser verification** (delegate to a lesser-model subagent): drive
   Chromium (playwright cache has chromium-1234; MCP chrome channel is
   missing — use `npx playwright` script or the chromium exe via CDP) against
   http://127.0.0.1:8090/ — enable the "webgl borders (retained)" checkbox,
   confirm: no console errors, canvas.gl present, GL border lines visually
   overlay the SVG borders (alignment = projection correctness), both
   projections, and a wheel-zoom leaves the GL layer live. Screenshot evidence.
2. **Fix** anything the verification finds (Fable).
3. Full workspace `cargo test` sweep.
4. Commit in repo voice (subject line style: declarative law sentence).
5. Final report: what landed, how to run, and the spec problems found (portrait
   bitmap vs §27/Stage 1 conflict; 64-bit hash stand-in vs true content
   addressing; hand-rolled HTTP server had no binary path; GL line-width/dash
   limitations; stages 5–11 out of scope: GPU fills/tessellation, glyph
   atlas text, refinement correspondences, temporal transitions, eviction).

## Spec problems to report (noted while implementing)

- §27/Stage 1 orders bitmap-snapshot removal, but commits 9ac26dd/ff6f57c
  built the "portrait" canvas deliberately with measured wins (150 ms style
  flips, 133 ms visibility recalcs avoided). Removing it before the GPU path
  covers all layers regresses measured jank. Kept; retire at Stage 11.
- Spec assumes true content hashes; the codebase's ContentHash is a 64-bit
  DefaultHasher stand-in (fine at this scale, disclosed).
- Spec's binary transport (§63) collided with the String-only HTTP server —
  fixed as part of Stage 3.
- §67 (no tuned constants) vs. practical client values (acquisition
  concurrency 6, GL line-width clamp) — declared in code comments.
