# Stage 2: The Ledger — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land spec §5's Stage 2 — the `Stands`/`Holds`/`Grade` Rust literals, the `stands_until` property strings, and the background shadow spans stop being scattered private mechanisms and become rows in ONE queryable ledger; the knobs they encoded become ONE versioned law set; `dispose` becomes a total pure function whose image is the census; `standings`, `disposition` and `laws` land on the wire; `surveys.rs` shrinks to circuit evidence — **and the census does not move by a single byte.**

**Architecture:** Three parts, in order. **Part A** builds the new types and proves they say what the old literals said, while the old literals are still in place: a disposition fingerprint frozen first (you cannot prove you changed nothing without a record of what you had), then `map_canon::ledger` (Standing, Endurance, Ledger — PresenceBook promoted to universal), then `map_canon::law` (Laws as versioned owner-reviewable data with a declared default, so "no rule matched" is unrepresentable), then `dispose` and the law that the census is its image. **Part B** deletes the literals one family at a time, each behind a red-then-green equality test against Part A's fingerprint: `Stands` → ledger rows, `Holds`/`Grade` → tenure declarations and a `Derivation` on the witness, `stands_until` → ledger rows keyed on Stage 1's canonical entity ids, shadow spans → a supersession law, `paint_rank` → the precedence law. **Part C** serves `standings`, `disposition` and `laws`, moves `contract()`'s `lawsVersion` off `"0.0-preledger"`, satisfies the disposition half of derivability, and closes the stage at `0.3.0` with the empty census diff as its judge.

**Tech Stack:** Rust (map-canon, map-adapters, map-compile, map-provider, map-viewer); Haskell contract runner (GHC 9.12.1, cabal 3.18.1.0, megaparsec, aeson, QuickCheck, hspec); Gherkin `.feature` corpus + blessed JSON fixtures; Node + playwright-core for the golden gate; `serde_json` for the owner-reviewable law and standing data files.

**Spec:** `docs/superpowers/specs/2026-09-06-map-api-contract-design.md`
**Diagnosis (Stage 0's evidence):** `docs/notes/2026-09-06-contract-diagnosis.md` — read the correction notice and §7.0 first.
**Stage 1 plan (what this stage consumes):** `docs/superpowers/plans/2026-09-07-contract-stage1.md` — its closing "What Stage 1 produces that Stage 2 consumes" section is binding on this plan.
**Stage 0 plan (format exemplar only):** `docs/superpowers/plans/2026-09-06-contract-stage0.md`

---

## Global Constraints

- **TDD is mandatory.** Every task writes its failing test first, runs it, sees it fail *for the stated reason*, then implements. No exceptions.
- **Owner's standing law:** first-class composable types with laws — never styling tricks, never special cases, never tuned constants. **This stage is that law's own subject:** it turns scattered literals into one queryable law set. If a fix in this stage needs a magic number, the number belongs in `data/authored/laws.json` with an id and a written reason, or it is wrong.
- **Whole-body assertions.** A scenario pins the ENTIRE answer, with don't-cares masked explicitly in the scenario text. Existential poke-assertions ("some row has…", "is an array", "has at least N") are forbidden.
- **A check satisfiable by the failure mode is not a check** — and this applies to the *inputs* as well as the assertion (diagnosis §7.0). Every new `@property` scenario is automatically checked by Stage 1 Task 5's hole-distinctness law in `contract-runner check`; every new Rust equality test in this plan carries a negative case that proves it discriminates.
- **Order within every task:** contract additions first (runner red) → Rust tests (cargo red) → implement (green) → golden gate holds → census diff reviewed → commit.
- **THE STAGE'S JUDGE — the census diff must be EMPTY.** Concretely and mechanically: the three blessed whole-body fixtures `contracts/map-api/fixtures/census-1405.json`, `census-1050.json`, `census-59.json` **must not be re-blessed by this stage.** If `contract-runner run` reports a census scenario red, the ledger is wrong or the literals were, and the difference is reviewed **row by row with the owner** before anything is re-blessed. Re-blessing a census fixture to make a red go away is a stage failure, not a fix.
- **`/api/census`'s row schema is FROZEN at five keys** (`entity`, `name`, `layer`, `kind`, `tenure`) for the whole of this stage. `dispose` produces richer answers — dress-class, rank, law-citations — and those are served by `/api/disposition`, which is additive. Widening the census is a schema change that would make every row read as `Changed` through Stage 1's `census_diff` instrument, which keys on `(layer, entity)` and compares whole rows; the instrument cannot tell a schema change from a policy change, so this stage does not ask it to. See "Interfaces" for the note handed to Stage 3.
- **The suite stays pre-release.** `contracts/VERSION` moves `0.2.0` → `0.3.0` in Task 13, with a `contracts/CHANGELOG.md` entry. v1.0 is a human act by the owner, never automatic.
- **The golden gate is the hard judge.** `node crates/map-viewer/tests/golden.js --check` must print `ALL GOLDEN VIEWS HOLD` (89/89 stops). Run it from `C:\Users\donov\.claude\jobs\c6946bce\tmp` so `playwright-core` resolves. Any visual change requires the owner's explicit approval and a re-blessing; a drifted probe STOPS the task.
- **Ports.** The workbench viewer is **8090**. `8080` (atlas API), `8081`, `8000`, `5000` belong to the atlas pipeline and must never be bound.
- **The atlas repo is a READ-ONLY path dependency.** Never edit it. Never edit `contracts/atlas-edge/` either — Stage 1 handed that suite to the atlas session.
- **The render pipeline is three steps** (MEMORY: render-pipeline-three-steps). After a Rust change: `cargo build --release`, stop the old viewer, relaunch it **detached**. Session background tasks get killed, so use `Start-Process`:
  ```powershell
  Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
  cargo build --release
  Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
  ```
  **Almost every task in this plan touches `crates/map-compile` or `crates/map-adapters`, so the map-compile build step is REQUIRED before the viewer restart** (`cargo run --release -p map-compile build`), or the canon is stale and every census comparison is a lie about the previous build.
- **Pieces are Stage 1's and are not touched here.** `map_types::{Piece, PieceSet}`, `Snapshot::restrict`, `piece` on manifest entries and `?pieces=` on `/api/scene` are inherited intact. If a scene law goes red in this stage, that is a regression this stage caused, not licence to change the scene contract.
- **Haskell commands run from `contracts/runner/`** unless stated otherwise.
- **Feature preambles are unverified prose** (diagnosis §9.2). Any task that changes what a feature *means* updates its preamble in the same commit; the reviewer checks the prose against the scenarios.
- **`make contract-gates` and `make ci`** (Stage 1 Task 6) are the gate runners. Every task's final step runs `make contract-gates`; every task that touches production code also runs the golden gate.

---

## What this stage retires — the exact sites, read from the code on 2026-09-07

Recorded here so no task has to rediscover them, and so a reviewer can check the deletions are complete. Every line number was read at commit `e05ea3d`.

| spec §2 "what dies" | where it lives today | count |
|---|---|---|
| `Stands` as a Rust literal | `crates/map-adapters/src/surveys.rs:778-784` (enum), `:812` (field), 25 `SurveySpec` rows in `SURVEYS` (`:837-866`) and `SURVEYS_MORE` (`:878-939`); consumed at `:1321-1329` (endurance prose) and `:1354-1357` (Interval) | 25 values, 2 consumers |
| `Holds` as a Rust literal | `surveys.rs:766-773` (enum), `:814` (field), the same 25 rows; consumed at `:1371-1374` (`RegionClass`) | 25 values, 1 consumer |
| `Grade` as a Rust literal | `surveys.rs:786-794` (enum), `:815` (field), the same 25 rows; consumed at `:1343-1346` (`EdgeCharacter`) | 25 values, 1 consumer |
| `stands_until` strings | `crates/map-compile/src/partition_bridge.rs:386` + `:544-550` (openbible regions), `:443-445` + `:524-535` (tribal cohorts); data: `data/openbible/regions.geojson` (6), `data/wikimedia/tribes12.geojson` (13 `stands_from` + 13 `stands_until`) | 4 code sites, 32 data rows |
| `bg_shadows` shadow spans | `crates/map-compile/src/main.rs:394-428` (built, reported, threaded), `crates/map-compile/src/timeline_bridge.rs:40`, `:45-57`, `:143-152`, `:180-186`; four call sites pass `&BTreeMap::new()` | 1 producer, 1 consumer, 5 call sites |
| per-pipeline precedence | `crates/map-provider/src/canon_provider.rs:559-572` (`paint_rank`, a `match layer` of six tuned integers) and `:678` (`.unwrap_or(2)`, an undeclared default) | 7 constants |
| prose-marker law enforcement | `crates/map-canon/src/lib.rs:176` (`ENDURES_MARK`), `:489-504` (`validate_frame_edge` greps `p.note.contains(...)`); marks written at `surveys.rs:1325-1328`, `partition_bridge.rs:682`, `:700` | 1 law, 3 writers |
| surveys-as-geometry-pipeline | `surveys.rs` is 1742 lines. `SurveySpec` keeps `tag/label/note/book/chapter/verse_from/verse_to/year/circuit` — the **circuit evidence**. It loses `stands`, `holds`, `grade`. | 3 fields |

**The one finding that will tempt a wrong fix.** The live census at −1405 holds **20 `scripture-claims` areas with tenure `held`** (`partition:ammon`, `partition:asher`, … `partition:zebulun`, `partition:canaan`, `partition:phoenicia`) and exactly **one** `scripture-claims` area with tenure `claimed`. Those twenty are `crates/map-compile/src/partition_bridge.rs:605`, which hardcodes `tenure: map_canon::Tenure::Held` on every partition face regardless of layer. Writing the tenure law as "a ScriptureClaims area is Claimed" is the natural, tidy, **wrong** move: it flips twenty rows and the census diff is no longer empty. The law set must reproduce today's answer exactly; whether the twenty rows are *right* is a policy question with its own non-empty diff, and it is **out of scope for this stage** — Task 13 Step 4 hands it to the owner as a written finding.

---

## Part A — the types, proved equal to the literals before anything moves

### Task 1: Freeze the evidence — a disposition fingerprint that fails the day an answer changes

You cannot prove you changed nothing without a record of what you had. Stage 1 Task 14 Step 1 captured `census-before-*.json` as *working evidence* that got deleted; this stage needs something durable and machine-checked, because five later tasks each need to prove they moved no answer. This task builds it and only it.

Two instruments, both TDD'd, both committed:

1. **A timeline fingerprint** — a content hash of everything `scripture_timeline()` emits, so a change to any of the 75 `Stands`/`Holds`/`Grade` values shows up as one failing assertion with a name attached.
2. **A whole-census fixture harness in Rust** — `cargo test -p map-canon census_fixtures_hold` reads the three blessed contract fixtures and asserts the compiled canon reproduces them, so the empty-diff law is checkable **without** a running server. Today it can only be checked by the contract runner against `:8090`, which means a task cannot know it broke the census until the very end.

**Files:**
- Create: `crates/map-adapters/src/fingerprint.rs`
- Modify: `crates/map-adapters/src/lib.rs` (add `pub mod fingerprint;`)
- Modify: `crates/map-adapters/src/tests.rs`
- Modify: `crates/map-canon/src/tests.rs`
- Read only: `contracts/map-api/fixtures/census-1405.json`, `census-1050.json`, `census-59.json`

**Interfaces:**
- Consumes: `map_adapters::scripture_timeline`, `map_types::WorldTimeline`, `map_canon::census`.
- Produces:
```rust
// map-adapters
pub fn timeline_fingerprint(tl: &map_types::WorldTimeline) -> u64;
```
Tasks 5, 6, 7, 8 and 9 each assert this value is unchanged. Nothing else imports from it.

- [ ] **Step 1: Write the failing fingerprint test**

Append to `crates/map-adapters/src/tests.rs`:

```rust
/// STAGE 2 TASK 1 — the disposition fingerprint.
///
/// Everything the survey table decides — WHEN a survey's world stands
/// (`Stands`), whether it holds ground (`Holds`), and how its edge
/// draws (`Grade`) — reaches the rest of the world through exactly
/// three emitted things: a boundary's `character`, a region's `class`,
/// and an `Interval`. This hashes all three for every region and
/// boundary the scripture timeline emits.
///
/// Tasks 5-9 delete the literals behind those three. Each of them
/// re-runs this assertion. A changed value here is a changed answer
/// somewhere in the world, and it must be explained before it is
/// blessed.
#[test]
fn the_scripture_timeline_fingerprint_is_pinned() {
    let tl = crate::scripture_timeline();
    let fp = crate::fingerprint::timeline_fingerprint(&tl);
    assert_eq!(
        format!("{fp:016x}"),
        "PASTE-THE-OBSERVED-VALUE-IN-STEP-4",
        "the scripture timeline's dispositions moved; name what changed before re-pinning"
    );
}

/// DISCRIMINATION. A fingerprint that ignores the thing it exists to
/// watch is worse than none. Flip one region's class and one
/// boundary's character and prove the hash notices each, separately.
#[test]
fn the_fingerprint_notices_a_changed_class_and_a_changed_character() {
    let base = crate::scripture_timeline();
    let fp0 = crate::fingerprint::timeline_fingerprint(&base);

    let mut flipped_class = base.clone();
    let key = *flipped_class.regions.keys().next().expect("regions exist");
    let hist = flipped_class.regions.get_mut(&key).expect("region");
    hist.class = match hist.class {
        map_types::RegionClass::Claim => map_types::RegionClass::Land,
        _ => map_types::RegionClass::Claim,
    };
    assert_ne!(
        crate::fingerprint::timeline_fingerprint(&flipped_class),
        fp0,
        "a changed tenure class must change the fingerprint"
    );

    let mut flipped_edge = base.clone();
    let bkey = *flipped_edge.boundaries.keys().next().expect("boundaries exist");
    let bh = flipped_edge.boundaries.get_mut(&bkey).expect("boundary");
    bh.versions[0].1.character = match bh.versions[0].1.character {
        map_types::EdgeCharacter::Line => map_types::EdgeCharacter::Unknown,
        _ => map_types::EdgeCharacter::Line,
    };
    assert_ne!(
        crate::fingerprint::timeline_fingerprint(&flipped_edge),
        fp0,
        "a changed edge character must change the fingerprint"
    );

    let mut shifted = base.clone();
    let skey = *shifted.regions.keys().next().expect("regions exist");
    let sh = shifted.regions.get_mut(&skey).expect("region");
    sh.geom_history[0].0.to = Some(atlas_graph_types::covenant::TimePoint::year_only(
        atlas_graph_types::covenant::Year::new(-1).expect("year -1 exists"),
    ));
    assert_ne!(
        crate::fingerprint::timeline_fingerprint(&shifted),
        fp0,
        "a changed standing interval must change the fingerprint"
    );
}
```

Adaptation note (verify, do not guess): `crates/map-adapters/src/tests.rs` already imports from `crate::` — follow its existing import style. Check whether `WorldTimeline`, `RegionHistory` and `BoundaryHistory` derive `Clone`; if `WorldTimeline` does not, add `#[derive(Clone)]` in `crates/map-types/src/` where it is declared (a derive on a plain data type is not a behaviour change, and the fingerprint tests need it). Confirm the field names `regions`, `boundaries`, `class`, `geom_history`, `versions`, `character` against `crates/map-types/` before writing — this plan read them at `surveys.rs:1359-1386` and `timeline_bridge.rs:60-108`, which is the consumer side.

- [ ] **Step 2: Run and verify FAIL**

```bash
cargo test -p map-adapters fingerprint 2>&1 | tail -20
```
Expected: compile error — `module fingerprint not found`. That is the failure this step wants.

- [ ] **Step 3: Write the fingerprint**

`crates/map-adapters/src/fingerprint.rs`:

```rust
//! THE DISPOSITION FINGERPRINT (Stage 2 Task 1).
//!
//! One content hash over every dispositional answer the scripture
//! timeline emits: for each region, its tenure class and every
//! standing interval; for each boundary, its edge character and every
//! version interval. It deliberately does NOT hash coordinates — the
//! golden gate judges pixels, and a fingerprint that moved every time
//! a waypoint moved would be re-pinned so often that nobody would read
//! it. This watches the three axes the ledger is about to take over:
//! WHEN it stands, whether it holds, and how its edge draws.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use map_types::{EdgeCharacter, Interval, RegionClass, WorldTimeline};

fn hash_interval(h: &mut DefaultHasher, iv: &Interval) {
    iv.from.year.get().hash(h);
    iv.from.month.hash(h);
    iv.from.day.hash(h);
    match &iv.to {
        None => 0u8.hash(h),
        Some(t) => {
            1u8.hash(h);
            t.year.get().hash(h);
            t.month.hash(h);
            t.day.hash(h);
        }
    }
}

fn class_tag(c: &RegionClass) -> u8 {
    match c {
        RegionClass::Land => 0,
        RegionClass::Water => 1,
        RegionClass::Claim => 2,
        RegionClass::Terrain(_) => 3,
    }
}

fn character_tag(c: &EdgeCharacter) -> u8 {
    match c {
        EdgeCharacter::Line => 0,
        EdgeCharacter::Unknown => 1,
    }
}

/// The fingerprint. BTreeMap iteration is ordered, so this is stable
/// across runs and machines; it is not stable across a deliberate
/// change to any disposition, which is the entire point.
pub fn timeline_fingerprint(tl: &WorldTimeline) -> u64 {
    let mut h = DefaultHasher::new();
    "map-stage2-disposition-fingerprint-v1".hash(&mut h);

    for (rid, hist) in &tl.regions {
        rid.0 .0.hash(&mut h);
        class_tag(&hist.class).hash(&mut h);
        for (iv, label) in &hist.label_history {
            hash_interval(&mut h, iv);
            label.hash(&mut h);
        }
        for (iv, _geom) in &hist.geom_history {
            hash_interval(&mut h, iv);
        }
    }
    for (bid, hist) in &tl.boundaries {
        bid.0 .0.hash(&mut h);
        for (iv, b) in &hist.versions {
            hash_interval(&mut h, iv);
            character_tag(&b.character).hash(&mut h);
            // The endurance declaration rides the provenance text
            // today (map_canon::ENDURES_MARK). Task 5 replaces the
            // prose with a typed field; until then, hash whether the
            // declaration is PRESENT so a silently-dropped endurance
            // cannot slip past. Not the whole string: the surrounding
            // provenance carries an atlas root hash that moves for
            // reasons that are none of this fingerprint's business.
            b.provenance.contains("ENDURES to the frame's edge").hash(&mut h);
        }
    }
    h.finish()
}
```

Adaptation note: `RegionClass`'s variants and `Interval`'s fields must be confirmed against `crates/map-types/`. If `RegionClass` has variants this plan did not name, add them to `class_tag` with distinct tags — a `_ => 4` catch-all would make two different classes hash the same, which is the failure mode this fingerprint exists to prevent, so the match must be **exhaustive with no wildcard**.

Add `pub mod fingerprint;` to `crates/map-adapters/src/lib.rs` next to the other module declarations.

- [ ] **Step 4: Run, read the observed value, pin it, and prove it discriminates**

```bash
cargo test -p map-adapters the_scripture_timeline_fingerprint_is_pinned -- --nocapture 2>&1 | tail -20
```
Expected: FAIL, with the assertion printing the real 16-hex-digit value against the placeholder. Copy that value into the test, re-run, and see it PASS. Then run the discrimination test:
```bash
cargo test -p map-adapters the_fingerprint_notices -- --nocapture
```
Expected: PASS. If it does not, the fingerprint is blind on that axis and must be fixed before anything downstream leans on it. **Record the pinned value in the commit message** — five later tasks quote it.

- [ ] **Step 5: Write the failing whole-census Rust harness**

Append to `crates/map-canon/src/tests.rs`:

```rust
/// STAGE 2 TASK 1 — the empty-diff law, checkable without a server.
///
/// Spec §5: "Census diff must be empty." The contract suite proves it
/// against :8090 at the end of the stage; this proves it in `cargo
/// test`, at every task, so a task learns it moved a census row in
/// seconds rather than at Task 13. The three fixtures are the SAME
/// blessed bodies the contract suite compares against — read, never
/// written. A test that blesses its own expectation is not a test.
#[test]
fn the_blessed_censuses_still_hold() {
    let store = load_compiled_canon();
    for (year, path) in [
        (-1405, "../../contracts/map-api/fixtures/census-1405.json"),
        (-1050, "../../contracts/map-api/fixtures/census-1050.json"),
        (59, "../../contracts/map-api/fixtures/census-59.json"),
    ] {
        let expected: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(fixture_path(path)).expect("blessed census fixture"),
        )
        .expect("fixture parses");
        let got = census_json(&store, &ts(year));
        assert_eq!(
            got, expected,
            "the census at {year} moved. THE STAGE'S JUDGE HAS SPOKEN: either the \
             ledger is wrong or the literals were. Review the difference row by row \
             with the owner. Do NOT re-bless."
        );
    }
}

