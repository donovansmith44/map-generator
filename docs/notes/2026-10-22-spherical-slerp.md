# Spherical GPU interpolation proof — 2026-10-22

## Survey and repair

The canonical kernel is `crates/map-types/src/geom.rs::slerp`: angular interpolation on the unit sphere; it rejects exact antipodes because they have no unique path. The renderer survey found two chord-normalization sites: the native page vertex shader and the transition target vertex shader. No importable GLSL slerp helper existed at either site, so both now insert the single GLSL source from `crates/map-viewer/src/retained-gpu.js`. This GPU patch adds no admission policy and does not prove that production excludes exact antipodes. The client owns only GPU presentation, with no second domain tessellator, f64 buffer, or antipodal fallback.

## Red reproduction and custody

The unchanged RED proof is `/home/donovan/mut/evidence/SPHERICAL-COMPILE-PROOF/run-quarter-proof.sh`, with pinned renderer revision `8fa4ce9e23254b2c9fcc517a8ae30f1e712c1265` and fixture SHA-256 `4830ac6d95d73b91d9c0406a0e26a5d243cc9f0c6bf6f77abe674c98fcc32688`. Its seven cases and 37 probes failed at fractions 0.25 and 0.75: 4,056 and 3,416 differing pixels, plus two containment errors. The original evidence and fixtures remain unchanged.

A private copy at `/home/donovan/mut/evidence/SPHERICAL-SLERP-GREEN/` applied only this branch's shared GLSL patch to the pinned native source and updated that copy's source hash manifest. Its `browser-result.json` passed seven cases, 37 probes, and fractions 0, 0.25, 0.5, 0.75, 1 with zero failures: each frame matched the cold canonical spherical frame and inverse raster. The fixture SHA-256 stayed identical. `custody-receipt.json` records the original and patched manifest hashes, fixture equality, RED and GREEN result hashes, and browser log hashes.

## Gates

- Focused SwiftShader transform-feedback edge test passed: quarter angles, endpoints, coincident and near-coincident points, representable near-antipodes, and inverse midpoint.
- Existing renderer JS gate passed 30 non-browser tests; five browser launches initially failed because Playwright 1.56.1 requested absent cached browser revisions. With Playwright 1.62.1 and the project WebKit wrapper, the retained GPU, label, and spherical browser tests passed 5/5 across Chromium and WebKit.
- The actual `transition-target.browser.test.mjs` gate passed 14/14 in Chromium and WebKit (Playwright 1.62.1, cached Chromium 1234, project WebKit wrapper 2336). Inputs were the unchanged served collection and Y20/water pages under `/home/donovan/mut/evidence/SHARED-OUTLINES/source-viewer-*.json`; the collection includes two multipolygons with holes and 221 anchored labels. A local ignored copy of the installed MapLibre package supplied the browser module.

The exact passing browser logs are `/home/donovan/mut/evidence/SPHERICAL-SLERP-GREEN/transition-target-browser.log` and `/home/donovan/mut/evidence/SPHERICAL-SLERP-GREEN/retained-browser.log`. The initial browser revision mismatch is preserved at `/tmp/spherical-renderer-gate.Y0M6N6.log`.

Served fixture SHA-256 pins: `source-viewer-collection.json` 3602a5269baac4899ab47cfa7ed78c9eb320e81125176c80eecb18a962bef393; `source-viewer-y20-page.json` c582bb0351f6261219e700313b2a38e0903b59514680e365f54ff640cadb3516; `source-viewer-water-page.json` 773ab482c238afdeb91c899e90ee2526fce7d756d8e5f8b888372921c9f865f4; `source-viewer-water-reverse-page.json` a6ba43e55ea46a2d4d84e56d5c213a649bfd9abc44c5c91d9a16aee102aef32e.

## Reproduction setup and scope

Run Node from a directory with `playwright-core` installed. Set `MG_TRANSITION_FIXTURE`, `MG_TRANSITION_Y20_FIXTURE`, `MG_TRANSITION_WATER_FIXTURE`, and `MG_TRANSITION_WATER_REVERSE_FIXTURE` to the collection, Y20, water, and water-reverse files named above; set `FSHARP_WEBKIT_EXECUTABLE=/home/donovan/.local/share/bible-atlas-webkit-deps/bin/webkit` and `GIO_USE_PROXY_RESOLVER=dummy`. Select the cached Chromium executable through a private Playwright launch override, then run `node --test crates/map-viewer/tests/transition-target.browser.test.mjs`. The native proof copy and launch override are private test setup, not tracked application code. Full workspace, contract regeneration, and paired production browser gates belong to the landing batch.
