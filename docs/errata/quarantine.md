# X0 source quarantine

The owner's permissive-source rule excludes NonCommercial, ShareAlike,
copyleft and unlicensed geography. Removing a drawing does not disprove its
biblical place or holding. Missing permitted geometry remains unrecorded and
undrawn; it never means unclaimed. No golden baseline is re-blessed.

Evidence: atlas MAPS migration design §5.9 and §8.2; golden 1446 specification
B9; the removed `data/wikimedia/LICENSE.md`; and the retained
`data/openbible/LICENSE.md`. These are the project's recorded source findings,
not a new attribution or a new licence grant.

## Excluded sources and descendants

| Removal | Licence or provenance reason | Replacement |
|---|---|---|
| `PLATE-CANAAN`, its 332 contour waypoints, `plate_canaan_ring` in `crates/map-adapters/src/surveys.rs` | Traced from Cory Baugher / Knowing the Bible LLC, *Canaan Before the Conquest of Joshua*, ©2020. Publisher permits personal, teaching and non-commercial use only. Authored stand-in provenance did not make the tracing CC0. | Canaan evidence from Gen 10:19 and permitted, justified geometry through X16; no substitute Canaan ring in X0. NUM 34's separate Scripture survey remains. |
| `crates/map-adapters/src/plate_water.rs`, its module and exports | Water polygons, rivers and chart calibration compiled from the same excluded plate. This module was compiled even though the main compiler had stopped drawing its water. | Existing Natural Earth water remains; X1 owns permitted river replacement. No plate-derived water remains compiled. |
| `data/wikimedia/tribes12.geojson` and `data/wikimedia/LICENSE.md` | Wikimedia Commons *12 Tribes of Israel Map.svg*, derived from *12 tribus de Israel.svg*, CC BY-SA 3.0. Raster tracing, georeferencing, Jordan splitting and shoreline splicing retain that ancestry. | Scripture evidence and surveys (Josh 13–19) in X3, with permitted Rawson 1873 geometry through X16. No tribal ring replacement in X0. |
| `data/openbible/regions.geojson` | Upstream OpenBible Bible-Geocoding-Data is CC BY 4.0, but this local derivative spliced its six outlines onto CC BY-SA tribes12 rings. The permissive upstream licence does not erase the excluded ancestor. | Re-derive from OpenBible alone with complete permitted lineage; nothing yet. Philistia, Phoenicia, Geshur, Ammon, Moab and Edom remain recorded identities, without these polygon witnesses. |
| Excluded region loaders, presence declarations and snapping in `crates/map-compile/src/partition_bridge.rs` | These paths admitted the plate, tribes12 and spliced-region descendants into one shared arrangement. Every partition-derived face could consequently inherit their geometry. | Build the same spherical partition from retained physical and supplied polity witnesses. No guessed holding or replacement border. |
| Plate and tribes exports in `crates/map-adapters/src/lib.rs`; unused era resolver in `crates/map-compile/src/main.rs` | Entry points and the era resolver existed to admit the removed witnesses. | Removed together with their consumers. |
| Partition source-family inventory in `crates/map-canon/src/lib.rs` | Previously claimed Authored, OpenBible and Wikimedia as arrangement inputs. Those loaders no longer supply the arrangement. | Exactly Atlas, NaturalEarth and Osm. Standalone survey and settlement provenance remains unchanged. This is not a whole-pipeline permissive-source claim. |

## Removed tools

Every file below lived under `tools/plate_trace/`; the directory is removed.
Removing a tool does not itself exclude every dataset that tool once produced:
actual source ancestry decides. No replacement tool is invented in X0.

| File | Reason | Replacement |
|---|---|---|
| `calibrate.py` | Detected reference-plate city dots and fitted its tracing chart. | Permitted-source control points in X16. |
| `trace_green.py` | Traced the excluded plate's Canaan colour mask. | X16 permitted geometry; none in X0. |
| `emit_plate.py` | Emitted the excluded Canaan contour as Rust waypoints. | Scripture evidence in X3/X16; none in X0. |
| `trace_water.py` | Traced excluded plate water. | Existing Natural Earth water. |
| `trace_water2.py` | Traced excluded plate water and river strokes. | Natural Earth and justified CC0 river work in X1/X16. |
| `emit_water.py` | Emitted excluded water into Rust source. | Existing Natural Earth adapter. |
| `emit_water2.py` | Emitted excluded plate water and river geometry. | Natural Earth and justified CC0 river work in X1/X16. |
| `overlay.py` | Compared the plate-derived tracing against the excluded reference. | Permitted-source golden checks; no new tool in X0. |
| `vendor_tribes12.py` | Raster-traced and spliced the CC BY-SA map. | X16's permitted-source geometry. |
| `vendor_openbible.py` | Spliced otherwise permitted regions onto tribes12. | Unspliced OpenBible derivation; none yet. |
| `vendor_osm_rivers.py` | Produced ODbL river geometry; this plate-tool directory belongs to X0. | X1 Natural Earth replacement. Existing OSM data removal belongs to X1. |
| `vendor_jordan_corridor.py` | Produced an ODbL corridor from OSM through the chart. | X1 permitted river geometry; no replacement corridor in X0. |
| `vendor_med.py` | Rasterized Natural Earth land through the working chart, rather than tracing plate water. Removed as part of the directory ownership. | Retained NE-derived `med_clip.geojson`; future native-course ingestion is X16. |
| `vendor_settlements.py` | Legacy atlas-coordinate/OpenBible-type join, not a plate-geometry trace. Removed as part of the directory ownership. | Retained `settlements.geojson`; current atlas identification producer owns future replacements. |

## Retained inputs and limits

Natural Earth's native lakes and its land-complement Mediterranean clip remain:
the latter's recorded source is `ne_10m_land`, not the plate's painted coast.
The chart is a coordinate transformation used by the old tool, not a new grant
of a licence to copied plate geometry. Settlements retain atlas coordinates and
OpenBible typing, not coordinates traced from plate dots.

OSM rivers and their OSM-derived corridor still enter the partition. X1 removes
them. Historical basemaps and the conservative Atlas-source licence remain for
X2 and the separately assigned atlas producers. X0 excludes the three source
families assigned to it; it does not approve the remaining pipeline for release.
The stale local OpenBible licence note is kept as evidence of the quarantined
splice, not as a statement that `regions.geojson` still exists.

Persisted legacy canon files outside this worktree are not changed. They must
be regenerated from the cleaned source branch before serving it. Existing
licence vocabulary remains able to describe legacy artifacts honestly.

## Executable content guard (reviewer 2 repair)

`data/authored/excluded-geometry-fingerprints.json` records107 excluded
geometries from6ac32bfbf67e26b9cfe94560806fda293db05801 with source family,
original path and source SHA-256. The shapes themselves remain removed.
`tools/quarantine_fingerprints.py --check` reproduces that catalogue.
Compiler admission checks every supplied polity and timeline boundary,
all repository GeoJSON inputs, and all compiled border outputs. Content
matching uses the partition's existing quantized Cartesian point keys
hashed with SHA-256; unions preserve excluded vertex sets across renamed,
reordered, nested and split inputs/outputs.

This replaces the former biblical-ID ban. A permitted future Judah or Canaan
geometry can be admitted under its biblical identity. The guard records
actual matching content with all excluded-source provenance; it does not
assert missing geography is unclaimed, grant a licence, or infer the ancestry
of arbitrarily changed coordinates without retained derivation records.
The general permissive-lineage policy remains C4's responsibility.