/// DISCRIMINATION: the harness must fail when the census really does
/// differ, or it is a check satisfiable by its own failure mode.
#[test]
fn the_census_harness_fails_on_a_real_difference() {
    let store = load_compiled_canon();
    let a = census_json(&store, &ts(-1405));
    let b = census_json(&store, &ts(-1050));
    assert_ne!(a, b, "two different instants must not compare equal");
}
```

Adaptation note: write three small helpers next to these tests.
- `fixture_path(rel: &str) -> std::path::PathBuf` — mirror `crates/map-compile/src/partition_bridge.rs:726-732`'s `data_path` exactly (try the relative path, fall back to `CARGO_MANIFEST_DIR`-joined), so the test runs from both the crate dir and the workspace root.
- `load_compiled_canon() -> CanonStore` — read `data/canon/canon.json` through `crate::persist::from_bytes`. If the file is absent, `panic!("run `cargo run --release -p map-compile build` first")` — an explicit refusal, never a silent skip; a test that quietly passes when its input is missing is exactly diagnosis §7.0.
- `census_json(store, at) -> serde_json::Value` — the same five-key row shape `crates/map-viewer/src/lib.rs:908-918` emits, as a `serde_json::Value::Array`. Factor it so the viewer and this test cannot disagree: put it in `crates/map-canon/src/lib.rs` as `pub fn census_json(store: &CanonStore, at: &Timestamp) -> serde_json::Value` and have the viewer route call it. That is one small production change and it is the right one — two hand-written row writers is how a fixture and a route drift apart.

`serde_json` must be a dependency of `map-canon` (check `crates/map-canon/Cargo.toml`; `persist.rs` already uses it, so it is).

- [ ] **Step 6: Run and verify FAIL, then GREEN**

```bash
cargo test -p map-canon the_blessed_censuses 2>&1 | tail -20
```
Expected first: compile error (`census_json` not found). After implementing: PASS — **if it does not pass on today's code, STOP.** That means the checked-in `data/canon/canon.json` and the blessed fixtures already disagree, and the stage cannot be judged until the owner rules on why.

- [ ] **Step 7: Full gate sweep and commit**

```bash
make contract-gates
```
```bash
git add crates/map-adapters/src/fingerprint.rs crates/map-adapters/src/lib.rs \
        crates/map-adapters/src/tests.rs crates/map-canon/src/lib.rs crates/map-canon/src/tests.rs \
        crates/map-viewer/src/lib.rs
git commit -m "Stage 2 Task 1: freeze the evidence — a disposition fingerprint and an offline census harness"
```

---

### Task 2: `map_canon::ledger` — Standing, typed Endurance, and PresenceBook promoted to universal

Spec §2: *"**Standing** — entity × right-open interval: when it exists. PresenceBook promoted to universal. Endurance is a typed field with a written reason, so the frame-edge law stops grepping prose."*

`PresenceBook` (`crates/map-canon/src/lib.rs:196-274`) is already the right shape and already carries the right laws (right-open spans, disjointness per key, totality — an undeclared claimant stands always, derived eras as the standing edges). What it is not is *universal*: its keys are bare `String`s drawn from **four different namespaces at once** in `partition_bridge.rs` — tribal cohort slugs (`:524-535`), the literal `"canaan"` (`:541-543`), openbible region slugs (`:544-550`), and atlas polity *era* keys of the form `"{id}@{from_year}"` (`:555-561`) — plus a second book, `entity_disjoint` (`:554`, `:562-564`), keyed by entity id to catch era-variants that would stand twice at once.

The promotion is exactly this: **one book, rows keyed by `(EntityId, WitnessKey)`, with the entity-disjointness check becoming a law of the book rather than a second book.** The `WitnessKey` is what today's String key is — WHICH witness's standing this is — and it is load-bearing: `bundle_faces(store, &part, &era.absent)` names a face by its first *present* claimant, and the atlas polity era-variants of one entity must stay distinguishable or the face naming changes and the census moves.

**Files:**
- Create: `crates/map-canon/src/ledger.rs`
- Modify: `crates/map-canon/src/lib.rs` (add `pub mod ledger;` and re-exports)
- Modify: `crates/map-canon/src/persist.rs` (the ledger crosses the compile→serve boundary)
- Modify: `crates/map-canon/src/tests.rs`

**Interfaces:**
- Consumes: `map_canon::{EntityId, Timestamp}`; Stage 1's `map_canon::registry::Registry` (for `resolve`, used by Task 7, not here).
- Produces:
```rust
/// WHICH witness's standing this is. Today's PresenceBook String key,
/// given a type: a tribal cohort slug, an openbible region slug, or an
/// atlas polity era key ("{id}@{from_year}"). One entity may hold
/// several, disjoint in time — that is an era-variant, not a
/// contradiction.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WitnessKey(pub String);

/// WHY something reaches the frame's edge. Spec §2: "endurance is a
/// typed field with a written reason, so the frame-edge law stops
/// grepping prose." `Endures` cannot be constructed without a reason,
/// so an undeclared endurance is unrepresentable rather than merely
/// discouraged.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Endurance {
    /// The standing ends at `until`; nothing to justify.
    Bounded,
    /// It reaches the frame's edge, because:
    Endures { reason: String, source: String },
}

/// ONE LEDGER ROW: entity × witness × right-open interval.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Standing {
    pub entity: EntityId,
    pub witness: WitnessKey,
    pub from: Timestamp,
    /// Right-open: `None` means "to the frame's edge", which is legal
    /// only with `Endurance::Endures`.
    pub until: Option<Timestamp>,
    pub endurance: Endurance,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ledger { /* private */ }

