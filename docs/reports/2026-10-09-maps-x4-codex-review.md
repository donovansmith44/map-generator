# MAPS-X4 second Codex review

**CHANGES REQUESTED** at `62d5a951bfb8b090e27d0912cda6d3c124f16f24`.
Reviewed `origin/lane/claude/MG-BASE` (`6ac32bfbf67e26b9cfe94560806fda293db05801`)
through that head. MG-BASE exists; the CX-M0 fallback was unnecessary. The sole
review branch, `lane/codex/MAPS-X4-review`, starts at the reviewed head. This is
the owner/controller-authorized second review, with Important/Critical findings
only, PRINCIPLES 14b and 24a/24b, and one scoped gate invocation.

The added parsed-row refusals are useful and preserve the inherited wiring.
The C2 inventory is complete for the requested declarations and PLACE-1a
retirements. Declaration closure, generated refusal coverage and the touched
interface vocabulary still need repair. No Critical finding was identified.

## Accepted evidence

- MG-BASE is the ordinary merge of CX-M0 `9f99a90` and MG-LOCAL `61cde6b`.
  `main.rs`, `compile.rs`, `timeline_bridge.rs` and `partition_bridge.rs` are
  byte-identical between the base and the reviewed head. The registry JSON
  has SHA-256 `726a07fb0676b61b2bd38105729802963403fb522866ee9f7e379fec2138da45`
  at local `089a4bf`, MG-LOCAL, MG-BASE and X4. No alias or reason was reminted.
- The three map namespace aliases are accounted for. The pinned polity,
  narrative and `place:`-prefixed gazetteer inventory law enumerates the inputs
  and finds zero outgoing registry moves. This is a declaration-set observation,
  not a whole-pipeline identity proof or a NodeId equivalence proof.
- Replayed the existing MG-LOCAL inventory tool against the atlas export:
  roots `a1b93a3b049a1fe0` to `21d9db9b4c4a9e188a95aa442f7e6b59`,
  1,373 to 1,358 places, all fifteen report pairs match the recorded absorptions,
  no added key, no unmatched removal, every survivor present. The report names
  Place/AbsorbedInto, the original decision commit and a pinned source containing
  the original reasons. C2 can consume these local keys through its existing
  NodeId vocabulary and retain those exact original reasons/provenance. The
  summary prose is not a replacement identity ruling. Ten legacy era Map moves
  remain explicitly C2-owned; X4 need not invent them or write C2's TOML file.
- Read the five available author red logs: duplicate justification overwrite,
  SameId used for distinct ids, malformed collections, conflicting kinds,
  blank ids, non-object documents and omitted collections fail before their
  fixes. These are preserved author diagnostics, not independently replayed
  historical builds. The new bounded id laws do exercise name collision,
  direct aliases, dangling homes and immutable duplicate refusals.
- All fourteen added test bodies have plain messages and one fact per assertion
  (24 assertions). Added application-comment scan and range whitespace check
  are clean. No new dead member was identified in the delta.

## F-279 — Important: the declaration decoder still silently discards conflicting fields

`crates/map-compile/src/identity.rs:78–100` first decodes into
`serde_json::Value`. Duplicate JSON object fields have already been folded before
the new collection and alias checks run. Consequently these author documents
are accepted:

```json
{"unifications":[{"canonical":"home","minted":"a","kind":"Place","reason":"written identity"}],"unifications":[]}
```

```json
{"unifications":[{"canonical":"home","minted":"original","minted":"replacement","kind":"Place","reason":"written identity"}]}
```

The first erases the entire authored declaration set; the second replaces an
authored source identity. Neither is a duplicate alias in the already-decoded
array, so the submitted duplicate-alias law cannot detect it. The author report
acknowledges this inherited decoder limitation, but the newly claimed
"refuse ambiguous registry declarations" category remains open at its public
load door. This is the same document/refusal category as the repaired omitted
and malformed collection paths, not a request to write a general JSON parser.

