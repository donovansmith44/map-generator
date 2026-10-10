# MAPS-X3 second Codex review

**CHANGES REQUESTED — Important F-290 and F-291.** The NUM34 removal is
sound as a partial source repair, but a retained consumer no longer
compiles and some survey relations lose the subject Scripture gives them.
F-142 and the deferred typed reader remain open with their existing owners.

## Exact scope

Owner/controller-authorized review in `~/w/CX2-rev-x3`, report branch
`lane/codex/MAPS-X3-review` created directly from the reviewed head.

- Requested base, fetched `origin/lane/claude/MAPS-X0`:
  `7bcbabab132443e535fa7b18c2cdb4500457e3c8`.
- Reviewed latest `origin/lane/claude/MAPS-X3`:
  `ecd35634dbe7ec1a9da4d3fb247e83d251f50160`.
- `origin/lane/claude/MG-BASE` exists at
  `6ac32bfbf67e26b9cfe94560806fda293db05801`; **no CX-M0 fallback**.
- X3 inherited X0 through `e3b20f4`, merge `59372d1`. X0 subsequently
  advanced to `7bcbaba`; the requested endpoint comparison therefore also
  shows the later X0 test/report changes absent from X3. These are not
  attributed to X3 as newly authored reversals. Own X3 commits inspected:
  `999262c` and `ecd3563`, plus the shared-file merge.

Binding review inputs: atlas AGENTS, PRINCIPLES, roadmap, locks and GO
files; current ops Lane B X3 row and F-142; migration plan from
`origin/lane/claude/MAPS-C12`; map handoff §A. The older queue path is
absent; the atlas AGENTS names `~/src/bible-atlas-ops/QUEUE.md` as authority.
The adopted emphasis/local-id reviews' admission, generated-law and
whole-body assertion bar was applied. Findings below are Important only.

## F-290 — deleted fixture API still has a compiled consumer

**Important.** `crates/map-adapters/src/lib.rs:27` removes the export and
`surveys.rs` removes `promised_land_timeline`. The retained generated law
`renamed_split_and_indirect_descendants_keep_the_excluded_content` still
calls `map_adapters::promised_land_timeline()` at
`crates/map-compile/tests/exclusion_content.rs:250`. This is an ordinary
compiled Proptest body, not an ignored or unreachable caller. It was
already present in the inherited `e3b20f4` checkpoint before X3's deletion.

The adapter-only gate cannot exercise this consumer. A compiler-only
probe against a freshly compiled library from this exact worktree returns
**E0425, cannot find function `promised_land_timeline`**. A control calling
the retained `scripture_timeline` compiles against the same library. This
is sufficient to establish the broken API site; no independent full
map-compile/workspace result is claimed.

**Category / closure:** removing a producer must migrate every consumer,
including generated lineage laws. Coordinate X3's removal with the sole
X0 law owner; use a legitimate nonempty retained or test-owned timeline
fixture, preserve the generated indirect-ancestor refusal and complete
unchanged-output controls, and enumerate every removed-symbol caller.
Do not restore the guessed NUM34 drawing, add a compatibility shim, drop
the property, or create another exclusion-rule owner. The current compiler
probe supplies red evidence for that repair.

## F-291 — relative survey predicates lose their grammatical target

**Important.** The new flat `{ name, verse, relation, site }` records cannot
preserve the different subjects and targets in a verse merely by keeping
the names in sequence.

- `allotments.toml:331` records Janohah with `relation = "east_of"`.
  Joshua 16:6 says the border passes Taanath-shiloh on its east on the way
  to Janohah. Janohah is the destination; the positional target is
  Taanath-shiloh. The row instead attaches the positional predicate to
  Janohah. Retaining both names does not retain that relation.
- `allotments.toml:260` and `:437` flatten Adummim as `before` in the
  Judah and Benjamin walks. Joshua 15:7 qualifies **Gilgal**, and 18:17
  qualifies **Geliloth**, relative to the ascent of Adummim. These are
  relations between references, rather than a separate border visit or
  a border-before-Adummim predicate. The flat row supplies neither the
  subject link nor a distinct relation family.

