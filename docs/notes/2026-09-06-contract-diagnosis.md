# The Diagnosis: contract v0.1 run against the world as it is

**Date:** 2026-09-06 (run and written 2026-09-07; **corrected and re-run 2026-09-07** — see the correction notice below)
**Suite version:** `contracts/VERSION` = `0.1.0` (pre-release)
**Spec:** `docs/superpowers/specs/2026-09-06-map-api-contract-design.md`
**Plan:** `docs/superpowers/plans/2026-09-06-contract-stage0.md`
**Servers under test:** ours at `http://127.0.0.1:8090`, the Bible atlas at `http://127.0.0.1:8080`
**Commit under test:** `7cd31bd` (was `a382c68` in the first version — see the correction notice)

---

> ## CORRECTION NOTICE — this document's original headline was wrong
>
> The first version of this file (commit `8e7a4af`) reported **three of four**
> `@target` laws as "already met" and built its recommendation on that. One of
> those three greens — **composition** — was an artifact of a defective check,
> not a property of the server. The check could not fail. It has been fixed
> (commit `7cd31bd`) and **composition is genuinely RED**.
>
> Everything below is re-run and re-derived against the fixed suite. The
> corrected headline is **two of four**, and the two reds are plausibly the
> same underlying gap. What happened, and why it matters more than any
> individual verdict, is recorded in [§7.0](#70-the-one-that-got-through-a-check-that-could-not-fail-reported-as-a-finding).

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
  v0.1 has no server-side combine endpoint to test. Even in that weakened
  form it is **red**, which makes the red stronger news, not weaker: the
  server fails a law that is easier than the one the spec actually asks for.
- **"Omission is subtractive, not destructive"** is green, but it asserts
  only `noWater ⊆ full` — one hardcoded piece at one hardcoded year. It
  cannot tell "subtractive" from "no-op", and §6.2 now shows it is
  **false today if you substitute `journeys` for `water`**. It is green
  because of the example it picked.
- **"Every manifest entry names its piece"** asserts that a field is
  *present*, never that its value is *right*.

And one more, added by the correction: **a green row is only as good as the
check behind it.** One green in the first version of this document was
produced by a check that no server could have failed. See §7.0.

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
| the scene — a picture composed from pieces | composition — pieces render separately and combine to the whole | 🔴 red (expected — @target) |
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

**Tally: 20 scenarios — 18 green, 2 expected red (both `@target`), 0 unexpected red.**
The 18 green break down as 16 plain green plus 2 `@target` scenarios that are
already met. The runner exits 0: a `@target` red is declared, not a build
failure.

*(Was 19 green / 1 red in the first version of this document. The row that
moved is composition — see the correction notice and §7.0.)*

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
| runner's own tests | `cabal test` | `112 examples, 0 failures` — `Test suite spec: PASS` |
| Rust workspace | `cargo test --workspace` | 19 × `test result: ok`, **0** × `test result: FAILED` |
| **the golden gate** | `node crates/map-viewer/tests/golden.js --check` | **`ALL GOLDEN VIEWS HOLD`** (89/89 stops) |

**What was re-run for this correction, and what was not.** The four contract
commands (both `run`s, both `check`s, both `vocab`s) and `cabal test` were
re-run at `7cd31bd` and the numbers above are from that run — `cabal test`
rose from 101 to 112 examples because the fix added tests for the defect it
closed. The Rust and golden-gate rows are carried forward unchanged from the
`a382c68` run: the fix touched only `contracts/`, no production code and no
pixels, so re-running them would confirm the same numbers. They are marked
here as carried forward rather than re-observed, so nobody reads them as
fresh evidence.

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

The plain reading, stated more carefully than the first version of this
document stated it: the *transport* the viewer already speaks — scene
manifests, content-addressed resources, subjects, changes — is sound.
Rendering is deterministic at every year and every piece subset that
property-fuzzing reached. Ids really are their bytes. The census route added
in this stage is total and stable at all three sampled instants.

**But "no unexpected reds" is not "no problems."** Both `@target` reds are
about the same thing, and §3 argues they are one gap seen twice: the server
does not model pieces as first-class. That gap was *predicted* — which is why
it is declared rather than surprising — but the first version of this
document reported half of it as already closed, and it is not. The
disease the spec diagnoses is not only "the wire has no vocabulary for the
facts underneath it"; it is also that **the pieces genuinely interfere**, and
§3.2 has the counterexample.

One near-miss worth recording, because it changed the corpus after the first
run: `Year`'s generator could draw **0**, and the server rejects year zero
(there is no year 0 — 1 BC is followed by AD 1). A drawn 0 would have turned
two non-`@target` determinism properties red at random, on a schedule nobody
controlled. Year 0 is now excluded from the `Year` type, its generator, and
its Vocabulary description. This is not a server finding; it is a corpus
finding, of the same family as §7.

---

## 3. The two real gaps — and the case that they are one

### 3.1 `@target`: "every manifest entry names its piece" — RED, as predicted

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
  forward — see §8.2.

### 3.2 `@target`: "composition — pieces render separately and combine to the whole" — RED

**This row was reported GREEN in the first version of this document. That was
wrong.** The check behind it could not fail; §7.0 records how. With the check
fixed, the law fails, and it fails for a concrete and reproducible reason.

- **The law, in the form the contract pins it.** Render piece set A, render
  piece set B, then render A ∪ B. The union render's set of resource ids must
  equal the union of the two parts' resource id sets. (Weaker than the spec's
  `⊕`; see §6.1.)