impl Ledger {
    pub fn declare(&mut self, row: Standing) -> Result<(), LedgerError>;
    /// TOTAL: an entity with no row stands always (PresenceBook's own
    /// rule, kept). Never panics, never returns an Option a caller
    /// cannot act on.
    pub fn stands(&self, witness: &WitnessKey, at: &Timestamp) -> bool;
    pub fn standing_of(&self, witness: &WitnessKey, at: &Timestamp) -> Option<&Standing>;
    pub fn standings_of_entity(&self, entity: &EntityId) -> Vec<&Standing>;
    pub fn standings_at(&self, at: &Timestamp) -> Vec<&Standing>;
    pub fn absent_at(&self, at: &Timestamp) -> BTreeSet<WitnessKey>;
    /// The derived eras from t0 — byte-identical to
    /// PresenceBook::eras, which this replaces.
    pub fn eras(&self, t0: Timestamp) -> Vec<Era>;
    pub fn rows(&self) -> impl Iterator<Item = &Standing>;
    pub fn validate(&self) -> Vec<LedgerViolation>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerError { BackwardsSpan, OverlappingSpans, EnduresWithAnEnd, EndlessWithoutAReason }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerViolation {
    /// One entity standing under two witnesses at one instant — the
    /// law `entity_disjoint` enforced as a second PresenceBook.
    EntityStandsTwice { entity: EntityId, at: Timestamp, a: WitnessKey, b: WitnessKey },
}
```
Task 3 consumes `Standing` and `Endurance`; Task 4 consumes `standing_of`; Tasks 5-8 write rows; Task 10 serves them.

- [ ] **Step 1: Write the failing tests**

Append to `crates/map-canon/src/tests.rs`:

```rust
#[test]
fn the_ledger_is_total_right_open_and_refuses_undeclared_endurance() {
    use crate::ledger::*;
    let e = |s: &str| EntityId(s.to_string());
    let w = |s: &str| WitnessKey(s.to_string());
    let mut l = Ledger::default();

    // TOTALITY: an undeclared witness stands always. This is
    // PresenceBook's own rule (lib.rs:231-238) and every caller
    // depends on it, so it is pinned before anything else.
    assert!(l.stands(&w("never-declared"), &ts(-1405)));

    // A bounded standing is right-open: the `from` year stands, the
    // `until` year does NOT.
    l.declare(Standing {
        entity: e("israel"), witness: w("israel@-1095"),
        from: ts(-1095), until: Some(ts(-975)), endurance: Endurance::Bounded,
    })
    .expect("a bounded standing");
    assert!(l.stands(&w("israel@-1095"), &ts(-1095)), "the from year stands");
    assert!(l.stands(&w("israel@-1095"), &ts(-976)), "the year before until stands");
    assert!(!l.stands(&w("israel@-1095"), &ts(-975)), "right-open: until does NOT stand");
    assert!(!l.stands(&w("israel@-1095"), &ts(-1096)), "before from, nothing stands");

    // BACKWARDS and OVERLAPPING are refused, by name.
    assert_eq!(
        l.declare(Standing {
            entity: e("x"), witness: w("x"), from: ts(-100), until: Some(ts(-200)),
            endurance: Endurance::Bounded,
        }),
        Err(LedgerError::BackwardsSpan)
    );
    assert_eq!(
        l.declare(Standing {
            entity: e("israel"), witness: w("israel@-1095"),
            from: ts(-1000), until: Some(ts(-900)), endurance: Endurance::Bounded,
        }),
        Err(LedgerError::OverlappingSpans)
    );

    // A RETURN is legal: disjoint spans under one witness.
    l.declare(Standing {
        entity: e("israel"), witness: w("israel@-1095"),
        from: ts(-900), until: Some(ts(-800)), endurance: Endurance::Bounded,
    })
    .expect("a claimant may return");

    // ENDURANCE IS TYPED AND WRITTEN. A row that reaches the frame's
    // edge without a reason is unrepresentable; a row that declares
    // endurance AND an end is a contradiction. Both refused by name —
    // this is the pair that lets validate_frame_edge stop grepping.
    assert_eq!(
        l.declare(Standing {
            entity: e("judaea"), witness: w("judaea"), from: ts(26), until: None,
            endurance: Endurance::Bounded,
        }),
        Err(LedgerError::EndlessWithoutAReason)
    );
    assert_eq!(
        l.declare(Standing {
            entity: e("judaea"), witness: w("judaea"), from: ts(26), until: Some(ts(70)),
            endurance: Endurance::Endures {
                reason: "the tetrarchies are the frame's final political order".into(),
                source: "data/authored/standings.json".into(),
            },
        }),
        Err(LedgerError::EnduresWithAnEnd)
    );
    l.declare(Standing {
        entity: e("judaea"), witness: w("judaea"), from: ts(26), until: None,
        endurance: Endurance::Endures {
            reason: "the tetrarchies are the frame's final political order".into(),
            source: "data/authored/standings.json".into(),
        },
    })
    .expect("endurance with a written reason");
    assert!(l.stands(&w("judaea"), &ts(1000)), "an enduring standing reaches the edge");
}

#[test]
fn one_entity_never_stands_under_two_witnesses_at_once() {
    // The law `entity_disjoint` enforced as a SECOND PresenceBook
    // (partition_bridge.rs:554, :562-564), now a law of the one book.
    use crate::ledger::*;
    let e = |s: &str| EntityId(s.to_string());
    let w = |s: &str| WitnessKey(s.to_string());
    let mut l = Ledger::default();
    l.declare(Standing {
        entity: e("egypt"), witness: w("egypt@-1500"),
        from: ts(-1500), until: Some(ts(-1200)), endurance: Endurance::Bounded,
    })
    .unwrap();
    // Disjoint era-variants of ONE entity are legal and must stay so:
    // that is how an empire morphs through the transition machinery.
    l.declare(Standing {
        entity: e("egypt"), witness: w("egypt@-1200"),
        from: ts(-1200), until: Some(ts(-1000)), endurance: Endurance::Bounded,
    })
    .unwrap();
    assert_eq!(l.validate(), Vec::new(), "disjoint era-variants are lawful");

    // OVERLAPPING era-variants are the violation, named with both
    // witnesses so the compile report can say which two.
    l.declare(Standing {
        entity: e("egypt"), witness: w("egypt@-1300-alt"),
        from: ts(-1300), until: Some(ts(-1100)), endurance: Endurance::Bounded,
    })
    .unwrap();
    let v = l.validate();
    assert!(
        v.iter().any(|x| matches!(x, LedgerViolation::EntityStandsTwice { entity, .. } if entity == &e("egypt"))),
        "one entity under two witnesses at one instant is a violation, got {v:?}"
    );
}

#[test]
fn the_ledgers_eras_equal_the_presence_books_eras() {
    // THE PROMOTION, PROVED. The Ledger replaces PresenceBook; if its
    // derived eras differ by one cut, every face naming downstream
    // moves and the census with it. Build the same standings both
    // ways and demand the same eras.
    use crate::ledger::*;
    let mut book = crate::PresenceBook::default();
    let mut l = Ledger::default();
    let rows = [
        ("canaan", -4004, Some(-1095)),
        ("dan", -1406, Some(-1050)),
        ("egypt@-1500", -1500, Some(-1200)),
        ("phoenicia", -1406, None),
    ];
    for (who, from, until) in rows {
        book.declare(who, ts(from), until.map(ts)).expect("presence");
        l.declare(Standing {
            entity: EntityId(who.split('@').next().unwrap().to_string()),
            witness: WitnessKey(who.to_string()),
            from: ts(from),
            until: until.map(ts),
            endurance: match until {
                Some(_) => Endurance::Bounded,
                None => Endurance::Endures { reason: "test".into(), source: "test".into() },
            },
        })
        .expect("ledger");
    }
    assert_eq!(l.eras(ts(-4004)), book.eras(ts(-4004)), "the promotion changes no era cut");

    // DISCRIMINATION: an added standing must change the cut set, or
    // the equality above is satisfiable by two constant functions.
    l.declare(Standing {
        entity: EntityId("moab".into()), witness: WitnessKey("moab".into()),
        from: ts(-1300), until: Some(ts(-1200)), endurance: Endurance::Bounded,
    })
    .unwrap();
    assert_ne!(l.eras(ts(-4004)), book.eras(ts(-4004)), "a new standing must add cuts");
}

#[test]
fn the_ledger_round_trips_through_persistence() {
    use crate::ledger::*;
    let mut store = CanonStore::default();
    let mut l = Ledger::default();
    l.declare(Standing {
        entity: EntityId("judaea".into()), witness: WitnessKey("judaea".into()),
        from: ts(26), until: None,
        endurance: Endurance::Endures { reason: "why".into(), source: "src".into() },
    })
    .unwrap();
    l.declare(Standing {
        entity: EntityId("dan".into()), witness: WitnessKey("dan".into()),
        from: ts(-1406), until: Some(ts(-1050)), endurance: Endurance::Bounded,
    })
    .unwrap();
    store.set_ledger(l.clone());
    let bytes = crate::persist::to_bytes(&store).expect("to_bytes");
    let back = crate::persist::from_bytes(&bytes).expect("from_bytes");
    assert_eq!(back.ledger(), &l, "the ledger crosses the compile->serve boundary whole");
    // DISCRIMINATION: the endurance REASON must survive, not just the
    // variant — a reader that dropped it would still round-trip a
    // Bounded/Endures distinction and look fine.
    let row = back.ledger().standing_of(&WitnessKey("judaea".into()), &ts(26)).expect("row");
    assert!(matches!(&row.endurance, Endurance::Endures { reason, .. } if reason == "why"));
}
```

- [ ] **Step 2: Run and verify FAIL** — `cargo test -p map-canon ledger 2>&1 | tail -20`. Expected: `module ledger not found` / `no method set_ledger`.

- [ ] **Step 3: Implement `crates/map-canon/src/ledger.rs`**

Write the module to the interface above. Invariants enforced in the code, not in comments:

- `declare` refuses `BackwardsSpan` (`until <= from`), `OverlappingSpans` (against the same `WitnessKey`'s existing rows — reuse `PresenceBook::declare`'s exact overlap test at `lib.rs:216-223`, which is already correct), `EnduresWithAnEnd` (`until.is_some() && matches!(endurance, Endures{..})`) and `EndlessWithoutAReason` (`until.is_none() && endurance == Bounded`). The last two are what make endurance typed rather than hoped-for.
- `Endures { reason, source }` — the constructor takes both. `source` names the owner-reviewable file the reason came from, exactly as Stage 1's `Unification::Declared` does. A reason with no source is a reason nobody can audit.
- `stands` is **total**: no row for a witness ⇒ `true`. Never panics.
- `eras` reproduces `PresenceBook::eras` (`lib.rs:244-273`) line for line, with `WitnessKey` in place of `String` in the `absent` set. Do not "improve" it; the test above compares against the original and a difference is a census move.
- `validate` returns **every** violation, never the first. `EntityStandsTwice` is found by grouping rows by entity and testing each pair of that entity's rows for interval intersection — with the pair sorted so the report is deterministic.
- Internal storage: `BTreeMap<WitnessKey, Vec<Standing>>` with each vec sorted by `from`, mirroring `PresenceBook::spans`. `Ledger` derives `PartialEq`/`Eq` so the persistence round-trip can compare whole books.

Add to `crates/map-canon/src/lib.rs`:
```rust
pub mod ledger;
pub use ledger::{Endurance, Ledger, LedgerError, LedgerViolation, Standing, WitnessKey};
```
and give `CanonStore` a ledger field with `pub fn set_ledger(&mut self, l: ledger::Ledger)` and `pub fn ledger(&self) -> &ledger::Ledger` — the same accessor shape Stage 1 gave `registry()`.

**`PresenceBook` is NOT deleted in this task.** It has one live caller (`partition_bridge.rs`) and that caller moves in Task 7. Deleting it here would make Task 7's "prove the eras did not change" impossible.

- [ ] **Step 4: Extend persistence**

In `crates/map-canon/src/persist.rs`, add a `"ledger"` key to the root object written by `to_bytes` and read by `from_bytes`, following the file's existing style (ordered maps in, byte-stable JSON out):

```rust
    let mut ledger = Vec::new();
    for row in store.ledger().rows() {
        let mut m = Map::new();
        m.insert("entity".into(), json!(row.entity.0));
        m.insert("witness".into(), json!(row.witness.0));
        m.insert("from".into(), ts_json(&row.from));
        if let Some(u) = &row.until {
            m.insert("until".into(), ts_json(u));
        }
        match &row.endurance {
            Endurance::Bounded => {}
            Endurance::Endures { reason, source } => {
                m.insert("endures".into(), json!({ "reason": reason, "source": source }));
            }
        }
        ledger.push(Value::Object(m));
    }
    root.insert("ledger".into(), Value::Array(ledger));
```
and on the way back in, rebuild through `Ledger::declare` — **never by writing the private map directly.** Loading is where a hand-edited canon gets caught; a loader that bypasses the constructor's laws is a loader that can produce an unlawful ledger. A `"ledger"` key that is absent loads as an empty ledger (a canon compiled before this stage), and that is the only tolerated absence.

- [ ] **Step 5: Run tests until green**

```bash
cargo test -p map-canon 2>&1 | grep "test result"
cargo test --workspace 2>&1 | grep -c "test result: ok"
```

- [ ] **Step 6: Full gates and commit** — no production behaviour changed yet, so the golden gate cannot move; run it anyway, because "cannot move" is a belief and the gate is evidence.

```bash
make contract-gates
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
```
```bash
git add crates/map-canon
git commit -m "map_canon::ledger: Standing with typed Endurance, PresenceBook promoted to universal"
```

---

### Task 3: `map_canon::law` — the law set as versioned, owner-reviewable data with a declared default

Spec §2: *"**Law** — data, versioned: tenure rules, precedence rules, supersession rules, dress rules. The knobs live here: one row, global reach."*

Today the knobs are seven tuned integers in a `match layer` (`canon_provider.rs:561-572`), an undeclared `.unwrap_or(2)` fallback (`:678`), two `match` blocks choosing an edge character and a paint route (`:287-311`, `:368-375`), and 50 `Holds`/`Grade` values in a Rust table. This task builds the type and the file; Tasks 5-9 move the values into it.

**The totality discipline, stated once and enforced by the type.** `dispose` is a total function (spec §2 equation 4). A law set with no matching rule would make it partial, and both honest repairs are bad: `unwrap_or` a number (a tuned constant — the owner's law forbids it) or return an `Option` (partiality by another name). So: **every rule family carries a mandatory declared default, `Laws::validate` refuses a set without one, and the default is a law row with an id like any other, so a disposition that fell through to it *says so in its citations*.** A default that cannot be cited is a special case wearing a law's clothes.

**Files:**
- Create: `crates/map-canon/src/law.rs`
- Create: `data/authored/laws.json`
- Modify: `crates/map-canon/src/lib.rs` (add `pub mod law;` and re-exports)
- Modify: `crates/map-canon/src/tests.rs`

**Interfaces:**
- Consumes: `map_canon::{EntityId, LayerKind, Tenure}`, `map_canon::ledger::Standing`.
- Produces:
```rust
/// The citation. Every disposition names the rows that produced it,
/// so "why is this claimed?" is answerable without reading Rust.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LawId(pub String);

/// WHAT KIND OF CANON FACT is being disposed. Mirrors `Feature`'s
/// variants; a rule may match on it (a Way is never Claimed).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FactKind { Area, Way, Point, Line, Memory }

/// HOW THE SHAPE IS KNOWN — spec §2's Witness `provenance`
/// (BorderText | CityDerived | Traced | Vendored), which is
/// `surveys::Grade` promoted into one queryable place. Named
/// `Derivation` and not `Provenance` because `map_canon::Provenance`
/// already exists and means something else (witness + verses + note);
/// two types with one name is how a codebase starts lying.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Derivation { BorderText, CityDerived, Traced, Vendored }

/// WHAT DRESS a disposition wears: the two decisions the provider
/// makes today at canon_provider.rs:287-311 and :368-375, named.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeStyle { Line, Unknown }
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FillRoute { Palette, Water, ReliefRamp, Region, None }
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DressClass { pub edge: EdgeStyle, pub fill: FillRoute }

/// A rule's LEFT-HAND SIDE. Every field is optional and an omitted
/// field matches anything; a Selector with every field None is the
/// family's default and matches everything, which is exactly what
/// makes the default a row rather than an exception.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selector {
    pub layer: Option<LayerKind>,
    pub kind: Option<FactKind>,
    pub derivation: Option<Derivation>,
    pub entity: Option<EntityId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TenureRule { pub id: LawId, pub when: Selector, pub then: Tenure, pub because: String }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DressRule { pub id: LawId, pub when: Selector, pub then: DressClass, pub because: String }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrecedenceRule { pub id: LawId, pub when: Selector, pub rank: u8, pub because: String }

/// WHO WINS when two witnesses speak for one entity at one instant.
/// The shadow spans (map-compile/src/main.rs:408-413) become these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupersessionRule {
    pub id: LawId,
    /// The witness whose standing SUPERSEDES.
    pub winner: Selector,
    /// The witness that yields for the duration of the winner's standing.
    pub loser: Selector,
    pub because: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Laws {
    pub version: String,
    pub tenure: Vec<TenureRule>,
    pub dress: Vec<DressRule>,
    pub precedence: Vec<PrecedenceRule>,
    pub supersession: Vec<SupersessionRule>,
}

impl Laws {
    pub fn parse(json: &str) -> Result<Laws, String>;
    pub fn render(&self) -> String;                  // law: parse . render == id
    /// The content hash of the rendered law set. `contract()` reports
    /// `version@digest` so a law-set edit that forgot its version bump
    /// is loud rather than silent.
    pub fn digest(&self) -> u64;
    pub fn validate(&self) -> Vec<LawViolation>;
    /// TOTAL. The last matching rule wins (later rows override
    /// earlier ones, so the default is written FIRST and the file
    /// reads top to bottom as "in general X, except Y"). `validate`
    /// guarantees a default exists, so these never return None.
    pub fn tenure_for(&self, s: &Selector) -> (Tenure, LawId);
    pub fn dress_for(&self, s: &Selector) -> (DressClass, LawId);
    pub fn rank_for(&self, s: &Selector) -> (u8, LawId);
    pub fn supersessions_for(&self, loser: &Selector) -> Vec<&SupersessionRule>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LawViolation {
    /// A rule family with no catch-all row: `dispose` would be partial.
    NoDefault(&'static str),
    /// Two rows with one id: a citation that names two rules names none.
    DuplicateId(LawId),
    /// A row that can never match, because a later row with an
    /// identical selector overrides it. Dead law is misleading law.
    Unreachable(LawId),
    /// A rule with an empty `because`. The knob is allowed; the
    /// unexplained knob is not.
    Unexplained(LawId),
}
```
Task 4 consumes `tenure_for`/`dress_for`/`rank_for`; Task 8 consumes `supersessions_for`; Task 12 serves `Laws` and its `digest`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/map-canon/src/tests.rs`:

```rust
#[test]
fn a_law_set_without_a_default_is_refused_so_dispose_is_never_partial() {
    use crate::law::*;
    // A tenure family with only a specific rule. `dispose` over an
    // unmatched selector would have no answer -- so the SET is
    // invalid, and the invalidity is named per family.
    let partial = Laws {
        version: "test-0".into(),
        tenure: vec![TenureRule {
            id: LawId("tenure/water".into()),
            when: Selector { layer: Some(LayerKind::Water), ..Default::default() },
            then: Tenure::Held,
            because: "water is held by nobody and everybody".into(),
        }],
        dress: Vec::new(),
        precedence: Vec::new(),
        supersession: Vec::new(),
    };
    let v = partial.validate();
    assert!(v.contains(&LawViolation::NoDefault("tenure")), "got {v:?}");
    assert!(v.contains(&LawViolation::NoDefault("dress")), "got {v:?}");
    assert!(v.contains(&LawViolation::NoDefault("precedence")), "got {v:?}");
}

#[test]
fn the_last_matching_rule_wins_and_says_which_one_it_was() {
    use crate::law::*;
    let laws = Laws {
        version: "test-0".into(),
        tenure: vec![
            TenureRule { id: LawId("tenure/default".into()), when: Selector::default(),
                         then: Tenure::Claimed, because: "unless a witness says it holds".into() },
            TenureRule { id: LawId("tenure/territory".into()),
                         when: Selector { layer: Some(LayerKind::Territory), ..Default::default() },
                         then: Tenure::Held, because: "an atlas polity governs its ground".into() },
        ],
        dress: vec![DressRule { id: LawId("dress/default".into()), when: Selector::default(),
                                then: DressClass { edge: EdgeStyle::Unknown, fill: FillRoute::Region },
                                because: "an undisclosed shape discloses itself".into() }],
        precedence: vec![PrecedenceRule { id: LawId("rank/default".into()), when: Selector::default(),
                                          rank: 2, because: "the declared middle".into() }],
        supersession: Vec::new(),
    };
    assert_eq!(laws.validate(), Vec::new(), "a complete set is lawful");

    // The specific rule wins over the default, and NAMES itself.
    let terr = Selector { layer: Some(LayerKind::Territory), kind: Some(FactKind::Area), ..Default::default() };
    assert_eq!(laws.tenure_for(&terr), (Tenure::Held, LawId("tenure/territory".into())));

    // TOTALITY with a citation: an unmatched selector falls to the
    // declared default and SAYS SO. That citation is the whole
    // difference between a law and an `unwrap_or`.
    let bg = Selector { layer: Some(LayerKind::Background), ..Default::default() };
    assert_eq!(laws.tenure_for(&bg), (Tenure::Claimed, LawId("tenure/default".into())));

    // A rule's selector must not match a query that said LESS than
    // the rule did: a caller who forgot to fill a field must get the
    // default, never a specific rule's answer by accident.
    assert_eq!(laws.tenure_for(&Selector::default()), (Tenure::Claimed, LawId("tenure/default".into())));
}

#[test]
fn the_shipped_law_set_round_trips_and_its_digest_notices_an_edit() {
    use crate::law::*;
    let text = std::fs::read_to_string(fixture_path("../../data/authored/laws.json"))
        .expect("the authored law set");
    let laws = Laws::parse(&text).expect("laws.json parses");
    assert_eq!(laws.validate(), Vec::new(), "the SHIPPED set is lawful, not just a test one");
    assert_eq!(Laws::parse(&laws.render()).unwrap(), laws, "parse . render == id");

    // DISCRIMINATION: the digest must move when a rule moves, or
    // `contract()`'s laws pin is decoration.
    let mut edited = laws.clone();
    edited.tenure[0].then = match edited.tenure[0].then {
        Tenure::Held => Tenure::Claimed,
        Tenure::Claimed => Tenure::Held,
    };
    assert_ne!(edited.digest(), laws.digest(), "an edited rule must move the digest");
    // ...and when only the REASON moves, too: a rule whose written
    // justification changed is a different rule to a reviewer.
    let mut reworded = laws.clone();
    reworded.tenure[0].because.push_str(" (reworded)");
    assert_ne!(reworded.digest(), laws.digest(), "a reworded reason must move the digest");
}

#[test]
fn dead_duplicate_and_unexplained_rules_are_named() {
    use crate::law::*;
    let mk = |id: &str, because: &str| TenureRule {
        id: LawId(id.into()), when: Selector::default(), then: Tenure::Held,
        because: because.into(),
    };
    let laws = Laws {
        version: "test-0".into(),
        tenure: vec![mk("a", "first"), mk("a", "same id"), mk("c", "")],
        dress: Vec::new(), precedence: Vec::new(), supersession: Vec::new(),
    };
    let v = laws.validate();
    assert!(v.contains(&LawViolation::DuplicateId(LawId("a".into()))), "got {v:?}");
    assert!(v.contains(&LawViolation::Unexplained(LawId("c".into()))), "got {v:?}");
    // The first two rows can never match: `c` has an identical
    // selector and comes later. Dead law is misleading law.
    assert!(v.iter().any(|x| matches!(x, LawViolation::Unreachable(_))), "got {v:?}");
}

#[test]
fn a_law_file_with_a_misspelled_section_fails_loudly() {
    use crate::law::*;
    // A silently-ignored "tenur" section would ship a set with NO
    // tenure rules, which `validate` then reports as NoDefault -- true,
    // and naming the wrong problem. Refuse unknown keys at the parse.
    let bad = r#"{"version":"x","tenur":[],"dress":[],"precedence":[],"supersession":[]}"#;
    let err = Laws::parse(bad).expect_err("an unknown section is refused");
    assert!(err.contains("tenur"), "the error names the offending key, got: {err}");
}
```

- [ ] **Step 2: Run and verify FAIL** — `cargo test -p map-canon law 2>&1 | tail -20`. Expected: `module law not found`.

- [ ] **Step 3: Implement `crates/map-canon/src/law.rs`**

Write the module to the interface above. The rules that matter:

- **Matching.** `Selector::matches(&self, query: &Selector) -> bool` — a `None` field in the *rule's* selector matches anything; a `Some(x)` matches only `Some(x)` in the query. Not the other way round, and not both ways.
- **Resolution.** `tenure_for` scans the family and keeps the **last** match, returning `(value, id)`. `validate`'s `NoDefault` guarantees at least one match, so write the fall-through as `.expect("Laws::validate guarantees a declared default")` — an `expect` whose message names the broken guarantee, never an index panic and never a silent number.
- **`digest`** hashes `render()`'s bytes. `render` writes the same JSON `parse` reads, with sorted keys, so `parse . render == id` holds and the digest is stable across runs and machines.
- **`Unreachable`** is detected by: a rule is unreachable iff some later rule in the same family has an *identical* selector. Do not attempt subsumption analysis — an over-clever reachability checker that flags a legitimate rule is worse than one that misses a subtle case, and identical-selector shadowing is the case that actually happens when someone edits the file.
- **`parse` refuses unknown keys**, at both the root and inside every rule object.
- `validate` returns **every** violation, never the first: a compile that stops at the first typo makes the owner run it twenty times.

- [ ] **Step 4: Write `data/authored/laws.json` with ONLY its declared defaults**

The rules that reproduce today's behaviour arrive in Tasks 5-9, each with its own equality proof. This task ships the file with its defaults, its comment, and nothing it has not earned:

```json
{
  "_comment": "THE LAW SET (2026-09-07, spec §2). Tenure, dress, precedence and supersession rules — the knobs, one row each, global reach. Every rule carries an id (dispositions cite it), a selector (an omitted field matches anything), and a written reason. The FIRST row of each family is its declared default: `dispose` is a total function, so 'no rule matched' must be unrepresentable, and a default that cannot be cited is a special case wearing a law's clothes. Later rows override earlier ones, so this file reads top to bottom as 'in general X, except Y'. A change here is reviewed AS A CENSUS DIFF before any pixel renders. OWNER-REVIEWABLE DATA.",
  "version": "0.3.0-stage2",
  "tenure": [
    {
      "id": "tenure/default-held",
      "when": {},
      "then": "held",
      "because": "The canon's own default (map_canon::Tenure derives Held, lib.rs:60-65), and what census reports today for every fact kind but a Claim-class Area. Stage 2 preserves it exactly; whether it SHOULD be Claimed is a policy change with its own census diff and is not this stage's to make."
    }
  ],
  "dress": [
    {
      "id": "dress/default",
      "when": {},
      "then": { "edge": "line", "fill": "region" },
      "because": "The provider's own fall-through at canon_provider.rs:299-309 and :371-374: a walked edge and the style's plain region paint."
    }
  ],
  "precedence": [
    {
      "id": "rank/default",
      "when": {},
      "rank": 2,
      "because": "The undeclared `.unwrap_or(2)` at canon_provider.rs:678, now declared. A region with no layer rank ranked with the scripture frame; that was true and unwritten, and it is now true and written."
    }
  ],
  "supersession": []
}
```

Add to `crates/map-canon/src/lib.rs`:
```rust
pub mod law;
pub use law::{
    Derivation, DressClass, DressRule, EdgeStyle, FactKind, FillRoute, LawId, LawViolation, Laws,
    PrecedenceRule, Selector, SupersessionRule, TenureRule,
};
```

- [ ] **Step 5: Run tests until green** — `cargo test -p map-canon law 2>&1 | grep "test result"`.

- [ ] **Step 6: Full gates and commit**

```bash
make contract-gates
```
```bash
git add crates/map-canon data/authored/laws.json
git commit -m "map_canon::law: the law set as versioned data, with a declared default so dispose stays total"
```

---

### Task 4: `dispose` — the total function, and the law that the census is its image

Spec §2 equation 4: *"`dispose : (Entity, Witness*, Standing, Laws) → {tenure, dress-class, rank}` is a total pure function; the census is its image, emitted every build; a law change is reviewed as a census diff before any pixel renders."*

This task builds `dispose` and rewires `census`'s `tenure` column to come from it — **producing byte-identical output**. Doing it now, before any literal moves, is what gives Tasks 5-9 somewhere to move a literal *to*, with Task 1's harness proving at each step that no answer changed.

**Files:**
- Modify: `crates/map-canon/src/lib.rs` (`dispose`, `Disposition`, `census`/`census_json`/`census_diff` signatures)
- Modify: `crates/map-canon/src/tests.rs`
- Modify: `crates/map-viewer/src/lib.rs` (load the law set at boot; thread it into the census route)
- Modify: `contracts/map-api/fact/census.feature`

**Interfaces:**
- Consumes: `map_canon::registry::Entity` (Stage 1), `map_canon::ledger::Standing`, `map_canon::law::*`.
- Produces:
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Disposition {
    pub tenure: Tenure,
    pub dress: law::DressClass,
    pub rank: u8,
    /// Every rule that produced an answer above, in family order
    /// (tenure, dress, precedence). A disposition that cannot say why
    /// is a disposition nobody can argue with.
    pub citations: Vec<law::LawId>,
}

/// TOTAL and PURE. Spec §2 equation 4's `(Entity, Witness*, Standing,
/// Laws)`: the `Witness*` arrives as `entity.witnesses` plus the
/// `(layer, kind, derivation)` triple naming WHICH witness's
/// contribution is being disposed. Never panics; never returns an
/// Option; no I/O; no clock; no globals.
pub fn dispose(
    entity: Option<&registry::Entity>,
    layer: LayerKind,
    kind: law::FactKind,
    derivation: Option<law::Derivation>,
    standing: Option<&ledger::Standing>,
    laws: &law::Laws,
) -> Disposition;

pub fn census(store: &CanonStore, laws: &law::Laws, at: &Timestamp) -> Vec<CensusRow>;
pub fn census_json(store: &CanonStore, laws: &law::Laws, at: &Timestamp) -> serde_json::Value;
pub fn census_diff(store: &CanonStore, laws: &law::Laws, from: &Timestamp, to: &Timestamp) -> Vec<CensusChange>;
```
Tasks 5-9 write the rules `dispose` reads; Task 11 serves it.

> **Note on `census_diff`'s signature.** Stage 1's closing Interfaces say Stage 2 "must not modify" the diff instrument. Threading `&Laws` through is not a modification of its *meaning*: the wire shape of `GET /api/census?year=A&to=B` is untouched, `CensusChange` is untouched, the `(layer, entity)` keying is untouched. A reviewer should check exactly that — the instrument's *answers* must not change, only its arguments.

- [ ] **Step 1: Contract addition FIRST — the image law as a scenario, runner red**

Append to `contracts/map-api/fact/census.feature`:

```gherkin
  Scenario: the census is the image of the disposition function, whole
    When I GET /api/census?year=-1405 as table
    Then every row of table is disposition's answer for that entity
```

and extend the feature's preamble with these two sentences, because a preamble is part of the interface (diagnosis §9.2):

> Since v0.3 the `tenure` column is not a stored field but the output of
> `dispose` (spec §2 equation 4). These fixtures are therefore also the
> proof that moving the `Stands`/`Holds`/`Grade` literals into the ledger
> changed no answer: if a byte here moves, a policy moved with it.

Run the gate and see the orphan:
```bash
cd contracts/runner && cabal run contract-runner -- check ../map-api
```
Expected: an ORPHAN row naming `every row of table is disposition's answer for that entity`. The step definition lands in Task 11, when `/api/disposition` exists. This scenario is **not** tagged `@target`: it is a law this stage commits to making green, and a tag would be doing the work the code should do.

- [ ] **Step 2: Write the failing Rust tests**

Append to `crates/map-canon/src/tests.rs`:

```rust
fn shipped_laws() -> crate::law::Laws {
    crate::law::Laws::parse(
        &std::fs::read_to_string(fixture_path("../../data/authored/laws.json"))
            .expect("data/authored/laws.json"),
    )
    .expect("laws.json parses")
}

#[test]
fn dispose_is_total_over_every_layer_and_kind_and_cites_every_answer() {
    use crate::law::*;
    let laws = shipped_laws();
    // TOTALITY at its hardest: no entity, no derivation, no standing.
    // Each of these is a real case in the canon (a feature whose id
    // the registry has never seen resolves to itself and has no
    // Entity row; a bridged region has no Grade).
    for layer in [
        LayerKind::Territory, LayerKind::ScriptureClaims, LayerKind::Journeys,
        LayerKind::Water, LayerKind::Relief, LayerKind::Background,
    ] {
        for kind in [FactKind::Area, FactKind::Way, FactKind::Point, FactKind::Line, FactKind::Memory] {
            let d = crate::dispose(None, layer, kind, None, None, &laws);
            assert_eq!(d.citations.len(), 3, "one citation per family: tenure, dress, precedence");
            assert!(d.citations.iter().all(|c| !c.0.is_empty()),
                    "every answer names the rule that produced it");
        }
    }
}

#[test]
fn dispose_is_pure_and_actually_reads_the_law_set() {
    use crate::law::*;
    let laws = shipped_laws();
    let a = crate::dispose(None, LayerKind::Territory, FactKind::Area, None, None, &laws);
    let b = crate::dispose(None, LayerKind::Territory, FactKind::Area, None, None, &laws);
    assert_eq!(a, b, "same inputs, same answer, always");

    // DISCRIMINATION: a different law set must give a different
    // answer, or `dispose` is ignoring the laws it claims to read --
    // which is exactly the shape of diagnosis §7.0's defect.
    let mut flipped = laws.clone();
    flipped.tenure[0].then = match flipped.tenure[0].then {
        Tenure::Held => Tenure::Claimed,
        Tenure::Claimed => Tenure::Held,
    };
    let c = crate::dispose(None, LayerKind::Territory, FactKind::Area, None, None, &flipped);
    assert_ne!(a.tenure, c.tenure, "dispose must actually read the law set");
}

#[test]
fn the_census_is_disposes_image_row_for_row() {
    // Equation 4, as a Rust law over the REAL compiled canon: every
    // census row's tenure equals what dispose says for that row's
    // (entity, layer, kind). Not a sample -- every row, at all three
    // blessed instants.
    use crate::law::*;
    let store = load_compiled_canon();
    let laws = shipped_laws();
    for year in [-1405, -1050, 59] {
        let at = ts(year);
        let rows = crate::census(&store, &laws, &at);
        assert!(!rows.is_empty(), "the census at {year} is not empty");
        for r in &rows {
            let kind = match r.kind {
                "area" => FactKind::Area, "way" => FactKind::Way, "point" => FactKind::Point,
                "line" => FactKind::Line, "memory" => FactKind::Memory,
                other => panic!("unknown census kind '{other}' -- add it to FactKind"),
            };
            let eid = EntityId(r.entity.clone());
            let d = crate::dispose(
                store.registry().get(&eid),
                layer_from_census_name(r.layer),
                kind,
                derivation_of(&store, &eid),
                store.ledger().standing_of(&crate::ledger::WitnessKey(r.entity.clone()), &at),
                &laws,
            );
            let tenure_text = match d.tenure { Tenure::Held => "held", Tenure::Claimed => "claimed" };
            assert_eq!(r.tenure, tenure_text,
                       "row {} @ {} at {year} is not dispose's image", r.entity, r.layer);
        }
    }
}
```

Adaptation note: `layer_from_census_name` is the inverse of `layer_name` (`lib.rs:792-801`) — write it beside `layer_name` and make **both** exhaustive with no wildcard, so a new layer breaks the build rather than mapping to a guess. `derivation_of(store, entity) -> Option<Derivation>` returns `None` until Task 6 puts `Derivation` on the witness; write it now with a `// Task 6 fills this in` comment, so the call sites do not move later.

- [ ] **Step 3: Run and verify FAIL** — `cargo test -p map-canon dispose 2>&1 | tail -20`. Expected: `cannot find function dispose`.

- [ ] **Step 4: Implement `dispose` and rewire `census`**

In `crates/map-canon/src/lib.rs`:

```rust
/// The disposition of one fact: what it is, how it dresses, where it
/// paints, and WHICH RULES SAID SO.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Disposition {
    pub tenure: Tenure,
    pub dress: law::DressClass,
    pub rank: u8,
    pub citations: Vec<law::LawId>,
}

/// SPEC §2 EQUATION 4. Total, pure, and citing.
///
/// Total: every family has a declared default (LawViolation::NoDefault
/// refuses a set without one), so there is no input for which this has
/// no answer. Pure: no I/O, no clock, no globals — the same six
/// arguments always give the same Disposition, which is what lets the
/// census be diffed and a law change be reviewed as a table before any
/// pixel renders.
///
/// `standing` is accepted and not yet read: it is equation 4's fourth
/// argument, and Tasks 5-8 give the rules that consult it. Threading
/// it now means those tasks edit a law file rather than a signature.
pub fn dispose(
    entity: Option<&registry::Entity>,
    layer: LayerKind,
    kind: law::FactKind,
    derivation: Option<law::Derivation>,
    standing: Option<&ledger::Standing>,
    laws: &law::Laws,
) -> Disposition {
    let _ = standing;
    let q = law::Selector {
        layer: Some(layer),
        kind: Some(kind),
        derivation,
        entity: entity.map(|e| e.id.clone()),
    };
    let (tenure, t_id) = laws.tenure_for(&q);
    let (dress, d_id) = laws.dress_for(&q);
    let (rank, r_id) = laws.rank_for(&q);
    Disposition { tenure, dress, rank, citations: vec![t_id, d_id, r_id] }
}
```

Then rewrite `census`'s per-feature arm (`lib.rs:815-830`) so `tenure` comes from `dispose` rather than from `a.tenure` and the four hardcoded `"held"` string literals.

> **STOP AND READ BEFORE RUNNING — this task's one trap.** The shipped `laws.json` says `tenure/default-held` for everything. Today's census carries exactly **one `claimed` row** at −1405 and −1050 (the `Feature::Area` bridged with `RegionClass::Claim`) and none at AD 59. So a naive rewiring flips that row and `the_blessed_censuses_still_hold` fails. **That failure is correct** — it is the harness doing its job on the first task that could move a row.
>
> The fix is a law row, and the law row needs `Derivation`, which is **Task 6**. So write the seam explicitly, with this exact comment:
> ```rust
> // TASK 4→6 SEAM. Area tenure still comes from the stored field,
> // because the law that reproduces it needs `Derivation` on the
> // witness and Task 6 puts it there. The other four fact kinds
> // already agree with the law set. Task 6 deletes this branch and
> // the stored field with it; if you are reading this after Task 6,
> // the seam was left behind and that is a bug.
> let tenure = match f {
>     Feature::Area(a) => a.tenure,
>     _ => d.tenure,
> };
> ```
> **Do NOT close the seam early with a layer-keyed tenure rule.** `scripture-claims` carries 20 `held` partition faces and 1 `claimed` bridged region at −1405; a rule keyed on the layer cannot separate them, and reaching for one is exactly how twenty rows get flipped by accident.

Update `crates/map-viewer/src/lib.rs`: load `data/authored/laws.json` once at boot, beside the canon; **refuse to start** with a named error if it is missing or `validate()` is non-empty, rather than serving a census with no laws behind it. Thread `&Laws` into the `/api/census` arm and into the `to=` diff branch Stage 1 added.

- [ ] **Step 5: Rebuild the world and prove nothing moved**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
cargo run --release -p map-compile build
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
```bash
cargo test -p map-canon 2>&1 | grep "test result"
cargo test -p map-adapters the_scripture_timeline_fingerprint_is_pinned 2>&1 | grep "test result"
make contract-gates
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api 2>&1 | tail -20
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
```
Expected: `the_blessed_censuses_still_hold` PASSES; the three census scenarios green; the new image-law scenario RED with its orphan message (its step lands in Task 11); `ALL GOLDEN VIEWS HOLD`.

- [ ] **Step 6: Commit**

```bash
git add crates/map-canon crates/map-viewer contracts/map-api/fact/census.feature
git commit -m "dispose: spec equation 4's total pure function, and the census as its image"
```

---

## Part B — the literals move into the ledger

Every task in this part deletes a family of literals. Every one of them re-runs Task 1's two instruments as its own acceptance test: `the_scripture_timeline_fingerprint_is_pinned` and `the_blessed_censuses_still_hold`. **A red from either is not a test to fix — it is a policy that moved, and it goes to the owner before anything else happens.**

### Task 5: `Stands` dies — survey standings become ledger rows, and endurance stops being prose

Two things end here, and they end together because one is the other's enforcement:

1. **`Stands` as a Rust literal** — `crates/map-adapters/src/surveys.rs:778-784`, `:812`, and 25 values across `SURVEYS`/`SURVEYS_MORE`, consumed at `:1354-1357` (the `Interval`) and `:1321-1329` (the endurance prose).
2. **Prose-marker law enforcement** — `crates/map-canon/src/lib.rs:176` (`ENDURES_MARK`) and `:489-504`, where `validate_frame_edge` decides whether a feature may reach the frame's edge by running `p.note.contains("ENDURES to the frame's edge")` over a provenance string. Spec §2: *"endurance is a typed field with a written reason, so the frame-edge law stops grepping prose."*

They are inseparable: delete the prose without replacing the law and the compile dies at `main.rs:444-453`; replace the law without the prose's writers and nothing endures. Three code paths write that phrase today — `surveys.rs:1325-1328` (the four tetrarchies), `partition_bridge.rs:700` (26 gazetteer settlements), `partition_bridge.rs:682` (the drowned memory) — and all three get ledger rows in this task.

**Files:**
- Create: `data/authored/standings.json`
- Modify: `crates/map-adapters/Cargo.toml` (add `map-canon`), `crates/map-adapters/src/surveys.rs`, `crates/map-adapters/src/tests.rs`
- Modify: `crates/map-canon/src/lib.rs` (`Ledger::parse`; `validate_frame_edge` takes the ledger; `ENDURES_MARK` deleted), `crates/map-canon/src/ledger.rs`, `crates/map-canon/src/tests.rs`
- Modify: `crates/map-compile/src/main.rs`, `crates/map-compile/src/partition_bridge.rs`, `crates/map-compile/src/tests.rs`

**On the new dependency.** `map-canon/Cargo.toml`'s comment says *"no adapters, no providers, no encoders"* — that is a statement about what **map-canon** depends on, not about who may depend on map-canon. `map-adapters → map-canon` is a downward edge and introduces no cycle. Add it, and quote that comment in the commit message so a reviewer does not have to re-derive the reasoning.

**Interfaces:**
- Consumes: `map_canon::ledger::{Ledger, Standing, Endurance, WitnessKey}` (Task 2).
- Produces:
```rust
// map-canon: one parser for the authored standing rows, so map-adapters
// and map-compile cannot disagree about what a row means.
impl Ledger {
    pub fn parse(json: &str) -> Result<Ledger, String>;
    pub fn render(&self) -> String;                       // law: parse . render == id
}

/// The frame-edge law, no longer grepping. Every feature of `layer`
/// still standing at `edge` must have a ledger row whose endurance is
/// `Endures`. A feature with NO row stands always (the ledger's
/// totality rule) and therefore has no written reason — so it leaks,
/// which is exactly what "no ENDURES mark" meant before.
impl CanonStore {
    pub fn validate_frame_edge(&self, layer: LayerKind, edge: &Timestamp) -> Vec<String>;
}

// map-adapters
pub fn scripture_timeline_with(atlas: Option<&AtlasExports>, ledger: &Ledger) -> WorldTimeline;
pub fn authored_standings() -> Ledger;   // parses data/authored/standings.json
```
Task 6 adds `Derivation` to the same file's rows; Task 7 declares rows from geojson; Task 10 serves them.

- [ ] **Step 1: Write the failing tests**

Append to `crates/map-adapters/src/tests.rs`:

```rust
/// Every survey tag has EXACTLY ONE standing row, and the row's entity
/// is the one the bridge actually mints. This is the safety net for
/// the whole task: the authored file is keyed by tag, the canon is
/// keyed by entity, and a mismatch between them would silently drop a
/// standing rather than fail.
#[test]
fn every_survey_has_exactly_one_declared_standing_and_the_entities_match() {
    let ledger = crate::authored_standings();
    let tags = crate::survey_tags();                 // new: the table's own tags
    for tag in &tags {
        let rows: Vec<_> = ledger
            .rows()
            .filter(|r| r.witness.0 == format!("survey:{tag}"))
            .collect();
        assert_eq!(rows.len(), 1, "survey {tag} must have exactly one standing row");
    }
    // ...and nothing else: a row for a tag the table does not have is
    // a typo that would otherwise sit in the file forever.
    for row in ledger.rows() {
        if let Some(tag) = row.witness.0.strip_prefix("survey:") {
            assert!(tags.iter().any(|t| t == tag), "standings.json names unknown survey '{tag}'");
        }
    }
}

/// THE EQUALITY THAT MATTERS: the ledger says exactly what `Stands`
/// said. Written while BOTH exist, so the deletion in Step 5 is
/// mechanical rather than hopeful.
#[test]
fn the_ledger_reproduces_every_stands_literal() {
    let ledger = crate::authored_standings();
    for (tag, from_year, until_year, endures) in crate::legacy_stands_table() {
        let row = ledger
            .rows()
            .find(|r| r.witness.0 == format!("survey:{tag}"))
            .unwrap_or_else(|| panic!("no standing row for {tag}"));
        assert_eq!(row.from.year.get(), from_year, "{tag}: from year");
        assert_eq!(
            row.until.map(|t| t.year.get()),
            until_year,
            "{tag}: until year (None means the frame's edge)"
        );
        match (&row.endurance, endures) {
            (map_canon::Endurance::Bounded, None) => {}
            (map_canon::Endurance::Endures { reason, .. }, Some(why)) => {
                assert_eq!(reason, why, "{tag}: the endurance reason must be carried WORD FOR WORD");
            }
            (a, b) => panic!("{tag}: endurance mismatch: ledger {a:?} vs literal {b:?}"),
        }
    }
}

/// DISCRIMINATION: the equality test above must fail when the file is
/// wrong, or it is a check satisfiable by its own failure mode.
#[test]
fn a_wrong_standing_row_is_caught() {
    let mut ledger = crate::authored_standings();
    let victim = ledger.rows().next().expect("rows exist").witness.clone();
    ledger.force_shift_for_test(&victim, 7);   // test-only: move `from` by 7 years
    let table = crate::legacy_stands_table();
    let (tag, from_year, ..) = table.iter().find(|(t, ..)| victim.0 == format!("survey:{t}")).unwrap();
    let row = ledger.rows().find(|r| r.witness == victim).unwrap();
    assert_ne!(row.from.year.get(), *from_year, "the mutation really moved {tag}");
}
```

Adaptation notes:
- `survey_tags()` and `legacy_stands_table()` are two small `pub(crate)` functions you add to `surveys.rs` **in this step**, both derived from the existing `SURVEYS.iter().chain(SURVEYS_MORE)` walk. `legacy_stands_table()` returns `Vec<(&'static str, i32, Option<i32>, Option<&'static str>)>` — `(tag, s.year, until, endures_reason)` — computed from the `Stands` literal exactly as `add_survey:1354-1357` and `:1325-1328` compute it today. **Both die in Step 5 together with the literals**; they exist only so the equality can be asserted while both sides are present. Note the `year` subtlety: `add_survey` uses the *atlas-resolved* year when the atlas binds the survey's verses (`:1314`), falling back to `s.year`. `legacy_stands_table()` must use the unbound `s.year`, and so must `standings.json` — because the ledger is the authored declaration and the atlas binding is applied on top of it, exactly as it is today. Task 5 Step 4 keeps that application in `add_survey`.
- `force_shift_for_test` is a `#[cfg(test)] pub fn` on `Ledger`. If you would rather not add test-only surface to a production type, build the mutated ledger by re-parsing an edited JSON string instead — either is fine, a discrimination test that cannot mutate is not.

Append to `crates/map-canon/src/tests.rs`:

```rust
#[test]
fn the_frame_edge_law_reads_the_ledger_and_not_the_prose() {
    use crate::ledger::*;
    let mut store = CanonStore::default();
    let edge = ts(100);

    let enduring = area(&mut store, "place:hebron", square(31.5, 35.1, 0.5));
    let leaking = area(&mut store, "authored:phantom", square(32.5, 36.1, 0.5));
    territory_like_layer(&mut store, LayerKind::ScriptureClaims, edge, &[enduring, leaking]);

    // Provenance notes carry NO endurance phrase at all any more --
    // if the law were still grepping, everything would leak and this
    // test would pass for the wrong reason. So declare the ledger row
    // and assert that EXACTLY ONE of the two leaks.
    let mut l = Ledger::default();
    l.declare(Standing {
        entity: EntityId("place:hebron".into()),
        witness: WitnessKey("place:hebron".into()),
        from: ts(-4004), until: None,
        endurance: Endurance::Endures {
            reason: "a place-name outlives every polity".into(),
            source: "data/authored/standings.json".into(),
        },
    })
    .unwrap();
    l.declare(Standing {
        entity: EntityId("authored:phantom".into()),
        witness: WitnessKey("authored:phantom".into()),
        from: ts(-4004), until: Some(ts(200)), endurance: Endurance::Bounded,
    })
    .unwrap();
    store.set_ledger(l);

    let leaks = store.validate_frame_edge(LayerKind::ScriptureClaims, &edge);
    assert_eq!(leaks, vec!["authored:phantom".to_string()],
               "the declared one endures; the bounded one leaks, by name");
}

#[test]
fn a_feature_with_no_ledger_row_leaks_at_the_frame_edge() {
    // The ledger's totality rule says an undeclared witness stands
    // always -- so it reaches the edge, and it has no written reason,
    // so it LEAKS. That is exactly what "no ENDURES mark" meant, and
    // the migration must not quietly turn silence into permission.
    use crate::ledger::*;
    let mut store = CanonStore::default();
    let edge = ts(100);
    let f = area(&mut store, "authored:undeclared", square(31.0, 35.0, 0.5));
    territory_like_layer(&mut store, LayerKind::ScriptureClaims, edge, &[f]);
    store.set_ledger(Ledger::default());
    assert_eq!(store.validate_frame_edge(LayerKind::ScriptureClaims, &edge),
               vec!["authored:undeclared".to_string()]);
}
```

Adaptation note: `territory_like_layer` — the tests file already has `territory_with` (`tests.rs:181`) which does this for `LayerKind::Territory`; generalise it to take the layer, or write a two-line sibling. Note the leak list now names **entities**, not display names as `lib.rs:499` does today; that is a deliberate improvement (an entity id is actionable, a display name is ambiguous) and it changes `main.rs:447-452`'s error text, which is prose, not a law.

- [ ] **Step 2: Run and verify FAIL** — `cargo test -p map-adapters authored_standings 2>&1 | tail -20`. Expected: `cannot find function authored_standings`.

- [ ] **Step 3: Write `data/authored/standings.json`**

25 survey rows + the two partition-bridge endurance families. Every `until` is a year; a missing `until` **requires** an `endures` block (the ledger's own law refuses otherwise). Generate the 25 rows mechanically from `legacy_stands_table()` rather than by hand — a transcription error here is a moved census row — then read them:

```json
{
  "_comment": "THE STANDING LEDGER, authored rows (2026-09-07, spec §2). WHEN each authored thing stands: a right-open [from, until) interval per witness, and — where it reaches the frame's edge — the WRITTEN REASON the frame-edge law now reads instead of grepping a provenance string. These 25 survey rows were `Stands::Until(y)` and `Stands::Enduring(why)` literals in crates/map-adapters/src/surveys.rs until Stage 2. Standings that come from source data (openbible regions, tribal cohorts, atlas polity eras) are declared by their own witnesses at compile time and are NOT repeated here. OWNER-REVIEWABLE DATA.",
  "standings": [
    {
      "witness": "survey:PLATE-CANAAN",
      "entity": "authored:canaan-traced-contour",
      "from": -2200,
      "until": -1406,
      "note": "The tracing is the PRE-CONQUEST reference world; from the conquest the allotment carries the plate's story."
    },
    {
      "witness": "survey:NUM34",
      "entity": "authored:the-land-promised-num-34",
      "from": -1452,
      "until": -586,
      "note": "The promise-as-map yields at the exile: the loss of the land ends the survey's world, not the covenant."
    },
    {
      "witness": "survey:NT-JUDAEA",
      "entity": "authored:judea",
      "from": 26,
      "endures": {
        "reason": "the tetrarchies are the frame's final political order; nothing within the frame supersedes them",
        "source": "LUK 3:1"
      }
    },
    {
      "witness": "place-names",
      "entity": "*",
      "from": -4004,
      "endures": {
        "reason": "a place-name outlives every polity",
        "source": "data/authored/standings.json"
      },
      "note": "The gazetteer settlements written at partition_bridge.rs:700. `entity: \"*\"` is a FAMILY row — see the family rule below."
    },
    {
      "witness": "memories",
      "entity": "*",
      "from": -4004,
      "endures": {
        "reason": "a memory is kept, not governed by eras",
        "source": "GEN 14:3"
      },
      "note": "The drowned traditional sites written at partition_bridge.rs:682."
    }
  ]
}
```

**The family rule, and why it is a rule and not a shortcut.** Two of the writers above endure *by kind*, not by name: every gazetteer settlement, every drowned memory. Writing 27 identical rows would be data duplication that drifts. So `entity: "*"` declares a **family**: at compile time, `partition_bridge` expands it into one real `Standing` row per feature it mints in that family, all carrying the same written reason and source. The expansion is real — `store.ledger()` holds 27 rows, not one wildcard — so `standings`, `disposition` and the frame-edge law all see ordinary rows and no consumer learns the word "family". Assert exactly that in `crates/map-compile/src/tests.rs`:

```rust
#[test]
fn a_family_standing_expands_into_one_real_row_per_feature() {
    let store = compiled_store();          // this file's existing builder
    let rows: Vec<_> = store.ledger().rows()
        .filter(|r| matches!(&r.endurance,
            map_canon::Endurance::Endures { reason, .. } if reason == "a place-name outlives every polity"))
        .collect();
    assert!(rows.len() > 1, "the family expanded into real rows, got {}", rows.len());
    assert!(rows.iter().all(|r| r.entity.0 != "*"), "no wildcard survives into the ledger");
    assert!(rows.iter().all(|r| r.witness.0.starts_with("place:")),
            "each row is keyed by the feature it stands for");
}
```

Fill the remaining 22 survey rows (`N-GOMER` … `N-JOKTAN`, `EZK47`, `EZK48`, `NT-GALILEE`, `NT-PEREA`, `NT-ITUREA`) from `legacy_stands_table()`. The three remaining tetrarchies carry the same `endures.reason` word for word as `NT-JUDAEA`; the seventeen `NATIONS_*` rows are `from: -2247, until: -1406`; `EZK47`/`EZK48` are `from: -574, until: -538`.

- [ ] **Step 4: Implement — the ledger parser, the reader, and the new frame-edge law**

- `Ledger::parse` / `Ledger::render` in `crates/map-canon/src/ledger.rs`, refusing unknown keys, building through `declare` so the file cannot express an unlawful ledger.
- `map_adapters::authored_standings()` — `include_str!`-free: read the file through the same `data_path` helper `partition_bridge.rs:726-732` uses, so it resolves from both the workspace root and a crate dir.
- `scripture_timeline_with(atlas, ledger)` — `add_survey` takes the ledger and replaces `:1354-1357` with a lookup:
  ```rust
  // The interval is the LEDGER's, not a literal's. The atlas binding
  // still moves the `from` year when it dates the survey's verses
  // (see `year` above, resolved at :1314) -- the authored row is the
  // declaration and the atlas is the authority over WHEN, exactly as
  // it was before this stage.
  let row = ledger
      .standing_of(&WitnessKey(format!("survey:{}", s.tag)), &tp(s.year))
      .unwrap_or_else(|| panic!("no standing declared for survey {}", s.tag));
  let valid = Interval { from: tp(year), to: row.until };
  ```
  and replaces `:1321-1329`'s prose branch with a provenance string that no longer carries an endurance phrase:
  ```rust
  // Endurance is a TYPED FIELD on the ledger row now (spec §2). The
  // provenance text goes back to being what it says it is: where the
  // coordinates came from.
  let provenance = circuit_provenance(atlas, bound, s.circuit.len());
  ```
  A missing row is a `panic!` naming the tag, not a default: a survey with no declared standing is a data error the owner must see, and Step 1's first test makes it impossible to reach production.
- `CanonStore::validate_frame_edge` reads `self.ledger()`; delete `ENDURES_MARK` (`lib.rs:176`) and every reference to it. `cargo build` names the references; there should be exactly three writers plus the validator.
- `crates/map-compile/src/main.rs`: build the ledger before the scripture timeline, expand the two family rows during `bridge_partition`, `store.set_ledger(...)` before `validate_frame_edge` runs, and pass the ledger to `scripture_timeline_with`. `partition_bridge.rs:682` and `:700` drop their `ENDURES to the frame's edge:` sentences and declare rows instead.

- [ ] **Step 5: Delete `Stands`**

Remove `enum Stands` (`surveys.rs:775-784`), the `stands` field (`:806-812`), the `stands:` value from all 25 rows, and the two helpers `legacy_stands_table()`/`survey_tags()` written in Step 1 — together with the two tests that used them (`the_ledger_reproduces_every_stands_literal`, `a_wrong_standing_row_is_caught`). **Keep** `every_survey_has_exactly_one_declared_standing_and_the_entities_match`, rewritten to walk the surviving `SURVEYS`/`SURVEYS_MORE` tags. Their job is done: the literals are gone, so an equality against them can no longer be written, and Task 1's fingerprint is what guards the answers from here on.

- [ ] **Step 6: Rebuild the world and prove nothing moved**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
cargo run --release -p map-compile build
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
```bash
cargo test -p map-adapters the_scripture_timeline_fingerprint_is_pinned 2>&1 | grep "test result"
cargo test -p map-canon the_blessed_censuses_still_hold 2>&1 | grep "test result"
make contract-gates
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api 2>&1 | tail -15
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
```
Expected: the fingerprint UNCHANGED (the value pinned in Task 1 Step 4); the three census scenarios green; `frame-edge law: ScriptureClaims clean at the frame's last year` in the compile output; `ALL GOLDEN VIEWS HOLD`.

> **If the fingerprint moved:** compare `legacy_stands_table()`'s output against `standings.json` row by row before touching the pin. The likeliest causes, in order: a transcribed year off by one; a `NATIONS_UNTIL` row given `-1405` instead of `-1406`; a `Bounded` row that should have been `Endures`. **Do not re-pin.**

- [ ] **Step 7: Commit**

```bash
git add crates/map-adapters crates/map-canon crates/map-compile data/authored/standings.json
git commit -m "Stands dies: survey standings become ledger rows, and the frame-edge law stops grepping prose"
```

---

### Task 6: `Holds` and `Grade` die — tenure by declaration, dress by law, and the Task 4→6 seam closes

`Holds` (`surveys.rs:766-773`, 25 values, consumed at `:1371-1374` → `RegionClass`) and `Grade` (`:786-794`, 25 values, consumed at `:1343-1346` → `EdgeCharacter`) are the last two dispositional literals in the survey table. The file's own comment (`:1363-1370`) says why they are two things and not one: *"grade says how the shape is known; Holds says whether the ground is possessed — orthogonal axes. NUM 34 recites its border verse by verse yet bounds a promise; the tetrarchies are city-derived hulls yet governed real ground."* That orthogonality survives the migration and shapes it:

- **`Grade` promotes to `Derivation`** (Task 3's type) — a property of the *witness*, spec §2's `provenance (BorderText | CityDerived | Traced | Vendored)`. It rides into the canon on the feature's provenance and feeds `dispose`'s selector.
- **`Holds` becomes a per-entity tenure declaration with its written reason** — 21 rows are the default (`Claim`) and 4 are `Ground(why)`. The **law** is the one `surveys.rs:1369-1370` already states: *"A survey that holds ground says why, in writing; everything else is a Claim."* The default is a law row; the four exceptions are data rows carrying the justification they already carry.

Closing this also closes the **Task 4→6 seam**: with `Derivation` on the witness, `tenure/bridged-claim` can be written, `census` stops reading `Area.tenure`, and the stored field goes.

**Files:**
- Modify: `crates/map-canon/src/lib.rs` (`Provenance` gains `derivation`; `Area.tenure` deleted; `derivation_of` implemented), `crates/map-canon/src/persist.rs`, `crates/map-canon/src/tests.rs`
- Modify: `crates/map-adapters/src/surveys.rs`, `crates/map-adapters/src/tests.rs`
- Modify: `crates/map-compile/src/timeline_bridge.rs`, `crates/map-compile/src/partition_bridge.rs`, `crates/map-compile/src/tests.rs`
- Modify: `crates/map-provider/src/canon_provider.rs`, `crates/map-provider/src/tests.rs`
- Modify: `data/authored/laws.json`, `data/authored/standings.json`

**Interfaces:**
- Consumes: `map_canon::law::{Derivation, DressClass, EdgeStyle, FillRoute}` (Task 3), `dispose` (Task 4).
- Produces:
```rust
/// Provenance gains the witness's DERIVATION — spec §2's `provenance
/// (BorderText | CityDerived | Traced | Vendored)`, which is
/// `surveys::Grade` promoted into one queryable place.
pub struct Provenance {
    pub witness: Witness,
    pub verses: Vec<String>,
    pub note: String,
    pub derivation: law::Derivation,
}

/// `Area` LOSES `tenure`. Tenure is dispose's answer, not a stored
/// field: two homes for one fact is the disease this refactor cures.
pub struct Area { pub entity: EntityId, pub name: String, pub rings: BTreeSet<BorderId>, pub holes: BTreeSet<BorderId> }

pub fn derivation_of(store: &CanonStore, entity: &EntityId) -> Option<law::Derivation>;
```
Task 9 consumes `dispose(...).dress`; Task 11 serves it.

- [ ] **Step 1: Write the failing tests**

Append to `crates/map-adapters/src/tests.rs`:

```rust
/// The equality, written while both sides exist (the Task 5 pattern).
#[test]
fn the_law_set_and_the_declarations_reproduce_every_holds_and_grade_literal() {
    let laws = shipped_laws();
    let ledger = crate::authored_standings();
    for (tag, entity, holds_is_ground, grade) in crate::legacy_holds_and_grade_table() {
        // GRADE -> DERIVATION -> the edge the dress law gives.
        let want_edge = match grade {
            "border-text" => map_canon::EdgeStyle::Line,
            "city-derived" => map_canon::EdgeStyle::Unknown,
            other => panic!("unknown grade {other}"),
        };
        let sel = map_canon::Selector {
            layer: Some(map_canon::LayerKind::ScriptureClaims),
            kind: Some(map_canon::FactKind::Area),
            derivation: Some(derivation_from_grade(grade)),
            entity: Some(map_canon::EntityId(entity.to_string())),
        };
        assert_eq!(laws.dress_for(&sel).0.edge, want_edge, "{tag}: edge character");

        // HOLDS -> tenure. The default is Claimed; Ground is declared,
        // WITH ITS REASON, and the reason must be non-empty or the
        // law "a survey that holds ground says why" is words only.
        let (tenure, id) = laws.tenure_for(&sel);
        if holds_is_ground {
            assert_eq!(tenure, map_canon::Tenure::Held, "{tag}: declared ground holds");
            let rule = laws.tenure.iter().find(|r| r.id == id).unwrap();
            assert!(!rule.because.trim().is_empty(), "{tag}: ground without a written reason");
        } else {
            assert_eq!(tenure, map_canon::Tenure::Claimed, "{tag}: everything else is a claim");
        }
        let _ = &ledger;
    }
}

/// DISCRIMINATION: the two axes are ORTHOGONAL, and the test above
/// must be able to tell them apart. NUM34 is border-text AND a claim;
/// NT-JUDAEA is city-derived AND ground. If a single axis drove both,
/// one of these two would be wrong.
#[test]
fn grade_and_holds_stay_orthogonal() {
    let laws = shipped_laws();
    let num34 = selector_for("authored:the-land-promised-num-34", map_canon::Derivation::BorderText);
    let judaea = selector_for("authored:judea", map_canon::Derivation::CityDerived);
    assert_eq!(laws.dress_for(&num34).0.edge, map_canon::EdgeStyle::Line);
    assert_eq!(laws.tenure_for(&num34).0, map_canon::Tenure::Claimed);
    assert_eq!(laws.dress_for(&judaea).0.edge, map_canon::EdgeStyle::Unknown);
    assert_eq!(laws.tenure_for(&judaea).0, map_canon::Tenure::Held);
}
```

Append to `crates/map-canon/src/tests.rs`:

```rust
#[test]
fn tenure_has_exactly_one_home_and_it_is_dispose() {
    // `Area` no longer carries a tenure field. This test does not
    // assert a value -- it asserts the SHAPE, and it exists so that a
    // future re-introduction of the field fails the build here with a
    // name attached rather than quietly creating a second home.
    let a = Area {
        entity: EntityId("x".into()), name: "X".into(),
        rings: Default::default(), holes: Default::default(),
    };
    let _ = a;   // if this compiles, `tenure` is gone
}

#[test]
fn the_derivation_survives_persistence_and_reaches_dispose() {
    let mut store = CanonStore::default();
    let fid = area(&mut store, "authored:judea", square(31.7, 35.2, 0.4));
    store.set_provenance(fid, Provenance {
        witness: Witness::Authored, verses: Vec::new(), note: "n".into(),
        derivation: law::Derivation::CityDerived,
    });
    let back = crate::persist::from_bytes(&crate::persist::to_bytes(&store).unwrap()).unwrap();
    assert_eq!(
        derivation_of(&back, &EntityId("authored:judea".into())),
        Some(law::Derivation::CityDerived),
        "the derivation crosses the compile->serve boundary"
    );
    // DISCRIMINATION: a different derivation must come back different.
    assert_ne!(
        derivation_of(&back, &EntityId("authored:judea".into())),
        Some(law::Derivation::BorderText)
    );
}
```

- [ ] **Step 2: Run and verify FAIL** — `cargo test -p map-canon derivation 2>&1 | tail -20`. Expected: `Provenance` has no field `derivation`.

- [ ] **Step 3: Put `Derivation` on the witness and carry it everywhere**

- `Provenance` gains `derivation: law::Derivation` — **not** an `Option`. Every producer states how its shape is known, because "unknown" is a real answer and it has a name: `Vendored`. The compiler will name every construction site; there are producers in `timeline_bridge.rs:123-130`, `partition_bridge.rs:493`, `:606-613`, `:677-685`, `:695-703`, `main.rs:219-226`, `compile.rs`, and the test files. Assign each from what it actually is:
  | site | derivation | why |
  |---|---|---|
  | `timeline_bridge.rs:123-130` (bridged regions) | the survey's promoted `Grade`, threaded through `Boundary::character` | it IS the grade |
  | `partition_bridge.rs:606-613` (partition faces) | `Traced` | the plate trace and the OSM/NE geometry, noded into the arrangement |
  | `partition_bridge.rs:677-703` (settlements, memories) | `Vendored` | the atlas gazetteer's own coordinates |
  | `main.rs:219-226` (kept authored routes) | `CityDerived` | stand-in station coordinates through named places |
  | `compile.rs` (atlas narratives) | `Vendored` | the atlas placed them |
- `derivation_of(store, entity)` looks up the provenance of any feature carrying that entity and returns its derivation. Two features of one entity with different derivations is possible in principle; return the **minimum** by `Ord` and add a `CanonViolation::DerivationConflict { entity, a, b }` so a real one is reported rather than silently resolved. A silent tie-break is a special case.
- Persistence: `"derivation"` on each provenance row, with an absent key loading as `Vendored` for a pre-Stage-2 canon.

- [ ] **Step 4: Write the tenure and dress rules, and delete `Area.tenure`**

Append to `data/authored/laws.json` (after each family's default, so they override it):

```json
    {
      "id": "tenure/authored-claim",
      "when": { "layer": "scripture-claims", "kind": "area", "derivation": "city-derived" },
      "then": "claimed",
      "because": "A city-derived authored survey bounds a promise, a vision, or a homeland hull: boundary and name, never ground. surveys.rs decided this with `Holds::Claim` and timeline_bridge.rs:114-121 carried it as RegionClass::Claim. The promise's unpossessed remainder once painted itself as a phantom state."
    },
    {
      "id": "tenure/authored-claim-border-text",
      "when": { "layer": "scripture-claims", "kind": "area", "derivation": "border-text" },
      "then": "claimed",
      "because": "A border walked verse by verse may still bound only a promise — NUM 34 recites its circuit and bounds no possession. Grade and tenure are orthogonal axes and this row is where that stops being a comment."
    },
    {
      "id": "tenure/tetrarchy-judea",
      "when": { "entity": "authored:judea" },
      "then": "held",
      "because": "the tetrarchies were governed districts of the Roman order (LUK 3:1); the hull approximates a real administration, and the dashed boundary discloses the approximation"
    }
```
plus the same row for `authored:galilee`, `authored:perea` and `authored:iturea-and-trachonitis`, each carrying `Holds::Ground`'s justification **word for word** from `surveys.rs:935-938`. And the dress rules:

```json
    {
      "id": "dress/city-derived-is-unknown",
      "when": { "derivation": "city-derived" },
      "then": { "edge": "unknown", "fill": "region" },
      "because": "The text names the cities; the hull between them is disclosed interpolation. Honesty renders: a walked border is a Line, a city-derived hull is Unknown and the styles draw it distinctly (law 6). surveys.rs:1343-1346."
    },
    {
      "id": "dress/background-is-unknown",
      "when": { "layer": "background" },
      "then": { "edge": "unknown", "fill": "palette" },
      "because": "Background scholarship draws dashed: it is another map's reading of the ground, disclosed as such. canon_provider.rs:372-373."
    },
    {
      "id": "dress/water",
      "when": { "layer": "water" },
      "then": { "edge": "line", "fill": "water" },
      "because": "canon_provider.rs:289 — water speaks in the style's own water paint."
    },
    {
      "id": "dress/relief",
      "when": { "layer": "relief" },
      "then": { "edge": "line", "fill": "relief-ramp" },
      "because": "canon_provider.rs:290-298 — bands tint along the topo ramp by MEASURED area order; no name parsing, no hashing."
    }
```

> **The `tenure/claimed-keeps-its-shape` rule is NOT written here.** The Claimed-area transparency at `canon_provider.rs:427-430` is a *consequence* of tenure, not a separate knob, and it stays in the provider reading `dispose(...).tenure`. Adding a law row for it would create two homes for one decision — the exact disease.

Then delete `Area.tenure` (`lib.rs:74`), the `Tenure::Claimed` branch in `timeline_bridge.rs:114-121`, the `tenure: Tenure::Held` literal at `partition_bridge.rs:605`, the persistence field (`persist.rs:104`, `:234-236`), and the Task 4→6 seam in `census`. `canon_provider.rs:368` and `:427` read `dispose(...).tenure` instead of `a.tenure`. The compiler names every remaining site.

- [ ] **Step 5: Delete `Holds` and `Grade`, and shrink `SurveySpec` to circuit evidence**

`SurveySpec` (`surveys.rs:798-817`) keeps `tag, label, note, book, chapter, verse_from, verse_to, year, circuit` — **the circuit evidence, and nothing that disposes anything.** Delete `enum Holds`, `enum Grade`, the `holds`/`grade` fields, their 50 values, the `legacy_holds_and_grade_table()` helper and the two tests that used it. Update `surveys.rs`'s module-level comment to say what the file is now: the verses, their traditional dates, and the circuits Scripture gives.

- [ ] **Step 6: Rebuild, prove nothing moved, and check the twenty**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
cargo run --release -p map-compile build
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
```bash
cargo test --workspace 2>&1 | grep "test result: FAILED" ; echo "---"
cargo test -p map-adapters the_scripture_timeline_fingerprint_is_pinned 2>&1 | grep "test result"
cargo test -p map-canon the_blessed_censuses 2>&1 | grep "test result"
curl -s "http://127.0.0.1:8090/api/census?year=-1405" | python -c "
import json,sys,collections
r=json.load(sys.stdin)
c=collections.Counter((x['layer'],x['kind'],x['tenure']) for x in r)
for k,v in sorted(c.items()): print(k,v)"
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
```
Expected, exactly: `('scripture-claims','area','claimed') 1` and `('scripture-claims','area','held') 20`. **The twenty `held` partition faces must still be `held`.** If they flipped to `claimed`, a layer-keyed rule crept into the law set — find it and key it on `derivation: "traced"` instead. That is this task's one recurring failure mode, and it is the whole reason `Derivation` exists.

- [ ] **Step 7: Commit**

```bash
git add crates/map-adapters crates/map-canon crates/map-compile crates/map-provider data/authored
git commit -m "Holds and Grade die: tenure by declaration, dress by law, Derivation on the witness; surveys.rs is circuit evidence"
```

---

### Task 7: `stands_until` strings become ledger rows, and `PresenceBook` retires

`crates/map-compile/src/partition_bridge.rs:496-585` is the last place that builds standings, and it builds them into **two** `PresenceBook`s from **four** key namespaces:

| lines | source | key | count |
|---|---|---|---|
| `:524-535` | `data/wikimedia/tribes12.geojson` `stands_from`/`stands_until` (era ids) | tribal cohort slug | 13 |
| `:541-543` | a Rust literal: `presence.declare("canaan", t0, Some(resolve_era("united-kingdom")?))` | `"canaan"` | 1 |
| `:544-550` | `data/openbible/regions.geojson` `stands_until` (era ids) | region slug | 6 |
| `:555-561` | vendored atlas polity rows | `"{id}@{from_year}"` | all polity eras |
| `:562-564` | the same rows again, into `entity_disjoint` | `row.id` | (a law, not data) |

This task moves all five into the one `Ledger` and deletes `PresenceBook`. **It is the highest-risk task in the plan**, because `presence.eras(t0)` (`:585`) drives `bundle_faces(store, &part, &era.absent)` (`:588`), which decides *which claimant names each face* — so one changed era cut renames faces, and renamed faces are moved census rows and moved pixels.

Task 2 already proved `Ledger::eras == PresenceBook::eras` on synthetic rows. This task proves it on the **real** ones, before deleting anything.

**Files:**
- Modify: `crates/map-compile/src/partition_bridge.rs`, `crates/map-compile/src/tests.rs`
- Modify: `crates/map-canon/src/lib.rs` (delete `PresenceBook`, `PresenceError`, `Era`'s old home if it moves), `crates/map-canon/src/tests.rs`
- Modify: `data/authored/standings.json` (the one Rust literal — `"canaan"` — becomes a declared row)

**Interfaces:**
- Consumes: `map_canon::ledger::{Ledger, Standing, WitnessKey, Endurance}`, Stage 1's `Registry::resolve`.
- Produces: `bridge_partition` builds and returns a populated `Ledger`; `store.set_ledger` carries it. `PresenceBook` no longer exists.

**On Stage 1's registry, and why the entity field is not the key.** Stage 1's closing Interfaces promise `Registry::resolve` is total, idempotent and one-hop, and that ledger rows "key on `resolve`'d ids". Each `Standing` therefore carries `entity: registry.resolve(minted).clone()` — so `standings_of_entity("phoenicia")` finds every witness's standing for the unified node. But the `WitnessKey` is **not** resolved: `"egypt@-1500"` and `"egypt@-1200"` are two standings of one entity and must stay two, or the atlas polity era-variants collapse and the empire stops morphing. Task 2's `EntityStandsTwice` law is what keeps that honest — it will now run over the real data for the first time.

- [ ] **Step 1: Write the failing test that pins the real era cuts**

Append to `crates/map-compile/src/tests.rs`:

```rust
/// THE RISKIEST EQUALITY IN THE STAGE. `presence.eras(t0)` decides
/// which claimant names each partition face; a single changed cut
/// renames faces, and renamed faces are moved census rows and moved
/// pixels. Build the standings BOTH ways from the REAL data and
/// demand the same eras, before PresenceBook is deleted.
#[test]
fn the_real_era_cuts_are_identical_under_the_ledger() {
    let polities = load_test_polities();          // this file's existing helper
    let resolve_era = test_era_resolver();        // this file's existing helper
    let t0 = ts(-4004);

    let book = crate::partition_bridge::legacy_presence_book(&resolve_era, &polities)
        .expect("the presence book as it was");
    let ledger = crate::partition_bridge::build_ledger(&resolve_era, &polities, &registry())
        .expect("the ledger");

    let a = book.eras(t0);
    let b = ledger.eras(t0);
    assert_eq!(a.len(), b.len(), "the same number of eras");
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!(x.from, y.from, "era from");
        assert_eq!(x.until, y.until, "era until");
        let xa: std::collections::BTreeSet<String> = x.absent.iter().cloned().collect();
        let ya: std::collections::BTreeSet<String> =
            y.absent.iter().map(|w| w.0.clone()).collect();
        assert_eq!(xa, ya, "the absent set at {:?}", x.from);
    }
    assert!(a.len() > 1, "there really are multiple eras -- this comparison has content");
}

/// DISCRIMINATION: the comparison above must be able to fail. Drop one
/// real standing and prove the eras diverge.
#[test]
fn a_dropped_standing_changes_the_era_cuts() {
    let polities = load_test_polities();
    let resolve_era = test_era_resolver();
    let full = crate::partition_bridge::build_ledger(&resolve_era, &polities, &registry()).unwrap();
    let thinned =
        crate::partition_bridge::build_ledger(&resolve_era, &polities[1..], &registry()).unwrap();
    assert_ne!(full.eras(ts(-4004)), thinned.eras(ts(-4004)),
               "removing a polity era must change the cut set");
}

/// The entity-disjointness law, run over the REAL polity rows for the
/// first time. `entity_disjoint` enforced it as a second book and
/// failed the compile; the ledger enforces it as a law of the one
/// book, and it must find exactly as many violations: none.
#[test]
fn the_real_ledger_is_lawful() {
    let ledger = crate::partition_bridge::build_ledger(
        &test_era_resolver(), &load_test_polities(), &registry()).unwrap();
    assert_eq!(ledger.validate(), Vec::new(), "no entity stands under two witnesses at once");
}
```

Adaptation note: `legacy_presence_book` is `bridge_partition`'s existing standings-building block (`:521-578`) extracted verbatim into a `pub(crate)` function — extract it in this step, change nothing about it, and delete it in Step 4. `registry()` is a helper returning the compiled `Registry` (Stage 1 Task 14 persists it); if the test harness has no compiled canon, `Registry::default()` is honest here, because `resolve` is total and an empty registry resolves every id to itself — which is precisely today's behaviour and therefore the right baseline for an equality test.

- [ ] **Step 2: Run and verify FAIL** — `cargo test -p map-compile era_cuts 2>&1 | tail -20`. Expected: `cannot find function build_ledger`.

- [ ] **Step 3: Declare the one Rust literal, and write `build_ledger`**

The `"canaan"` standing at `:541-543` is a Rust literal declaring a policy in prose comments (`:536-540`). It becomes a row in `data/authored/standings.json`:

```json
    {
      "witness": "canaan",
      "entity": "partition:canaan",
      "from": -4004,
      "until_era": "united-kingdom",
      "note": "Canaan the named territory stands from the frame's dawn until the monarchy rises — from there the land is Israel's story (Territory carries it), and the plate frame no longer names the ground."
    }
```

`until_era` is a second, era-id-shaped form of `until` — it exists because the era table is vendored and its years move when the atlas re-dates an era, and hardcoding the resolved year here would silently freeze a date the atlas owns. `Ledger::parse` does not resolve it (map-canon knows nothing of eras); it returns the row with `until: None` and an `until_era: Option<String>` that **the compiler must resolve before declaring**. Give `Ledger::parse` a sibling for exactly this:
```rust
/// Rows whose end is named by an ERA rather than a year, returned
/// unresolved. The caller holds the era table and must resolve each
/// one before `declare`; a row left unresolved is a row that would
/// stand forever without a written reason, which `declare` refuses —
/// so forgetting is a loud error, not a silent endurance.
pub fn pending_era_ends(json: &str) -> Result<Vec<(WitnessKey, EntityId, Timestamp, String)>, String>;
```

`build_ledger(resolve_era, polities, registry) -> Result<Ledger, String>` then declares, in this order (order does not affect the result — `declare` is commutative over disjoint keys — but a fixed order makes the compile report reproducible):

1. the authored rows from `standings.json`, era-ends resolved;
2. the 13 tribal cohorts from `load_tribal_rings()` (`:524-535`, unchanged logic, `WitnessKey(cohort.slug)`);
3. the 6 openbible regions from `load_openbible_regions()` (`:544-550`, `WitnessKey(slug)`);
4. the polity eras (`:555-561`, `WitnessKey(format!("{}@{}", row.id, row.from_year))`).

Every row's `entity` is `registry.resolve(&minted).clone()`. Every row's `endurance` is `Bounded` — **every one of these four families has an `until`**, which is why none of them needed an ENDURES mark before. A row that arrives with no `until` from any of these sources is a data error and `declare` will refuse it by name (`EndlessWithoutAReason`); let it.

- [ ] **Step 4: Rewire `bridge_partition` and delete `PresenceBook`**

- `bridge_partition` takes the ledger (built by `main.rs` before the call, so `main.rs` can `store.set_ledger` before `validate_frame_edge` runs) and replaces `:585`'s `presence.eras(t0)` with `ledger.eras(t0)`.
- `bundle_faces(store, &part, &era.absent)` now receives a `BTreeSet<WitnessKey>`; change its signature and its lookups. Do **not** convert to `BTreeSet<String>` at the call site — the whole point of the type is that a face's claimant key and an entity id can no longer be confused, and a conversion at the boundary throws that away.
- Delete `entity_disjoint` (`:554`, `:562-564`); its law now lives in `Ledger::validate`, and `main.rs` reports its violations beside the canon's. The error text must stay at least as informative: today it says `polity '{id}': eras overlap in time`, so the new report must name the entity *and* both witnesses.
- Delete `PresenceBook`, `PresenceError`, and their tests from `crates/map-canon` (`lib.rs:187-274` and the `presence_*` tests around `tests.rs:381`). Move the `presence_defaults_to_always_standing` test's *law* — an undeclared claimant stands always — into the ledger's own test if Task 2 did not already state it; it did (`the_ledger_is_total_...`, first assertion), so the old test is deleted rather than ported.
- `Era` keeps its home in `map-canon` but its `absent` field becomes `BTreeSet<WitnessKey>`.
- Delete `legacy_presence_book` and the two tests that used it, keeping `the_real_ledger_is_lawful`.

- [ ] **Step 5: Rebuild and prove nothing moved**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
cargo run --release -p map-compile build
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
```bash
cargo test --workspace 2>&1 | grep -c "test result: ok"
cargo test -p map-canon the_blessed_censuses 2>&1 | grep "test result"
make contract-gates
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
```

> **The compile report is evidence here, not noise.** `cargo run --release -p map-compile build` prints a partition summary line (`partition_bridge.rs:717-721`): *"partition: N faces, M river paths, 4π residual R; C cities stand, D remembered beneath the waters"*. **Record N, M, C and D before this task and compare after.** A changed face count is a changed arrangement, and it will reach the census. Comparing the summary line takes ten seconds and localises a failure that would otherwise surface as an unexplained census diff.

- [ ] **Step 6: Commit**

```bash
git add crates/map-canon crates/map-compile data/authored/standings.json
git commit -m "stands_until becomes ledger rows: one book keyed by entity and witness; PresenceBook retires"
```

---

### Task 8: Shadow spans become a supersession law

`crates/map-compile/src/main.rs:394-428` computes `bg_shadows` and threads it into `bridge_filtered`, which consumes it at `timeline_bridge.rs:143-152` (shadow edges are moments) and `:180-186` (a shadowed row is omitted from that moment's snapshot). Stage 1 killed the *slug matcher* that built the map; Stage 2 kills the *map*. Spec §2's "what dies" names both: `bg_shadows` slug matching (Stage 1) and per-pipeline precedence (here).

A shadow is not a data structure. It is a **supersession rule plus a standing**: *for as long as an atlas Territory witness stands for entity E, the Basemap witness for E yields.* The spans were that sentence, precomputed per pipeline, in a `BTreeMap<String, Vec<(i32,i32)>>` that only one bridge could read.

**Files:**
- Modify: `crates/map-compile/src/main.rs`, `crates/map-compile/src/timeline_bridge.rs`, `crates/map-compile/src/tests.rs`
- Modify: `crates/map-canon/src/law.rs` (`supersessions_for` gains its first real caller), `crates/map-canon/src/tests.rs`
- Modify: `data/authored/laws.json`

**Interfaces:**
- Consumes: `Laws::supersessions_for` (Task 3), `Ledger` (Task 7), `Registry::resolve` (Stage 1).
- Produces:
```rust
/// Is this witness's contribution superseded at `at`? True iff some
/// supersession rule names it as the loser AND the winning witness
/// for the SAME entity stands at `at`. Total: no rule, no supersession.
pub fn superseded(
    entity: &EntityId, layer: LayerKind, witness: Witness,
    at: &Timestamp, ledger: &Ledger, laws: &Laws,
) -> Option<LawId>;
```
`bridge_filtered` loses its `shadow_spans` parameter entirely; its four `&BTreeMap::new()` call sites lose an argument.

- [ ] **Step 1: Write the failing test that pins the current shadow set**

Append to `crates/map-compile/src/tests.rs`:

```rust
/// THE EQUALITY: for every (background entity, year) the old span map
/// shadowed, the supersession law must agree -- and for every one it
/// did NOT shadow, the law must agree too. Both directions, because a
/// rule that shadows everything would pass a one-directional test.
#[test]
fn the_supersession_law_reproduces_the_shadow_spans_exactly() {
    let polities = load_test_polities();
    let legacy = crate::legacy_bg_shadows(&polities);        // extracted, unchanged
    let laws = shipped_laws();
    let ledger = crate::partition_bridge::build_ledger(
        &test_era_resolver(), &polities, &registry()).unwrap();

    let mut agreed = 0usize;
    let mut shadowed = 0usize;
    for (slug, spans) in &legacy {
        let entity = map_canon::EntityId(format!("basemap:{slug}"));
        // Probe every year the spans touch, plus the year on either
        // side of every edge: an off-by-one at a span boundary is the
        // failure mode, and only edge-adjacent probes can catch it.
        let mut years: std::collections::BTreeSet<i32> = Default::default();
        for (a, b) in spans {
            for y in [*a - 1, *a, *a + 1, *b - 1, *b, *b + 1] {
                if y != 0 { years.insert(y); }
            }
        }
        for y in years {
            let want = spans.iter().any(|(a, b)| *a <= y && y <= *b);
            let got = map_canon::superseded(
                &entity, map_canon::LayerKind::Background, map_canon::Witness::Basemap,
                &ts(y), &ledger, &laws,
            )
            .is_some();
            assert_eq!(got, want, "{slug} at {y}: law says {got}, spans said {want}");
            agreed += 1;
            if want { shadowed += 1; }
        }
    }
    assert!(agreed > 0, "the comparison has content");
    assert!(shadowed > 0, "and some of it is actually shadowed -- otherwise this test \
                           passes against a law that never supersedes anything");
}

/// DISCRIMINATION: an empty supersession family must FAIL the test
/// above, or the law is not being read.
#[test]
fn without_the_rule_nothing_is_superseded() {
    let mut laws = shipped_laws();
    laws.supersession.clear();
    let ledger = crate::partition_bridge::build_ledger(
        &test_era_resolver(), &load_test_polities(), &registry()).unwrap();
    let any = load_test_polities().iter().any(|p| {
        map_canon::superseded(
            &map_canon::EntityId(format!("basemap:{}", p.id)),
            map_canon::LayerKind::Background, map_canon::Witness::Basemap,
            &ts(p.from_year), &ledger, &laws,
        )
        .is_some()
    });
    assert!(!any, "with no rule, nothing yields");
}
```

Adaptation note: `legacy_bg_shadows(&polities)` is `main.rs:394-413`'s slugify-and-collect block extracted verbatim as a `pub(crate)` function in `map-compile`'s lib, so a test can call it. **Stage 1 Task 14 has already replaced its slug matching with registry-declared shadows** — extract whatever is there *at the time you run this task*, not what this plan quotes, and say in the commit message which you found. Both die in Step 4 either way.

- [ ] **Step 2: Run and verify FAIL** — `cargo test -p map-compile supersession 2>&1 | tail -20`. Expected: `cannot find function superseded`.

- [ ] **Step 3: Write the rule**

Append to `data/authored/laws.json`'s `supersession` family:

```json
    {
      "id": "supersession/territory-over-background",
      "winner": { "layer": "territory" },
      "loser": { "layer": "background", "kind": "area" },
      "because": "The same entity must not wear two witnesses at once. While an atlas Territory era stands for an entity, the historical-basemaps reading of that entity yields — for the duration of the era, and never outside it: a shadow is never a hole, and the background region still exists on either side. This was `bg_shadows`, a BTreeMap<String, Vec<(i32,i32)>> that only one bridge could read (map-compile/src/main.rs:408-413); it is one row with global reach now."
    }
```

and implement `superseded` in `crates/map-canon/src/law.rs`:

```rust
/// Total. `None` means "nothing supersedes this", which is the answer
/// for every entity no rule names — so a caller never handles an
/// absence it cannot act on.
pub fn superseded(
    entity: &EntityId, layer: LayerKind, witness: Witness,
    at: &Timestamp, ledger: &ledger::Ledger, laws: &Laws,
) -> Option<LawId> {
    let me = Selector { layer: Some(layer), entity: Some(entity.clone()), ..Default::default() };
    let _ = witness;
    for rule in laws.supersessions_for(&me) {
        // Does the WINNING witness stand for THIS ENTITY at `at`? The
        // ledger is keyed by witness, so ask for every standing of the
        // entity and test the ones the winner's selector matches.
        let stands = ledger.standings_of_entity(entity).into_iter().any(|s| {
            rule.winner.matches(&Selector {
                layer: Some(s.layer_hint()), entity: Some(entity.clone()), ..Default::default()
            }) && s.covers(at)
        });
        if stands {
            return Some(rule.id.clone());
        }
    }
    None
}
```

Adaptation note: `Standing` has no layer today, and `layer_hint()` above is a placeholder for a decision you must make explicitly rather than inherit. The honest options, in order of preference:
1. **Give `Standing` a `layer: LayerKind` field.** A standing IS a witness's standing, and a witness has a layer (Stage 1's `WitnessRef.layer`). This is the smallest true statement, it makes the rule's `winner` selector mean what it reads, and it costs one field in `ledger.rs`, `persist.rs` and `build_ledger`. **Take this one unless it turns out to be wrong.**
2. Match the winner on `WitnessKey` shape. Rejected: that is slug matching wearing a law's clothes, and Stage 1 just killed its predecessor.

If you take option 1, delete `layer_hint()` and write `Some(s.layer)`. Update Task 7's `build_ledger` to set it: authored rows and cohorts are `ScriptureClaims`, openbible regions `ScriptureClaims`, polity eras `Territory`, the two family rows `ScriptureClaims`.

- [ ] **Step 4: Delete the span map**

- `main.rs`: delete `slugify`, `bg_shadows`, and whatever Stage 1 left in its place; the report line becomes a count of supersession rules and the entities they touched, computed from the law and the ledger.
- `timeline_bridge.rs`: delete the `shadow_spans` parameter, the shadow-edge sweep (`:143-152`), and the `shadowed` test (`:180-186`). In their place, `bridge_filtered` takes `&Ledger` and `&Laws` and:
  - the **edge sweep** gains every ledger cut that touches an entity this bridge is placing — the shadow edges were exactly "the year an atlas era ends", and the ledger already knows every one of them (`Ledger::eras`);
  - the **membership test** becomes `superseded(&entity, layer, witness, &edge, ledger, laws).is_none()`.
- `bridge_timeline_regions` (`:33-41`) loses its `&BTreeMap::new()` and gains the two references. Its four other call sites in `main.rs` follow.

- [ ] **Step 5: Rebuild, and read the report line**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
cargo run --release -p map-compile build 2>&1 | tee compile-after-task8.log
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
```bash
grep -i "background\|shadow\|supersed" compile-after-task8.log
cargo test -p map-canon the_blessed_censuses 2>&1 | grep "test result"
make contract-gates
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
rm -f compile-after-task8.log
```
Expected: the census at all three instants unmoved, `ALL GOLDEN VIEWS HOLD`. The live census carries **63 `background` areas at −1405**; that count is the shadow set's fingerprint and it must not change.

- [ ] **Step 6: Commit**

```bash
git add crates/map-canon crates/map-compile data/authored/laws.json
git commit -m "shadow spans become a supersession law: one row, global reach, replacing a per-pipeline span map"
```

---

### Task 9: Per-pipeline precedence dies — `paint_rank` becomes `dispose(...).rank`

`crates/map-provider/src/canon_provider.rs:559-572` is seven tuned constants in a `match layer`, with a five-line comment explaining a bug they once caused (*"Background once ranked beneath Relief, and the opaque bands entombed the whole non-Biblical world"*) — which is the strongest possible argument that these belong in reviewable data. `:678`'s `.unwrap_or(2)` is an eighth, undeclared. The comment at `:558` names the reason it lives there: *"Recorded at push time because the scene type carries no layer"* — Stage 1 fixed that half by making pieces first-class; this task fixes the other half.

Spec §3 law 2 is the reason this matters beyond tidiness: *"paint order comes from the precedence LAW, never composition order."* Until now there has been no precedence law to come from.

**Files:**
- Modify: `crates/map-provider/src/canon_provider.rs`, `crates/map-provider/src/tests.rs`
- Modify: `data/authored/laws.json`

**Interfaces:**
- Consumes: `dispose(...).rank` (Task 4), the `precedence` family (Task 3).
- Produces: no new names. `CanonProvider` gains a `laws: Laws` field, loaded beside the canon.

- [ ] **Step 1: Write the failing test**

Append to `crates/map-provider/src/tests.rs`:

```rust
/// The six layer ranks and the declared default, pinned as a WHOLE
/// table. Not a spot check: every layer, and the fall-through, in one
/// assertion, so a rule that went missing cannot hide behind the five
/// that stayed.
#[test]
fn the_precedence_law_is_the_paint_order_whole() {
    let laws = shipped_laws();
    let want = [
        (map_canon::LayerKind::Relief, 0u8),
        (map_canon::LayerKind::Background, 1),
        (map_canon::LayerKind::ScriptureClaims, 2),
        (map_canon::LayerKind::Territory, 3),
        (map_canon::LayerKind::Water, 4),
        (map_canon::LayerKind::Journeys, 5),
    ];
    let got: Vec<_> = want
        .iter()
        .map(|(l, _)| {
            let d = map_canon::dispose(
                None, *l, map_canon::FactKind::Area, None, None, &laws);
            (*l, d.rank)
        })
        .collect();
    assert_eq!(got, want.to_vec(), "the whole paint order, by law");

    // THE DECLARED DEFAULT, and its citation. `.unwrap_or(2)` was
    // true and unwritten; it is now true and written, and it names
    // itself when it fires.
    let d = map_canon::dispose(None, map_canon::LayerKind::Relief, map_canon::FactKind::Area,
                               None, None, &laws);
    assert!(d.citations.iter().any(|c| c.0.starts_with("rank/")),
            "the rank answer cites a rule, got {:?}", d.citations);
}

/// DISCRIMINATION, and the bug the comment remembers. Relief must
/// rank BELOW Background: when it did not, the opaque bands entombed
/// the whole non-Biblical world. Assert the ordering, not just the
/// numbers, so a uniform renumbering that preserved the bug would
/// still fail.
#[test]
fn relief_stays_beneath_background_and_water_stays_above_every_claim() {
    let laws = shipped_laws();
    let rank = |l| map_canon::dispose(None, l, map_canon::FactKind::Area, None, None, &laws).rank;
    assert!(rank(map_canon::LayerKind::Relief) < rank(map_canon::LayerKind::Background),
            "Berbers, Saami and Ainu stood as labels on bare land the last time this was wrong");
    assert!(rank(map_canon::LayerKind::Water) > rank(map_canon::LayerKind::Territory),
            "a lake is never buried");
    assert!(rank(map_canon::LayerKind::ScriptureClaims) < rank(map_canon::LayerKind::Territory),
            "when both speak at once the kingdom paints over the frame it rose from");
}
```

- [ ] **Step 2: Run and verify FAIL** — `cargo test -p map-provider precedence 2>&1 | tail -20`. Expected: the ranks all come back as the declared default `2`, because the six rules do not exist yet. That is the right failure: the default is working and the specifics are missing.

- [ ] **Step 3: Write the six rules**

Append to `data/authored/laws.json`'s `precedence` family, after `rank/default`, carrying `canon_provider.rs:550-572`'s own reasoning across verbatim — the comment is the justification, and it should not be paraphrased into something weaker:

```json
    {
      "id": "rank/relief",
      "when": { "layer": "relief" },
      "rank": 0,
      "because": "RELIEF is the stage under everything — it is the ground itself, not a claim — and every named region paints over it, background scholarship included. When Background once ranked beneath Relief, the opaque bands entombed the whole non-Biblical world: Berbers, Saami and Ainu stood as labels on bare land, and in bible mode even the ghost disclosure was buried."
    },
    {
      "id": "rank/background",
      "when": { "layer": "background" },
      "rank": 1,
      "because": "Another map's reading of the ground: above the stage, beneath every claim this map makes for itself."
    },
    {
      "id": "rank/scripture-claims",
      "when": { "layer": "scripture-claims" },
      "rank": 2,
      "because": "The scripture-frame (Canaan, the allotments, the nations) lies beneath the POLITICAL layer: eras hand off between them, and when both speak at once the kingdom paints over the frame it rose from."
    },
    {
      "id": "rank/territory",
      "when": { "layer": "territory" },
      "rank": 3,
      "because": "Governed ground paints over the frame it rose from."
    },
    {
      "id": "rank/water",
      "when": { "layer": "water" },
      "rank": 4,
      "because": "Water stays above every claim — a lake is never buried."
    },
    {
      "id": "rank/journeys",
      "when": { "layer": "journeys" },
      "rank": 5,
      "because": "A journey is walked across whatever it crosses."
    }
```

- [ ] **Step 4: Delete the constants**

In `canon_provider.rs`:
- give the provider a `laws: Laws` field, loaded once beside the canon (the viewer already loads it — pass the same value in rather than reading the file twice, so the provider and the census route can never disagree about the law set);
- replace `:561-572`'s `match layer` with `dispose(...).rank`;
- replace `:678`'s `.unwrap_or(2)` with the same call, so an unranked region gets the *declared* default and there is no second answer;
- while you are here, replace `:368-375`'s `character` match and `:287-311`'s `area_paint` layer match with `dispose(...).dress.edge` and `dispose(...).dress.fill` — they are the dress rules Task 6 wrote, and leaving them behind would mean the law set is authoritative for the census and decorative for the pixels.

> **`area_paint`'s palette-slot and relief-ramp lookups stay in the provider.** `FillRoute::Palette` says *which route*; `self.palette_slot` and `self.relief_pos` are measured, per-entity render state, not policy. The law decides the route; the provider walks it. Moving the measurement into the law set would put per-entity data in a global rule table — the opposite of "one row, global reach".
>
> Note also the `hash64(&entity.0) % 8` fallback at `:305`. That is a **tuned constant with an entity-dependent answer** and it is a real violation of the owner's law — but it is a *fallback for a missing measurement*, not a disposition, and fixing it is a change to palette assignment that would move pixels. **Record it for the owner in Task 13 Step 4; do not fix it here.**

- [ ] **Step 5: Rebuild and prove nothing moved**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
cargo run --release -p map-compile build
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
```bash
cargo test --workspace 2>&1 | grep "test result: FAILED"; echo "--- (empty above is the pass)"
make contract-gates
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api 2>&1 | tail -15
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
```

> **This is the task most likely to move a pixel**, because it is the only one that changes the *provider*. The golden gate is the judge and 89/89 is the only acceptable answer. If a stop drifts, the cause is almost certainly an ordering difference in the `sort_by_cached_key` at `:674-680` — the tuple is `(rank, Reverse(radius), pts, region_id)` and only `rank` may change hands. **STOP and report; do not re-bless.**

- [ ] **Step 6: Commit**

```bash
git add crates/map-provider data/authored/laws.json
git commit -m "per-pipeline precedence dies: paint order comes from the precedence law, the default declared"
```

---

## Part C — the wire

### Task 10: `standings` on the wire

Spec §3's function set: `standings(at | over) → [(entity, interval)]`. Two forms, because "who stands now" and "who stood across this span" are different questions and collapsing them into one would make the second a client-side fold over the first.

**Files:**
- Create: `contracts/map-api/fact/standings.feature`
- Create (blessed): `contracts/map-api/fixtures/standings-1405.json`, `standings-over-1446-1050.json`
- Modify: `contracts/runner/src/Steps.hs`, `contracts/runner/src/Capture.hs`
- Modify: `crates/map-viewer/src/lib.rs`, `crates/map-canon/src/tests.rs`

**Interfaces:**
- Consumes: `map_canon::ledger::Ledger` (Task 2), `Registry::resolve` (Stage 1).
- Produces:
```
GET /api/standings?at=YEAR            -> [{entity, witness, from, until, endurance}]
GET /api/standings?from=YEAR&to=YEAR  -> the same rows, for every standing intersecting [from, to)
```
Rows sorted by `(entity, witness, from)` for deterministic wire bytes. `until` is **absent** when the standing reaches the frame's edge — a `null` and a missing key would be two spellings of one fact. `endurance` is absent for `Bounded` and `{reason, source}` for `Endures`, for the same reason.

- [ ] **Step 1: Contract addition FIRST — the feature, runner red**

`contracts/map-api/fact/standings.feature`:

```gherkin
Feature: standings — who stands, and when
  Spec §2: a Standing is an entity × a right-open interval. This is the
  ledger on the wire: the rows that used to be `Stands::Until(y)`
  literals in Rust, `stands_until` strings in geojson properties, and a
  span map only one bridge could read. Every scenario pins a WHOLE
  answer; the interval laws are the scenarios, not the shapes.

