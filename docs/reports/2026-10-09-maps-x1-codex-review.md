# MAPS-X1 second Codex review

**Verdict: CHANGES REQUESTED.** Three Important findings, no new Critical finding.

Reviewed `origin/lane/claude/MAPS-X0..origin/lane/claude/MAPS-X1`, exact
`7bcbabab132443e535fa7b18c2cdb4500457e3c8..cf3c348d92068a921dac2b8c6c1d61573f3ee3a2`.
Review branch `lane/codex/MAPS-X1-review` was created from that head.
MG-BASE exists at `6ac32bfbf67e26b9cfe94560806fda293db05801`; no fallback used.
The X1 queue row, C12 migration plan and both golden specifications at atlas
`075c0898`, handoff §A, PRINCIPLES and the earlier emphasis/local-ID reviews
govern this pass. Only Important/Critical issues are reported.

## F-292 — Important: the no-OSM law checks paths, while renamed OSM courses are admitted

`crates/map-adapters/src/tests.rs:680–696` checks two source files for
`data/osm` and checks that directory's absence. X0's inherited content guard
does run before the real partition loader, but its catalogue and closed
`ExcludedSource` vocabulary contain only KnowingTheBible, Tribes12 and
SplicedRegions. None of the removed OSM river content is represented.

Independent diagnostic: read the longest historical River Jordan line from
MG-BASE, preserve its geometry, replace only its properties with a new name,
NE river number 229 and course class River, and place it under the normal
NE river filename in a temporary input directory. The content guard returns
`Ok(())`. The real `gather_witnesses` loader also succeeds, admitting one
`renamed-permitted-looking-course-1` witness with **96 OSM vertices**. Both
expected-refusal probes fail. The genuine NE Jordan control succeeds with
five clipped paths. All existing source/exclusion tests pass in the same run.

This demonstrates an open admission category, not contamination of today's
retained NE file or of a served canon. No excluded source bytes were restored
in the worktree, committed as fixtures, or passed through a full canon build.

**Closure:** X1 owns extending the existing X0 content-admission abstraction
to the removed river vendor and its corridor descendants, coordinated with
the single catalogue/source-inventory writer. Generated controls restore,
rename, split and indirectly carry excluded river content through actual
nonempty inputs and assert complete typed refusals; permitted NE controls
must still pass. Fence actual outputs as well as input filenames. Do not add
a competing licensing policy: C4 retains general permissive-source and
coordinate-changing ancestry admission. Consult before shared type changes.
The related X0 F-283 repair covers controls for its three source families;
its remaining partial-descendant gap and F-287 point-key ownership stay
with X0. X1 adds the removed OSM family through that same door, without
declaring those existing findings closed.

## F-293 — Important: the purported complete golden-river census omits world requirements

`docs/errata/rivers.md` lists 18 golden river identities, repeated manually in
`tests.rs:783–801`. Neither the census nor its passing law covers the world
requirements in the 1446 specification: **Diyala** (§4.3, lines 214/217),
**Yellow River** (Shang, line 236), and **Gan** (Wucheng, line 238).
The source-reading contexts also name **Gadar**, **Nam** and **Sangha**
(lines 496/499/508), without a recorded census disposition.

The retained NE source has candidate courses named Huang, Gan and Sangha;
the other three names require an identification search or an explicit
missing-course record. A spelling match alone does not establish the chosen
identity or the ancient course. These omissions escaped because the law's
input inventory is a second handwritten subset of the spec.

**Closure:** the X1 census owner records every named world and Canaan river
from both golden specifications, including alternatives and contextual
mentions with their precise role. Carry the selected NE identity/course or
an explicit missing disposition, plus admission-window and modern-course
limits. Derive the coverage check from one requirements inventory tied to
the existing golden authority, rather than restating an 18-name list in
tests. This requires no frame expansion, invented course, new holding,
chronology change or golden blessing. X16 retains later geometry completion.

## F-294 — Important: new river runtime laws use fixed examples instead of generated input

`tests.rs:827–859` calls its law generated, but executes a fixed seven-by-five
coordinate grid and two fixed geometry shapes. `tests.rs:863–915` enumerates
fixed malformed positions, geometry classes, path lengths and course classes.
These are useful examples, not generated properties under the owner's bar.
There are no generated names, river identities, arbitrary multipart/path
lengths, document failures or per-field admission refusals for the new door.

