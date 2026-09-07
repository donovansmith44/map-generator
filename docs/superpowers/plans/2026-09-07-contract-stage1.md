# Stage 1: The Entity Registry, and Pieces Made First-Class — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land spec §5's Stage 1 — one entity registry through which every minted identity resolves, with `entities` / `node` / `edge_summary` / `edges` on the wire and slug-matching supersession dead — and, pulled forward on the diagnosis's recommendation, make a **piece** a first-class thing that owns its own geometry, so the two red `@target` laws become true rather than merely labelled.

**Architecture:** Three parts, in order. **Part A** sharpens the instruments before they are used to judge anything (shrinking, a hole-distinctness law, CI-enforced semver, the census-diff instrument): you cannot review a diff with an instrument you are building at the same time. **Part B** introduces `Piece`/`PieceSet` as a first-class type in `map-types`, stamps a piece on every scene element at push time in `map-provider`, splits the one aggregated points buffer in `map-encoders`, and puts a real `pieces=` parameter on `/api/scene`. **Part C** adds `map_canon::Registry` — canonical entities with witnesses and *typed, declared* unification reasons read from owner-reviewable data — retires `bg_shadows` slug matching, and serves the four Explorable-shaped fact-tier routes.

**Tech Stack:** Rust (map-types, map-canon, map-compile, map-provider, map-encoders, map-viewer); Haskell contract runner (GHC 9.12.1, cabal 3.18.1.0, megaparsec, aeson, QuickCheck, hspec); Gherkin `.feature` corpus + blessed JSON fixtures; Node + playwright-core for the golden gate.

**Spec:** `docs/superpowers/specs/2026-09-06-map-api-contract-design.md`
**Diagnosis (the evidence this plan argues from):** `docs/notes/2026-09-06-contract-diagnosis.md`
**Stage 0 plan (format exemplar only):** `docs/superpowers/plans/2026-09-06-contract-stage0.md`

## Global Constraints

- **TDD is mandatory.** Every task writes its failing test first, runs it, sees it fail for the stated reason, then implements. No exceptions.
- **Owner's standing law:** first-class composable types with laws. Never styling tricks, never special cases, never tuned constants. If a fix needs a magic number, the number is wrong or it belongs in declared data.
- **Whole-body assertions.** A scenario pins the ENTIRE answer, with don't-cares masked explicitly in the scenario text. Existential poke-assertions ("some entry equals…", "is an array", "has at least N") are forbidden.
- **A check satisfiable by the failure mode is not a check.** This applies to the *inputs* as well as the assertion — Stage 0's worst defect (diagnosis §7.0) was an assertion that was correct and inputs that could never make it fail. Every property scenario added by this plan must be able to go red, and Task 5 makes that machine-checked.
- **Order within every task:** contract additions first (runner red) → Rust tests (cargo red) → implement (green) → golden gate holds → census diff reviewed → commit.
- **The suite stays pre-release.** `contracts/VERSION` moves `0.1.0` → `0.2.0` in Task 17, and that bump is the owner's declaration of what changed. v1.0 is a human act, never automatic.
- **The golden gate is the hard judge.** `node crates/map-viewer/tests/golden.js --check` must print `ALL GOLDEN VIEWS HOLD` (89/89 stops). Run it from `C:\Users\donov\.claude\jobs\c6946bce\tmp` so `playwright-core` resolves. Any visual change requires the owner's explicit approval and a re-blessing; a drifted probe STOPS the task.
- **Ports.** The workbench viewer is **8090**. `8080` (atlas API), `8081`, `8000`, `5000` belong to the atlas pipeline and must never be bound.
- **The atlas repo is a READ-ONLY path dependency** (`../../../../scratch/bible-atlas-sketch/.claude/worktrees/bible-atlas-m1/graph-types`). Never edit it. The upstreamability law is therefore enforced on OUR side, as a Rust law test asserting our relation labels are an additive extension of theirs.
- **Never change the atlas-edge suite's meaning without the atlas session.** Task 17 hands that suite over; until then, edits to `contracts/atlas-edge/*.feature` are limited to wiring an existing diff into a failure message (Task 3), never to a scenario, step, or projection.
- **The render pipeline is three steps** (MEMORY: render-pipeline-three-steps): after a Rust change, `cargo build --release`, then stop the old viewer, then relaunch it DETACHED. Session background tasks get killed, so use `Start-Process`:
  ```powershell
  Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
  cargo build --release
  Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
  ```
  A change to `crates/map-compile` additionally requires the map-compile build step before the viewer restart (`cargo run --release -p map-compile`), or the canon is stale.
- **Haskell commands run from `contracts/runner/`** unless stated otherwise.
- **Feature preambles are unverified prose** (diagnosis §9.2). Any task that changes what a feature *means* must update its preamble in the same commit; the reviewer checks the prose against the scenarios.

---

## Part A — sharpen the instruments before using them

### Task 1: Confirm or refute the shared-buffer hypothesis, and pin the answer as a test

Diagnosis §8.2 step 1: *"Read the renderer and establish whether the shared points buffer is a general pattern or a single case. This is a half-day question and the size of everything below depends on its answer."* This task does that reading, and — because a finding nobody can re-run is not a finding — leaves behind a test that fails the day the answer changes.

**The answer this plan was sized on** (established by reading the code and by two live HTTP calls; the executor must reproduce it, not trust it):

- `crates/map-encoders/src/gpu.rs`, in `GpuSceneEncoder::encode`, is the ONLY aggregation site. Regions emit one `RingLoop` resource per ring piece keyed `region:HEX`, and boundaries one `LineStrip` per line piece keyed `boundary:HEX` — both per-element, both already separable. Markers do this instead:
  ```rust
  let mut by_style: BTreeMap<StyleKey, (MarkerStyle, Vec<UnitVec>)> = BTreeMap::new();
  for m in &scene.markers {
      let sk = style_of(&mut styles, GpuStyle::Marker { color: m.style.color, size: m.style.size });
      by_style.entry(sk).or_insert_with(|| (m.style, Vec::new())).1.push(m.at);
  }
  for (sk, (_, pts)) in by_style {
      let (id, geom) = add(&mut resources, &mut seen, ResourceKind::Points, &pts);
      features.push(FeatureInstance { feature: "markers".to_string(), geometry: geom, resource: id, style: sk });
  }
  ```
  Every marker in the scene, from every piece, is grouped by **paint** (`StyleKey` = colour + size) and nothing else.
- `crates/map-provider/src/canon_provider.rs` pushes markers at three sites, all using the same `style.marker_style()`: journey stations inside `push_way` (piece Journeys), `Feature::Point` landmarks in the layer loop (piece Markers), and the `RenderSubject::Point` subject marker (piece Markers). One style ⇒ one `StyleKey` ⇒ **one buffer for all three**.
- Live, at `year=-1405&zoom=90.0000&style=canaan&relief=1`: with journeys on, one `points` resource `4fe3efb87e9631a7`, 31 vertices / 420 bytes; with `&journeys=0`, one `points` resource `fb387872526ea52b`, 26 vertices / 360 bytes. The manifest carries exactly one `{"feature":"markers"}` entry in each case. `noJourneys`' resource set is NOT a subset of the full one — the single divergent id is that buffer.
- The provider's own comment at `canon_provider.rs:558` names the root cause: *"Recorded at push time because the scene type carries no layer."*

**So: the hypothesis is CONFIRMED, and the interference is one aggregation site, not a pervasive pattern.** That is the cheaper of the diagnosis's two branches, and it sizes Part B at four tasks (8–11) rather than a rewrite. It is not, however, a labelling job: the buffer really must be split, which means the scene types must carry a piece, which means `map-types`, `map-provider`, `map-encoders` and `map-viewer` all move.

**Branch points — what this plan does under each outcome:**