These statements are checked against the repository's public-domain KJV,
not against a modern drawing. The evidence stays undrawn, so this is a
content/admission defect, not a claim of an observed rendered error.
The current laws check citation membership, names and the Judah south
order; none asserts these relational outcomes or generates controls that
change the subject/target while retaining a valid verse and place id.

**Category / closure:** X3 owns the Scripture evidence, X5 the deferred
typed reader, C14 Site selection and X16 Course/drawing. Preserve the
verse's subject, relation target and travel destination separately at
that one evidence door; survey every relative-reference and neighbour
sequence, including the hill/valley clauses at 15:8 and 18:16. Record
interpretive alternatives where needed. Pin whole admitted outcomes and
generated subject/target/direction mutations red before repair. Do not
invent coordinates, select another course, or implement a competing reader
to work around X5's dependency reservation. This finding does not ask for
unilateral map-types/graph-types changes.

## Accepted evidence and remaining limits

Independent maintained-TOML audit counts **14 surveys, 43 sequences and
300 verse-cited references**. Every survey has a KJV source and a citation
in Numbers 34 or Joshua 13–19; every reference cites an existing verse.
Thirty-six references have an explicit unresolved identity. An atlas id
is not a claim of a located Site. The table supplies no coordinates or
drawable geometry and has no High promotion. Unlocated Mahanaim, Hukkok
and other chosen Sites are retained as references; C14/X16 remain the
uncertainty/alternative owners. City lists, undated Dan's later move and
the explicit Levi exclusion remain recorded separately. No new excluded
source byte or production reader is introduced by this packet. This is
not an independent acceptance of all inherited X0 lineage machinery.

The dating record preserves 1446, 1 Kings 6:1, the 480-year count and the
approximately 966 temple anchor, with Ussher explicitly source provenance.
The late-wilderness context is distinguished from a precise NUM34 event
placement. No blanket shift or substitute NUM34 date is introduced.
NUM34's guessed circuit, lifetime, stand-in catalogue contribution and
dedicated producer are removed. The chronology test scans the **full,
nonempty** retained timeline; the 201 matched-export dates also cannot
restore the missing course. Its green result is honest for that removal.

**F-142 is not closed.** The complete adopted interval door is still
missing across add_survey, add_route, add_era and kept-route canon
insertion; producer chronology/root compatibility remains unresolved.
The X3 report and ops row explicitly record this with X5/producer. No
duplicate finding number or review-side implementation is added. The
serde/TOML typed reader is explicitly deferred to the single dependency
writer; Python/TOMLI is used only by evidence tests. It is not an
application reader or a replacement pipeline. Completed X3 is not approved.

## Ten KJV spot-checks

Read-only KJV: `~/src/bible-atlas/data/raw/kjv.json`, atlas checkout
`f6eb9fb963c6f608beffdaf09c259214059a421b`, SHA-256
`f0b09dc49dfb97bb84f03aae1fbf026485048c3cab31a7a41017e2d86ac1d11c`.
Full verse text and exact sampled rows are retained in `audit.json`.

| Survey reference | Verse | Result |
|---|---|---|
| NUM34 Riblah / Ain | Num 34:11 | Names and relative east side retained; undrawn |
| Reuben Aroer / Arnon | Josh 13:16 | Named feature and unlocated river city distinguished |
| Gad Mahanaim | Josh 13:26 | Reference retained; no fabricated located course |
| Judah south of Kadesh | Josh 15:3 | Relative wording retained |
| Judah Adummim | Josh 15:7 | F-291: Gilgal's relation is flattened |
| Ephraim Janohah | Josh 16:6 | F-291: positional target changed |
| Manasseh's northern Asher neighbour | Josh 17:10 | Neighbour retained without a course |
| Benjamin Adummim | Josh 18:17 | F-291: Geliloth's relation is flattened |
| Naphtali Adami / Nekeb | Josh 19:33 | Separate textual names retained |
| Dan before Japho | Josh 19:46 | Limit retained without a possession inference |

## Verification and review passes

