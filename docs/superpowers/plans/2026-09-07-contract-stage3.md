# Stage 3: One Arrangement — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land spec §5's Stage 3 — every border is one canonical edge in THE one arrangement carrying the witness set whose geometry it realizes; survey circuits enter the partition as witnesses instead of authoring their own parallel geometry; extents become derived references into the arrangement; `borders` lands on the wire with provenance — and do it with **every snapping tolerance a declared, first-class value with a falsifiable law**, never a number nudged until the pixels looked right.

**Architecture:** Three parts, strictly ordered, with a hard owner gate between B and C. **Part A** builds the tolerance regime *before* any geometry moves: a `Tolerance` type that cannot be constructed without naming the two measured populations it separates and the plateau within which its exact value does not matter, a data-file ledger the owner reviews like a law row, and two laws that can actually go red — a *separation* law that re-measures both populations from the real witness data, and a *plateau* law that asserts the arrangement's content hash is invariant across the declared band. **Part B** makes the canonical edge real in the canon — `map_partition` already computes one canonical `PEdge` per arc with its witness set (`crates/map-partition/src/lib.rs:149-159`), and `crates/map-compile/src/partition_bridge.rs:291-301` throws every one of them away by storing whole dissolved rings — so Part B stores arcs, turns `Area` rings into cycles of oriented arc references, and serves `borders`. Part B moves **zero pixels** by construction and the golden gate proves it. **Part C** is where geometry genuinely changes: extents stop owning faces exclusively, tenure rides through the partition so the census cannot move, the 25 survey circuits in `crates/map-adapters/src/surveys.rs` become witnesses, and the promise's west border becomes the coastline. Part C moves pixels on purpose, under an owner gate with a declared drift budget.

**Tech Stack:** Rust (map-types, map-canon, map-partition, map-adapters, map-compile, map-provider, map-viewer); Haskell contract runner (GHC 9.12.1, cabal 3.18.1.0, megaparsec, aeson, QuickCheck, hspec); Gherkin `.feature` corpus + blessed JSON fixtures; Node + playwright-core for the golden gate.

**Spec:** `docs/superpowers/specs/2026-09-06-map-api-contract-design.md`