  Vocabulary:
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |

  Scenario: everyone standing at the conquest, whole
    When I GET /api/standings?at=-1405
    Then the response equals fixture "standings-1405"

  Scenario: the standings across the conquest span, whole
    When I GET /api/standings?from=-1446&to=-1050
    Then the response equals fixture "standings-over-1446-1050"

  Scenario: a standing is RIGHT-OPEN — its until year does not stand
    When I GET /api/standings?at=-1446 as atFrom
    And I GET /api/standings?from=-1446&to=-1445 as overOne
    Then atFrom is the standings of overOne that contain -1446

  @property
  Scenario: standings are deterministic at any year
    When I GET /api/standings?at=<someYear> as first
    And I GET /api/standings?at=<someYear> as second
    Then first equals second

  @property
  Scenario: an instant's standings are exactly the span of that instant alone
    When I GET /api/standings?at=<someYear> as instant
    And I GET /api/standings?from=<someYear>&to=<someYear> as degenerate
    Then instant equals degenerate
```

The last scenario is the one worth arguing for: a right-open span `[y, y)` is empty, so a naive `over` implementation returns nothing while `at=y` returns everything standing — and the two would disagree forever without a law saying they must not. The correct reading, and the one the implementation must take: `from`/`to` is **inclusive of both endpoints as instants to test**, so `from=y&to=y` asks "who stood at y". Say that in the route's doc comment, because it is the kind of decision that gets re-litigated.

```bash
cd contracts/runner && cabal run contract-runner -- check ../map-api && cabal run contract-runner -- vocab ../map-api
```
Expected: ORPHAN rows naming `is the standings of ... that contain ...`. `vocab` must pass — the `year` table is real, so this feature is not one of the exempt three (diagnosis §9.2).

- [ ] **Step 2: Write the failing Rust test**

Append to `crates/map-canon/src/tests.rs`:

```rust
#[test]
fn standings_over_a_span_are_exactly_those_intersecting_it() {
    use crate::ledger::*;
    let mut l = Ledger::default();
    let row = |e: &str, from, until| Standing {
        entity: EntityId(e.into()), witness: WitnessKey(e.into()),
        from: ts(from), until: Some(ts(until)), endurance: Endurance::Bounded,
    };
    l.declare(row("early", -2000, -1500)).unwrap();     // wholly before
    l.declare(row("straddles", -1500, -1000)).unwrap(); // overlaps
    l.declare(row("inside", -1400, -1300)).unwrap();    // contained
    l.declare(row("late", -900, -800)).unwrap();        // wholly after
    // Touching at the open end is NOT intersecting: [-2000,-1500) and
    // [-1500,...) share no instant. That is the right-open law, and
    // it is the assertion most likely to be got wrong.
    let got: Vec<String> =
        l.standings_over(&ts(-1500), &ts(-1100)).into_iter().map(|s| s.entity.0.clone()).collect();
    assert_eq!(got, vec!["inside".to_string(), "straddles".to_string()],
               "sorted by entity; neither the early nor the late one intersects");

    // The degenerate span is the instant, and it agrees with `stands`.
    let at: Vec<String> =
        l.standings_over(&ts(-1400), &ts(-1400)).into_iter().map(|s| s.entity.0.clone()).collect();
    assert_eq!(at, vec!["inside".to_string(), "straddles".to_string()]);
    for e in &at {
        assert!(l.stands(&WitnessKey(e.clone()), &ts(-1400)), "{e} agrees with `stands`");
    }
    assert!(!l.stands(&WitnessKey("early".into()), &ts(-1400)));
}
```

- [ ] **Step 3: Implement `Ledger::standings_over` and the route**

`standings_over(&self, from: &Timestamp, to: &Timestamp) -> Vec<&Standing>` — every row whose `[from, until)` contains any instant in `[from, to]`, sorted by `(entity, witness, from)`. Write the intersection test as `row.from <= to && row.until.map_or(true, |u| u > *from)`, and put the right-open reasoning in a comment beside it, because `>` versus `>=` there is the whole law.

In `crates/map-viewer/src/lib.rs`, add the `"/api/standings"` arm beside `/api/census`, refusing a request that gives neither `at` nor a `from`/`to` pair with a named error (never a default year — a default is a guess about what the caller meant).

- [ ] **Step 4: Teach the runner the one new step**

`contracts/runner/src/Steps.hs`, following the file's existing ordering rule (specific before generic):
```haskell
  , mkStep Then (lit "" *> ((,) <$> capUntil @BindName " is the standings of "
                                <*> capUntil @BindName " that contain ")
                          <*> capRest @Year) $
      \((BindName a, BindName b), y) w -> pure $ do
        av <- lookupBound a w
        bv <- lookupBound b w
        -- WHOLE-BODY: filter the span's rows to those covering the
        -- year and demand the ENTIRE remaining array equals the
        -- instant's entire array. Not a subset, not a count.
        let want = filterCovering y bv
        if av == want then Right w
          else Left ("the instant's standings are not the span's rows covering "
                     <> renderCap y <> ": " <> maybe "(no leaf difference)" id (firstDiff want av))
```
Reuse Stage 1 Task 3's `firstDiff` on both sides; a comparison failure that does not name a path is a comparison the next reader cannot act on.

- [ ] **Step 5: Bless, eyeball, and run everything**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
```bash
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api --bless
```
**Then read both new fixtures before trusting them.** `standings-1405.json` must name real entities with real intervals — if it is `[]`, the ledger never reached the server and blessing it would pin that bug forever. `standings-over-1446-1050.json` must be a superset of it. **A blessed error body is a lie that passes forever.**

- [ ] **Step 6: Full gates and commit**

```bash
make contract-gates
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api 2>&1 | tail -15
```
```bash
git add crates/map-canon crates/map-viewer contracts/
git commit -m "standings on the wire: the ledger, queryable at an instant and over a span"
```

---

### Task 11: `disposition` on the wire, the census image law, and half of derivability

Three obligations close together because they are one step definition apart:

1. Spec §3: `disposition(entity, at) → {tenure, dress-class, rank, law-citations}`.
2. Task 4's census image law scenario, still orphaned.
3. Diagnosis §9.1 item 2 — derivability. Stage 1 wrote `contracts/map-api/fact/derivability.feature` as `@target`, failing honestly with *"derivability cannot be sampled: /api/disposition (Stage 2) and /api/borders (Stage 3) do not exist yet"*. Stage 2 provides one of the two. The `@target` stays, but its **disposition half becomes a real green scenario** and the failure message narrows to name only what is genuinely missing. A `@target` whose message overstates what is missing is a `@target` nobody will re-read.

**Files:**
- Create: `contracts/map-api/fact/disposition.feature`
- Create (blessed): `contracts/map-api/fixtures/disposition-judea-59.json`, `disposition-num34-1405.json`
- Modify: `contracts/map-api/fact/derivability.feature`, `contracts/map-api/fact/census.feature` (no text change; its scenario goes green)
- Modify: `contracts/runner/src/Steps.hs`
- Modify: `crates/map-viewer/src/lib.rs`

**Interfaces:**
- Consumes: `dispose` (Task 4), `Ledger` (Task 2), `Laws` (Task 3), `Registry` (Stage 1).
- Produces:
```
GET /api/disposition?id=ENTITY&at=YEAR
  -> { "entity": ..., "at": ...,
       "dispositions": [ { "layer": ..., "kind": ...,
                           "tenure": "held"|"claimed",
                           "dress": {"edge": ..., "fill": ...},
                           "rank": N,
                           "laws": ["tenure/...", "dress/...", "rank/..."] } ] }
```
One row per `(layer, kind)` the entity is witnessed in at that instant, sorted. An entity that stands nowhere at `at` returns `"dispositions": []` and **HTTP 200** — the absence is an answer, not an error, exactly as spec §3 law 1 says of an omitted piece.

- [ ] **Step 1: Contract additions FIRST — runner red**

`contracts/map-api/fact/disposition.feature`:

```gherkin
Feature: disposition — what a thing is, how it dresses, and by whose law
  Spec §2 equation 4: `dispose : (Entity, Witness*, Standing, Laws) →
  {tenure, dress-class, rank}` is a TOTAL PURE FUNCTION. Total, so
  every entity at every instant has an answer and none of these
  scenarios can 404. Pure, so the two determinism properties below are
  laws rather than luck. And citing: every answer names the law rows
  that produced it, which is what makes a policy change reviewable as
  a census diff before any pixel renders.