**Exactly one test gate:**
`. ~/.bible-atlas-env; CARGO_TARGET_DIR=~/mut/target CARGO_INCREMENTAL=0
nice -n 10 cargo test -p map-adapters -j 4`.
**28 passed, 0 failed, 0 ignored; doc tests 0/0; no warnings.**

One subsequent **build-only** diagnostic used `cargo rustc -p map-adapters
--lib -j 4 --message-format=json -- --cfg maps_x3_review` to ensure the API
probes used this worktree's library, then two rustc metadata probes.
The initial probe picked the newest shared-target rlib by timestamp and
unexpectedly compiled the deleted API: that artifact was stale for this
head. That observation is retained in `stale-api-probes.json` and is not
accepted evidence. The corrected Cargo artifact identifies this worktree's
manifest and `fresh:false`; the same-library removed/control probes then
return **1/E0425** and **0**. No test gate was repeated.

The atlas `heavy` lock remains held by `codex-maps-land2` for C12 landing,
`e0ef86a1`, since 20:09:32 EDT. Workspace, make-contract and 8090 goldens
were not independently run. No canon built, fixture or view blessed,
producer/export rewritten, foreign lock released or port touched.

D.R.Y./Haskell-bar and category passes: F-291 is the failed relational
abstraction; F-290 is the incomplete producer-removal migration. The
deferred typed door is recorded instead of claiming structural closure
from the string/JSON evidence validators. Manual whole-body assertion
review covers the 16 new test/helper macros and all 12 macros in the
touched legacy fixture: every macro has one fact and a plain behavior
message, with no tuple/Boolean-vector observations. There are no added
application/test comments. The NUM34 producer has no spare compatibility
implementation; its live dangling consumer is reported above. Other
inherited chronology/identity/source categories retain their owners.

Red-before-green: the author's report records evidence laws red before
the table and the original chronology/matched-date controls red before
removal. The intermediate `999262c` retains the chronology law with the
legacy producer. I did not rerun historical red gates or claim independent
execution of those logs. The current gate and API diagnostic are retained.
Endpoint diff-check has whitespace-only defects (including the TOML final
blank line and older X0 evidence); these are outside this Important-only
finding list, not claimed clean.

Review-only files are this report and its checksummed evidence. Source,
data, KJV, types, contracts and goldens were not edited. No own lock,
server or background job remains. Own implementation LOC delta **0**.

## Repair re-review at 5681779 (2026-10-09)

**CHANGES REQUESTED — F-290 CLOSED; Important F-291 remains.** The deleted
producer's consumer migration is accepted. The original Janohah, Gilgal
and Geliloth attachments are repaired, but the new unnamed-river reference
contradicts the adopted relation in the same verse. No new finding number
is allocated for this existing category; the next free number at this
review's queue snapshot is F-295.

### Exact scope and ownership

Owner/controller-authorized review in `~/w/CX2-rev-x3b`. Repair range:
`ecd35634dbe7ec1a9da4d3fb247e83d251f50160..568177905021f566d3a8c2a1919fc43f42e67c77`.
The latest remote head was fetched again after verification and is unchanged.
The source was inspected and tested detached at that exact head, then this
worktree returned to `lane/codex/MAPS-X3-review` at `be5dd7b` to append only
review documentation and evidence. No author branch was changed.

The requested whole endpoint comparison is current
`origin/lane/claude/MAPS-X0`,
`46c738aabc0c7f629574e3317ffd76a78f00c7dd..568177905021f566d3a8c2a1919fc43f42e67c77`.
X3 actually inherits X0 through `7bcbaba`, merged by `c0e3b94`. The current
X0's later ordered-lineage/point-key repair is absent from this head; that
endpoint difference is not attributed to X3 as a newly authored reversal.
X3's own full range `7bcbaba..5681779` and the whole migration range
`6ac32bfbf67e26b9cfe94560806fda293db05801..5681779` were also inspected.
MG-BASE exists, so **no CX-M0 fallback** was used.

Re-read the binding rules, current Lane B X3 row, migration plan from atlas
`origin/lane/claude/MAPS-C12`, and earlier emphasis/local-id reviews on their
local review branches. The legacy `.superpowers/QUEUE.md` path remains
absent; the ops queue is authoritative. Important/Critical findings only,
one native gate, no delegation or implementation repair.