**Closure:** decode directly into an explicit declaration document/row using the
maintained serde machinery, refuse duplicate collection and semantic row fields
before information loss, and preserve the intentionally supported empty document
and authored metadata explicitly. Map library errors once into the domain's
closed refusal vocabulary. Coordinate any manifest change with its reserved
writer. Add generated duplicate-field controls at root and row level, including
field order and conflicting canonical/minted/kind/reason values, plus valid
controls. Repair through the one Identity load owner; no caller-local checks.

## F-280 — Important: new runtime refusal tests still use fixed examples

`identity.rs:186–262` tests scalar documents, missing collections, malformed
collections and conflicting kinds using only fixed literals/pairs.
`registry.rs:324–348,374–388` tests missing justification/source and blank ids
using fixed strings. These are runtime admission/refusal laws, not source or
committed-data inventories. The bounded generated-id loops in other tests do
not make these changed behaviors generated. The main A-EMPHASIS and
A-WIRE-LOCAL-IDS reviews reject this same distinction under F-207.

The agreeing-kind test also repeats all ten kind spellings at
`identity.rs:267–278`, separately from `kind_of` and `EntityKind`. A new kind can
be added without extending the purported all-kind law. This is 14b's duplicated
closed vocabulary and 24b's incomplete category enumeration.

**Closure:** use maintained generated-property tooling for valid/invalid
documents, ids, reasons and sources, taking these readable examples as seeds.
Generate declaration sets and relevant insertion/field orders; check complete
typed refusals and unchanged registry state, with one plain fact per assertion.
Derive the finite kind universe from its single typed declaration, including
conflicting pairs, instead of maintaining another string list. Keep the real
authored-fixture and export inventories as honest structural checks. No
production behavior repair should precede its failing law.

## F-281 — Important: touched public rules are still carried by strings

`Registry::declare` (`registry.rs:65–92`), `Identity::check` and `load_registry`
(`identity.rs:50–145`) expose `Result<_, String>`. This packet extends that
interface with new refusal sentences; the new laws assert formatted prose
instead of a closed declaration/refusal outcome. `WitnessRef.kind`
(`registry.rs:25–30`) and `Identity::witness`/`implied_kind`
(`identity.rs:22–36,164–175`) use a string for the closed geometry vocabulary.
An unknown spelling silently enters the wildcard kind inference. `kind_of`
separately restates the enum's vocabulary as string matching.

The report discloses this inherited Important type debt and assigns a future
coordinated writer. That is honest, but it is not an owner-approved exception to
the binding "everything in types" and 14b bar for the touched registry door.

**Closure:** reuse one closed geometry-kind vocabulary and one domain refusal
sum at their existing owners. Keep free prose as reason/source text, and render
the refusal's user message at one outer boundary. Reuse the EntityKind declaration
for decoding and law enumeration. Coordinate every caller migration with X5 and
the contract/dependency owners; do not create a second vocabulary, change
map-types/graph-types unilaterally or patch reserved files on the side. Generated
laws cover every variant and unknown wire spelling, and the type system rejects
unknown internal geometry kinds.

## Existing producer category remains open at its reserved owners

Retain the already recorded Important queue category "X4 identity outside
Registry" rather than allocate another finding number:

- `main.rs:403–439`: bg_shadows decides suppression using slugified polity ids
  and names; X2 owns removal/migration.
- `timeline_bridge.rs:60–113`: the available source region id is ignored when
  minting `prefix:slug(label)`. Distinct source identities with colliding labels
  collapse before Registry sees them; X5 owns explicit source-id migration.
- `compile.rs:124` and `main.rs:220–224`: atlas narrative and kept authored Route
  entities are emitted without Identity observation/resolution; X5 owns both.

These are inherited faithfully, not introduced by the three-file delta. The
name-collision test exercises Registry directly; the words "same coordinates"
are only a name string. It cannot fence lost identities or emitters outside that
door. Registry itself has no coordinate input, which correctly gives proximity
no authority there. Whole-pipeline closure needs generated source identities
with colliding names and identical/nearby geometry, explicit written aliases,
and an exhaustive emitted-feature/registered-home law. Every feature family
must enter through the one identity owner. `Identity::check` alone cannot check
features it never observed. No X4 side edits or expanded source admission are
requested; the controller must coordinate the reserved writers before claiming
the plan's every-minted-id completion condition.