  Vocabulary:
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |

  Scenario: Judea under the tetrarchies, whole
    When I GET /api/disposition?id=authored:judea&at=59
    Then the response equals fixture "disposition-judea-59"

  Scenario: the promise of NUM 34 at the conquest, whole
    When I GET /api/disposition?id=authored:the-land-promised-num-34&at=-1405
    Then the response equals fixture "disposition-num34-1405"

  Scenario: an entity nobody has ever minted is disposed, not refused
    When I GET /api/disposition?id=no-such-entity&at=-1405
    Then the response equals fixture "disposition-empty"

  @property
  Scenario: disposition is deterministic at any year
    When I GET /api/disposition?id=authored:judea&at=<someYear> as first
    And I GET /api/disposition?id=authored:judea&at=<someYear> as second
    Then first equals second

  Scenario: every answer cites the law rows that produced it
    When I GET /api/disposition?id=authored:judea&at=59 as judea
    Then every disposition in judea cites a rule of every family
```

The third scenario is the totality law with teeth: a server that 404s an unknown id fails it, and a server that returns `{}` fails it too, because the fixture pins the whole body including the empty `dispositions` array and the echoed `entity`/`at`.

Rewrite `contracts/map-api/fact/derivability.feature`'s scenario into two, and narrow the preamble:

```gherkin
  Scenario: every manifest entry traces to a disposition
    When I render pieces fills, borders at year -1405 in style canaan as sampled
    Then every entry in sampled traces to a disposition