### F-290 closure accepted

`crates/map-compile/tests/exclusion_content.rs:254` now obtains a retained
`scripture_timeline`; its next assertion establishes a nonempty control.
The full generated body still checks renamed/split/indirect descendants,
complete independent provenance refusals and the unchanged compiler store.
The source-wide search finds no live `promised_land_timeline` or
`NUM_34_CIRCUIT` site. The complete two-crate gate compiles the formerly
broken integration target and executes the property successfully.

The author's retained red-api log reproduces the original E0425. I inspected
that log and the repair; I did not repeat its historical build. No shim,
guessed NUM34 restoration, property deletion or new exclusion-rule owner
was introduced. **F-290 is CLOSED.**

### F-291 residual: the unnamed referent reverses its own adopted relation

**Important, existing category.** At exact `5681779`,
`data/authored/surveys/allotments.toml:262` asserts that the **ascent of
Adummim is south of the river**, with an explicit subject and target.
The new reference at `:263`, including its `site.unlocated` identity, is
instead named **"river south of Adummim"**. Its name describes the opposite
ordering. The recorded primary at `:759` explicitly adopts the ascent
south of the unnamed river, so this is an internal contradiction even
before choosing between the recorded grammatical alternatives.

Read-only repository KJV Joshua 15:7 supports that adopted primary's
attachment; its alternative qualifies Gilgal instead. Neither recorded
reading justifies the newly named river south of Adummim. The new whole
inventory oracle at `crates/map-compile/tests/survey_relations.rs:183` and
the whole-waypoint expectation at `:228` repeat the inverted reference
verbatim, so all five submitted evidence tests pass it. Keeping it undrawn
avoids a current rendering error but does not make the evidence correct.

A separate read-only evidence diagnostic is **red, exit 1**, with one
plain-message assertion that the river reference must not reverse the
adopted Adummim-south-of-river relation. An in-memory neutral-reference
control passes, exit 0; it updates the reference and its links consistently
without changing any repository file, relation, coordinate or dating.
These are evidence diagnostics, not another native gate or a substitute
pipeline. Full observed rows and KJV text are retained in `audit.json`.

**Closure:** X3's evidence owner gives unnamed referents neutral textual
identities and keeps directional claims solely in their explicit
subject/predicate/target records. Sweep every relationally named unlocated
reference for the same reversal/unsupported-attachment category, preserve
the Scripture-grounded alternatives, and correct all reference links and
complete expectations together. Pin the contradiction red first. Keep
X5's single typed reader, C14's Site decisions and X16's Course/drawing
ownership; no invented river identification, coordinate, certainty,
shared-type change or competing admission door is requested. The original
three attachment repairs and the hill-clause sweep are accepted progress,
but **F-291 is not CLOSED**.

### Accepted checks and limits

Independent maintained-TOMLI audit gives **14 surveys, 43 sequences,
302 cited references, 37 unresolved identities**, with **34 Position and
5 Neighbour statements**. All survey/waypoint citations exist in the
repository KJV and lie in Num 34 or Josh 13–19; the undated Josh 19:47
reading remains separate. All survey sources are `kjv`, every named
subject/target resolves in its survey, and there are no coordinate or
geometry fields. The additional unnamed river and valley-of-giants
reference remain recorded. No new excluded source read, Course, Site
selection, precision or High promotion was introduced by X3's repair.
The four grammatical-reading records retain their justifications and
Medium confidence; C14/X16 still own Site/Course alternatives and drawing.

Ten exact rows were independently compared with read-only
`~/src/bible-atlas/data/raw/kjv.json`, atlas checkout
`f6eb9fb963c6f608beffdaf09c259214059a421b`, unchanged SHA-256
`f0b09dc49dfb97bb84f03aae1fbf026485048c3cab31a7a41017e2d86ac1d11c`:

