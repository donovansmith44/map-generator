# The Map API Contract: typed functions over one graph

**Date:** 2026-09-06
**Status:** Approved design, pre-implementation
**Owner decisions logged:** convergent graph (c) · read-only now, write-ready types (c) · two tiers with derivability (b) · dogfood acceptance (a) · language-agnostic semver'd contract tests · TDD mandatory · Haskell runner with hand-rolled interpreter · scenes readable for dummies · scene-as-monoid-over-pieces · pre-release until the owner declares v1.0

## 1. Purpose

Create an API that serves everything necessary to drive a thin front end, comprehensively contract-tested, so the Bible atlas can migrate onto this work with ease. The API is a set of typed functions over a graph with a single source of truth for all that generates the map.

The refactor cures the diagnosed disease: four parallel ingestion pipelines answering the same five questions (when does it stand, does it fill, how does its edge draw, who wins overlap, what is it) with five private mechanisms — Rust literals, geojson property strings, vendor JSON, slug matching — none of them queryable, none of them composing, every knob-turn a surgery. Policy changes had no blast radius (the Judea incident); only pixels could reveal what a rule change did.

## 2. The fact graph (single source of truth)

Six node types. Everything else becomes data feeding them or pure functions over them.

- **Entity** — one identity per real thing, minted once in a registry. `{ id, names[], kind: Polity | District | People | Allotment | Promise | Vision | Waterbody | Terrain | Place | Route }`. Phoenicia is ONE node; Judea is ONE node; the promise is ONE node.
- **Witness** — one source's attestation of an entity: `{ entity, source (Scripture verses | Atlas | NaturalEarth | Basemap | PlateTrace | OpenBible), evidence (circuit | rings | point | polyline), provenance (BorderText | CityDerived | Traced | Vendored), interval }`. The node type is TARGET state; its vocabularies are promotions of current scattered enums (`map_canon::Witness`, `surveys::Grade`) into one queryable place — nothing invented. The atlas's whole graph enters as Witness rows (convergence).
- **Standing** — entity × right-open interval: when it exists. PresenceBook promoted to universal. Endurance is a typed field with a written reason; the frame-edge law stops grepping prose.
- **Border** — one canonical edge in THE one arrangement, content-addressed, carrying the witness set whose geometry it realizes. All pipelines' geometry funnels here.
- **Extent** — derived: entity × era → boundary cycles / face set, resolved in the arrangement by the precedence law. Never authored directly.
- **Law** — data, versioned: tenure rules, precedence rules, supersession rules, dress rules. The knobs live here: one row, global reach.

### The five equations

1. **Identity** — no witness mints an entity; every witness references the registry.
2. **Geometry** — every border exists once, in the one arrangement; extents are references; Σ faces = 4π.
3. **Time** — standings right-open; one world-state per instant; nothing endures the frame's edge without a typed written reason.
4. **Disposition** — `dispose : (Entity, Witness*, Standing, Laws) → {tenure, dress-class, rank}` is a total pure function; the census is its image, emitted every build; a law change is reviewed as a census diff before any pixel renders.
5. **Derivability** — every scene-tier answer is a pure composition of fact-tier answers with style and camera.

### Explorable and upstreamability

All node types are graph positions with typed relations (`witnessed-by`, `stands-during`, `bounded-by`, `extends-over`, `disposed-under`, `supersedes`) built on the atlas's `Graph`/`EdgeKind`/`EdgeMeta` machinery, each implementing the atlas `Explorable` trait (`edge_summary` + paged `edges`) exactly as their `PositionRef` does — the atlas exploration UI walks our nodes with zero new client machinery.

**The upstreamability law** (contract-tested design constraint): every map node and relation type must be expressible as an ADDITIVE extension of atlas graph-types — new variants, never reshaped existing ones — so a PR contributing our nodes into the Bible atlas is always a mechanical option.

### What dies

`Stands`/`Holds`/`Grade` as Rust literals; `stands_until` strings; `bg_shadows` slug matching; per-pipeline precedence; prose-marker law enforcement; surveys-as-geometry-pipeline (circuits become Witness evidence).

## 3. The scene algebra and the two-tier function set

### The scene is a monoid over pieces

```
Piece = Ground(topography) | Water | Fills | Borders | Claims
      | Labels | Markers | Journeys | Chrome | Veil        (extensible)
scene : Query → Scene    where Query selects any SUBSET of pieces
```