Inherited unused imports, CITY_NOTE and partition/provider dead members stay
with their recorded owners. No new warning waiver, golden acceptance, licensed
geometry admission or whole-map registry approval is granted by this review.

## Independent gate and handoff

One invocation, with the reviewed source and the temporary review-only
integration diagnostic:

```text
. ~/.bible-atlas-env
CARGO_TARGET_DIR=~/mut/target nice -n 10 cargo test -j 2 -p map-canon -p map-compile --no-fail-fast
```

| Observation | Result |
|---|---|
| Submitted map-canon tests | 36 passed, 0 failed |
| Submitted map-compile tests | 26 passed, 0 failed |
| Ambiguous root/row decoder diagnostics | 2 expected failures, each first failing at index 0 |
| Unambiguous decoder control | 1 passed, all 32 generated identities |
| Overall invocation | Exit 101, solely the review diagnostic target |

The run initially waited on the shared Cargo build-directory lock held by other
work. Cargo then compiled the dependencies and map crates from this review
worktree; no shared-target clean or source timestamp workaround was used. The
reported 2m35s build duration includes the lock wait. Warnings are the inherited
CITY_NOTE, unused imports and Bundle.biggest sites; none originates in the two
X4 source files. The temporary integration test was removed after the invocation;
all application/test paths are restored byte-for-byte to the reviewed head.

The atlas heavy lock is held by `codex-maps-land2` for C12 landing gates. This
review ran only the scoped non-mutation tests above, not a competing workspace
gate. Full workspace/contract/golden gates were not independently run. The
author's single aggregate gate is carried evidence: Haskell 381 passed,
contract check/vocabulary passed, Rust stopped at the inherited encoder limb
fixture drift; later semver unrun. The browser golden remains unrun because the
X4 worktree has no compiled canon/workbench, and excluded ingests must not be
used to manufacture one. No fixture was re-blessed and port 8080 was untouched.

Evidence lives in `docs/reports/evidence/2026-10-09-maps-x4-review/`: review-only
decoder diagnostics, gate log, replayed inventory, author red logs, range diff
and their stored/decoded SHA-256 manifest. The diagnostics are evidence, not a
production pipeline or repaired author tests. No source repair, source/data
admission, fixture blessing, serving change or geometry generation was made.
Claude owns repair and landing; re-review the next exact head on this same review
branch. Review changes are documentation/evidence only.
No own lock, server or running job remains. Tracked application LOC delta is
zero; all review additions are this report and its evidence.

## Repair re-review — 2026-10-09

**CHANGES REQUESTED** at `53591b5de91733c2e3706823480a7b3fe02980bb`.
Repair range `62d5a951bfb8b090e27d0912cda6d3c124f16f24..53591b5de91733c2e3706823480a7b3fe02980bb`;
whole item `6ac32bfbf67e26b9cfe94560806fda293db05801..53591b5de91733c2e3706823480a7b3fe02980bb`.
MG-BASE exists, so no CX-M0 fallback. The author and review remote heads were
fetched again before recording this verdict and remained unchanged. This
appendix supersedes the earlier verdict's repair status, not its retained
historical evidence. Only Important/Critical findings were considered.

### Accepted repairs and C2 input

F-279's duplicate-field information loss is closed at the tool's original load
door: Serde admits explicit map-only document/row types directly, with duplicate
semantic and metadata fields refused before conversion to a Value. Empty objects,
explicit empty collections and the declared metadata field retain support.
Both exact decoder probes from b319166 were rerun unchanged: each passes all
32 cases, and the unambiguous control passes all 32 cases. The submitted decoder
target's eight tests also pass, including generated field orders, all semantic
fields, metadata duplication and strict kind-string representation.