  @target
  Scenario: every manifest entry traces to a border
    When I render pieces fills, borders at year -1405 in style canaan as sampled
    Then every entry in sampled traces to a border
```

and replace the preamble sentence *"Until `disposition` (Stage 2) and `borders` (Stage 3) exist on the wire, this law has nothing to sample against"* with:

> As of v0.3 the disposition half is real and green: every entry in a
> sampled manifest names an entity, and that entity has an answer from
> `/api/disposition` at the manifest's own year. The border half waits
> on Stage 3 and stays `@target` — the two tiers are now checked
> against each other on one axis of two, and saying so precisely is the
> point of splitting the scenario.

```bash
cd contracts/runner && cabal run contract-runner -- check ../map-api && cabal run contract-runner -- vocab ../map-api
```
Expected: ORPHANs for `every disposition in ... cites a rule of every family`, `traces to a disposition`, `traces to a border`, and Task 4's `every row of table is disposition's answer for that entity`. Four orphans; four step definitions in Step 3.

- [ ] **Step 2: Implement the route**

In `crates/map-viewer/src/lib.rs`, add the `"/api/disposition"` arm. For each `(layer, kind)` the entity is witnessed in at `at` — walk the canon's layers exactly as `census` does, filtering to features whose entity resolves to the requested id — call `dispose` and emit the row. Requesting an id nobody minted yields `"dispositions": []` and 200.

Resolve the requested id through `Registry::resolve` before matching, so `/api/disposition?id=partition:phoenicia` and `?id=phoenicia` give the same answer. Stage 1's registry guarantees that is total, idempotent and one-hop; **echo the RESOLVED id back in `entity`**, so a caller who asked with a minted alias can see which canonical node answered.

- [ ] **Step 3: Teach the runner the four steps**

In `contracts/runner/src/Steps.hs`:

```haskell
  -- Equation 4's image law (census.feature, added in Task 4). For the
  -- WHOLE census: fetch each row's disposition at the same year and
  -- demand the tenure agrees. Not a sample -- the fixture bodies are
  -- already whole, and a sampled check here would be the weak half of
  -- a strong pair.
  , mkStep Then (lit "every row of " *> capUntil @BindName
                 " is disposition's answer for that entity") $
      \(BindName n) w -> case Map.lookup n (bound w) of
        Nothing -> pure (Left ("unbound " <> n))
        Just (_, body) -> checkCensusIsDisposesImage w body

  -- Derivability, the disposition half (spec §4, diagnosis §9.1 #2).
  , mkStep Then (lit "every entry in " *> capUntil @BindName " traces to a disposition") $
      \(BindName n) w -> traceEntriesToDispositions w n

  -- Derivability, the border half: still @target, and the message now
  -- names ONLY what is missing. A target that overstates its gap is a
  -- target nobody re-reads.
  , mkStep Then (lit "every entry in " *> capUntil @BindName " traces to a border") $
      \(BindName n) w -> pure $ case Map.lookup n (bound w) of
        Nothing -> Left ("unbound " <> n)
        Just _  -> Left "derivability's border half cannot be sampled: \
                        \/api/borders lands in Stage 3"

  , mkStep Then (lit "every disposition in " *> capUntil @BindName
                 " cites a rule of every family") $
      \(BindName n) w -> pure (checkCitations w n)
```

Three implementation notes, each of which is a way this could pass vacuously:
- `checkCensusIsDisposesImage` must **refuse an empty census** with a named failure. A body with no rows satisfies "every row agrees" and would report the law met against a broken server — diagnosis §7.0's shape exactly, and §6.3's already-recorded near-miss on the piece-attribution step.
- `traceEntriesToDispositions` must likewise refuse an empty manifest, and must fail if any entry carries **no entity to trace** — "has no entity, therefore vacuously traces" is not tracing.
- `checkCitations` must require a citation from **each of the three families** (`tenure/`, `dress/`, `rank/`), not merely a non-empty list. One citation repeated three times would pass a length check.

- [ ] **Step 4: Bless, eyeball, run everything**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
```bash
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api --bless
```
**Eyeball all three fixtures.** `disposition-judea-59.json` must show `"tenure": "held"` with a `tenure/tetrarchy-judea` citation — if it says `claimed`, Task 6's declaration did not reach the server. `disposition-empty.json` must be a real 200 body with an empty array, not an error page. `disposition-num34-1405.json` must show `"claimed"` and `"edge": "line"` — the orthogonality of grade and tenure, visible on the wire for the first time.

```bash
make contract-gates
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api 2>&1 | tail -20
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
```
Expected: Task 4's census image scenario now GREEN; derivability's disposition half GREEN; its border half the only `@target` red left in the corpus; `ALL GOLDEN VIEWS HOLD`.

- [ ] **Step 5: Commit**

```bash
git add crates/map-viewer contracts/
git commit -m "disposition on the wire: the census is dispose's image, and derivability's first half is green"
```

---

### Task 12: `laws` on the wire, and `contract()` stops saying "preledger"

Spec §3: `laws() → active law set, versioned`. And `crates/map-viewer/src/lib.rs:895` currently advertises `"lawsVersion": "0.0-preledger"` — a placeholder that has been telling the truth and is about to stop.

**Files:**
- Create: `contracts/map-api/fact/laws.feature`
- Create (blessed): `contracts/map-api/fixtures/laws.json`
- Modify: `contracts/map-api/meta/contract.feature`, `contracts/map-api/fixtures/contract.json` (re-blessed — a MINOR bump, and the semver gate will demand the CHANGELOG entry Task 13 writes)
- Modify: `crates/map-viewer/src/lib.rs`

**Interfaces:**
- Consumes: `Laws::{render, digest, version}` (Task 3).
- Produces:
```
GET /api/laws -> { "version": "...", "digest": "0123456789abcdef",
                   "tenure": [...], "dress": [...], "precedence": [...], "supersession": [...] }
```
`contract()`'s `lawsVersion` becomes `"<version>@<digest>"`. Stage 3 consumes both.

- [ ] **Step 1: Contract additions FIRST — runner red**

`contracts/map-api/fact/laws.feature`:

```gherkin
Feature: laws — the knobs, in one place, versioned
  Spec §2: "Law — data, versioned: tenure rules, precedence rules,
  supersession rules, dress rules. The knobs live here: one row,
  global reach." Before v0.3 these were seven tuned integers in a
  `match layer`, an undeclared `.unwrap_or(2)`, and fifty enum values
  in a Rust table; a policy change had no blast radius and only pixels
  could reveal what it did. The whole set is pinned here, so a change
  to any rule is a diff a person reads before a build renders.

  Scenario: the active law set is exactly its blessed body
    When I GET /api/laws
    Then the response equals fixture "laws"

  Scenario: every rule carries a written reason
    When I GET /api/laws as active
    Then every rule in active has a non-empty reason

  Scenario: every family has a rule that matches everything
    When I GET /api/laws as active
    Then every family in active declares a default
```

The last two are the law-as-scenario principle applied to the law set itself: a rule with no reason is an unexplained knob, and a family with no catch-all makes `dispose` partial. Both are already `LawViolation`s in Rust; putting them on the wire means a *consumer* can check them, which is what a contract is for.

Add to `contracts/map-api/meta/contract.feature`'s existing scenario nothing at all — its fixture is a whole body and `lawsVersion` moving is precisely what re-blessing it records. Update its preamble to say the pin is now `version@digest` and why: a law-set edit that forgot its version bump changes the digest and the fixture goes red.

```bash
cd contracts/runner && cabal run contract-runner -- check ../map-api
```
Expected: two ORPHANs (`has a non-empty reason`, `declares a default`).

- [ ] **Step 2: Implement**

Add `"/api/laws"` to the viewer, serving `Laws::render()`'s object with `version` and `digest` added. Change `lawsVersion` to `format!("{}@{:016x}", laws.version, laws.digest())`.

Add the two step definitions. Each must **refuse an empty law set** with a named failure — "every rule in an empty set has a reason" is true and worthless.

- [ ] **Step 3: Bless, eyeball, and re-run**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
```bash
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api --bless
git diff --stat contracts/map-api/fixtures/contract.json
```
`contract.json`'s diff must be **exactly one line** — `lawsVersion`. If `graphPin` also moved, the canon changed in this task, which it must not have; find out why before continuing.

Read `laws.json`: it must carry every rule written in Tasks 3, 6, 8 and 9, each with its `because`. A rule missing here is a rule the server is not reading.

- [ ] **Step 4: Full gates and commit**

```bash
make contract-gates
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api 2>&1 | tail -20
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
```
```bash
git add crates/map-viewer contracts/
git commit -m "laws on the wire, and contract() stops saying preledger"
```

---

### Task 13: Close the stage — v0.3.0, the EMPTY census diff, and the findings the law set made visible

**Files:**
- Modify: `contracts/VERSION`, `contracts/CHANGELOG.md`
- Modify: `contracts/map-api/fact/census.feature` (final preamble pass)

- [ ] **Step 1: The full gate sweep, all four strata, recorded**

```bash
make contract-gates
cd contracts/runner
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api    | tee ../../map-api.out
cabal run contract-runner -- run --base-url http://127.0.0.1:8080 ../atlas-edge | tee ../../atlas-edge.out
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
```
Expected: map-api all green except the single declared `@target` (derivability's border half, waiting on Stage 3); atlas-edge 6/6 — **and if atlas-edge is not 6/6, that is upstream drift, not this stage's doing; report it and do not "fix" the suite, which now belongs to the atlas session**; `cargo test --workspace` with zero `FAILED`; `ALL GOLDEN VIEWS HOLD` at 89/89. Anything else stops the stage.

- [ ] **Step 2: THE EMPTY CENSUS DIFF — the stage's judge**

Spec §5: *"Census diff must be empty."* This stage's proof is mechanical and it has three independent legs, which is why it is worth stating as three commands rather than one claim:

```bash
# 1. The blessed whole-body fixtures were never re-blessed. If this
#    prints anything, the diff was not empty and something was
#    blessed to hide it.
git diff --stat master -- contracts/map-api/fixtures/census-1405.json \
                          contracts/map-api/fixtures/census-1050.json \
                          contracts/map-api/fixtures/census-59.json

# 2. The three census scenarios are green against the live server.
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api 2>&1 | grep -i census

# 3. Stage 1's diff instrument, run at the five years that matter, on
#    a server that is now serving dispose's answers.
for Y in -1446 -1405 -1050 -586 59; do
  echo -n "self-diff at $Y: "
  curl -s "http://127.0.0.1:8090/api/census?year=$Y&to=$Y" | python -c "
import json,sys; d=json.load(sys.stdin); print('EMPTY' if d==[] else f'NON-EMPTY: {len(d)} changes')"
done
```
Leg 1 is the strongest: it is a claim about the repository that cannot be argued with. Leg 3 is the weakest and is included anyway, because it exercises the route that Stage 3 will be judged by and a broken instrument should be found now.

> **STOP. OWNER GATE.** If leg 1 shows any change to a census fixture, the stage does not close. Bring the difference to the owner **row by row**, name which task introduced it, and say which law row is responsible. There is no version of this stage in which a census fixture is re-blessed without that conversation.

- [ ] **Step 3: Move the version and write the changelog entry**

`contracts/VERSION`:
```text
0.3.0
```

Prepend to `contracts/CHANGELOG.md` under `## Unreleased` a new `## 0.3.0` section. **Write it as the owner's declaration of what changed, not as a commit log.** It must name: the ledger (`standings`, `disposition`, `laws` on the wire); the 75 `Stands`/`Holds`/`Grade` values and 32 `stands_until` data rows that became ledger rows; the shadow span map that became one supersession rule; the seven precedence constants that became six rules and a declared default; endurance becoming a typed field with a written reason, so the frame-edge law stopped grepping prose; `Area.tenure` deleted, because tenure now has exactly one home and it is `dispose`; `census.feature`'s new image law; derivability's disposition half going green; `contract()`'s `lawsVersion` moving off `0.0-preledger` to `version@digest`; `contract.json` and `census.feature`'s preamble re-blessed; **and, in one sentence, that the three census fixtures did not move, which is the stage's whole claim.**

- [ ] **Step 4: Report the findings the law set made visible — and do NOT act on them**

Writing the laws down surfaced three things that were always true and never written. Each is a **policy question with a non-empty census diff or a pixel change**, so each is out of this stage's scope by construction. Take all three to the owner as written findings:

1. **Twenty partition faces in the `scripture-claims` layer are `held`.** `partition:ammon`, `partition:asher`, `partition:benjamin`, `partition:canaan`, `partition:dan`, `partition:edom`, `partition:ephraim`, `partition:gad`, `partition:geshur`, `partition:issachar`, `partition:judah`, `partition:manasseh-east`, `partition:manasseh-west`, `partition:moab`, `partition:naphtali`, `partition:philistia`, `partition:phoenicia`, `partition:reuben`, `partition:simeon`, `partition:zebulun` — `partition_bridge.rs:605` hardcoded `Tenure::Held` on every face regardless of layer, and the law set now reproduces that through `derivation: "traced"`. Whether a tribal allotment or Canaan-the-frame should *hold* ground or *claim* it is a real question and the answer would move twenty census rows and the fill of twenty regions. **The instrument to decide it now exists**: change one law row, run `/api/census?year=A&to=B`, read the diff before any pixel renders. That is what this stage was for.
2. **`canon_provider.rs:305`'s `hash64(&entity.0) % 8`** is a tuned constant with an entity-dependent answer, used as a fallback when an entity has no measured palette slot. It is a violation of the owner's standing law. It was left alone because it is a fallback for a missing *measurement*, not a disposition, and changing it moves palette assignments and therefore pixels.
3. **`census`'s five-column schema is now narrower than `dispose`'s answer.** The census reports `tenure` only; `dispose` also produces dress-class, rank and citations. Widening the census would make Stage 1's `census_diff` report every row as `Changed`, because it keys on `(layer, entity)` and compares whole rows — the instrument cannot distinguish a schema change from a policy change. Recommendation, for the owner and for Stage 3's planning pass: widen it once, deliberately, in a stage whose diff is *expected* to be non-empty, with a companion check that the five original columns are byte-identical. Do not widen it inside a stage judged by an empty diff.

- [ ] **Step 5: Re-run the semver gate against itself**

```bash
bash scripts/contract-semver-gate.sh origin/master
```
Expected: exit 0 — this stage edited existing features and re-blessed `contract.json`, and both `contracts/VERSION` and `contracts/CHANGELOG.md` moved. If it exits 1, the gate is right and Step 3 is incomplete.

- [ ] **Step 6: Commit**

```bash
git add contracts/VERSION contracts/CHANGELOG.md contracts/
git commit -m "Stage 2 closes at v0.3.0: the ledger, judged by an empty census diff"
```

- [ ] **Step 7: Report to the owner and STOP**

Report: the four strata's numbers; the empty census diff with all three legs of its proof; the literal counts retired (75 `Stands`/`Holds`/`Grade` values, 32 `stands_until` data rows, one shadow span map, seven precedence constants, one prose-grepping law); the one remaining declared `@target` (derivability's border half, waiting on Stage 3); and Step 4's three findings, **unacted on and named as the owner's to rule on**. **Do not begin Stage 3.** It gets its own planning pass against this stage's evidence, with the owner in the loop.

---

## What Stage 2 produces that Stage 3 consumes

Stage 3 is "one arrangement": survey circuits enter the partition as witnesses, every border becomes canonical, claims reference edges (the promise's west border IS the coastline), `borders` lands with provenance — **and its census diff must also be empty**. Everything below is what Stage 3 leans on, and all of it must be stable and queryable before it starts.

**The ledger.**
- `map_canon::ledger::{Ledger, Standing, Endurance, WitnessKey}`, persisted with the canon and reachable as `store.ledger()`.
- `Standing { entity, witness, layer, from, until, endurance }`. `entity` is a Stage-1-`resolve`'d canonical id; `witness` is **not** resolved and must not be — `"egypt@-1500"` and `"egypt@-1200"` are two standings of one entity, and collapsing them collapses the era-variants that let an empire morph.
- `Ledger::stands` is **total** (an undeclared witness stands always); `standing_of`, `standings_at`, `standings_over`, `standings_of_entity`, `absent_at` and `eras` are its queries. `eras` reproduces the retired `PresenceBook::eras` exactly, and Task 7's test is the proof — Stage 3's arrangement work must not change an era cut without saying so.
- `Ledger::validate` enforces `EntityStandsTwice`. `declare` refuses `BackwardsSpan`, `OverlappingSpans`, `EnduresWithAnEnd` and `EndlessWithoutAReason`, so an unlawful ledger is unrepresentable rather than merely discouraged — **including when loading from disk**, because `persist` rebuilds through `declare`.
- `data/authored/standings.json` is the owner-reviewable declaration file for authored standings. Stage 3 adds no rows to it; a Stage 3 change that needs one is a standing change and belongs in a Stage 2 amendment with its own census diff review.

**The law set.**
- `map_canon::law::Laws`, versioned, loaded from `data/authored/laws.json` at compile AND at serve, with `validate` refusing a set that would make `dispose` partial.
- Four families — `tenure`, `dress`, `precedence`, `supersession` — each with a **declared default that is a citable row**. Stage 3 adds rules by adding rows; the `NoDefault`, `DuplicateId`, `Unreachable` and `Unexplained` laws then cover them automatically.
- `Selector { layer, kind, derivation, entity }` is the rule's left-hand side. **Stage 3 will want a fifth field** (a border's provenance, so a canonical edge's dress can be ruled on): add it to `Selector`, to `parse`/`render`, and to `dispose`'s query — everything else follows, and the `Unreachable` check keeps the older rows honest.
- `Laws::digest` and the `version@digest` pin in `contract()`. A law-set edit that forgets its version bump changes the digest and `contract.json` goes red.

**The disposition function.**
- `dispose(entity, layer, kind, derivation, standing, laws) -> Disposition { tenure, dress, rank, citations }` — **total, pure, and citing.** Stage 3 may call it freely; it does no I/O and reads no clock.
- `map_canon::Derivation { BorderText, CityDerived, Traced, Vendored }` rides on `Provenance`. Stage 3's canonical borders carry it, and it is the field a border-provenance rule will select on.
- **`Area` has no `tenure` field.** Tenure has exactly one home and it is `dispose`. Re-introducing a stored tenure anywhere is a return of the disease.

**The wire, for Stage 3 to extend additively.**
- `GET /api/standings?at=` and `?from=&to=`; `GET /api/disposition?id=&at=`; `GET /api/laws`.
- `GET /api/census?year=` — **frozen at five columns for Stage 3 as well**, unless the owner rules otherwise on Task 13 Step 4's third finding. `GET /api/census?year=A&to=B` is Stage 1's diff instrument and remains the judge; Stage 2 threaded `&Laws` through it and changed none of its answers.
- `contracts/map-api/fact/census.feature`'s image law: **the census is `dispose`'s image, row for row.** Stage 3 must keep it green, which means any new fact kind it introduces needs a `FactKind` variant and a law row, not a special case in `census`.

**Instruments, inherited and extended.**
- Task 1's `map_adapters::timeline_fingerprint` and `the_blessed_censuses_still_hold` — the offline half of the empty-diff law. **Stage 3 should re-use both as its own per-task acceptance tests**; they are why this stage could tell which of nine tasks moved an answer.
- Stage 1's `forAllShrink`, the hole-distinctness law in `contract-runner check`, `make ci` / `make contract-gates`, the semver gate and `contracts/CHANGELOG.md`. Stage 3 opens at `0.3.0` and bumps to `0.4.0` as its own closing declaration.

**Explicitly NOT produced here, so Stage 3 does not assume it:** `borders` and canonical edges; a `⊕` combine endpoint (diagnosis §8.5 still argues against building one); `Witness.evidence` as a first-class node (only `Derivation` and the standing interval landed); dress-class on the census; and any correction to the twenty `held` partition faces, the palette hash fallback, or the census schema — all three are Task 13 Step 4's findings and all three are the owner's to rule on.

---

## Self-Review (performed at write time)

**1. Spec coverage.** §5 Stage 2's five named deliverables: `Stands`/`Holds`/`Grade` literals become ledger rows (Tasks 5, 6); `stands_until` strings become ledger rows (Task 7); shadow spans become ledger rows (Task 8 — as a supersession rule plus the winner's standing, which is what a span *was*); laws become the law set (Tasks 3, 6, 8, 9); `standings`/`disposition`/`laws` added (Tasks 10, 11, 12); `surveys.rs` shrinks to circuit evidence (Task 6 Step 5); census diff empty (Task 1's harness at every task, Task 13 Step 2's three legs). ✓ §2's Standing node, PresenceBook promoted, endurance typed with a written reason so the frame-edge law stops grepping prose: Tasks 2, 5, 7. ✓ §2's Law node — data, versioned, four families, one row global reach: Task 3. ✓ §2 equation 4, `dispose` total and pure with the census as its image: Task 4, with the image law as a contract scenario. ✓ §2's "what dies" list: every one of its seven items has a named task and a cited file:line, in the table at the top. ✓ §4's derivability-by-sampling: half green in Task 11, half honestly `@target`. ✓ §6's four strata every stage: Rust tests in every task, the contract suite in 4/10/11/12/13, the census diff in 1 (built) and 13 (proved), the golden gate in 4/5/6/7/8/9/11/12/13. ✓ §4's semver: Task 13, through Stage 1's gate.

**Gaps I am leaving open, deliberately and with the reason.** The census stays five columns, so it is `dispose`'s image on the tenure axis and not on the dress-class or rank axes — spec §2 says "the census is its image", and widening it inside a stage judged by an empty diff would make the judge unable to speak. This is the one place I read the spec as internally tense, and Task 13 Step 4 finding 3 hands the tension to the owner with a recommendation rather than resolving it silently. Spec §3 law 2's true `⊕` still is not tested; diagnosis §8.5 argues against building a combine now and nothing in this stage changes that.

**2. Placeholder scan.** No "TBD", no "add error handling", no "similar to Task N". Every code step carries real code. Eleven places say "adapt to what the file already does" and each names the exact file, the exact thing to read, and what to do if it differs — those are instructions to read code that exists, not deferred decisions. Three places carry an explicit branch the executor must choose (Task 5's `force_shift_for_test` versus re-parsing, Task 8's `layer_hint` decision with the recommendation stated and the rejected alternative named, Task 7's `registry()` baseline), and each names the preferred option and why. Two STOP-AND-ASK owner gates (Task 13 Step 2, and Task 9 Step 5's golden-gate stop) are decisions this plan must not make.

**3. Type consistency.** `Standing`/`Endurance`/`WitnessKey`/`Ledger` keep their Task 2 names and fields through Tasks 5, 7, 8, 10 and the closing section; `Standing.layer` is added in Task 8 Step 3 and the closing section lists it, so the field set is stated in exactly two places and they agree. `Laws`/`LawId`/`Selector`/`Derivation`/`DressClass`/`EdgeStyle`/`FillRoute`/`FactKind` keep their Task 3 names through Tasks 4, 6, 8, 9, 11, 12. `dispose`'s six-argument signature is written once in Task 4's Interfaces and used unchanged in Tasks 4, 6, 9, 11 and the closing section. `census`/`census_json`/`census_diff` all gain `&Laws` in Task 4 and are called with it thereafter. `derivation_of` is declared in Task 4 as a `None`-returning stub with a named successor task and implemented in Task 6. `timeline_fingerprint` and `the_blessed_censuses_still_hold` keep their Task 1 names in every later task's verification step.

**One inconsistency I found and fixed inline:** Task 4's first draft had `census` reading `dispose` for all five fact kinds, which flips the single `claimed` row before Task 6 exists to declare it. The Task 4→6 seam, its exact comment, and the explicit refusal to close it early with a layer-keyed rule are the fix.

**Two corrections to this plan's own brief, recorded so they are not rediscovered:**
- The brief says `surveys.rs` is at `crates/map-compile/src/surveys.rs`. It is **`crates/map-adapters/src/surveys.rs`** (1742 lines). `map-compile` has no such file.
- The brief lists "the census diff law as a contract scenario" among the obligations still uncovered and not assigned to Stage 1. **Stage 1's Task 7 does add it** — three scenarios in `census.feature` (a whole diff between two instants, a self-diff, and a `@property` self-diff at any year) plus the `?to=` route. This stage therefore does not re-add it; what it adds instead is the *image* law (the census is `dispose`'s image), which spec §2 equation 4 requires and which nothing in Stage 0 or Stage 1 states.