1. **Omission-totality** — `scene(q \ P)` is valid and byte-deterministic; equals `scene(q)` minus exactly P's contribution. Absence is the identity element, never an error.
2. **Composition** — `scene(a ⊕ b) = scene(a) ⊕ scene(b)`; ⊕ associative, identity = empty scene; paint order comes from the precedence LAW, never composition order.
3. **Dress-locality** — restyling piece P changes only P's contribution.
4. **Default-totality** — an omitted dress is the declared classical default; a style may dress any subset of pieces.

Topography injection falls out as a corollary: `TopographyDress` (multi-stop land/bathymetry ramps, band structure, intensity) is Ground's dress under laws 3/4 — injectable, omittable, swappable with impunity. Ground takes a data-source parameter later without touching the algebra (out of scope now; signature leaves room).

### Function set (wire-level; every function = Gherkin feature + fixtures, semver'd)

```
FACT TIER
  entities(filter)            → [Entity]
  node(id)                    → NodeCard
  edge_summary(id)            → {relation → count}          (atlas Explorable shape)
  edges(id, relation, cursor) → EdgePage                    (atlas Explorable shape)
  standings(at | over)        → [(entity, interval)]
  disposition(entity, at)     → {tenure, dress-class, rank, law-citations}
  census(at)                  → the full disposition table
  borders(entity, at)         → boundary cycles + witness provenance
  changes(from, to)           → [ChangeEvent]
  laws()                      → active law set, versioned

SCENE TIER (pure composition of fact answers × style × camera)
  scene(pieces, at|over, style, camera?) → manifest (content-addressed refs)
  resources(ids)              → geometry payloads
  transition(from, to, pieces)→ morph/fade plan
  styles()                    → [StyleId + which pieces each dresses]

META
  contract()                  → {version, laws-version, graph-pin}
```

Derivability binds the tiers concretely: every manifest entry must be traceable to `disposition` + `borders` answers — contract-tested by sampling.

## 4. Contract machinery

### Tree

```
contracts/
  VERSION                    # suite semver (pre-release 0.x until the owner declares 1.0)
  map-api/
    fact/  *.feature         # one file per function; scenarios ARE its laws
    scene/ scene.feature     # the four algebra laws + derivability AS the scenarios
           resources.feature # content-addressing laws
           transition.feature
    meta/  contract.feature
    fixtures/                # canonical JSON, content-addressed, blessed
  atlas-edge/                # consumer-driven contract for what WE consume
    polities.feature, narratives.feature, events.feature,
    eras.feature, landmarks.feature, land-mask.feature
    fixtures/                # the minimal shapes our parsers require
  runner/                    # Haskell cabal project → one binary
```

**Law-as-scenario principle:** there is no shapes-only feature anywhere — a function whose contract is only its shape has no contract. `census.feature`'s scenarios are the disposition-totality and diff laws; `edges.feature`'s are the paging laws; `scene.feature`'s are the algebra.

### The runner (Haskell, hand-rolled interpreter — verified: no maintained alternative exists)

The Haskell Gherkin shelf is abandoned (abacate 2012, chuchu 2014, cucumber-haskell unfinished; the official cucumber/gherkin project has no Haskell port), and our two load-bearing extensions (Vocabulary drift-law, @property holes) exist in no stock parser in any language. ~550–650 lines, owned:

- **AST + parser** (~160 lines, megaparsec, line-oriented; law: `parse . render ≡ id`; And/But resolve to prior keyword at parse).
- **Typed capture patterns** (no regexes): step definitions are applicative patterns whose captures are typed (`FromCapture`); a capture that doesn't parse fails with the type name. Stringly-typed step glue is unrepresentable.
- **World + runner:** `World { baseUrl, lastResponse, fixtures, bound }`; Then-steps assert typed diffs against fixtures. **Totality law** (CI `--check` mode): every step in every feature matches exactly one definition, or CI fails naming the orphans.
- **@property scenarios:** `<angle-bracket>` holes whose `FromCapture` types carry QuickCheck `Gen` + shrink; the scenario becomes `forAllShrink` over generated bindings. One text file = human-readable contract + example test + fuzzed law with shrinking.
- **Vocabulary blocks (readable for dummies):** `FromCapture` gains `universe :: Universe (Enumerated | Ranged | Described)`. Each feature carries a `Vocabulary:` table listing legal values; `--vocab` mode regenerates/verifies it against the types actually used — documentation that cannot drift. Enumerated values round-trip through the parser (law). Wrong guesses get did-you-mean errors listing the full universe.