F-281's touched refusal and geometry interfaces are closed types. EntityKind
owns Serde decoding and finite iteration; GeometryKind is derived from Feature;
declare, load and check return typed refusals. Library errors are classified at
one decode boundary and diagnostics rendered at the compiler boundary. Existing
witness callers were migrated mechanically; no map-types/graph-types or contract
change occurred. The original MG-LOCAL observation/resolution wiring is retained.
The reserved producer bypasses remain X2/X5-owned, as recorded above.

F-280's fixed runtime examples and duplicated kind-spelling list have been
replaced by 33 maintained Proptest properties and type-derived enumeration.
Generated duplicate declarations, immutable refused state, dangling live homes,
chains, names colliding across distinct ids and complete kind conflicts are
covered. Registry has no coordinate input, so proximity has no identity authority
there. The new law/oracle defects below prevent acceptance of this test packet;
these properties do not claim closure over X2/X5's source minting and emitters.

Read the preserved author red evidence: exact decoder probes 1 pass/3 fail,
generated decoder packet 2 pass/4 fail, strict kind representation 0 pass/1 fail,
and missing DeclarationRefusal compile failure. The stored and decoded evidence
digests match the author's manifest. These are inspected historical author runs,
not independently repeated historical builds.

The Canaan retirement is justified by Gen 10:19 plus X0's excluded-source
quarantine, with its exact original alias/reason and alternatives retained in
the report. It retires a drawing-dependent live declaration without inferring
absence or inventing a successor. Both surviving declarations and their reasons
are exactly unchanged: **2 active map aliases, 0 pinned atlas moves**. Replayed
the existing MG-LOCAL inventory tool against the atlas export: roots
`a1b93a3b049a1fe0` to `21d9db9b4c4a9e188a95aa442f7e6b59`, **1373 to 1358**,
exactly the fifteen PLACE-1a removals, no added/unmatched key, every written
survivor present. All fifteen report pairs match. Original decision provenance
and reason-source revision remain available for C2's AbsorbedInto rows. The
appendix explicitly supersedes the earlier three-active-alias inventory. C2's
ten legacy era Map moves and successor serving remain C2-owned.

### F-288 — Important: generated laws do not have sound, independent oracles

`crates/map-compile/src/identity.rs:242–247` generates
`"unification{token}"` as an allegedly unknown field. `token = "s"` produces the
valid document `{"unifications":[]}`, which the property requires the decoder to
refuse. The independent gate actually shrank to this input after 179 successful
trials: actual `None`, expected `Some(InvalidDocument)`. This is a false negative
law and a randomly failing core gate, not a decoder defect. The author's passing
run does not prove the property true.

The same generated-law ownership/category pass found
`identity.rs:642–656`'s `expected_kind`: it repeats the complete production
`implied_kind` match at lines 195–210, including the nested layer match.
`generated_geometry_kinds_have_total_entity_kind_inference` computes its expected
Entity using that copy. This restates the rule instead of documenting independent
expected behavior; copying a mistaken branch into both matches preserves green.
It fails PRINCIPLES 14b's D.R.Y. pass even though the closed type vocabulary itself
is now sound.

**Closure:** the X4 law owner partitions genuinely unknown fields from supported
fields, retaining the saved `"s"` counterexample as a valid control. Use generated
inputs with independent, complete expected entities for geometry/layer behavior;
remove the copied decision match. Do not replace it with a call to the production
inference function to compute its own expected answer. The type-derived universe
must still fence every case, and each assertion keeps one fact/plain message.
No production decoder rejection of the valid collection is requested.

### F-289 — Important: assertions inside touched registry tests lack plain messages

The mechanical GeometryKind migration changes seven test bodies in
`crates/map-canon/src/tests.rs`. Eleven assertions in those touched bodies still
lack their plain behavior message: lines **696, 697, 698, 702, 706, 707, 714,
728, 735** in `the_registry_resolves_totally_in_one_hop_and_refuses_chains`, and
**749, 750** in `slug_equality_alone_never_unifies_anything`. These are readily
visible whole-body sites, not generated macro internals or untouched tests.
Inherited assertions inside touched bodies retain the same bar as F-284's X0
review and the main reviewer. The new property assertions and the mechanically
changed compiler refusal assertions do have plain messages.