**Closure:** use maintained generated-property tooling for the adapter's
valid and invalid source documents, identity/name/class admission, multipart
fidelity and unlocated/refusal behavior. Preserve the present examples as
seeds, complete result/refusal oracles, one fact and a plain message per
assertion. Keep real-data census/source inventories as structural checks.
Coordinate dependency edits through the reserved manifest writer; do not
hand-roll a generator or a second decoder. X1 owns these river laws; X0's
Proptest laws do not exercise this adapter vocabulary.

## Accepted evidence and existing owners

The current OSM vendor and loaders are removed. The retained NE file's
SHA256 matches the report, and independent counts confirm all 1,455 features
(1,202 River, 253 Lake Centerline). The complete removed-label table matches
**all 231 labels / 1,752 line features / 125 networks** exactly. Its Jordan
figures also match history: 38 named lines and 452 vertices, replaced by 161
NE source vertices across five parts. Missing names and the empty Loire
course remain explicit; the removed length/mouth cutoffs and buffered
corridors have no replacement styling trick. The census distinguishes
modern source courses from ancient evidence, and missing from absent.

D.R.Y., type/readability and category passes: RiverNumber, RiverCourse,
RiverShape and RiverError name the adapter vocabulary; no map-types or
graph-types change. The local dead provenance closure is removed. Whole-body
maintained tree-sitter audit: **6 touched functions / 18 assertions / zero
message-free assertions / zero tuple or Boolean-vector observations**.
Added application/test comments: zero. F-293 is the duplicated requirements
owner; F-292 is the incomplete admission family; F-294 is the runtime-law gap.

Preserve the already disclosed composite-credit gap with the single
source-inventory owner: `Witness::PARTITION_INPUTS` still declares Osm, and
its exact inventory/terms tests require it. X4 retains the stale Canaan
registry blocker. The plan's per-named-river `natural-earth:` canonical
identity step remains incomplete: this bridge retains `partition:jordan`
and `partition:rivers`; existing X2/X5 identity-producer ownership applies,
with X4 owning Registry admission. No duplicate finding number or side fix.
X5 also retains the general GeoJSON library migration. These dispositions
do not grant complete X1 acceptance or waive the binding plan.

## One gate packet and limits

Exactly **one Cargo invocation**, on the exact head plus a temporary review
test target: adapters 21, compiler unit 18, exclusion inventories 4, content
laws 7 = **50 existing tests passed / zero failed**, one intentionally ignored
X0 child. Review probes: **one permitted control passed / two refusal probes
failed**, one ignored subprocess entry. Aggregate exit **101**, solely the
diagnostic target. The temporary target was removed afterward; executable
probe source and compressed raw output are retained below. Standard Cargo
dev/test debug level 1 and nonincremental compilation used the mandated
shared `~/mut/target`, with no target cleanup or second Cargo run. Only
inherited unused imports appeared.

Atomic heavy-lock acquisition was refused by the foreign MAPS-C12 landing
holder. No full workspace or `make contract-gates` run was started. Worker
contract 381/0, semver 6/0 and totality/vocabulary outcomes are carried
evidence, not freshly repeated results. The worker narrates three historical
red-first checks in rivers.md; separate raw X1 red logs are not committed,
so their execution order was not independently replayed. Review diagnostics
establish the present negative result, not historical implementation order.

The exact `node crates/map-viewer/tests/golden.js --check` command exits 1
at missing `playwright-core`, before browser launch; 8090 has no listener.
No pixel difference is measured. Worker census **329 faces / 563 river paths
→ 314 / 5**, zero residual, and its traced no-OSM-path read are reported
compiler evidence, not an independent full build here. Expected golden
changes are honestly marked expected in rivers.md. Actual golden differences
remain a completion blocker. No golden file differs across this range and
none was re-blessed. No served-canon, whole-workspace, contract, browser,
mutation or integrated migration acceptance is claimed.

Evidence: [checksummed packet](evidence/2026-10-09-maps-x1-review/SHA256.json),
including diagnostic source, compressed Cargo/golden logs, exact inventory,
spec contexts, pinned AST audit and verification metadata. Report/evidence
only; no application, source-data, KJV, contract or fixture changes. No own
lock, server or running job remains. Repair on the existing X1 lane, then
submit its exact forward head for independent review and the blocked gates.