The runner speaks only HTTP+JSON against a `BASE_URL`; it compiles to a standalone binary the atlas session can run against `:8080` without our workspace. Language-agnosticism lives in the `.feature` files and fixtures.

### Semver (pre-release regime)

The suite is **pre-release (0.x) until the owner declares otherwise**. Discipline holds throughout: breaking change to any existing feature/fixture = MINOR bump + changelog entry (CI-enforced: a diff editing an existing feature or blessed fixture without a bump fails); additive = PATCH. `contract()` advertises the version; the runner refuses skew. Deprecations are additive `@deprecated(since, replacement)` tags. **v1.0.0 is a human act by the owner**, tagged when the contract is ready for the atlas to lean on — never automatic.

### CDC on the atlas edge

The six consumed endpoints (`/api/polities`, `/api/narratives`, `/api/event/:id`, `/api/eras`, `/api/landmarks`, `/api/land-mask`) get a consumer-driven suite stating exactly what our `parse_*` functions need. Runs against our fixtures (consumer side, our CI) and against the live atlas `:8080` (provider verification — handed to the atlas session to adopt). Verified 2026-09-06: atlas worktree is 4 ahead of origin with graph-types identical; our 19 suites pass against it; `:8080` is live.

## 5. Staged migration (contract-first strangler, canon-evolution interior)

Every stage: contract additions first (runner red) → Rust tests (cargo red) → implement (green) → golden gate holds → census diff reviewed → commit.

- **Stage 0 — freeze the surface (v0.1).** Features + fixtures for what today's seam honestly serves: `scene` (with the four algebra laws — where today's implementation fails one, that is a real bug found on day one), `resources`, `subjects`, `changes`, `contract`. Runner crate + CI. Production changes limited to `/api/contract` and `/api/census`. The atlas-edge CDC suite lands here, verified against `:8080`.
- **Stage 1 — entity registry (v0.2).** One registry; all minted identities resolve through it; the two Phoenicias and three Canaans become one node each with multiple witnesses. Adds `entities`, `node`, `edge_summary`, `edges` (Explorable on the wire). Slug-matching supersession dies.
- **Stage 2 — the ledger (v0.3).** `Stands`/`Holds`/`Grade` literals, `stands_until` strings, shadow spans become ledger rows; laws become the law set. Adds `standings`, `disposition`, `laws`. `surveys.rs` shrinks to circuit evidence. **Census diff must be empty.**
- **Stage 3 — one arrangement (v0.4).** Survey circuits enter the partition as witnesses; every border canonical; claims reference edges (the promise's west border IS the coastline). Adds `borders` with provenance. Census empty-diff; the golden gate is the hard judge (snapping tolerances declared, not tuned).
- **Stage 4 — retire the 13 routes (v0.5, the dogfood bar).** The viewer consumes only contract functions; legacy routes deleted; `TopographyDress` lands as dress-locality's show-piece. Pre-release continues until the owner's v1.0 declaration.

Ordering: registry before ledger (rows need identities), ledger before arrangement (claim resolution needs dispositions), routes last. Each stage independently shippable and revertible.

## 6. Verification strata and acceptance

Four strata, each with its own jurisdiction, all required per stage:

1. **Rust unit & law tests** (inner) — TDD-first.
2. **The contract suite** (wire) — features-as-laws, vocabularies drift-checked, properties fuzzing the algebra; against `BASE_URL` for map-api and `:8080` for the atlas edge.
3. **Census diffs** (model) — stage 1's diff IS the identity unification (merged rows, reviewed name by name); stages 2–3 must be empty; any other non-empty diff is reviewed by the owner before merge.
4. **The golden gate** (pixels) — 89 blessed stops hold at every stage; re-blessing only on owner-approved visual change.

**Dogfood-bar checkables (Stage 4 exit):** viewer entirely on contract functions; 13 legacy routes deleted; contract suite green against the server; atlas-edge CDC green against `:8080` and handed to the atlas session; census stable; golden gate green; `contract()` reports the current 0.x.

## 7. Out of scope (types leave room; no code)

The write API (mutation-ready types only) · topography data-source selection (Ground's parameter later) · atlas-side implementation work (their session's) · the upstream PRs themselves (the law is in; exercising it is a future act) · v1.0 (the owner's declaration).

## 8. Risks

Stage 3 geometry snapping (golden gate judges; tolerances declared) · Haskell toolchain new to CI (standalone cabal project, GHC pinned via ghcup) · v0 freezes some warts (`@deprecated` tags) · atlas dep pins a live worktree (CDC suite + graph-pin in `contract()` make drift loud).