**Closure:** the X4 test owner supplies a plain behavior message for every
assertion in the touched test bodies, retaining one fact per assertion and the
complete outcomes. Audit the whole touched bodies, not only added assertion
lines. This does not authorize repair of unrelated test files or a second
registry rule owner.

### One independent gate and handoff

Exactly one invocation, at the reviewed author head, with the original three
review-only decoder diagnostics temporarily restored:

```text
. ~/.bible-atlas-env
CARGO_TARGET_DIR=/home/donovan/mut/target nice -n 10 cargo test -j 4 -p map-canon -p map-compile --no-fail-fast
```

| Target | Passed | Failed |
|---|---:|---:|
| map-canon | 39 | 0 |
| map-compile library | 36 | 1 |
| submitted decoder target | 8 | 0 |
| exact original reviewer probes and control | 3 | 0 |
| Total | 86 | 1 |

Exit **101**, solely the invalid unknown-field property in F-288; build 5.24s.
No retry, mutation run, workspace gate or aggregate contract run. The foreign
atlas heavy lock remains `codex-maps-land2 / MAPS-C12`; this was a permitted
scoped run. Full workspace/contract gates and browser goldens remain unrun in
this review. No fresh canon/workbench, excluded-source compile, golden blessing
or port-8080 action was performed. Range whitespace and added-comment checks
pass; no new application dead member was identified. Inherited dead code and
producer closure remain with their recorded owners; integrated X4 completion
still requires those coordinated producers and whole-map gates.

The gate log and Proptest-generated regression seed are retained under
`docs/reports/evidence/2026-10-09-maps-x4-rereview/`. Temporary diagnostics and the
generated source-tree regression directory were removed after the run; the
author source was clean before returning to the existing review branch tip.
Review additions are report/evidence only, application LOC delta **0**. No own
lock, server or running build remains. Repair through the sole X4 law/test owner,
then re-review its next exact head on `lane/codex/MAPS-X4-review`; no side repair,
source admission, contract change or landing was made.

## Second Codex final law/assertion re-review (2026-10-09)

**APPROVED at `709122f90b4f63029ed2cfdc804e00826dccf968`. F-288 and F-289 CLOSED.**
Reviewed repair `53591b5de91733c2e3706823480a7b3fe02980bb..709122f90b4f63029ed2cfdc804e00826dccf968`
and whole item `6ac32bfbf67e26b9cfe94560806fda293db05801..709122f90b4f63029ed2cfdc804e00826dccf968`.
MG-BASE exists, so no CX-M0 fallback was used. Read the X4 queue row, migration
plan X4/C2 requirements, both previous review sections and the archived main
A-EMPHASIS/A-WIRE-LOCAL-IDS review reports. No new Important/Critical finding;
no new F-number. Next free at queue readback is F-295, left unallocated.

### F-288/F-289 closure and red evidence

The unknown-field property retains its original suffix family and excludes
exactly `s`: that is the sole suffix producing supported `unifications`;
`_comment` cannot be produced by this family. The named valid control admits
that saved input and compares the entire registry with `Registry::default()`.
The author retained the original seed's pre-repair red result, 0/1 at token `s`;
all nine entries in its law-repair checksum list independently verify. This
historical red run was inspected, not rerun. The independent final gate restored
the same seed temporarily and passes both the repaired property and valid control.

The copied `expected_kind` match is deleted. Explicit expectation data supplies
complete expected Entities: Ways are Route, Points/Memories Place, Lines
Waterbody, and Areas have the separately written Water/Relief/ScriptureClaims
and Territory/Background/Journeys expectations. The oracle neither calls nor
copies the production inference function. A separate law counts each pair
against GeometryKind/LayerKind from their existing typed owners, requiring one
example per pair; missing or overlapping cases fail. Every generated identity
still exercises every typed geometry/layer combination.