- **The counterexample the runner reported.**

  ```
  someA    = borders, fills, ground, veil, water
  someB    = borders, chrome, claims, fills, ground, journeys,
             labels, markers, veil, water
  someYear = -1059
  message  = "union scene is not the union of its parts' resources"
  ```

- **Reproduced by hand, outside the runner, against the live server.** Only
  four pieces are toggleable on the wire in v0.1 (ground, water, labels,
  journeys — the wart the scene feature's own preamble records), so on the
  wire A is "the full scene minus labels and journeys" and B ∪ A is just B.
  Fetching both scenes directly:

  ```
  |A| = 6856 resources    |B| = 6857    |A ∪ B| = 6858
  resources in A that are absent from B: 1   (fb387872526ea52b)
  ```

  So the union render is *missing a resource that one of its own parts
  produced*. Narrowing further, at the same year and style, shows exactly
  which piece causes it:

  ```
  render minus labels    → every resource also appears in the full render
  render minus journeys  → one resource does NOT appear in the full render
  ```

  The diverging resource is a `points` buffer: **26 vertices / 360 bytes**
  with journeys off, **33 vertices / 444 bytes** with journeys on. The two
  are not prefixes of one another — they are different payloads, hence
  different content addresses.

- **What that means in plain language.** The server packs the points
  contributed by *more than one piece* into a single shared buffer. Turning
  journeys off does not remove a journeys-shaped thing and leave the rest
  alone; it changes the contents of a buffer that other pieces are also
  using, which changes that buffer's content hash, which makes it a
  different resource. **The pieces are not independent. They interfere.**

### 3.3 The case that these are one gap, not two

Stated as an argument from the evidence, not as an assumption:

1. Piece attribution is missing because the manifest has no per-entry notion
   of which piece an entry belongs to (§3.1).
2. Composition fails because a single resource is jointly produced by more
   than one piece (§3.2).

(2) is a *reason* for (1): you cannot stamp one `piece` value on a manifest
entry whose geometry was contributed by two pieces at once. And (1) is why
(2) went undiagnosed for so long: without attribution there is no way to ask
the server which piece owns a buffer, so the interference is invisible from
the wire.

The single underlying statement both reds are making is: **the server does
not model a piece as a first-class thing with its own geometry.** It models
a set of render flags that jointly determine one output. That is one gap, and
§8.2 argues from it.

**Caveat, so the owner is not oversold.** This is an inference from one
narrowed counterexample plus the absence of attribution. It has *not* been
confirmed by reading the renderer — this correction pass was forbidden from
touching `crates/`, and no such reading was done. It is a strong hypothesis
with a reproducible symptom, not an established fact about the code. Confirming
it (or refuting it) is a small, well-defined first task for whoever picks up
§8.2, and it should be done before any Stage 1 estimate leans on it.

That is the complete list of failing laws. Two, and probably one.

---

## 4. The headline: two of four predicted gaps are already closed — the dress half, not the pieces half

The corpus has exactly four `@target` scenarios. `@target` means *"the plan
predicted this would be red against today's server; a red here is expected
and does not fail the build."* **Two of the four are green.**

| `@target` law | expected red by | actual | what it means |
|---|---|---|---|
| composition — pieces render separately and combine to the whole | Stage 4 (spec §3 law 2; pieces parameter) | 🔴 **RED** | The union render is missing a resource one of its own parts produced. Pieces share buffers and interfere. See §3.2. **This row read GREEN in the first version of this document; that green was an artifact — §7.0.** |
| every manifest entry names its piece | Stage 4 | 🔴 **RED** | Manifest entries carry no `piece` field at all. See §3.1. |
| dress-locality — restyling one piece leaves the others untouched | Stage 4 (spec §3 law 3; `TopographyDress`) | 🟢 **GREEN** | Style genuinely rides styles, not payloads: re-rendering canaan → slate changed **not one geometry id**, *and* the two bodies genuinely differ once geometry ids are masked. Both halves of the law's own name are proved. |
| a batch equals its singles | Stage 0/4 (content-addressing laws) | 🟢 **GREEN** | `/api/resources?ids=a,b` is byte-for-byte the concatenation of `/api/resource?id=a` and `?id=b`. Exact byte equality, not shape equality. |

The two greens are stronger than the first version claimed, and the two reds
are the whole story:

- **Dress-locality is now proved in both directions.** The original check
  asserted only that geometry ids were unchanged across a restyle — which a
  server that ignored `style=` entirely would also satisfy. It was
  strengthened in `7cd31bd` to *also* require that the two response bodies
  genuinely differ once geometry ids are masked out. It stayed green under
  the stricter check, against live server bodies. So: the dress really
  changed, and the geometry really did not.
- **The content-addressing substrate is finished.** An id is its bytes; a
  batch is its singles; a restyle changes no ids. Stage 4 does not need to
  build this — it needs to *not break* it, which the golden gate and this
  suite will both judge.

**How this reshapes the staged migration.** The plan's original guess was
that Stage 0 would find the scene algebra substantially unimplemented. The
first version of this document then over-corrected in the opposite
direction, claiming the algebra was essentially already there and only its
*vocabulary* was missing. The corrected reading sits between them and is
sharper than either:

- The **dress axis is done.** Style is separated from geometry, and proved
  so. `TopographyDress` lands on ground that already obeys the law it was
  meant to demonstrate.
- The **pieces axis is not done at all.** It is not merely unspoken (the
  first version's claim); it is untrue. Pieces share resources, so they
  cannot be attributed and cannot be composed. Both `@target` reds are that
  one fact.

That is a *cleaner* result than "three of four green" was, because it points
at one thing to build instead of one thing to say.

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

   **Corrected in this pass, and worth recording as its own small lesson.**
   That correction reached the *projection trees* — the machine-checked part
   — but only one of the six features' English preambles was updated with
   them. The drift law (`vocab`) protects the Vocabulary table against the
   types; **nothing protects the prose.** Three preambles were still
   describing the old, wrong projections, and this section's claim that "this
   suite states what we actually parse" was true of the code and false of the
   sentences a provider engineer reads first. Since this suite is the artifact
   we hand to another team, the prose is not decoration — it is the interface.
   Now corrected against `vendor.rs`, field by field:

   | feature | prose said | `vendor.rs` actually keeps |
   |---|---|---|
   | `eras.feature` | "id and from_year per era" | `id`, `name`, `from_year`, `to_year` |
   | `landmarks.feature` | "name and kind per row" | `name`, `kind`, `lat`, `lon` |
   | `polities.feature` | "id, name, from, to, and rings" | those five plus `color_key`, `transition.verses`, `fall.verses` |

   Scenarios, steps, and Vocabulary blocks were untouched; all six scenarios
   are still green and both static gates still pass. Note for the next
   planning pass: **a feature preamble is unverified prose.** There is no law
   that catches it drifting, and it drifted within one stage.

---

## 6. Honest weaknesses: what a green here does *not* buy

These were known at the time they were written, argued at review, and
recorded, and they are here because a diagnosis that overstates its own
strength is worthless.

**One correction to that framing, which the first version got wrong.** It
opened by claiming *"None is a defect discovered after the fact."* That is no
longer true and the sentence has been removed. §7.0 is exactly such a defect:
discovered after this document had already published its consequence as a
headline. The honest version of the claim is narrower — *these particular
weaknesses* were known when written; the corpus also contained one that was
not.

### 6.1 Composition is checked in a weaker form than the spec's `⊕` — and fails even so

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

**The direction of the weakness matters now that the row is red.** A weak law
going green tells you little. A weak law going *red* tells you a lot: the
server fails a strictly easier obligation than the one the spec imposes, so
the spec's `⊕` cannot possibly hold either. Whatever `⊕` eventually costs, it
is not the *first* thing that needs fixing — §3.2's shared buffers are.

There is also a second reason this check was weak until `7cd31bd`, and it is
not a design trade-off but a defect: the two generated piece sets were bound
to the same value on every iteration, so the "union" being tested was always
a set with itself. That is §7.0.

### 6.2 "Omission is subtractive, not destructive" is green only because of the piece it happened to pick — the law is FALSE today

The scenario renders the full scene and a scene without water, then asserts
`noWater's resources ⊆ full's resources`. Subset. **A server that ignored
the pieces parameter entirely, returning the identical full scene both
times, would pass this scenario.** It cannot distinguish "water was
subtracted" from "nothing happened".

This is the same wart as §3 seen from the other side: without piece
attribution on manifest entries there is no way to assert *the water and
only the water went away*. Strengthening it is cheap once attribution lands —
`full − noWater` should be exactly the water-attributed entries, and nothing
else should move.

**New in this correction, and worse than the paragraph above says.** The
scenario is not merely *weak*; it is green because of the specific piece it
names. Running the identical law at the identical year and style, changing
only which piece is omitted:

```
year -1405, style canaan
  omit water     → noWater's resources ⊆ full's resources    ✅  (what the scenario pins)
  omit journeys  → one resource in noJourneys is ABSENT from full   ❌
```

**Written with `journeys` instead of `water`, this scenario would be red
today.** It is the same shared-buffer interference as §3.2, surfacing through
a second law. So this is no longer a "weak assertion" note for the record —
it is a live, reproducible bug in the server that the corpus is one word away
from catching, and the property-fuzzing that *would* have caught it lives in
the composition scenario, which could not fail.

**Flagged for the owner: this scenario's name promises more than its
assertion delivers, and the gap between the two is currently hiding a real
defect.** Rewriting it as a `@property` over generated piece sets — rather
than one hardcoded omission — should be part of the same work as §8.2.

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

**§3.2 is now a worked example of the cost.** The composition counterexample
the runner reported is:

```
someA = borders, fills, ground, veil, water
someB = borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water
```

Fifteen piece mentions across two sets. The *minimal* counterexample, which
this document had to find by hand afterwards, is one piece: **journeys**.
Every other piece in that report is noise. Shrinking would have printed the
one-word answer; instead it took a manual narrowing pass to get from the
report to the diagnosis in §3.2. That narrowing was affordable here because
there were only four toggleable pieces. It will not be affordable over
Stage 1's paging and identity laws.

**This is real outstanding work and must be carried into the Stage 1–4
planning pass.** The moment a property law goes red on a large generated
piece set, the absence of shrinking is what will make the root cause slow to
find.

### 6.5 Two more small honesties

- The property generators are deterministic by construction, so these
  verdicts reproduce exactly. One test — the empty-piece-set codomain check —
  deliberately uses real entropy and is exempt. **Corrected in `7cd31bd`:**
  the seed used to be derived from the iteration index *alone*, which meant
  every hole in a scenario drew from the same seed and therefore got the same
  value. It is now derived per hole as well as per iteration. That single
  line is what §7.0 is about.
- `Year`'s generator no longer draws **0**. There is no year zero (1 BC is
  followed by AD 1) and the server rejects it, so a drawn 0 would have made
  two determinism properties red at random. Excluded from the type, the
  generator, and the Vocabulary description together, so the three cannot
  disagree.
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

### 7.0 The one that got through: a check that could not fail, reported as a finding

**Read this one first. It is the most instructive thing in Stage 0.**

Everything in the numbered list below was caught *before* it reached the
owner. This one was not. It is here at the head of the section because the
difference is the whole lesson.

**What the defect was.** The composition scenario has two generated holes,
`<someA>` and `<someB>` — two piece sets, meant to be different, so that
rendering their union can be compared against rendering them apart. Both holes
were registered to the same generator, and the seed handed to that generator
was derived from **the iteration index alone**. Generation is pure. Same seed,
same generator, same value — so on every single iteration, `<someA>` and
`<someB>` were bound to the *identical* piece set.

**What that made the check assert.** The scenario computed the union of two
identical sets and compared it against the union of their two identical
resource sets. In symbols:

```
x  ==  x ∪ x
```

which is true for every `x`, on every server, forever. A server that returned
the same scene for every request would have passed. A server with no scene
endpoint at all would have passed, once the response was a scene-shaped thing.
**The check had no discriminating power whatsoever.** It reported green
because green was the only value it could produce.

**How far it got.** It survived the task that wrote it. It survived that
task's review. It survived every subsequent per-task review of the stage. It
survived the blessing pass. It was then run, reported green, and **published
as a finding in the first version of this document** — where it became one of
the "three of four targets already met", and then a load-bearing premise in
the recommendation section, which argued that piece work was cheap *because*
composition's green proved the server already rendered pieces separably. It
was caught only by the final whole-branch review, reading the corpus as a
whole rather than task by task.

**Why the per-task reviews all missed it.** Each review asked the natural
question — *is this scenario's assertion correct?* — and the assertion **was**
correct. `union(A,B) == resources(A) ∪ resources(B)` is exactly the law the
spec wants. The defect was not in the assertion; it was in the *bindings*, one
level away, in a seeding line that looked like plumbing. Nothing you could see
by reading the scenario, and nothing you could see by reading the assertion.
You could only see it by asking a different question: **"what inputs does this
check actually receive?"**

**The three lessons, stated plainly, because they generalise:**

1. **A green from a property scenario is worth nothing until you have seen
   its inputs vary.** For example-based scenarios the inputs are on the page.
   For generated ones they are not, and "the assertion is right" does not
   imply "the assertion is being asked anything". Every `@property` scenario
   needs a check that its holes actually take different values — cheap to
   write, and it would have caught this on day one.

2. **The owner's standing law is not only about assertions.** *"A check
   satisfiable by the failure mode isn't a check"* was applied thoroughly to
   the seven items below, all of which are assertion-shaped. This defect was
   the same law violated on the *input* side, and that side had no
   discipline attached to it. The law needs both halves: an assertion that can
   fail, **and** inputs that can make it fail.

3. **Per-task review cannot catch a whole-corpus defect.** Every review that
   saw this scenario saw it in isolation, and in isolation it was fine. It
   took a review whose unit was the entire branch. That is an argument for
   keeping a whole-branch review as a standing gate in Stages 1–4, not as an
   optional final polish — it is the only review that found this, and it found
   it after the document had already been written.

**And the meta-lesson, which is the uncomfortable one.** This document's §7
existed, before the correction, to celebrate that the discipline had caught
six could-not-fail checks inside the tooling. It was itself carrying a seventh
that the discipline had missed — and reporting that seventh's output as a
headline finding. A record of self-scrutiny is not evidence of completeness.
**Stage 0's most reliable output is not any single verdict; it is the count of
checks that turned out to be unable to fail — now seven — and there is no
reason to believe that count is final.**

---

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
built to enforce it, and missed a seventh (§7.0) until the final review.**
Every one of the six was found by making a law real and running it, not by
reading code; the seventh was found by reading the corpus as a whole. Both
are arguments for continuing the discipline into Stages 1–4 — and the seventh
is the argument for adding the whole-branch review to it.

---

## 8. Recommendation: what Stage 1 should absorb first

Argued from the evidence above, not from the plan's original guesses — and
**re-derived from scratch after the correction.** The first version of this
section rested in part on composition's green: it argued that piece
attribution was cheap *because* the server already rendered pieces separably
and merely failed to say so. That premise is gone. The recommendation below
survives, but for a different and stronger reason, and the estimate attached
to it is larger.

### 8.1 Keep the spec's ordering. The evidence supports it.

Spec §5 orders the stages registry → ledger → arrangement → route retirement,
on the reasoning that rows need identities and claim resolution needs
dispositions. **Nothing in this diagnosis disturbs that.** The census route
built in this stage is green and total at every sampled instant, which is
direct evidence that the disposition table is a coherent thing to build a
ledger around. Start Stage 1 where the spec says: the entity registry.

The correction does not disturb it either. §8.2 pulls scene work forward, but
that work is orthogonal to registry → ledger → arrangement: it is about how
geometry is packaged on the wire, not about who stands where. It can run
beside Stage 1 rather than displacing it.

### 8.2 The first thing Stage 1 should absorb: **make pieces first-class** — pulled forward out of Stage 4

This is the recommendation that departs from the plan, and the correction
strengthens it rather than weakening it.

Both scene `@target`s are red, and §3.3 argues they are one gap: the server
does not model a piece as a thing that owns its own geometry. Piece
attribution and composition are scheduled for Stage 4. The gap underneath
them should move to Stage 1, or be done alongside it. The evidence:

- **It is the whole of what is red.** Two failing laws out of twenty
  scenarios, and both are this. Nothing else in the corpus is red.
- **It is not a labelling job — it is real work, and knowing that now is
  worth more than the earlier, wrong, cheaper estimate.** The first version
  of this section called attribution "a field the server already knows how to
  compute, because it already renders the pieces separately." §3.2 shows it
  does not: at least one resource buffer is jointly produced by more than one
  piece, so there is no single correct `piece` value to stamp on it. The
  buffers have to be split before the field can be honest. **That is exactly
  why it should come early rather than late** — it is the one piece of scene
  work whose size the plan had wrong, and Stage 4 is the worst place to
  discover that.
- **It is the blocker on three other laws' honesty**, not just its own.
  §6.2's subset assertion, §6.3's presence-only assertion, and §6.1's
  weakened composition all stay weak until pieces own their geometry. And
  §6.2 is not merely weak — it is *hiding a live bug*, one word away from
  going red. Landing this unlocks four strengthenings for one piece of work.
- **It de-risks Stage 4** by moving the one genuinely unknown piece of scene
  work earlier, where the golden gate can judge it in isolation rather than
  alongside a 13-route deletion and a viewer migration.
- **The golden gate makes it safe to attempt early.** Splitting shared
  buffers is exactly the kind of change that could move pixels. 89 blessed
  stops will say so immediately if it does.

Concretely, in order:

1. **Confirm or refute §3.3 first.** Read the renderer and establish whether
   the shared points buffer is a general pattern or a single case. This is a
   half-day question and the size of everything below depends on its answer.
   Nothing in this document establishes it; §3.2 only establishes the symptom.
2. Give each piece its own resources, so that omitting a piece removes that
   piece's buffers and changes no others.
3. Add `piece` to every manifest entry — now well-defined, because every
   entry has exactly one owner.
4. Strengthen the three weak laws in the same pass: "omission is subtractive"
   from `noWater ⊆ full` to *`full − noPiece` is exactly that piece's
   entries*, and as a `@property` over generated piece sets rather than one
   hardcoded omission; the attribution `@target` from presence to correct
   values; composition from resource-set union toward the spec's `⊕` as far
   as v0.1 allows.
5. Drop the `@target` tags.

If step 1 comes back saying the interference is one isolated buffer, this is
a small job and the first version's optimism was merely mis-argued rather than
wrong. If it comes back saying resources are aggregated across pieces
throughout, this is the largest single item in Stages 1–4 and the plan needs
to know that before Stage 1 is scoped, not during Stage 4.

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

§6.4. This one gets more urgent after the correction, not less. There are
four `@property` scenarios and **one of them is now red** — so the absence of
shrinking has already cost something concrete: the composition report names
fifteen pieces where the real answer is one word (`journeys`), and closing
that gap took a manual narrowing pass. Stage 1 adds `entities`, `node`,
`edge_summary`, and `edges` — paging laws and identity laws are exactly the
kind that get fuzzed hard, and exactly the kind whose counterexamples are
large and structured. **Do the `forAllShrink` restructuring while the
property corpus is still four scenarios**, not after it is twenty. It is
listed in spec §4 as a requirement and it is currently unmet; it should not
enter Stage 1's planning as a discovery.

Pair it with the cheaper fix §7.0 argues for: **a check that every
`@property` scenario's holes actually take distinct values across a run.**
That is a few lines, it is what would have caught the composition defect on
day one, and it protects every property scenario Stage 1 adds.

### 8.5 What Stage 1 does *not* need to build

Worth saying explicitly so the Stage 1 plan does not budget for it:

- **A combine endpoint.** Not needed for anything Stage 1 does, and the
  weakened composition law (§6.1) does not depend on one. The composition red
  is *not* an argument for building `⊕` — the failure is upstream of any
  combine operator, in how resources are packaged (§3.2). Building a combine
  now would sit on top of the bug rather than fix it. Defer it to whichever
  stage genuinely needs `⊕` on the wire.
- **Content-addressing work.** Finished, on the axis it covers. Ids are their
  bytes, batches are their singles, restyles change no ids. Note the exact
  boundary: content-addressing is correct, and §3.2 is not a
  content-addressing bug — the hash faithfully reflects a buffer whose
  *contents* wrongly depend on which pieces are on. Stage 1's job is to leave
  the addressing alone while fixing what gets addressed.
- **Dress work.** The dress axis is done and now proved in both directions
  (§4). `TopographyDress` lands on ground that already obeys its law.

**And one thing the first version wrongly put on this list.** It said scene
algebra rework was *not* needed, on the grounds that "the pieces already do
not interfere." They do interfere (§3.2). That line is retracted; the work is
§8.2.

### 8.6 The gates Stage 1 inherits

All four verification strata from spec §6 are live and green as of `7cd31bd`,
and every one of them must stay green through Stage 1:

- Rust unit and law tests: `cargo test --workspace` — 19 suites ok, 0 failed
  (carried forward from `a382c68`; the correction touched no Rust).
- The contract suite: `contract-runner run` — map-api 18/20 with two declared
  targets, atlas-edge 6/6.
- Census diffs: the census route exists and is stable; spec §5 requires
  Stage 1's diff to *be* the identity unification (merged rows, reviewed name
  by name) — the first non-empty diff the owner reviews.
