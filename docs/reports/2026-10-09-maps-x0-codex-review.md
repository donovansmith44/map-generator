# MAPS-X0 second Codex review

**CHANGES REQUESTED** — Important F-283 and F-284. Reviewed exact range
`origin/lane/claude/MG-BASE` **6ac32bfbf67e26b9cfe94560806fda293db05801**
through **9cb7e9aa7ec8f5c6d11e8c710835f69c98a41861**. MG-BASE exists;
the CX-M0 fallback was not used. The report branch
`lane/codex/MAPS-X0-review` starts at that reviewed head.

Owner/controller authorization covers this second review. Applied the atlas
AGENTS/PRINCIPLES, map-system-handoff §A, the Lane B X0 ownership row, the
migration plan at `origin/lane/claude/MAPS-C12`, and the earlier A-EMPHASIS and
A-WIRE-LOCAL-IDS review bar, read from their retained local review refs.
Only Important/Critical findings are raised. No implementation repair,
source admission, type change, golden blessing or landing was performed.

## Accepted source-removal work

The removed plate contour, its 332 waypoints, `PLATE-CANAAN` survey,
`plate_canaan_ring`, compiled plate-water module/exports, tribal/spliced-region
loaders, presence declarations and unused snapping machinery have no remaining
build entry point. `data/wikimedia/`, the spliced OpenBible regions and every
file in `tools/plate_trace/` are deleted. The unused era resolver, unused
`Bundle.biggest` field and plate-only `CITY_NOTE` declaration are removed with
their consumers; the separate NUM 34 survey remains.

`docs/errata/quarantine.md` records each removal, its licensing or ancestry
reason and replacement. It distinguishes Scripture identities from excluded
drawings, preserves missing geometry as undrawn, and explicitly assigns OSM
and historical basemap work to X1/X2. Its tool inventory also identifies the
two producers whose data ancestry differs from the excluded tracing: the
NE-land-derived Mediterranean clip and atlas-coordinate/OpenBible-type
settlements remain unchanged. Inspection of the deleted producers supports
that distinction. The only deleted data files are the three listed excluded
files; no retained data byte changed in the reviewed range.

The retained partition inventory is Atlas, NaturalEarth and Osm; standalone
Authored/OpenBible provenance and legacy Witness/license vocabulary remain.
This is an X0 exclusion decision, not approval of the remaining ShareAlike,
ODbL or GPL pipeline. No KJV, contract/version, golden fixture, renderer or
map-types change is present. No added Rust application or test comment line
was found. Retiring assertions requiring excluded shapes is appropriate.

## F-283 — Important: the negative lineage law is an ID/path inventory

`crates/map-compile/tests/excluded_lineage.rs:9–59` asserts a manually declared
source-family constant, gathers with **no polity inputs**, then compares a
fixed list of region IDs. Lines 64–81 inspect two provenance/name substrings.
`excluded_bytes.rs:4–16` checks four paths. None inspects canon-row ancestry,
generates source/derivation graphs, or binds actual input bytes to the declared
lineage. Enumerating fixed names does not satisfy the generated-law ruling.

Independent negative control: restore the original removed Judah geometry
from the base commit into a **temporary** `data/natural-earth/med_clip.geojson`
input. The real `gather_witnesses` admits all **156 vertices** as
`great-sea-1`. All **four submitted exclusion tests still pass**. The real
loader uses the temporary input through its existing relative-path door;
the worktree's retained file and every deleted path remain unchanged.
This demonstrates an undetected reintroduced excluded descendant, not a
claim that this submitted head currently serves one. The blocked full canon
build was not bypassed.

The failed abstraction is input ancestry at compiler admission: the changed
source inventory and the admitted geometry are independent declarations.
The law also bans biblical IDs rather than excluded ancestry, so it cannot
become the rule for admitting future permitted replacements under those IDs.