All eleven reported canon assertions now carry plain behavior messages, with
observations unchanged. Replayed the maintained tree-sitter body audit over the
whole item: **57 changed test bodies / 82 assertions / zero missing messages /
zero tuple or Boolean-vector observations**. Read the changed bodies as well as
the audit output; each assertion states one fact. The initial system-Python
module lookup failed; the replay then reused already installed pinned grammar
modules, without changing dependencies or adding a parser.

### Whole-item identity, C2 and review passes

F-279/F-281 decoder and interface closures remain accepted. F-280's maintained
generated Registry/Identity/decoder laws now pass with sound expectations:
duplicate aliases and semantic JSON fields refuse, rejected declarations preserve
the complete Registry, dangling homes/chains report complete typed outcomes,
and distinct generated identities with colliding names remain distinct. Registry
accepts no coordinates, so proximity cannot remint an identity at this door.
This guarantee does not recover source identities lost before the door.

MG-LOCAL observation/resolution wiring is retained. Whole-item bridge edits are
only the previously reviewed typed witness arguments; main edits render typed
refusals. `compile.rs` is unchanged. The new repair itself changes tests and
evidence only: production Identity bytes before cfg(test) and registry JSON
bytes exactly match 53591b5, application LOC delta **0**, Rust test LOC **+64**.
No map-types/graph-types, contract, source, geometry, fixture or version change.
No application comment added and no new dead member identified. Source/data/report
whitespace checks and owned identity rustfmt check pass; literal captured author
logs retain trailing whitespace/EOF blanks, so a whole-evidence diff-check is
not claimed clean.

Independently replayed the existing MG-LOCAL inventory tool and matched the
report table: **15/15 PLACE-1a AbsorbedInto pairs**, **1373 to 1358**, no added
keys or unmatched removal, all survivors present. Both active alias rows and
written reasons are unchanged; **2 map aliases / 0 pinned atlas moves**. The
retired Canaan declaration, original justification, Scripture grounds and
alternatives remain recorded. C2 has complete local keys, kinds, moves and
original provenance to consume; its ten legacy era Map moves and served
successors remain C2-owned.

PRINCIPLES 14b accepts one declaration door, library-owned decoding and typed
vocabulary, and independent declarative expectations rather than a duplicate
inference rule. The 24a/24b pass accepts closure of the reported document,
law-oracle and assertion categories. Existing bg_shadows/name suppression,
timeline label-based minting and unobserved Route emitters retain the already
recorded X2/X5 producer owner. Existing warnings/dead-code sites retain their
recorded owners; this approval does not close or duplicate those findings.

### Exactly one independent gate and handoff

```text
. ~/.bible-atlas-env
CARGO_TARGET_DIR=/home/donovan/mut/target nice -n 10 cargo test -j 4 -p map-canon -p map-compile --no-fail-fast
```

**39 canon + 39 compiler library + 8 submitted decoder + 3 exact original
reviewer probes = 89 passed / 0 failed**, exit 0, build 6.93 seconds. Original
reviewer probes retain their 32-case duplicate-root/duplicate-minted refusals
and valid control. The final run included the saved regression seed; no retry
or second Cargo invocation. Only inherited CITY_NOTE, Area/PolityRow imports
and Bundle.biggest warnings remain.

The foreign heavy lock `e0ef86a1` belongs to codex-maps-land2 / MAPS-C12 and is
not a mutation lock. This scoped gate used the shared target; no own lock was
acquired. Workspace/aggregate contract and browser goldens remain unrun here:
full gates require the foreign heavy reservation to end and integrated permitted
producers/fresh canon/workbench. No excluded-source compile, golden blessing,
mutation, port action or landing was performed.

Evidence is retained with stored/decoded SHA-256 values under
`docs/reports/evidence/2026-10-09-maps-x4-final-review/`. Temporary probes and
regression directory were removed, and the exact reviewed source was clean
before returning to the sole review branch. Review additions are report/evidence
only. No own lock/server/build remains. Controller may integrate this approved
X4 packet with its reserved producer work and run the outstanding whole-map gates;
whole migration completion and rendered approval are not claimed.