- The golden gate: `ALL GOLDEN VIEWS HOLD`, 89/89.

---

## 9. Spec obligations with NO coverage — recorded here because they are recorded nowhere else

**This section is new in the correction, and it is the one the next planning
pass must not skip.** Everything above reports on laws the corpus *contains*.
This reports on laws the spec requires and the corpus **does not contain at
all** — so they appear in no table, go neither green nor red, and were
invisible until the final whole-branch review went looking for them. Nothing
here is a server defect. Every item is a **missing check**, which §7.0 should
have taught us is the more dangerous kind.

They are listed so that Stages 1–4 inherit them as known work rather than
rediscovering them.

### 9.1 Four spec obligations with no scenario

| # | spec clause | what it requires | status |
|---|---|---|---|
| 1 | **§3 law 4 — default-totality** | "an omitted dress is the declared classical default; a style may dress any subset of pieces" | **No scenario.** |
| 2 | **§4 — derivability** | "every manifest entry must be traceable to `disposition` + `borders` answers — contract-tested by sampling" | **No scenario.** |
| 3 | **§4 — census diff law** | "`census.feature`'s scenarios are the disposition-totality **and diff** laws" | **No diff scenario.** |
| 4 | **§5 Stage 0 — "Runner crate + CI"** | CI wiring, so the semver rule is enforced | **Does not exist.** |