**Closure, X0 owner:** make the exclusion law read the actual admitted lineage
through the compiler/canon boundary, using the existing authoritative source
decisions. Generate permitted and excluded ancestors, indirect derivations,
renamed descendants and nonempty input families; prove each excluded ancestor
is refused and permitted controls survive. Preserve named examples as seeds,
complete outcomes and one fact/plain message per assertion. Record executable
red-before-green evidence, including the reintroduced-byte control. Keep the
real-data/path inventory as supplemental checks. Coordinate any required
shared representation with X5/C4 and consult before map-types/graph-types
changes; do not introduce a competing licensing rule or weaken the live-home
registry check. C4 remains the owner of the general permissive licensing law.

## F-284 — Important: touched tests retain ten message-free assertions

The maintained tree-sitter Rust audit over the **whole changed test bodies**
finds **9 changed functions, 28 assertions, 10 without messages**. Nine are in
`crates/map-adapters/src/tests.rs`,
`promised_land_survey_is_lawful_alone_and_merged`, at
324, 328, 339, 340, 344, 357, 367, 377 and 378. The tenth is
`crates/map-canon/src/tests.rs:928`,
`the_share_alike_origins_are_named_not_inferred`.
These assertions are inherited inside bodies this packet touched; the earlier
review bar applies to the entire changed body, not only added assertion lines.

**Closure, X0 test owner:** give each assertion a plain behavior message,
preserving its complete observation and one fact per assertion. Re-run the
same body audit on the repaired range. The tuple-vector candidate at canon
tests line 902 is a legitimate whole source-to-license table, not bundled
independent Boolean facts; it is accepted after manual inspection.

## Existing blocker and ownership

The author and queue already record **X0-REGISTRY**, owned by X4:
`data/authored/registry.json` still requires canonical `partition:canaan`
after removal of its excluded witness. Fresh full compilation and exact
golden sampling remain blocked on that reconciliation. No duplicate F-number
or X0-side registry repair is requested. Preserve source decisions/provenance,
publish the id moves in X4, and never mint unsupported replacement geometry
or relax validation. Existing compiler unused-import/type/decode/producer
categories retain their recorded owners; no new category was introduced here.

## Verification and limits

Exactly **one scoped Cargo gate invocation** was made:
`. ~/.bible-atlas-env; CARGO_TARGET_DIR=~/mut/target nice -n 10 cargo test -j 4
-p map-adapters -p map-canon -p map-compile`.
It returned **101**, with **67 passes / 1 failure** before doctests: the
exclusion executable observed the old six-source constant despite this head's
three-source declaration. The canon unit executable also accepted its old
license expectation. These observations invalidate that cached run as a
clean-head verification claim.

Investigation used a **build-only** `cargo test --no-run` with
`CARGO_INCREMENTAL=0` in the same required shared target. Rebuilt exclusion
executables then passed **4/0** on the unchanged source head; their temporary
reintroduced-byte control also passed **4/0**, exposing F-283. The old/new
source-inventory behavior establishes an incremental artifact mismatch; its
full tooling cause was not investigated. No full gate was repeated. The
nonincremental canon executable lists **30 tests** (29 in tests.rs and one
persist test), so the author's reported 36 canon passes are carried evidence,
not an independently confirmed count for this head.

The heavy-lock take was rejected: `codex-maps-land2` holds it for MAPS-C12
landing gates. No foreign lock was changed. `cargo test --workspace` and
`make contract-gates` were not run by this reviewer. The author's scoped
Haskell/contract results and pre-removal red observations are reported
evidence; their raw logs were not supplied in this source range. Browser
goldens were not run because a fresh canon cannot be built through the known
X4 blocker. Baseline bytes are unchanged; no re-bless occurred. No server,
mutation gate or owner port was touched.

14b D.R.Y./type pass: the source-inventory/actual-input split and disconnected
ID vocabulary are F-283; no parallel replacement geometry/type was added.
24a/24b category pass: current forbidden loaders are all removed, but the
lineage category remains writable and the negative control escapes the law.
Assertion readability is F-284. No Critical finding is raised.