| Survey reference | Verse | Assessment |
|---|---|---|
| NUM34 Riblah / Ain | Num 34:11 | Separate subject/target and Medium alternative retained |
| Reuben Aroer / Arnon | Josh 13:16 | Bank target and separately unlocated river city retained |
| Gad Mahanaim | Josh 13:26 | Unlocated reference retained without a drawn course |
| Judah Adummim / unnamed river | Josh 15:7 | Gilgal attachment repaired; inverted river identity is F-291 residual |
| Judah hill / two valleys | Josh 15:8 | Explicit valley relations and Medium alternative retained |
| Ephraim Janohah | Josh 16:6 | Destination distinct from Taanath-shiloh positional target |
| Manasseh northern Asher neighbour | Josh 17:10 | Subject and neighbour direction explicit; no course inferred |
| Benjamin Geliloth | Josh 18:17 | Geliloth relates to Adummim's ascent |
| Naphtali Adami / Nekeb | Josh 19:33 | Separate textual names retained |
| Dan before Japho | Josh 19:46 | Border relation retained without possession inference |

The dating record still derives the adopted 1446 scale from 1 Kings 6:1,
480 years and the approximately 966 temple anchor. It distinguishes source
chronology, late-wilderness context, Gilgal and the owner-chosen Shiloh
stop; no blanket shift or precise guessed NUM34 date is introduced.
The full nonempty chronology law and all 201 matched-export placement
controls pass honestly because the unadmitted NUM34 drawing is absent.
**F-142 remains OPEN** across add_survey/add_route/add_era/kept-route
insertion and the producer/root seam. There is still no complete adopted
interval door. The production serde/TOML reader is explicitly deferred
to X5. The closed serde shapes and TOMLI subprocess here are test-owned
evidence controls; they do not establish production semantic closure.
No runtime subprocess reader, copied export, hand-edited root or chronology
workaround appeared. Complete X3 remains blocked:X5/producer.

### Verification and review bar

Exactly **ONE native gate** at the exact candidate, sourced environment,
shared target, incremental disabled, nice 10 and four jobs:
`cargo test -p map-adapters -p map-compile -j 4`.
**62 passed, 0 failed, 1 inherited ignored helper**, exit 0; both binaries,
integration targets and doc tests compile. The ignored restoration child
is exercised by its passing parent test. The same three inherited unused
import sites warn; no new repair warning. No gate was repeated.

The maintained tree-sitter whole-body assertion audit gives repair
**19 changed bodies / 28 assertions**, and X3's own full range
**34 bodies / 48 assertions**, each **zero message-free, zero tuple or
Boolean-vector observations**. The MG-BASE whole-migration audit gives
54/74 with one inherited witness/licence mapping table candidate; manual
inspection confirms a whole semantic mapping outcome, rather than a
combined Boolean observation. The nine inherited X0 evidence checksums
verify. Author red-relation and E0425 logs were read and independently
hashed; their historical execution remains author evidence. The generated
attachment controls exercise this finite authored corpus, not an admitted
production reader. Their independent expected-reference defect is reported
under F-291 rather than excused by the green gate.

D.R.Y./Haskell-bar and category pass: explicit relation sums improve the
evidence structure and the removed API has no spare implementation. F-291
is the remaining duplication of a directional claim inside a free-text
reference identity; copied expectations preserve that contradiction.
X5 retains the one production door and its eventual type ownership.
No added application/test comments in the repair or X3 range; the current
X0 endpoint comparison's point-key doc comment is retained older code,
not a new X3 addition. Relevant diff checks are clean. No dependency,
public type, wire, contract or golden change is made by this review.

The foreign atlas heavy lock is still C12's `codex-maps-land2`, `e0ef86a1`,
since 20:09:32 EDT. Workspace, make-contract and fresh 8090 golden gates
were not independently run. No view/fixture was blessed, canon rebuilt,
port touched, foreign lock released or owner permission inferred for a
type change. No reviewer-owned lock, host or background job remains.
Own implementation LOC delta **0**.

Evidence: [checksummed payloads](evidence/2026-10-09-maps-x3-review/5681779/SHA256.json),
including the one gate log, read-only red/control, ten full KJV samples,
exact range diffs, source/author-log hashes and reproducible assertion audits.
Next: X3 repairs F-291's reference/category residual on its sole branch;
X5/producer blockers retain their existing owners. This verdict does not
approve the full X3 migration or silently close F-142.