**1. Default-totality — and this one is testable today.** Render with no
`style` parameter at all; the response must be the declared classical default
dress. Nothing in the corpus does this. It matters because it is the law that
makes *every other* dress claim meaningful: if there is no defined behaviour
for "no style given", then §4's dress-locality green is a statement about two
named styles rather than about dress as such. It is also cheap — one scenario,
one render, against the server as it stands. Note that the runner's own
`sceneUrl` helper always emits `&style=`, so the step vocabulary cannot
currently *express* a styleless render; that is the small piece of work
attached.

**2. Derivability — the clause that binds the two tiers.** Spec §4 makes
derivability the join between the scene tier and the fact tier: a manifest
entry is not allowed to be free-floating geometry, it must be traceable back
to a `disposition` answer and a `borders` answer. There is no scenario, so
**the two tiers are currently contract-tested in isolation and never against
each other.** This is arguably the most important omission on the list: the
whole architecture rests on the two tiers agreeing, and nothing checks that
they do. It is not testable in full until Stage 2–3 provide `disposition` and
`borders`, but the sampling scenario should be *written* (as `@target`) now,
so the obligation is visible in the corpus rather than in this document alone.

**3. The census diff law — and this one has teeth.** `census.feature` pins
three whole-body censuses (1050 BC, AD 59, the conquest) plus determinism.
Those are the *totality* law. The **diff** law — census at t₁ compared against
census at t₂ — is absent. That matters more than a missing scenario usually
would, because **the census diff is the exact instrument spec §6 stratum 3
uses to judge Stages 1–3**: Stage 1's diff *is* the identity unification
reviewed name by name, and Stages 2 and 3 must produce an *empty* diff. So a
law the spec names as a contract obligation is also the measuring device for
three stages, and it does not exist. It should be built before Stage 1, not
during it — you cannot review a diff with an instrument you are building at
the same time.