Evidence is in [the review directory](evidence/2026-10-09-maps-x0-review/):
checksummed input hashes, gate/build logs, diagnostic observations and probe,
and the pinned assertion audit. No excluded geometry fixture is committed.
Repair on the sole X0 author lane; re-review the exact repaired head before
landing. Integrated completion also waits on the separately owned X4 blocker
and unrun gates. No own lock, server or background job remains.

## Repair review: 9cb7e9a..7bcbaba (second Codex, 2026-10-09)

**CHANGES REQUESTED — F-283 remains open; new Important F-287. F-284 CLOSED.**
Reviewed repair **9cb7e9aa7ec8f5c6d11e8c710835f69c98a41861..7bcbabab132443e535fa7b18c2cdb4500457e3c8**
and whole item **6ac32bfbf67e26b9cfe94560806fda293db05801..7bcbabab132443e535fa7b18c2cdb4500457e3c8**.
`origin/lane/claude/MG-BASE` exists at 6ac32bf; no fallback was used. This
appendix supersedes the original findings' repair disposition. Applied the
same binding rules, current Lane B X0 row, C12 migration plan and retained
local A-EMPHASIS/A-WIRE-LOCAL-IDS review reports. The author head was checked
out temporarily in this review worktree; its source bytes were restored
before returning to the sole review branch. No author worktree was edited.

### Accepted removal and test repair

All previously reviewed excluded loaders, PLATE-CANAAN, plate-water code,
tribes12, spliced regions and plate tools remain removed. Quarantine records
each removal, its reason and replacement, and distinguishes excluded drawings
from recorded biblical identities. The repair deletes no additional data.
Independent whole-item comparison confirms **43 retained data files unchanged**;
the only data addition is the fingerprint catalogue. No KJV, map-types,
graph-types, contract, golden, baseline or fixture change occurs. The separate
NUM 34 survey, NE-derived Mediterranean input and atlas-coordinate settlements
remain. OSM/basemap and registry work retain X1/X2/X4 ownership; none is approved
here as a permissive source by implication.

**F-284 CLOSED:** replay of the pinned whole-body maintained tree-sitter audit
at the exact head gives **22 changed bodies / 47 assertions / 0 message-free**.
All ten reported inherited assertions now have a plain behavior message.
The complete source-to-license tuple-vector remains the already accepted
table observation. Manual inspection finds no combined independent Boolean
facts. No added application/test comment line is present in the whole Rust
diff. Independent decoding verifies all supplied evidence-file checksums;
the retained red restoration log records two behavioral failures before
wiring, with later green logs, rather than treating the initial compile
error as behavioral red evidence. Catalogue regeneration reports 107 shapes.

### F-283 residual — Important: dropping one vertex admits excluded ancestry

`crates/map-compile/src/exclusion.rs:50–62` refuses a source only when **every**
fingerprint of one complete historical geometry occurs in the observed union.
The generated property at `tests/exclusion_content.rs:234–267` rotates, reverses,
adds vertices and partitions the complete list into chunks, but retains every
original vertex. Thus its “split and indirect descendants” never exercise
clipping, selecting part of a ring or removing a vertex.

The original independent Judah restoration probe was rerun against this head:
the complete 156-vertex raw source in a temporary
`data/natural-earth/med_clip.geojson` is now refused with the complete expected
source/provenance outcome. **Accept that regression repair.**

The additional independent restoration route loads the same original Judah
ring from the pinned base, removes exactly one uniquely occurring vertex
(index 0), and supplies the other 155 unchanged coordinates through a renamed
`PolityRow`. Real `partition_bridge::gather_witnesses` returns success and admits
`review-renamed@-1000` with **all 155 vertices unchanged**. A whole-ring assertion
confirms that observation; the assertion demanding the excluded-source refusal
then fails with `left: None`. No coordinate was tuned, replaced or perturbed.
This is a retained-content descendant, so the report's limitation for arbitrary
coordinate-changing ancestry does not dispose it. No contaminated full canon
or served map is claimed.