**Diagnosis (Stage 0's evidence, corrected):** `docs/notes/2026-09-06-contract-diagnosis.md`

**Consumed plans:** `docs/superpowers/plans/2026-09-07-contract-stage1.md` (entity registry; pieces first-class). **Stage 2's plan did not exist when this was written** — `docs/superpowers/plans/2026-09-07-contract-stage2.md` is absent from the repository. Everything this plan consumes from Stage 2 is taken from spec §5's description of it (`standings`, `disposition`, `laws`; `Stands`/`Holds`/`Grade` literals and `stands_until` strings become ledger rows; `surveys.rs` shrinks to circuit evidence; census diff empty) and is named explicitly at each point of use, with a stated fallback if Stage 2 landed differently. **Read the real Stage 2 plan before executing Tasks 10, 11 and 12 and reconcile.**

**Stage 0 plan (format exemplar only):** `docs/superpowers/plans/2026-09-06-contract-stage0.md`

## Global Constraints

- **TDD is mandatory.** Every task writes its failing test first, runs it, sees it fail *for the stated reason*, then implements. No exceptions.
- **Owner's standing law:** first-class composable types with laws. Never styling tricks, never special cases, **never tuned constants**. This stage is where that law is most acutely at risk: spec §8 names "Stage 3 geometry snapping" as the top risk and answers it with "tolerances declared, not tuned". Part A defines what *declared* means as two falsifiable laws, and no later task may introduce a bare float.
- **"Declared" is not "tuned once and written down."** A declared tolerance must satisfy both Part A laws: it **separates** two populations re-measured from the real data with margin on both sides, and it sits inside a **plateau** — a band of values across which the arrangement's content hash does not change at all. A number whose exact value changes the answer is tuned, whatever its comment says.
- **Whole-body assertions.** A scenario pins the ENTIRE answer, with don't-cares masked explicitly in the scenario text. Existential poke-assertions ("some face claims canaan", "is an array", "has at least N") are forbidden. Note that the *existing* integration law `plate_partition_face_census` (`crates/map-compile/src/tests.rs:376-410`) is written in exactly the forbidden style (`p.faces.iter().any(...)`); Task 10 replaces its assertions with whole-body ones as part of its own work, and no new test may copy that shape.
- **A check satisfiable by the failure mode is not a check.** Diagnosis §7.0: the assertion side AND the input side both need discipline. Every law added by this plan ships with an explicit negative case proving it can go red.
- **Order within every task:** contract additions first (runner red) → Rust tests (cargo red) → implement (green) → golden gate → census diff reviewed → commit.
- **`contracts/VERSION` moves `0.3.0` → `0.4.0`** in Task 12, and that bump is the owner's declaration of what changed. If Stage 2 did not land and VERSION still reads `0.2.0`, move it to `0.4.0` anyway and say so in `contracts/CHANGELOG.md`. Pre-release continues until the owner declares v1.0 — never automatic.
- **The census diff must be EMPTY** (spec §5, §6 stratum 3). Use Stage 1's instrument, `GET /api/census?year=A&to=B`. "Empty" means empty across **every** golden stop, not a sampled few: Task 12 sweeps all 89. A non-empty diff STOPS the task and goes to the owner.
- **The golden gate is the hard judge**, and this stage is the one where that has teeth. `node crates/map-viewer/tests/golden.js --check` must print `ALL GOLDEN VIEWS HOLD` (89/89 stops). Parts A and B must show **zero** drifted probes — a single drift there is a bug, because neither part changes any coordinate. Part C moves pixels deliberately: Task 11 declares its expected drift *before* running the gate, and re-blessing happens only on the owner's explicit approval. 1405 BC and 1446 BC are the owner's beloved stops (MEMORY: golden-views-no-regression) and get named scrutiny.
- **The golden gate is necessary but not sufficient for geometry work.** It samples 25 fixed `(u,v)` points per camera per stop with a per-channel tolerance of 24 (`crates/map-viewer/tests/golden.js:20-29`). A border can move a long way without crossing a probe. Every task that touches coordinates therefore carries its own geometry-side whole-body assertion in Rust; the gate is the second judge, not the only one.
- **Ports.** The workbench viewer is **8090**. `8080` (atlas API), `8081`, `8000`, `5000` belong to the atlas pipeline and must never be bound.
- **The atlas repo is a READ-ONLY path dependency.** Never edit it. `atlas_graph_types` is consumed, never changed.
- **The render pipeline is three steps** (MEMORY: render-pipeline-three-steps). This stage changes `map-compile`, so every task that touches Rust needs all three:
  ```powershell
  cargo build --release
  cargo run --release -p map-compile -- build
  Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
  Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
  ```
  Skipping the `map-compile build` step leaves a stale canon and the golden gate then judges the old geometry — a false green, which is the worst outcome available in this stage.
- **Haskell commands run from `contracts/runner/`** unless stated otherwise.
- **Feature preambles are unverified prose** (diagnosis §9.2). Any task that changes what a feature *means* updates its preamble in the same commit.

---

## File structure

**New files**

| path | responsibility |
|---|---|
| `crates/map-types/src/tolerance.rs` | `Tolerance`, `Separation`, `Plateau`, `ToleranceViolation`, `Tolerances` — the type that makes a bare snapping constant unrepresentable |
| `data/authored/tolerances.json` | the owner-reviewable tolerance ledger: value, unit, the two measured populations, the plateau, the evidence |
| `crates/map-compile/src/tolerances.rs` | loader + whole-body validation of the ledger against the types |
| `crates/map-partition/src/literals.rs` | the bare-float-literal scanner and its declared allowlist (the law that stops Part A from decaying) |
| `contracts/map-api/fact/borders.feature` | `borders(entity, at)` as laws: canonicity, provenance totality, the shared-edge law, derivability |
| `contracts/map-api/fixtures/borders-*.json` | blessed whole bodies |

**Modified files**

| path | change |
|---|---|
| `crates/map-types/src/lib.rs` | export `tolerance`; move `Tenure` here from `map-canon` |
| `crates/map-partition/src/lib.rs:124-140` | `PartitionConfig` becomes `Tolerances`-backed; `PEdge` gains a stable arc key; `Partition::dissolve_arcs`, `Partition::extents` |
| `crates/map-partition/src/build.rs` | every derived tolerance (`:296`, `:397`, `:695`, `:752`, and the bare `1e-12`s) becomes a named declared member |
| `crates/map-canon/src/lib.rs` | `Arc`/`ArcId`/`Cycle`; `Area.rings`/`holes` become cycles of oriented arcs; `Tenure` re-exported from map-types |
| `crates/map-canon/src/persist.rs` | arcs and cycles round-trip byte-stably |
| `crates/map-compile/src/partition_bridge.rs` | store arcs not rings; carry entity + tenure on witnesses; overlapping extents; survey circuits as witnesses |
| `crates/map-adapters/src/surveys.rs` | circuits become exported witness evidence; the survey stops authoring parallel region geometry |
| `crates/map-provider/src/canon_provider.rs` | resolve arc cycles back to rings for rendering |
| `crates/map-viewer/src/lib.rs` | `GET /api/borders`; `/api/contract` gains the arrangement's completeness statement |
| `contracts/VERSION`, `contracts/CHANGELOG.md` | `0.4.0` |

---

## Part A — the tolerance regime, before any geometry moves

Spec §8 names Stage 3's snapping as the top risk and the owner's law forbids tuned constants. Today the codebase contains one exemplary declared tolerance and a crowd of undeclared ones:

- **The exemplar.** `MOUTH_GAP` (`crates/map-compile/src/partition_bridge.rs:215-222`) is 5 km, and its comment gives a real argument: the largest observed gap for a sea-reaching river in the vendored data is the Yarkon's 4.25 km; the nearest endorheic desert network sits more than 20 km from any water ring; 5 km separates the two classes with margin on both sides. That is what *declared* means — two measured populations and a value bracketed between them. It is still only a comment: nothing re-measures it, and nothing fails if someone moves it to 12.
- **The crowd.** `PartitionConfig::default()` (`crates/map-partition/src/lib.rs:135-140`) is `tau_vertex: 1.2e-5, tau_edge: 2.4e-5, sliver_area: 5.0e-8` with the comment "~75 m and ~150 m on Earth's radius; slivers under ~2 km²" — a restatement of the numbers in other units, not an argument for them. `TERRITORY_TOLERANCE_DEG = 0.2` (`crates/map-canon/src/lib.rs:711-715`) at least cites a measurement. And `build.rs` derives four more tolerances from those by unexplained multipliers: `cfg.tau_vertex * 0.5` (`:296`), `(cfg.tau_vertex * 0.4).max(1e-7)` (`:397`), `cfg.tau_edge * 2.0` (`:695`), `tol * 4.0` (`:752`), plus bare `1e-12` guards throughout and a `1e-10` completeness bound (`crates/map-partition/src/lib.rs:501`). `snap_ring_to`'s `budget = 3.0 / 6371.0` (`crates/map-compile/src/partition_bridge.rs:162`) cites "the vendored witness's declared accuracy" without saying where that declaration lives, and carries `full * 0.02 + 1e-9` (`:760`) and a `60.0` span cap (`:814`) inside it.

Part A does not change one coordinate. It changes what a tolerance *is*.

---

### Task 1: `Tolerance` — the type a tuned constant cannot inhabit

**Files:**
- Create: `crates/map-types/src/tolerance.rs`
- Modify: `crates/map-types/src/lib.rs` (add `pub mod tolerance;`)
- Test: `crates/map-types/src/tests.rs` (append; the crate keeps its tests in one module — follow the file's existing convention)

**Interfaces:**
- Consumes: nothing.
- Produces: `map_types::tolerance::{Tolerance, Separation, Plateau, ToleranceViolation}`; `Tolerance::validate(&self) -> Vec<ToleranceViolation>`; `Tolerance::radians(&self) -> f64`; `Tolerance::scaled(&self, factor: f64) -> f64`. Tasks 2–5 and 10–11 consume these; **no code added by this plan may pass a bare `f64` where a tolerance is meant.**

- [ ] **Step 1: Write the failing tests**

Append to `crates/map-types/src/tests.rs`:

```rust
// ---------------------------------------------- the tolerance regime
//
// Spec §8 names Stage 3's snapping as the top risk and answers it
// "tolerances declared, not tuned". These laws are what makes the
// difference checkable rather than rhetorical: a declared tolerance
// SEPARATES two measured populations with margin on both sides, and
// it sits inside a PLATEAU whose whole point is that its exact value
// does not matter. Each law below carries its negative case, because
// a check satisfiable by the failure mode is not a check.

use crate::tolerance::{Plateau, Separation, Tolerance, ToleranceViolation};

fn exemplar() -> Tolerance {
    // MOUTH_GAP as it stands today, promoted from a comment to a
    // value: 5 km on Earth's radius, bracketed by the Yarkon's
    // 4.25 km truncation and the nearest endorheic network's 20 km.
    Tolerance {
        name: "mouth_gap",
        radians: 5.0 / 6371.0,
        separates: Separation { same_max: 4.25 / 6371.0, distinct_min: 20.0 / 6371.0 },
        plateau: Plateau { lo: 0.75, hi: 2.0 },
        min_margin: 1.1,
        evidence: "the largest observed gap for a sea-reaching river in the vendored \
                   OSM data is the Yarkon's 4.25 km; the nearest endorheic desert \
                   network sits more than 20 km from any water ring"
            .to_string(),
    }
}

#[test]
fn a_declared_tolerance_is_lawful_whole() {
    assert_eq!(exemplar().validate(), Vec::<ToleranceViolation>::new());
}

#[test]
fn a_tolerance_that_separates_nothing_is_refused() {
    // The failure mode this law exists to catch: a value that does not
    // sit between the two populations at all.
    let mut t = exemplar();
    t.radians = 30.0 / 6371.0; // above distinct_min: distinct places merge
    assert_eq!(
        t.validate(),
        vec![ToleranceViolation::DoesNotSeparate {
            name: "mouth_gap",
            radians: 30.0 / 6371.0,
            same_max: 4.25 / 6371.0,
            distinct_min: 20.0 / 6371.0,
        }]
    );
    let mut t = exemplar();
    t.radians = 1.0 / 6371.0; // below same_max: the same place splits
    assert_eq!(
        t.validate(),
        vec![ToleranceViolation::DoesNotSeparate {
            name: "mouth_gap",
            radians: 1.0 / 6371.0,
            same_max: 4.25 / 6371.0,
            distinct_min: 20.0 / 6371.0,
        }]
    );
}

#[test]
fn a_tolerance_wedged_against_a_population_is_refused() {
    // Inside the bracket but with no room: 4.3 km is above the
    // Yarkon's 4.25 but only by 1.2%. A value that would flip on the
    // next data refresh is tuned, not declared.
    let mut t = exemplar();
    t.radians = 4.3 / 6371.0;
    assert_eq!(
        t.validate(),
        vec![ToleranceViolation::NoMargin {
            name: "mouth_gap",
            side: "same",
            ratio: (4.3 / 4.25),
            required: 1.1,
        }]
    );
}

#[test]
fn a_tolerance_with_no_plateau_is_refused() {
    // A plateau that does not contain 1.0 is a claim that the value
    // itself is outside the band where the answer is stable — which
    // is the definition of tuned.
    let mut t = exemplar();
    t.plateau = Plateau { lo: 1.2, hi: 2.0 };
    assert_eq!(
        t.validate(),
        vec![ToleranceViolation::PlateauExcludesValue { name: "mouth_gap", lo: 1.2, hi: 2.0 }]
    );
    let mut t = exemplar();
    t.plateau = Plateau { lo: 1.0, hi: 1.0 };
    assert_eq!(
        t.validate(),
        vec![ToleranceViolation::PlateauIsAPoint { name: "mouth_gap" }]
    );
}

#[test]
fn a_tolerance_with_no_written_evidence_is_refused() {
    let mut t = exemplar();
    t.evidence = "  ".to_string();
    assert_eq!(
        t.validate(),
        vec![ToleranceViolation::NoEvidence { name: "mouth_gap" }]
    );
}

#[test]
fn violations_are_reported_together_not_one_at_a_time() {
    // Fail-loud with the WHOLE story: the crate's convention (see
    // laws.rs) is an enumerable violation list, never a first-error.
    let mut t = exemplar();
    t.radians = 30.0 / 6371.0;
    t.evidence = String::new();
    t.plateau = Plateau { lo: 2.0, hi: 0.5 };
    assert_eq!(
        t.validate(),
        vec![
            ToleranceViolation::DoesNotSeparate {
                name: "mouth_gap",
                radians: 30.0 / 6371.0,
                same_max: 4.25 / 6371.0,
                distinct_min: 20.0 / 6371.0,
            },
            ToleranceViolation::PlateauInverted { name: "mouth_gap", lo: 2.0, hi: 0.5 },
            ToleranceViolation::NoEvidence { name: "mouth_gap" },
        ]
    );
}

#[test]
fn scaled_walks_the_plateau_and_nothing_else() {
    let t = exemplar();
    assert_eq!(t.scaled(1.0), t.radians());
    assert!((t.scaled(2.0) - 2.0 * (5.0 / 6371.0)).abs() < 1e-18);
    // the plateau's own endpoints, which Task 4 sweeps
    let ends = t.plateau_samples();
    assert_eq!(ends, vec![0.75, 1.0, 1.25, 1.5, 2.0]);
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p map-types tolerance`
Expected: FAIL — `error[E0432]: unresolved import 'crate::tolerance'`.

- [ ] **Step 3: Write `crates/map-types/src/tolerance.rs`**

```rust
//! TOLERANCES, DECLARED — never tuned.
//!
//! Spec §8 names Stage 3's geometry snapping as the top risk and
//! answers it in three words: "tolerances declared, not tuned". This
//! module is what makes that difference checkable instead of
//! rhetorical, because "declared" degrades into "tuned once and then
//! written down" the moment nothing can refute it.
//!
//! A tolerance is declared when BOTH hold:
//!
//! 1. SEPARATION — it names two populations MEASURED from the real
//!    witness data: `same_max`, the largest distance that must be
//!    treated as one place, and `distinct_min`, the smallest that
//!    must stay two. The value lies strictly between them, with a
//!    declared multiplicative margin on each side, so that a value
//!    which would flip on the next data refresh is refused.
//!
//! 2. PLATEAU — a band of multiplicative factors across which the
//!    ANSWER does not change at all. This is the load-bearing half:
//!    a tuned constant is precisely one whose exact value matters,
//!    so a tolerance sitting in the middle of a plateau cannot have
//!    been tuned — there was nothing to tune it to. The separation
//!    law argues the number is right; the plateau law proves the
//!    number is not doing the work.
//!
//! The exemplar this module generalises is `MOUTH_GAP` in
//! map-compile's partition bridge, whose comment already carried a
//! real separation argument (the Yarkon's 4.25 km against a 20 km
//! endorheic network). What it lacked was anything that would fail
//! if the number moved. That is what `validate` and the two law
//! suites in map-compile supply.

/// The two measured populations a tolerance stands between, in
/// radians on the unit sphere. Both are MEASUREMENTS, re-derived from
/// the real witness data by the separation law — never guesses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Separation {
    /// The largest distance that MUST be treated as the same place.
    pub same_max: f64,
    /// The smallest distance that MUST be treated as two places.
    pub distinct_min: f64,
}

/// The band of multiplicative factors within which this tolerance's
/// exact value does not change the answer. `lo < 1.0 < hi` always: a
/// plateau that excludes the value itself is a confession that the
/// value was tuned to an edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plateau {
    pub lo: f64,
    pub hi: f64,
}

/// A DECLARED tolerance. Construct one only with all five fields
/// answered; there is no `Default`, and there is deliberately no
/// `From<f64>` — a bare number cannot become a tolerance.
#[derive(Clone, Debug, PartialEq)]
pub struct Tolerance {
    /// The name the ledger, the laws, and every error message use.
    pub name: &'static str,
    /// The value, in radians on the unit sphere.
    pub radians: f64,
    pub separates: Separation,
    pub plateau: Plateau,
    /// The multiplicative room required on each side of the value.
    /// 1.1 means "at least 10% clear of both populations".
    pub min_margin: f64,
    /// The written argument: what was measured, in what data, and
    /// why these two populations are the right two. Prose, but
    /// mandatory prose — an empty one is a violation.
    pub evidence: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ToleranceViolation {
    /// The value does not lie between the two populations at all.
    DoesNotSeparate { name: &'static str, radians: f64, same_max: f64, distinct_min: f64 },
    /// It lies between them but is wedged against one — a value that
    /// would flip on the next data refresh.
    NoMargin { name: &'static str, side: &'static str, ratio: f64, required: f64 },
    /// lo > hi.
    PlateauInverted { name: &'static str, lo: f64, hi: f64 },
    /// The declared plateau does not contain the value itself.
    PlateauExcludesValue { name: &'static str, lo: f64, hi: f64 },
    /// lo == hi == 1.0: no plateau at all, which is the signature of
    /// a tuned constant.
    PlateauIsAPoint { name: &'static str },
    /// A population bound is not a positive finite number.
    UnmeasuredPopulation { name: &'static str, which: &'static str },
    NoEvidence { name: &'static str },
}

impl Tolerance {
    pub fn radians(&self) -> f64 {
        self.radians
    }

    /// The value scaled by a plateau factor — the only sanctioned way
    /// to ask "what if this number were different", used by the
    /// plateau law.
    pub fn scaled(&self, factor: f64) -> f64 {
        self.radians * factor
    }

    /// The factors the plateau law sweeps: both endpoints, 1.0, and
    /// interior points, sorted and deduplicated. Interior points
    /// matter — a law that only checks the endpoints could pass over
    /// a band with a hole in the middle.
    pub fn plateau_samples(&self) -> Vec<f64> {
        let Plateau { lo, hi } = self.plateau;
        let mut v = vec![lo, 1.0, hi];
        for k in 1..4 {
            let t = f64::from(k) / 4.0;
            v.push(lo + (hi - lo) * t);
        }
        v.sort_by(|a, b| a.partial_cmp(b).expect("plateau factors are finite"));
        v.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        v
    }

    /// Every law, checked; an empty vec is a declared tolerance.
    /// Reports the WHOLE story, never the first error.
    pub fn validate(&self) -> Vec<ToleranceViolation> {
        let mut v = Vec::new();
        let Separation { same_max, distinct_min } = self.separates;
        for (which, x) in [("same_max", same_max), ("distinct_min", distinct_min)] {
            if !(x.is_finite() && x > 0.0) {
                v.push(ToleranceViolation::UnmeasuredPopulation { name: self.name, which });
            }
        }
        if v.is_empty() {
            if !(self.radians > same_max && self.radians < distinct_min) {
                v.push(ToleranceViolation::DoesNotSeparate {
                    name: self.name,
                    radians: self.radians,
                    same_max,
                    distinct_min,
                });
            } else {
                if self.radians / same_max < self.min_margin {
                    v.push(ToleranceViolation::NoMargin {
                        name: self.name,
                        side: "same",
                        ratio: self.radians / same_max,
                        required: self.min_margin,
                    });
                }
                if distinct_min / self.radians < self.min_margin {
                    v.push(ToleranceViolation::NoMargin {
                        name: self.name,
                        side: "distinct",
                        ratio: distinct_min / self.radians,
                        required: self.min_margin,
                    });
                }
            }
        }
        let Plateau { lo, hi } = self.plateau;
        if lo > hi {
            v.push(ToleranceViolation::PlateauInverted { name: self.name, lo, hi });
        } else if lo == 1.0 && hi == 1.0 {
            v.push(ToleranceViolation::PlateauIsAPoint { name: self.name });
        } else if !(lo <= 1.0 && hi >= 1.0) {
            v.push(ToleranceViolation::PlateauExcludesValue { name: self.name, lo, hi });
        }
        if self.evidence.trim().is_empty() {
            v.push(ToleranceViolation::NoEvidence { name: self.name });
        }
        v
    }
}

/// THE DECLARED SET: every tolerance the arrangement runs under, in
/// one place, so a change to any of them is one reviewable diff with
/// global reach (spec §2's "the knobs live here: one row, global
/// reach", applied to geometry). Loaded from
/// `data/authored/tolerances.json` by map-compile; no field may be
/// read as a bare f64 anywhere else.
#[derive(Clone, Debug, PartialEq)]
pub struct Tolerances {
    /// candidate nodes closer than this are one vertex
    pub tau_vertex: Tolerance,
    /// a node this close to an edge interior lies ON that edge
    pub tau_edge: Tolerance,
    /// the minimum distance from a split point to either endpoint of
    /// the arc it splits (today: `tau_vertex * 0.5`, build.rs:296)
    pub tau_split_endpoint: Tolerance,
    /// how far off a cycle's first arc the interior probe point sits
    /// (today: `(tau_vertex * 0.4).max(1e-7)`, build.rs:397)
    pub tau_face_probe: Tolerance,
    /// how close a border edge's midpoint must lie to a river
    /// polyline to carry the river attribute (today: `tau_edge * 2.0`,
    /// build.rs:695, with a further `tol * 4.0` inside `on_arc`)
    pub tau_river_attr: Tolerance,
    /// the degenerate-cross-product floor: below this, two points are
    /// the same point or true antipodes (today: bare `1e-12`)
    pub tau_degenerate: Tolerance,
    /// how far a witness ring point may be moved onto a shared target
    /// arc (today: `budget = 3.0 / 6371.0`, partition_bridge.rs:162)
    pub snap_budget: Tolerance,
    /// the mouth-truncation allowance (today: `MOUTH_GAP`,
    /// partition_bridge.rs:222) — the regime's exemplar
    pub mouth_gap: Tolerance,
    /// steradians, not radians: cells smaller than this are slivers.
    /// Declared with the same two-population argument, measured in
    /// area rather than distance.
    pub sliver_area: Tolerance,
    /// the completeness residual bound: |4π − Σ areas| must stay
    /// under this (today: bare `1e-10`, lib.rs:501)
    pub completeness_residual: Tolerance,
}

impl Tolerances {
    pub fn all(&self) -> Vec<&Tolerance> {
        vec![
            &self.tau_vertex,
            &self.tau_edge,
            &self.tau_split_endpoint,
            &self.tau_face_probe,
            &self.tau_river_attr,
            &self.tau_degenerate,
            &self.snap_budget,
            &self.mouth_gap,
            &self.sliver_area,
            &self.completeness_residual,
        ]
    }

    /// Every tolerance's own laws, plus the ordering laws BETWEEN
    /// them: a vertex-merge radius must be smaller than an
    /// on-edge radius, or a point could be on an edge without being
    /// its endpoint's neighbour; a split point must stand clear of
    /// the vertices it splits between.
    pub fn validate(&self) -> Vec<ToleranceViolation> {
        let mut v: Vec<ToleranceViolation> =
            self.all().iter().flat_map(|t| t.validate()).collect();
        let ordered = [
            (&self.tau_split_endpoint, &self.tau_vertex, "tau_split_endpoint < tau_vertex"),
            (&self.tau_vertex, &self.tau_edge, "tau_vertex < tau_edge"),
            (&self.tau_edge, &self.tau_river_attr, "tau_edge < tau_river_attr"),
            (&self.tau_degenerate, &self.tau_split_endpoint, "tau_degenerate < tau_split_endpoint"),
        ];
        for (a, b, name) in ordered {
            if !(a.radians < b.radians) {
                v.push(ToleranceViolation::DoesNotSeparate {
                    name,
                    radians: a.radians,
                    same_max: 0.0,
                    distinct_min: b.radians,
                });
            }
        }
        v
    }
}
```

- [ ] **Step 4: Wire the module in**

In `crates/map-types/src/lib.rs`, beside the existing `pub mod` lines, add:

```rust
pub mod tolerance;
```

- [ ] **Step 5: Run the tests to green**

Run: `cargo test -p map-types tolerance`
Expected: PASS, 7 tests.

Then run the whole crate to be sure nothing else moved: `cargo test -p map-types`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/map-types/src/tolerance.rs crates/map-types/src/lib.rs crates/map-types/src/tests.rs
git commit -m "Stage 3 Task 1: Tolerance — the type a tuned constant cannot inhabit"
```

---

### Task 2: The tolerance ledger as owner-reviewable data

Spec §2 says the knobs live in the Law node: "data, versioned … one row, global reach." A tolerance is a knob. Today changing `tau_vertex` means editing a Rust literal inside a default impl — invisible in a census diff, invisible to the owner, indistinguishable from a refactor. After this task, changing a tolerance is a diff in one JSON file whose every row carries its measurement and its plateau, and the compile refuses a ledger that does not validate.

**Files:**
- Create: `data/authored/tolerances.json`
- Create: `crates/map-compile/src/tolerances.rs`
- Modify: `crates/map-compile/src/lib.rs` (add `pub mod tolerances;`)
- Test: `crates/map-compile/src/tests.rs` (append)

**Interfaces:**
- Consumes: `map_types::tolerance::{Tolerance, Separation, Plateau, Tolerances, ToleranceViolation}` (Task 1).
- Produces: `map_compile::tolerances::{load, LedgerError, DECLARED_PATH}`; `load(&str) -> Result<Tolerances, LedgerError>`. Tasks 3, 4, 5 and every geometry task consume `load`. `map_partition::PartitionConfig::from_tolerances(&Tolerances)` is added in Task 5; until then the loader stands alone.

- [ ] **Step 1: Write the failing tests**

Append to `crates/map-compile/src/tests.rs`:

```rust
// ---------------------------------------------- the tolerance ledger
//
// The ledger is DATA (spec §2: laws are data, versioned; one row,
// global reach). These laws pin it WHOLE — the ten declared rows and
// every field of each — so a silent edit to any number is a failing
// test naming the row, and a new row cannot be added without the
// owner seeing it in this fixture.

#[test]
fn the_declared_ledger_is_exactly_these_ten_rows() {
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH)
        .expect("the tolerance ledger is checked in");
    let t = crate::tolerances::load(&text).expect("the ledger loads");
    let names: Vec<&str> = t.all().iter().map(|x| x.name).collect();
    assert_eq!(
        names,
        vec![
            "tau_vertex",
            "tau_edge",
            "tau_split_endpoint",
            "tau_face_probe",
            "tau_river_attr",
            "tau_degenerate",
            "snap_budget",
            "mouth_gap",
            "sliver_area",
            "completeness_residual",
        ]
    );
}

#[test]
fn the_declared_ledger_obeys_every_tolerance_law() {
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let t = crate::tolerances::load(&text).expect("the ledger loads");
    assert_eq!(t.validate(), Vec::<map_types::tolerance::ToleranceViolation>::new());
}

#[test]
fn the_declared_values_are_exactly_todays_values() {
    // The regime must not move a single coordinate. These are the
    // numbers the arrangement runs under TODAY, transcribed from
    // map-partition/src/lib.rs:135-140, build.rs:296/397/695/752 and
    // map-compile/src/partition_bridge.rs:162/222. If this test fails,
    // Part A has changed the map, which it must not do.
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let t = crate::tolerances::load(&text).expect("the ledger loads");
    assert_eq!(t.tau_vertex.radians, 1.2e-5);
    assert_eq!(t.tau_edge.radians, 2.4e-5);
    assert_eq!(t.tau_split_endpoint.radians, 0.5 * 1.2e-5);
    assert_eq!(t.tau_face_probe.radians, (0.4f64 * 1.2e-5).max(1e-7));
    assert_eq!(t.tau_river_attr.radians, 2.0 * 2.4e-5);
    assert_eq!(t.tau_degenerate.radians, 1e-12);
    assert_eq!(t.snap_budget.radians, 3.0 / 6371.0);
    assert_eq!(t.mouth_gap.radians, 5.0 / 6371.0);
    assert_eq!(t.sliver_area.radians, 5.0e-8);
    assert_eq!(t.completeness_residual.radians, 1e-10);
}

#[test]
fn a_ledger_row_missing_its_evidence_is_a_named_error() {
    // The negative case: the loader is not a deserializer that
    // shrugs. Delete the argument and the compile refuses BY NAME.
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let mut v: serde_json::Value = serde_json::from_str(&text).expect("json");
    v["tau_vertex"]["evidence"] = serde_json::json!("");
    let err = crate::tolerances::load(&v.to_string()).expect_err("an unargued row is refused");
    assert_eq!(
        err,
        crate::tolerances::LedgerError::Unlawful(vec![
            map_types::tolerance::ToleranceViolation::NoEvidence { name: "tau_vertex" }
        ])
    );
}

#[test]
fn a_ledger_missing_a_row_is_a_named_error() {
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let mut v: serde_json::Value = serde_json::from_str(&text).expect("json");
    v.as_object_mut().expect("object").remove("mouth_gap");
    let err = crate::tolerances::load(&v.to_string()).expect_err("a missing row is refused");
    assert_eq!(err, crate::tolerances::LedgerError::MissingRow("mouth_gap"));
}

#[test]
fn a_ledger_row_that_stopped_separating_is_a_named_error() {
    // The whole point of the regime: move the number and the build
    // stops, naming the row and both populations.
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let mut v: serde_json::Value = serde_json::from_str(&text).expect("json");
    v["mouth_gap"]["radians"] = serde_json::json!(30.0 / 6371.0);
    let err = crate::tolerances::load(&v.to_string()).expect_err("refused");
    assert_eq!(
        err,
        crate::tolerances::LedgerError::Unlawful(vec![
            map_types::tolerance::ToleranceViolation::DoesNotSeparate {
                name: "mouth_gap",
                radians: 30.0 / 6371.0,
                same_max: 4.25 / 6371.0,
                distinct_min: 20.0 / 6371.0,
            }
        ])
    );
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p map-compile tolerance`
Expected: FAIL — `error[E0433]: failed to resolve: could not find 'tolerances' in the crate root`.

- [ ] **Step 3: Write the ledger**

`data/authored/tolerances.json`. Every row's `evidence` is the argument, and every row's `same_max` / `distinct_min` are the two populations the separation law (Task 3) re-measures. **Where the argument for a row is genuinely reconstructed rather than recorded — the four derived tolerances, whose original multipliers carry no written reason — say so in the evidence rather than inventing a history.** Populations are in radians on the unit sphere; `sliver_area` is in steradians and `completeness_residual` is a dimensionless area residual, and each says so.

```json
{
  "note": "THE TOLERANCE LEDGER. Spec §8 names Stage 3's snapping as the top risk and answers it 'tolerances declared, not tuned'. Every row here carries the two measured populations it separates (radians on the unit sphere, unless the row says otherwise), the multiplicative plateau across which the arrangement's content hash does not change, and the written argument. map-compile refuses a ledger that does not validate; map-compile's separation law re-measures every population from the real witness data; map-compile's plateau law sweeps every band. Changing any number here is an owner-reviewable diff with global reach.",
  "tau_vertex": {
    "radians": 1.2e-5,
    "same_max": 6.2e-6,
    "distinct_min": 4.4e-5,
    "plateau": { "lo": 0.6, "hi": 1.6 },
    "min_margin": 1.5,
    "evidence": "~76 m on Earth's radius. The two populations, measured over the real witness set by the separation law: (same) the largest gap between two candidate nodes that MUST merge — coincident endpoints where a tribal ring was snapped onto a shared shoreline or parent arc, whose residual disagreement after snapping tops out at ~40 m; (distinct) the smallest gap between two candidate nodes that must stay apart — the closest pair of genuinely distinct vertices anywhere in the arrangement, ~280 m. 76 m clears the first by 1.9x and is cleared by the second by 3.6x."
  },
  "tau_edge": {
    "radians": 2.4e-5,
    "same_max": 1.35e-5,
    "distinct_min": 8.8e-5,
    "plateau": { "lo": 0.6, "hi": 1.8 },
    "min_margin": 1.5,
    "evidence": "~153 m. An on-edge test needs more room than a vertex-merge test, because a vertex sits at a point while an arc is sampled along its length: the same tracing disagreement projects to a larger perpendicular offset mid-arc than it does at an endpoint. (same) the largest perpendicular offset of a node that must be treated as lying ON an arc — snapped ring points against their target arc, ~86 m; (distinct) the smallest perpendicular offset of a node that must stay off, ~560 m."
  },
  "tau_split_endpoint": {
    "radians": 6.0e-6,
    "same_max": 3.1e-6,
    "distinct_min": 2.4e-5,
    "plateau": { "lo": 0.5, "hi": 1.9 },
    "min_margin": 1.5,
    "evidence": "RECONSTRUCTED, not recorded: this is build.rs:296's `tau_vertex * 0.5`, whose 0.5 carried no written reason. The argument that justifies it: a split point closer to an endpoint than the vertex-merge radius would have been merged INTO that endpoint by clustering, so admitting it as a distinct split creates a zero-length edge that the topology validator then rejects. Half the vertex radius is the largest value that cannot collide with clustering; (distinct) is tau_edge, above which the point is not on the arc at all."
  },
  "tau_face_probe": {
    "radians": 4.8e-6,
    "same_max": 1.0e-7,
    "distinct_min": 6.0e-6,
    "plateau": { "lo": 0.4, "hi": 1.2 },
    "min_margin": 1.5,
    "evidence": "RECONSTRUCTED: build.rs:397's `(tau_vertex * 0.4).max(1e-7)`. The probe point sits just left of a cycle's first arc and must land strictly inside that cycle's face. (same) the floor below which the offset vanishes into f64 noise on a unit sphere, 1e-7; (distinct) tau_split_endpoint, above which the probe could cross into a neighbouring face at a narrow throat. The `.max(1e-7)` in the original expression IS the same_max bound, promoted here from an inline guard to a declared population."
  },
  "tau_river_attr": {
    "radians": 4.8e-5,
    "same_max": 2.4e-5,
    "distinct_min": 1.92e-4,
    "plateau": { "lo": 0.6, "hi": 2.4 },
    "min_margin": 1.5,
    "evidence": "RECONSTRUCTED: build.rs:695's `tau_edge * 2.0`, with a further `tol * 4.0` inside on_arc at :752 that this row absorbs (see tau_river_span). A border edge carries the river attribute when its midpoint lies this close to a river polyline. (same) tau_edge — a river running along a border is noded into the same vertices, so its midpoint offset is bounded by the on-edge tolerance; (distinct) the smallest midpoint distance from a NON-river border edge to any river polyline in the real data, ~1.2 km, which is the Jordan corridor's own bank separation."
  },
  "tau_degenerate": {
    "radians": 1e-12,
    "same_max": 1e-14,
    "distinct_min": 1e-7,
    "plateau": { "lo": 0.01, "hi": 100.0 },
    "min_margin": 10.0,
    "evidence": "The degenerate-cross-product floor, today a bare 1e-12 repeated in seven places. Below it a pair of points is the same point (or true antipodes) and no unique great-circle arc exists. (same) the f64 round-off floor of a normalized cross product, ~1e-14; (distinct) tau_face_probe's own floor, 1e-7, the smallest offset any geometry in this codebase deliberately constructs. Five orders of clearance on both sides is why this row's plateau is enormous and its margin requirement is 10x rather than 1.5x: this is a numerical guard, not a semantic threshold, and if it ever mattered to two decimal places something else is wrong."
  },
  "snap_budget": {
    "radians": 4.7088e-4,
    "same_max": 2.4e-4,
    "distinct_min": 1.5e-3,
    "plateau": { "lo": 0.7, "hi": 1.5 },
    "min_margin": 1.5,
    "evidence": "3 km on Earth's radius (partition_bridge.rs:162). How far a witness ring point may be moved onto a shared target arc so that the shared line exists exactly once and knife-edge parallels cannot form. (same) the vendored tribal trace's own declared accuracy — the affine fit over 12 detected city dots reports a mean residual of 1.6 km and a max of 2.8 km at surveys.rs:79-84, and 1.53 km is that max's half-width; (distinct) the smallest distance from a tribal ring vertex to a NON-adjacent target arc in the real data, ~9.6 km. A budget above that would drag a ring onto a shoreline it does not touch."
  },
  "mouth_gap": {
    "radians": 7.848e-4,
    "same_max": 6.671e-4,
    "distinct_min": 3.139e-3,
    "plateau": { "lo": 0.7, "hi": 2.5 },
    "min_margin": 1.1,
    "evidence": "5 km (partition_bridge.rs:215-222) — the regime's exemplar, promoted verbatim from the comment that already made this argument. OSM river lines can end where urban channels take over, short of the Natural Earth shoreline. (same) the largest observed gap for a sea-reaching river in the vendored data, the Yarkon's 4.25 km; (distinct) the nearest endorheic desert network's distance to any water ring, 20 km. This row's min_margin is 1.1 rather than 1.5 because BOTH populations are directly observed rather than derived, so a tighter bracket is honest here in a way it is not elsewhere."
  },
  "sliver_area": {
    "radians": 5.0e-8,
    "same_max": 1.1e-8,
    "distinct_min": 6.4e-7,
    "plateau": { "lo": 0.5, "hi": 2.0 },
    "min_margin": 1.5,
    "evidence": "STERADIANS, not radians — an area threshold, validated by the same two-population law in its own unit. Cells below it are tracing artifacts and are absorbed semantically into their longest-boundary neighbour. ~2.0 km². (same) the largest arrangement cell that MUST be absorbed — the biggest sliver produced where two witnesses trace the same coast at different densities, ~0.45 km²; (distinct) the smallest cell that must survive as itself — the smallest real face any witness claims, the Sea of Galilee's southern lobe at ~26 km²."
  },
  "completeness_residual": {
    "radians": 1e-10,
    "same_max": 4.0e-12,
    "distinct_min": 1.0e-8,
    "plateau": { "lo": 0.2, "hi": 10.0 },
    "min_margin": 5.0,
    "evidence": "DIMENSIONLESS steradian residual: the bound on |4π − Σ face areas| (equation 2's completeness law, lib.rs:500-503). (same) the observed residual of the real arrangement under Neumaier-compensated summation, which the law suite re-measures and which sits at ~1e-12; (distinct) the residual a single missing or double-counted face of the smallest survivable size would produce, which is sliver_area itself at 5e-8 — so 1e-8 is a conservative floor two orders below any real topology error. Between them there is nothing: this bound cannot be tuned because there is no phenomenon between 4e-12 and 1e-8 to tune it against, which is exactly what its five-order plateau records."
  }
}
```

**Note for the implementer, and it is the honest one:** the population numbers above are the *shape* of the argument and several are stated to a precision this plan has not measured. Task 3 measures every one of them from the real data and **its instruction is to correct the ledger to the measured values, not to bend the measurement to the ledger.** If a measured population lands where the declared value no longer separates with margin, that is a finding: stop, record it, and take it to the owner. Do not adjust the tolerance to make the test pass — that is tuning, wearing the regime's clothes.

- [ ] **Step 4: Write the loader**

`crates/map-compile/src/tolerances.rs`:

```rust
//! The tolerance ledger: data in, declared tolerances out, or a
//! NAMED refusal. Every row must parse, every row must obey its own
//! laws, and the set must obey the ordering laws between rows —
//! there is no partial load and no defaulting.

use map_types::tolerance::{Plateau, Separation, Tolerance, Tolerances, ToleranceViolation};
use serde_json::Value;

pub const DECLARED_PATH: &str = "data/authored/tolerances.json";

#[derive(Clone, Debug, PartialEq)]
pub enum LedgerError {
    BadJson(String),
    MissingRow(&'static str),
    MissingField { row: &'static str, field: &'static str },
    Unlawful(Vec<ToleranceViolation>),
}

fn row(v: &Value, name: &'static str) -> Result<Tolerance, LedgerError> {
    let r = v.get(name).ok_or(LedgerError::MissingRow(name))?;
    let f = |field: &'static str| -> Result<f64, LedgerError> {
        r.get(field)
            .and_then(Value::as_f64)
            .ok_or(LedgerError::MissingField { row: name, field })
    };
    let plateau = r.get("plateau").ok_or(LedgerError::MissingField { row: name, field: "plateau" })?;
    let p = |field: &'static str| -> Result<f64, LedgerError> {
        plateau
            .get(field)
            .and_then(Value::as_f64)
            .ok_or(LedgerError::MissingField { row: name, field })
    };
    Ok(Tolerance {
        name,
        radians: f("radians")?,
        separates: Separation { same_max: f("same_max")?, distinct_min: f("distinct_min")? },
        plateau: Plateau { lo: p("lo")?, hi: p("hi")? },
        min_margin: f("min_margin")?,
        evidence: r
            .get("evidence")
            .and_then(Value::as_str)
            .ok_or(LedgerError::MissingField { row: name, field: "evidence" })?
            .to_string(),
    })
}

pub fn load(text: &str) -> Result<Tolerances, LedgerError> {
    let v: Value = serde_json::from_str(text).map_err(|e| LedgerError::BadJson(e.to_string()))?;
    let t = Tolerances {
        tau_vertex: row(&v, "tau_vertex")?,
        tau_edge: row(&v, "tau_edge")?,
        tau_split_endpoint: row(&v, "tau_split_endpoint")?,
        tau_face_probe: row(&v, "tau_face_probe")?,
        tau_river_attr: row(&v, "tau_river_attr")?,
        tau_degenerate: row(&v, "tau_degenerate")?,
        snap_budget: row(&v, "snap_budget")?,
        mouth_gap: row(&v, "mouth_gap")?,
        sliver_area: row(&v, "sliver_area")?,
        completeness_residual: row(&v, "completeness_residual")?,
    };
    let violations = t.validate();
    if violations.is_empty() {
        Ok(t)
    } else {
        Err(LedgerError::Unlawful(violations))
    }
}
```

Add to `crates/map-compile/src/lib.rs`:

```rust
pub mod tolerances;
```

- [ ] **Step 5: Run the tests to green**

Run: `cargo test -p map-compile tolerance`
Expected: PASS, 6 tests. If `the_declared_ledger_obeys_every_tolerance_law` fails, a declared value does not separate its own populations as written — **fix the ledger's populations by measurement in Task 3, not by moving the value.**

- [ ] **Step 6: Commit**

```bash
git add data/authored/tolerances.json crates/map-compile/src/tolerances.rs crates/map-compile/src/lib.rs crates/map-compile/src/tests.rs
git commit -m "Stage 3 Task 2: the tolerance ledger — knobs as reviewable data, one row, global reach"
```

---

### Task 3: The separation law — re-measure both populations from the real witness data

Task 2's ledger states its populations. This task makes them *measurements* rather than claims. It is the half of "declared" that says the number is in the right place; Task 4 is the half that says the number is not doing the work.

**Files:**
- Modify: `crates/map-compile/src/tolerances.rs` (add the measurement functions)
- Test: `crates/map-compile/src/tests.rs` (append)
- Modify: `data/authored/tolerances.json` (correct the populations to what is measured — expected, and the point of the task)

**Interfaces:**
- Consumes: `map_compile::tolerances::load` (Task 2); `map_compile::partition_bridge::gather_witnesses` (already `pub` at `crates/map-compile/src/partition_bridge.rs:40`, exactly so "the law suite builds what the compiler builds").
- Produces: `map_compile::tolerances::measure(&[WitnessRegion], &[WitnessPolyline]) -> Measured`; `Measured { tau_vertex: (f64, f64), tau_edge: (f64, f64), snap_budget: (f64, f64), mouth_gap: (f64, f64), sliver_area: (f64, f64), .. }` — one `(same_max, distinct_min)` pair per measurable row. Task 4 reuses `gather_witnesses`; Task 12 cites `Measured` in the closing report.

- [ ] **Step 1: Write the failing tests**

Append to `crates/map-compile/src/tests.rs`:

```rust
// ------------------------------------------- the separation law
//
// A declared tolerance names two populations. This law goes and
// MEASURES them, in the real witness set the compiler itself builds,
// and asserts three things: the ledger's declared populations match
// what is out there, the value brackets them, and the margins hold.
//
// The negative case is the load-bearing one. A law that merely
// re-derives numbers from data and compares them to numbers written
// down from the same data would pass no matter what the value was;
// what makes this a check is that it can be made to fail by moving
// the VALUE while leaving the populations alone.

fn measured_witnesses() -> (Vec<map_partition::WitnessRegion>, Vec<map_partition::WitnessPolyline>)
{
    // the plate core alone: the same witness set the arrangement's
    // heart is built from, independent of the bordering world
    crate::partition_bridge::gather_witnesses(&[]).expect("witnesses gather")
}

#[test]
fn every_declared_population_matches_the_measurement() {
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let declared = crate::tolerances::load(&text).expect("ledger loads");
    let (regions, polylines) = measured_witnesses();
    let m = crate::tolerances::measure(&regions, &polylines);

    // Declared populations are recorded to the precision the ledger
    // states; the law allows 5% drift before it calls the ledger
    // stale, and NAMES the row and both numbers when it does.
    let check = |name: &str, declared: (f64, f64), found: (f64, f64)| {
        for (side, d, f) in [("same_max", declared.0, found.0), ("distinct_min", declared.1, found.1)]
        {
            assert!(
                (d - f).abs() <= 0.05 * f.abs().max(f64::MIN_POSITIVE),
                "{name}.{side}: ledger says {d:e}, the data says {f:e} — \
                 re-measure the ledger, do not move the tolerance"
            );
        }
    };
    check(
        "tau_vertex",
        (declared.tau_vertex.separates.same_max, declared.tau_vertex.separates.distinct_min),
        m.tau_vertex,
    );
    check(
        "tau_edge",
        (declared.tau_edge.separates.same_max, declared.tau_edge.separates.distinct_min),
        m.tau_edge,
    );
    check(
        "snap_budget",
        (declared.snap_budget.separates.same_max, declared.snap_budget.separates.distinct_min),
        m.snap_budget,
    );
    check(
        "mouth_gap",
        (declared.mouth_gap.separates.same_max, declared.mouth_gap.separates.distinct_min),
        m.mouth_gap,
    );
    check(
        "sliver_area",
        (declared.sliver_area.separates.same_max, declared.sliver_area.separates.distinct_min),
        m.sliver_area,
    );
}

#[test]
fn the_measured_populations_bracket_every_declared_value_with_margin() {
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let declared = crate::tolerances::load(&text).expect("ledger loads");
    let (regions, polylines) = measured_witnesses();
    let m = crate::tolerances::measure(&regions, &polylines);
    // Re-run each row's OWN law against the measured populations
    // rather than the declared ones: this is the assertion that
    // could not be satisfied by writing convenient numbers down.
    for (t, found) in [
        (&declared.tau_vertex, m.tau_vertex),
        (&declared.tau_edge, m.tau_edge),
        (&declared.snap_budget, m.snap_budget),
        (&declared.mouth_gap, m.mouth_gap),
        (&declared.sliver_area, m.sliver_area),
    ] {
        let mut as_measured = t.clone();
        as_measured.separates =
            map_types::tolerance::Separation { same_max: found.0, distinct_min: found.1 };
        assert_eq!(
            as_measured.validate(),
            Vec::<map_types::tolerance::ToleranceViolation>::new(),
            "{} does not separate the populations actually present in the data",
            t.name
        );
    }
}

#[test]
fn moving_a_value_off_its_measured_bracket_fails_the_law() {
    // THE NEGATIVE CASE. Leave the data alone, move the number, and
    // the law must go red naming the row. Without this, everything
    // above is a tautology over numbers copied from the same source.
    let (regions, polylines) = measured_witnesses();
    let m = crate::tolerances::measure(&regions, &polylines);
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let declared = crate::tolerances::load(&text).expect("ledger loads");

    let mut tuned = declared.mouth_gap.clone();
    tuned.separates =
        map_types::tolerance::Separation { same_max: m.mouth_gap.0, distinct_min: m.mouth_gap.1 };
    tuned.radians = m.mouth_gap.1 * 1.5; // above the endorheic population
    assert_eq!(
        tuned.validate(),
        vec![map_types::tolerance::ToleranceViolation::DoesNotSeparate {
            name: "mouth_gap",
            radians: m.mouth_gap.1 * 1.5,
            same_max: m.mouth_gap.0,
            distinct_min: m.mouth_gap.1,
        }]
    );

    let mut wedged = declared.snap_budget.clone();
    wedged.separates =
        map_types::tolerance::Separation { same_max: m.snap_budget.0, distinct_min: m.snap_budget.1 };
    wedged.radians = m.snap_budget.0 * 1.01; // inside the bracket, no room
    let v = wedged.validate();
    assert_eq!(v.len(), 1, "exactly one violation: {v:?}");
    assert!(matches!(
        v[0],
        map_types::tolerance::ToleranceViolation::NoMargin { name: "snap_budget", side: "same", .. }
    ));
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p map-compile separation -- --nocapture` (and `cargo test -p map-compile population`)
Expected: FAIL — `no function 'measure' in module 'tolerances'`.

- [ ] **Step 3: Write the measurement functions**

Append to `crates/map-compile/src/tolerances.rs`. Each measurement answers exactly the question its ledger row's evidence claims to answer, and each prints its finding so the implementer can transcribe corrected numbers back into the ledger.

```rust
use map_partition::{cycle_area, WitnessPolyline, WitnessRegion};
use map_types::UnitVec;

/// The two populations, measured from the real witness set. Each
/// pair is (same_max, distinct_min) in the row's own unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measured {
    pub tau_vertex: (f64, f64),
    pub tau_edge: (f64, f64),
    pub snap_budget: (f64, f64),
    pub mouth_gap: (f64, f64),
    /// steradians
    pub sliver_area: (f64, f64),
}

/// Perpendicular angular offset of p from the great-circle arc a→b,
/// or None when p's foot falls outside the arc.
fn offset_from_arc(p: &UnitVec, a: &UnitVec, b: &UnitVec) -> Option<f64> {
    let (nx, ny, nz) = a.cross_raw(b);
    let nn = (nx * nx + ny * ny + nz * nz).sqrt();
    if nn < 1e-14 {
        return None;
    }
    let off = ((p.x() * nx + p.y() * ny + p.z() * nz) / nn).abs().asin();
    let full = a.angle_to(b);
    let within = (p.angle_to(a) * p.angle_to(a) - off * off).max(0.0).sqrt()
        + (p.angle_to(b) * p.angle_to(b) - off * off).max(0.0).sqrt()
        <= full + 1e-9;
    if within {
        Some(off)
    } else {
        None
    }
}

/// THE MEASUREMENT. Two populations per row, both derived from the
/// witness set the compiler itself builds — never from constants.
pub fn measure(regions: &[WitnessRegion], polylines: &[WitnessPolyline]) -> Measured {
    let all_pts: Vec<(usize, UnitVec)> = regions
        .iter()
        .enumerate()
        .flat_map(|(i, r)| r.rings.iter().flatten().map(move |p| (i, *p)))
        .collect();

    // tau_vertex. SAME: two points from DIFFERENT witnesses that are
    // meant to be one node — the residual disagreement left after
    // vendor-time snapping, so the largest such near-coincidence.
    // DISTINCT: the closest pair of points from the SAME witness ring
    // that are consecutive-but-one, i.e. genuinely different vertices
    // the arrangement must keep apart.
    let mut cross_witness_near = 0.0f64;
    for i in 0..all_pts.len() {
        for j in (i + 1)..all_pts.len() {
            if all_pts[i].0 == all_pts[j].0 {
                continue;
            }
            let d = all_pts[i].1.angle_to(&all_pts[j].1);
            // "meant to be one node" = already essentially coincident;
            // anything beyond a kilometre is two places, not a snap
            // residual, and would poison the population
            if d < 1.6e-4 && d > cross_witness_near {
                cross_witness_near = d;
            }
        }
    }
    let mut same_ring_min = f64::INFINITY;
    for r in regions {
        for ring in &r.rings {
            let n = ring.len();
            for k in 0..n {
                if n < 3 {
                    continue;
                }
                let d = ring[k].angle_to(&ring[(k + 2) % n]);
                if d > 0.0 {
                    same_ring_min = same_ring_min.min(d);
                }
            }
        }
    }

    // tau_edge. SAME: the largest perpendicular offset of a witness
    // point from ANOTHER witness's arc, among points close enough
    // that they are tracing the same line. DISTINCT: the smallest
    // such offset among points that are NOT on the other's line —
    // taken as the smallest offset above the same population's band.
    let mut on_line_max = 0.0f64;
    let mut off_line_min = f64::INFINITY;
    for (wi, r) in regions.iter().enumerate() {
        for ring in &r.rings {
            for p in ring {
                for (wj, s) in regions.iter().enumerate() {
                    if wi == wj {
                        continue;
                    }
                    for other in &s.rings {
                        let m = other.len();
                        for k in 0..m {
                            let Some(off) = offset_from_arc(p, &other[k], &other[(k + 1) % m])
                            else {
                                continue;
                            };
                            if off < 3.2e-4 {
                                on_line_max = on_line_max.max(off);
                            } else if off < off_line_min {
                                off_line_min = off;
                            }
                        }
                    }
                }
            }
        }
    }

    // snap_budget. SAME: the vendored trace's own declared accuracy,
    // read from the data file that declares it rather than from a
    // comment. DISTINCT: the smallest distance from a tribal ring
    // vertex to a target arc it must NOT be dragged onto.
    let trace_accuracy_km = declared_trace_accuracy_km().unwrap_or(2.8);
    let snap_same = (trace_accuracy_km / 2.0) / 6371.0;
    let mut snap_distinct = f64::INFINITY;
    for (wi, r) in regions.iter().enumerate() {
        for ring in &r.rings {
            for p in ring {
                for (wj, s) in regions.iter().enumerate() {
                    if wi == wj {
                        continue;
                    }
                    for other in &s.rings {
                        let m = other.len();
                        for k in 0..m {
                            if let Some(off) = offset_from_arc(p, &other[k], &other[(k + 1) % m]) {
                                if off > snap_same * 1.5 && off < snap_distinct {
                                    snap_distinct = off;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // mouth_gap. SAME: the largest endpoint-to-water gap among river
    // paths that DO reach the map's water. DISTINCT: the smallest
    // endpoint-to-water gap among networks that do not.
    let waters: Vec<&Vec<UnitVec>> = regions
        .iter()
        .filter(|r| matches!(r.kind, map_partition::FaceKind::Sea | map_partition::FaceKind::Lake))
        .flat_map(|r| r.rings.iter())
        .collect();
    let gap_of = |pl: &WitnessPolyline| -> f64 {
        let mut best = f64::INFINITY;
        for p in [pl.pts.first(), pl.pts.last()].into_iter().flatten() {
            for ring in &waters {
                let m = ring.len();
                for k in 0..m {
                    let (a, b) = (ring[k], ring[(k + 1) % m]);
                    let d = offset_from_arc(p, &a, &b)
                        .unwrap_or_else(|| p.angle_to(&a).min(p.angle_to(&b)));
                    best = best.min(d);
                }
            }
        }
        best
    };
    let mut gaps: Vec<f64> = polylines.iter().map(gap_of).filter(|g| g.is_finite()).collect();
    gaps.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    // the reaching population is everything under 2 km; the first gap
    // above 8 km opens the non-reaching population. Both cut points
    // are an order below and above the declared value respectively,
    // so neither can define it.
    let reach_max = gaps.iter().copied().filter(|g| *g < 2.0 / 6371.0).fold(0.0, f64::max);
    let strand_min = gaps
        .iter()
        .copied()
        .find(|g| *g > 8.0 / 6371.0)
        .unwrap_or(f64::INFINITY);

    // sliver_area. SAME: the largest ring area among witness rings
    // that are tracing artifacts (duplicate coast fragments).
    // DISTINCT: the smallest ring area among rings that name a real
    // place.
    let mut areas: Vec<f64> =
        regions.iter().flat_map(|r| r.rings.iter()).map(|r| cycle_area(r).abs()).collect();
    areas.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    let sliver_same = areas.iter().copied().filter(|a| *a < 5.0e-8).fold(0.0, f64::max);
    let sliver_distinct = areas.iter().copied().find(|a| *a > 5.0e-8).unwrap_or(f64::INFINITY);

    let m = Measured {
        tau_vertex: (cross_witness_near, same_ring_min),
        tau_edge: (on_line_max, off_line_min),
        snap_budget: (snap_same, snap_distinct),
        mouth_gap: (reach_max, strand_min),
        sliver_area: (sliver_same, sliver_distinct),
    };
    eprintln!("MEASURED POPULATIONS (transcribe into the ledger): {m:#?}");
    m
}

/// The vendored tribal trace declares its own accuracy in the file it
/// ships in; read it there rather than repeating it here. Returns the
/// max residual in km.
fn declared_trace_accuracy_km() -> Option<f64> {
    let text = std::fs::read_to_string(super::partition_bridge::vendor_path(
        "data/wikimedia/tribes12.geojson",
    ))
    .ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("trace_accuracy_km_max").and_then(serde_json::Value::as_f64)
}
```

`vendor_path` is `partition_bridge`'s existing `data_path` helper (`crates/map-compile/src/partition_bridge.rs:726-732`), which resolves repo-relative paths from both the binary and the test harness. Make it `pub(crate) fn vendor_path` — rename in place and update its four call sites in that file, or add `pub(crate) fn vendor_path(rel: &str) -> std::path::PathBuf { data_path(rel) }`; either is fine, the second is smaller.

- [ ] **Step 4: Run, read the measurements, correct the ledger**

Run: `cargo test -p map-compile every_declared_population -- --nocapture`

The `MEASURED POPULATIONS` line prints the real numbers. **Transcribe them into `data/authored/tolerances.json`**, replacing the `same_max` / `distinct_min` of `tau_vertex`, `tau_edge`, `snap_budget`, `mouth_gap`, `sliver_area`, and update each row's `evidence` prose so the sentence matches the number. Leave `radians` alone.

If `data/wikimedia/tribes12.geojson` has no `trace_accuracy_km_max` key, add one whose value is the max residual already stated in `crates/map-adapters/src/surveys.rs:79-84` (2.8), so the number lives in the data it describes rather than in a Rust comment. That is a one-line data edit and belongs in this commit.

- [ ] **Step 5: Run the three laws to green**

Run: `cargo test -p map-compile -- separation bracket population moving_a_value`
Expected: PASS.

**If `the_measured_populations_bracket_every_declared_value_with_margin` still fails after honest transcription, STOP.** That is the regime working: a tolerance in the codebase does not, in fact, separate the populations it is supposed to. Record which row, both measured numbers, and the value, and take it to the owner. Do not move `radians` to make it pass.

- [ ] **Step 6: Commit**

```bash
git add crates/map-compile/src/tolerances.rs crates/map-compile/src/tests.rs crates/map-compile/src/partition_bridge.rs data/authored/tolerances.json data/wikimedia/tribes12.geojson
git commit -m "Stage 3 Task 3: the separation law — populations measured from the data, not written down"
```

---

### Task 4: The plateau law — proving the number is not doing the work

This is the task that makes "declared, not tuned" a claim with teeth rather than a claim with footnotes. Task 3 argues each value sits in the right place. This one proves the exact value does not matter: the arrangement's own content hash (`crates/map-partition/src/lib.rs:507-542` — order-, winding- and construction-path-independent by design, which is precisely why it is the right instrument) must be **identical** across every factor in the declared plateau.

A tuned constant is one whose exact value changes the answer. If the answer is invariant across a 0.6×–1.8× band, there was nothing to tune it to.

**Files:**
- Test: `crates/map-compile/src/tests.rs` (append)
- Modify: `data/authored/tolerances.json` (correct plateaus to what is measured)

**Interfaces:**
- Consumes: `map_compile::tolerances::load` (Task 2); `gather_witnesses`; `map_partition::build_with`; `Tolerance::plateau_samples` (Task 1).
- Produces: nothing new on the interface — it produces *confidence*, and a plateau column in the ledger that later stages can trust. Task 5 relies on it: retiring the derived multipliers is safe exactly because their values sit inside plateaus.

- [ ] **Step 1: Write the failing test**

Append to `crates/map-compile/src/tests.rs`:

```rust
// ------------------------------------------------- the plateau law
//
// The half of "declared, not tuned" that cannot be faked by prose.
// Sweep each tolerance across its declared band and require the
// arrangement's content hash to be BIT-IDENTICAL throughout. A
// number that can be moved 60% in either direction without changing
// one edge of the answer was not tuned to anything; a number whose
// plateau is a point is a tuned constant however it is documented.
//
// This test is slow by nature — it rebuilds the arrangement once per
// sample per row. It is marked #[ignore] and run explicitly, by the
// task that changes a tolerance and by Task 12's closing sweep.

fn arrangement_hash_with(t: &map_types::tolerance::Tolerances) -> u64 {
    let (regions, polylines) = crate::partition_bridge::gather_witnesses(&[])
        .expect("witnesses gather");
    let cfg = map_partition::PartitionConfig::from_tolerances(t);
    map_partition::build(&regions, &polylines, &cfg)
        .expect("the arrangement builds")
        .content_hash()
}

#[test]
#[ignore = "rebuilds the arrangement ~40 times; run explicitly"]
fn every_tolerance_sits_inside_a_plateau_where_the_answer_does_not_move() {
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let base = crate::tolerances::load(&text).expect("ledger loads");
    let want = arrangement_hash_with(&base);

    // The four rows that reach the arrangement builder. snap_budget,
    // mouth_gap and sliver_area are swept by their own tests below,
    // because they act on the witness set rather than on build().
    let rows: [(&str, fn(&mut map_types::tolerance::Tolerances) -> &mut map_types::tolerance::Tolerance); 4] = [
        ("tau_vertex", |t| &mut t.tau_vertex),
        ("tau_edge", |t| &mut t.tau_edge),
        ("tau_split_endpoint", |t| &mut t.tau_split_endpoint),
        ("tau_face_probe", |t| &mut t.tau_face_probe),
    ];
    for (name, pick) in rows {
        let samples = pick(&mut base.clone()).plateau_samples();
        for f in samples {
            let mut trial = base.clone();
            let row = pick(&mut trial);
            row.radians *= f;
            let got = arrangement_hash_with(&trial);
            assert_eq!(
                got, want,
                "{name} at {f}x changed the arrangement ({got:016x} != {want:016x}) — \
                 the declared plateau is wrong, or this tolerance is TUNED"
            );
        }
    }
}

#[test]
#[ignore = "rebuilds the arrangement; run explicitly"]
fn a_tolerance_outside_its_plateau_does_change_the_answer() {
    // THE NEGATIVE CASE, and without it the law above is worthless:
    // a build that ignored the config entirely would satisfy every
    // assertion in it. Far outside the band, the hash MUST move.
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let base = crate::tolerances::load(&text).expect("ledger loads");
    let want = arrangement_hash_with(&base);
    let mut coarse = base.clone();
    // 40x the vertex-merge radius: ~3 km, far above the smallest
    // distance between genuinely distinct vertices measured in
    // Task 3, so real geometry MUST collapse.
    coarse.tau_vertex.radians *= 40.0;
    coarse.tau_edge.radians *= 40.0;
    coarse.tau_split_endpoint.radians *= 40.0;
    let got = arrangement_hash_with(&coarse);
    assert_ne!(
        got, want,
        "40x the snapping radii left the arrangement bit-identical — \
         the config is not reaching the builder, and the plateau law \
         above is measuring nothing"
    );
}

#[test]
#[ignore = "rebuilds the witness set; run explicitly"]
fn the_witness_side_tolerances_sit_inside_their_plateaus_too() {
    // snap_budget, mouth_gap and sliver_area shape the WITNESS set,
    // upstream of build(). Their plateau is measured on the witness
    // set's own digest: same rings, same polylines, same ids.
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let base = crate::tolerances::load(&text).expect("ledger loads");
    let digest = |t: &map_types::tolerance::Tolerances| -> u64 {
        let (regions, polylines) =
            crate::partition_bridge::gather_witnesses_with(&[], t).expect("witnesses gather");
        let mut h = map_partition::Fnv::new();
        for r in &regions {
            h.bytes(r.id.as_bytes());
            for ring in &r.rings {
                h.u64(ring.len() as u64);
                for p in ring {
                    h.u64(((p.x() * 1e9).round() as i64) as u64);
                    h.u64(((p.y() * 1e9).round() as i64) as u64);
                    h.u64(((p.z() * 1e9).round() as i64) as u64);
                }
            }
        }
        for pl in &polylines {
            h.bytes(pl.id.as_bytes());
            h.u64(pl.pts.len() as u64);
        }
        h.finish()
    };
    let want = digest(&base);
    let rows: [(&str, fn(&mut map_types::tolerance::Tolerances) -> &mut map_types::tolerance::Tolerance); 2] = [
        ("snap_budget", |t| &mut t.snap_budget),
        ("mouth_gap", |t| &mut t.mouth_gap),
    ];
    for (name, pick) in rows {
        for f in pick(&mut base.clone()).plateau_samples() {
            let mut trial = base.clone();
            pick(&mut trial).radians *= f;
            assert_eq!(digest(&trial), want, "{name} at {f}x changed the witness set");
        }
    }
}
```

- [ ] **Step 2: Run and watch it fail**

Run: `cargo test -p map-compile plateau -- --ignored --nocapture`
Expected: FAIL — `no function 'from_tolerances' for 'PartitionConfig'`, and `no function 'gather_witnesses_with'`.

- [ ] **Step 3: Thread the tolerances through**

In `crates/map-partition/src/lib.rs`, beside `PartitionConfig`:

```rust
impl PartitionConfig {
    /// The declared ledger, in the shape the builder consumes. There
    /// is no other constructor for a non-default config: a caller
    /// cannot invent a tolerance, only carry a declared one.
    pub fn from_tolerances(t: &map_types::tolerance::Tolerances) -> Self {
        PartitionConfig {
            tau_vertex: t.tau_vertex.radians(),
            tau_edge: t.tau_edge.radians(),
            tau_split_endpoint: t.tau_split_endpoint.radians(),
            tau_face_probe: t.tau_face_probe.radians(),
            tau_river_attr: t.tau_river_attr.radians(),
            tau_degenerate: t.tau_degenerate.radians(),
            sliver_area: t.sliver_area.radians(),
            completeness_residual: t.completeness_residual.radians(),
        }
    }
}
```

and widen `PartitionConfig` itself to those eight fields (the four new ones are the derived tolerances Task 5 will consume; declaring them now, unused, keeps this task's diff to threading only — `#[allow(dead_code)]` is not needed because `from_tolerances` reads them).

`Default for PartitionConfig` stays exactly as it is for now, with the four new fields set to today's derived expressions (`0.5 * 1.2e-5`, `(0.4 * 1.2e-5).max(1e-7)`, `2.0 * 2.4e-5`, `1e-12`, `1e-10`) so no behaviour changes; Task 5 deletes `Default` outright.

In `crates/map-compile/src/partition_bridge.rs`, split `gather_witnesses` into a tolerance-taking form and keep the old signature as a thin wrapper that loads the declared ledger:

```rust
/// Assemble the partition's witnesses under a DECLARED tolerance set.
/// Public so the law suite builds exactly what the compiler builds —
/// and so the plateau law can sweep the band.
pub fn gather_witnesses_with(
    polities: &[PolityRow],
    tol: &map_types::tolerance::Tolerances,
) -> Result<(Vec<WitnessRegion>, Vec<WitnessPolyline>), String> {
    /* today's body, with these three substitutions and nothing else:
         PartitionConfig::default().tau_edge   -> tol.tau_edge.radians()      (line 85)
         MOUTH_GAP                             -> tol.mouth_gap.radians()     (line 88)
         let budget = 3.0 / 6371.0;            -> tol.snap_budget.radians()   (line 162)
       delete the `const MOUTH_GAP` at :215-222; its argument now lives
       in the ledger row's `evidence`, which is where the owner reads it. */
}

pub fn gather_witnesses(
    polities: &[PolityRow],
) -> Result<(Vec<WitnessRegion>, Vec<WitnessPolyline>), String> {
    let text = std::fs::read_to_string(data_path(crate::tolerances::DECLARED_PATH))
        .map_err(|e| format!("tolerance ledger: {e}"))?;
    let tol = crate::tolerances::load(&text).map_err(|e| format!("tolerance ledger: {e:?}"))?;
    gather_witnesses_with(polities, &tol)
}
```

and make `bridge_partition` (`:479-487`) load the ledger once and pass `PartitionConfig::from_tolerances(&tol)` to `build`, replacing `PartitionConfig::default()`.

- [ ] **Step 4: Run the plateau sweep and correct the declared bands**

Run: `cargo test -p map-compile plateau -- --ignored --nocapture`

Expect the first run to fail on at least one row: the plateau widths in Task 2's ledger are estimates. For each failing row, **narrow the declared band to the widest factor range that actually holds** — bisect if you like, but a handful of samples is enough — and update the row's `plateau`. That is measurement, not tuning: the band is an observation about the answer's stability, and the value is not being moved.

**If a row's honest plateau collapses to a point** (any factor other than 1.0 changes the hash), STOP. That row is a tuned constant, and the finding is exactly what Part A exists to surface. Record the row, the smallest factor that moved the hash, and how the hash moved, and take it to the owner before proceeding. Do not widen the band to make the test pass, and do not delete the row from the sweep.

- [ ] **Step 5: Confirm the negative case genuinely fails when it should**

Run: `cargo test -p map-compile a_tolerance_outside -- --ignored --nocapture`
Expected: PASS (i.e. the hash *did* move at 40×). If this test fails — the hash did not move at 40× the snapping radii — then the config is not reaching the builder and **every other assertion in this task is vacuous**. Fix the threading before going on; this is diagnosis §7.0's lesson in its exact original shape.

- [ ] **Step 6: Golden gate — nothing may have moved**

Part A changes no coordinate. Rebuild all three steps, then:

Run: `node crates/map-viewer/tests/golden.js --check`
Expected: `ALL GOLDEN VIEWS HOLD` (89/89). **Any drift here is a bug in the threading, not a visual change to approve.**

- [ ] **Step 7: Commit**

```bash
git add crates/map-partition/src/lib.rs crates/map-compile/src/partition_bridge.rs crates/map-compile/src/tests.rs data/authored/tolerances.json
git commit -m "Stage 3 Task 4: the plateau law — the answer is invariant across the declared band"
```

---

### Task 5: Retire the undeclared derived tolerances, and the law that stops the regime decaying

Part A is worth nothing if the next magic number walks straight back in. `build.rs` derives four tolerances by unexplained multipliers and guards seven places with a bare `1e-12`; `snap_ring_to` carries `full * 0.02 + 1e-9` and a bare `60.0` span cap. This task moves each into the ledger and then installs a law that fails on the next bare float in the geometry modules.

**Files:**
- Modify: `crates/map-partition/src/build.rs` (`:296`, `:397`, `:695`, `:752`, the `1e-12` guards, the `1e-9` on-arc slack)
- Modify: `crates/map-partition/src/lib.rs` (`:501` residual bound; delete `impl Default for PartitionConfig`)
- Modify: `crates/map-compile/src/partition_bridge.rs` (`snap_ring_to`'s `full * 0.02 + 1e-9` at `:760`, the `60.0` cap at `:814`, the `1e-9` cleanups at `:836`/`:840`)
- Create: `crates/map-partition/src/literals.rs`
- Test: `crates/map-partition/src/tests.rs` (append)

**Interfaces:**
- Consumes: `PartitionConfig::from_tolerances` (Task 4).
- Produces: `map_partition::literals::{scan, Literal, ALLOWED}` — `scan(&str) -> Vec<Literal>` finds float literals in Rust source outside comments and strings. Task 12's closing sweep re-runs it over the whole geometry surface.

- [ ] **Step 1: Write the failing tests**

Append to `crates/map-partition/src/tests.rs`:

```rust
// -------------------------------------- no bare floats in geometry
//
// Part A moved the arrangement's tolerances into a declared ledger.
// This law is what stops the next one walking back in: the geometry
// modules may contain only the literals on a short declared list,
// and every other float must come from PartitionConfig.
//
// The scanner is tested BEFORE it is trusted — a scanner that found
// nothing would pass the law vacuously, which is exactly the shape
// of defect diagnosis §7.0 records.

use crate::literals::{scan, Literal};

#[test]
fn the_scanner_finds_floats_and_ignores_comments_and_strings() {
    let src = r#"
        // a comment with 3.5 in it
        /* and a block one with 0.25 */
        let msg = "a string with 9.75 inside";
        let a = 1.5;
        let b = 2.0e-3;
        let c = 1_000.0;
        let d = 4;          // an integer is not a float literal
    "#;
    assert_eq!(
        scan(src),
        vec![
            Literal { text: "1.5".into(), line: 5 },
            Literal { text: "2.0e-3".into(), line: 6 },
            Literal { text: "1_000.0".into(), line: 7 },
        ]
    );
}

#[test]
fn the_scanner_can_fail_a_clean_file() {
    // the negative case for the scanner itself
    assert_eq!(scan("let x = cfg.tau_vertex;\n"), Vec::<Literal>::new());
}

#[test]
fn the_geometry_modules_contain_no_undeclared_float() {
    // ALLOWED is deliberately tiny and each entry carries its reason
    // in literals.rs. Anything else is a tolerance in disguise and
    // belongs in data/authored/tolerances.json.
    for (name, src) in [
        ("build.rs", include_str!("build.rs")),
        ("lib.rs", include_str!("lib.rs")),
    ] {
        let stray: Vec<Literal> = scan(src)
            .into_iter()
            .filter(|l| !crate::literals::ALLOWED.contains(&l.text.as_str()))
            .collect();
        assert_eq!(
            stray,
            Vec::<Literal>::new(),
            "{name} carries undeclared float literals — each is a tolerance \
             wearing a disguise; declare it in data/authored/tolerances.json"
        );
    }
}
```

- [ ] **Step 2: Run and watch them fail**

Run: `cargo test -p map-partition literal -- --nocapture`
Expected: FAIL — `could not find 'literals' in the crate root`. After Step 3 the third test will still fail, listing every stray literal in `build.rs`; that list is Step 4's worklist.

- [ ] **Step 3: Write the scanner**

`crates/map-partition/src/literals.rs`:

```rust
//! THE LAW THAT KEEPS THE REGIME HONEST. Part A declared every
//! tolerance the arrangement runs under; this module makes the next
//! undeclared one a failing test rather than a code review's luck.
//!
//! A float literal in a geometry module is, almost always, a
//! tolerance nobody argued for. The allowlist below is deliberately
//! tiny, and every entry carries the reason it is not a tolerance.

/// One float literal found in source, with the line it sits on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Literal {
    pub text: String,
    pub line: usize,
}

/// Literals that are NOT tolerances, each with its reason:
/// - "0.0"/"1.0"/"2.0": additive and multiplicative identities and
///   the doubling in `2.0 * det.atan2(..)` — the spherical excess
///   formula's own constant, not a threshold;
/// - "0.5": the midpoint in `slerp`-free bisections, a geometric
///   halving, not a threshold;
/// - "4.0": the 4π of the completeness law, which is mathematics;
/// - "1.0e9"/"1e9": the content-hash quantization grid, a hashing
///   detail that no geometric decision reads;
/// - "6371.0": Earth's radius in km, a unit conversion for human
///   readable diagnostics only.
pub const ALLOWED: &[&str] = &["0.0", "1.0", "2.0", "0.5", "4.0", "1.0e9", "1e9", "6371.0"];

/// Every float literal in `src` outside comments and string literals,
/// in source order.
pub fn scan(src: &str) -> Vec<Literal> {
    let mut out = Vec::new();
    let mut in_block = false;
    for (i, raw) in src.lines().enumerate() {
        let line = i + 1;
        let mut chars: Vec<char> = Vec::new();
        let bytes: Vec<char> = raw.chars().collect();
        let mut k = 0usize;
        let mut in_str = false;
        while k < bytes.len() {
            let c = bytes[k];
            let next = bytes.get(k + 1).copied();
            if in_block {
                if c == '*' && next == Some('/') {
                    in_block = false;
                    k += 2;
                    continue;
                }
                k += 1;
                continue;
            }
            if in_str {
                if c == '\\' {
                    k += 2;
                    continue;
                }
                if c == '"' {
                    in_str = false;
                }
                k += 1;
                continue;
            }
            if c == '/' && next == Some('/') {
                break;
            }
            if c == '/' && next == Some('*') {
                in_block = true;
                k += 2;
                continue;
            }
            if c == '"' {
                in_str = true;
                k += 1;
                continue;
            }
            chars.push(c);
            k += 1;
        }
        let code: String = chars.into_iter().collect();
        let cs: Vec<char> = code.chars().collect();
        let mut j = 0usize;
        while j < cs.len() {
            if !(cs[j].is_ascii_digit()
                && (j == 0 || !(cs[j - 1].is_alphanumeric() || cs[j - 1] == '_' || cs[j - 1] == '.')))
            {
                j += 1;
                continue;
            }
            let start = j;
            while j < cs.len() && (cs[j].is_ascii_digit() || cs[j] == '_') {
                j += 1;
            }
            let mut is_float = false;
            if j < cs.len() && cs[j] == '.' && cs.get(j + 1).is_some_and(|c| c.is_ascii_digit()) {
                is_float = true;
                j += 1;
                while j < cs.len() && (cs[j].is_ascii_digit() || cs[j] == '_') {
                    j += 1;
                }
            }
            if j < cs.len() && (cs[j] == 'e' || cs[j] == 'E') {
                let mut p = j + 1;
                if cs.get(p).is_some_and(|c| *c == '+' || *c == '-') {
                    p += 1;
                }
                if cs.get(p).is_some_and(char::is_ascii_digit) {
                    is_float = true;
                    j = p;
                    while j < cs.len() && cs[j].is_ascii_digit() {
                        j += 1;
                    }
                }
            }
            if is_float {
                out.push(Literal { text: cs[start..j].iter().collect(), line });
            }
        }
    }
    out
}
```

Add `pub mod literals;` to `crates/map-partition/src/lib.rs`.

- [ ] **Step 4: Retire every stray literal the law names**

Work down the failing test's list. The substitutions, each replacing a multiplier with a declared row:

| site | today | becomes |
|---|---|---|
| `build.rs:296` | `sa > cfg.tau_vertex * 0.5 && sb > cfg.tau_vertex * 0.5` | `sa > cfg.tau_split_endpoint && sb > cfg.tau_split_endpoint` |
| `build.rs:397` | `let eps = (cfg.tau_vertex * 0.4).max(1e-7);` | `let eps = cfg.tau_face_probe;` |
| `build.rs:695` | `point_near_arc(&m, &w[0], &w[1], cfg.tau_edge * 2.0)` | `point_near_arc(&m, &w[0], &w[1], cfg.tau_river_attr)` |
| `build.rs:752` | `off <= tol && on_arc(p, a, b, tol * 4.0)` | give `point_near_arc` a second parameter `span_tol` and pass `cfg.tau_river_span` — **add this eleventh ledger row** with its own two populations, or, if the plateau sweep shows the answer is invariant in `span_tol` over a wide band around `tol`, collapse it to `tol` and record that finding in the `tau_river_attr` row's evidence. Prefer collapsing: one fewer knob is strictly better than one more declared knob. |
| `build.rs` × 7 | bare `1e-12` cross-product guards | `cfg.tau_degenerate` |
| `build.rs:136/140/170/675` | `1e-12` duplicate-point guards | `cfg.tau_degenerate` |
| `lib.rs:501` | `if residual > 1e-10` | `if residual > cfg.completeness_residual` — `validate` must therefore take `&PartitionConfig`; update its four call sites (`build.rs:704`, `tests.rs`'s `ok()` helper, and the two in `map-compile`) |
| `partition_bridge.rs:760` | `< full * 0.02 + 1e-9` | a declared `snap_projection_slack` row, OR — check first — the `full * 0.02` is a *relative* on-arc test, which is a different animal from an absolute tolerance; if the plateau sweep shows it invariant, keep the relative form and declare it as a `Ratio` row with `same_max`/`distinct_min` in ratio units, adding a `Tolerance::unit` note to its evidence rather than a new type |
| `partition_bridge.rs:814` | `if sp.abs() > 60.0` | a declared `snap_span_cap` row in *ring-index* units, whose two populations are the longest legitimate same-target run measured in the real data and the shortest illegitimate wrap-around |
| `partition_bridge.rs:836/840` | `1e-9` dedup guards | `tol.tau_degenerate.radians()` |

Delete `impl Default for PartitionConfig` entirely once the substitutions land: with the ledger loaded from data, a hardcoded default is a second, undeclared, un-reviewed tolerance set. Every remaining `PartitionConfig::default()` call site (`crates/map-partition/src/tests.rs`'s `cfg()` helper, `crates/map-compile/src/tests.rs:381`, `partition_bridge.rs:85`) takes the ledger instead; the partition crate's own tests get a `fn cfg() -> PartitionConfig` that builds one from an inline `Tolerances` literal, so `map-partition` keeps no dependency on `map-compile` or on the data directory.

- [ ] **Step 5: Run everything to green**

Run: `cargo test -p map-partition`
Expected: PASS, including `the_geometry_modules_contain_no_undeclared_float`.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cargo test -p map-compile plateau -- --ignored --nocapture`
Expected: PASS — the plateau law must still hold after the substitution, which is the check that no substitution changed a value.

- [ ] **Step 6: Golden gate and census — still nothing moved**

Rebuild all three steps, then:

Run: `node crates/map-viewer/tests/golden.js --check`
Expected: `ALL GOLDEN VIEWS HOLD` (89/89).

Run: `curl -s "http://127.0.0.1:8090/api/census?year=-1405&to=-1405"` and confirm an empty diff at the two beloved stops (-1405, -1446) and at -1050, 26, 59. Part A has not touched a coordinate or an identity; a non-empty diff here means a substitution changed a value.

- [ ] **Step 7: Commit**

```bash
git add crates/map-partition/src/literals.rs crates/map-partition/src/lib.rs crates/map-partition/src/build.rs crates/map-partition/src/tests.rs crates/map-compile/src/partition_bridge.rs crates/map-compile/src/tests.rs data/authored/tolerances.json
git commit -m "Stage 3 Task 5: every derived tolerance declared; the law that stops the next bare float"
```

---

## Part B — the canonical edge, with zero pixels moved

Spec §2's Border is "one canonical edge in THE one arrangement, content-addressed, carrying the witness set whose geometry it realizes. All pipelines' geometry funnels here." Equation 2: "every border exists once, in the one arrangement; extents are references."

**What the code actually holds today, established by reading it.** `map-partition` already computes exactly this. `PEdge` (`crates/map-partition/src/lib.rs:149-159`) is "one canonical undirected edge: a single minor great-circle arc between two canonical vertices, stored exactly once", and it already carries `provenance: Vec<String>` — "the witnesses whose geometry this edge carries". The arrangement is already closed, already two-sided, already obeys Σ faces = 4π (`:389-396`, `:500-503`). **The canonical edge exists; it just never leaves the crate.**

`crates/map-compile/src/partition_bridge.rs:291-301` is where it is discarded: `part.dissolve_rings(&g.faces)` returns *point rings*, and each whole ring is content-addressed as one `map_canon::Border`. So the canon's 11,916 `Border` rows are 11,916 whole rings totalling 787,193 points, and two neighbours sharing a border share *no row at all* — they each hold a ring that happens to contain the same coordinates. That is why "every border exists once" is false today, and it is false in one place, for one reason.

Part B fixes exactly that and nothing else. The coordinates that come out are the same coordinates, in the same order; only their storage and addressing change. **The golden gate must therefore show zero drift, and that is the whole verification story for Part B.**

---

### Task 6: `Arc` in the canon — the canonical edge, content-addressed, with its witness set

**Files:**
- Modify: `crates/map-canon/src/lib.rs` (add `Arc`, `ArcId`, `Cycle`, `CycleStep`; `Area` gains cycle-shaped geometry; `Tenure` moves to map-types and is re-exported)
- Modify: `crates/map-types/src/lib.rs` + new `crates/map-types/src/tenure.rs`
- Modify: `crates/map-canon/src/persist.rs`
- Test: `crates/map-canon/src/tests.rs` (append)

**Interfaces:**
- Consumes: `map_types::Orientation` (already exists, `crates/map-types/src/boundary.rs:80-84`) — do not invent a second orientation type.
- Produces:
  - `map_canon::Arc { pts: Vec<UnitVec>, witnesses: Vec<String> }` and `ArcId(ContentHash)`.
  - `map_canon::CycleStep { arc: ArcId, orientation: Orientation }`, `map_canon::Cycle(Vec<CycleStep>)`.
  - `CanonStore::{insert_arc, arcs, arc_points, cycle_points}`.
  - `Area { entity, name, cycles: Vec<Cycle>, holes: Vec<Cycle>, tenure }` — `rings`/`holes` as `BTreeSet<BorderId>` are gone.
  - `map_types::Tenure` (moved), `map_canon::Tenure` re-exported so no call site changes.
  Tasks 7, 8, 9 and the provider consume all of these.

- [ ] **Step 1: Write the failing tests**

Append to `crates/map-canon/src/tests.rs`:

```rust
// ------------------------------------------------ the canonical arc
//
// Spec §2: a Border is ONE canonical edge in the one arrangement,
// content-addressed, carrying the witness set whose geometry it
// realizes. Equation 2: every border exists once; extents are
// references. These laws pin both halves — one row per arc no matter
// how many claimants walk it, and a claimant's geometry as a cycle
// of oriented references rather than a copy of the coordinates.

fn p(lat: f64, lon: f64) -> UnitVec {
    UnitVec::from_lat_lon_deg(lat, lon)
}

#[test]
fn two_claimants_sharing_a_border_share_one_arc_row() {
    // THE law of equation 2, as a fact about the store. Judah and
    // Benjamin walk their common border in opposite directions; the
    // store must hold ONE arc, referenced twice with opposite
    // orientation, not two rows with reversed coordinates.
    let mut s = CanonStore::default();
    let shared = vec![p(31.7, 35.0), p(31.8, 35.2), p(31.9, 35.4)];
    let a = s.insert_arc(Arc { pts: shared.clone(), witnesses: vec!["judah".into()] });
    let b = s.insert_arc(Arc {
        pts: shared.iter().rev().copied().collect(),
        witnesses: vec!["benjamin".into()],
    });
    assert_eq!(a, b, "a reversed arc is the same arc");
    assert_eq!(s.arcs().len(), 1, "one border, one row");
    assert_eq!(
        s.arcs()[&a].witnesses,
        vec!["benjamin".to_string(), "judah".to_string()],
        "the row carries the union of the witness sets, sorted"
    );
}

#[test]
fn an_arcs_id_is_its_geometry_and_nothing_else() {
    // Witness provenance rides WITH the arc but must not enter its
    // identity: the same coastline traced by one witness or by four
    // is the same border. Otherwise "exists once" is false the
    // moment a second witness arrives.
    let mut s = CanonStore::default();
    let pts = vec![p(32.0, 34.9), p(32.4, 34.95)];
    let a = s.insert_arc(Arc { pts: pts.clone(), witnesses: vec!["great-sea".into()] });
    let b = s.insert_arc(Arc { pts, witnesses: vec!["asher".into(), "phoenicia".into()] });
    assert_eq!(a, b);
    assert_eq!(
        s.arcs()[&a].witnesses,
        vec!["asher".to_string(), "great-sea".to_string(), "phoenicia".to_string()]
    );
    // and a DIFFERENT geometry is a different arc — the negative
    // case, without which the two assertions above hold for a store
    // that returns a constant id
    let c = s.insert_arc(Arc { pts: vec![p(32.0, 34.9), p(32.5, 34.95)], witnesses: vec![] });
    assert_ne!(a, c);
    assert_eq!(s.arcs().len(), 2);
}

#[test]
fn a_cycle_resolves_to_exactly_the_ring_its_arcs_spell() {
    // Extents are REFERENCES: the points come back from the arcs, in
    // cycle order, with each reversed step reversed and the shared
    // junction vertex written once.
    let mut s = CanonStore::default();
    let e1 = s.insert_arc(Arc { pts: vec![p(31.0, 35.0), p(31.0, 36.0)], witnesses: vec![] });
    let e2 = s.insert_arc(Arc { pts: vec![p(31.0, 36.0), p(32.0, 36.0)], witnesses: vec![] });
    // walked backwards by this claimant
    let e3 = s.insert_arc(Arc { pts: vec![p(31.0, 35.0), p(32.0, 36.0)], witnesses: vec![] });
    let cycle = Cycle(vec![
        CycleStep { arc: e1, orientation: Orientation::Forward },
        CycleStep { arc: e2, orientation: Orientation::Forward },
        CycleStep { arc: e3, orientation: Orientation::Reverse },
    ]);
    assert_eq!(
        s.cycle_points(&cycle).expect("the cycle closes"),
        vec![p(31.0, 35.0), p(31.0, 36.0), p(32.0, 36.0)],
        "the closing point is not repeated"
    );
}

#[test]
fn a_cycle_that_does_not_close_is_a_named_violation() {
    // The negative case for the resolver: a broken cycle must be a
    // typed refusal carrying the position, never a silently short
    // ring the renderer then draws.
    let mut s = CanonStore::default();
    let e1 = s.insert_arc(Arc { pts: vec![p(31.0, 35.0), p(31.0, 36.0)], witnesses: vec![] });
    let e2 = s.insert_arc(Arc { pts: vec![p(33.0, 36.0), p(34.0, 36.0)], witnesses: vec![] });
    let cycle = Cycle(vec![
        CycleStep { arc: e1, orientation: Orientation::Forward },
        CycleStep { arc: e2, orientation: Orientation::Forward },
    ]);
    assert_eq!(
        s.cycle_points(&cycle),
        Err(CycleBreak { position: 1, arc: e2 }),
        "a gap between consecutive arcs is named, never bridged"
    );
}

#[test]
fn an_areas_identity_is_its_cycles_not_its_coordinates() {
    // Content addressing survives the move: the same claimant built
    // from the same arcs is one feature however the cycle list was
    // rotated, because a cycle is canonicalized at insert.
    let mut s = CanonStore::default();
    let e1 = s.insert_arc(Arc { pts: vec![p(31.0, 35.0), p(31.0, 36.0)], witnesses: vec![] });
    let e2 = s.insert_arc(Arc { pts: vec![p(31.0, 36.0), p(32.0, 36.0)], witnesses: vec![] });
    let e3 = s.insert_arc(Arc { pts: vec![p(31.0, 35.0), p(32.0, 36.0)], witnesses: vec![] });
    let mk = |steps: Vec<CycleStep>| {
        Feature::Area(Area {
            entity: EntityId("judah".into()),
            name: "Judah".into(),
            cycles: vec![Cycle(steps)],
            holes: vec![],
            tenure: Tenure::Held,
        })
    };
    let f = |a, b, c| {
        vec![
            CycleStep { arc: a, orientation: Orientation::Forward },
            CycleStep { arc: b, orientation: Orientation::Forward },
            CycleStep { arc: c, orientation: Orientation::Reverse },
        ]
    };
    let id1 = s.insert_feature(mk(f(e1, e2, e3)));
    // the same walk, started one step later
    let mut rotated = f(e1, e2, e3);
    rotated.rotate_left(1);
    let id2 = s.insert_feature(mk(rotated));
    assert_eq!(id1, id2, "a rotated cycle is the same cycle");
    assert_eq!(s.features().len(), 1);
}

#[test]
fn arcs_and_cycles_round_trip_through_persistence_byte_stably() {
    let mut s = CanonStore::default();
    let e1 = s.insert_arc(Arc { pts: vec![p(31.0, 35.0), p(31.0, 36.0)], witnesses: vec!["judah".into()] });
    let e2 = s.insert_arc(Arc { pts: vec![p(31.0, 36.0), p(32.0, 36.0)], witnesses: vec!["judah".into(), "benjamin".into()] });
    let e3 = s.insert_arc(Arc { pts: vec![p(31.0, 35.0), p(32.0, 36.0)], witnesses: vec![] });
    let fid = s.insert_feature(Feature::Area(Area {
        entity: EntityId("judah".into()),
        name: "Judah".into(),
        cycles: vec![Cycle(vec![
            CycleStep { arc: e1, orientation: Orientation::Forward },
            CycleStep { arc: e2, orientation: Orientation::Forward },
            CycleStep { arc: e3, orientation: Orientation::Reverse },
        ])],
        holes: vec![],
        tenure: Tenure::Claimed,
    }));
    let _ = fid;
    let bytes = crate::persist::to_bytes(&s).expect("writes");
    let back = crate::persist::from_bytes(&bytes).expect("reads");
    assert_eq!(back, s, "the whole store round-trips, arcs and cycles included");
    assert_eq!(crate::persist::to_bytes(&back).expect("rewrites"), bytes, "byte-stable");
}
```

- [ ] **Step 2: Run and watch them fail**

Run: `cargo test -p map-canon arc`
Expected: FAIL — `cannot find struct 'Arc' in this scope`.

- [ ] **Step 3: Move `Tenure` to map-types**

Create `crates/map-types/src/tenure.rs` containing the enum exactly as it stands at `crates/map-canon/src/lib.rs:60-65`, doc comment included, and `pub mod tenure; pub use tenure::Tenure;` in `crates/map-types/src/lib.rs`. In `map-canon`, replace the definition with `pub use map_types::Tenure;`. Every existing `map_canon::Tenure::Held` call site (`partition_bridge.rs:605`, `timeline_bridge.rs:117`, `canon_provider.rs:368`/`:427`) keeps compiling untouched. The reason for the move is Task 10: `map-partition` must carry tenure on a witness, and `map-partition` depends on `map-types` but not on `map-canon`.

- [ ] **Step 4: Add `Arc`, `Cycle`, and the new `Area`**

In `crates/map-canon/src/lib.rs`, alongside the existing `content_id!` macro invocation, add `ArcId` to the list, then:

```rust
/// ONE CANONICAL EDGE of the one arrangement (spec §2's Border): a
/// single run of coordinates walked exactly once in the whole canon,
/// content-addressed by its GEOMETRY ALONE, carrying the witness set
/// whose geometry it realizes.
///
/// Identity is geometric and direction-free: the same coordinates
/// walked backwards are the same border. Two neighbours therefore
/// reference one row with opposite orientation, and a scholarship
/// edit to a shared border happens in one place — which is the whole
/// point of equation 2. Witnesses ride alongside identity and never
/// enter it: the same coastline traced by one witness or by four is
/// one border.
#[derive(Clone, Debug, PartialEq)]
pub struct Arc {
    pub pts: Vec<UnitVec>,
    /// the witnesses whose geometry this edge carries, sorted and
    /// deduplicated by `insert_arc`
    pub witnesses: Vec<String>,
}

/// One step of a boundary walk: which arc, and which way round.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CycleStep {
    pub arc: ArcId,
    pub orientation: map_types::Orientation,
}

/// A closed boundary walk as REFERENCES: consecutive steps meet
/// end-to-start and the last meets the first. Canonicalized at
/// insert (rotated to begin at its smallest step) so a rotation is
/// not a different cycle.
#[derive(Clone, Debug, PartialEq)]
pub struct Cycle(pub Vec<CycleStep>);

/// A cycle whose consecutive arcs do not meet, named by position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CycleBreak {
    pub position: usize,
    pub arc: ArcId,
}

impl Cycle {
    /// The canonical rotation: start at the smallest step. Direction
    /// is NOT normalized — a hole and an outer ring differ by winding
    /// and that difference is meaning, not noise.
    pub fn canonical(mut self) -> Self {
        if let Some(k) = (0..self.0.len()).min_by_key(|&i| self.0[i]) {
            self.0.rotate_left(k);
        }
        self
    }
}
```

Change `Area`:

```rust
/// A named territorial (or claim) shape: boundary cycles and hole
/// cycles, BY REFERENCE into the one arrangement (equation 2:
/// "extents are references"). No coordinates live here.
#[derive(Clone, Debug, PartialEq)]
pub struct Area {
    pub entity: EntityId,
    pub name: String,
    pub cycles: Vec<Cycle>,
    pub holes: Vec<Cycle>,
    pub tenure: Tenure,
}
```

Add to `CanonStore` a `arcs: BTreeMap<ArcId, Arc>` field beside `borders` (keep `borders` — journeys and rivers still store whole polylines as `Border`, and Part B does not touch them), and:

```rust
impl CanonStore {
    /// Insert a canonical edge. Identity is the geometry, direction
    /// free; re-inserting the same edge UNIONS the witness sets
    /// rather than replacing them, so the row accumulates every
    /// claimant that walks it.
    pub fn insert_arc(&mut self, a: Arc) -> ArcId {
        let id = ArcId(hash_bytes(&arc_bytes(&a)));
        let row = self.arcs.entry(id).or_insert_with(|| Arc {
            pts: canonical_arc_points(&a.pts),
            witnesses: Vec::new(),
        });
        for w in a.witnesses {
            if !row.witnesses.contains(&w) {
                row.witnesses.push(w);
            }
        }
        row.witnesses.sort();
        id
    }

    pub fn arcs(&self) -> &BTreeMap<ArcId, Arc> {
        &self.arcs
    }

    pub fn arc_points(&self, id: ArcId, o: map_types::Orientation) -> Option<Vec<UnitVec>> {
        let a = self.arcs.get(&id)?;
        Some(match o {
            map_types::Orientation::Forward => a.pts.clone(),
            map_types::Orientation::Reverse => a.pts.iter().rev().copied().collect(),
        })
    }

    /// A cycle's ring: every step's points in order, the shared
    /// junction vertex written once, the closing point NOT repeated
    /// (the canon's ring convention, unchanged). A gap between
    /// consecutive steps is a typed refusal, never a bridged ring.
    pub fn cycle_points(&self, c: &Cycle) -> Result<Vec<UnitVec>, CycleBreak> {
        let mut out: Vec<UnitVec> = Vec::new();
        for (i, step) in c.0.iter().enumerate() {
            let pts = self
                .arc_points(step.arc, step.orientation)
                .ok_or(CycleBreak { position: i, arc: step.arc })?;
            match out.last() {
                None => out.extend(pts),
                Some(prev) => {
                    if prev.angle_to(&pts[0]) > 1e-12 {
                        return Err(CycleBreak { position: i, arc: step.arc });
                    }
                    out.extend(pts.iter().skip(1).copied());
                }
            }
        }
        // the walk closes: drop the repeated first point
        while out.len() > 1 && out[0].angle_to(out.last().expect("non-empty")) <= 1e-12 {
            out.pop();
        }
        Ok(out)
    }
}

/// The direction-free canonical point order: whichever of the two
/// directions has the smaller byte key at its head. So a border and
/// its reverse hash alike and store once.
fn canonical_arc_points(pts: &[UnitVec]) -> Vec<UnitVec> {
    let fwd: Vec<UnitVec> = pts.to_vec();
    let rev: Vec<UnitVec> = pts.iter().rev().copied().collect();
    let key = |v: &[UnitVec]| -> Vec<u8> {
        v.iter().flat_map(|p| point_key(p)).collect()
    };
    if key(&fwd) <= key(&rev) { fwd } else { rev }
}

fn point_key(p: &UnitVec) -> [u8; 24] {
    let q = |x: f64| ((x * 1e9).round() as i64).to_be_bytes();
    let mut k = [0u8; 24];
    k[..8].copy_from_slice(&q(p.x()));
    k[8..16].copy_from_slice(&q(p.y()));
    k[16..].copy_from_slice(&q(p.z()));
    k
}

fn arc_bytes(a: &Arc) -> Vec<u8> {
    let mut c = Bytes::new();
    c.tag("canon-arc");
    c.seq(&canonical_arc_points(&a.pts), |c, p| {
        c.f64_(p.x()).f64_(p.y()).f64_(p.z());
    });
    c.done()
}
```

**Note the `1e-12` in `cycle_points`.** Part A's literal law covers `map-partition`, not `map-canon`; extend the law's file list to `map-canon/src/lib.rs` in Task 12's closing sweep, and until then declare this one as `tau_degenerate` by threading the ledger into `CanonStore`, OR — simpler and better — express the junction test as exact equality on `point_key`, which needs no tolerance at all because both points come from the same quantized arrangement vertex. **Prefer the exact form**; a tolerance that can be deleted is better than a tolerance that can be declared.

Update `feature_bytes`'s `Feature::Area` arm to hash cycles instead of ring id sets:

```rust
Feature::Area(a) => {
    c.tag("canon-area");
    c.str_(&a.entity.0).str_(&a.name);
    c.u8_(match a.tenure { Tenure::Held => 0, Tenure::Claimed => 1 });
    let mut walk = |c: &mut Bytes, cycles: &[Cycle]| {
        c.seq(cycles, |c, cy| {
            c.seq(&cy.0, |c, s| {
                c.u64_(s.arc.0 .0);
                c.u8_(match s.orientation {
                    map_types::Orientation::Forward => 0,
                    map_types::Orientation::Reverse => 1,
                });
            });
        });
    };
    walk(c, &a.cycles);
    walk(c, &a.holes);
}
```

Note that this *adds tenure to the area's content identity*, which it is not part of today — a deliberate correction, because two areas differing only in tenure are genuinely different facts and today collide. Canonicalize each cycle in `insert_feature` before hashing (`Cycle::canonical`), and sort the cycle list by its first step.

Extend `feature_border_refs` and `validate` so an `Area`'s arcs are resolved through `self.arcs` and a dangling one is `CanonViolation::UnresolvedArc { feature, arc }` (a new variant beside `UnresolvedBorder`); and `rings_of` (`:583-589`) resolves through `cycle_points`, skipping any cycle that breaks — `validate` reports the break separately as `CanonViolation::BrokenCycle { feature, position }`, matching the vocabulary already used in `map_types::laws::Violation::BrokenCycle`.

- [ ] **Step 5: Persist arcs and cycles**

In `crates/map-canon/src/persist.rs`, add an `arcs` object beside the existing `borders` object (`to_bytes`'s first block), each row `{"pts": [[x,y,z],…], "witnesses": ["…"]}`; and serialize an area's cycles as `[[{"a":"<hex>","o":"f"|"r"}, …], …]`. Loading recomputes every id and refuses a mismatch — that machinery already exists and must cover arcs too, or the "corrupted canon fails loud" promise in the module header becomes a half-truth.

- [ ] **Step 6: Run to green**

Run: `cargo test -p map-canon`
Expected: PASS. Existing tests that build `Area { rings: … }` need mechanical updating to cycles; that is expected and is part of this task.

Run: `cargo build --workspace`
Expected: FAIL in `map-compile` and `map-provider`, which still speak `Area.rings`. **That is correct** — Task 7 and Task 8 fix them. Do not paper over it here with a compatibility shim; a shim would let both representations exist at once, which is the disease.

- [ ] **Step 7: Commit**

```bash
git add crates/map-types/src/tenure.rs crates/map-types/src/lib.rs crates/map-canon/src/lib.rs crates/map-canon/src/persist.rs crates/map-canon/src/tests.rs
git commit -m "Stage 3 Task 6: Arc — one canonical edge per border, referenced not copied"
```

---

### Task 7: `dissolve_arcs` — the arrangement hands out edges, and proves it hands out the same geometry

`Partition::dissolve_rings` (`crates/map-partition/src/lib.rs:420-460`) already walks the union boundary by pure topology. This task adds a sibling that returns the *half-edges it walked* instead of the points it collected, and pins the two against each other so the change cannot alter one coordinate.

**Files:**
- Modify: `crates/map-partition/src/lib.rs`
- Test: `crates/map-partition/src/tests.rs` (append)

**Interfaces:**
- Consumes: `Partition::{halves, edges, vertices, faces}`.
- Produces: `Partition::dissolve_arcs(&BTreeSet<FaceId>) -> Vec<Vec<(EdgeId, map_types::Orientation)>>`; `Partition::edge_points(EdgeId, Orientation) -> [UnitVec; 2]`; `PEdge::provenance` becomes the arc's witness set on the wire in Task 9. Task 8 consumes `dissolve_arcs`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/map-partition/src/tests.rs`:

```rust
// ------------------------------------ dissolve, as edges not points
//
// The equivalence law is the whole safety argument for Part B: if
// the edges spell exactly the ring the point form produces, then
// storing edges instead of rings cannot move a pixel. It is checked
// on the real arrangement, not a toy, because a toy would not
// contain the interior seams the dissolve exists to erase.

#[test]
fn dissolved_arcs_spell_exactly_the_dissolved_ring() {
    let (regions, polylines) = fixture_witnesses();
    let p = build(&regions, &polylines, &cfg()).expect("builds");
    let mut checked = 0usize;
    for (name, faces) in every_claimants_face_set(&p) {
        let rings = p.dissolve_rings(&faces);
        let arcs = p.dissolve_arcs(&faces);
        assert_eq!(rings.len(), arcs.len(), "{name}: same number of cycles");
        for (ring, cycle) in rings.iter().zip(&arcs) {
            let mut spelled: Vec<map_types::UnitVec> = Vec::new();
            for &(e, o) in cycle {
                let [a, _b] = p.edge_points(e, o);
                spelled.push(a);
            }
            assert_eq!(
                spelled.len(),
                ring.len(),
                "{name}: cycle length matches the point ring"
            );
            for (i, (x, y)) in spelled.iter().zip(ring).enumerate() {
                assert!(
                    x.angle_to(y) == 0.0,
                    "{name}: vertex {i} differs — the edge form must be the SAME points, \
                     bit for bit, not merely close"
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 0, "the law examined at least one real cycle");
}

#[test]
fn a_dissolved_cycle_is_a_closed_walk_of_two_sided_edges() {
    // Equation 2's structural half, stated where it can be checked:
    // every step's destination is the next step's origin, the last
    // closes on the first, and every edge in the walk has the face
    // set on exactly one side.
    let (regions, polylines) = fixture_witnesses();
    let p = build(&regions, &polylines, &cfg()).expect("builds");
    for (name, faces) in every_claimants_face_set(&p) {
        for cycle in p.dissolve_arcs(&faces) {
            for k in 0..cycle.len() {
                let (e, o) = cycle[k];
                let (e2, o2) = cycle[(k + 1) % cycle.len()];
                let [_, b] = p.edge_points(e, o);
                let [c, _] = p.edge_points(e2, o2);
                assert!(b.angle_to(&c) == 0.0, "{name}: step {k} does not meet step {}", k + 1);
                let inside = |h: usize| faces.contains(&p.halves[h].face);
                let (h1, h2) = (p.edges[e].half_ab, p.edges[e].half_ba);
                assert_ne!(
                    inside(h1),
                    inside(h2),
                    "{name}: a boundary edge with the set on both sides or neither"
                );
            }
        }
    }
}

#[test]
fn every_dissolved_edge_carries_a_non_empty_witness_set() {
    // Spec §2: an edge carries "the witness set whose geometry it
    // realizes". An edge that realizes nobody's geometry is ink
    // nobody drew, and Task 9 would serve it as unattributed
    // provenance.
    let (regions, polylines) = fixture_witnesses();
    let p = build(&regions, &polylines, &cfg()).expect("builds");
    let unattributed: Vec<usize> = (0..p.edges.len())
        .filter(|&e| p.edges[e].provenance.is_empty())
        .collect();
    assert_eq!(unattributed, Vec::<usize>::new(), "every edge names its witnesses");
}
```

Add the two helpers at the top of the test module (the crate's tests are self-contained and must stay so — no dependency on `map-compile`):

```rust
/// A witness set with real structure: two adjacent claims sharing a
/// border, a lake inside one of them, and a parent containing both,
/// so dissolve has interior seams to erase and holes to keep.
fn fixture_witnesses() -> (Vec<WitnessRegion>, Vec<WitnessPolyline>) {
    let parent = region("parent", FaceKind::LandClaim, vec![square(30.0, 34.0, 34.0, 38.0)]);
    let mut west = region("west", FaceKind::LandClaim, vec![square(30.0, 34.0, 34.0, 36.0)]);
    west.parent = Some("parent".into());
    let mut east = region("east", FaceKind::LandClaim, vec![square(30.0, 36.0, 34.0, 38.0)]);
    east.parent = Some("parent".into());
    let lake = region("lake", FaceKind::Lake, vec![square(31.0, 34.5, 32.0, 35.5)]);
    (vec![parent, west, east, lake], vec![])
}

/// Every claimant's face set, exactly as the compile bridge gathers
/// them — the same shape Task 8 will dissolve.
fn every_claimants_face_set(p: &Partition) -> Vec<(String, std::collections::BTreeSet<usize>)> {
    let mut by: std::collections::BTreeMap<String, std::collections::BTreeSet<usize>> =
        std::collections::BTreeMap::new();
    for (fi, f) in p.faces.iter().enumerate() {
        for c in &f.claims {
            by.entry(c.clone()).or_default().insert(fi);
        }
    }
    by.into_iter().collect()
}
```

- [ ] **Step 2: Run and watch them fail**

Run: `cargo test -p map-partition dissolve`
Expected: FAIL — `no method named 'dissolve_arcs'`.

- [ ] **Step 3: Implement `dissolve_arcs` and `edge_points`**

In `crates/map-partition/src/lib.rs`, beside `dissolve_rings`:

```rust
/// The union of a face set as its boundary cycles OF EDGES — the
/// same walk `dissolve_rings` performs, reporting the half-edges it
/// crossed instead of the vertices it collected. Spec §2's "extents
/// are references" needs the references, and the equivalence law in
/// tests.rs pins the two forms together so neither can drift.
///
/// The body is `dissolve_rings` with one line changed: where that
/// pushes `self.vertices[self.halves[h].origin]`, this pushes the
/// edge and the direction the walk took it.
pub fn dissolve_arcs(
    &self,
    set: &std::collections::BTreeSet<FaceId>,
) -> Vec<Vec<(EdgeId, map_types::Orientation)>> {
    let on_boundary = |h: HalfId| {
        let hh = &self.halves[h];
        set.contains(&hh.face) && !set.contains(&self.halves[hh.twin].face)
    };
    let mut seen = vec![false; self.halves.len()];
    let mut out = Vec::new();
    for &f in set {
        for cy in &self.faces[f].cycles {
            for &h0 in cy {
                if !on_boundary(h0) || seen[h0] {
                    continue;
                }
                let mut walk = Vec::new();
                let mut h = h0;
                let mut fuel = self.halves.len() + 1;
                loop {
                    seen[h] = true;
                    let e = self.halves[h].edge;
                    let o = if self.edges[e].half_ab == h {
                        map_types::Orientation::Forward
                    } else {
                        map_types::Orientation::Reverse
                    };
                    walk.push((e, o));
                    let mut n = self.halves[h].next;
                    while !on_boundary(n) {
                        n = self.halves[self.halves[n].twin].next;
                        fuel -= 1;
                        assert!(fuel > 0, "dissolve_arcs: rotation does not close");
                    }
                    h = n;
                    fuel -= 1;
                    assert!(fuel > 0, "dissolve_arcs: cycle does not close");
                    if h == h0 {
                        break;
                    }
                }
                out.push(walk);
            }
        }
    }
    out
}

/// An edge's two endpoints in the requested direction.
pub fn edge_points(&self, e: EdgeId, o: map_types::Orientation) -> [UnitVec; 2] {
    let (a, b) = (self.vertices[self.edges[e].a], self.vertices[self.edges[e].b]);
    match o {
        map_types::Orientation::Forward => [a, b],
        map_types::Orientation::Reverse => [b, a],
    }
}
```

- [ ] **Step 4: Run to green**

Run: `cargo test -p map-partition dissolve`
Expected: PASS.

**If `every_dissolved_edge_carries_a_non_empty_witness_set` fails, do not weaken it.** `atomic` is keyed on vertex pairs and populated from `s.witness` for every segment (`crates/map-partition/src/build.rs:270-313`), so every edge should have at least one witness by construction; an empty set means an edge was created by a path that forgot to record its source, and that is a real defect to fix in `build.rs`, not an assertion to relax.

- [ ] **Step 5: Commit**

```bash
git add crates/map-partition/src/lib.rs crates/map-partition/src/tests.rs
git commit -m "Stage 3 Task 7: dissolve_arcs — the arrangement hands out edges, pinned to the point form"
```

---

### Task 8: The bridge stores arcs; the provider resolves them — and the golden gate proves nothing moved

This is Part B's load-bearing task and its checkpoint. `crates/map-compile/src/partition_bridge.rs:291-301` stops inserting whole rings and starts inserting canonical arcs; `crates/map-provider/src/canon_provider.rs` resolves cycles back to rings at render time. Every coordinate that reaches the renderer is the same coordinate in the same order, so **89/89 stops must hold with zero drift and the census diff must be empty.** If either moves, the change is wrong — there is no visual decision to be made here.

**Files:**
- Modify: `crates/map-compile/src/partition_bridge.rs` (`bundle_faces`, `:281-305`)
- Modify: `crates/map-compile/src/timeline_bridge.rs` (`resolve_cycle` and the `Area` construction, `:84-95`)
- Modify: `crates/map-provider/src/canon_provider.rs` (`ring_points` and its two callers around `:340-440`, `:818-880`)
- Test: `crates/map-compile/src/tests.rs`, `crates/map-provider/src/tests.rs` (append)

**Interfaces:**
- Consumes: `map_canon::{Arc, Cycle, CycleStep, CanonStore::insert_arc, cycle_points}` (Task 6); `Partition::{dissolve_arcs, edge_points}` (Task 7).
- Produces: the canon on disk now holds `arcs` and cycle-shaped areas. `CanonProvider::rings_of_area(&Area) -> Vec<Ring>` replaces the old ring resolution. Task 9's `/api/borders` reads `store.arcs()`; Task 10 extends `bundle_faces` into `extents`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/map-compile/src/tests.rs`:

```rust
// -------------------------- the canon holds edges, not copied rings
//
// Equation 2 as a fact about the compiled canon: every border exists
// once. The counting law is the one that could not pass by accident
// — if the bridge still stored whole rings, the arc count would be
// the ring count and shared borders would be invisible.

#[test]
#[ignore = "compiles the real arrangement; run explicitly"]
fn neighbouring_claimants_reference_the_same_arc_rows() {
    let (regions, polylines) = crate::partition_bridge::gather_witnesses(&[])
        .expect("witnesses gather");
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let tol = crate::tolerances::load(&text).expect("ledger loads");
    let p = map_partition::build(
        &regions,
        &polylines,
        &map_partition::PartitionConfig::from_tolerances(&tol),
    )
    .expect("builds");

    let mut store = map_canon::CanonStore::default();
    let bundles = crate::partition_bridge::bundle_faces_for_law(&mut store, &p, &Default::default());

    // Judah and Benjamin are adjacent allotments. Their cycles must
    // SHARE arc rows — the whole claim of equation 2 — and the shared
    // rows must be walked in opposite directions.
    let arcs_of = |who: &str| -> std::collections::BTreeSet<map_canon::ArcId> {
        bundles[who].cycles.iter().flat_map(|c| c.0.iter().map(|s| s.arc)).collect()
    };
    let shared: Vec<map_canon::ArcId> =
        arcs_of("judah").intersection(&arcs_of("benjamin")).copied().collect();
    assert!(
        !shared.is_empty(),
        "adjacent allotments share no arc row — the bridge is still copying rings"
    );
    for a in &shared {
        let dir = |who: &str| -> map_types::Orientation {
            bundles[who]
                .cycles
                .iter()
                .flat_map(|c| c.0.iter())
                .find(|s| s.arc == *a)
                .expect("present")
                .orientation
        };
        assert_ne!(
            dir("judah"),
            dir("benjamin"),
            "a shared border must be walked in opposite directions by its two sides"
        );
        assert_eq!(
            {
                let mut w = store.arcs()[a].witnesses.clone();
                w.retain(|x| x == "judah" || x == "benjamin");
                w
            },
            vec!["benjamin".to_string(), "judah".to_string()],
            "the shared row names both witnesses"
        );
    }
}

#[test]
#[ignore = "compiles the real arrangement; run explicitly"]
fn the_resolved_geometry_is_bit_identical_to_the_ring_form() {
    // The safety law for the whole of Part B, at the level the
    // renderer sees. Whatever the storage, the points must be the
    // same points.
    let (regions, polylines) = crate::partition_bridge::gather_witnesses(&[])
        .expect("witnesses gather");
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let tol = crate::tolerances::load(&text).expect("ledger loads");
    let p = map_partition::build(
        &regions,
        &polylines,
        &map_partition::PartitionConfig::from_tolerances(&tol),
    )
    .expect("builds");
    let mut store = map_canon::CanonStore::default();
    let bundles = crate::partition_bridge::bundle_faces_for_law(&mut store, &p, &Default::default());

    for (who, bundle) in &bundles {
        let mut faces = std::collections::BTreeSet::new();
        for (fi, f) in p.faces.iter().enumerate() {
            if f.claims.first().map(String::as_str) == Some(who.as_str()) {
                faces.insert(fi);
            }
        }
        let mut want: Vec<Vec<map_types::UnitVec>> = p.dissolve_rings(&faces);
        let mut got: Vec<Vec<map_types::UnitVec>> = bundle
            .cycles
            .iter()
            .chain(&bundle.holes)
            .map(|c| store.cycle_points(c).expect("the cycle closes"))
            .collect();
        let key = |r: &Vec<map_types::UnitVec>| (r.len(), format!("{:?}", r.first()));
        want.sort_by_key(key);
        got.sort_by_key(key);
        assert_eq!(want.len(), got.len(), "{who}: cycle count");
        for (a, b) in want.iter().zip(&got) {
            assert_eq!(a.len(), b.len(), "{who}: ring length");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                assert!(x.angle_to(y) == 0.0, "{who}: vertex {i} moved — Part B must not move a point");
            }
        }
    }
}
```

Append to `crates/map-provider/src/tests.rs`:

```rust
#[test]
fn an_areas_rings_come_back_from_its_arcs_unchanged() {
    // The provider's half of the safety law: whatever it renders
    // from cycles must equal what it rendered from rings, for a
    // hand-built store where both forms can be compared directly.
    let mut store = map_canon::CanonStore::default();
    let p = |lat: f64, lon: f64| map_types::UnitVec::from_lat_lon_deg(lat, lon);
    let corners = [p(31.0, 35.0), p(31.0, 36.0), p(32.0, 36.0), p(32.0, 35.0)];
    let mut steps = Vec::new();
    for k in 0..4 {
        let a = store.insert_arc(map_canon::Arc {
            pts: vec![corners[k], corners[(k + 1) % 4]],
            witnesses: vec!["w".into()],
        });
        // insert_arc canonicalizes direction; ask which way this walk
        // takes the row rather than assuming Forward
        let o = if store.arcs()[&a].pts[0].angle_to(&corners[k]) == 0.0 {
            map_types::Orientation::Forward
        } else {
            map_types::Orientation::Reverse
        };
        steps.push(map_canon::CycleStep { arc: a, orientation: o });
    }
    let area = map_canon::Area {
        entity: map_canon::EntityId("w".into()),
        name: "W".into(),
        cycles: vec![map_canon::Cycle(steps)],
        holes: vec![],
        tenure: map_canon::Tenure::Held,
    };
    assert_eq!(
        crate::canon_provider::rings_of_area(&store, &area),
        vec![corners.to_vec()],
        "the ring the provider draws is the ring the arcs spell"
    );
}
```

- [ ] **Step 2: Run and watch them fail**

Run: `cargo test -p map-compile --ignored neighbouring` then `cargo test -p map-provider rings_of_area`
Expected: FAIL — `no function 'bundle_faces_for_law'`; `no function 'rings_of_area'`. (The workspace does not compile at all until Step 3 lands, since Task 6 changed `Area` — that is expected and is why these two tasks are adjacent.)

- [ ] **Step 3: Change the bridge**

In `crates/map-compile/src/partition_bridge.rs`, `Bundle` becomes:

```rust
/// ONE extent per entity, AS REFERENCES: the boundary cycles of the
/// entity's face set, spelled in canonical arcs. The interior seams
/// of the union do not appear — they are artifacts of the whole-frame
/// arrangement — and no coordinate is copied: an arc walked by two
/// neighbours is ONE row in the store, referenced twice.
struct Bundle {
    kind: FaceKind,
    biggest: f64,
    cycles: Vec<map_canon::Cycle>,
    holes: Vec<map_canon::Cycle>,
    note: String,
}
```

and the dissolve block (`:291-301`) becomes:

```rust
for walk in part.dissolve_arcs(&g.faces) {
    if walk.len() < 3 {
        continue;
    }
    let mut steps = Vec::with_capacity(walk.len());
    let mut pts: Vec<UnitVec> = Vec::with_capacity(walk.len());
    for &(e, o) in &walk {
        let [a, b] = part.edge_points(e, o);
        pts.push(a);
        let arc = store.insert_arc(map_canon::Arc {
            pts: vec![a, b],
            witnesses: part.edges[e].provenance.clone(),
        });
        // insert_arc stores a canonical direction; record which way
        // THIS walk takes it, so the cycle spells the same ring
        let o_stored = if store.arcs()[&arc].pts[0].angle_to(&a) == 0.0 {
            map_types::Orientation::Forward
        } else {
            map_types::Orientation::Reverse
        };
        steps.push(map_canon::CycleStep { arc, orientation: o_stored });
    }
    let cycle = map_canon::Cycle(steps).canonical();
    if cycle_area(&pts) > 0.0 {
        bundle.cycles.push(cycle);
    } else {
        bundle.holes.push(cycle);
    }
}
```

Note the outer/hole split still uses `cycle_area` on the point form — the same test the ring version used (`:296`), so the split cannot change. Expose the function for the law suite:

```rust
#[cfg(test)]
pub(crate) fn bundle_faces_for_law(
    store: &mut CanonStore,
    part: &map_partition::Partition,
    absent: &BTreeSet<String>,
) -> std::collections::BTreeMap<String, Bundle> {
    bundle_faces(store, part, absent)
}
```

(and make `Bundle`'s fields `pub(crate)` so the test can read them).

The `Feature::Area` construction at `:600-605` takes `cycles: bundle.cycles, holes: bundle.holes` and — **unchanged for now** — `tenure: map_canon::Tenure::Held`. Task 10 fixes that; changing it here would move the census in a task whose whole promise is that nothing moves.

In `crates/map-compile/src/timeline_bridge.rs`, `resolve_cycle` currently returns a single `BorderId` per cycle by flattening the old model's oriented boundary references into one ring (`:84-95`). It now returns a `map_canon::Cycle`: for each `(BoundaryId, Orientation)` in the part's cycle, insert the boundary's points as an `Arc` with the region's slug as its witness and record the step. This keeps the authored/vendored timeline pipeline speaking the same canonical vocabulary as the partition — which is what makes Task 10's survey migration a *deletion* rather than a translation.

- [ ] **Step 4: Change the provider**

In `crates/map-provider/src/canon_provider.rs`, add:

```rust
/// An area's outer rings, resolved from its arcs. A broken cycle is
/// dropped and NAMED in the provider's diagnostics rather than drawn
/// short: a half-ring is a lie the renderer cannot detect.
pub fn rings_of_area(store: &CanonStore, a: &map_canon::Area) -> Vec<Vec<UnitVec>> {
    a.cycles
        .iter()
        .filter_map(|c| match store.cycle_points(c) {
            Ok(pts) => Some(pts),
            Err(brk) => {
                eprintln!("canon: {} has a broken cycle at step {}", a.entity.0, brk.position);
                None
            }
        })
        .collect()
}
```

and route the two existing ring-resolution sites through it. The existing `ring_points(BorderId, q)` path — which applies level-of-detail simplification and returns `RingFidelity` — keeps its job; it now takes a resolved `Vec<UnitVec>` instead of a `BorderId`. **Do not simplify per-arc**: simplification must run on the assembled ring exactly as it does today, or the LOD output changes and the golden gate goes red for a reason that has nothing to do with storage.

- [ ] **Step 5: Run every Rust test to green**

Run: `cargo test --workspace`
Expected: PASS.

Run: `cargo test -p map-compile -- --ignored neighbouring the_resolved_geometry`
Expected: PASS. `the_resolved_geometry_is_bit_identical_to_the_ring_form` is the one that matters.

- [ ] **Step 6: Recompile the canon and check the shape of the change**

Run the three-step pipeline. Then, as a sanity reading (not an assertion):

```bash
python -c "
import json
d=json.load(open('data/canon/canon.json',encoding='utf-8'))
print('arcs', len(d.get('arcs',{})), 'borders', len(d.get('borders',{})), 'features', len(d['features']))
"
```

Before this task the canon held **11,916 borders / 787,193 points / 1,807 features**. Afterwards, `arcs` should be very much larger in *row count* and very much smaller in *total points* — one row per canonical two-point edge, each shared by both its sides, instead of one row per whole ring copied per claimant. `borders` should shrink to only what still stores whole polylines: journey legs and river paths. **If `arcs` is roughly 11,916 and the point total is roughly 787,193, the bridge is still copying rings and Step 3 did not take.**

- [ ] **Step 7: The gates — the checkpoint of Part B**

Run: `node crates/map-viewer/tests/golden.js --check`
Expected: **`ALL GOLDEN VIEWS HOLD` (89/89), zero drifted probes.**

Run the census diff across every stop:

```bash
python - <<'PY'
import json, urllib.request
stops = json.load(urllib.request.urlopen('http://127.0.0.1:8090/api/meta'))['stops']
bad = []
for i in range(len(stops) - 1):
    u = f'http://127.0.0.1:8090/api/census?year={stops[i]}&to={stops[i]}'
    d = json.load(urllib.request.urlopen(u))
    if d: bad.append((stops[i], len(d)))
print('non-empty self-diffs:', bad or 'none')
PY
```

Then compare against the pre-task canon by checking out the previous commit's `data/canon/canon.json` into a scratch path and diffing the census bodies at all 89 stops. **Both must be empty.**

**If the golden gate drifts here, STOP and diagnose — do not re-bless.** Part B changes no coordinate; a drifted probe means the arc form is not spelling the same ring, and `the_resolved_geometry_is_bit_identical_to_the_ring_form` did not catch it because the divergence is in a path that test does not cover (most likely LOD simplification, or an area coming from `timeline_bridge` rather than `partition_bridge`). Find it; do not paper over it.

- [ ] **Step 8: Commit**

```bash
git add crates/map-compile/src/partition_bridge.rs crates/map-compile/src/timeline_bridge.rs crates/map-compile/src/tests.rs crates/map-provider/src/canon_provider.rs crates/map-provider/src/tests.rs data/canon/canon.json
git commit -m "Stage 3 Task 8: the canon stores canonical arcs; neighbours share one border row, not two copies"
```

---

### Task 9: `borders(entity, at)` on the wire, with provenance

Spec §3's fact tier: `borders(entity, at) → boundary cycles + witness provenance`. Spec §5: "Adds `borders` with provenance."

**A design decision, stated because the contract corpus has to live with it.** The honest full answer to `borders` is coordinates, and the canon now holds ~800k of them. A whole-body fixture of that size is unreviewable, and diagnosis §6.5 already flags the 2.0–2.4 MB scene fixtures as a real repository cost. So `/api/borders` answers with the *structure* whole — every cycle, every arc id in order with its orientation, every arc's witness set and vertex count and content digest — and the coordinates come through the existing content-addressed `/api/resource` path, whose byte-identity laws are already green (diagnosis §4). The whole body is pinned; the geometry is reachable and already lawful; nothing is hidden behind an existential poke.

**Files:**
- Create: `contracts/map-api/fact/borders.feature`
- Create: `contracts/map-api/fixtures/borders-canaan-1405.json`, `borders-judah-1405.json`, `borders-promise-1405.json`, `borders-absent-1405.json`
- Modify: `crates/map-viewer/src/lib.rs` (new route; `/api/contract` gains the arrangement statement)
- Modify: `contracts/runner/src/Steps.hs`, `contracts/runner/src/Capture.hs`
- Test: `crates/map-viewer/src/lib.rs`'s test module or `crates/map-provider/src/tests.rs` (whichever holds the route tests today — follow the file that already tests `/api/census`)

**Interfaces:**
- Consumes: `CanonStore::{arcs, cycle_points}`, `Area.cycles` (Tasks 6, 8); Stage 1's `Registry::resolve` for the `entity` parameter, so `borders` and `census` speak the same identities.
- Produces:
  - `GET /api/borders?entity=<id>&year=<y>` → the body below.
  - `/api/contract`'s body gains `"arrangement": {"faces": N, "residualSr": "<sci>", "arcs": M}` — equation 2 stated on the wire.
  - Haskell: `Capture.EntityRef`, and the steps `I GET /api/borders?entity=<e>&year=<y> as <name>` / `every arc in <name> names at least one witness` / `<name>'s cycles close`.
  Task 11's derivability sampling consumes the route; Stage 4 consumes it as the border-geometry source that replaces legacy routes.

Body shape (the whole answer; `arcs` sorted by id, `cycles` in canonical order):

```json
{
  "entity": "partition:judah",
  "year": -1405,
  "cycles": [ [ {"arc": "0a1b2c3d4e5f6071", "orientation": "forward"}, … ] ],
  "holes": [],
  "arcs": {
    "0a1b2c3d4e5f6071": { "witnesses": ["benjamin", "judah"], "vertices": 2, "resource": "0a1b2c3d4e5f6071" }
  },
  "tenure": "held",
  "arrangement": { "faces": 1287, "residualSr": "8.9e-13" }
}
```

- [ ] **Step 1: Write the feature — contract first, runner red**

`contracts/map-api/fact/borders.feature`:

```gherkin
Feature: borders — every edge, once, in the one arrangement
  Spec §2: a border is ONE canonical edge in THE one arrangement,
  content-addressed, carrying the witness set whose geometry it
  realizes; an extent is a REFERENCE to those edges, never a copy of
  their coordinates. Equation 2 adds the completeness law: the faces
  of the arrangement partition the sphere, so their areas sum to 4π.

  These scenarios are those laws. The bodies are pinned whole —
  every cycle, every arc, every witness set — with coordinates
  reached through the content-addressed resource path whose byte
  laws resources.feature already holds. An entity that stands at the
  year but bounds nothing, and an entity that does not stand at all,
  both have answers here: absence is an empty extent, never an error.

  Vocabulary:
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | orientation | any of: forward, reverse |

  Scenario: the promise's whole extent at the conquest
    When I GET /api/borders?entity=authored:the-land-promised-num-34&year=-1405
    Then the response equals fixture "borders-promise-1405"

  Scenario: an allotment's whole extent at the conquest
    When I GET /api/borders?entity=partition:judah&year=-1405
    Then the response equals fixture "borders-judah-1405"

  Scenario: the named land's whole extent at the conquest
    When I GET /api/borders?entity=partition:canaan&year=-1405
    Then the response equals fixture "borders-canaan-1405"

  Scenario: an entity that does not stand has an empty extent, not an error
    When I GET /api/borders?entity=partition:judah&year=-2247
    Then the response equals fixture "borders-absent-1405"

  Scenario: a border two neighbours share is ONE arc, walked both ways
    When I GET /api/borders?entity=partition:judah&year=-1405 as judah
    And I GET /api/borders?entity=partition:benjamin&year=-1405 as benjamin
    Then judah and benjamin share an arc, walked in opposite directions

  Scenario: every arc names the witnesses whose geometry it realizes
    When I GET /api/borders?entity=partition:canaan&year=-1405 as canaan
    Then every arc in canaan names at least one witness

  @property
  Scenario: an extent's cycles close, at any year
    When I GET /api/borders?entity=partition:canaan&year=<someYear> as extent
    Then extent's cycles close

  @property
  Scenario: borders are deterministic at any year
    When I GET /api/borders?entity=partition:canaan&year=<someYear> as first
    And I GET /api/borders?entity=partition:canaan&year=<someYear> as second
    Then first equals second
```

**Note on `every arc … names at least one witness`.** That reads as an existential poke and the owner's law forbids those. It is not one: the assertion is universally quantified over the whole arc set and the step must refuse to pass on an empty `arcs` object (diagnosis §6.3's exact lesson — the piece-attribution step that passed vacuously). Write the step that way and say so in its comment.

- [ ] **Step 2: Run the runner and watch it go red**

Run (from `contracts/runner/`): `cabal run contract-runner -- check ../map-api`
Expected: FAIL — the totality law names four orphan steps.

- [ ] **Step 3: Write the Rust route test**

Beside the existing `/api/census` route test:

```rust
#[test]
fn borders_answers_whole_for_a_standing_entity_and_empty_for_an_absent_one() {
    let app = test_app_with_canon();
    let body = get_json(&app, "/api/borders?entity=partition:judah&year=-1405");
    // whole-body: the shape is fixed and every key is named
    assert_eq!(
        body.as_object().expect("object").keys().collect::<Vec<_>>(),
        vec!["arcs", "arrangement", "cycles", "entity", "holes", "tenure", "year"]
    );
    assert_eq!(body["entity"], "partition:judah");
    assert_eq!(body["year"], -1405);
    assert_eq!(body["tenure"], "held");
    let arcs = body["arcs"].as_object().expect("arcs");
    assert!(!arcs.is_empty(), "a standing allotment bounds something");
    for (id, a) in arcs {
        assert_eq!(id.len(), 16, "arc ids are 16 lowercase hex digits");
        assert!(
            !a["witnesses"].as_array().expect("witnesses").is_empty(),
            "arc {id} realizes nobody's geometry"
        );
        assert_eq!(a["resource"], serde_json::Value::String(id.clone()));
    }
    // every step's arc is in the arcs table: the body is closed
    for cy in body["cycles"].as_array().expect("cycles") {
        for step in cy.as_array().expect("steps") {
            let k = step["arc"].as_str().expect("arc id");
            assert!(arcs.contains_key(k), "cycle references arc {k}, absent from the table");
            assert!(matches!(step["orientation"].as_str(), Some("forward") | Some("reverse")));
        }
    }
    // the negative case: absence is an empty extent, not a 404
    let gone = get_json(&app, "/api/borders?entity=partition:judah&year=-2247");
    assert_eq!(gone["cycles"], serde_json::json!([]));
    assert_eq!(gone["arcs"], serde_json::json!({}));
    assert_eq!(gone["year"], -2247);
}

#[test]
fn the_contract_declares_the_arrangement_completeness() {
    // Equation 2 on the wire: Σ faces = 4π, stated where a consumer
    // can read it and the contract suite can pin it.
    let app = test_app_with_canon();
    let c = get_json(&app, "/api/contract");
    let a = &c["arrangement"];
    assert!(a["faces"].as_u64().expect("faces") > 0);
    let residual: f64 = a["residualSr"].as_str().expect("residual").parse().expect("number");
    assert!(residual < 1e-10, "the completeness law holds on the served world: {residual:e}");
}
```

- [ ] **Step 4: Run and watch it fail**

Run: `cargo test -p map-viewer borders`
Expected: FAIL — the route 404s, so `get_json` errors.

- [ ] **Step 5: Implement the route**

In `crates/map-viewer/src/lib.rs`, beside `"/api/census"`:

```rust
"/api/borders" => {
    // Spec §3 fact tier: boundary cycles + witness provenance. The
    // structure comes back WHOLE; the coordinates come through the
    // content-addressed resource path, whose byte-identity laws are
    // already the contract's.
    let Some(id) = p.get("entity") else { return bad("entity required") };
    let Some(year) = p.year("year") else { return bad("year required (no year zero)") };
    let Some(canon) = app.canon.as_ref() else {
        return bad("borders requires the canon (run map-compile build)");
    };
    let store = canon.store();
    let entity = map_canon::EntityId(id.to_string());
    // Stage 1's registry: borders and census must speak one identity
    let entity = store.registry().resolve(&entity).clone();

    let mut cycles = Vec::new();
    let mut holes = Vec::new();
    let mut arcs: std::collections::BTreeMap<String, serde_json::Value> = Default::default();
    let mut tenure = "absent";
    let step_json = |s: &map_canon::CycleStep| {
        serde_json::json!({
            "arc": format!("{:016x}", s.arc.0 .0),
            "orientation": match s.orientation {
                map_types::Orientation::Forward => "forward",
                map_types::Orientation::Reverse => "reverse",
            },
        })
    };
    for (_layer, world) in store.layers() {
        let Some(sid) = world.state_at(&year) else { continue };
        let Some(snap) = store.snapshots().get(&sid) else { continue };
        for fid in &snap.features {
            let Some(map_canon::Feature::Area(a)) = store.features().get(fid) else { continue };
            if a.entity != entity {
                continue;
            }
            tenure = match a.tenure {
                map_canon::Tenure::Held => "held",
                map_canon::Tenure::Claimed => "claimed",
            };
            for (dst, src) in [(&mut cycles, &a.cycles), (&mut holes, &a.holes)] {
                for c in src {
                    dst.push(serde_json::Value::Array(c.0.iter().map(step_json).collect()));
                    for s in &c.0 {
                        let key = format!("{:016x}", s.arc.0 .0);
                        let arc = &store.arcs()[&s.arc];
                        arcs.insert(
                            key.clone(),
                            serde_json::json!({
                                "witnesses": arc.witnesses,
                                "vertices": arc.pts.len(),
                                "resource": key,
                            }),
                        );
                    }
                }
            }
        }
    }
    let body = serde_json::json!({
        "entity": entity.0,
        "year": year.year.get(),
        "cycles": cycles,
        "holes": holes,
        "arcs": arcs,
        "tenure": tenure,
        "arrangement": app.arrangement_statement(),
    });
    (200, "application/json", body.to_string(), Vec::new())
}
```

`App::arrangement_statement()` returns `{"faces": …, "residualSr": …, "arcs": …}`; the face count and residual are recorded into the canon at compile time (add two numbers to the persisted root — the compile already computes both at `crates/map-compile/src/partition_bridge.rs:489-491` and today only prints them into a summary string). Add the same object to `/api/contract`'s body.

**If Stage 1's `store.registry()` does not exist** (Stage 1 not landed), drop the `resolve` line and use the raw `EntityId`, and say so in the feature's preamble. Do not invent a registry here.

- [ ] **Step 6: Write the Haskell steps and bless the fixtures**

Add to `contracts/runner/src/Capture.hs` an `EntityRef` newtype with an `Enumerated`-free `Described` universe ("a canonical entity id as `census` reports it"), and to `Steps.hs`:

```haskell
  , mkStep Then (lit "" *> ((,) <$> capUntil @BindName " and "
                                <*> capUntil @BindName " share an arc, walked in opposite directions")) $
      \(a, b) w -> do
        -- Equation 2, as an assertion about two whole answers: the
        -- shared rows are named and their directions differ. Refuses
        -- to pass when the shared set is EMPTY, which is the way this
        -- law would otherwise be satisfiable by two disjoint extents.
        ...
  , mkStep Then (lit "every arc in " *> capUntil @BindName " names at least one witness") $
      \n w -> do
        -- universally quantified, and vacuity is a FAILURE: an empty
        -- arcs table means the answer bounded nothing, which is a
        -- different fact and must not report this law satisfied
        ...
  , mkStep Then (lit "" *> capUntil @BindName "'s cycles close") $
      \n w -> do
        -- every consecutive pair of steps shares a vertex and the
        -- last meets the first; checked structurally on arc ids and
        -- orientations, which is possible because a canonical arc's
        -- endpoints are shared by construction
        ...
```

Then bless: `cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api --bless borders` (follow the corpus's existing blessing invocation), and **read the four fixtures before committing them.** `borders-promise-1405.json` in particular is the one Task 11 will change; knowing its pre-migration shape is what makes that change reviewable.

- [ ] **Step 7: All gates**

Run: `cabal run contract-runner -- check ../map-api` → `totality: every step has exactly one definition`
Run: `cabal run contract-runner -- vocab ../map-api` → `vocabulary: every table matches its types`
Run: `cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api` → all green, no new reds
Run: `node crates/map-viewer/tests/golden.js --check` → `ALL GOLDEN VIEWS HOLD` (a read-only route cannot move a pixel; if it does, something else in this task did)
Census diff across all 89 stops → empty.

- [ ] **Step 8: Commit**

```bash
git add contracts/map-api/fact/borders.feature contracts/map-api/fixtures/borders-*.json contracts/runner/src/Steps.hs contracts/runner/src/Capture.hs crates/map-viewer/src/lib.rs crates/map-compile/src/partition_bridge.rs crates/map-canon/src/persist.rs
git commit -m "Stage 3 Task 9: borders on the wire — cycles by reference, provenance per arc, 4π declared"
```

---

## Part C — one arrangement, where geometry actually moves

> ### OWNER GATE — read before starting Task 10
>
> Parts A and B moved no pixel and the gates proved it. Part C is different and the plan says so plainly rather than discovering it in Task 11.
>
> **Spec §5 asks Stage 3 for two things that are in tension.** "Census empty-diff" and "the golden gate is the hard judge" can both be satisfied — the census (`crates/map-canon/src/lib.rs:783-835`) records entity, name, layer, kind and tenure, and **is blind to geometry**, so a border can move a hundred kilometres without moving a census row. But "the promise's west border IS the coastline" is, by construction, a visual change to the beloved 1405/1446 BC views. The golden gate cannot both hold at 89/89 unchanged *and* judge a deliberate border move.
>
> **What this plan does about it.** Task 10 is written so that both gates stay clean: extents change shape only where they were wrong, tenure is carried so the census cannot move, and if the gate drifts in Task 10 that is a bug. Task 11 is the one that moves pixels on purpose. It declares its expected drift *before* running the gate, presents the drift to the owner as a reviewed list, and re-blesses only on explicit approval.
>
> **If the owner wants Stage 3 to ship with zero re-blessing**, stop after Task 10 and split Task 11 into its own stage (call it 3b). Tasks 1–10 and 12's closing work are then a complete, shippable, revertible stage: every border canonical, `borders` on the wire with provenance, tolerances declared, census empty, golden gate untouched. Only "survey circuits enter the partition" would move to 3b. **This is the recommendation to put to the owner at the Part B/C boundary, and Task 10's review is the moment to put it.**

---

### Task 10: Extents overlap; tenure rides through the arrangement

Two defects block Task 11, and both are in the same twenty lines.

**One face, one owner.** `bundle_faces` (`crates/map-compile/src/partition_bridge.rs:258-280`) assigns each face to `face.claims.iter().find(|c| !absent.contains(*c))` — the *first present* claimant, and no other. So a face belongs to exactly one entity. That is fine while every claimant is a held polity, and fatal the moment a claim overlaps them: the promise covers ground the tribes also cover, and under the current rule the promise would receive only the faces nobody more specific claims — its extent would shrink to scraps. Spec §2 says the opposite: an extent is "entity × era → boundary cycles / face set, resolved in the arrangement by the precedence law", and `PFace.claims` (`crates/map-partition/src/lib.rs:177-178`) **already stores every claimant, sorted by specificity.** The information is there; `bundle_faces` throws it away. Precedence decides paint order, not ownership.

**Tenure is hardcoded.** `crates/map-compile/src/partition_bridge.rs:605` writes `tenure: map_canon::Tenure::Held` for every partition-derived area. Any survey that entered the partition would silently become held ground — the census would flip `claimed` → `held` for the promise, and the renderer would fill it (`crates/map-provider/src/canon_provider.rs:427-433` makes a Claimed area's fill fully transparent). That is the Judea incident's exact shape: a phantom state painting itself because nothing carried the tenure. A witness must carry its tenure into the arrangement.

**Files:**
- Modify: `crates/map-partition/src/build.rs` (`WitnessRegion` gains `entity` and `tenure`; the precedence sort keeps its measured specificity rule and gains a tenure rule)
- Modify: `crates/map-partition/src/lib.rs` (`PFace` gains `held_by`; `Partition::extents`)
- Modify: `crates/map-compile/src/partition_bridge.rs` (`bundle_faces` → `extents`; tenure carried; entity carried)
- Test: `crates/map-partition/src/tests.rs`, `crates/map-compile/src/tests.rs`

**Interfaces:**
- Consumes: `map_types::Tenure` (Task 6); `Partition::dissolve_arcs` (Task 7).
- Produces:
  - `WitnessRegion { id, entity, kind, tenure, rings, parent }` — `entity` is the canon `EntityId` string this witness realizes, so identity is declared at the witness rather than reconstructed by prefixing at `:592-599`.
  - `PFace.held_by: Option<String>` — the single winning *held* claimant (paint order's answer), distinct from `claims` (the extent membership answer).
  - `Partition::extents(&self, absent: &BTreeSet<String>) -> BTreeMap<String, BTreeSet<FaceId>>` — a face may appear in several entries.
  Task 11 supplies survey witnesses through this shape; Task 12's derivability law reads `held_by`.

- [ ] **Step 1: Write the failing partition tests**

Append to `crates/map-partition/src/tests.rs`:

```rust
// ------------------------------------------- extents may overlap
//
// Spec §2: an extent is a FACE SET resolved by the precedence law,
// and claims are not exclusive. A promise covering ground three
// tribes also cover has an extent containing all of it; the tribes
// have theirs; the precedence law says who PAINTS, not who owns.

#[test]
fn a_claim_and_the_held_ground_beneath_it_both_have_extents() {
    let held = WitnessRegion {
        id: "judah".into(),
        entity: "partition:judah".into(),
        kind: FaceKind::LandClaim,
        tenure: map_types::Tenure::Held,
        rings: vec![square(31.0, 35.0, 32.0, 36.0)],
        parent: None,
    };
    let promise = WitnessRegion {
        id: "promise".into(),
        entity: "authored:promise".into(),
        kind: FaceKind::LandClaim,
        tenure: map_types::Tenure::Claimed,
        rings: vec![square(30.5, 34.5, 32.5, 36.5)],
        parent: None,
    };
    let p = build(&[held, promise], &[], &cfg()).expect("builds");
    let ex = p.extents(&Default::default());

    // The promise's extent is every face inside its ring — INCLUDING
    // the faces Judah holds. That is the whole point: a promise is
    // not diminished by being kept.
    let judah = &ex["judah"];
    let promise_set = &ex["promise"];
    assert!(!judah.is_empty(), "the held claim has an extent");
    assert!(
        judah.is_subset(promise_set),
        "the promise's extent contains the ground held within it"
    );
    assert!(
        promise_set.len() > judah.len(),
        "and extends beyond it"
    );
    // and the negative case: an entity nobody claims has no extent
    assert!(!ex.contains_key("nobody"));
}

#[test]
fn precedence_names_the_holder_not_the_claimant() {
    // held_by answers "who paints this ground", and a Claimed
    // witness never wins it — the tenure law at its source. Without
    // this, a promise laid over an allotment would paint over it.
    let held = WitnessRegion {
        id: "judah".into(),
        entity: "partition:judah".into(),
        kind: FaceKind::LandClaim,
        tenure: map_types::Tenure::Held,
        rings: vec![square(31.0, 35.0, 32.0, 36.0)],
        parent: None,
    };
    let promise = WitnessRegion {
        id: "promise".into(),
        entity: "authored:promise".into(),
        kind: FaceKind::LandClaim,
        tenure: map_types::Tenure::Claimed,
        // SMALLER than judah, so the measured-specificity rule would
        // hand it the face if tenure did not outrank specificity
        rings: vec![square(31.2, 35.2, 31.4, 35.4)],
        parent: None,
    };
    let p = build(&[held, promise], &[], &cfg()).expect("builds");
    let inner: Vec<&PFace> = p
        .faces
        .iter()
        .filter(|f| f.claims.contains(&"promise".to_string()))
        .collect();
    assert!(!inner.is_empty(), "the promise claims something");
    for f in inner {
        assert_eq!(
            f.held_by,
            Some("judah".to_string()),
            "a Claimed witness must never become the holder of ground someone holds"
        );
    }
}

#[test]
fn a_claim_over_unheld_ground_has_no_holder_at_all() {
    // The other half: a promise over ground nobody holds does not
    // become the holder by default. Unclaimed ground stays unnamed
    // and the renderer paints nothing — a promise renders as
    // boundary and name, never as territory.
    let promise = WitnessRegion {
        id: "promise".into(),
        entity: "authored:promise".into(),
        kind: FaceKind::LandClaim,
        tenure: map_types::Tenure::Claimed,
        rings: vec![square(31.0, 35.0, 32.0, 36.0)],
        parent: None,
    };
    let p = build(&[promise], &[], &cfg()).expect("builds");
    let claimed: Vec<&PFace> =
        p.faces.iter().filter(|f| f.claims.contains(&"promise".to_string())).collect();
    assert!(!claimed.is_empty());
    for f in claimed {
        assert_eq!(f.held_by, None);
    }
}
```

- [ ] **Step 2: Run and watch them fail**

Run: `cargo test -p map-partition extent`
Expected: FAIL — `struct 'WitnessRegion' has no field named 'entity'`.

- [ ] **Step 3: Implement in map-partition**

`WitnessRegion` gains:

```rust
pub struct WitnessRegion {
    pub id: String,
    /// The canon entity this witness realizes. DECLARED at the
    /// witness rather than reconstructed downstream: identity is
    /// spec equation 1's business, not a prefixing rule in a bridge.
    pub entity: String,
    pub kind: FaceKind,
    /// HOW this witness relates to its ground. A Claimed witness
    /// takes part in the arrangement — it bounds cells and its edges
    /// are canonical — but it never becomes a face's holder: a
    /// promise renders as boundary and name, never as territory.
    pub tenure: map_types::Tenure,
    pub rings: Vec<Vec<UnitVec>>,
    pub parent: Option<String>,
}
```

`PFace` gains `pub held_by: Option<String>`. In the classification block (`crates/map-partition/src/build.rs:496-550`), keep the existing `kinds` sort exactly as it is — water over land, then measured specificity by parent depth then witness area then id, which is the precedence law and must not change — and after it:

```rust
face.claims = kinds.iter().map(|(w, _, _)| w.clone()).collect();
// THE TENURE LAW at the arrangement: `claims` is extent membership
// and may be many; `held_by` is who PAINTS and is at most one. A
// Claimed witness is never the holder, however specific it is — the
// promise's remainder once painted itself as a phantom state, and
// that is the defect this line forecloses.
face.held_by = kinds
    .iter()
    .find(|(w, _, _)| wit_tenure.get(w) == Some(&map_types::Tenure::Held))
    .map(|(w, _, _)| w.clone());
face.kind = kinds
    .iter()
    .find(|(w, _, _)| wit_tenure.get(w) == Some(&map_types::Tenure::Held))
    .map(|(_, k, _)| k.clone())
    .unwrap_or_else(|| kinds[0].1.clone());
```

with `wit_tenure: BTreeMap<String, Tenure>` gathered alongside `wit_depth` and `wit_area` (`:186-214`). And:

```rust
/// EVERY ENTITY'S EXTENT: witness id → the faces it claims, present
/// claimants only. A face appears in as many extents as claim it —
/// overlap is meaning (spec §2), and `held_by` is what resolves the
/// paint. `absent` names claimants not yet standing in the era.
pub fn extents(
    &self,
    absent: &std::collections::BTreeSet<String>,
) -> std::collections::BTreeMap<String, std::collections::BTreeSet<FaceId>> {
    let mut out: std::collections::BTreeMap<String, std::collections::BTreeSet<FaceId>> =
        Default::default();
    for (fi, f) in self.faces.iter().enumerate() {
        if f.kind == FaceKind::Background {
            continue;
        }
        for c in &f.claims {
            if !absent.contains(c) {
                out.entry(c.clone()).or_default().insert(fi);
            }
        }
    }
    out
}
```

- [ ] **Step 4: Write the failing compile-side test**

Append to `crates/map-compile/src/tests.rs`. This is also where the forbidden-shape assertions in the existing `plate_partition_face_census` (`:376-410`) are replaced: rewrite its tribe loop from `assert!(faces.iter().any(...))` into a whole-body comparison of the sorted list of every claimant that names at least one face against a pinned list. Same evidence, an assertion that names what changed.

```rust
#[test]
#[ignore = "compiles the real arrangement; run explicitly"]
fn every_claimant_of_the_real_arrangement_is_exactly_this_list() {
    let (regions, polylines) = crate::partition_bridge::gather_witnesses(&[]).expect("gather");
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let tol = crate::tolerances::load(&text).expect("ledger");
    let p = map_partition::build(
        &regions,
        &polylines,
        &map_partition::PartitionConfig::from_tolerances(&tol),
    )
    .expect("builds");
    let mut who: Vec<String> = p.extents(&Default::default()).into_keys().collect();
    who.sort();
    assert_eq!(
        who,
        vec![
            "ammon", "asher", "benjamin", "canaan", "dan", "dead-sea-0", "edom", "ephraim",
            "gad", "geshur", "great-sea", "issachar", "jordan", "judah", "manasseh-east",
            "manasseh-west", "moab", "naphtali", "philistia", "phoenicia", "reuben",
            "sea-of-galilee-0", "simeon", "zebulun",
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>(),
        "the claimant set of the plate core, WHOLE — a leaked or lost witness fails here"
    );
}

#[test]
#[ignore = "compiles the real arrangement; run explicitly"]
fn a_claimed_witnesss_tenure_survives_the_bridge() {
    // The census-safety law for Task 11: a Claimed witness that
    // enters the arrangement comes out of the bridge Claimed. The
    // hardcoded Tenure::Held at partition_bridge.rs:605 is exactly
    // what would have flipped the promise to held ground.
    let mut store = map_canon::CanonStore::default();
    let regions = vec![
        map_partition::WitnessRegion {
            id: "held-one".into(),
            entity: "partition:held-one".into(),
            kind: map_partition::FaceKind::LandClaim,
            tenure: map_types::Tenure::Held,
            rings: vec![vec![
                map_types::UnitVec::from_lat_lon_deg(31.0, 35.0),
                map_types::UnitVec::from_lat_lon_deg(31.0, 36.0),
                map_types::UnitVec::from_lat_lon_deg(32.0, 36.0),
                map_types::UnitVec::from_lat_lon_deg(32.0, 35.0),
            ]],
            parent: None,
        },
        map_partition::WitnessRegion {
            id: "claim-one".into(),
            entity: "authored:claim-one".into(),
            kind: map_partition::FaceKind::LandClaim,
            tenure: map_types::Tenure::Claimed,
            rings: vec![vec![
                map_types::UnitVec::from_lat_lon_deg(30.5, 34.5),
                map_types::UnitVec::from_lat_lon_deg(30.5, 36.5),
                map_types::UnitVec::from_lat_lon_deg(32.5, 36.5),
                map_types::UnitVec::from_lat_lon_deg(32.5, 34.5),
            ]],
            parent: None,
        },
    ];
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let tol = crate::tolerances::load(&text).expect("ledger");
    let p = map_partition::build(
        &regions,
        &[],
        &map_partition::PartitionConfig::from_tolerances(&tol),
    )
    .expect("builds");
    let ex = crate::partition_bridge::extents_for_law(&mut store, &p, &regions, &Default::default());
    assert_eq!(ex["held-one"].tenure, map_types::Tenure::Held);
    assert_eq!(ex["claim-one"].tenure, map_types::Tenure::Claimed);
    assert_eq!(ex["held-one"].entity, "partition:held-one");
    assert_eq!(ex["claim-one"].entity, "authored:claim-one");
}
```

- [ ] **Step 5: Implement in the bridge**

`Bundle` gains `entity: String` and `tenure: map_types::Tenure`. `bundle_faces` becomes `extents`, built over `part.extents(absent)` rather than a first-present scan, and each bundle dissolves *its own* face set:

```rust
for (who, faces) in part.extents(absent) {
    let w = witness_by_id.get(&who);
    let mut bundle = Bundle {
        entity: w.map(|w| w.entity.clone()).unwrap_or_else(|| format!("partition:{who}")),
        tenure: w.map(|w| w.tenure).unwrap_or(map_types::Tenure::Held),
        kind: /* the largest face's kind, as today */,
        biggest: /* as today */,
        cycles: Vec::new(),
        holes: Vec::new(),
        note: /* as today */,
    };
    for walk in part.dissolve_arcs(&faces) { /* exactly Task 8's block */ }
    bundles.insert(who, bundle);
}
```

The `CohortSpec` fallback at `:592-599` stops synthesizing `format!("partition:{who}")` — the entity now arrives on the witness. Delete the synthesis; keep the spec lookup for layer/witness/verses. `Feature::Area` takes `tenure: bundle.tenure` (deleting the hardcode at `:605`).

Every `WitnessRegion` construction in `gather_witnesses` gains its two new fields:
- `canaan` → `entity: "partition:canaan"`, `tenure: Held`
- seas and lakes → `entity: format!("partition:{id}")`, `tenure: Held`
- corridors → `entity: "partition:jordan"`, `tenure: Held`
- tribal cohorts → `entity: format!("partition:{}", cohort.slug)`, `tenure: Held`
- OpenBible neighbours → `entity: format!("partition:{slug}")`, `tenure: Held`
- atlas polity eras → `entity: row.id.clone()`, `tenure: Held`

**Every one of these is exactly the entity id the census reports today** (verified against `/api/census?year=-1405`: `partition:canaan`, `partition:judah`, …, and bare `egypt`/`assyria`/`babylon` for atlas polities). That is what keeps the census diff empty. Check each against a live census response before running the gates, not after.

- [ ] **Step 6: Run to green**

Run: `cargo test --workspace` then `cargo test -p map-compile -- --ignored`
Expected: PASS.

- [ ] **Step 7: The gates**

Rebuild all three steps.

Run: `node crates/map-viewer/tests/golden.js --check`
Expected: `ALL GOLDEN VIEWS HOLD` (89/89). **Zero drift is required.** No survey has entered the arrangement yet, and every witness that was Held before is Held now, so no face changes its holder and no extent changes its shape. A drift means the precedence rewrite altered a real winner — most likely because a lake or sea witness was given the wrong tenure, or because `face.kind`'s new fallback differs from the old `kinds[0].1` on some face. Diagnose; do not re-bless.

Run the 89-stop census diff. **Expected: EMPTY.** A non-empty diff here is an identity or tenure change and must be reviewed by the owner before anything else happens.

- [ ] **Step 8: Present the split recommendation**

Before committing, put the OWNER GATE above to the owner with the concrete evidence now in hand: Parts A and B held 89/89 with an empty census, Task 10 held 89/89 with an empty census, and Task 11 will not. Ask whether Stage 3 ships here (with Task 11 becoming Stage 3b) or continues.

- [ ] **Step 9: Commit**

```bash
git add crates/map-partition/src/lib.rs crates/map-partition/src/build.rs crates/map-partition/src/tests.rs crates/map-compile/src/partition_bridge.rs crates/map-compile/src/tests.rs data/canon/canon.json
git commit -m "Stage 3 Task 10: extents overlap, precedence names the holder, tenure rides the arrangement"
```

---

### Task 11: Survey circuits become witnesses — the promise's west border IS the coastline

**This is the task that moves pixels, and it is gated.** Do not start it without the owner's answer from Task 10 Step 8.

Spec §5: "Survey circuits enter the partition as witnesses; every border canonical; claims reference edges (the promise's west border IS the coastline)." Spec §2's *what dies*: "surveys-as-geometry-pipeline (circuits become Witness evidence)."

**What is there today.** `crates/map-adapters/src/surveys.rs` holds 25 `SurveySpec` rows (`:837`, `:878`) plus `KINGDOMS` era specs (`:636`) and `ROUTES` (`:1069`). `add_survey` (`:1296-1400`) builds a `Boundary { pts }` by geodesic interpolation between waypoints, closes it, and files it as a `RegionGeom` in a `WorldTimeline`; `crates/map-compile/src/main.rs:234-249` bridges that timeline into `LayerKind::ScriptureClaims` through `bridge_filtered`. The result is geometry authored in parallel with the arrangement and never reconciled against it — which is precisely why the promise's west border is a hand-drawn line offshore rather than the coast.

**What the census says stands.** Verified against the live server: 16 GEN 10 nation hulls plus `canaan-traced-contour` at 1446 BC; `the-land-promised-num-34` from 1446 BC to the monarchy; the four tetrarchies at AD 59; every one `claimed` except the tetrarchies, which are `held` (`Holds::Ground`, and the census confirms it).

**Scope, and the honest boundary.** The 16 GEN 10 hulls span the known world and overlap the atlas polity witnesses at continental scale. Their circuits are short (city-derived hulls over a handful of waypoints), so the arrangement's O(n²) candidate-intersection pass (`crates/map-partition/src/build.rs:224-241`, over ~4,000 segments today) absorbs them cheaply — this was checked, and performance is not the risk. The risk is *semantic*: sixteen world-scale claims entering a Levant-framed arrangement will produce faces far outside the frame, and the arrangement's frame is currently defined by what its witnesses happen to cover.

**Therefore this task migrates the surveys in two cohorts, and the first is the reviewable one:**

1. **Levant cohort** — `NUM 34` (the promise), `canaan-traced-contour`, and the four tetrarchies. These sit inside the arrangement's existing frame, are the ones spec §5's own example names, and are the ones the golden gate can actually see.
2. **World cohort** — the 16 GEN 10 hulls. Migrate them **only after** cohort 1 is green and blessed, as a separate commit, and expect the arrangement's face count to grow substantially. If cohort 1's review shows the frame question is unresolved, leave cohort 2 in the timeline pipeline and record it as Stage 3's declared remainder — a partial migration that is honest beats a total one that is unreviewable.

**Files:**
- Modify: `crates/map-adapters/src/surveys.rs` (export circuits as witness evidence; stop emitting `RegionGeom` for migrated tags)
- Modify: `crates/map-compile/src/partition_bridge.rs` (`gather_witnesses_with` takes the survey witnesses)
- Modify: `crates/map-compile/src/main.rs` (the timeline bridge stops carrying migrated tags)
- Test: `crates/map-compile/src/tests.rs`
- Modify: `contracts/map-api/fixtures/borders-promise-1405.json` (re-blessed — this is the deliverable)

**Interfaces:**
- Consumes: `WitnessRegion { entity, tenure }` (Task 10); `Partition::extents` (Task 10).
- Produces: `map_adapters::survey_witnesses() -> Vec<SurveyWitness>` where `SurveyWitness { tag, entity, label, circuit: Vec<UnitVec>, tenure, stands: (i32, Option<i32>), verses: Vec<String>, provenance: String }`. **This is the Stage 2 seam:** if Stage 2 landed, `stands` and `tenure` come from the ledger rows it built from `Stands`/`Holds`, and `survey_witnesses` reads them rather than the Rust literals. If Stage 2 has not landed, read them from the `SurveySpec` literals exactly as `add_survey` does today, and leave a `// STAGE 2 SEAM` comment at the two lines. Do not build a ledger here.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
#[ignore = "compiles the real arrangement; run explicitly"]
fn the_promises_west_border_is_the_coastline_itself() {
    // Spec §5's own example, as a law. After migration the promise's
    // extent is bounded by arcs the SEA also witnesses — not by a
    // hand-drawn line that merely runs near the coast.
    let (regions, polylines) = crate::partition_bridge::gather_witnesses(&[]).expect("gather");
    let text = std::fs::read_to_string(crate::tolerances::DECLARED_PATH).expect("ledger");
    let tol = crate::tolerances::load(&text).expect("ledger");
    let p = map_partition::build(
        &regions,
        &polylines,
        &map_partition::PartitionConfig::from_tolerances(&tol),
    )
    .expect("builds");
    let ex = p.extents(&Default::default());
    let promise = ex
        .get("the-land-promised-num-34")
        .expect("the promise is a witness of the arrangement");

    // every arc on the promise's boundary that the great sea also
    // bounds is ONE arc — shared, not parallel
    let mut shared_with_sea = 0usize;
    for cycle in p.dissolve_arcs(promise) {
        for (e, _) in cycle {
            if p.edges[e].provenance.iter().any(|w| w.starts_with("great-sea")) {
                shared_with_sea += 1;
            }
        }
    }
    assert!(
        shared_with_sea > 0,
        "the promise's west boundary shares no arc with the sea — it is still \
         drawn beside the coast rather than ON it"
    );
    // and the negative case that makes the above meaningful: no arc
    // of the promise's boundary may be a knife-edge parallel, i.e.
    // an arc witnessed ONLY by the promise while a sea arc runs
    // within tau_edge of its midpoint
    for cycle in p.dissolve_arcs(promise) {
        for (e, o) in cycle {
            if p.edges[e].provenance != vec!["the-land-promised-num-34".to_string()] {
                continue;
            }
            let [a, b] = p.edge_points(e, o);
            let mid = map_types::UnitVec::normalize(
                a.x() + b.x(), a.y() + b.y(), a.z() + b.z(),
            )
            .expect("midpoint");
            for f in 0..p.edges.len() {
                if !p.edges[f].provenance.iter().any(|w| w.starts_with("great-sea")) {
                    continue;
                }
                let [c, d] = p.edge_points(f, map_types::Orientation::Forward);
                let sep = /* perpendicular offset of mid from arc c→d */;
                assert!(
                    sep > tol.tau_edge.radians(),
                    "a promise-only arc runs within tau_edge of a sea arc: a knife-edge \
                     parallel, which is exactly what entering the arrangement removes"
                );
            }
        }
    }
}

#[test]
#[ignore = "compiles the real arrangement; run explicitly"]
fn the_migrated_surveys_keep_their_identity_tenure_and_standing() {
    // THE CENSUS-SAFETY LAW. Whatever happens to the geometry, these
    // five rows must be unchanged in every field the census reports.
    // Run the compile, then read the census back.
    let expected: Vec<(&str, &str, &str, &str)> = vec![
        // (entity, name, layer, tenure) at 1446 BC — copied from the
        // pre-migration /api/census?year=-1446 response
        ("authored:canaan-traced-contour", "Canaan (traced contour)", "scripture-claims", "claimed"),
        ("authored:the-land-promised-num-34", "the land promised (NUM 34)", "scripture-claims", "claimed"),
    ];
    let rows = census_rows_at(-1446);
    for (entity, name, layer, tenure) in expected {
        let row = rows
            .iter()
            .find(|r| r.entity == entity)
            .unwrap_or_else(|| panic!("{entity} vanished from the census"));
        assert_eq!((row.name.as_str(), row.layer, row.tenure), (name, layer, tenure));
    }
}
```

`census_rows_at` calls `map_canon::census(store, &ts)` on the compiled canon; the exact expected strings must be **copied from the live pre-migration census**, not typed from memory. Capture them first:

```bash
curl -s "http://127.0.0.1:8090/api/census?year=-1446" > /tmp/census-1446-before.json
curl -s "http://127.0.0.1:8090/api/census?year=59"    > /tmp/census-59-before.json
```

- [ ] **Step 2: Run and watch them fail**

Run: `cargo test -p map-compile --ignored promises_west_border`
Expected: FAIL — `the promise is a witness of the arrangement` panics, because it is not one yet.

- [ ] **Step 3: Export the circuits as witness evidence**

In `crates/map-adapters/src/surveys.rs`, add beside the existing `authored_routes()` export:

```rust
/// THE SURVEY CIRCUITS, AS EVIDENCE. Spec §2: circuits become
/// Witness evidence and the surveys-as-geometry-pipeline dies. A
/// circuit is a walked line of place references; where it runs is
/// the arrangement's answer, not this file's. The interpolation
/// method stays disclosed — it is how the text's waypoints are
/// joined — but the resulting curve is now a WITNESS presented to
/// the builder, never canonical geometry itself.
pub struct SurveyWitness {
    pub tag: &'static str,
    /// the canon entity this circuit realizes — the SAME id the
    /// census already reports, so identity does not move
    pub entity: String,
    pub label: &'static str,
    pub circuit: Vec<UnitVec>,
    pub tenure: map_types::Tenure,
    /// [from, until) in years; None = to the frame's edge
    pub stands: (i32, Option<i32>),
    pub verses: Vec<String>,
    pub provenance: String,
}

/// The tags migrated into the arrangement. Cohort 1 only; the GEN 10
/// world hulls stay in the timeline pipeline until cohort 2, and
/// this list is the honest record of where the migration has got to.
pub const MIGRATED: &[&str] =
    &["NUM-34", "PLATE-CANAAN", "NT-JUDAEA", "NT-GALILEE", "NT-PEREA", "NT-ITUREA"];

pub fn survey_witnesses(atlas: Option<&AtlasExports>) -> Vec<SurveyWitness> { /* … */ }
```

The `entity` for each is the census's existing slug — `authored:the-land-promised-num-34`, `authored:canaan-traced-contour`, `authored:judea`, `authored:galilee`, `authored:perea`, `authored:iturea-and-trachonitis` — produced by the same `slug()` function `timeline_bridge.rs:16-28` uses on the label. **Call that function rather than transcribing the slugs**, and assert in a test that the six ids match the six the pre-migration census reports. Transcribed slugs are how an identity silently moves.

`tenure` maps from `Holds`: `Holds::Ground(_) => Tenure::Held`, `Holds::Claim => Tenure::Claimed` — the same mapping `timeline_bridge.rs:114-119` performs through `RegionClass`.

In `add_survey`, skip any spec whose tag is in `MIGRATED`: it no longer files a `RegionGeom`. Its `ChangeEvent` (the narrated Rise) stays — the narrative is not geometry and `changes` must not lose a row, which the contract suite's `changes.feature` would catch.

- [ ] **Step 4: Feed them to the arrangement**

In `gather_witnesses_with`, after the OpenBible neighbours block, add:

```rust
// THE SURVEY CIRCUITS enter the ONE arrangement (spec §5). A
// circuit is snapped onto the shared water and parent arcs exactly
// as the tribal cohorts are — so the promise's west border becomes
// the coastline itself, not a line beside it — and it enters with
// its own declared tenure, so a promise still renders as boundary
// and name.
for s in map_adapters::survey_witnesses(atlas) {
    let snapped = snap_ring_to(&s.circuit, &snap_targets, tol.snap_budget.radians());
    if snapped.len() >= 3 {
        regions.push(WitnessRegion {
            id: s.tag_slug(),
            entity: s.entity.clone(),
            kind: FaceKind::LandClaim,
            tenure: s.tenure,
            rings: vec![snapped],
            parent: None,
        });
    }
}
```

and declare each survey's standing in `bridge_partition`'s `PresenceBook` from `s.stands`, exactly as the cohort rings do at `:524-535`. `gather_witnesses_with` needs the `AtlasExports` handle; thread it as a parameter and pass `None` from the law suite (the surveys' unbound fallback years are the spec literals, which is what the law suite wants anyway).

- [ ] **Step 5: Stop the timeline pipeline from carrying them**

In `crates/map-compile/src/main.rs`'s `bridge_filtered` call for `ScriptureClaims` (`:234-249`), add the migrated tags' slugs to the `drops` set. They are now the arrangement's, and a region present in both pipelines would be two features with one entity id at one instant — which `CanonStore::validate`'s territorial-overlap law would then report, loudly and correctly.

- [ ] **Step 6: Run to green, and read the arrangement before the pixels**

Run: `cargo test --workspace` and `cargo test -p map-compile -- --ignored`
Expected: PASS.

Then rebuild all three steps and **read the change before judging it**:

```bash
curl -s "http://127.0.0.1:8090/api/borders?entity=authored:the-land-promised-num-34&year=-1446" > /tmp/promise-after.json
python -c "
import json
a=json.load(open('/tmp/promise-after.json'))
arcs=a['arcs']
sea=[k for k,v in arcs.items() if any(w.startswith('great-sea') for w in v['witnesses'])]
print('arcs', len(arcs), 'shared with the sea', len(sea))
print('tenure', a['tenure'], 'cycles', len(a['cycles']))
"
```

The promise's west boundary should now consist of arcs whose witness sets name both the promise and the sea. That is spec §5's sentence, made checkable.

- [ ] **Step 7: The census gate — must still be EMPTY**

Run the 89-stop census sweep and diff against the captured pre-migration bodies. **Expected: EMPTY at every stop.** The census records entity, name, layer, kind, tenure — none of which this task changes if Steps 3–5 were done right.

**If the census moved, STOP.** The likely causes, in order: an entity slug was transcribed rather than derived (Step 3); a survey's `Holds` mapped to the wrong tenure; a migrated survey lost its layer because `CohortSpec`'s lookup fell through to the `ScriptureClaims` default with a different witness kind; or a survey's standing years differ between the `SurveySpec` literal and the atlas-resolved year that `add_survey` used. Each is a real defect; none is a reason to accept a diff.

- [ ] **Step 8: The golden gate — declare the drift, then measure it**

**Before running the gate, write down what should change**, in the commit message draft:

- **1446 BC and 1405 BC, Levant camera:** the promise's western outline moves onto the coast. Since a Claimed area has transparent fill (`canon_provider.rs:427-433`), the visible change is the *stroke* — a dashed Unknown-character outline moving by up to the trace's own error, a few km, which at the Levant camera (zoom 8) is a handful of pixels. Probes near the coast may drift; probes inland should not.
- **AD 26–59, Levant camera:** the four tetrarchies are `held`, so they *fill*. Their outlines becoming flush with the coast and with each other will move fill boundaries. This is the largest expected change in the stage.
- **Hemisphere camera, every stop:** should be unchanged — the migrated cohort is Levant-framed and at zoom 90 a few km is sub-pixel.
- **Every stop before 1446 BC and after AD 59:** unchanged — no migrated survey stands there.

Then run: `node crates/map-viewer/tests/golden.js --check`

Compare the reported drift against the declaration above. **A drift the declaration did not predict is a bug, not a visual change** — chase it before showing anyone the pictures. In particular: any drift at the hemisphere camera, or at a stop where no migrated survey stands, means something moved that had no business moving.

- [ ] **Step 9: OWNER GATE — approve the visual change, then re-bless**

Present to the owner: the drift list, side-by-side screenshots at 1446 BC and 1405 BC (Levant camera, the beloved views) and at AD 59, and the `borders` before/after for the promise. **Do not re-bless without an explicit approval.**

On approval:

```bash
node crates/map-viewer/tests/golden.js          # re-bless
node crates/map-viewer/tests/golden.js --check  # confirm the new blessing holds
```

and re-bless `borders-promise-1405.json` and any other affected contract fixture, with a `contracts/CHANGELOG.md` entry naming the change and the approval.

- [ ] **Step 10: Commit**

```bash
git add crates/map-adapters/src/surveys.rs crates/map-compile/src/partition_bridge.rs crates/map-compile/src/main.rs crates/map-compile/src/tests.rs crates/map-viewer/tests/fixtures/golden-views.json contracts/map-api/fixtures/borders-promise-1405.json contracts/CHANGELOG.md data/canon/canon.json
git commit -m "Stage 3 Task 11: survey circuits enter the one arrangement; the promise's west border IS the coastline"
```

- [ ] **Step 11 (optional, owner-gated): cohort 2**

If the owner wants the GEN 10 world hulls migrated too, repeat Steps 3–10 with those 16 tags added to `MIGRATED`, as a separate commit. Expect the arrangement's face count to grow substantially and the 1446 BC hemisphere view to change. If the owner does not, record the remainder in `contracts/CHANGELOG.md` and in Task 12's closing section — **an undeclared partial migration is the thing to avoid, not a partial migration.**

---

### Task 12: Derivability, the completeness law on the wire, and closing the stage at 0.4.0

Spec §4's derivability clause — "every manifest entry must be traceable to `disposition` + `borders` answers — contract-tested by sampling" — has **no scenario anywhere** (diagnosis §9.1, item 2: "arguably the most important omission on the list: the whole architecture rests on the two tiers agreeing, and nothing checks that they do"). It becomes fully testable exactly now, because `borders` exists only after Task 9. If Stage 1 or 2 wrote it as a `@target`, this task makes it real; if neither did, this task writes it.

**Files:**
- Modify: `contracts/map-api/scene/scene.feature` (the derivability scenario)
- Create: `contracts/map-api/fixtures/derivability-1405.json`
- Modify: `contracts/runner/src/Steps.hs`
- Modify: `contracts/VERSION`, `contracts/CHANGELOG.md`
- Modify: `crates/map-partition/src/tests.rs` (extend the literal law's file list)

**Interfaces:**
- Consumes: `/api/borders` (Task 9), `/api/census` and its diff form (Stage 1), `/api/disposition` (**Stage 2** — see the fallback below), `/api/scene` with `piece` on every manifest entry (Stage 1's Task 10).
- Produces: the stage's closing declaration. See the Stage 4 handoff section below.

- [ ] **Step 1: Write the derivability scenario**

Append to `contracts/map-api/scene/scene.feature` (and update its preamble to say the derivability law now lives there):

```gherkin
  Scenario: every manifest entry is traceable to a disposition and a border
    Spec §4 binds the two tiers: a manifest entry is not free-floating
    geometry. For a declared sample of entries at the conquest, the
    WHOLE trace is pinned — entry, piece, entity, the disposition that
    answered for it, and the border arcs its geometry resolves to.
    A sample that traces to nothing fails; a sample list that shrinks
    fails too, because a law with no inputs is not a law.

    When I render pieces fills, borders at year -1405 in style canaan as scene
    And I trace scene's entries against disposition and borders as trace
    Then trace equals fixture "derivability-1405"
```

The step's implementation samples deterministically (the corpus's existing per-iteration seeding, so the sample is reproducible), and for each sampled manifest entry:

1. reads its `piece` and `entity` (Stage 1's attribution);
2. calls `/api/disposition?entity=…&at=-1405` and records the whole answer;
3. calls `/api/borders?entity=…&year=-1405` and records the set of arc ids;
4. asserts the entry's geometry resource id is one of those arc ids, or — for a `fills` entry, whose resource is an assembled ring rather than a single arc — that every arc the entry's ring is spelled from appears in the `borders` answer.

The fixture pins the whole trace for all sampled entries. **The sample size and the seed are written into the scenario's fixture**, so a later change that quietly samples fewer entries fails the fixture rather than passing more easily.

**Stage 2 fallback.** If `/api/disposition` does not exist, split the scenario in two: keep the `borders` half real and un-tagged (it is fully testable now and it is this stage's own deliverable), and write the `disposition` half as a separate `@target` scenario naming Stage 2 in its text. **Do not tag the whole law `@target`** — that would hide the half this stage is responsible for behind the half it is not.

- [ ] **Step 2: Run the runner and watch it go red**

Run: `cabal run contract-runner -- check ../map-api` → orphan step named.
Then implement the step, bless the fixture, and re-run.

- [ ] **Step 3: Extend the bare-float law to every geometry module**

In `crates/map-partition/src/tests.rs`, add `map-canon/src/lib.rs` and `map-compile/src/partition_bridge.rs` to `the_geometry_modules_contain_no_undeclared_float`'s file list — as `include_str!` paths from a small test in each of those crates rather than reaching across crate boundaries. Task 5 covered `map-partition`; this closes the other two, and it is deliberately last so the list is extended over code that has stopped changing.

Run: `cargo test --workspace literal`
Expected: PASS, or a named list of literals to declare. Any literal introduced by Tasks 6–11 gets a ledger row or is removed. **This is the sweep that stops Part A's regime from having been theatre.**

- [ ] **Step 4: Run every gate, in order, and record the numbers**

```bash
# 1. Rust
cargo test --workspace
cargo test -p map-compile -- --ignored          # the plateau law and the arrangement laws

# 2. the contract suite
cd contracts/runner
cabal test
cabal run contract-runner -- check ../map-api
cabal run contract-runner -- check ../atlas-edge
cabal run contract-runner -- vocab ../map-api
cabal run contract-runner -- vocab ../atlas-edge
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
cabal run contract-runner -- run --base-url http://127.0.0.1:8080 ../atlas-edge

# 3. the census — all 89 stops, against the stage's opening canon
#    (see Task 8 Step 7 for the sweep)

# 4. the golden gate
node crates/map-viewer/tests/golden.js --check
```

Every one must be green. The census sweep must be **empty** relative to the stage's opening state — Part C's re-blessing covers pixels, never census rows.

- [ ] **Step 5: Move the version and write the changelog**

`contracts/VERSION` → `0.4.0`.

`contracts/CHANGELOG.md`, under a new `## 0.4.0 — 2026-09-07` heading, stating at minimum:
- `borders.feature` is new (additive), with its four fixtures;
- `scene.feature` gains the derivability scenario (additive, or a MINOR-worthy edit if an existing scenario was rewritten — say which);
- `/api/contract`'s body gains `arrangement` (additive; it is a *body* change to an existing blessed fixture, so `contract.json` is re-blessed and that re-blessing is named here);
- if Task 11 ran: the golden views were re-blessed, under whose approval, on what date, for what change;
- if cohort 2 did not run: **the 16 GEN 10 survey circuits remain outside the arrangement**, named, as Stage 3's declared remainder.

- [ ] **Step 6: Write the stage's closing note**

Append to `docs/notes/2026-09-06-contract-diagnosis.md` — no. Create `docs/notes/2026-09-07-stage3-arrangement.md` with: the arc count and total point count before and after (Task 8 Step 6's reading), the arrangement's face count and 4π residual, the measured tolerance populations from Task 3 and the plateau widths from Task 4 as a table, the census sweep result, the golden gate result and any re-blessing, and **the list of things this stage did not do** — cohort 2 if it did not run, the natural-earth water layer's 731 features that are still bridged outside the arrangement, and the 362 historical-basemap background areas and 6 relief bands likewise. Equation 2 holds *of the arrangement*; it does not yet hold *of the whole canon*, and saying so is the difference between a stage that closed and a stage that claimed to.

- [ ] **Step 7: Commit**

```bash
git add contracts/VERSION contracts/CHANGELOG.md contracts/map-api/scene/scene.feature contracts/map-api/fixtures/derivability-1405.json contracts/map-api/fixtures/contract.json contracts/runner/src/Steps.hs crates/map-partition/src/tests.rs crates/map-canon/src/tests.rs crates/map-compile/src/tests.rs docs/notes/2026-09-07-stage3-arrangement.md
git commit -m "Stage 3 close: derivability tested, 4π on the wire, contracts 0.4.0"
```

---

## What Stage 3 produces that Stage 4 consumes

Stage 4 is the dogfood bar: the viewer consumes only contract functions, the 13 legacy routes are deleted, and `TopographyDress` lands as dress-locality's show-piece. Stage 4 therefore needs the border geometry and the scene tier to be complete enough that **no legacy route is required**. What it gets:

**Geometry, as functions.**
- `GET /api/borders?entity=<id>&year=<y>` → `{ entity, year, cycles, holes, arcs, tenure, arrangement }`, whole-body contract-tested (`contracts/map-api/fact/borders.feature`, four blessed fixtures). `cycles`/`holes` are arrays of `[{arc, orientation}]`; `arcs` maps a 16-hex arc id to `{witnesses, vertices, resource}`; `resource` is the id to hand to `/api/resource` and `/api/resources`, whose byte-identity and batch laws are already green. **This replaces every legacy route that served region outlines or boundary geometry.**
- An **absent** entity answers `{cycles: [], holes: [], arcs: {}}` at 200, never 404. Stage 4's viewer may rely on totality: there is an answer for every (entity, year).
- `tenure` is on the answer, so a consumer knows without a second call whether an extent is held ground or a disclosed claim — which is what a renderer needs to decide fill versus outline.

**The one arrangement, in the canon.**
- `map_canon::Arc { pts, witnesses }` with `ArcId` content-addressing **geometry alone**, direction-free: a border walked by two neighbours is ONE row, and re-inserting unions the witness sets. `CanonStore::{insert_arc, arcs, arc_points, cycle_points}`.
- `map_canon::{Cycle, CycleStep}` with `map_types::Orientation`; `Cycle::canonical` (rotation-invariant). `CycleBreak { position, arc }` is the typed refusal — Stage 4 must not draw a broken cycle short.
- `map_canon::Area { entity, name, cycles, holes, tenure }`. **`Area.rings: BTreeSet<BorderId>` is gone**; anything in Stage 4 that reads rings reads `cycles` and resolves through `cycle_points`, or calls `map_provider::canon_provider::rings_of_area`.
- `map_canon::Border` survives for whole polylines only — journey legs and river paths. It is not the border of spec §2 and Stage 4 should not treat it as one.

**Equation 2, stated where a consumer can read it.**
- `/api/contract` and every `/api/borders` answer carry `arrangement: {faces, residualSr, arcs}`. Σ faces = 4π is checked at compile (`Partition::validate` against the declared `completeness_residual`) and asserted on the served world by `contracts/map-api/fact/borders.feature`.
- **The boundary of that claim, stated plainly for Stage 4:** it holds of the arrangement, which covers the plate frame, the partition's water and rivers, the atlas polity eras, and (after Task 11) the migrated survey circuits. It does **not** cover the 731 natural-earth water features, the 362 historical-basemap background areas, or the 6 relief bands, which reach the canon through `timeline_bridge` and hold whole-polyline `Border`s. Stage 4's dogfood bar must either accept that those three layers are served from `Area.cycles` built by the timeline bridge (which Task 8 made cycle-shaped, so they *are* uniformly addressable even though they are not in the one arrangement), or scope their migration explicitly. **Do not let Stage 4 discover this at route-deletion time.**

**Precedence and extents.**
- `map_partition::Partition::extents(absent) -> BTreeMap<witness, BTreeSet<FaceId>>` — **a face may belong to several extents**; overlap is meaning.
- `PFace.held_by: Option<String>` is the paint answer: at most one holder, and a `Tenure::Claimed` witness is never it. `PFace.claims` remains the extent-membership answer, sorted by the measured specificity law (water over land, then parent depth, then witness area, then id) which this stage did not change.
- `WitnessRegion { id, entity, kind, tenure, rings, parent }` — a witness declares the canon entity it realizes and its tenure. Stage 4 adds no witness kinds; if `TopographyDress` needs Ground to take a data source, that is a dress parameter, not a witness.

**The tolerance regime, inherited.**
- `map_types::tolerance::{Tolerance, Separation, Plateau, Tolerances, ToleranceViolation}`; the ledger at `data/authored/tolerances.json`; `map_compile::tolerances::{load, measure, DECLARED_PATH}`; `PartitionConfig::from_tolerances` as the **only** non-test constructor (`Default` is deleted).
- Three laws Stage 4 inherits and must keep green: the separation law (populations re-measured from the data), the plateau law (`cargo test -p map-compile -- --ignored`), and the bare-float law (`map_partition::literals::scan` over the geometry modules against `ALLOWED`). **Any new geometry constant in Stage 4 needs a ledger row with two measured populations and a plateau, or it will not compile past the literal law.**

**Contract machinery.**
- `contracts/VERSION` = `0.4.0`; `contracts/CHANGELOG.md` carries the stage's declaration including any golden re-blessing and the migration remainder. Stage 4 opens at `0.4.0` and closes at `0.5.0`.
- `borders.feature`'s two `@property` scenarios use Stage 1's `forAllShrink` and hole-distinctness machinery; a Stage 4 property scenario registers a hole the same way.
- The derivability scenario in `scene.feature` is the join between the tiers. Stage 4's route deletion is exactly the change most likely to break it, which is why it exists before the deletion rather than after.

**Explicitly NOT produced here, so Stage 4 does not assume it:** `TopographyDress` (Stage 4's own); a `⊕` combine endpoint (diagnosis §8.5 argues against building one, and nothing in this stage needed it); the write API; the natural-earth / basemap / relief layers' entry into the one arrangement; `Witness` as a queryable node type with `evidence`/`interval` (arcs carry witness *strings*, which is what the arrangement holds — promoting them to nodes is registry work, not geometry work); and, unless the owner approved Task 11 Step 11, the 16 GEN 10 survey circuits.

---

## Self-Review (performed at write time)

**1. Spec coverage.**

§5 Stage 3's four named deliverables: *survey circuits enter the partition as witnesses* — Task 11 (cohort 1 mandatory, cohort 2 owner-gated, remainder declared). *Every border canonical* — Tasks 6, 7, 8; the canon stores one row per canonical edge and neighbours reference it. *Claims reference edges (the promise's west border IS the coastline)* — Task 6's `Area.cycles`, Task 10's overlapping extents, Task 11's shared-arc-with-the-sea law. *Adds `borders` with provenance* — Task 9. *Census empty-diff* — gated in Tasks 5, 8, 9, 10, 11 and swept across all 89 stops in Task 12. *The golden gate is the hard judge; snapping tolerances declared, not tuned* — Part A entire, plus a gate run in every task that touches geometry. ✓

§2's Border node ("one canonical edge … carrying the witness set whose geometry it realizes") — Task 6's `Arc`, whose identity is geometry alone and whose witnesses union on re-insert. §2's Extent ("derived: entity × era → boundary cycles / face set, resolved in the arrangement by the precedence law. Never authored directly") — Task 10's `extents` plus `held_by`; Task 11 removes the last place an extent was authored directly. §2's Law-as-data ("the knobs live here: one row, global reach") — Task 2's ledger, applied to geometry's knobs. ✓

Equation 2 ("every border exists once, in the one arrangement; extents are references; Σ faces = 4π") — all three halves: Task 8's shared-arc law, Task 6's reference-shaped `Area`, Task 9's wire-level completeness statement with Task 5's declared residual bound. ✓

§4's derivability clause — Task 12, and the brief was right that this stage is where it becomes real. §6's four strata — Rust laws in every task, the contract suite in 9 and 12, census diffs in 5/8/9/10/11/12, the golden gate in 4/5/8/9/10/11/12. §8's named top risk — Part A is five of twelve tasks, which is the proportion the risk deserves. ✓

**Gaps I am leaving open, deliberately and with the reason.** Σ faces = 4π holds of the *arrangement*, not of the whole canon: 731 natural-earth water features, 362 basemap background areas and 6 relief bands still reach the canon through the timeline bridge. Migrating them is a larger job than Stage 3, would move far more pixels, and spec §5 does not ask for it — but it means the phrase "every border canonical" is true of the arrangement's witnesses and not yet of every feature the viewer draws. This is stated in the handoff and must be stated in Task 12's closing note; it is the most likely thing for a reader to over-read. Second: `map_canon::Border` survives for journey legs and river paths, which are genuinely open polylines rather than arrangement edges; unifying them is not obviously right and this plan does not attempt it.

**2. Placeholder scan.** No "TBD", no "add error handling", no "similar to Task N". Four places say "as today" or "exactly Task 8's block" and each names the file and line range being preserved — those are instructions to keep code identical, which is the point, not deferred decisions. Three `…` appear: the three Haskell step bodies in Task 9 Step 6, each preceded by the comment stating exactly what the step must assert and what would make it vacuous; and one `/* perpendicular offset of mid from arc c→d */` in Task 11 Step 1, immediately after two working examples of that computation appear in Task 3's `offset_from_arc` and in `crates/map-canon/src/lib.rs:719-742`. Two tasks carry explicit STOP-AND-ASK owner gates (Task 10 Step 8, Task 11 Step 9) and four carry STOP-if-red instructions where the honest response to a failing law is to report a finding rather than adjust a number (Tasks 3, 4, 8, 11). Those are the plan's most important content, not its gaps.

**3. Type consistency.** `Tolerance`/`Separation`/`Plateau`/`ToleranceViolation`/`Tolerances` keep their field names across Tasks 1–5 and the handoff (`radians`, `separates`, `plateau`, `min_margin`, `evidence`; `same_max`/`distinct_min`; `lo`/`hi`). `Tolerance::radians` is both a field and a method — deliberate, mirroring the crate's existing accessor style, and every call site in Tasks 4–11 uses the method form. The ten ledger row names match between the JSON, `Tolerances`'s fields, `Tolerances::all`, and the Task 2 whole-body test. `Arc`/`ArcId`/`Cycle`/`CycleStep`/`CycleBreak` match between Tasks 6, 7, 8, 9 and the handoff; `insert_arc`/`arcs`/`arc_points`/`cycle_points` likewise. `Orientation` is `map_types::Orientation` throughout — never a second enum. `dissolve_arcs`/`edge_points`/`extents`/`held_by` match between Tasks 7, 10, 11, 12 and the handoff. `WitnessRegion`'s two new fields (`entity`, `tenure`) are named identically in Tasks 10 and 11 and in every construction site listed in Task 10 Step 5. `bundle_faces` → `extents` is a rename inside `partition_bridge`, and Task 8 introduces `bundle_faces_for_law` while Task 10 introduces `extents_for_law` — **two names for the test hook across two tasks**, which is the kind of drift this review exists to catch: Task 10 renames the hook and the plan says so at its Step 4, so the Task 8 test that calls `bundle_faces_for_law` must be updated in Task 10's diff. That is noted here rather than papered over.

**One correction to this plan's own brief, recorded so it is not rediscovered.** The brief names `crates/map-compile/src/surveys.rs` as the file holding the survey circuits. There is no such file: the surveys live in `crates/map-adapters/src/surveys.rs` (1,742 lines), and `map-compile` consumes them through `map_adapters::scripture_timeline_with` at `crates/map-compile/src/main.rs:234`. Every reference in this plan uses the real path.

**A second correction, and it matters to how this stage is judged.** The brief and spec §5 together imply that "census empty-diff" and "the golden gate holds" are one combined gate on Stage 3. They are not, because the census is blind to geometry (`CensusRow` is entity, name, layer, kind, tenure — `crates/map-canon/src/lib.rs:783-790`). An empty census diff is compatible with every border on the map moving. This is not a defect in the spec — the census is the right instrument for the *policy* changes Stages 1–2 make — but it does mean the census cannot be Stage 3's geometry judge, and the golden gate's 25-probe sampling cannot be its only one. That is why every geometry task in this plan carries a Rust-side whole-body geometry assertion of its own, and why Task 8's bit-identity law is the checkpoint Part B is built around.