**4. CI, and everything that hangs off it.** Spec §5 lists "Runner crate +
CI" as Stage 0 scope. The runner crate exists; **there is no CI wiring
anywhere in the repository** (no `.github/workflows`, nothing equivalent), and
there is no changelog under `contracts/`. The plan dropped this silently and
the spec is the authority, so it is recorded here as unmet Stage 0 scope
rather than as a Stage 1 idea. What is lost with it is specific: spec §4
requires the semver rule to be **CI-enforced** — *"a diff editing an existing
feature or blessed fixture without a bump fails"* — and with no CI, that rule
is currently a convention that nothing checks. Given that this stage produced
~2.0–2.4 MB blessed fixtures whose whole point is to be compared byte for
byte, an unenforced blessing rule is a real exposure. Also worth noting: the
static gates (`check`, `vocab`) and the totality law are described in the spec
as CI modes, and today they only run when somebody remembers to run them.

### 9.2 Three smaller gaps, one line each

- **A quarter of the corpus is exempt from the drift law without saying so.**
  Three of twelve features — `census`, `changes`, `subjects` — carry **no
  `Vocabulary:` block at all**, because every capture in them is free prose
  with a Described universe rather than an enumerated or ranged one. `vocab`
  therefore passes *vacuously* on them, against spec §4's "each feature
  carries a `Vocabulary:` table". This is §7.0's shape again in miniature: a
  gate reporting green over ground it never examined. Either those captures
  should get real types with real universes, or the exemption should be
  explicit and counted, so "vocabulary: every table matches its types" stops
  sounding like a statement about the whole corpus.