| what Step 3 finds | what happens to this plan |
|---|---|
| **As above: markers aggregate by `StyleKey`; regions and boundaries are per-element.** | Proceed unchanged. Tasks 8–11 as written. |
| Regions or boundaries ALSO merge across pieces (e.g. `add`'s `seen` dedupe unifies content produced by two pieces). | Tasks 8–11 still stand — a resource shared by two pieces is fine, because `piece` rides the `FeatureInstance` and not the resource descriptor, and resource sets union correctly. Add one scenario to Task 12 pinning that a shared resource carries two entries with different pieces. Do NOT split by content. |
| A THIRD aggregation exists that this plan did not name (some other `BTreeMap`-keyed grouping in `gpu.rs` or the provider). | STOP. Report to the owner before Task 8. Part B grows by one task per aggregation site; the owner decides whether Part C still fits in this stage. |
| The interference is NOT reproducible at all (the live probe shows a subset). | STOP and report. The diagnosis's §3.2 would then be wrong and Part B has no premise; the owner re-scopes. |

**Files:**
- Test: `crates/map-encoders/src/tests.rs` (add to the existing `mod tests`)
- Read only: `crates/map-encoders/src/gpu.rs`, `crates/map-provider/src/canon_provider.rs`, `crates/map-types/src/scene.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: the confirmed finding, and `markers_from_two_origins_share_one_points_buffer` — a RED-then-GREEN pin that Task 10 will invert. Nothing else in this plan imports from it.

- [ ] **Step 1: Reproduce the symptom against the live server (read-only, no restart)**

Both servers are already up. From the repo root:
```bash
S='http://127.0.0.1:8090/api/scene?year=-1405&zoom=90.0000&style=canaan&relief=1'
curl -s "$S"            -o full.tmp.json
curl -s "$S&journeys=0" -o noj.tmp.json
python -c "
import json
f=json.load(open('full.tmp.json')); n=json.load(open('noj.tmp.json'))
pts=lambda d:[(r['id'],r['vertices'],r['bytes']) for r in d['resources'] if r['kind']=='points']
print('full points:',pts(f)); print('noj  points:',pts(n))
print('full markers entries:',[e for e in f['features'] if e['feature']=='markers'])
print('noj  markers entries:',[e for e in n['features'] if e['feature']=='markers'])
fi={r['id'] for r in f['resources']}; ni={r['id'] for r in n['resources']}
print('noj-only ids:', ni-fi)
"
rm -f full.tmp.json noj.tmp.json
```
Expected: exactly one `points` resource on each side, 31 vs 26 vertices, exactly one `markers` feature entry on each side, and a non-empty `noj-only ids`. If any of those differ, take the fourth branch above.

- [ ] **Step 2: Read the three files and write down every aggregation site**

Read `crates/map-encoders/src/gpu.rs` (`GpuSceneEncoder::encode`, roughly lines 480–660), `crates/map-provider/src/canon_provider.rs` (`push_area`, `push_way`, the `for layer in layers_wanted(q.layers)` loop, and the `RenderSubject::Point` tail), and `crates/map-types/src/scene.rs`. An *aggregation site* is any place where geometry contributed by more than one call site is concatenated into one buffer before `add(...)` is called. Enumerate them. The expectation is exactly one (`by_style`). If you find more, take the third branch.

- [ ] **Step 3: Write the failing test that pins the finding**

Append to `crates/map-encoders/src/tests.rs`:

```rust
/// TASK 1 (Stage 1): the shared-buffer finding, pinned so it cannot
/// change silently. Two markers of DIFFERENT semantic origin — one a
/// journey station, one a gazetteer landmark — wearing the SAME
/// MarkerStyle land in ONE `points` resource today. That is the whole
/// of the composition red: omitting either origin changes the other's
/// bytes, hence its content address, hence the resource set.
///
/// Task 10 inverts this test. Until then it is the record.
#[test]
fn markers_from_two_origins_share_one_points_buffer() {
    use map_types::scene::{Snapshot, StyledMarker};
    use map_types::style::{MarkerStyle, Rgba};
    use map_types::{SceneEncoder, UnitVec};

    let paint = MarkerStyle { color: Rgba(10, 20, 30, 255), size: 3.0 };
    let mk = |lat: f64, lon: f64| StyledMarker {
        at: UnitVec::from_lat_lon_deg(lat, lon),
        style: paint,
        sources: Default::default(),
        place: None,
    };
    let scene = Snapshot {
        markers: vec![mk(31.0, 35.0), mk(32.0, 35.5)],
        ..Snapshot::default()
    };
    let encoded = crate::gpu::GpuSceneEncoder::default().encode(&scene).expect("encode");

    let points: Vec<_> = encoded
        .resources
        .iter()
        .filter(|r| r.descriptor.kind == crate::gpu::ResourceKind::Points)
        .collect();
    assert_eq!(points.len(), 1, "today: one shared points buffer for every marker origin");
    assert_eq!(points[0].descriptor.vertex_count, 2, "both markers packed into it");

    let marker_entries: Vec<_> =
        encoded.manifest.features.iter().filter(|f| f.feature == "markers").collect();
    assert_eq!(marker_entries.len(), 1, "one undifferentiated 'markers' entry");
}
```

Adaptation note for the implementer (verify, do not guess): `crates/map-encoders/src/tests.rs` already imports several of these names — reuse the file's existing import style rather than shadowing it, and check whether `ResourceKind`/`GpuSceneEncoder` are re-exported from `crate::` root or must be reached through `crate::gpu::`. If `Snapshot` has no `..Default::default()`-friendly literal in this crate's tests, use `Snapshot::empty()` and assign `markers` (the crate's other tests show which is idiomatic here).

- [ ] **Step 4: Run it and see it PASS, then break it deliberately to prove it discriminates**

```bash
cargo test -p map-encoders markers_from_two_origins -- --nocapture
```
Expected: PASS. Then temporarily change `assert_eq!(points.len(), 1, ...)` to `2` and re-run: it must FAIL. Change it back. A test that cannot fail is not a test (diagnosis §7.0) — this 30 seconds is the whole point of the step.

- [ ] **Step 5: Report the finding to the owner and take the branch**

Report, in one paragraph: the aggregation sites found, the live vertex counts, and which branch row above applies. If the branch is not the first row, STOP HERE and wait.

- [ ] **Step 6: Commit**

```bash
git add crates/map-encoders/src/tests.rs
git commit -m "Stage 1 Task 1: pin the shared points buffer — markers aggregate by paint, not by piece"
```

---

### Task 2: Bound the Levenshtein, and give `Capture` an export list with whole-type round-trip laws

Two debts in one file, one review unit. `didYouMean`'s edit distance is the naive exponential triple recursion — it already hung a test once, and a long garbage value in a feature file would hang `check`, which is the gate CI is about to depend on (Task 6). And `Capture.hs` has no export list, so the round-trip laws are only ever exercised on values that `parseCap` produced — which is exactly the set of values `parseCap` accepts, so the law proves less than its name claims.

**Files:**
- Modify: `contracts/runner/src/Capture.hs`
- Modify: `contracts/runner/test/Spec.hs`

**Interfaces:**
- Consumes: nothing.
- Produces: `Capture`'s explicit export list — `Universe(..)`, `FromCapture(..)`, `describeUniverse`, `didYouMean`, `editDistance`, `Piece(..)`, `pieceText`, `PieceSet(..)`, `allPieces`, `Year(..)`, `StyleName(..)`, `styleNames`, `FixtureRef(..)` and every other name the existing modules import. `editDistance :: Text -> Text -> Int` is new and total in O(mn).

- [ ] **Step 1: Write the failing tests**

Append to `contracts/runner/test/Spec.hs` (imports: `Capture`, `qualified Data.Text as T`, `Test.QuickCheck`, `Test.Hspec.QuickCheck (prop)`, `Data.Proxy`, `System.Timeout (timeout)`):

```haskell
  describe "capture hardening (Stage 1 Task 2)" $ do
    it "editDistance agrees with the textbook answer on known pairs" $ do
      editDistance "kitten" "sitting" `shouldBe` 3
      editDistance "" "abc"           `shouldBe` 3
      editDistance "abc" ""           `shouldBe` 3
      editDistance "abc" "abc"        `shouldBe` 0
      editDistance "topografy" "topography" `shouldBe` 2
    prop "editDistance is symmetric" $ \a b ->
      editDistance (T.pack a) (T.pack b) === editDistance (T.pack b) (T.pack a)
    prop "editDistance is bounded by the longer string" $ \a b ->
      editDistance (T.pack a) (T.pack b) <= max (length a) (length b)
    it "didYouMean answers on a long garbage value instead of hanging" $ do
      -- 400 chars of junk against the piece universe. The old triple
      -- recursion is exponential in the shorter string and never
      -- returns; one second is three orders of magnitude of headroom.
      let junk = T.replicate 400 "q"
      r <- timeout 1000000 (evaluate (T.length (didYouMean (map pieceText [minBound .. maxBound]) junk)))
      r `shouldSatisfy` \x -> case x of Just _ -> True; Nothing -> False
    prop "every Piece round-trips through renderCap/parseCap" $ \(p :: Piece) ->
      parseCap (renderCap p) === Right p
    prop "every PieceSet round-trips, INCLUDING the empty set" $ \(ps :: PieceSet) ->
      parseCap (renderCap ps) === Right ps
    prop "every StyleName round-trips" $ \(s :: StyleName) ->
      parseCap (renderCap s) === Right s
    prop "every Year in the frame round-trips" $ \(y :: Year) ->
      parseCap (renderCap y) === Right y
```

and at the top level of `Spec.hs`, the `Arbitrary` instances that make those four properties quantify over the WHOLE type rather than over `parseCap`'s image:

```haskell
instance Arbitrary Piece where
  arbitrary = elements [minBound .. maxBound]
  shrink p = takeWhile (< p) [minBound .. maxBound]

instance Arbitrary PieceSet where
  arbitrary = PieceSet . Set.fromList <$> sublistOf [minBound .. maxBound]
  shrink (PieceSet s) = [ PieceSet (Set.delete p s) | p <- Set.toList s ]

instance Arbitrary StyleName where
  arbitrary = StyleName <$> elements styleNames
  shrink _ = []

instance Arbitrary Year where
  arbitrary = Year <$> chooseInt (-4004, 100) `suchThat` (/= 0)
  shrink (Year y) = [ Year y' | y' <- shrink y, y' /= 0, y' >= -4004, y' <= 100 ]
```
(add `import qualified Data.Set as Set` and `import Control.Exception (evaluate)` if not already present.)

- [ ] **Step 2: Run and verify FAIL**

```bash
cabal test 2>&1 | tail -20
```
Expected: compile error — `editDistance` is not in scope. That is the failure this step wants; the timeout test cannot even be reached yet.

- [ ] **Step 3: Replace the exponential distance with the standard row-by-row DP**

In `contracts/runner/src/Capture.hs`, replace the whole `didYouMean` block (currently lines 25–36) with:

```haskell
-- Wagner–Fischer, one row at a time: O(len a * len b) time, O(len b)
-- space. The previous implementation was the naive triple recursion,
-- which is exponential in the shorter string -- it hung a test during
-- Stage 0, and a long garbage value in a feature file would hang
-- `check`, the gate CI is about to depend on. No cap, no cutoff, no
-- tuned constant: the algorithm is simply the right one.
editDistance :: Text -> Text -> Int
editDistance a b = last (foldl' row [0 .. T.length b] (T.unpack a))
  where
    bs = T.unpack b
    row prev ca = scanl' step (head prev + 1) (zip3 bs prev (tail prev))
      where
        step left (cb, diag, up) =
          minimum [left + 1, up + 1, if ca == cb then diag else diag + 1]

didYouMean :: [Text] -> Text -> Text
didYouMean vocab w =
  case sortOn (editDistance w) vocab of
    (best : _) | editDistance w best <= 3 -> "  Did you mean: " <> best <> "?"
    _ -> ""
```
(add `import Data.List (foldl', scanl', sort, sortOn)` — the module already imports `sort` and `sortOn`; extend that line rather than adding a second import of the same module.)

- [ ] **Step 4: Give the module an explicit export list**

Change line 1 of `Capture.hs` from `module Capture where` to:

```haskell
-- An explicit export list, so the round-trip laws in test/Spec.hs can
-- quantify over the WHOLE type (via Arbitrary) rather than only over
-- values that `parseCap` happened to produce -- which is the set
-- `parseCap` accepts, making the law circular. Constructors are
-- exported deliberately for that reason.
module Capture
  ( Universe (..)
  , FromCapture (..)
  , describeUniverse
  , didYouMean
  , editDistance
  , Piece (..)
  , pieceText
  , PieceSet (..)
  , allPieces
  , Year (..)
  , StyleName (..)
  , styleNames
  , FixtureRef (..)
  ) where
```

Adaptation note: this list must cover every name the other modules already import from `Capture`. Build after editing; the compiler names any omission (`Variable not in scope: …`), and each one is either added to the list or was dead. Do NOT add a name the compiler did not ask for.

- [ ] **Step 5: Run tests until green**

```bash
cabal test 2>&1 | tail -8
```
Expected: PASS, example count up by 9 from 112.

- [ ] **Step 6: Confirm the static gates still pass, then commit**

```bash
cabal run contract-runner -- check ../map-api    && cabal run contract-runner -- vocab ../map-api
cabal run contract-runner -- check ../atlas-edge && cabal run contract-runner -- vocab ../atlas-edge
git add contracts/runner
git commit -m "bounded Levenshtein and an export list on Capture; round-trip laws over whole types"
```

---

### Task 3: A real diff on the two fixture-comparison paths that still lack one

**Correction to the carried-forward debt list, established by reading the code:** `blessOrCompare` already has `firstDiff` — a short-circuiting, path-addressed, whole-tree first-difference report, landed in `7cd31bd`. That debt is *partly* paid. Two comparison paths still report an undiagnosable message and are the remaining work:

- the **masked** whole-body compare — `"body differs from fixture " <> f <> " outside the mask"` (`Steps.hs` ~line 228);
- the **consumed projection** compare — `"consumed projection " <> pn <> " differs from fixture " <> f` (`Steps.hs` ~line 435).

The second one matters most: it is the message a *provider engineer on the atlas team* will read when their change breaks us, and Task 17 hands that suite to them.

**Files:**
- Modify: `contracts/runner/src/Steps.hs`
- Modify: `contracts/runner/test/Spec.hs`

**Interfaces:**
- Consumes: `firstDiff :: Value -> Value -> Maybe Text` (already in `Steps.hs`).
- Produces: no new names. Both failure messages gain `": " <> firstDiff`-derived text.

- [ ] **Step 1: Write the failing tests**

Append to `test/Spec.hs`:

```haskell
  describe "fixture diffs on every comparison path (Stage 1 Task 3)" $ do
    it "firstDiff names the path and both values on a nested leaf" $ do
      let e = fromJust (A.decodeStrict "{\"a\":{\"b\":[1,2,3]}}")
          g = fromJust (A.decodeStrict "{\"a\":{\"b\":[1,9,3]}}")
      case firstDiff e g of
        Nothing -> expectationFailure "no difference found in differing values"
        Just m  -> do
          m `shouldSatisfy` T.isInfixOf "$.a.b[1]"
          m `shouldSatisfy` T.isInfixOf "2"
          m `shouldSatisfy` T.isInfixOf "9"
    it "firstDiff is Nothing on equal values (it does not invent differences)" $
      let v = fromJust (A.decodeStrict "{\"a\":[1,2]}")
      in firstDiff v v `shouldBe` Nothing
    it "the masked whole-body failure names WHERE it differs" $ do
      -- masked compare against a fixture that differs outside the mask
      msg <- maskedFailureMessage
      msg `shouldSatisfy` T.isInfixOf "outside the mask"
      msg `shouldSatisfy` T.isInfixOf "$."
    it "the consumed-projection failure names WHERE it differs" $ do
      msg <- projectionFailureMessage
      msg `shouldSatisfy` T.isInfixOf "consumed projection"
      msg `shouldSatisfy` T.isInfixOf "$."
```

Adaptation note: `maskedFailureMessage` and `projectionFailureMessage` are two small top-level helpers you write in `Spec.hs`. Each builds a `World` with a fake transport returning a known body, points `fixtureDir` at a temp directory holding a fixture that differs from it at a known path, runs the relevant step from `allSteps`, and returns the `Left` text. `Spec.hs` already has the `firstMatch` helper and a fake-transport idiom from Stage 0 Task 5 — follow it exactly rather than inventing a second one. If `firstDiff` is not currently exported from `Steps`, add it to that module's export list (or, if `Steps` has no export list, that is fine — it is `module Steps where` today).

- [ ] **Step 2: Run and verify FAIL**

```bash
cabal test 2>&1 | tail -20
```
Expected: the two message tests fail — the strings contain no `$.` path.

- [ ] **Step 3: Wire `firstDiff` into both paths**

In `Steps.hs`, the masked branch:
```haskell
                    if actual' == expected then Right w
                    else Left ("body differs from fixture " <> f <> " outside the mask: "
                               <> maybe "(no leaf difference found)" id
                                    (firstDiff expected actual'))
```
and the projection branch:
```haskell
                        | got == expected -> Right w
                        | otherwise -> Left ("consumed projection " <> pn
                                             <> " differs from fixture " <> f <> ": "
                                             <> maybe "(no leaf difference found)" id
                                                  (firstDiff expected got))
```
Note the argument order in both: `firstDiff expected actual` — `firstDiff`'s own message says "fixture has X, response has Y", so swapping them prints a lie.

- [ ] **Step 4: Run tests until green** — `cabal test 2>&1 | tail -8`.

- [ ] **Step 5: Commit**

```bash
git add contracts/runner
git commit -m "a diff on the masked and consumed-projection compare paths, not just the plain one"
```

---

### Task 4: `forAllShrink` for `@property` scenarios

Spec §4 requires it: *"`<angle-bracket>` holes whose `FromCapture` types carry QuickCheck `Gen` + shrink; the scenario becomes `forAllShrink` over generated bindings."* It is unimplemented (diagnosis §6.4), and §3.2 is the worked cost: the composition counterexample named fifteen pieces where the real answer is one word, and a human had to narrow it. Stage 1 adds paging and identity laws — exactly the kind whose counterexamples are large and structured. Do this while the property corpus is four scenarios.

**Design note, so the implementer does not reach for the wrong tool.** QuickCheck's `forAllShrink` combinator wants a pure `Testable`; our scenario body is `IO`, and — decisively — the runner's reproducibility rests on `Prop.holeSeed`, a pure function of (hole name, iteration) that `quickCheckWith`'s replay machinery would take over. So implement `forAllShrink`'s *semantics* directly: generate deterministically as today, and on the first failing iteration run a greedy shrink loop to a local minimum. Same guarantee, same determinism, no second seeding regime.

**Files:**
- Modify: `contracts/runner/src/Prop.hs`
- Modify: `contracts/runner/test/Spec.hs`

**Interfaces:**
- Consumes: `Capture` (Task 2's exports), `Run.runScenario`, `World`.
- Produces:
```haskell
data SomeHole = forall a. FromCapture a => SomeHole (Gen a) (a -> [a])
shrinkHole      :: SomeHole -> Text -> [Text]          -- rendered value -> rendered smaller values
shrinkToMinimal :: [StepDef] -> World -> Scenario -> Map Text Text
                -> IO (Map Text Text, Text)            -- minimal failing env + its failure text
```
`runScenarioProperty` keeps its signature `[StepDef] -> World -> Int -> Scenario -> IO Verdict`.

- [ ] **Step 1: Write the failing tests**

Append to `test/Spec.hs`:

```haskell
  describe "property shrinking (Stage 1 Task 4)" $ do
    it "shrinkHole offers strictly smaller renderings, and eventually none" $ do
      let h = holeRegistry Map.! "somePieces"
      shrinkHole h "borders, ground, water" `shouldSatisfy` (not . null)
      shrinkHole h "none" `shouldBe` []
    it "a property failing on ONE piece shrinks to that one piece" $ do
      -- A fake transport that fails only when `journeys` is in the URL.
      -- The generated piece sets are large; the MINIMAL failing set is
      -- exactly {journeys}, and that is what the report must name.
      let src = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: s"
            , "    When I render pieces <somePieces> at year -1405 in style canaan as only"
            , "    Then only equals only"
            ]
          Right f = parseFeature "t.feature" src
          fake url
            | "journeys=0" `T.isInfixOf` url = pure (Left "boom: journeys")
            | otherwise = pure (Right (okBody, fromJust (A.decodeStrict okBody)))
          w = testWorld fake
      v <- runScenarioProperty allSteps w 50 (head (ftScenarios f))
      case v of
        Failed e -> do
          e `shouldSatisfy` T.isInfixOf "somePieces = "
          -- the minimal binding is the one-piece set that still fails
          e `shouldSatisfy` \t -> "somePieces = journeys" `T.isInfixOf` t
                                  || "somePieces = none" `T.isInfixOf` t
        other -> expectationFailure ("expected a failure, got " <> show other)
    it "shrinking does not turn a PASSING property into a failure" $ do
      let src = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: s"
            , "    When I render pieces <somePieces> at year <someYear> in style canaan as only"
            , "    Then only equals only"
            ]
          Right f = parseFeature "t.feature" src
          fake _ = pure (Right (okBody, fromJust (A.decodeStrict okBody)))
      v <- runScenarioProperty allSteps (testWorld fake) 20 (head (ftScenarios f))
      v `shouldBe` Passed
```

Adaptation note: `okBody` is a `ByteString` holding a scene-shaped body (`"{\"features\":[],\"resources\":[],\"labels\":[],\"markers\":[]}"` is enough for the two steps used here); `testWorld` is a top-level helper wrapping the `World` constructor with the same defaults `Spec.hs` already uses elsewhere — reuse the existing one if `Spec.hs` has it, otherwise add it once and use it in later tasks too. The middle test's `journeys=0` predicate depends on `sceneUrl`'s piece→wire mapping, which Task 11 changes; the test is written against the *rendered URL*, so re-check it after Task 11 and update the predicate to whatever `pieces=` emits.

- [ ] **Step 2: Run and verify FAIL** — `cabal test 2>&1 | tail -20`. Expected: `SomeHole` takes one argument, not two; `shrinkHole` not in scope.

- [ ] **Step 3: Teach `SomeHole` to shrink**

In `Prop.hs`, replace the `SomeHole` declaration and `holeRegistry`:

```haskell
-- Spec §4: a hole carries a Gen AND a shrink. The shrinker is part of
-- the hole's contract, not a decoration -- §3.2's composition
-- counterexample named fifteen pieces where the true answer was one
-- word, and closing that gap cost a manual narrowing pass.
data SomeHole = forall a. FromCapture a => SomeHole (Gen a) (a -> [a])

-- Drop one piece at a time: every subset is reachable by repeated
-- application, and each candidate is strictly smaller, so the greedy
-- loop below terminates.
shrinkPieces :: PieceSet -> [PieceSet]
shrinkPieces (PieceSet s) = [ PieceSet (Set.delete p s) | p <- Set.toList s ]

-- Toward the frame's most-quoted year, then integer-shrink; year 0 is
-- excluded by the same law that excludes it from the generator.
shrinkYear :: Year -> [Year]
shrinkYear (Year y) =
  [ Year y' | y' <- [-1405 | y /= -1405] ++ shrink y
            , y' /= 0, y' >= -4004, y' <= 100, abs y' < abs y || y' == -1405, y' /= y ]

holeRegistry :: Map Text SomeHole
holeRegistry = Map.fromList
  [ ("someYear",   SomeHole genYear shrinkYear)
  , ("somePieces", SomeHole genPieces shrinkPieces)
  , ("somePiece",  SomeHole (elements [minBound .. maxBound] :: Gen Piece)
                            (\p -> takeWhile (< p) [minBound .. maxBound]))
  , ("someA",      SomeHole genPieces shrinkPieces)
  , ("someB",      SomeHole genPieces shrinkPieces)
  , ("someStyle",  SomeHole (StyleName <$> elements styleNames) (const []))
  ]

-- A hole's shrink candidates, in the rendered form the substituter
-- speaks. Parsing back through `parseCap` is what keeps this honest:
-- a rendering that does not parse is not a candidate.
shrinkHole :: SomeHole -> Text -> [Text]
shrinkHole (SomeHole _ shr) rendered =
  case parseCap rendered of
    Left _  -> []
    Right a -> map renderCap (shr a)
```
(`somePiece` is registered here for Task 12's strengthened omission law. `renderHole` changes only in its pattern: `renderHole name (SomeHole g _) i = …`, and `substituteExamples`'s `rep (h, SomeHole g _) = …`.)

- [ ] **Step 4: Implement the greedy shrink loop and wire it into `runScenarioProperty`**

Append to `Prop.hs`:

```haskell
-- forAllShrink's semantics over IO: from a known-failing binding
-- environment, repeatedly try replacing ONE hole with ONE strictly
-- smaller value; keep the first replacement that still fails; stop at a
-- local minimum. Deterministic (the candidate order is the shrinker's
-- own), terminating (every candidate is strictly smaller by the
-- shrinker's contract, and the fuel bound is a hard stop, not a tuning
-- knob -- it exists so a mis-written shrinker that returns a value
-- equal to its input cannot loop forever).
shrinkToMinimal
  :: [StepDef] -> World -> Scenario -> Map Text Text -> IO (Map Text Text, Text)
shrinkToMinimal defs w sc env0 = do
  e0 <- failureOf env0
  go (1000 :: Int) env0 (maybe "(no failure text)" id e0)
  where
    failureOf env = do
      v <- runScenario defs w (substitute env sc)
      pure $ case v of Failed e -> Just e; _ -> Nothing
    candidates env =
      [ Map.insert h v env
      | (h, cur) <- Map.toList env
      , Just hole <- [Map.lookup h holeRegistry]
      , v <- shrinkHole hole cur
      , v /= cur
      ]
    go fuel env msg
      | fuel <= 0 = pure (env, msg)
      | otherwise = try (candidates env)
      where
        try [] = pure (env, msg)
        try (c : cs) = do
          r <- failureOf c
          case r of
            Just msg' -> go (fuel - 1) c msg'
            Nothing   -> try cs
```

and replace `runScenarioProperty`'s failure branch:

```haskell
            Failed _ -> do
              (minEnv, minMsg) <- shrinkToMinimal defs w sc env
              pure . Failed $
                minMsg <> "\n    with " <> T.intercalate ", "
                  [ h <> " = " <> val | (h, val) <- Map.toList minEnv ]
                <> (if minEnv == env then "" else "\n    (shrunk from " <>
                      T.intercalate ", " [ h <> " = " <> val | (h, val) <- Map.toList env ]
                      <> ")")
```
The "(shrunk from …)" tail keeps the original counterexample visible — shrinking must sharpen the report, never hide what was actually generated.

- [ ] **Step 5: Run tests until green** — `cabal test 2>&1 | tail -8`.

- [ ] **Step 6: Re-run the live suite and confirm composition's counterexample is now one word**

```bash
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api 2>&1 | tail -30
```
Expected: still 18 green / 2 `@target` red, exit 0 — but composition's red now names a one- or two-piece `someA`/`someB` rather than fifteen. Record the shrunk counterexample in the commit message; it is the evidence this task worked.

- [ ] **Step 7: Commit**

```bash
git add contracts/runner
git commit -m "forAllShrink for @property scenarios: minimal counterexamples, same determinism"
```

---

### Task 5: The hole-distinctness law — the check that would have caught §7.0 on day one

Diagnosis §7.0's cheapest lesson: *"A green from a property scenario is worth nothing until you have seen its inputs vary. Every `@property` scenario needs a check that its holes actually take different values — cheap to write, and it would have caught this on day one."* Two holes registered to the same generator were bound to the identical value on every iteration, and the composition law degenerated to `x == x ∪ x`. This task makes that unrepresentable.

Two laws, both enforced by `contract-runner check` (so CI catches them in Task 6):

1. **Variation** — across a run, each hole of a `@property` scenario takes at least two distinct values. A hole pinned to one value is a constant wearing a generator's clothes.
2. **Independence** — any two distinct holes in the same scenario are not bound to the identical value on every iteration. Two holes that are always equal are one hole, and any law comparing them is degenerate.

**Files:**
- Modify: `contracts/runner/src/Prop.hs`
- Modify: `contracts/runner/src/Check.hs`
- Modify: `contracts/runner/test/Spec.hs`

**Interfaces:**
- Consumes: `Prop.holeRegistry`, `Prop.holesOf`, `Prop.renderHole`, `Prop.dedupe`.
- Produces:
```haskell
-- Prop.hs
data HoleDefect = Constant Text | AlwaysEqual Text Text deriving (Eq, Show)
holeDefects :: Int -> Scenario -> [HoleDefect]
```
`Check.checkDir` reports these alongside orphans/ambiguity and exits non-zero.

- [ ] **Step 1: Write the failing tests**

```haskell
  describe "hole distinctness (Stage 1 Task 5)" $ do
    it "the real composition scenario's holes vary and are independent" $ do
      let src = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: s"
            , "    When I render pieces <someA> at year <someYear> in style canaan as a"
            , "    And I render pieces <someB> at year <someYear> in style canaan as b"
            , "    Then a equals b"
            ]
          Right f = parseFeature "t.feature" src
      holeDefects 30 (head (ftScenarios f)) `shouldBe` []
    it "catches a hole that never varies" $ do
      -- someStyle over a 3-value universe still varies; a hole whose
      -- registered generator is a constant does not. Register one
      -- locally is not possible, so assert the detector directly on a
      -- scenario whose only hole is drawn from a singleton universe.
      constantHoleDefects `shouldSatisfy` any (\d -> case d of Constant _ -> True; _ -> False)
    it "catches two holes that are always equal — the §7.0 defect itself" $
      alwaysEqualDefects `shouldSatisfy`
        any (\d -> case d of AlwaysEqual _ _ -> True; _ -> False)
```

Adaptation note: the last two tests need a registry with a deliberately-defective hole. Do NOT pollute the real `holeRegistry`. Make `holeDefects` take the registry as its first argument internally — export a `holeDefectsWith :: Map Text SomeHole -> Int -> Scenario -> [HoleDefect]` and define `holeDefects = holeDefectsWith holeRegistry`. Then `constantHoleDefects` and `alwaysEqualDefects` are top-level helpers in `Spec.hs` that call `holeDefectsWith` with a local registry containing `SomeHole (pure (StyleName "canaan")) (const [])` bound to one name (Constant) and to two names (AlwaysEqual).

- [ ] **Step 2: Run and verify FAIL** — `holeDefects` not in scope.

- [ ] **Step 3: Implement the detector in `Prop.hs`**

```haskell
-- §7.0's law, made machine-checkable. A @property scenario whose holes
-- do not vary, or two of whose holes are always equal, has no
-- discriminating power on the axis it claims to fuzz -- exactly the
-- defect that reported "composition: target already met" against a
-- server that fails it.
data HoleDefect = Constant Text | AlwaysEqual Text Text deriving (Eq, Show)

holeDefectsWith :: Map Text SomeHole -> Int -> Scenario -> [HoleDefect]
holeDefectsWith reg n sc =
  [ Constant h | (h, vs) <- draws, length (nub vs) < 2 ]
  ++ [ AlwaysEqual h1 h2
     | ((h1, v1) : rest) <- tails draws, (h2, v2) <- rest, v1 == v2 ]
  where
    hs = [ h | h <- dedupe (holesOf sc), Map.member h reg ]
    draws = [ (h, [ renderHole h (reg Map.! h) i | i <- [0 .. n - 1] ]) | h <- hs ]

holeDefects :: Int -> Scenario -> [HoleDefect]
holeDefects = holeDefectsWith holeRegistry
```
(imports: `Data.List (nub, tails)`.)

- [ ] **Step 4: Wire it into `check`**

In `Check.hs`, extend `checkDir`'s per-feature accumulation with a second source of `bad` rows:

```haskell
      Right f ->
        [ (T.pack p <> " / " <> s, label v, describe b v) | (s, b, v) <- violations defs f ]
        ++ [ (T.pack p <> " / " <> scName sc, "DEGENERATE-HOLE", describeDefect d)
           | sc <- ftScenarios f
           , Tag "property" `elem` scTags sc
           , d <- Prop.holeDefects 30 sc ]
```
and add, in the `where`:
```haskell
    describeDefect (Prop.Constant h) =
      "<" <> h <> "> takes one value across the whole run -- a constant "
      <> "wearing a generator's clothes; this scenario fuzzes nothing on that axis"
    describeDefect (Prop.AlwaysEqual a b) =
      "<" <> a <> "> and <" <> b <> "> are bound to the same value on every "
      <> "iteration -- any law comparing them is degenerate (diagnosis 7.0)"
```
and change the success line to say what it actually checked:
```haskell
  if null bad then TIO.putStrLn
    "totality: every step has exactly one definition; every property hole varies"
```

Adaptation note: `Check.hs` currently imports neither `Prop` nor `Gherkin.Ast`'s `Tag`. Add `import qualified Prop` and whatever `Ast` names are missing. If this creates an import cycle (`Prop` imports `Run`, `Run` imports `World`; `Check` imports `World`), it will not — `Prop` does not import `Check`. Build to confirm.

- [ ] **Step 5: Run tests and both `check` gates until green**

```bash
cabal test 2>&1 | tail -8
cabal run contract-runner -- check ../map-api
cabal run contract-runner -- check ../atlas-edge
```
Expected: `totality: every step has exactly one definition; every property hole varies` for both directories. **If a real scenario reports a defect, that is a live finding — report it to the owner before "fixing" it.**

- [ ] **Step 6: Commit**

```bash
git add contracts/runner
git commit -m "the hole-distinctness law: a property whose inputs cannot vary is not a check"
```

---

### Task 6: CI, semver enforcement, and a changelog — **OWNER DECISION REQUIRED**

Spec §5 lists "Runner crate + CI" as Stage 0 scope; the runner landed and the CI did not (diagnosis §9.1 item 4). Spec §4 requires the semver rule to be **CI-enforced**: *"a diff editing an existing feature or blessed fixture without a bump fails."* With no CI, that rule is a convention nothing checks — and this stage is about to re-bless multi-megabyte fixtures.

> **STOP AND ASK THE OWNER BEFORE STEP 2.** The CI system is the owner's choice, not this plan's. Present exactly these options and implement the one chosen:
>
> - **(a) GitHub Actions** (`.github/workflows/contract.yml`) — standard, free for public repos, needs a GitHub remote and a Windows or Linux runner. The Haskell toolchain installs via `haskell-actions/setup`; the golden gate needs a headless Chromium, which the MEMORY workaround already provides locally but would need re-establishing on a runner.
> - **(b) A local pre-push git hook** (`.githooks/pre-push`, enabled by `git config core.hooksPath .githooks`) — no remote, no runner, no network; runs on this machine where both servers and the cached Chromium already live. Catches everything before it leaves the machine; catches nothing a collaborator does.
> - **(c) A `make ci` target** (`Makefile` already exists at the repo root) invoked manually and by (a) or (b) — the gates in one place, so whichever trigger the owner picks calls the same script.
>
> **This plan's recommendation, stated as a recommendation and not a decision:** implement **(c) first as the single source of truth for what "the gates" means**, then wire **(b)** to call it, because the golden gate genuinely needs this machine's cached Chromium and both live servers. Add (a) later, restricted to the gates that do not need a browser or a server (`cabal test`, `cargo test --workspace`, `check`, `vocab`, the semver rule). Do not implement (a) in this task unless the owner asks for it.
>
> Note the standing constraint that makes the split necessary: the contract `run` modes need our server on 8090 and the atlas on 8080, and the golden gate needs a browser. A CI system that cannot provide those must not pretend to run those gates — a gate that silently skips is §7.0 again.

**Files:**
- Create: `contracts/CHANGELOG.md`
- Create: `scripts/contract-semver-gate.sh`
- Modify: `Makefile` (add a `ci` target and a `contract-gates` target)
- Create: `.githooks/pre-push` *(only if the owner picks (b))*
- Create: `.github/workflows/contract.yml` *(only if the owner picks (a))*
- Test: `scripts/tests/semver-gate.test.sh`

**Interfaces:**
- Consumes: `contracts/VERSION`, `contracts/CHANGELOG.md`.
- Produces: `make ci` (all gates) and `make contract-gates` (the toolchain-only subset); `scripts/contract-semver-gate.sh BASE_REF` exits non-zero when the diff `BASE_REF..HEAD` edits an existing `.feature` or an existing file under any `fixtures/` directory without both a `contracts/VERSION` change and a `contracts/CHANGELOG.md` change.

- [ ] **Step 1: Write the failing test for the semver gate**

`scripts/tests/semver-gate.test.sh` — a shell test that builds throwaway commits in a temp git repo and asserts the gate's exit codes:

```bash
#!/usr/bin/env bash
# The semver rule (spec §4) as an executable test. Four cases, and the
# NEGATIVE ones matter most: a gate that passes everything is not a gate.
set -u
GATE="$(cd "$(dirname "$0")/.." && pwd)/contract-semver-gate.sh"
fail=0
scenario() { # name, expected exit, setup commands
  local name="$1" want="$2"; shift 2
  local d; d="$(mktemp -d)"
  ( cd "$d"
    git init -q . && git config user.email t@t && git config user.name t
    mkdir -p contracts/map-api/fixtures
    printf '0.1.0\n' > contracts/VERSION
    printf '# Changelog\n' > contracts/CHANGELOG.md
    printf 'Feature: f\n  Scenario: s\n    When I GET /a\n' > contracts/map-api/a.feature
    printf '{"x":1}\n' > contracts/map-api/fixtures/a.json
    git add -A && git commit -qm base
    eval "$@"
    git add -A && git commit -qm change
    "$GATE" HEAD~1 ) >/dev/null 2>&1
  local got=$?
  if [ "$got" != "$want" ]; then echo "FAIL $name: want exit $want, got $got"; fail=1
  else echo "ok   $name"; fi
  rm -rf "$d"
}
scenario "edit a feature with no bump -> reject" 1 \
  "printf 'Feature: f\n  Scenario: s2\n    When I GET /b\n' > contracts/map-api/a.feature"
scenario "edit a fixture with no bump -> reject" 1 \
  "printf '{\"x\":2}\n' > contracts/map-api/fixtures/a.json"
scenario "edit a feature WITH bump and changelog -> accept" 0 \
  "printf 'Feature: f\n  Scenario: s2\n    When I GET /b\n' > contracts/map-api/a.feature; \
   printf '0.2.0\n' > contracts/VERSION; \
   printf '# Changelog\n\n## 0.2.0\n- changed a.feature\n' > contracts/CHANGELOG.md"
scenario "ADD a new feature with no bump -> accept (additive)" 0 \
  "printf 'Feature: g\n  Scenario: s\n    When I GET /c\n' > contracts/map-api/b.feature"
exit $fail
```

- [ ] **Step 2: Run it and verify it fails** — `bash scripts/tests/semver-gate.test.sh`. Expected: every scenario errors because `scripts/contract-semver-gate.sh` does not exist.

- [ ] **Step 3: Write the gate**

`scripts/contract-semver-gate.sh`:

```bash
#!/usr/bin/env bash
# Spec §4: "a diff editing an existing feature or blessed fixture
# without a bump fails". ADDING a file is additive and needs no bump;
# EDITING or DELETING one is a change to a published promise and needs
# both a VERSION move and a CHANGELOG entry. Deliberately dumb about
# WHAT changed -- the version bump is the owner's declaration, and this
# gate's whole job is to make sure the declaration happens.
set -u
base="${1:-origin/master}"
changed() { git diff --name-only --diff-filter="$1" "$base"..HEAD -- contracts/; }

breaking="$( { changed M; changed D; changed R; } \
  | grep -E '(\.feature$|/fixtures/)' || true )"
[ -z "$breaking" ] && exit 0

bumped=0
git diff --name-only "$base"..HEAD -- contracts/VERSION      | grep -q . && bumped=1
logged=0
git diff --name-only "$base"..HEAD -- contracts/CHANGELOG.md | grep -q . && logged=1

if [ "$bumped" = 1 ] && [ "$logged" = 1 ]; then exit 0; fi

echo "semver gate: these published contract files were edited or removed:"
echo "$breaking" | sed 's/^/  /'
[ "$bumped" = 0 ] && echo "  MISSING: a change to contracts/VERSION"
[ "$logged" = 0 ] && echo "  MISSING: an entry in contracts/CHANGELOG.md"
echo "The suite is pre-release: breaking = MINOR bump, additive = PATCH."
exit 1
```
`chmod +x scripts/contract-semver-gate.sh scripts/tests/semver-gate.test.sh` (on Windows, `git update-index --chmod=+x` after adding, so the bit survives).

- [ ] **Step 4: Write the changelog with Stage 0's history recorded honestly**

`contracts/CHANGELOG.md`:
```markdown
# Contract suite changelog

The suite is **pre-release (0.x)**. Breaking change to an existing feature or
blessed fixture = MINOR bump; additive = PATCH. v1.0.0 is the owner's act,
never automatic. Enforced by `scripts/contract-semver-gate.sh`.

## Unreleased

- (nothing yet)

## 0.1.0 — 2026-09-06

The freeze. 12 features (`map-api` 6, `atlas-edge` 6), 16 blessed fixtures,
the Haskell runner with the totality law, the vocabulary drift law, and
`@property` scenarios. Two `@target` reds recorded, not fixed:
"composition" and "every manifest entry names its piece".

Corrected in `7cd31bd` before this entry was written: the composition
property's two holes drew from one seed and were always equal, so the law
could not fail. See `docs/notes/2026-09-06-contract-diagnosis.md` §7.0.
```

- [ ] **Step 5: Put every gate in one place**

Add to the root `Makefile` (adapt to the file's existing style and tab conventions — Make requires real tabs):
```make
# The toolchain-only gates: no server, no browser. Safe anywhere.
contract-gates:
	cd contracts/runner && cabal test
	cd contracts/runner && cabal run contract-runner -- check ../map-api
	cd contracts/runner && cabal run contract-runner -- check ../atlas-edge
	cd contracts/runner && cabal run contract-runner -- vocab ../map-api
	cd contracts/runner && cabal run contract-runner -- vocab ../atlas-edge
	cargo test --workspace
	bash scripts/tests/semver-gate.test.sh

# Everything, including the gates that need the live servers (8090 ours,
# 8080 the atlas) and the browser. Never binds a port itself.
ci: contract-gates
	cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
	cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8080 ../atlas-edge
	bash scripts/contract-semver-gate.sh $${BASE_REF:-origin/master}
	node crates/map-viewer/tests/golden.js --check
```

- [ ] **Step 6: Wire the owner's chosen trigger**

*(b), if chosen* — `.githooks/pre-push`:
```bash
#!/usr/bin/env bash
set -e
echo "pre-push: contract gates"
make contract-gates
BASE_REF="${1:-origin/master}" bash scripts/contract-semver-gate.sh "${BASE_REF:-origin/master}"
```
then `git config core.hooksPath .githooks`. Record that command in the commit message — a hook nobody enabled is not a gate.

*(a), if chosen* — `.github/workflows/contract.yml` running `make contract-gates` plus the semver gate on `pull_request`, with `fetch-depth: 0` so `git diff origin/master..HEAD` has both sides. Do NOT add the `run`/golden steps to it; they need this machine's servers and browser, and a gate that skips is worse than no gate.

- [ ] **Step 7: Run everything green**

```bash
bash scripts/tests/semver-gate.test.sh     # all four scenarios ok
make contract-gates
```

- [ ] **Step 8: Commit**

```bash
git add contracts/CHANGELOG.md scripts/ Makefile .githooks .github 2>/dev/null
git commit -m "CI gates, the CI-enforced semver rule, and a changelog — Stage 0 scope, paid late"
```

---

### Task 7: The census diff instrument, and the three corpus obligations with no scenario

Diagnosis §9.1 item 3: the census **diff** law does not exist, and *"the census diff is the exact instrument spec §6 stratum 3 uses to judge Stages 1–3… It should be built before Stage 1, not during it — you cannot review a diff with an instrument you are building at the same time."* Task 17 will use it to review this stage's identity unification, so it lands now, while the census is still stable.

Also in this task, because they are the same corpus edit and the same review: §9.1 item 1 (**default-totality**, testable today) and item 2 (**derivability**, written as `@target` so the obligation lives in the corpus rather than only in a document), plus §9.2's honesty fix about vocabulary exemptions.

**Files:**
- Modify: `crates/map-viewer/src/lib.rs` (the `/api/census` route gains an optional `to=` parameter)
- Modify: `crates/map-canon/src/lib.rs` (add `census_diff`)
- Modify: `crates/map-canon/src/tests.rs`
- Modify: `contracts/map-api/fact/census.feature`
- Modify: `contracts/map-api/scene/scene.feature`
- Create: `contracts/map-api/fact/derivability.feature`
- Modify: `contracts/runner/src/Steps.hs` (a styleless render, and the diff step)
- Create (blessed): `contracts/map-api/fixtures/census-diff-1405-1050.json`, `contracts/map-api/fixtures/census-diff-empty.json`, `contracts/map-api/fixtures/scene-1405-default-dress.json`

**Interfaces:**
- Consumes: `map_canon::census`, `map_canon::CensusRow`.
- Produces:
```rust
// map-canon
#[derive(Clone, Debug, PartialEq)]
pub enum CensusChange { Added(CensusRow), Removed(CensusRow), Changed { from: CensusRow, to: CensusRow } }
pub fn census_diff(store: &CanonStore, from: &Timestamp, to: &Timestamp) -> Vec<CensusChange>;
```
Wire: `GET /api/census?year=A&to=B` → the diff as a JSON array of `{"change":"added"|"removed"|"changed", …}`; `GET /api/census?year=A` is unchanged (additive — no existing fixture moves).
Runner: `sceneUrl` gains a styleless form so a render with no `style=` is expressible.

- [ ] **Step 1: Contract additions FIRST — write the scenarios, expect runner red**

Append to `contracts/map-api/fact/census.feature`:
```gherkin
  Scenario: the diff between two instants, whole
    When I GET /api/census?year=-1405&to=-1050
    Then the response equals fixture "census-diff-1405-1050"

  Scenario: an instant differs from itself in nothing
    When I GET /api/census?year=-1050&to=-1050
    Then the response equals fixture "census-diff-empty"

  @property
  Scenario: the diff of an instant with itself is empty at any year
    When I GET /api/census?year=<someYear>&to=<someYear> as selfDiff
    Then selfDiff equals fixture "census-diff-empty"
```

Append to `contracts/map-api/scene/scene.feature` (§9.1 item 1 — the law that makes every other dress claim meaningful):
```gherkin
  Scenario: default-totality — an omitted dress is the declared classical default
    When I render pieces ground, water, fills, borders, labels, journeys at year -1405 in no style
    Then the response equals fixture "scene-1405-default-dress"
```

Create `contracts/map-api/fact/derivability.feature` (§9.1 item 2 — written now, red until Stages 2–3 provide `disposition` and `borders`):
```gherkin
@target
Feature: derivability — the scene tier is a composition of the fact tier
  Spec §4: "every manifest entry must be traceable to `disposition` +
  `borders` answers — contract-tested by sampling." Until `disposition`
  (Stage 2) and `borders` (Stage 3) exist on the wire, this law has
  nothing to sample against and is declared RED rather than absent: the
  two tiers are currently contract-tested in isolation and never against
  each other, and that gap belongs in the corpus, not only in a note.

  Vocabulary:
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |

  @target
  Scenario: every manifest entry traces to a disposition and a border
    When I render pieces fills, borders at year -1405 in style canaan as sampled
    Then every entry in sampled traces to a disposition and a border
```

Run the gates and see them red for the right reasons:
```bash
cd contracts/runner
cabal run contract-runner -- check ../map-api
```
Expected: ORPHAN rows naming `in no style`, `as selfDiff` on a GET, and `traces to a disposition and a border`. That orphan list is this task's to-do list.

- [ ] **Step 2: Write the failing Rust test for `census_diff`**

Append to `crates/map-canon/src/tests.rs`:
```rust
#[test]
fn census_diff_is_empty_against_itself_and_names_every_real_change() {
    let store = fixture_store(); // the module's existing builder
    let t1 = ts(-1405);
    let t2 = ts(-1050);

    // Reflexivity: an instant differs from itself in nothing. This one
    // is cheap and it is the law Stages 2 and 3 are judged by.
    assert_eq!(census_diff(&store, &t1, &t1), Vec::new());

    // Discrimination: the diff is NOT empty where the census differs,
    // and it accounts for the difference exactly -- every row present
    // at t1 and absent at t2 appears once as Removed, and vice versa.
    let a = census(&store, &t1);
    let b = census(&store, &t2);
    let d = census_diff(&store, &t1, &t2);
    assert_eq!(d.is_empty(), a == b, "a non-empty diff iff the censuses differ");

    let removed: Vec<_> = d.iter().filter_map(|c| match c {
        CensusChange::Removed(r) => Some(r.clone()), _ => None }).collect();
    let added: Vec<_> = d.iter().filter_map(|c| match c {
        CensusChange::Added(r) => Some(r.clone()), _ => None }).collect();
    for r in &a { if !b.contains(r) && !changed_from(&d, r) { assert!(removed.contains(r)); } }
    for r in &b { if !a.contains(r) && !changed_to(&d, r) { assert!(added.contains(r)); } }
}
```
Adaptation note: `fixture_store`, `ts`, `changed_from`, `changed_to` — the first two exist in `crates/map-canon/src/tests.rs` under whatever names that file already uses (read it; do not invent). The last two are three-line helpers you add locally. If the fixture store's census is identical at both instants, pick two instants where it is not; a reflexivity test alone is satisfiable by `census_diff = const vec![]`, and a check satisfiable by the failure mode is not a check.

- [ ] **Step 3: Run and verify FAIL** — `cargo test -p map-canon census_diff` → `cannot find function census_diff`.

- [ ] **Step 4: Implement `census_diff`**

Append to `crates/map-canon/src/lib.rs`, next to `census`:
```rust
/// What changed between two instants of the disposition table. Spec §6
/// stratum 3 judges Stages 1–3 with this: Stage 1's diff IS the identity
/// unification (reviewed name by name); Stages 2 and 3 must produce an
/// EMPTY one. Rows are matched on (layer, entity) -- the pair that names
/// WHO is disposed WHERE -- so a row whose name, kind, or tenure moved
/// is a Changed, not a Removed+Added pair that hides what actually moved.
#[derive(Clone, Debug, PartialEq)]
pub enum CensusChange {
    Added(CensusRow),
    Removed(CensusRow),
    Changed { from: CensusRow, to: CensusRow },
}

pub fn census_diff(store: &CanonStore, from: &Timestamp, to: &Timestamp) -> Vec<CensusChange> {
    let key = |r: &CensusRow| (r.layer, r.entity.clone());
    let a: BTreeMap<_, _> = census(store, from).into_iter().map(|r| (key(&r), r)).collect();
    let b: BTreeMap<_, _> = census(store, to).into_iter().map(|r| (key(&r), r)).collect();
    let mut out = Vec::new();
    for (k, ra) in &a {
        match b.get(k) {
            None => out.push(CensusChange::Removed(ra.clone())),
            Some(rb) if rb != ra =>
                out.push(CensusChange::Changed { from: ra.clone(), to: rb.clone() }),
            Some(_) => {}
        }
    }
    for (k, rb) in &b {
        if !a.contains_key(k) {
            out.push(CensusChange::Added(rb.clone()));
        }
    }
    out.sort_by(|x, y| sort_key(x).cmp(&sort_key(y)));
    out
}

fn sort_key(c: &CensusChange) -> (&'static str, &'static str, String) {
    let (tag, r) = match c {
        CensusChange::Added(r) => ("added", r),
        CensusChange::Changed { to, .. } => ("changed", to),
        CensusChange::Removed(r) => ("removed", r),
    };
    (tag, r.layer, r.entity.clone())
}
```
Adaptation note: `CensusRow` must derive `Clone`, `PartialEq`, `Eq`, `Ord` for this (`census` already sorts, so an `Ord` derive is consistent with existing behaviour). `r.layer` is a `&'static str` today — confirm and match. Add `use std::collections::BTreeMap;` if the module does not already have it.

- [ ] **Step 5: Run Rust green, then add the route**

```bash
cargo test -p map-canon 2>&1 | grep "test result"
```
Then in `crates/map-viewer/src/lib.rs`, in the `"/api/census"` arm, before the existing single-instant body:
```rust
        "/api/census" => {
            let Some(year) = p.year("year") else { return bad("year required") };
            // Additive: `to=` asks for the DIFF between two instants.
            // Without it the route answers exactly as it did in v0.1,
            // so no blessed census fixture moves.
            if let Some(to) = p.year("to") {
                let mut s = String::from("[");
                for (i, c) in map_canon::census_diff(&app.canon, &year, &to).iter().enumerate() {
                    if i > 0 { s.push(','); }
                    let (tag, from, to_row) = match c {
                        map_canon::CensusChange::Added(r) => ("added", None, Some(r)),
                        map_canon::CensusChange::Removed(r) => ("removed", Some(r), None),
                        map_canon::CensusChange::Changed { from, to } => ("changed", Some(from), Some(to)),
                    };
                    s.push_str(&format!("{{\"change\":\"{}\"", tag));
                    if let Some(r) = from { s.push_str(&format!(",\"from\":{}", census_row_json(r))); }
                    if let Some(r) = to_row { s.push_str(&format!(",\"to\":{}", census_row_json(r))); }
                    s.push('}');
                }
                s.push(']');
                return (200, "application/json", s, Vec::new());
            }
            // ... the existing single-instant body, unchanged ...
        }
```
Factor the existing row-writing into `fn census_row_json(r: &map_canon::CensusRow) -> String` and use it on both paths, so the two can never disagree. Adaptation notes: `app.canon` and `p.year` — match the accessors the existing arm already uses; and this arm sits inside a function whose return type may not permit a bare `return` of that tuple shape — if not, restructure as an `if/else` yielding the tuple rather than adding a second return path.

- [ ] **Step 6: Teach the runner the three new steps**

In `contracts/runner/src/Steps.hs`:

*A styleless render.* `sceneUrl` always emits `&style=`, so "in no style" is currently inexpressible (diagnosis §9.1 item 1's attached work). Add a sibling that omits it, and a step:
```haskell
sceneUrlNoStyle :: Text -> PieceSet -> Year -> Text
sceneUrlNoStyle base ps y = T.replace ("&style=" <> defaultStyleText) "" (sceneUrl base ps y (StyleName defaultStyleText))
```
No — do not build it by string surgery on another URL; that is the kind of trick the owner's law forbids. Instead factor `sceneUrl` so the style is a `Maybe`:
```haskell
-- One builder, style optional: "no style given" is a real point in the
-- parameter's domain (spec §3 law 4, default-totality), not a string
-- edit of a styled URL.
sceneUrlWith :: Text -> PieceSet -> Year -> Maybe StyleName -> Text
sceneUrlWith base ps y mst = <the existing body, emitting "&style=" only when mst is Just>

sceneUrl :: Text -> PieceSet -> Year -> StyleName -> Text
sceneUrl base ps y st = sceneUrlWith base ps y (Just st)
```
and add the step definition (place it BEFORE the styled render definitions, per the file's own ordering rule — specific before generic):
```haskell
  , mkStep When (lit "I render pieces "
                 *> ((,) <$> capUntil @PieceSet " at year " <*> capRest @Year)
                 <* lit " in no style") $
      \(ps, y) w -> getUrl (sceneUrlWith (baseUrl w) ps y Nothing) w
```
Adaptation note: `capRest` consumes to end of line, so the trailing `lit` above will not match. Write it as `capUntil @Year " in no style"` with nothing after, and check `matchP`'s trailing-text rule.

*A bound GET.* `When I GET <path> as <name>` does not exist (the `@property` census-diff scenario needs it). Add it next to the existing `I GET` definition, before it:
```haskell
  , mkStep When (lit "I GET " *> ((,) <$> capUntil @UrlPath " as " <*> capRest @BindName)) $
      \(path, BindName n) w -> do
        r <- getUrl (baseUrl w <> renderCap path) w
        pure (r >>= bindLast n)
```

*A bound fixture comparison.* `Then <name> equals fixture <fixture>`:
```haskell
  , mkStep Then (lit "" *> ((,) <$> capUntil @BindName " equals fixture " <*> capRest @FixtureRef)) $
      \(BindName n, FixtureRef f) w -> case Map.lookup n (bound w) of
        Nothing -> pure (Left ("unbound " <> n))
        Just pair -> blessOrCompare f w { bound = Map.insert "_last" pair (bound w) }
```

*The derivability `@target`.* It must fail HONESTLY — naming the missing routes, never passing vacuously:
```haskell
  , mkStep Then (lit "every entry in " *> capUntil @BindName " traces to a disposition and a border") $
      \(BindName n) w -> pure $ case Map.lookup n (bound w) of
        Nothing -> Left ("unbound " <> n)
        Just _  -> Left "derivability cannot be sampled: /api/disposition (Stage 2) \
                        \and /api/borders (Stage 3) do not exist yet"
  ]
```

- [ ] **Step 7: Make the vocabulary exemption explicit (§9.2)**

Three of twelve features carry no `Vocabulary:` block, so `vocab` passes vacuously over a quarter of the corpus while announcing "every table matches its types". In `contracts/runner/src/Vocab.hs`'s success message, count and name what was actually examined:
```haskell
  TIO.putStrLn $ "vocabulary: " <> tshow withTables <> " table(s) match their types; "
    <> tshow (length exempt) <> " feature(s) declare no vocabulary (every capture Described): "
    <> T.intercalate ", " exempt
```
This is a message change, not a law change — but it stops a green from sounding like a statement about ground it never examined.

- [ ] **Step 8: Rebuild, restart detached, bless the three new fixtures, run everything**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
```bash
cd contracts/runner
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api --bless
```
**Then eyeball all three new fixtures.** `census-diff-empty.json` must be `[]`. `census-diff-1405-1050.json` must be a real change list naming real entities — if it is empty, the diff instrument is broken and blessing it would pin the bug forever. `scene-1405-default-dress.json` must be a real manifest, not an error body. A blessed error is a lie that passes forever.

- [ ] **Step 9: Full gate sweep and commit**

```bash
make contract-gates
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
```
Expected: map-api now 22 green / 3 `@target` red (composition, piece attribution, derivability) / 0 unexpected red; `ALL GOLDEN VIEWS HOLD`.
```bash
git add crates/map-canon crates/map-viewer contracts/
git commit -m "the census diff instrument, default-totality, and derivability declared @target"
```

---

## Part B — pieces made first-class

### Task 8: `Piece` and `PieceSet` in map-types, replacing `LayerSet`

The provider's own comment names the gap: *"Recorded at push time because the scene type carries no layer."* Today a query selects `LayerSet` — a five-flag `u8` bitset whose `GEOMETRY` bit lumps Background, Territory and ScriptureClaims together, so fills, borders and claims cannot be selected apart. That is why only four of ten pieces are toggleable on the wire, which is the wart the scene feature's preamble records. Replace it with the spec's own vocabulary as a first-class type; `LayerKind` selection becomes a total function of it.

**Files:**
- Create: `crates/map-types/src/piece.rs`
- Modify: `crates/map-types/src/lib.rs`, `crates/map-types/src/query.rs`, `crates/map-types/src/style.rs` (delete `LayerSet`)
- Modify: `crates/map-types/src/tests.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
```rust
// map-types::piece
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Piece { Ground, Water, Fills, Borders, Claims, Labels, Markers, Journeys, Chrome, Veil }
impl Piece { pub const ALL: [Piece; 10]; pub fn name(self) -> &'static str; pub fn parse(s: &str) -> Option<Piece>; }
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PieceSet(u16);
impl PieceSet {
    pub fn empty() -> Self; pub fn all() -> Self;
    pub fn with(self, p: Piece) -> Self; pub fn without(self, p: Piece) -> Self;
    pub fn contains(self, p: Piece) -> bool;
    pub fn union(self, other: Self) -> Self; pub fn bits(self) -> u16;
    pub fn iter(self) -> impl Iterator<Item = Piece>;
    pub fn parse(s: &str) -> Result<Self, String>;   // "ground, water" | "none" | "all"
    pub fn render(self) -> String;                    // sorted, comma-space
}
```
`RenderQuery.layers: LayerSet` becomes `RenderQuery.pieces: PieceSet`. Task 9 consumes `PieceSet`; Task 11 parses it off the wire.

- [ ] **Step 1: Write the failing tests**

Append to `crates/map-types/src/tests.rs`:
```rust
#[test]
fn piece_set_is_a_monoid_over_pieces_and_round_trips() {
    use crate::piece::{Piece, PieceSet};

    // identity
    assert_eq!(PieceSet::empty().union(PieceSet::all()), PieceSet::all());
    assert_eq!(PieceSet::all().union(PieceSet::empty()), PieceSet::all());
    // associativity, over every triple of singletons
    for a in Piece::ALL { for b in Piece::ALL { for c in Piece::ALL {
        let (sa, sb, sc) = (PieceSet::empty().with(a), PieceSet::empty().with(b), PieceSet::empty().with(c));
        assert_eq!(sa.union(sb).union(sc), sa.union(sb.union(sc)));
    }}}
    // with/without are inverse on every piece, from every starting set
    for p in Piece::ALL {
        assert!(PieceSet::empty().with(p).contains(p));
        assert!(!PieceSet::all().without(p).contains(p));
        assert_eq!(PieceSet::all().without(p).with(p), PieceSet::all());
    }
    // every set round-trips through its rendering, INCLUDING the empty one
    let mut s = PieceSet::empty();
    assert_eq!(PieceSet::parse(&s.render()), Ok(s), "the empty set is a set");
    for p in Piece::ALL {
        s = s.with(p);
        assert_eq!(PieceSet::parse(&s.render()), Ok(s));
    }
    // and the whole 2^10 lattice, so no subset is unreachable
    for bits in 0u16..1024 {
        let set = Piece::ALL.iter().enumerate()
            .filter(|(i, _)| bits & (1 << i) != 0)
            .fold(PieceSet::empty(), |acc, (_, p)| acc.with(*p));
        assert_eq!(PieceSet::parse(&set.render()), Ok(set), "bits {bits}");
        assert_eq!(set.iter().count(), bits.count_ones() as usize);
    }
    // a wrong name is refused BY NAME -- not silently dropped
    assert!(PieceSet::parse("ground, topografy").is_err());
    assert!(PieceSet::parse("ground, topografy").unwrap_err().contains("topografy"));
}
```

- [ ] **Step 2: Run and verify FAIL** — `cargo test -p map-types piece_set` → module `piece` not found.

- [ ] **Step 3: Write `crates/map-types/src/piece.rs`**

```rust
//! THE PIECE: what a scene is composed of (spec §3). A piece is a
//! first-class thing that owns its own geometry — not a render flag.
//! `LayerSet`, which this replaces, lumped Background, Territory and
//! ScriptureClaims under one GEOMETRY bit, so fills, borders and claims
//! could not be selected apart; that is why only four of ten pieces
//! were toggleable on the wire, and why the manifest had nothing
//! truthful to stamp on an entry.
//!
//! Laws (crates/map-types/src/tests.rs): PieceSet is a monoid under
//! union with the empty set as identity; with/without are inverse;
//! every one of the 2^10 subsets round-trips through render/parse.

/// Extensible by adding a variant here and one arm in `name`/`parse` —
/// the compiler names every site that must follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Piece {
    Ground, Water, Fills, Borders, Claims, Labels, Markers, Journeys, Chrome, Veil,
}

impl Piece {
    pub const ALL: [Piece; 10] = [
        Piece::Ground, Piece::Water, Piece::Fills, Piece::Borders, Piece::Claims,
        Piece::Labels, Piece::Markers, Piece::Journeys, Piece::Chrome, Piece::Veil,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Piece::Ground => "ground", Piece::Water => "water", Piece::Fills => "fills",
            Piece::Borders => "borders", Piece::Claims => "claims", Piece::Labels => "labels",
            Piece::Markers => "markers", Piece::Journeys => "journeys",
            Piece::Chrome => "chrome", Piece::Veil => "veil",
        }
    }
    pub fn parse(s: &str) -> Option<Piece> {
        Piece::ALL.into_iter().find(|p| p.name() == s.trim())
    }
    fn bit(self) -> u16 {
        1 << Piece::ALL.iter().position(|p| *p == self).expect("Piece::ALL is total")
    }
}

/// A subset of the pieces. The empty set is the monoid identity and a
/// legal query, never an error (spec §3 law 1, omission-totality).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PieceSet(u16);

impl PieceSet {
    pub fn empty() -> Self { PieceSet(0) }
    pub fn all() -> Self { Piece::ALL.into_iter().fold(PieceSet(0), |s, p| s.with(p)) }
    pub fn with(self, p: Piece) -> Self { PieceSet(self.0 | p.bit()) }
    pub fn without(self, p: Piece) -> Self { PieceSet(self.0 & !p.bit()) }
    pub fn contains(self, p: Piece) -> bool { self.0 & p.bit() != 0 }
    pub fn union(self, other: Self) -> Self { PieceSet(self.0 | other.0) }
    pub fn bits(self) -> u16 { self.0 }
    pub fn iter(self) -> impl Iterator<Item = Piece> {
        Piece::ALL.into_iter().filter(move |p| self.contains(*p))
    }
    /// "none" is the empty set's own token: "" would be ambiguous with a
    /// malformed list, and a set that cannot be written cannot be a
    /// value in a contract scenario.
    pub fn render(self) -> String {
        if self.0 == 0 { return "none".to_string(); }
        self.iter().map(Piece::name).collect::<Vec<_>>().join(", ")
    }
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s == "none" { return Ok(PieceSet::empty()); }
        if s == "all" { return Ok(PieceSet::all()); }
        let mut out = PieceSet::empty();
        for part in s.split(',') {
            match Piece::parse(part) {
                Some(p) => out = out.with(p),
                None => return Err(format!(
                    "'{}' is not a piece. Pieces are: {}",
                    part.trim(),
                    Piece::ALL.iter().map(|p| p.name()).collect::<Vec<_>>().join(", "))),
            }
        }
        Ok(out)
    }
}
```
Note `render` emits in `Piece::ALL` order, and the round-trip law holds for that order — the contract runner's `PieceSet` renders alphabetically, which is a *different* rendering of the same set. Both parse; the law is round-trip, not cross-language string equality. Do not "fix" one to match the other.

- [ ] **Step 4: Replace `LayerSet` with `PieceSet` in `RenderQuery`, and delete `LayerSet`**

- `crates/map-types/src/lib.rs`: add `pub mod piece;` and re-export `pub use piece::{Piece, PieceSet};` alongside the existing re-exports.
- `crates/map-types/src/query.rs`: `pub layers: LayerSet` → `pub pieces: PieceSet`; in `canonical_bytes`, `c.u8_(self.layers.bits())` → `c.u16_(self.pieces.bits())` (if `Canon` has no `u16_`, use `c.u64_(self.pieces.bits() as u64)` — do not invent a new Canon method in this task).
- `crates/map-types/src/style.rs`: delete the `LayerSet` struct and its `impl`. The compiler now names every consumer; Task 9 fixes the provider, Task 11 the viewer. Leave those broken until then only if you are executing 8–11 back to back; otherwise do Steps 5–6 of Task 9 and Step 3 of Task 11 in the same commit as this one so the workspace always builds.

> **Executor note:** Tasks 8, 9 and 10 leave the workspace un-buildable between them. Execute them as one continuous run, committing at each task boundary only after `cargo build --workspace` succeeds. If a reviewer's gate must sit between them, merge 8+9 into one commit.

- [ ] **Step 5: Run map-types tests green** — `cargo test -p map-types 2>&1 | grep "test result"`.

- [ ] **Step 6: Commit (after Task 9's build is green)**

```bash
git add crates/map-types
git commit -m "Piece and PieceSet: the scene's vocabulary as a first-class type; LayerSet dies"
```

---

### Task 9: The provider stamps a piece on every scene element

Every scene element already knows its piece at push time — `push_area` takes `layer: LayerKind`, `push_way` is the Journeys layer, `Feature::Point` is Markers, the `RenderSubject::Point` tail is Markers — and the information is thrown away because the scene type has nowhere to put it. Give it somewhere.

**The attribution function.** Piece is a total function of (LayerKind, element kind). Not a special case, not a lookup table with exceptions:

| LayerKind | region | boundary | marker | label |
|---|---|---|---|---|
| `Relief` | Ground | — | — | — |
| `Water` | Water | Water | — | Labels |
| `Background` | Fills | Borders | — | Labels |
| `Territory` | Fills | Borders | — | Labels |
| `ScriptureClaims` | Claims | Claims | — | Labels |
| `Journeys` | — | Journeys | Journeys | Labels |
| (no layer: `Feature::Point`, `RenderSubject::Point`) | — | — | Markers | Labels |

`Chrome` and `Veil` are **dress pieces**: they contribute no scene elements at all, and their contribution to the manifest is the `dress` block. That is a declared property of those two pieces, written into the contract in Task 12 — not a gap papered over.

**Files:**
- Modify: `crates/map-types/src/scene.rs`
- Modify: `crates/map-provider/src/canon_provider.rs`
- Modify: `crates/map-provider/src/tests.rs`

**Interfaces:**
- Consumes: `map_types::{Piece, PieceSet}` (Task 8).
- Produces: `pub piece: Piece` on `StyledRegion`, `StyledBoundary`, `StyledMarker`, `PlacedLabel`; `Snapshot::restrict(self, PieceSet) -> Snapshot`; in the provider, `fn piece_of_region(LayerKind) -> Option<Piece>`, `fn piece_of_boundary(LayerKind) -> Option<Piece>`. Task 10 reads `piece` off every element.

- [ ] **Step 1: Write the failing tests**

Append to `crates/map-provider/src/tests.rs`:
```rust
#[test]
fn every_scene_element_names_its_piece_and_restriction_is_subtractive() {
    use map_types::{Piece, PieceSet};
    let p = fixture_provider();               // this file's existing builder
    let q = world_query(-1405, PieceSet::all()); // ditto, adapted to take a PieceSet
    let whole = p.render(&q).expect("render");

    // Totality: not one element is unattributed. (The type makes this
    // true by construction; the test exists so the day someone adds a
    // fifth element list, it fails.)
    assert!(!whole.regions.is_empty() && !whole.boundaries.is_empty());

    // Discrimination: the attribution is not constant. A single-valued
    // `piece` would satisfy "every element names its piece" while
    // saying nothing -- the §6.3 trap, refused here.
    let region_pieces: std::collections::BTreeSet<_> =
        whole.regions.iter().map(|r| r.piece).collect();
    assert!(region_pieces.len() >= 2, "regions come from more than one piece: {region_pieces:?}");

    // Subtractivity, per piece, over EVERY piece: restricting to a set
    // yields exactly the elements attributed to that set.
    for omitted in Piece::ALL {
        let kept = PieceSet::all().without(omitted);
        let part = p.render(&world_query(-1405, kept)).expect("render");
        assert!(part.regions.iter().all(|r| r.piece != omitted));
        assert!(part.boundaries.iter().all(|b| b.piece != omitted));
        assert!(part.markers.iter().all(|m| m.piece != omitted));
        assert_eq!(
            part.regions,
            whole.regions.iter().filter(|r| r.piece != omitted).cloned().collect::<Vec<_>>(),
            "omitting {omitted:?} removed something else too");
        assert_eq!(
            part.markers,
            whole.markers.iter().filter(|m| m.piece != omitted).cloned().collect::<Vec<_>>(),
            "omitting {omitted:?} disturbed the markers of other pieces");
    }
}
```
Adaptation note: `fixture_provider` and the query builder exist in `crates/map-provider/src/tests.rs` under this file's own names — read them and reuse. `Chrome` and `Veil` iterations of the loop are trivially satisfied (they attribute nothing), which is correct and is exactly what Task 12's contract scenario declares.

- [ ] **Step 2: Run and verify FAIL** — `cargo test -p map-provider every_scene_element_names_its_piece` → no field `piece`.

- [ ] **Step 3: Add `piece` to the four scene element types**

In `crates/map-types/src/scene.rs`, add `pub piece: crate::piece::Piece,` to `StyledRegion`, `StyledBoundary`, `StyledMarker` and `PlacedLabel`, each with a doc line:
```rust
    /// WHICH PIECE this element belongs to. The scene type used to carry
    /// no such notion, so the provider recorded paint rank at push time
    /// and threw the rest away — which is why the encoder could only
    /// group markers by paint, why one points buffer held every piece's
    /// markers, and why omitting journeys changed the bytes of a buffer
    /// other pieces were using (diagnosis §3.2).
    pub piece: crate::piece::Piece,
```
Include it in `Snapshot::canonical_bytes` for each of the four sequences (`c.str_(r.piece.name());` and so on). Two scenes differing only in attribution are genuinely different answers, so the pid must see it — this changes `scene_revision`, which re-blesses the scene fixtures in Task 11 under a version bump, and moves no pixels.

Add the restriction operator next to `select_region`:
```rust
    /// Spec §3 law 1: `scene(q \ P)` equals `scene(q)` minus exactly P's
    /// contribution. With every element attributed, that is a filter —
    /// the law becomes a definition rather than a hope.
    pub fn restrict(&self, keep: crate::piece::PieceSet) -> Snapshot {
        Snapshot {
            regions: self.regions.iter().filter(|r| keep.contains(r.piece)).cloned().collect(),
            boundaries: self.boundaries.iter().filter(|b| keep.contains(b.piece)).cloned().collect(),
            markers: self.markers.iter().filter(|m| keep.contains(m.piece)).cloned().collect(),
            labels: self.labels.iter().filter(|l| keep.contains(l.piece)).cloned().collect(),
            attribution: self.attribution.clone(),
        }
    }
```

- [ ] **Step 4: Stamp the piece at every push site**

In `crates/map-provider/src/canon_provider.rs`:
```rust
/// The attribution function: a piece is determined by the layer a
/// feature lives in and the KIND of element it produces. Total by
/// construction — a new LayerKind fails to compile until it declares
/// which piece its regions and boundaries belong to.
fn piece_of_region(l: LayerKind) -> Option<Piece> {
    match l {
        LayerKind::Relief => Some(Piece::Ground),
        LayerKind::Water => Some(Piece::Water),
        LayerKind::Background | LayerKind::Territory => Some(Piece::Fills),
        LayerKind::ScriptureClaims => Some(Piece::Claims),
        LayerKind::Journeys => None, // a way is a line, never a face
    }
}

fn piece_of_boundary(l: LayerKind) -> Option<Piece> {
    match l {
        LayerKind::Relief => None,   // relief bands draw no outline today
        LayerKind::Water => Some(Piece::Water),
        LayerKind::Background | LayerKind::Territory => Some(Piece::Borders),
        LayerKind::ScriptureClaims => Some(Piece::Claims),
        LayerKind::Journeys => Some(Piece::Journeys),
    }
}
```
Then: `push_area` stamps `piece: piece_of_region(layer)` on its `StyledRegion` and `piece_of_boundary(layer)` on its `StyledBoundary` (returning early where the function says `None`); `push_way` stamps `Piece::Journeys` on its boundary AND on its station markers; the `Feature::Point` arm stamps `Piece::Markers`; the `RenderSubject::Point` tail stamps `Piece::Markers`; every `PlacedLabel` stamps `Piece::Labels`.

- [ ] **Step 5: Select by piece, not by layer**

Replace `layers_wanted(bits: LayerSet) -> Vec<LayerKind>` with:
```rust
/// Which layers can contribute anything to this piece set. A layer is
/// visited when ANY of its pieces is wanted; the per-element `piece`
/// stamp then does the fine-grained filtering, so fills, borders and
/// claims are genuinely separable for the first time.
fn layers_wanted(pieces: PieceSet) -> Vec<LayerKind> {
    [
        LayerKind::Relief, LayerKind::Background, LayerKind::ScriptureClaims,
        LayerKind::Territory, LayerKind::Water, LayerKind::Journeys,
    ]
    .into_iter()
    .filter(|l| {
        piece_of_region(*l).is_some_and(|p| pieces.contains(p))
            || piece_of_boundary(*l).is_some_and(|p| pieces.contains(p))
            || (pieces.contains(Piece::Markers) && *l == LayerKind::Territory)
            || (pieces.contains(Piece::Journeys) && *l == LayerKind::Journeys)
    })
    .collect()
}
```
and at every push site, skip an element whose stamped piece is not in `q.pieces`. `q.layers.contains(LayerSet::LABELS)` becomes `q.pieces.contains(Piece::Labels)`, everywhere (four sites).

- [ ] **Step 6: Run the whole workspace green**

```bash
cargo build --workspace && cargo test --workspace 2>&1 | grep -c "test result: ok"
```
Expected: 19 (map-viewer will not compile until Task 11 Step 3; do those two edits now if the build stops there — `RenderQuery { layers: … }` becomes `RenderQuery { pieces: … }` and `build_query` maps today's flags onto a `PieceSet`).

- [ ] **Step 7: Commit**

```bash
git add crates/map-types crates/map-provider crates/map-viewer
git commit -m "the provider stamps a piece on every scene element; the scene type carries the layer at last"
```

---

### Task 10: Split the shared points buffer; stamp `piece` on every manifest entry

Task 1's finding, fixed. Group markers by `(piece, style)` rather than by style alone, and put the piece on every manifest entry. Both `@target` reds are downstream of this one change.

**Files:**
- Modify: `crates/map-encoders/src/gpu.rs`
- Modify: `crates/map-encoders/src/tests.rs`
- Modify: `crates/map-viewer/src/page.html` *(only if the client needs it — see Step 5)*

**Interfaces:**
- Consumes: `piece` on the four scene element types (Task 9).
- Produces: `pub piece: Piece` on `FeatureInstance`, `LabelResource`, `MarkerResource`; manifest JSON entries gain `"piece":"<name>"` in `features[]`, `labels[]` and `markers[]`. Task 11's contract scenarios read that field; Task 12's laws are stated over it.

- [ ] **Step 1: Invert Task 1's test and add the new laws**

In `crates/map-encoders/src/tests.rs`, replace `markers_from_two_origins_share_one_points_buffer` with:
```rust
/// TASK 10 (Stage 1): the inversion of Task 1's pin. Two markers of
/// different pieces wearing the SAME paint now land in TWO buffers, so
/// omitting one piece cannot change the other's bytes.
#[test]
fn markers_of_different_pieces_never_share_a_points_buffer() {
    use map_types::piece::Piece;
    use map_types::scene::{Snapshot, StyledMarker};
    use map_types::style::{MarkerStyle, Rgba};
    use map_types::{SceneEncoder, UnitVec};

    let paint = MarkerStyle { color: Rgba(10, 20, 30, 255), size: 3.0 };
    let mk = |lat: f64, piece: Piece| StyledMarker {
        at: UnitVec::from_lat_lon_deg(lat, 35.0),
        style: paint, sources: Default::default(), place: None, piece,
    };
    let scene = Snapshot {
        markers: vec![mk(31.0, Piece::Markers), mk(32.0, Piece::Journeys)],
        ..Snapshot::default()
    };
    let enc = crate::gpu::GpuSceneEncoder::default().encode(&scene).expect("encode");

    let points: Vec<_> = enc.resources.iter()
        .filter(|r| r.descriptor.kind == crate::gpu::ResourceKind::Points).collect();
    assert_eq!(points.len(), 2, "one buffer per (piece, paint), not per paint");
    assert!(points.iter().all(|r| r.descriptor.vertex_count == 1));

    // THE LAW THE WHOLE STAGE IS FOR: dropping one piece leaves the
    // other's resource byte-identical.
    let only_markers = scene.restrict(map_types::PieceSet::empty().with(Piece::Markers));
    let enc2 = crate::gpu::GpuSceneEncoder::default().encode(&only_markers).expect("encode");
    let kept: Vec<_> = enc2.resources.iter()
        .filter(|r| r.descriptor.kind == crate::gpu::ResourceKind::Points).collect();
    assert_eq!(kept.len(), 1);
    assert!(points.iter().any(|r| r.descriptor.id == kept[0].descriptor.id),
            "the surviving piece's buffer kept its content address");
}

/// Attribution is present AND correct AND varies — §6.3's trap refused:
/// a server that stamps one value on everything must fail this.
#[test]
fn every_manifest_entry_names_its_piece_correctly() {
    use map_types::piece::Piece;
    // ... build a scene with a Fills region, a Borders boundary, a
    // Journeys station marker and a Labels label (mirror the
    // constructors above) ...
    let enc = crate::gpu::GpuSceneEncoder::default().encode(&scene).expect("encode");
    let seen: std::collections::BTreeSet<_> =
        enc.manifest.features.iter().map(|f| f.piece).collect();
    assert!(seen.contains(&Piece::Fills) && seen.contains(&Piece::Borders)
            && seen.contains(&Piece::Journeys));
    assert!(seen.len() >= 3, "a constant attribution is not an attribution");
    assert!(enc.manifest.labels.iter().all(|l| l.piece == Piece::Labels));
    let json = enc.manifest_json();
    assert!(json.contains("\"piece\":\"fills\"") && json.contains("\"piece\":\"borders\""));
}
```

- [ ] **Step 2: Run and verify FAIL** — `cargo test -p map-encoders` → `points.len()` is 1, and `FeatureInstance` has no `piece`.

- [ ] **Step 3: Split the aggregation and carry the piece**

In `crates/map-encoders/src/gpu.rs`, add `pub piece: Piece` to `FeatureInstance`, `LabelResource` and `MarkerResource`, then replace the marker block:
```rust
        // MARKERS, BY PIECE AND PAINT. The previous key was paint alone,
        // so every piece's markers landed in one buffer whose CONTENTS
        // depended on which pieces were enabled — turning journeys off
        // changed a buffer that Markers was also using, changing its
        // content address, so `noJourneys`' resource set was not a
        // subset of the full scene's (diagnosis §3.2). The piece is now
        // part of the grouping key, so a piece's buffer is a function of
        // that piece alone.
        let mut by_piece_style: BTreeMap<(Piece, StyleKey), Vec<UnitVec>> = BTreeMap::new();
        for m in &scene.markers {
            let sk = style_of(&mut styles, GpuStyle::Marker { color: m.style.color, size: m.style.size });
            by_piece_style.entry((m.piece, sk)).or_default().push(m.at);
        }
        for ((piece, sk), pts) in by_piece_style {
            let (id, geom) = add(&mut resources, &mut seen, ResourceKind::Points, &pts);
            features.push(FeatureInstance {
                feature: format!("markers:{}", piece.name()),
                geometry: geom, resource: id, style: sk, piece,
            });
        }
```
and stamp `piece: r.piece` / `piece: b.piece` / `piece: l.piece` on the region, boundary, label and marker constructions above and below it.

**Note on the feature key.** `"markers"` becomes `"markers:<piece>"` so the key stays a faithful name for what the entry holds. That is a wire change to an existing field — it moves fixtures and needs the version bump, which Task 17 makes. Check `page.html`'s consumers of `f.feature` (Step 5) before committing.

- [ ] **Step 4: Emit `piece` in `manifest_json`**

In the `features[]` writer:
```rust
                "{}{{\"feature\":\"{}\",\"piece\":\"{}\",\"geometry\":\"{:016x}\",\"resource\":\"{:016x}\",\"style\":\"{:016x}\"}}",
                if i > 0 { "," } else { "" }, f.feature, f.piece.name(),
                f.geometry.0 .0, f.resource.0 .0, f.style.0
```
and add `"piece": l.piece.name()` to the `labels[]` `serde_json::json!` row and `"piece": m2.piece.name()` to the `markers[]` row.

- [ ] **Step 5: Check the client, and the paint order**

`crates/map-viewer/src/page.html` groups consecutive fill entries by `f.feature` and reads `it.key.startsWith("region:")`; markers are handled per-entry (`items.push({ type: "marker", key: f.feature, id: f.resource, … })`). Splitting one marker entry into N adds N items drawing the same points with the same style, in `(piece, style)` key order. Read the block around `page.html:855–890` and confirm:
1. nothing matches on the exact string `"markers"` (grep it);
2. the fill-grouping logic is unaffected (marker entries reset `fillGroup` either way).

If both hold, the client needs no change. If either fails, the fix is one line in `page.html` and it belongs in this commit.

- [ ] **Step 6: Run encoder tests green, then the whole workspace**

```bash
cargo test -p map-encoders 2>&1 | grep "test result"
cargo test --workspace 2>&1 | grep -c "test result: ok"
```

- [ ] **Step 7: Commit**

```bash
git add crates/map-encoders crates/map-viewer
git commit -m "one buffer per (piece, paint): the shared points buffer splits, and every manifest entry names its piece"
```

---

### Task 11: A real `pieces=` parameter on `/api/scene`, and the golden gate

The wart the scene feature's preamble records — *"In v0.1 only ground, water, labels, and journeys are toggleable on the wire; the rest are always present"* — is now removable, because the provider can filter by piece.

**Files:**
- Modify: `crates/map-viewer/src/lib.rs` (`build_query`)
- Modify: `contracts/runner/src/Steps.hs` (`sceneUrlWith` emits `pieces=`)
- Re-bless: `contracts/map-api/fixtures/scene-1405-full.json`, `scene-1405-nolabels.json`, `scene-1405-default-dress.json`

**Interfaces:**
- Consumes: `PieceSet` (Task 8), the provider's piece filtering (Task 9), `piece` on the wire (Task 10).
- Produces: `/api/scene?pieces=ground,water,fills,…` — additive. When `pieces=` is absent, the legacy `labels=`/`topo=`/`relief=`/`journeys=` flags decide, exactly as today, so the viewer page and the golden gate are untouched.

- [ ] **Step 1: Write the failing Rust test for the parameter mapping**

Append to `crates/map-viewer/src/lib.rs`'s test module (or `crates/map-viewer/tests/` if that is where this crate's unit tests live — check):
```rust
#[test]
fn pieces_parameter_wins_and_the_legacy_flags_still_mean_what_they_meant() {
    use map_types::{Piece, PieceSet};
    let q = |s: &str| build_query(&test_app(), &params(s), "", None).expect("query");

    // Explicit pieces: exactly that set, nothing more.
    let p = q("year=-1405&pieces=fills,borders").pieces;
    assert_eq!(p, PieceSet::empty().with(Piece::Fills).with(Piece::Borders));

    // "none" is a legal query, not an error (omission-totality).
    assert_eq!(q("year=-1405&pieces=none").pieces, PieceSet::empty());

    // No `pieces=`: the v0.1 defaults, unchanged. Water and journeys on,
    // labels on, relief opt-in — this is the row the golden gate rests on.
    let d = q("year=-1405").pieces;
    assert!(d.contains(Piece::Water) && d.contains(Piece::Labels)
            && d.contains(Piece::Journeys) && d.contains(Piece::Fills)
            && d.contains(Piece::Borders) && d.contains(Piece::Claims));
    assert!(!d.contains(Piece::Ground), "relief is opt-in today");
    assert!(!q("year=-1405&labels=0").pieces.contains(Piece::Labels));
    assert!(!q("year=-1405&topo=0").pieces.contains(Piece::Water));
    assert!(!q("year=-1405&journeys=0").pieces.contains(Piece::Journeys));
    assert!(q("year=-1405&relief=1").pieces.contains(Piece::Ground));

    // A bad piece name is refused, not silently dropped.
    assert!(build_query(&test_app(), &params("year=-1405&pieces=topografy"), "", None).is_none());
}
```
Adaptation note: `test_app` and `params` — reuse whatever this crate's tests already use to construct an `App` and a `Params`. If `map-viewer` has no unit-test module, add one (`#[cfg(test)] mod tests { … }` at the end of `lib.rs`) rather than reaching for an integration test that needs a running server.

- [ ] **Step 2: Run and verify FAIL** — no field `pieces` on the query returned by `build_query`.

- [ ] **Step 3: Implement the parameter**

Replace `build_query`'s layer block:
```rust
    // THE PIECES PARAMETER (spec §3): the caller names the subset it
    // wants. Absent, the v0.1 flags decide — those are the viewer's own
    // parameters and the golden gate's, so they keep meaning exactly
    // what they meant. A malformed piece name is a rejected query, not
    // a silently smaller scene.
    let pieces = match p.get("pieces") {
        Some(spec) => PieceSet::parse(spec).ok()?,
        None => {
            let mut ps = PieceSet::empty()
                .with(Piece::Fills).with(Piece::Borders).with(Piece::Claims)
                .with(Piece::Markers).with(Piece::Chrome).with(Piece::Veil);
            if p.get("labels")   != Some("0") { ps = ps.with(Piece::Labels); }
            if p.get("topo")     != Some("0") { ps = ps.with(Piece::Water); }
            if p.get("journeys") != Some("0") { ps = ps.with(Piece::Journeys); }
            if p.get("relief")   == Some("1") { ps = ps.with(Piece::Ground); }
            ps
        }
    };
```
and `RenderQuery { subject, time, viewport, lod, pieces, style: … }`.

- [ ] **Step 4: Teach the runner to speak `pieces=`**

In `contracts/runner/src/Steps.hs`, `sceneUrlWith` now emits the piece set directly instead of the four-flag bridge:
```haskell
-- The v0.1 wart is retired: the wire takes the piece set itself, so the
-- contract's `pieces` vocabulary and the server's `PieceSet` are the
-- same set of names, and the four-flag bridge (which could only express
-- four of ten pieces) is gone.
sceneUrlWith :: Text -> PieceSet -> Year -> Maybe StyleName -> Text
sceneUrlWith base ps (Year y) mst =
  base <> "/api/scene?year=" <> T.pack (show y) <> "&zoom=90.0000"
       <> "&pieces=" <> renderCap ps
       <> maybe "" (\(StyleName s) -> "&style=" <> s) mst
```
Update the scene feature's preamble in the same commit — the wart sentence is now false, and a preamble is unverified prose that nothing else will catch:
```gherkin
  The scene is a monoid over pieces: any subset renders, absence is the
  identity, and adding a piece back changes nothing else. Every piece in
  the vocabulary below is selectable on the wire via `pieces=`. Two of
  them — chrome and veil — are DRESS pieces: they contribute no manifest
  entries, only the dress block, and the laws below say so explicitly
  rather than quietly excusing them.
```

- [ ] **Step 5: Rebuild, restart detached, and run the GOLDEN GATE FIRST**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
```bash
cd /c/Users/donov/.claude/jobs/c6946bce/tmp
node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -3
```
Expected: `ALL GOLDEN VIEWS HOLD`. **Run this BEFORE blessing anything.** Splitting shared buffers is exactly the change that could move pixels, and 89 blessed stops are the instrument that says so. A drift here STOPS the task: report the drifted probes to the owner and do not bless.

- [ ] **Step 6: Re-bless the three scene fixtures and eyeball them**

```bash
cd contracts/runner
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api --bless
```
Then confirm by hand, on `contracts/map-api/fixtures/scene-1405-full.json`: every `features[]` entry has a `piece`; more than one distinct `piece` value appears; there are now at least two `points` resources where there was one. If any of those is false, the bless captured a broken server.

- [ ] **Step 7: Commit**

```bash
git add crates/map-viewer contracts/
git commit -m "pieces= on the wire: the v0.1 four-flag wart retired, golden gate holds"
```

---

### Task 12: Strengthen the three weak laws and drop the `@target`s

Diagnosis §8.2 step 4. One piece of work unlocks four strengthenings; do them together so the corpus never sits in the state where the field exists and the laws still do not use it.

**Files:**
- Modify: `contracts/map-api/scene/scene.feature`
- Modify: `contracts/runner/src/Steps.hs`
- Modify: `contracts/runner/test/Spec.hs`

**Interfaces:**
- Consumes: `piece` on every manifest entry (Task 10), `pieces=` on the wire (Task 11), `somePiece` in `holeRegistry` (Task 4).
- Produces: three new steps and no new modules.

- [ ] **Step 1: Rewrite the scenarios**

Replace the three weak scenarios in `contracts/map-api/scene/scene.feature`:

```gherkin
  @property
  Scenario: omission is subtractive, not destructive — for EVERY piece
    When I render pieces all at year <someYear> in style canaan as whole
    And I render pieces all except <somePiece> at year <someYear> in style canaan as part
    Then part is whole with every <somePiece> entry removed and nothing else moved

  Scenario: every manifest entry names its piece, and names it correctly
    When I render pieces ground, water, fills, borders, labels, journeys at year -1405 in style canaan
    Then every entry names a piece, more than one piece is named, and no entry names a piece that was not requested

  Scenario: the dress pieces contribute no entries, by declaration
    When I render pieces chrome, veil at year -1405 in style canaan as dressOnly
    And I render pieces none at year -1405 in style canaan as nothing
    Then dressOnly equals nothing

  @property
  Scenario: composition — pieces render separately and combine to the whole
    When I render pieces <someA> at year <someYear> in style canaan as sceneA
    And I render pieces <someB> at year <someYear> in style canaan as sceneB
    Then combining sceneA and sceneB equals rendering <someA> plus <someB>
```
The `@target` tags come off "every manifest entry names its piece" and "composition". The "dress pieces contribute no entries" scenario is what keeps the declaration in the previous task honest: chrome and veil are exempt *because they render nothing*, and now that is a law rather than an excuse. Note it also proves the empty piece set is a legal, non-error answer — omission-totality's identity element, tested rather than assumed.

- [ ] **Step 2: Run and verify runner RED for the right reasons**

```bash
cd contracts/runner && cabal run contract-runner -- check ../map-api
```
Expected: ORPHAN rows for the three new step bodies. Then, once implemented, the two former `@target`s must go GREEN — if either stays red after Tasks 8–11, Part B did not do its job and that is a finding, not a tag to re-add.

- [ ] **Step 3: Write the failing Haskell tests for the three steps**

Append to `test/Spec.hs` — each with a NEGATIVE case, because a step that cannot fail is not a step:
```haskell
  describe "strengthened scene laws (Stage 1 Task 12)" $ do
    it "the subtractive step accepts a genuine subtraction" $
      subtractiveCheck fullBody partBodyWithJourneysRemoved "journeys" `shouldSatisfy` isRight
    it "the subtractive step REJECTS a part that lost someone else's entry" $
      subtractiveCheck fullBody partBodyMissingAFill "journeys" `shouldSatisfy` isLeft
    it "the subtractive step REJECTS a part that kept the omitted piece" $
      subtractiveCheck fullBody fullBody "journeys" `shouldSatisfy` isLeft
    it "the attribution step REJECTS a manifest that stamps one piece on everything" $
      attributionCheck constantPieceBody `shouldSatisfy` isLeft
    it "the attribution step REJECTS an empty manifest (no vacuous pass)" $
      attributionCheck emptyManifestBody `shouldSatisfy` isLeft
    it "the attribution step accepts a correctly varied manifest" $
      attributionCheck variedPieceBody `shouldSatisfy` isRight
```
(`fullBody`, `partBodyWithJourneysRemoved`, `partBodyMissingAFill`, `constantPieceBody`, `emptyManifestBody`, `variedPieceBody` are small hand-written JSON `Value`s at the top level of `Spec.hs` — four or five entries each, written by hand so the expected answer is obvious by inspection. `subtractiveCheck` and `attributionCheck` are the pure cores of the two steps, exported from `Steps` so they can be tested without a transport.)

- [ ] **Step 4: Implement the three steps**

In `Steps.hs`, the pure cores first (so the tests above can reach them), then the `mkStep` wrappers:
```haskell
-- Spec §3 law 1, in its strong form. `full - part` must be EXACTLY the
-- entries attributed to the omitted piece, and every other entry must be
-- byte-identical and in the same order. The old form asserted only
-- `part's resources ⊆ full's resources`, which a server that ignored the
-- pieces parameter entirely would satisfy — and which was green only
-- because the scenario happened to name `water` (diagnosis §6.2).
subtractiveCheck :: Value -> Value -> Text -> Either Text ()
subtractiveCheck full part omitted = do
  fe <- entriesOf full
  pe <- entriesOf part
  let survivors = [ e | e <- fe, pieceOf e /= Just omitted ]
  if not (null [ e | e <- pe, pieceOf e == Just omitted ])
    then Left ("the omitted piece '" <> omitted <> "' still has entries in the part")
    else if pe == survivors then Right ()
    else Left ("removing '" <> omitted <> "' moved something else: "
               <> maybe "(entry lists differ in length only)" id
                    (firstDiff (Array (V.fromList survivors)) (Array (V.fromList pe))))

-- Presence AND correctness AND variation. §6.3: a check that only
-- asserts the field exists blesses a server that stamps `ground` on
-- everything; a check that passes on an empty manifest blesses a server
-- with no entries at all. Both are refused here.
attributionCheck :: Value -> Either Text ()
attributionCheck v = do
  es <- entriesOf v
  if null es then Left "no entries to attribute — a vacuous pass is not a pass" else Right ()
  let ps = [ p | e <- es, Just p <- [pieceOf e] ]
  if length ps /= length es then Left "some entry carries no piece field" else Right ()
  if length (nub ps) < 2
    then Left ("every entry names the same piece (" <> head ps
               <> ") — a constant attribution is not an attribution")
    else Right ()
```
`entriesOf` concatenates the manifest's `features`, `labels` and `markers` arrays; `pieceOf` reads the `piece` string field. The wrapper for the third scenario needs no new step — `dressOnly equals nothing` uses the existing `{name} equals {name}`.

The "and no entry names a piece that was not requested" clause needs the requested set, which the step does not have. Thread it the same way `lastRender` threads year and style: add `lastPieces :: Maybe PieceSet` to `World`, set it on every render step, and read it in the attribution step. Do NOT smuggle it through the stringly-typed `bound` map — that is the exact mistake `lastRender`'s comment records.

- [ ] **Step 5: Run everything green**

```bash
cabal test 2>&1 | tail -8
cabal run contract-runner -- check ../map-api
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
```
Expected: **map-api all green except the one remaining `@target` (derivability)**. The two piece `@target`s are gone. If composition is still red, shrinking (Task 4) now hands you a one- or two-piece counterexample — chase it; do not re-tag it.

- [ ] **Step 6: Golden gate, then commit**

```bash
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
git add contracts/
git commit -m "the strengthened scene laws: subtractive per piece, attribution correct and varied, @targets dropped"
```

---

## Part C — the entity registry (spec §5 Stage 1 proper)

### Task 13: `map_canon::Registry` — canonical entities, witnesses, and typed unification reasons

Spec §2 equation 1: *"no witness mints an entity; every witness references the registry."* Today every ingestion path mints its own `EntityId` string with its own prefix, and the live census proves the consequence: at year −1050 `Phoenicia` is `partition:phoenicia` (scripture-claims) and `phoenicia` (territory); at −1405 `Dan` is `partition:dan` and `place:dan`; at AD 59 `Judea` is `basemap:judea` and `authored:judea`. Six id namespaces are in use (`basemap:`, `natural-earth:`, `etopo:`, `partition:`, `place:`, `authored:`, plus bare atlas ids).

**Files:**
- Create: `crates/map-canon/src/registry.rs`
- Modify: `crates/map-canon/src/lib.rs`
- Modify: `crates/map-canon/src/tests.rs`
- Create: `data/authored/registry.json`

**Interfaces:**
- Consumes: `map_canon::{EntityId, LayerKind, Witness, Feature}`.
- Produces:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityKind { Polity, District, People, Allotment, Promise, Vision, Waterbody, Terrain, Place, Route }

/// WHY two minted ids are one entity. A written, typed reason — never a
/// string coincidence. `bg_shadows` matched slugs; this does not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unification { SameId, Declared { reason: String, source: String } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessRef { pub minted_as: EntityId, pub witness: Witness, pub layer: LayerKind, pub kind: &'static str }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity { pub id: EntityId, pub names: Vec<String>, pub kind: EntityKind, pub witnesses: Vec<WitnessRef> }

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Registry { /* private */ }

impl Registry {
    pub fn declare(&mut self, canonical: EntityId, minted: EntityId, why: Unification) -> Result<(), String>;
    pub fn observe(&mut self, minted: EntityId, name: &str, kind: EntityKind, w: WitnessRef);
    pub fn resolve(&self, minted: &EntityId) -> &EntityId;          // TOTAL: unknown ids resolve to themselves
    pub fn get(&self, canonical: &EntityId) -> Option<&Entity>;
    pub fn entities(&self) -> impl Iterator<Item = &Entity>;
    pub fn why(&self, minted: &EntityId) -> Option<&Unification>;
    pub fn validate(&self) -> Vec<RegistryViolation>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryViolation {
    /// A declared canonical id nothing ever minted — a typo in the data file.
    DanglingCanonical(EntityId),
    /// A -> B -> C: resolution must be one hop, so the data says so directly.
    ChainedUnification { minted: EntityId, via: EntityId },
    /// Two witnesses of one entity disagree about its kind.
    KindConflict { entity: EntityId, a: EntityKind, b: EntityKind },
}
```
Tasks 14–16 consume `resolve`, `get`, `entities` and `why`. **Stage 2 consumes `resolve` and the canonical `EntityId` set** — see the closing Interfaces section.

- [ ] **Step 1: Write the failing tests**

Append to `crates/map-canon/src/tests.rs`:
```rust
#[test]
fn the_registry_resolves_totally_in_one_hop_and_refuses_chains() {
    use crate::registry::*;
    let mut r = Registry::default();
    let e = |s: &str| EntityId(s.to_string());
    let w = |s: &str, wit| WitnessRef { minted_as: e(s), witness: wit, layer: LayerKind::Territory, kind: "area" };

    r.observe(e("phoenicia"), "Phoenicia", EntityKind::Polity, w("phoenicia", Witness::Atlas));
    r.observe(e("partition:phoenicia"), "Phoenicia", EntityKind::Polity, w("partition:phoenicia", Witness::Authored));
    r.declare(e("phoenicia"), e("partition:phoenicia"),
              Unification::Declared { reason: "the atlas polity and the partitioned claim are one coast".into(),
                                      source: "data/authored/registry.json".into() }).unwrap();

    // TOTALITY: resolve never panics and never returns None.
    assert_eq!(r.resolve(&e("partition:phoenicia")), &e("phoenicia"));
    assert_eq!(r.resolve(&e("phoenicia")), &e("phoenicia"));
    assert_eq!(r.resolve(&e("never-heard-of-it")), &e("never-heard-of-it"));

    // IDEMPOTENCE: resolving a resolved id is a fixed point.
    let once = r.resolve(&e("partition:phoenicia")).clone();
    assert_eq!(r.resolve(&once), &once);

    // The entity has BOTH witnesses and one name.
    let ent = r.get(&e("phoenicia")).expect("canonical entity");
    assert_eq!(ent.names, vec!["Phoenicia".to_string()]);
    assert_eq!(ent.witnesses.len(), 2);
    assert!(r.why(&e("partition:phoenicia")).is_some(), "the reason is recorded, not implied");

    // DISCRIMINATION: a chain is refused, by name.
    r.observe(e("third:phoenicia"), "Phoenicia", EntityKind::Polity, w("third:phoenicia", Witness::Basemap));
    r.declare(e("partition:phoenicia"), e("third:phoenicia"),
              Unification::Declared { reason: "x".into(), source: "t".into() }).unwrap();
    assert!(r.validate().iter().any(|v| matches!(v, RegistryViolation::ChainedUnification { .. })));

    // DISCRIMINATION: a canonical id nobody minted is a typo, and is caught.
    let mut r2 = Registry::default();
    r2.observe(e("a"), "A", EntityKind::Polity, w("a", Witness::Atlas));
    r2.declare(e("typo"), e("a"), Unification::Declared { reason: "x".into(), source: "t".into() }).unwrap();
    assert!(r2.validate().iter().any(|v| matches!(v, RegistryViolation::DanglingCanonical(_))));
}

#[test]
fn slug_equality_alone_never_unifies_anything() {
    // The disease this replaces: `bg_shadows` unified on slugified name
    // equality. Two entities with the SAME slug and no declaration stay
    // two entities. Unification is a written act.
    use crate::registry::*;
    let mut r = Registry::default();
    let e = |s: &str| EntityId(s.to_string());
    let w = |s: &str| WitnessRef { minted_as: e(s), witness: Witness::Atlas, layer: LayerKind::Territory, kind: "area" };
    r.observe(e("basemap:judea"), "Judea", EntityKind::Polity, w("basemap:judea"));
    r.observe(e("authored:judea"), "Judea", EntityKind::Polity, w("authored:judea"));
    assert_ne!(r.resolve(&e("basemap:judea")), r.resolve(&e("authored:judea")));
    assert_eq!(r.entities().count(), 2);
}
```

- [ ] **Step 2: Run and verify FAIL** — `cargo test -p map-canon registry` → module `registry` not found.

- [ ] **Step 3: Implement `crates/map-canon/src/registry.rs`**

Write the module to the interface above, with these invariants enforced in the code rather than in comments:
- `resolve` is **total**: `self.aliases.get(minted).unwrap_or(minted)`. An unknown id is its own canonical id, so no caller ever handles an `Option` it cannot act on.
- `declare` records the reason; it never infers one. `Unification::SameId` is only ever produced by `observe` seeing a minted id it has already seen.
- `observe` merges names into a sorted deduped `Vec<String>` and appends the `WitnessRef`; witnesses sort by `(witness, layer, minted_as)` so the entity's byte form is deterministic.
- `validate` returns every violation, never the first — a compile that stops at the first typo makes the owner run it twenty times.
- No `slugify`. No normalization of names for matching. Name equality has no authority here; that is the whole point.

Add `pub mod registry;` to `crates/map-canon/src/lib.rs` and re-export `Registry`, `Entity`, `EntityKind`, `WitnessRef`, `Unification`, `RegistryViolation`.

- [ ] **Step 4: Write the owner-reviewable declaration file**

`data/authored/registry.json`, in the shape and spirit of the existing `data/authored/reconcile.json` (which is the precedent: *"the compiler refuses silent precedence… OWNER-REVIEWABLE DATA"*):
```json
{
  "_comment": "THE ENTITY REGISTRY (2026-09-07, spec §2 equation 1). One identity per real thing. Every row here is a written act: two minted ids are ONE entity because a person said so and said why. Slug equality has no authority — this file replaces the bg_shadows slug matcher in map-compile. OWNER-REVIEWABLE DATA.",
  "unifications": [
    {
      "canonical": "phoenicia",
      "minted": "partition:phoenicia",
      "kind": "Polity",
      "reason": "The atlas polity 'phoenicia' and the partitioned scripture-claims face 'partition:phoenicia' are the same coast, witnessed twice."
    }
  ]
}
```
**Populate the rest in Task 14, from evidence, not from guesses.** This file starts with the one case this plan verified against the live census; Task 14's Step 1 enumerates the real candidate set and brings it to the owner.

- [ ] **Step 5: Run map-canon tests green** — `cargo test -p map-canon 2>&1 | grep "test result"`.

- [ ] **Step 6: Commit**

```bash
git add crates/map-canon data/authored/registry.json
git commit -m "map_canon::Registry: one identity per real thing, unified by written reason and never by slug"
```

---

### Task 14: Populate the registry at compile time, and kill slug-matching supersession

`crates/map-compile/src/main.rs:392–425` builds `bg_shadows` by slugifying every atlas polity's `id` AND `name` and shadowing any background region whose slug matches. That is spec §2's "what dies" list, item three. It must die — but it must die **without moving a pixel**, and this is the riskiest task in the plan.

**Files:**
- Create: `crates/map-compile/src/registry_load.rs`
- Modify: `crates/map-compile/src/main.rs`
- Modify: `crates/map-compile/src/tests.rs`
- Modify: `crates/map-canon/src/persist.rs` (the registry must survive the compile→serve boundary)
- Modify: `data/authored/registry.json`

**Interfaces:**
- Consumes: `map_canon::registry::*` (Task 13).
- Produces: `parse_registry(json: &str) -> Result<Vec<(EntityId, EntityId, Unification, EntityKind)>, String>`; a populated `Registry` persisted alongside the canon and reachable from the provider as `store.registry()`.

- [ ] **Step 1: Establish the ground truth BEFORE changing anything**

Capture the current shadow set so the change can be judged rather than hoped about:
```bash
cargo run --release -p map-compile 2>&1 | tee /c/Users/donov/Documents/the-best-maps-ever/compile-before.log
grep -i "shadow" compile-before.log
```
Record the count the report line prints (`Background: N entity slugs shadowed…`). Then capture the current census at three instants for the diff review:
```bash
for Y in -1405 -1050 59; do curl -s "http://127.0.0.1:8090/api/census?year=$Y" -o census-before-$Y.json; done
```
Keep these three files out of git (`.gitignore` them or delete after Task 17) — they are working evidence, not artifacts.

- [ ] **Step 2: Write the failing test that pins the shadow set**

Append to `crates/map-compile/src/tests.rs`:
```rust
#[test]
fn the_registry_reproduces_the_slug_matcher_exactly_or_names_the_difference() {
    // The bg_shadows slug matcher is being deleted. The registry must
    // shadow the SAME set — or, where it does not, this test names the
    // difference so the owner rules on it. A silent difference here is a
    // pixel change nobody authorized.
    let polities = load_test_polities();               // this file's existing helper
    let legacy: BTreeSet<String> = legacy_slug_shadows(&polities);
    let registry = load_registry(include_str!("../../../data/authored/registry.json")).unwrap();
    let declared: BTreeSet<String> = registry_shadows(&registry, &polities);
    let only_legacy: Vec<_> = legacy.difference(&declared).collect();
    let only_declared: Vec<_> = declared.difference(&legacy).collect();
    assert!(only_legacy.is_empty() && only_declared.is_empty(),
        "registry and slug matcher disagree.\n  only the slug matcher shadows: {only_legacy:?}\
         \n  only the registry shadows: {only_declared:?}\n\
         Every name above needs a row in data/authored/registry.json or an owner ruling.");
}
```
`legacy_slug_shadows` is the current `slugify`+`bg_shadows` code lifted verbatim into the test file — it is the *specification of today's behaviour*, kept alive exactly long enough to prove the replacement matches, then deleted in Step 6.

- [ ] **Step 3: Run and see it fail with the real list**

`cargo test -p map-compile the_registry_reproduces` — the failure message IS the work list: every slug the matcher shadows and the registry does not.

- [ ] **Step 4: Bring that list to the owner**

> **STOP. OWNER GATE.** Present the failing list — every entity the slug matcher unified — and for each: the two minted ids, both names, and both layers. The owner rules on each: *unify (with a written reason), or leave separate (and accept the pixel change, which the golden gate will then show)*. Their rulings become rows in `data/authored/registry.json`, reason text included. Do not invent reasons; a row whose reason reads "matches by name" is the slug matcher with extra steps.

- [ ] **Step 5: Implement the loader**

`crates/map-compile/src/registry_load.rs` — parse `data/authored/registry.json` in the style of `reconcile.rs` (which is the file to imitate): explicit field errors naming the row, no silent defaults, a missing `reason` is a hard error.

- [ ] **Step 6: Replace `bg_shadows` with registry lookups**

In `crates/map-compile/src/main.rs`, delete the `slugify` closure and the `bg_shadows` construction, build the registry instead, and pass the registry's declared supersessions into `bridge_filtered` in place of the slug map. Adaptation note: `bridge_filtered`'s `&bg_shadows` parameter is a `&BTreeMap<String, Vec<(i32, i32)>>` — keep that shape, and construct it from the registry's declared unifications plus the polity eras, so `partition_bridge.rs` does not have to change in this task. The report line becomes:
```rust
    report_md.push_str(&format!(
        "- Background: {} entity IDS shadowed by atlas Territory DURING its eras, each by a written registry declaration (never by slug)\n",
        shadows.len()));
```

- [ ] **Step 7: Persist the registry across the compile→serve boundary**

The provider needs `resolve` at request time. Extend `crates/map-canon/src/persist.rs` to write and read the registry alongside the store, and add `pub fn registry(&self) -> &Registry` to `CanonStore`. Write the round-trip test first:
```rust
#[test]
fn a_persisted_registry_round_trips_including_its_reasons() {
    let mut store = fixture_store();
    // ... populate a registry with one declared and one SameId unification ...
    let bytes = persist::write(&store).unwrap();
    let back = persist::read(&bytes).unwrap();
    assert_eq!(back.registry(), store.registry(), "reasons and witnesses survive the boundary");
}
```

- [ ] **Step 8: Recompile, restart, and judge with all four instruments**

```powershell
cargo build --release
cargo run --release -p map-compile
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
```bash
cargo test --workspace 2>&1 | grep -c "test result: ok"
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
for Y in -1405 -1050 59; do
  curl -s "http://127.0.0.1:8090/api/census?year=$Y" -o census-after-$Y.json
  python -c "
import json,sys
a=json.load(open('census-before-$Y.json')); b=json.load(open('census-after-$Y.json'))
ka={(r['layer'],r['entity']) for r in a}; kb={(r['layer'],r['entity']) for r in b}
print('$Y  gone:',sorted(ka-kb)); print('$Y  new :',sorted(kb-ka))"
done
```
The census delta here is **Stage 1's diff, and it is expected to be non-empty** — it IS the identity unification. Every gone/new pair must be explainable by a row in `registry.json`. `ALL GOLDEN VIEWS HOLD` is not optional.

- [ ] **Step 9: Commit**

```bash
git add crates/map-compile crates/map-canon data/authored/registry.json
git commit -m "slug-matching supersession dies: the registry declares every unification, in writing"
```

---

### Task 15: `entities` and `node` on the wire

Spec §3's fact tier: `entities(filter) → [Entity]` and `node(id) → NodeCard`.

**Files:**
- Modify: `crates/map-viewer/src/lib.rs`
- Create: `contracts/map-api/fact/entities.feature`
- Create: `contracts/map-api/fact/node.feature`
- Create (blessed): `contracts/map-api/fixtures/entities-all.json`, `entities-kind-polity.json`, `node-phoenicia.json`, `node-unknown.json`

**Interfaces:**
- Consumes: `store.registry()` (Task 14).
- Produces:
  - `GET /api/entities` → `[{"id","kind","names":[…],"witnesses":[{"mintedAs","witness","layer","kind"}]}]`, sorted by `id`.
  - `GET /api/entities?kind=polity` → the same, filtered.
  - `GET /api/node?id=<minted-or-canonical>` → `{"id","kind","names","witnesses","resolvedFrom","why","edgeSummary":{…}}` where `resolvedFrom` is the id as asked and `why` is the unification reason (or `null` for `SameId`).
  - An unknown id is **200 with a resolved-to-itself card carrying an empty witness list**, not a 404: `resolve` is total, and a client asking about an id nobody minted deserves the same shape as any other answer. This is a deliberate design call — record it in the feature's preamble so it is reviewed, not discovered.

- [ ] **Step 1: Write the features first (runner red)**

`contracts/map-api/fact/entities.feature`:
```gherkin
Feature: entities — one identity per real thing, whole
  The registry is the single source of identity (spec §2 equation 1): no
  witness mints an entity, every witness references the registry, and a
  unification is a WRITTEN act recorded in data/authored/registry.json —
  never a slug coincidence. This feature pins the whole table, so a new
  entity, a lost witness, or a silent re-minting all show up as a diff.

  Vocabulary:
    | entity kind | any of: allotment, district, people, place, polity, promise, route, terrain, vision, waterbody |

  Scenario: the whole registry, whole
    When I GET /api/entities
    Then the response equals fixture "entities-all"

  Scenario: filtered to polities, whole
    When I GET /api/entities?kind=polity
    Then the response equals fixture "entities-kind-polity"

  Scenario: every entity in the filtered answer is in the whole answer
    When I GET /api/entities as everything
    And I GET /api/entities?kind=polity as polities
    Then every entity in polities appears identically in everything
```
`contracts/map-api/fact/node.feature`:
```gherkin
Feature: node — one entity's card, and how it was reached
  Resolution is TOTAL: every minted id resolves, and an id nobody minted
  resolves to itself with an empty witness list rather than a 404 — a
  client asking about an unknown id gets the same shape as any other
  answer. Recorded here because it is a design decision, not an accident.

  Scenario: a twice-witnessed entity, reached by its canonical id
    When I GET /api/node?id=phoenicia
    Then the response equals fixture "node-phoenicia"

  Scenario: the same entity, reached by a minted alias — same card, different reachedBy
    When I GET /api/node?id=phoenicia as canonical
    And I GET /api/node?id=partition:phoenicia as aliased
    Then aliased equals canonical except for resolvedFrom and why

  Scenario: an id nobody minted, whole
    When I GET /api/node?id=no-such-entity
    Then the response equals fixture "node-unknown"
```

```bash
cd contracts/runner && cabal run contract-runner -- check ../map-api
```
The orphan list is the step work: `every entity in {name} appears identically in {name}`, and `{name} equals {name} except for resolvedFrom and why`.

- [ ] **Step 2: Write the failing Rust test for the two route bodies**

In `crates/map-viewer/src/lib.rs`'s test module, assert the JSON shape of both routes against a small hand-built registry, including the negative cases: an unknown `kind=` filter yields `[]` (not every entity); a `node` card for an unknown id has an empty `witnesses` array and `resolvedFrom` equal to the id asked.

- [ ] **Step 3: Run and verify FAIL.**

- [ ] **Step 4: Implement the two routes**

In `route_text`'s match, next to `/api/census`. Both are read-only, both serialize through one shared `fn entity_json(e: &Entity) -> serde_json::Value` so the two routes can never disagree about an entity's shape. Sort `entities` by `id` and `witnesses` by `(witness, layer, minted_as)` — determinism is a law here (`census` already does this; follow it).

- [ ] **Step 5: Implement the two new steps in `Steps.hs`**

Both need negative-case tests in `Spec.hs` before the implementation, per the standing rule. `except for resolvedFrom and why` is a masked whole-body comparison — reuse the masking machinery Task 10 of Stage 0 built for `contract.feature`, do not write a second one.

- [ ] **Step 6: Rebuild, restart detached, bless, eyeball**

Bless the four fixtures, then read them. `entities-all.json` must contain the Phoenicia entity with **two** witnesses. `node-unknown.json` must have an empty witness list and no invented name. If `entities-all.json` is `[]`, the registry did not survive persistence (Task 14 Step 7) — stop and fix that, do not bless.

- [ ] **Step 7: Full gates, commit**

```bash
make contract-gates
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
git add crates/map-viewer contracts/
git commit -m "entities and node on the wire: the registry, queryable"
```

---

### Task 16: `edge_summary` and `edges` — Explorable on the wire, and the upstreamability law

Spec §2: our nodes must be walkable by the atlas exploration UI *"with zero new client machinery"*, using the same `edge_summary` + paged `edges` shape its `PositionRef` uses. And the upstreamability law — *"every map node and relation type must be expressible as an ADDITIVE extension of atlas graph-types — new variants, never reshaped existing ones"* — becomes a contract-tested design constraint. **The atlas repo is read-only, so the law is enforced on our side**: our relation labels are either atlas labels verbatim or new labels disjoint from every atlas label, and the check reads `atlas_graph_types`' own `RelationId::ALL` rather than a copied list.

The atlas shapes (read, not guessed, from `graph-types/src/explore.rs`): `EdgeQuery { kind, cursor: Option<usize>, limit }`, `EdgeEntry { edge, node, meta }`, `EdgePage { kind, entries, next: Option<usize> }`, `EdgeSummary = BTreeMap<EdgeKind, usize>`.

**Stage 1's relations, and only those the data supports:**

| label | inverse | what it relates |
|---|---|---|
| `witnessed-by` | `witnesses` | a canonical entity → each `WitnessRef` |
| `supersedes` | `superseded-by` | a canonical entity → each minted id unified into it |

No `stands-during` (Stage 2), no `bounded-by` / `extends-over` (Stage 3). A relation with no data behind it is a shape-only promise, and spec §4 forbids those.

**Files:**
- Create: `crates/map-canon/src/explore.rs`
- Modify: `crates/map-canon/src/lib.rs`, `crates/map-canon/src/tests.rs`
- Modify: `crates/map-viewer/src/lib.rs`
- Create: `contracts/map-api/fact/edges.feature`
- Create (blessed): `contracts/map-api/fixtures/edge-summary-phoenicia.json`, `edges-phoenicia-witnessed-by.json`, `edges-page-1.json`, `edges-page-2.json`

**Interfaces:**
- Consumes: `Registry` (Task 13), `atlas_graph_types::edge::RelationId` (read-only).
- Produces:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MapRelation { WitnessedBy, Supersedes }
impl MapRelation {
    pub const ALL: [MapRelation; 2];
    pub fn forward_label(self) -> &'static str;   // "witnessed-by" | "supersedes"
    pub fn inverse_label(self) -> &'static str;   // "witnesses"    | "superseded-by"
    pub fn parse(label: &str) -> Option<(MapRelation, bool)>; // (relation, is_forward)
}
pub fn edge_summary(reg: &Registry, id: &EntityId) -> BTreeMap<&'static str, usize>;
pub struct EdgePage { pub relation: String, pub entries: Vec<EdgeEntry>, pub next: Option<usize> }
pub struct EdgeEntry { pub node: String, pub meta: Option<String> }
pub fn edges(reg: &Registry, id: &EntityId, relation: &str, cursor: usize, limit: usize) -> Result<EdgePage, String>;
```
Wire: `GET /api/edge_summary?id=X` and `GET /api/edges?id=X&relation=L&cursor=N&limit=M`.

- [ ] **Step 1: Write the paging laws as scenarios first**

`contracts/map-api/fact/edges.feature`:
```gherkin
Feature: edges — walking the graph, in the atlas's own shape
  Our nodes are graph positions with typed relations, answered in
  exactly the shape the atlas's Explorable trait uses (edge_summary +
  paged edges), so the atlas exploration UI walks them with zero new
  client machinery (spec §2). Stage 1 carries two relations —
  witnessed-by/witnesses and supersedes/superseded-by — because those
  are the two the registry's data supports. stands-during arrives with
  the ledger; bounded-by with the arrangement. A relation with no data
  behind it would be a shape-only promise, which this contract forbids.

  Vocabulary:
    | relation | any of: supersedes, superseded-by, witnessed-by, witnesses |

  Scenario: an entity's edge summary, whole
    When I GET /api/edge_summary?id=phoenicia
    Then the response equals fixture "edge-summary-phoenicia"

  Scenario: one relation's whole page
    When I GET /api/edges?id=phoenicia&relation=witnessed-by&cursor=0&limit=100
    Then the response equals fixture "edges-phoenicia-witnessed-by"

  Scenario: paging is a partition — the pages concatenate to the whole, with no gap and no repeat
    When I GET /api/edges?id=phoenicia&relation=witnessed-by&cursor=0&limit=100 as whole
    And I GET /api/edges?id=phoenicia&relation=witnessed-by&cursor=0&limit=1 as page1
    And I GET /api/edges?id=phoenicia&relation=witnessed-by&cursor=1&limit=1 as page2
    Then page1 then page2 concatenate to whole

  Scenario: the last page says so, and only the last page says so
    When I GET /api/edges?id=phoenicia&relation=witnessed-by&cursor=0&limit=1 as page1
    And I GET /api/edges?id=phoenicia&relation=witnessed-by&cursor=1&limit=1 as page2
    Then page1 has a next cursor and page2 does not

  Scenario: the summary's counts are the pages' lengths
    When I GET /api/edge_summary?id=phoenicia as summary
    And I GET /api/edges?id=phoenicia&relation=witnessed-by&cursor=0&limit=1000 as all
    Then summary's witnessed-by count equals the number of entries in all

  Scenario: an unknown relation is refused by name, never answered empty
    When I GET /api/edges?id=phoenicia&relation=invented-by&cursor=0&limit=10
    Then the response is an error naming "invented-by"
```
The last scenario matters: an unknown relation answered as an empty page is a check satisfiable by the failure mode — a client would read "no witnesses" where the truth is "you asked a question this server does not understand".

- [ ] **Step 2: Write the failing Rust tests, including the upstreamability law**

```rust
#[test]
fn our_relations_are_an_additive_extension_of_the_atlas_vocabulary() {
    use atlas_graph_types::edge::RelationId;
    use crate::explore::MapRelation;

    // Every atlas label, read from the atlas's own type — never a
    // copied list, which would go stale silently.
    let atlas: BTreeSet<&'static str> = RelationId::ALL.iter()
        .flat_map(|r| [r.forward_label(), r.inverse_label()])
        .collect();
    let ours: BTreeSet<&'static str> = MapRelation::ALL.iter()
        .flat_map(|r| [r.forward_label(), r.inverse_label()])
        .collect();

    // ADDITIVE: we add labels, we never redefine one of theirs.
    assert!(ours.is_disjoint(&atlas),
        "these labels collide with the atlas vocabulary: {:?}", ours.intersection(&atlas).collect::<Vec<_>>());
    // And the check has teeth: the atlas vocabulary is non-empty, so
    // disjointness is a real constraint and not a statement about nothing.
    assert!(atlas.len() >= 20, "read the atlas's own relation table, not an empty set");
    // Round-trip: every label we emit, we can parse back.
    for r in MapRelation::ALL {
        assert_eq!(MapRelation::parse(r.forward_label()), Some((r, true)));
        assert_eq!(MapRelation::parse(r.inverse_label()), Some((r, false)));
    }
    assert_eq!(MapRelation::parse("contains"), None, "an atlas label is not ours to answer");
}

#[test]
fn paging_partitions_the_edge_list_at_every_limit() {
    // For limits 1..=n+1 over an entity with n edges: concatenating
    // every page in cursor order yields the whole list exactly once,
    // and `next` is Some on every page but the last.
    // ... build a registry with 5 witnesses, loop over limits ...
}
```

- [ ] **Step 3: Run and verify FAIL.**

- [ ] **Step 4: Implement `crates/map-canon/src/explore.rs`**

`edges` returns `Err` naming the label for an unknown relation. `next` is `Some(cursor + entries.len())` when more remain and `None` otherwise — computed from the underlying length, never from `entries.len() == limit` (a full last page would then lie).

- [ ] **Step 5: Implement the two routes and the four new steps**

Route bodies are thin; the laws live in `explore.rs` and are already tested. The four runner steps (`concatenate to`, `has a next cursor and … does not`, `count equals the number of entries in`, `is an error naming`) each get a positive AND a negative unit test in `Spec.hs` first. The error step needs `transport` to surface non-2xx as a `Left` — Stage 0 already made that true; confirm rather than re-implement.

- [ ] **Step 6: Rebuild, restart, bless, eyeball, gate, commit**

```bash
make contract-gates
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
git add crates/map-canon crates/map-viewer contracts/
git commit -m "edge_summary and edges: Explorable on the wire, additive over the atlas vocabulary"
```

---

### Task 17: Close the stage — v0.2.0, the census diff reviewed name by name, and the atlas hand-off

**Files:**
- Modify: `contracts/VERSION`, `contracts/CHANGELOG.md`
- Modify: `contracts/map-api/scene/scene.feature` (final preamble pass)

- [ ] **Step 1: The full gate sweep, all four strata, recorded**

```bash
make contract-gates
cd contracts/runner
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api    | tee ../../map-api.out
cabal run contract-runner -- run --base-url http://127.0.0.1:8080 ../atlas-edge | tee ../../atlas-edge.out
cd /c/Users/donov/.claude/jobs/c6946bce/tmp && node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
```
Expected: map-api all green except the one declared `@target` (derivability); atlas-edge 6/6; `cargo test --workspace` 19 ok; `ALL GOLDEN VIEWS HOLD`. Anything else stops the stage.

- [ ] **Step 2: THE CENSUS DIFF REVIEW — the owner's gate**

Spec §6 stratum 3: *"stage 1's diff IS the identity unification (merged rows, reviewed name by name)."* Produce it with the instrument built in Task 7, at the three sampled instants and at the two years the golden gate loves most:
```bash
for Y in -1446 -1405 -1050 -586 59; do
  echo "=== $Y"
  curl -s "http://127.0.0.1:8090/api/census?year=$Y" | python -c "
import json,sys,collections
rows=json.load(sys.stdin); byname=collections.defaultdict(list)
for r in rows: byname[r['name']].append(r['entity']+'@'+r['layer'])
print(' rows',len(rows))
for k,v in sorted(byname.items()):
  if len(v)>1: print('  STILL DOUBLED:',k,v)"
done
```
Then, against the `census-before-*.json` captured in Task 14 Step 1, produce the full gone/new list and take it to the owner **name by name**. Every merged row must be explainable by a `data/authored/registry.json` row and its written reason. A row nobody can explain is a bug, not a merge.

> **STOP. OWNER GATE.** The diff is reviewed and approved before Step 3. Record the approval in the commit message.

- [ ] **Step 3: Move the version and write the changelog entry**

`contracts/VERSION`:
```text
0.2.0
```
and prepend to `contracts/CHANGELOG.md` under `## Unreleased` → a new `## 0.2.0` section stating, in the owner's own terms, what changed: the entity registry and the four fact-tier routes; pieces made first-class and the two `@target` reds retired; the `pieces=` wire parameter replacing the four-flag bridge; `piece` on every manifest entry; `markers` → `markers:<piece>` as a wire change; every scene fixture re-blessed; the census diff instrument; CI and the enforced semver rule; shrinking and the hole-distinctness law. **The bump is the owner's declaration of what changed — write it as a declaration, not a commit log.**

- [ ] **Step 4: Hand the atlas-edge suite to the atlas session**

Diagnosis §8.3: it is green now, and handing it over costs nothing while buying provider-side protection through Stages 1–3's long internal churn. Build the standalone binary and report its path plus the one command that runs it:
```bash
cd contracts/runner && cabal build && cabal list-bin contract-runner
# the atlas session runs, with no access to this workspace and no Rust toolchain:
#   contract-runner run --base-url http://127.0.0.1:8080 <path-to>/contracts/atlas-edge
```
Report to the owner: the binary path, the `contracts/atlas-edge/` directory (6 features + 6 fixtures), the fact that all six pass today, and the request that the atlas session adopt it in their CI. **Do not edit the suite. Do not touch the atlas repo.**

- [ ] **Step 5: Re-run the semver gate against itself**

```bash
bash scripts/contract-semver-gate.sh origin/master
```
Expected: exit 0 — this stage edited many features and fixtures, and both `contracts/VERSION` and `contracts/CHANGELOG.md` moved. If it exits 1, the gate is right and Step 3 is incomplete.

- [ ] **Step 6: Commit**

```bash
git add contracts/VERSION contracts/CHANGELOG.md contracts/
git commit -m "Stage 1 closes at v0.2.0: the registry, pieces first-class, census diff reviewed"
```

- [ ] **Step 7: Report to the owner and STOP**

Report: the four strata's numbers; the census diff as approved; the two retired `@target`s with the evidence (a `points` resource per piece, `piece` on every entry); the atlas hand-off; and the one remaining declared `@target` (derivability, waiting on Stages 2–3). **Do not begin Stage 2.** It gets its own planning pass against this stage's evidence, with the owner in the loop — the same discipline that made this plan possible.

---

## What Stage 1 produces that Stage 2 consumes

Stage 2 is "the ledger": `Stands`/`Holds`/`Grade` literals and `stands_until` strings become ledger rows, adding `standings`, `disposition` and `laws`, and **its census diff must be EMPTY**. That last requirement is the binding one: Stage 2 cannot move an identity, so everything below must be stable and queryable before it starts.

**Identity.**
- `map_canon::registry::Registry`, persisted with the canon and reachable as `store.registry()`.
- `Registry::resolve(&EntityId) -> &EntityId` — **total** (an unknown id resolves to itself), **idempotent** (`resolve(resolve(x)) == resolve(x)`), and **one-hop** (chains are a `RegistryViolation`, refused at compile). Stage 2's ledger rows key on `resolve`'d ids and may rely on all three.
- The canonical `EntityId` set is exactly `Registry::entities()`. Every `Feature::entity()` in the canon resolves into it.
- `Registry::why(&EntityId) -> Option<&Unification>` — the written reason a minted id was unified. Stage 2's law citations may cite it.
- `data/authored/registry.json` is the owner-reviewable declaration file. Stage 2 adds no rows to it; a Stage 2 change that needs one is an identity change and belongs in a Stage 1 amendment with its own census diff review.

**The wire, for Stage 2 to extend additively.**
- `GET /api/entities[?kind=…]`, `GET /api/node?id=…`, `GET /api/edge_summary?id=…`, `GET /api/edges?id=…&relation=…&cursor=…&limit=…`.
- `map_canon::explore::MapRelation` with `forward_label`/`inverse_label`/`parse`, and the law that our labels are disjoint from `atlas_graph_types::edge::RelationId`'s. Stage 2 adds `stands-during`/`stood-by` by adding a variant here; the disjointness law then covers it automatically.
- `GET /api/census?year=A&to=B` — the diff instrument. **This is the instrument that judges Stage 2**, and Stage 2 must not modify it.

**Pieces, for Stage 2 to leave alone.**
- `map_types::{Piece, PieceSet}`; `piece` on every scene element and every manifest entry; `Snapshot::restrict(PieceSet)`; `?pieces=` on `/api/scene`. Stage 2 touches none of it, and the strengthened scene laws will say so if it does.

**Instruments, inherited.**
- `forAllShrink` over `@property` holes with per-hole shrinkers in `Prop.holeRegistry` — Stage 2 registers a hole by adding a `SomeHole gen shrink` row.
- The hole-distinctness law in `contract-runner check`: every Stage 2 property scenario is automatically checked for holes that cannot vary.
- `make ci` / `make contract-gates`, the semver gate, and `contracts/CHANGELOG.md`. Stage 2 opens at `0.2.0` and bumps to `0.3.0` as its own closing declaration.

**Explicitly NOT produced here, so Stage 2 does not assume it:** `standings`, `disposition`, `laws`, `borders`, a `⊕` combine endpoint, and the `Witness` node's `evidence`/`interval` fields. `WitnessRef` carries only what the canon actually holds today — `minted_as`, `witness`, `layer`, `kind`. Stage 2 adds the interval; do not fake it now.

---

## Self-Review (performed at write time)

**1. Spec coverage.** §5 Stage 1's four named deliverables: one registry (Task 13), all identities resolve through it (13–14), the duplicated entities become one node with multiple witnesses (14, reviewed in 17), `entities`/`node`/`edge_summary`/`edges` on the wire (15–16), slug-matching supersession dies (14). ✓ §2's Entity node and Explorable/upstreamability: Tasks 13 and 16, with the additive-extension law as a Rust test reading the atlas's own table. ✓ §3's algebra: laws 1 (omission-totality) and 2 (composition) become true in Tasks 8–12; law 4 (default-totality) gets its first scenario in Task 7; law 3 was already green and is not disturbed. ✓ §4's semver enforcement, changelog, and `forAllShrink`: Tasks 6 and 4. ✓ §6's four strata every stage: Rust tests in every task, the contract suite in 7/12/15/16/17, the census diff in 7 (built) and 17 (reviewed), the golden gate in 11/14/15/16/17. ✓ §9's four uncovered obligations: default-totality (7), derivability (7, `@target`), the census diff law (7), CI (6). ✓ §9.2's three smaller gaps: shrinking (4), the vocabulary exemption made explicit (7 step 7), preamble drift addressed by a standing constraint and by Task 11 step 4.

**Gaps I am leaving open, deliberately and with the reason:** the spec's true `⊕` (§3 law 2's operator form) still is not tested, because there is no server-side combine and diagnosis §8.5 argues explicitly against building one now. Feature preambles remain unverified prose — the standing constraint makes it a reviewer's job, not a law, and inventing a prose-checking law is not something this stage should attempt.

**2. Placeholder scan.** No "TBD", no "add error handling", no "similar to Task N". Six places say "adapt to what the file already does" and each names the exact file, the exact thing to read, and what to do if it differs — those are honest instructions to read code that exists, not deferred decisions. Three tasks contain a `…` inside a test body (Task 10's second test's scene construction, Task 16's paging loop, Task 14's persistence test): each is immediately preceded by the constructors to mirror, and expanding them here would be transcription rather than instruction. Two tasks carry explicit STOP-AND-ASK owner gates (6, 14, 17 step 2) — those are decisions this plan must not make, and naming them as gates is the point.

**3. Type consistency.** `Piece`/`PieceSet` are the Rust names throughout Tasks 8–12 (`Piece::ALL`, `PieceSet::{empty,all,with,without,contains,union,iter,parse,render,bits}`); the Haskell `Capture.Piece`/`PieceSet` keep their existing Stage 0 names and are unchanged except for the `Arbitrary` instances in Task 2. `piece` is the field name on all four scene element types (Task 9), all three manifest resource types and all three JSON arrays (Task 10). `census_diff`/`CensusChange` match between Task 7's test, implementation, route and Task 17's review. `Registry::{declare,observe,resolve,get,entities,why,validate}` match between Tasks 13, 14, 15 and 16 and the closing Interfaces section. `MapRelation::{ALL,forward_label,inverse_label,parse}` match between Task 16's test, implementation and the closing section. `SomeHole` gains its second field in Task 4 and every later reference (`renderHole`, `substituteExamples`, `shrinkHole`, `holeDefectsWith`) uses the two-field pattern. `sceneUrlWith` is introduced in Task 7 and re-bodied in Task 11 — same name, same signature, both stated.

**One inconsistency I found and fixed inline:** Task 7's first draft added `Then <name> equals fixture <fixture>` and Task 15 assumed a masked variant of it; Task 15 step 5 now says explicitly to reuse Stage 0's masking machinery rather than write a second one.

**One correction to this plan's own brief, recorded so it is not rediscovered:** the carried-forward debt "a real diff in `blessOrCompare`'s failure message" is **already paid** — `firstDiff` landed in `7cd31bd` and is wired into the plain comparison path. Task 3 is therefore smaller than the brief implies: two remaining paths (masked, and consumed-projection) still lack it. The diagnosis does not claim otherwise; the brief's summary of it does.
