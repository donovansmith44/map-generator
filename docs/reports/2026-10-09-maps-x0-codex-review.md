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
