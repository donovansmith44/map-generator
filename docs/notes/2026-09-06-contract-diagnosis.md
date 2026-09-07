# The Diagnosis: contract v0.1 run against the world as it is

**Date:** 2026-09-06 (run and written 2026-09-07)
**Suite version:** `contracts/VERSION` = `0.1.0` (pre-release)
**Spec:** `docs/superpowers/specs/2026-09-06-map-api-contract-design.md`
**Plan:** `docs/superpowers/plans/2026-09-06-contract-stage0.md`
**Servers under test:** ours at `http://127.0.0.1:8090`, the Bible atlas at `http://127.0.0.1:8080`
**Commit under test:** `a382c68`

---

## Read this first: what a green row means, and what it does not

This document reports which laws hold. Before the tables, the one thing that
must not be misread:

**A green row means "the law, in the exact form this contract pins it, holds
against today's server." It does not mean "the spec's full algebra is
implemented."** In three places the contract deliberately pins something
*weaker* than the spec's §3 algebra, because v0.1 has no machinery for the
stronger form. Those places are named honestly in
[§6 Honest weaknesses](#6-honest-weaknesses-what-a-green-here-does-not-buy)
and you should read that section before drawing any conclusion about
what Stages 1–4 still have to build.

The short version of the caveat:

- **Composition** is checked as "the union render's resource set equals the
  union of the parts' resource sets" — not as a real `⊕` operator, because
  v0.1 has no server-side combine endpoint to test.
- **"Omission is subtractive, not destructive"** is green, but it asserts
  only `noWater ⊆ full`. A server that ignored the pieces parameter
  altogether would satisfy it too. It cannot tell "subtractive" from "no-op".
- **"Every manifest entry names its piece"** asserts that a field is
  *present*, never that its value is *right*.

---

## 1. The verdict tables, verbatim

### 1.1 map-api — against our server, `http://127.0.0.1:8090`

`cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api`

| feature | scenario | verdict |
|---|---|---|
| the scene — a picture composed from pieces | the twelve tribes scene, whole | ✅ green |
| the scene — a picture composed from pieces | a scene with no labels is still a scene — pinned whole | ✅ green |
| the scene — a picture composed from pieces | omission is subtractive, not destructive | ✅ green |
| the scene — a picture composed from pieces | rendering is deterministic | ✅ green |
| the scene — a picture composed from pieces | determinism holds for any piece subset at any year | ✅ green |
| the scene — a picture composed from pieces | every manifest entry names its piece | 🔴 red (expected — @target) |
| the scene — a picture composed from pieces | composition — pieces render separately and combine to the whole | 🟢 green (target already met!) |
| the scene — a picture composed from pieces | dress-locality — restyling one piece leaves the others untouched | 🟢 green (target already met!) |
| resources — geometry by content address | the same id fetched twice is byte-identical | ✅ green |
| resources — geometry by content address | a batch equals its singles | 🟢 green (target already met!) |
| the contract endpoint — a server declares what it speaks | the contract declaration is exactly its blessed body | ✅ green |
| subjects — what can be asked about at a moment | the twelve tribes era, whole | ✅ green |
| subjects — what can be asked about at a moment | the tetrarchies era, whole | ✅ green |
| subjects — what can be asked about at a moment | subjects are deterministic at any year | ✅ green |
| changes — the narrative between two instants | the conquest is a change the timeline knows | ✅ green |
| changes — the narrative between two instants | an empty span has no changes | ✅ green |
| the census — every disposition, queryable | the whole census at 1050 BC | ✅ green |
| the census — every disposition, queryable | the whole census at AD 59 | ✅ green |
| the census — every disposition, queryable | the whole census at the conquest | ✅ green |
| the census — every disposition, queryable | the census is deterministic at any year | ✅ green |

**Tally: 20 scenarios — 19 green, 1 expected red (`@target`), 0 unexpected red.**
The 19 green break down as 16 plain green plus 3 `@target` scenarios that are
already met.

### 1.2 atlas-edge (consumer-driven contract) — against the live atlas, `http://127.0.0.1:8080`

`cabal run contract-runner -- run --base-url http://127.0.0.1:8080 ../atlas-edge`

| feature | scenario | verdict |
|---|---|---|
| polities — the eras of governed ground we vendor | the whole polity book, as we consume it | ✅ green |
| narratives — the journeys we vendor | the whole narrative book, as we consume it | ✅ green |
| landmarks — named waters and places we label by | the whole landmark list, as we consume it | ✅ green |
| the land mask — the coastline our partition builds on | the whole mask, as we consume it | ✅ green |
| events — a leg's when, where, and why | a known leg event, as we consume it | ✅ green |
| eras — the named periods that resolve standings | the whole era table, as we consume it | ✅ green |

**Tally: 6 scenarios — 6 green, 0 red.**

### 1.3 The static gates and the surrounding batteries

| gate | command | result |
|---|---|---|
| totality (map-api) | `contract-runner check ../map-api` | `totality: every step has exactly one definition` |
| totality (atlas-edge) | `contract-runner check ../atlas-edge` | `totality: every step has exactly one definition` |
| vocabulary drift (map-api) | `contract-runner vocab ../map-api` | `vocabulary: every table matches its types` |
| vocabulary drift (atlas-edge) | `contract-runner vocab ../atlas-edge` | `vocabulary: every table matches its types` |
| runner's own tests | `cabal test` | `101 examples, 0 failures` — `Test suite spec: PASS` |
| Rust workspace | `cargo test --workspace` | 19 × `test result: ok`, **0** × `test result: FAILED` |
| **the golden gate** | `node crates/map-viewer/tests/golden.js --check` | **`ALL GOLDEN VIEWS HOLD`** (89/89 stops) |

**Stage 0 moved no pixels.** All 89 blessed stops hold. This was the hard
condition on the whole stage: the only production changes permitted were two
new read-only routes (`/api/contract`, `/api/census`), and the golden gate
confirms nothing else shifted.

---

## 2. The unexpected reds: there are none

**Ranked list of every non-`@target` red: (empty).**

This is a finding in its own right, not an absence of one. Sixteen laws were
written down as scenarios *without* being marked as known gaps — the whole
scene-omission and determinism family, content-addressing, the contract
declaration, the subjects table, the changes timeline, and the four census
laws — and **today's server satisfies every one of them.** The corpus was
written from the spec, not from the code; it was blessed against captured
output only after the shapes were fixed by the spec's requirements. Nothing
in it went red by surprise.

The plain reading: the *seam* the viewer already speaks — scene manifests,
content-addressed resources, subjects, changes — is behaviourally sound.
Rendering is deterministic at every year and every piece subset that
property-fuzzing reached. Ids really are their bytes. The census route added
in this stage is total and stable at all three sampled instants. **The disease
the spec diagnoses is not "the wire misbehaves." It is that the wire has no
vocabulary for the facts underneath it** — which is exactly what the one real
red says, and exactly what Stages 1–3 are for.

---

## 3. The one real gap

### `@target`: "every manifest entry names its piece" — RED, as predicted

- **The law.** Every entry in a scene manifest must say which piece it
  belongs to (ground, water, fills, borders, labels, journeys, …).
- **The observed behaviour.** v0.1 manifest entries carry no `piece` field at
  all. The runner reports: `manifest entries carry no piece attribution (v0.1 wart)`.
- **Why it matters.** Without piece attribution the manifest is an
  undifferentiated bag of geometry. A client cannot ask "show me only the
  borders" without re-rendering; the server cannot prove that omitting a
  piece removed *that piece's* contribution and nothing else. It is the
  missing link that makes the two weak laws in §6 weak.
- **Which stage retires it.** Spec §5 **Stage 4** — "retire the 13 routes
  (v0.5, the dogfood bar)", where the pieces parameter becomes real on the
  wire and `TopographyDress` lands as dress-locality's show-piece. The
  scene feature's own preamble records the wart: *"In v0.1 only ground,
  water, labels, and journeys are toggleable on the wire; the rest are always
  present — a wart this contract records rather than hides, retired when the
  pieces parameter lands."*
  Piece attribution is arguably cheaper than that and could be pulled
  forward — see §7.

That is the complete list of failing laws. One.

---

## 4. The headline: three of four predicted gaps are already closed

The corpus has exactly four `@target` scenarios. `@target` means *"the plan
predicted this would be red against today's server; a red here is expected
and does not fail the build."* **Three of the four are green.**

| `@target` law | expected red by | actual | what it means |
|---|---|---|---|
| composition — pieces render separately and combine to the whole | Stage 4 (spec §3 law 2; pieces parameter) | 🟢 **GREEN** | The union of two piece sets renders exactly the union of their geometry. No combine work needed to satisfy the pinned form. |
| dress-locality — restyling one piece leaves the others untouched | Stage 4 (spec §3 law 3; `TopographyDress`) | 🟢 **GREEN** | Style genuinely rides styles, not payloads: re-rendering canaan → slate changed **not one geometry id**. Content-addressing already separates dress from geometry. |
| a batch equals its singles | Stage 0/4 (content-addressing laws) | 🟢 **GREEN** | `/api/resources?ids=a,b` is byte-for-byte the concatenation of `/api/resource?id=a` and `?id=b`. Exact byte equality, not shape equality. |
| every manifest entry names its piece | Stage 4 | 🔴 **RED** | The one real gap. See §3. |

**This reshapes the staged migration.** The plan's original guess was that
Stage 0 would find the scene algebra substantially unimplemented and that
Stages 3–4 would have to build it. Instead:

- The **content-addressing substrate is finished.** An id is its bytes; a
  batch is its singles; a restyle changes no ids. Stage 4 does not need to
  build this — it needs to *not break* it, which the golden gate and this
  suite will both judge.
- **Dress-locality is already true of the geometry layer.** When
  `TopographyDress` lands (spec §5 Stage 4), it is landing on ground that
  already obeys the law it was supposed to demonstrate. That is a much
  smaller, much safer piece of work than planned.
- The **remaining scene work is attribution, not algebra.** The pieces are
  already separable in practice; the server just does not *say so* in the
  manifest.

Read carefully, though — the composition green is the one that carries the
biggest asterisk. See §6.

---

## 5. The atlas edge: provider verification PASSES today

The `atlas-edge` suite is a **consumer-driven contract (CDC)**: it is our
written statement of exactly what our parsers need from the six atlas
endpoints we consume — `/api/polities`, `/api/narratives`, `/api/event/:id`,
`/api/eras`, `/api/landmarks`, `/api/land-mask`. Not what those endpoints
happen to return; what we would *break without*.

**All six scenarios are green against the live atlas at `:8080`.** In CDC
terms this is a passing **provider verification**: the atlas as it stands
today satisfies every promise we depend on. There is no drift, no missing
field, no shape surprise.

Two things worth stating plainly:

1. **This suite is the artifact to hand to the atlas session.** It compiles to
   a standalone binary that runs against `:8080` with no access to our
   workspace and no Rust toolchain. If the atlas adopts it in their CI, any
   future change on their side that would break our parsers goes red *in
   their build*, before it reaches us. That is the whole point of a
   consumer-driven contract, and it is ready to hand over now.
2. **The projections were corrected against the real code, not the plan.**
   The plan's original description of what we consume was wrong in several
   places; the shapes in these features were rebuilt field-for-field against
   `crates/map-compile/src/vendor.rs` (lines 105–237) and re-verified
   independently at review. One concrete correction: the event endpoint's
   `label` is derived from `title` falling back to `label`, not two separate
   raw fields as first written. So this suite states what we *actually*
   parse.

---

## 6. Honest weaknesses: what a green here does *not* buy

Every one of these was known at the time it was written, argued at review,
and recorded. None is a defect discovered after the fact. They are here
because a diagnosis that overstates its own strength is worthless.

### 6.1 Composition is checked in a weaker form than the spec's `⊕`

Spec §3 law 2 says `scene(a ⊕ b) = scene(a) ⊕ scene(b)`, with `⊕` an
associative operator with the empty scene as identity. **v0.1 has no
server-side combine, so there is no `⊕` to test.** What the runner actually
does (`Steps.hs`, the `combining` step) is:

> render the *union of the two piece sets* at the same year and style, and
> assert that its set of resource ids equals the *set union* of the two
> parts' resource id sets.

That is a real and useful law — it says pieces do not interfere, and it is
fuzzed across generated piece sets and years — but it is **resource-set
union, not scene composition**. It says nothing about paint order (spec §3
requires paint order to come from the precedence *law*, never from
composition order), nothing about associativity as an operator, and nothing
about the identity element beyond what the empty-piece-set generator reaches.
The true law becomes testable only when a combine exists.

### 6.2 "Omission is subtractive, not destructive" is satisfiable by a no-op server

The scenario renders the full scene and a scene without water, then asserts
`noWater's resources ⊆ full's resources`. Subset. **A server that ignored
the pieces parameter entirely, returning the identical full scene both
times, would pass this scenario.** It cannot distinguish "water was
subtracted" from "nothing happened".

This is the same wart as §3 seen from the other side: without piece
attribution on manifest entries there is no way to assert *the water and
only the water went away*. Strengthening it is cheap once attribution lands —
`full − noWater` should be exactly the water-attributed entries, and nothing
else should move. **Flagged for the owner: this is a scenario whose name
promises more than its assertion delivers.**

### 6.3 "Every manifest entry names its piece" asserts presence, not correctness

The `@target` in §3 checks that a `piece` field exists on every feature
entry. It does not check that the value is *right* — that the coastline is
attributed to water and not to journeys. When Stage 4 makes this green, the
scenario must be strengthened at the same time or it will bless a server that
stamps `piece: "ground"` on everything. (The step does at least refuse to
pass vacuously on an empty manifest — an earlier version would have reported
the target met when there were no entries to check.)

### 6.4 `@property` scenarios have NO SHRINKING

Spec §4 requires it explicitly: *"`<angle-bracket>` holes whose `FromCapture`
types carry QuickCheck `Gen` **+ shrink**; the scenario becomes
**`forAllShrink`** over generated bindings."* **Neither the Stage 0 plan's own
code nor the implementation provides shrinking.** The four `@property`
scenarios (determinism at any year/subset, composition, subjects
determinism, census determinism) generate bindings, run N deterministically
seeded iterations, and on failure report **whatever raw binding that
iteration happened to produce** — an arbitrary counterexample, not a minimal
one.

Why it was deferred rather than fixed: it needs restructuring around
QuickCheck's `Property` machinery, it interacts with the deterministic
per-iteration seeding that this diagnosis's reproducibility depends on, and —
decisively — its absence makes the diagnosis **less sharp, never less
honest**. An unshrunk counterexample is still a true counterexample. Stage 0
is measurement, and no verdict in §1 depends on shrink quality.

**This is real outstanding work and must be carried into the Stage 1–4
planning pass.** The moment a property law goes red on a large generated
piece set, the absence of shrinking is what will make the root cause slow to
find.

### 6.5 Two more small honesties

- The property generators are deterministic by construction (fixed seed
  derived from the iteration index), so these verdicts reproduce exactly.
  One test — the empty-piece-set codomain check — deliberately uses real
  entropy and is exempt.
- The scene fixtures are large (~2.0–2.4 MB, 9339 features each) because the
  whole-body law pins entire manifests rather than poking at fields. That is
  the owner's decree working as intended, not a blessing bug, but it is a
  real cost in repository size that Stage 1+ should keep an eye on.

---

## 7. What Stage 0 found by construction

Stage 0's stated purpose was to measure the server. Its *other* yield — and
in raw defect count, its larger one — is what the contract machinery found
in **itself** while being built. Each of these is a case where a check would
have reported clean while being unable to detect the thing it existed to
detect. They are recorded because the same failure mode is what Stages 1–4
must keep out of their own tests.

1. **Cross-definition step ambiguity, prevented only by list order.** The
   plan claimed ambiguity was "unrepresentable by construction". It was not:
   that guarantee held *within* one step pattern, never *across* two step
   definitions sharing a literal. On its first real run the ambiguity
   detector found two live collisions — including that **every**
   `I render pieces … as <name>` line was claimed by both render
   definitions, with only declaration order deciding the winner. Fixed
   structurally (a three-state match result: no-match / claimed-with-error /
   matched), so "exactly one definition" is now true by construction rather
   than by luck of ordering.

2. **A totality law that detected only orphans, never ambiguity.** The
   checker originally reported steps with *zero* definitions and said nothing
   about steps with *two*. Half a law. Now: 0 = orphan, 1 = correct,
   2+ = ambiguous and named.

3. **A never-failing URL capture that let a missing step report clean.** The
   line `I GET /api/subjects?year=<someYear> as first` was reported **clean**
   by the totality check even though the step it needed did not exist. The
   URL capture's parser was `Right . strip` — it could not fail — so it
   silently swallowed `" as first"` as part of the URL. *A capture that never
   fails has no discriminating power, so the law had nothing to catch.* Fixed
   by type: a `UrlPath` capture that rejects whitespace.

4. **An HTTP transport that returned error pages as success.** Neither
   transport inspected the response status. A 404 or 500 body came back as
   `Right`. The consequence was concrete and non-hypothetical: the scenario
   *"fetching a resource twice yields identical bytes"* — **not** a `@target`,
   expected green — would have reported **green against a server with no
   resource endpoint at all**, because both fetches returned the identical
   error page. A law passing against a broken server is worse than no law.
   Fixed: non-2xx is `Left`, naming status and URL, in both transports.

5. **A vocabulary writer that partially rewrote a corpus on failure.** In
   `--write` mode the tool wrote each file as it went, and only afterwards
   checked whether any file had failed to parse. A directory with one bad
   feature got its *other* files physically rewritten and then reported
   failure. Fixed by restructuring into two phases — parse everything, and
   write nothing at all unless every parse succeeded.

6. **Several tests that could not fail.** Found and fixed across the stage:
   an ambiguity test using two identically-named definitions (so it passed
   even if the detector named the wrong things); a URL test asserting
   substring presence (so it passed for a URL with duplicated flags or
   injected parameters); an equality test whose fake transport returned the
   same body for every URL (so it passed against a stub that always
   succeeded); a piece-attribution step that passed vacuously on an empty
   manifest (so an empty response reported the `@target` already met); a
   sortedness test that passed with the sort deleted. Each was replaced with
   an assertion that genuinely discriminates, plus a negative case.

7. **Two tuned constants and one wrong sort order.** A `resize 4` cap that
   permanently made tables of more than four rows unreachable at any
   QuickCheck size (replaced with a scaling rule that keeps every shape
   reachable); a hardcoded style name in the combine step at the very line
   where the year had just been threaded through properly (replaced by a
   typed field carrying the last render's year *and* style); and a piece-set
   renderer sorting by declaration order while its sibling sorted
   alphabetically — invisible because the one existing example's two orders
   coincided.

The through-line, and the reason these belong in the permanent record: **the
owner's standing law — "a check satisfiable by the failure mode isn't a
check" — caught six distinct instances of itself inside the very tooling
built to enforce it.** Every one was found by making a law real and running
it, not by reading code. That is the argument for continuing the discipline
into Stages 1–4.

---

## 8. Recommendation: what Stage 1 should absorb first

Argued from the evidence above, not from the plan's original guesses.

### 8.1 Keep the spec's ordering. The evidence supports it.

Spec §5 orders the stages registry → ledger → arrangement → route retirement,
on the reasoning that rows need identities and claim resolution needs
dispositions. **Nothing in this diagnosis disturbs that.** The census route
built in this stage is green and total at every sampled instant, which is
direct evidence that the disposition table is a coherent thing to build a
ledger around. Start Stage 1 where the spec says: the entity registry.

### 8.2 The first thing Stage 1 should absorb: **piece attribution on manifest entries** — pulled forward out of Stage 4

This is the recommendation that departs from the plan.

Piece attribution is scheduled for Stage 4. It should move to Stage 1, or be
done alongside it. The evidence:

- It is the **only failing law in the entire corpus.** Nothing else is red.
- It is the **blocker on two other laws' honesty**, not just its own. §6.2's
  subset assertion and §6.3's presence-only assertion both stay weak until
  attribution exists. Landing it unlocks *three* strengthenings for one piece
  of work.
- It is **cheap relative to what Stage 4 pairs it with.** Stage 4 also
  deletes 13 routes and moves the viewer onto contract functions. Attribution
  needs none of that: it is a field on manifest entries the server already
  knows how to compute, because it already renders the pieces separately —
  §4's composition green proves the separation is real *today*.
- It **de-risks Stage 4** by moving the one genuinely unknown piece of scene
  work earlier, where the golden gate can judge it in isolation rather than
  alongside a route deletion.

Concretely: add `piece` to every manifest entry; strengthen "omission is
subtractive" from `noWater ⊆ full` to *`full − noWater` is exactly the
water-attributed entries*; strengthen the `@target` from presence to correct
values; then drop the `@target` tag on all three.

### 8.3 Second: hand the atlas-edge suite over now, not at Stage 4

Spec §6 lists "atlas-edge CDC green against `:8080` and handed to the atlas
session" as a **Stage 4 exit** checkable. It is green *now* (§5). Handing it
over at Stage 1 costs nothing and buys real protection: our Stages 1–3 are
going to be a long stretch of internal churn, and a provider-side contract
running in the atlas's CI throughout that window means we find out about
upstream drift when it happens rather than at Stage 4 when we are least able
to absorb it. Note the standing constraint: the atlas repo is a read-only
path dependency for this session, so this is a hand-off to the atlas session,
not work we do.

### 8.4 Third: shrinking, before the property corpus grows

§6.4. Right now there are four `@property` scenarios and all four are green,
so the absence of shrinking costs nothing. Stage 1 adds `entities`, `node`,
`edge_summary`, and `edges` — paging laws and identity laws are exactly the
kind that get fuzzed hard, and exactly the kind whose counterexamples are
large and structured. **Do the `forAllShrink` restructuring while the
property corpus is still four scenarios**, not after it is twenty. It is
listed in spec §4 as a requirement and it is currently unmet; it should not
enter Stage 1's planning as a discovery.

### 8.5 What Stage 1 does *not* need to build

Worth saying explicitly so the Stage 1 plan does not budget for it:

- **A combine endpoint.** Not needed for anything Stage 1 does, and the
  weakened composition law (§6.1) does not depend on one. Defer it to
  whichever stage genuinely needs `⊕` on the wire.
- **Content-addressing work.** Finished. Ids are their bytes, batches are
  their singles, restyles change no ids. Stage 1's job is to leave it alone.
- **Scene algebra rework.** The pieces already do not interfere. What is
  missing is the manifest's *vocabulary* for saying so — see §8.2.

### 8.6 The gates Stage 1 inherits

All four verification strata from spec §6 are live and green as of `a382c68`,
and every one of them must stay green through Stage 1:

- Rust unit and law tests: `cargo test --workspace` — 19 suites ok, 0 failed.
- The contract suite: `contract-runner run` — map-api 19/20 with the single
  declared target, atlas-edge 6/6.
- Census diffs: the census route exists and is stable; spec §5 requires
  Stage 1's diff to *be* the identity unification (merged rows, reviewed name
  by name) — the first non-empty diff the owner reviews.
- The golden gate: `ALL GOLDEN VIEWS HOLD`, 89/89.

---

## Appendix: how to reproduce every number in this document

```bash
cd contracts/runner
cabal run contract-runner -- check ../map-api
cabal run contract-runner -- check ../atlas-edge
cabal run contract-runner -- vocab ../map-api
cabal run contract-runner -- vocab ../atlas-edge
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
cabal run contract-runner -- run --base-url http://127.0.0.1:8080 ../atlas-edge
cabal test

cargo test --workspace

# the golden gate — run from the job tmp dir so playwright-core resolves
node crates/map-viewer/tests/golden.js --check
```

Toolchain: GHC 9.12.1, cabal 3.18.1.0. The runner forces UTF-8 on stdout, so
the em dashes above survive any console code page. Requires both servers
live: ours on `:8090`, the atlas on `:8080`.