- **`@property` scenarios still have no shrinking**, though spec §4 requires
  `forAllShrink`. §6.4 has the detail; §3.2's fifteen-piece counterexample
  against a one-word real answer is the worked cost.
- **Feature preambles are unverified prose.** The drift law protects the
  Vocabulary table; nothing protects the English above it, and three atlas-edge
  preambles had drifted from their own projections within a single stage
  (§5). Since the atlas-edge suite is an artifact handed to another team, its
  prose is part of the interface.

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

To reproduce §3.2's composition failure by hand, without the runner — this is
what turned the runner's fifteen-piece counterexample into the one-word
diagnosis:

```bash
S='http://127.0.0.1:8090/api/scene?year=-1405&zoom=90.0000&style=canaan&relief=1'
curl -s "$S"             -o full.json
curl -s "$S&journeys=0"  -o nojourneys.json
curl -s "$S&labels=0"    -o nolabels.json

# omitting labels is subtractive; omitting journeys is not
python -c "
import json
ids = lambda p: {r['id'] for r in json.load(open(p))['resources']}
full = ids('full.json')
for p in ('nolabels.json','nojourneys.json'):
    print(p, 'subset of full?', ids(p) <= full)
"
```

Expected output today: `nolabels.json` is a subset, `nojourneys.json` is not.
The one diverging resource is a `points` buffer — 26 vertices with journeys
off, 33 with journeys on.

Toolchain: GHC 9.12.1, cabal 3.18.1.0. The runner forces UTF-8 on stdout, so
the em dashes above survive any console code page. Requires both servers
live: ours on `:8090`, the atlas on `:8080`.
