# MG-LOCAL: preserved history and migration id inventory

Owner-authorized O-MG-LOCAL work, 2026-10-07. The twelve-tribe golden stop is
**1399 BC**, with **1400 BC retained as an alternative**. That is an adopted
ruling, not a pending date question or approval to draw geometry.

Local map-generator master has been published and merged with origin/master
on **lane/codex/MG-LOCAL**. No master branch was changed or pushed, and no
history was rewritten. This is a preservation/inventory checkpoint for Claude
to review; it does not approve the inherited implementation or its fixtures.

## Histories preserved

| Input | Pinned commit | Outcome |
|---|---|---|
| WSL local master | 089a4bf899f9bd4da08061d710618e7efb2f7c90 | Published unchanged first; first merge parent |
| Windows master | 089a4bf899f9bd4da08061d710618e7efb2f7c90 | Same committed history; read only |
| Fetched origin/master | 63bb038897a036a75a9dce050676cc79c7d690aa | Second merge parent |
| Ordinary merge | 4d3200774a34c91cb4da1c708ec2415047559a09 | Both histories retained |

The two commits mentioned as “on Windows”, 91d089c and 089a4bf, are already
part of the WSL local history. There are **43 local-only and 2 remote-only
commits**, not 45 local-only commits. All 45 distinct commits were individually
checked as ancestors of the merge; every check passed. The merged application,
data and fixture tree is byte-for-byte the local committed tree: the only
paths added by the merge are **AGENTS.md and CLAUDE.md**, from origin/master.
There were no merge conflicts or source resolutions.

The Windows tracked working-tree diff is entirely CRLF representation:
`git diff --ignore-cr-at-eol --quiet` returned zero. Untracked files are outside
this publication. Nothing in that working directory was edited, staged,
cleaned or committed. Existing WSL master and origin/master retain their pinned
commits.

## Registry declarations: three aliases, no atlas-place/polity relocation

The merged `data/authored/registry.json` has exactly these declarations:

| Minted identity | Registry home | Kind | Migration meaning |
|---|---|---|---|
| partition:phoenicia | phoenicia | Polity | A second witness joins the existing atlas home; atlas Phoenicia does not move |
| authored:judea | basemap:judea | District | Map namespace unification; no existing atlas Place or Polity is its source |
| basemap:canaan | partition:canaan | Polity | Map namespace unification; no existing atlas Place or Polity is its source |

Each has its existing written reason. There are no repeated minted keys,
self aliases or declared chains. No minted alias is a key in the old atlas
place export or the vendored atlas polity inventory. The registry has no
Era/Map-kind declaration. Thus the declared inventory gives **zero atlas ids
homed away by these rows**, and **three map namespace aliases**. The absence of
a declaration is not proof that every producer obeys the registry: the runtime
`Identity::check` remains a build/compile gate for the integration with CX-M0.

Two homes retain excluded-source provenance: basemap:judea and partition:canaan.
Preserving their ids/history does not permit GPL or traced non-commercial
geometry to enter the new artifact. The maps spec's lineage/quarantine policy
must be enforced at migration. No alias was rewritten to disguise that lineage.

## The fifteen retired atlas places

Compared the merged map-generator export, root **a1b93a3b049a1fe0**, with the
current atlas export, root **fa95e31a4c84a41ac12339853cefc795**: **1373 → 1358**
place keys, fifteen removed and none added. Every removal matches an explicit
existing `MERGE_PAIRS` decision. Every named survivor exists in the current
export. This is an exhaustive comparison of those two pinned inputs, not a
name/proximity heuristic or a new identity ruling.

| Absorbed PlaceId | Survivor PlaceId | Succession inventory for C2 |
|---|---|---|
| ai_36 | ai-1 | AbsorbedInto |
| antioch_69 | antioch-2 | AbsorbedInto |
| antipatris_70 | aphek-2 | AbsorbedInto |
| appii_forum_427 | forum-of-appius | AbsorbedInto |
| bethlehem_218 | bethlehem-1 | AbsorbedInto |
| bethsaida_231 | bethsaida-2 | AbsorbedInto |
| calneh_269 | calneh-1 | AbsorbedInto |
| ephraim_401 | ephraim-2 | AbsorbedInto |
| lydda_741 | lod | AbsorbedInto |
| moreh_817 | moreh-1 | AbsorbedInto |
| olivet_907 | mount-of-olives | AbsorbedInto |
| phenice_945 | phoenix | AbsorbedInto |
| rehoboth-ir_992 | rehoboth-ir | AbsorbedInto |
| solomons_portico_1109 | solomon-s-portico | AbsorbedInto |
| ur_1189 | ur-1 | AbsorbedInto |

The original decision is atlas **0b29bf129786a43f78ce62cf40aee7dd7a845635**,
`server/atlas-core/src/merge.rs`; the recompilation/export checkpoint is
**67bd433b**. The table remains present at atlas **9da8a6c5** with each written
reason. The evidence JSON links every removed row to that source and retains
its old export record. It finds no unmatched removal and no missing survivor.

These are PlaceId keys, not manufactured content hashes. C2 can construct the
shared `IdSuccession`/`AbsorbedInto` rows through the graph's own id vocabulary,
with the original decision as provenance, then prove the element/route/trail
successor law. This checkpoint does not write that proposed file, change a
served id, or claim the successor route already exists. Survivor existence is
verified in the export; no current HTTP read or graph reconstruction was run.
The earlier Hazor and Kedesh pairs are not among these fifteen export removals.

## Verification, limits and next step

Evidence is in `docs/reports/evidence/2026-10-07-mg-local-integration/`.
The portable inventory script accepts `--atlas <atlas repo>` and reads pinned
Git blobs plus the root-stamped atlas export. It reproduces the JSON exactly.
It checks all inherited commit ancestry, the complete declaration set and all
fifteen removals/survivors. Input SHA-256 values are retained in the JSON.
`git diff --check` passes. A whole-tree comparison checks every tracked path
outside the two inherited agent documents and this report/evidence directory.

No production code was authored, so there is no invented red/green test packet.
No Rust build, contract gate, data rebuild, fixture blessing, renderer gate or
history purge was run. The inherited Windows-only graph-types paths remain:
CX-M0 is **not** an ancestor of this merge. The current scope did not authorize
folding it in or changing graph-types. Claude's next integration step is to
combine this reviewed lane with CX-M0 deliberately and run the appropriate
build/property/contract gates, before using it as the migration base. No
unanswered MAPS type or geometry gate is bypassed.

The 14b review of this change finds one registry source, one existing place
merge source and their derived inventory, with no copied application logic.
The 24a/24b sweep is scoped to preservation: ordinary merge ancestry and the
complete tree comparison fence accidental dropped commits or source edits.
Inherited application findings are not declared closed by a history merge.

Handoff: **MG-LOCAL is ready for Claude review**, on its one live lane. Continue
C2 from the fifteen explicit absorption pairs, not guessed aliases; preserve
source lineage and the owner-selected 1399/1400 choice. No Codex lock/build/host
was taken. Source/raw/cache, the Windows directory and both master branches
are untouched. Disk capacity is ample, so no cleanup was necessary.