**Closure remains X0's existing F-283:** preserve excluded ancestry through
actual source admission and derivation, including partial/clipped/simplified
descendants and nonempty compiler families; generate those operations with
complete refusal and permitted controls. Coordinate the authoritative lineage
representation with X5/C4, consult before shared type changes, and leave the
general licensing policy with C4. Complete-shape fingerprint checks can be
supplemental evidence; lowering a matching threshold or banning biblical IDs
would neither establish ancestry nor protect legitimate permitted geometry.
The original category stays open, so no duplicate F-number is assigned to it.

### F-287 — Important: point-key policy is copied into the new guard and producer

`crates/map-compile/src/exclusion.rs:154–159` reproduces the partition point-key
policy: multiply Cartesian components by 1e9, round to signed integers, encode
big-endian. It does not call an owning geometry operation. The same rule
already exists in `map-partition/src/build.rs:74–82` and the local key closure
in `map-partition/src/lib.rs:512–519`; the new
`tools/quarantine_fingerprints.py:26–30` independently reimplements it again.
The report calls this reuse, but the production geometry equivalence rule now
has four declarations. The new Rust literal also has no domain name. This is
an explicit 14b D.R.Y. finding, independent of the ancestry counterexample;
no current rounding mismatch is alleged. The test's independent historical
decoder is appropriate and is not the offender.

**Closure, X0 with the existing partition owner:** if the fingerprint guard is
retained, have one owned point-key operation define quantization/byte identity
and migrate the same-category production sites to it. Derive the catalogue
through that owner or a mechanically generated/gated shared specification,
instead of another handwritten copy of its rule. Generated boundary/rounding
controls should bind the producer and compiler to that single declaration.
No unsolicited map-types/graph-types change or alternate licensing rule is
requested. F-287 is the next free queue number checked for this review.

### Verification, category pass and limits

Exactly **one scoped Cargo gate invocation** ran, using the required environment,
shared target, nonincremental rebuild and `nice -n 10 cargo test -j 4
-p map-adapters -p map-canon -p map-compile`. It passed **75/0**:
adapters 16, canon 30, compiler 18, original exclusion 4, content 7; zero doctests.
The author's subprocess entry remains deliberately ignored and is executed
by its parent law. Three additional reviewer diagnostic entries were compiled
as ignored tests, then invoked directly from that same executable, without
another Cargo gate/build: full-med parent and child **green**; partial-polity
refusal **red**. The latter is diagnostic review evidence, not a production
test added to the author branch. Its source and full outputs are retained
below; the temporary test and restored GeoJSON were removed immediately.

The existing two library unused-import warnings and one test import warning
remain; no new warning category was introduced. The foreign heavy lock still
belongs to codex-maps-land2 for C12 landing. No full workspace, make contract,
fresh canon, mutation or browser gate was run, no server was started, no lock
was acquired and no owner port was touched. Exact golden sampling and integrated
completion remain blocked on the separately owned X4 canonical-Canaan repair
and outstanding full gates. No golden was re-blessed.

14b accepts the closed excluded-source/refusal sums and existing maintained
Serde/SHA2/Proptest/Syn tools; copied point-key ownership is F-287. 24a/24b
accepts the complete current loader removals, but ancestry remains writable
through partial descendants (F-283). All touched compiler exits use the one
checker, whose completeness rather than wiring count is the outstanding issue.
F-284's reported test sites are closed. No Critical finding or implementation
repair is added. Reviewer application LOC delta: **0**. Retained review tools are evidence only;
all reviewer diagnostics were removed from executable source directories.

Evidence: [repair re-review directory](evidence/2026-10-09-maps-x0-rereview/),
with the pinned audit/results, exact range/source and 43 retained-data hashes,
single gate log, independent restoration source/outcomes and SHA-256 manifest.
No excluded coordinate fixture is committed. Claude owns the repair on the
same author lane; review its next exact head before landing. No own lock,
server or background job remains.
