# Stage 4: The Dogfood Bar — the Legacy Routes Deleted, and `TopographyDress` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land spec §5's Stage 4 — the viewer consumes only contract functions, every legacy route is deleted, and `TopographyDress` lands as dress-locality's show-piece — then put spec §6's dogfood-bar checkables in front of the owner as the evidence for a v1.0 declaration this plan does not make.

**Architecture:** A strangler, finished. Each legacy route's *consumer* moves onto a contract function first, and only then does the handler die; the surviving surface is pinned as a whole-body law in two directions — the routes the server serves, and the URLs the client fetches — so "the viewer consumes only contract functions" is machine-checked rather than asserted. `TopographyDress` then replaces the borrowed two-stop `AgeRamp` with a first-class multi-stop dress carrying its own laws, and default-totality (spec §3 law 4) becomes real in the template loader, so a style may dress any subset of pieces. The whole stage is judged by the golden gate, which this plan repairs before it leans on it.

**Tech Stack:** Rust (map-types, map-provider, map-viewer, map-encoders), RON style templates, the embedded browser client (`crates/map-viewer/src/page.html`, `limb.js`), the Haskell contract runner (`contracts/runner`), Node `--test` for the client-side laws, Playwright-core for the golden gate.

**Spec:** `docs/superpowers/specs/2026-09-06-map-api-contract-design.md`
**Diagnosis (Stage 0's evidence):** `docs/notes/2026-09-06-contract-diagnosis.md` — read the correction notice and §7.0 first.
**Consumed plans:** `docs/superpowers/plans/2026-09-07-contract-stage1.md` (exists at write time). **Stage 2 and Stage 3 plans did not exist when this was written** — sibling agents were writing them concurrently. Every dependency this plan takes on Stages 2 and 3 is stated against spec §5's description of them and is flagged inline as `[STAGE-2 DEP]` / `[STAGE-3 DEP]` with the fallback to take if the delivered interface differs. Reconcile those flags against the real plans before starting Task 4.

---

## Global Constraints

- **TDD is mandatory.** Every task writes its failing test first, runs it, sees it fail *for the stated reason*, then implements. No exceptions.
- **Owner's standing law:** first-class composable types with laws. Never styling tricks, never special cases, never tuned constants. If a fix needs a magic number, the number is wrong or it belongs in declared data. `TopographyDress` is this law's exhibit in this stage: it is a type with laws, not a knob.
- **Whole-body assertions.** A scenario pins the ENTIRE answer, with don't-cares masked explicitly in the scenario text. Existential poke-assertions ("some entry equals…", "is an array", "has at least N") are forbidden. This stage extends the decree to two new bodies: the *served route surface* and the *client's fetch surface* — both pinned entire.
- **A check satisfiable by the failure mode is not a check.** This applies to inputs as much as assertions (diagnosis §7.0). **Task 1 exists because the golden gate currently contains exactly this defect** — a changed stop set is reported as `NEW STOP … bless to adopt` and skipped, so the gate can print `ALL GOLDEN VIEWS HOLD` while checking nothing. Stage 4 is the stage most likely to move the stop list. Repair the instrument before using it.
- **Order within every task:** contract additions first (runner red) → Rust tests (cargo red) → implement (green) → golden gate holds → census diff reviewed → commit.
- **The suite stays pre-release.** `contracts/VERSION` moves `0.4.0` → `0.5.0` in Task 15. **This plan must not bump to 1.0.** v1.0 is the owner's declaration, a human act, never automatic (spec §4, §7). Task 15's whole job is to hand the owner the evidence and stop.
- **The golden gate is the hard judge.** `node crates/map-viewer/tests/golden.js --check` must print `ALL GOLDEN VIEWS HOLD` (89/89 stops). Run it from `C:\Users\donov\.claude\jobs\c6946bce\tmp` so `playwright-core` resolves (MEMORY: browser-verify-workaround). **Deleting routes and rewiring the client is the most pixel-dangerous work in the whole migration.** Any drifted probe STOPS the task. Re-blessing requires the owner's explicit approval, per change, in writing.
- **Ports.** The workbench viewer is **8090**. `8080` (atlas API), `8081`, `8000`, `5000` belong to the atlas pipeline and must never be bound (MEMORY: port-reservations).
- **The atlas repo is a READ-ONLY path dependency.** Never edit it (MEMORY: never-edit-atlas-repo). Spec §7 keeps atlas-side implementation and the upstream PRs out of scope; this plan crosses neither line.
- **The atlas-edge CDC suite was handed to the atlas session in Stage 1** (Stage 1 Task 17 Step 4, on diagnosis §8.3's recommendation). Stage 4 does **not** re-do the hand-off. Task 14 re-verifies it green against `:8080` and confirms the hand-off happened; if Stage 1's report shows it did not, Task 14 performs it. **Never change the atlas-edge suite's meaning without the atlas session.**
- **The render pipeline is three steps** (MEMORY: render-pipeline-three-steps): after a Rust change, `cargo build --release`, stop the old viewer, relaunch it DETACHED (session background tasks get killed — MEMORY: demo-launch-detached):
  ```powershell
  Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
  cargo build --release
  Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
  ```
  A change to `crates/map-compile` additionally requires `cargo run --release -p map-compile` before the viewer restart, or the canon is stale. **No task in this plan changes `map-compile`.**
- **Haskell commands run from `contracts/runner/`** unless stated otherwise.
- **Feature preambles are unverified prose** (diagnosis §9.2). Any task that changes what a feature *means* updates its preamble in the same commit; the reviewer checks the prose against the scenarios.
- **Owner speaks plain language** (MEMORY): when a step calls for an owner gate, state the question in concrete terms — what will look different, what will stop working — never in GIS or algebra jargon.

---

## The legacy-route census: what actually exists, and what the spec's "13" means

Counted by hand at `crates/map-viewer/src/lib.rs` as of commit `e05ea3d`. **There are 15 `/api/*` routes today, not 13.** Two of them (`/api/contract` at `lib.rs:886`, `/api/census` at `lib.rs:901`) were added by Stage 0 (commit `d758215`, "the census and the contract declaration: two read-only routes for v0.1"). The spec was written at `a81fbf3`, one commit earlier — **so the spec's "13" is exactly the `/api/*` surface as it stood the day the spec was written, and it is correct for that day.**

Verified, not inferred:
```
$ git show a81fbf3:crates/map-viewer/src/lib.rs | grep -o '"/api/[a-z_]*"' | sort -u
"/api/changes" "/api/entities" "/api/features" "/api/meta" "/api/overlay"
"/api/region_times" "/api/render" "/api/resource" "/api/resources"
"/api/scaffold" "/api/scene" "/api/subjects" "/api/transition"
                                                          -> 13
```

The spec's §6 checkable says "13 legacy routes deleted". Read literally that is impossible and would be wrong: three of those 13 (`/api/scene`, `/api/changes`, `/api/subjects`) *are* contract functions that spec §3 and §5 Stage 0 require the server to keep serving. The honest reading, which this plan adopts and which Task 15 puts to the owner explicitly, is: **the 13-route legacy surface is retired — each route either becomes a named contract function with laws, or is deleted. Six are deleted; nine survive under contract names.**

| # | route | `lib.rs` | what it serves today | client fetches it? | fate in Stage 4 |
|---|---|---|---|---|---|
| 1 | `/api/resource` | 833 | one content-addressed geometry payload | no (only the batch) | **KEEP** — `resources()`, singular form |
| 2 | `/api/resources` | 818 | batched payloads, one stream | yes — `page.html:748` | **KEEP** — `resources(ids)` |
| 3 | `/api/meta` | 867 | scrub stops, style list, encoder list, projection list, chronology anchor | yes — `page.html:2246` | **DELETE** (Task 4) → stops from `changes()`, styles from `styles()`, anchor from `laws()`; encoders/projections die with `/api/render` |
| 4 | `/api/contract` | 886 | version, laws-version, graph-pin | no | **KEEP** — `contract()` |
| 5 | `/api/census` | 901 | the disposition table at an instant (+ Stage 1's diff mode) | no | **KEEP** — `census()` |
| 6 | `/api/subjects` | 924 | what can be asked about at a year | yes — `page.html:2147` | **KEEP** — grandfathered as a contract function by spec §5 Stage 0 and `contracts/map-api/fact/subjects.feature` |
| 7 | `/api/changes` | 943 | change events between two instants | yes — `page.html:2181` | **KEEP** — `changes(from, to)` |
| 8 | `/api/transition` | 972 | the morph/fade plan | no | **KEEP** — `transition(from, to, pieces)` |
| 9 | `/api/scaffold` | 993 | land + water + relief as a served SVG picture, camera required | no | **DELETE** (Task 7) → `scene(pieces=ground,water)` |
| 10 | `/api/entities` | 1031 | entities standing at an instant | no | **KEEP** — `entities(filter)`; Stage 1 reshapes it |
| 11 | `/api/features` | 1053 | GeoJSON for a named set of entity ids | no | **DELETE** (Task 7) → `borders(entity, at)` `[STAGE-3 DEP]` |
| 12 | `/api/render` | 1087 | a served picture (SVG or GeoJSON) | yes — `page.html:327`, **inspection panel only** | **DELETE** (Task 8) → no contract function; the encoders live on in `map-cli` |
| 13 | `/api/scene` | 1114 | the retained-scene manifest | yes — `page.html:832, 939` | **KEEP** — `scene(pieces, at\|over, style, camera?)` |
| 14 | `/api/region_times` | 1165 | which scrub stops carry a mapping for one subject | yes — `page.html:2059` | **DELETE** (Task 5) → `standings(over)` `[STAGE-2 DEP]` |
| 15 | `/api/overlay` | 1179 | two subject-sides composed at the semantic level, served as a picture | yes — `page.html:2328` | **DELETE** (Task 6) → `scene(…, over=…)` + a client-side `⊕` |

Plus two non-API paths that are the application shell, not an API surface, and stay: `/` (`lib.rs:860`, the page) and `/limb.js` (`lib.rs:865`, the shared containment/clip module the Node cross-language test also loads).

**One more legacy thing, not a route but the same disease.** `composed_scene` at `lib.rs:703` reads a query parameter *also named* `pieces=` whose values are **entity ids**, not scene pieces — the "composable API" from commit `c72ea95`, sibling to `/api/features` and `/api/scaffold`, consumed by no client. **This collides by name with the `pieces=` PieceSet parameter Stage 1 Task 11 adds to `build_query`**, and because `composed_scene` checks it *first* and short-circuits into `canon.render_pieces`, the entity-id branch shadows the PieceSet parameter on `/api/scene`. Task 7 deletes the entity-id branch. If Stage 1 did not already resolve the collision, its `?pieces=` parameter is not reaching `/api/scene` at all and Task 7 is where that is discovered — check it there first, before anything else in this plan is believed.

---

## Part A — sharpen what will judge the demolition

### Task 1: The golden gate must FAIL when the stop set changes

`crates/map-viewer/tests/golden.js:84-85` reads:

```js
const w = (want.cams[name] || {})[stops[i]];
if (!w) { console.log(`NEW STOP ${stops[i]} (${name}) — bless to adopt`); continue; }
```

A stop present in the run but absent from the fixture is *skipped*. A stop present in the fixture but absent from the run is *never looked at* — the loop walks the run's stops, not the fixture's. So if the stop list changes, the gate compares fewer probes, or none, and still prints `ALL GOLDEN VIEWS HOLD`. This is diagnosis §7.0's defect in the one instrument the whole stage rests on, and Task 4 rebuilds the stop list from a different source, which is precisely the change this hole would hide.

**Files:**
- Modify: `crates/map-viewer/tests/golden.js`
- Create: `crates/map-viewer/tests/golden.test.mjs`

**Interfaces:**
- Consumes: nothing.
- Produces: `require('./golden.js').stopSetComplaints(blessedStops, runStops) -> string[]` — empty iff the two lists are equal as ordered sequences. `golden.js --check` exits non-zero when it is non-empty. Tasks 4–15 all rely on this.

- [ ] **Step 1: Write the failing test**

Create `crates/map-viewer/tests/golden.test.mjs`:

```js
// The gate's own gate. golden.js is the instrument that judges every
// visual change in Stage 4; diagnosis §7.0 is the standing warning that
// an instrument reporting green over ground it never examined is worse
// than no instrument. These cases are the negative ones: each is a
// stop-set change that the gate USED to wave through.
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { stopSetComplaints } = require("../tests/golden.js");

const BLESSED = [-4100, -4000, -3000, -1405, 59];

test("an identical stop set is silent", () => {
  assert.deepEqual(stopSetComplaints(BLESSED, [...BLESSED]), []);
});

test("a stop that appeared is a complaint, named", () => {
  const c = stopSetComplaints(BLESSED, [-4100, -4000, -3500, -3000, -1405, 59]);
  assert.equal(c.length, 1);
  assert.match(c[0], /-3500/);
  assert.match(c[0], /not blessed/);
});

test("a stop that vanished is a complaint, named", () => {
  const c = stopSetComplaints(BLESSED, [-4100, -4000, -3000, 59]);
  assert.equal(c.length, 1);
  assert.match(c[0], /-1405/);
  assert.match(c[0], /blessed but not/);
});

test("a reordered stop set is a complaint — the scrubber's order IS the timeline", () => {
  const c = stopSetComplaints(BLESSED, [-4000, -4100, -3000, -1405, 59]);
  assert.notEqual(c.length, 0);
});

test("an empty run against a blessed set complains about every stop, not none", () => {
  // The failure mode that matters most: a rewiring that leaves the page
  // with no stops at all must not read as 'nothing drifted'.
  assert.equal(stopSetComplaints(BLESSED, []).length, BLESSED.length);
});
```

- [ ] **Step 2: Run and verify FAIL**

Run: `node --test crates/map-viewer/tests/golden.test.mjs`
Expected: FAIL — `stopSetComplaints is not a function` (golden.js exports nothing today).

- [ ] **Step 3: Implement in `golden.js`**

Insert immediately after the `const TOL = 24;` line (`golden.js:29`):

```js
// THE STOP SET IS PART OF THE BLESSING. A stop that appears or vanishes
// is a change to what the gate examines, and a gate that quietly
// examines less is not a gate (diagnosis §7.0). Compared as ORDERED
// sequences: the scrubber's order is the timeline, and a reordering
// would silently re-key every probe.
function stopSetComplaints(blessed, run) {
  const complaints = [];
  const b = blessed.map(Number), r = run.map(Number);
  const bs = new Set(b), rs = new Set(r);
  for (const y of r) if (!bs.has(y)) complaints.push(`STOP ${y} is in the run but not blessed`);
  for (const y of b) if (!rs.has(y)) complaints.push(`STOP ${y} is blessed but not in the run`);
  if (complaints.length === 0 && b.some((y, i) => y !== r[i])) {
    complaints.push(`STOP ORDER changed: blessed ${b.join(",")} ran ${r.join(",")}`);
  }
  return complaints;
}
module.exports = { stopSetComplaints };
```

Then wire it into `--check`. Replace the `const stops = await page.evaluate(() => state.stops);` line (`golden.js:51`) with:

```js
  const stops = await page.evaluate(() => state.stops);
  if (check) {
    const blessedStops = Object.keys(JSON.parse(fs.readFileSync(FIXTURE, 'utf8')).cams[CAMS[0][0]]).map(Number);
    const complaints = stopSetComplaints(blessedStops, stops);
    if (complaints.length) {
      complaints.forEach(c => console.log(c));
      console.log(`REGRESSION: the stop set changed (${complaints.length} complaints) — nothing was compared`);
      await browser.close();
      process.exit(1);
    }
  }
```

and replace the skip at `golden.js:85` with a hard failure, because after the guard above it is unreachable except through a bug:

```js
        if (!w) { console.log(`UNBLESSED STOP ${stops[i]} (${name})`); drifted++; continue; }
```

Note for the implementer: the fixture stores `cams[name]` as an object keyed by stop year, so `Object.keys(...)` returns strings; `.map(Number)` above is what makes the comparison typed. JSON object key order is insertion order for integer-like keys in V8 — **it is not**, integer-like keys are ordered numerically ascending. The blessed stops are ascending in the fixture and `state.stops` is ascending, so the ordered comparison is meaningful; if a future stop list is ever non-ascending this guard must be revisited, and the comment above says so.

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test crates/map-viewer/tests/golden.test.mjs`
Expected: PASS, 5 tests.

- [ ] **Step 5: Run the gate itself, unchanged world**

Run (from `C:\Users\donov\.claude\jobs\c6946bce\tmp`): `node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check`
Expected: `ALL GOLDEN VIEWS HOLD`. The guard is silent because nothing has changed yet. **If it complains now, the repair has found a pre-existing drift and that is a finding — stop and report it, do not adjust the guard.**

- [ ] **Step 6: Commit**

```bash
git add crates/map-viewer/tests/golden.js crates/map-viewer/tests/golden.test.mjs
git commit -m "The golden gate refuses a changed stop set: an instrument that examines less is not a gate"
```

---

### Task 2: The client-surface law — every URL the viewer fetches names a contract function

Spec §6's first dogfood-bar checkable is "viewer entirely on contract functions". That is a claim about a body of client code, so it gets a whole-body assertion, not an eyeball. Written now, it goes RED immediately and stays red until Task 9 — which is the point: it is the stage's progress bar and its exit criterion in one file.

**Files:**
- Create: `crates/map-viewer/tests/surface.test.mjs`
- Modify: `crates/map-viewer/src/lib.rs` (add a `#[cfg(test)] mod surface_tests` — see Step 3)

**Interfaces:**
- Consumes: nothing.
- Produces: two whole-body pins that every later task must keep true —
  - `crates/map-viewer/tests/surface.test.mjs`: the exact set of URL paths appearing in `page.html` + `limb.js`.
  - `served_surface_is_exactly_the_contract` in `lib.rs`: the exact set of paths `route` answers with something other than 404.

- [ ] **Step 1: Write the failing client-surface test**

Create `crates/map-viewer/tests/surface.test.mjs`:

```js
// THE DOGFOOD BAR, as a law (spec §6): the viewer consumes only
// contract functions. Whole-body — the ENTIRE set of paths the client
// fetches is pinned, so a route that creeps back in fails here, and a
// route that leaves must be removed from the expected set deliberately.
// Not "no legacy path appears": that check is satisfiable by a client
// that fetches nothing at all.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const SRC = ["../src/page.html", "../src/limb.js"]
  .map(f => readFileSync(new URL(f, import.meta.url), "utf8"))
  .join("\n");

// Every string literal that starts with "/" and looks like a path we
// hand to fetch() or a <script src>. Deliberately blunt: it must catch
// a path built anywhere, not only inside a fetch( call.
function pathsIn(src) {
  const found = new Set();
  for (const m of src.matchAll(/["'`](\/[A-Za-z0-9_./-]*)/g)) found.add(m[1]);
  return [...found].sort();
}

// The contract function set (spec §3), as URLs, plus the application
// shell. `subjects` is a contract function by spec §5 Stage 0, which
// froze it into the corpus as fact/subjects.feature.
const CONTRACT_SURFACE = [
  "/",              // the page itself
  "/api/changes",   // changes(from, to)
  "/api/laws",      // laws() -- the chronology frame lives here
  "/api/resources", // resources(ids)
  "/api/scene",     // scene(pieces, at|over, style, camera?)
  "/api/standings", // standings(at | over)
  "/api/styles",    // styles()
  "/api/subjects",  // subjects(at)
  "/limb.js",       // the shared containment module, the app shell
].sort();

test("the viewer fetches exactly the contract surface — no more, no less", () => {
  assert.deepEqual(pathsIn(SRC), CONTRACT_SURFACE);
});
```

- [ ] **Step 2: Run and verify FAIL**

Run: `node --test crates/map-viewer/tests/surface.test.mjs`
Expected: FAIL, with a diff naming the extra paths `/api/meta`, `/api/overlay`, `/api/region_times`, `/api/render` and the missing `/api/laws`, `/api/standings`, `/api/styles`. **Record that exact list in the commit message — it is this stage's worklist.**

Adaptation note: `pathsIn` is blunt on purpose and may also catch CSS `url(...)` paths or href fragments that exist in `page.html`. Run it, read what it returns, and if it catches non-fetch paths that are genuinely part of the page's markup (not URLs the client requests from this server), add them to `CONTRACT_SURFACE` with a one-line comment saying what they are. Do **not** narrow the regex to only `fetch(` — the whole value of the blunt version is that it catches a path assembled anywhere.

- [ ] **Step 3: Write the failing server-surface test**

Append to `crates/map-viewer/src/lib.rs`'s test module (the crate already has one — `lib.rs:1300` is inside it; add there):

```rust
// The OTHER half of the dogfood bar: the surface the server actually
// serves, pinned whole. Probing "is /api/meta gone?" is satisfiable by
// a server that answers nothing; this asserts the entire set.
#[test]
fn served_surface_is_exactly_the_contract() {
    let app = test_app();
    let candidates = [
        "/", "/limb.js",
        "/api/borders", "/api/census", "/api/changes", "/api/contract",
        "/api/disposition", "/api/edge_summary", "/api/edges", "/api/entities",
        "/api/features", "/api/laws", "/api/meta", "/api/node", "/api/overlay",
        "/api/region_times", "/api/render", "/api/resource", "/api/resources",
        "/api/scaffold", "/api/scene", "/api/standings", "/api/styles",
        "/api/subjects", "/api/transition",
    ];
    // A route is SERVED if it does not 404. A 400 (missing required
    // parameter) is a served route refusing a bad query -- exactly what
    // we want to count, since it proves the handler exists.
    let served: Vec<&str> =
        candidates.iter().copied().filter(|p| route(&app, p, "").0 != 404).collect();
    let expected = vec![
        "/", "/limb.js",
        "/api/borders", "/api/census", "/api/changes", "/api/contract",
        "/api/disposition", "/api/edge_summary", "/api/edges", "/api/entities",
        "/api/laws", "/api/node", "/api/resource", "/api/resources",
        "/api/scene", "/api/standings", "/api/styles", "/api/subjects",
        "/api/transition",
    ];
    assert_eq!(served, expected, "the served surface is not the contract surface");
}
```

Adaptation note: `test_app()` — reuse whatever this crate's existing test module already builds an `App` with. If the existing tests construct params only and never an `App`, build one through the same composition root the binary uses and mark the test `#[ignore]`-free but tolerant of a missing canon: the routes that need the canon return 400, not 404, which is what this test counts. **`candidates` deliberately lists routes from Stages 2 and 3** (`borders`, `disposition`, `laws`, `standings`, `node`, `edges`, `edge_summary`) — if a sibling stage named one of them differently, correct the two lists together and note the correction in the commit message.

- [ ] **Step 4: Run and verify FAIL**

Run: `cargo test -p map-viewer served_surface_is_exactly_the_contract`
Expected: FAIL — `served` still contains `/api/features`, `/api/meta`, `/api/overlay`, `/api/region_times`, `/api/render`, `/api/scaffold` and lacks `/api/styles`.

- [ ] **Step 5: Do NOT implement. Commit the two red laws.**

These two tests are the stage's specification. They go green in Task 9 and Task 3 respectively. Mark them so no one "fixes" them by weakening them:

```bash
git add crates/map-viewer/tests/surface.test.mjs crates/map-viewer/src/lib.rs
git commit -m "The dogfood bar as two whole-body laws: the served surface and the client's fetch surface

Both RED until Stage 4 finishes. The worklist they name:
  client still fetches: /api/meta /api/overlay /api/region_times /api/render
  client still lacks:   /api/standings /api/styles
  server still serves:  /api/features /api/meta /api/overlay /api/region_times /api/render /api/scaffold
  server still lacks:   /api/styles"
```

---

## Part B — the strangler, finished: the consumer moves, then the route dies

### Task 3: `styles()` on the wire, carrying which pieces each style dresses

Spec §3's scene tier requires `styles() → [StyleId + which pieces each dresses]`. No earlier stage adds it, and it is the only home for the second half of law 4 — *"a style may dress any subset of pieces"* — on the wire. It also replaces `/api/meta`'s `styles` array, which Task 4 needs.

**Files:**
- Create: `contracts/map-api/scene/styles.feature`
- Create: `contracts/map-api/fixtures/styles.json` (blessed in Step 6)
- Modify: `crates/map-viewer/src/lib.rs` (new `"/api/styles"` arm in `route_text`)
- Modify: `crates/map-types/src/style.rs` (add `Style::dresses`)
- Modify: `contracts/runner/src/Steps.hs` (one new step definition)
- Modify: `contracts/CHANGELOG.md`

**Interfaces:**
- Consumes: `map_types::{Piece, PieceSet}` and `PieceSet::render` (Stage 1 Task 8).
- Produces:
  - `Style::dresses(&self) -> PieceSet` — which pieces this style declares dress data for. Task 11 makes this vary; until then every loaded style dresses all ten.
  - `GET /api/styles` → a JSON array, sorted by `name`, of `{ "id": "<16 lowercase hex>", "name": "<stem>", "default": <bool>, "dresses": ["<piece>", …] }` with `dresses` sorted alphabetically. Tasks 4, 11, 12 consume it.

- [ ] **Step 1: Write the failing contract scenario**

Create `contracts/map-api/scene/styles.feature`:

```gherkin
Feature: styles — the dresses on offer, and what each one dresses
  A style is a dress, and spec §3 law 4 says a style may dress any
  SUBSET of pieces: what it omits takes the declared classical default.
  This function is where that subset becomes visible to a client
  instead of being a private fact of the template loader. The whole
  answer is pinned: a style appearing, vanishing, or changing what it
  dresses is a change to a published promise.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |

  Scenario: the whole style book, as it is served
    When I GET /api/styles as book
    Then book equals fixture "styles"

  Scenario: exactly one style is the declared default
    When I GET /api/styles as book
    Then exactly one entry in book is the default

  Scenario: every style dresses at least Ground, and no style dresses an unknown piece
    When I GET /api/styles as book
    Then every entry's dresses is a subset of the piece vocabulary
```

Note on the second and third scenarios: these read like existential pokes and would be forbidden as such — they are not. The whole body is already pinned by scenario 1 against a blessed fixture; these two state the *invariants that must survive re-blessing*, which a fixture cannot state. When Task 12 adds `terrain` and re-blesses `styles.json`, scenario 1 changes and these two must still hold. That is their entire job, and the reviewer should refuse them if scenario 1 is ever removed.

- [ ] **Step 2: Add the two step definitions to the runner**

In `contracts/runner/src/Steps.hs`, beside the existing `I GET <url> as <name>` definition, add:

```haskell
  -- Law-shaped steps over the style book. Both read the WHOLE body and
  -- fail naming the offending entry, never "some entry is wrong".
  , step "exactly one entry in <name> is the default" $ \nm w -> do
      body <- bound nm w
      let entries = body ^.. values
          defaults = [ e | e <- entries, e ^? key "default" . _Bool == Just True ]
      case defaults of
        [_] -> pure ()
        ds  -> failWith $ "expected exactly one default style, found "
                          <> tshow (length ds) <> ": " <> tshow (map (^? key "name") ds)
  , step "every entry's dresses is a subset of the piece vocabulary" $ \nm w -> do
      body <- bound nm w
      let known = Set.fromList (map pieceName [minBound .. maxBound])
          bad = [ (e ^? key "name", d)
                | e <- body ^.. values
                , d <- e ^.. key "dresses" . values . _String
                , not (d `Set.member` known) ]
      unless (null bad) $ failWith $ "unknown piece in dresses: " <> tshow bad
```

Adaptation note: this is written against Stage 0's `Steps.hs` shape (a list of `step "<pattern>" $ \captures world -> …`) and Stage 0's `Capture.Piece` type. Match the file's actual combinator names and its `bound` / `failWith` equivalents; if it uses lens-free JSON access, use that. **Do not invent a new assertion helper** — the totality law (`contract-runner check`) will name any step you leave orphaned.

- [ ] **Step 3: Run the runner and verify RED**

```bash
cd contracts/runner
cabal run contract-runner -- check ../map-api
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
```
Expected: `check` passes (every step has exactly one definition); `run` reports the three `styles` scenarios red — `/api/styles` returns 404.

- [ ] **Step 4: Write the failing Rust test**

Append to `crates/map-viewer/src/lib.rs`'s test module:

```rust
#[test]
fn every_loaded_style_declares_what_it_dresses() {
    use map_types::piece::{Piece, PieceSet};
    let app = test_app();
    for (name, id) in &app.styles {
        let st = app.style_values.get(id).expect("a loaded style has values");
        let d = st.dresses();
        // Today every template declares every field, so every style
        // dresses everything. Task 11 makes this vary; this assertion
        // is what will notice when it does.
        assert_eq!(d, PieceSet::all(), "style {name} dresses {}", d.render());
        assert!(d.contains(Piece::Ground), "a dress that does not reach Ground is not a dress");
    }
}
```

- [ ] **Step 5: Run and verify FAIL**

Run: `cargo test -p map-viewer every_loaded_style_declares_what_it_dresses`
Expected: FAIL — no method `dresses` on `Style`.

- [ ] **Step 6: Implement `Style::dresses` and the route**

In `crates/map-types/src/style.rs`, beside `topo_ramp` (`style.rs:419`):

```rust
    /// WHICH PIECES THIS DRESS REACHES (spec §3 law 4: a style may
    /// dress any subset of pieces). Derived from what the style
    /// declares, never hand-listed: until a field can be omitted
    /// (Task 11) every constructed Style declares everything, so this
    /// is total. When omission lands, this becomes the honest answer
    /// and nothing that consumes it has to change.
    pub fn dresses(&self) -> crate::piece::PieceSet {
        crate::piece::PieceSet::all()
    }
```

In `crates/map-viewer/src/lib.rs`, add an arm to `route_text` immediately after the `"/api/meta"` arm (it will outlive `/api/meta` by four tasks):

```rust
        // STYLES: the dresses on offer, and which pieces each reaches
        // (spec §3, scene tier). The style book is the templates
        // directory; the default is DECLARED, never alphabetical.
        "/api/styles" => {
            let default_id = app.styles.first().map(|(_, id)| *id);
            let mut rows: Vec<serde_json::Value> = app
                .styles
                .iter()
                .map(|(name, id)| {
                    let dresses: Vec<String> = app
                        .style_values
                        .get(id)
                        .map(|s| s.dresses())
                        .unwrap_or_else(map_types::piece::PieceSet::all)
                        .iter()
                        .map(|p| p.name().to_string())
                        .collect();
                    serde_json::json!({
                        "id": format!("{:016x}", id.0 .0),
                        "name": name,
                        "default": Some(*id) == default_id,
                        "dresses": dresses,
                    })
                })
                .collect();
            rows.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
            (200, "application/json", serde_json::Value::Array(rows).to_string(), Vec::new())
        }
```

Note: `app.styles` is already ordered default-first (`lib.rs:266`, `loaded.sort_by_key(|(name, _, is_default)| (!*is_default, *name))`), which is why `default_id` is `first()`. The response re-sorts by name so the wire answer is stable under a change of default — the `default` flag carries that fact, not the ordering.

- [ ] **Step 7: Rebuild, restart, run the tests green**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
Run: `cargo test -p map-viewer every_loaded_style_declares_what_it_dresses` → PASS.

- [ ] **Step 8: Bless the fixture and confirm the scenarios green**

```bash
cd contracts/runner
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api --bless styles
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
cabal run contract-runner -- vocab ../map-api
```
Expected: `styles.json` written with three entries (`canaan` default, `parchment`, `slate`), all three scenarios green, vocabulary clean.
Adaptation note: match Stage 0's actual blessing flag — check `contract-runner --help`; the plan for Stage 0 named it in Task 14.

- [ ] **Step 9: Golden gate and the surface laws**

Run (from `C:\Users\donov\.claude\jobs\c6946bce\tmp`): `node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check`
Expected: `ALL GOLDEN VIEWS HOLD` — this task added a route and touched no rendering.
Run: `cargo test -p map-viewer served_surface_is_exactly_the_contract`
Expected: still FAIL, but `/api/styles` has moved from the "lacks" side to the "serves" side. Confirm that by reading the diff, not by assuming it.

- [ ] **Step 10: Commit**

```bash
git add contracts/map-api/scene/styles.feature contracts/map-api/fixtures/styles.json \
        contracts/runner/src/Steps.hs contracts/CHANGELOG.md \
        crates/map-types/src/style.rs crates/map-viewer/src/lib.rs
git commit -m "styles(): the dresses on offer, and which pieces each one reaches"
```
Add a line to `contracts/CHANGELOG.md` under `## Unreleased`: `- Added: scene/styles.feature and GET /api/styles (additive).`

---

### Task 4: `/api/meta` dies — stops from `changes()`, styles from `styles()`, the frame from `laws()`

`/api/meta` (`lib.rs:867`) serves five things. Four have contract homes; the fifth dies with `/api/render` in Task 8.

| meta field | contract home | evidence |
|---|---|---|
| `stops` | `changes(-4004, 1900)`, years adjacent-deduped, with a lead-in stop of `first - 100` | `lib.rs:313-317` computes exactly this from `provider.changes_between`. Verified live at write time: `/api/changes?from=-4004&to=1900` is 729 rows / 64 774 bytes, whose adjacent-deduped years are the 88 stops that `/api/meta` reports after its lead-in, and `/api/meta`'s first stop is `-4100 = -4000 - 100`. 89 stops, matching the golden fixture. |
| `styles` | `styles()` | Task 3 |
| `anchor` | `laws()` `[STAGE-2 DEP]` | the chronology frame is a law datum: spec §2's Time equation is stated in terms of "the frame's edge" |
| `encoders` | — | dies with `/api/render` (Task 8) |
| `projections` | — | a client constant: `["globe", "flat"]` are two client-side charts, not two server answers. `lib.rs:879` hardcodes them already. |

**`[STAGE-2 DEP]`:** this task needs `GET /api/laws` to carry the chronology frame as `{"frame": "biblical (Ussher tradition)", "year": -4004}` under a `frame` key. **Read Stage 2's plan before starting.** If `laws()` exists but carries no frame, add it additively here (one row in the law set, one scenario in `laws.feature`, a `PATCH`-level changelog entry). If `laws()` does not exist at all, this task is blocked — report it and stop; do not invent a route outside the spec's function set.

**Files:**
- Modify: `crates/map-viewer/src/page.html` (`init`, `state`)
- Modify: `crates/map-viewer/src/lib.rs` (delete the `"/api/meta"` arm; `App.stops` becomes internal-only)
- Modify: `crates/map-viewer/tests/surface.test.mjs` (nothing — it already expects this)
- Modify: `contracts/map-api/fact/changes.feature` (one new scenario)
- Modify: `contracts/map-api/fixtures/` (one new blessed fixture)

**Interfaces:**
- Consumes: `GET /api/styles` (Task 3); `GET /api/changes?from&to`; `GET /api/laws` `[STAGE-2 DEP]`.
- Produces: `deriveStops(changeRows) -> number[]` in `page.html`, exported onto `window` so the golden gate and the Node test can both reach it.

- [ ] **Step 1: Write the failing contract scenario — the stop list is a law, not a coincidence**

Append to `contracts/map-api/fact/changes.feature`:

```gherkin
  Scenario: the timeline's stops are exactly the years the world changes
    When I GET /api/changes?from=-4004&to=1900 as book
    Then the adjacent-deduped years of book equal fixture "timeline-stops"
```

This is the load-bearing scenario of the whole task: it pins the derivation the client is about to take over. Add its step to `Steps.hs`:

```haskell
  , step "the adjacent-deduped years of <name> equal fixture <fixture>" $ \nm fx w -> do
      body <- bound nm w
      let ys = body ^.. values . key "year" . _Integer
          dedup = map head (group ys)          -- ADJACENT, not distinct:
                                               -- the client dedups a stream
      blessOrCompare fx (toJSON dedup) w
```

- [ ] **Step 2: Run and verify RED**

```bash
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
```
Expected: red — fixture `timeline-stops` does not exist. Bless it:
`cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api --bless timeline-stops`
Then **verify by eye** that the blessed array has 88 entries beginning `-4000, -3000, -2247, -2242, -2200, -2100` — and that `/api/meta`'s `stops` is that same array with `-4100` prepended. If the two disagree, the derivation below is wrong and this task stops here.

- [ ] **Step 3: Write the failing Node test for the client's derivation**

Create `crates/map-viewer/tests/stops.test.mjs`:

```js
// The scrub timeline moves from a server field to a client derivation.
// The golden gate keys 89 blessed stops by year, so this derivation
// being byte-identical to the old server field is the single thing
// standing between Stage 4 and 89 dead probes. Pinned whole, against
// the same fixture the contract suite blessed.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const BLESSED = JSON.parse(readFileSync(
  new URL("../../../contracts/map-api/fixtures/timeline-stops.json", import.meta.url), "utf8"));
const GOLDEN = JSON.parse(readFileSync(
  new URL("./fixtures/golden-views.json", import.meta.url), "utf8"));

// The derivation, lifted out of page.html verbatim for testing.
const page = readFileSync(new URL("../src/page.html", import.meta.url), "utf8");
const body = page.match(/function deriveStops\(rows\) \{[\s\S]*?\n\}/);
assert.ok(body, "deriveStops must exist in page.html");
const deriveStops = new Function("rows", body[0] + "\nreturn deriveStops(rows);");

test("the derived stops are exactly the golden gate's blessed stops", () => {
  const rows = BLESSED.map(y => ({ year: y }));
  const derived = deriveStops(rows);
  const blessedGolden = Object.keys(GOLDEN.cams.levant).map(Number).sort((a, b) => a - b);
  assert.deepEqual(derived, blessedGolden);
  assert.equal(derived.length, 89);
});

test("the lead-in stop is the first change minus a century — the world before the first record", () => {
  assert.deepEqual(deriveStops([{ year: -1000 }, { year: -900 }]), [-1100, -1000, -900]);
});

test("adjacent duplicates collapse; non-adjacent repeats are impossible and must not be silently merged", () => {
  assert.deepEqual(deriveStops([{ year: -10 }, { year: -10 }, { year: -5 }, { year: -10 }]),
                   [-110, -10, -5, -10]);
});

test("an empty change book yields an empty timeline, not a crash", () => {
  assert.deepEqual(deriveStops([]), []);
});
```

The third case is the one that matters: `lib.rs:314` uses `Vec::dedup`, which collapses *adjacent* duplicates only. A client that used `new Set(...)` would silently differ on any out-of-order change stream. The test pins the Rust semantics, not the convenient JS ones.

- [ ] **Step 4: Run and verify FAIL**

Run: `node --test crates/map-viewer/tests/stops.test.mjs`
Expected: FAIL — `deriveStops must exist in page.html`.

- [ ] **Step 5: Implement the client side**

In `crates/map-viewer/src/page.html`, immediately above `async function init()` (`page.html:2245`):

```js
// THE TIMELINE IS DERIVED, NOT SERVED. A scrub stop is a year at which
// the world changes -- exactly the definition changes() answers. The
// server used to compute this in its composition root (lib.rs:313) and
// publish it as /api/meta.stops; that route is not a contract function,
// so the derivation moves here, unchanged. `dedup` is ADJACENT-only,
// matching Rust's Vec::dedup: a repeated year that is not adjacent is a
// different stop and merging it would silently re-key the golden views.
function deriveStops(rows) {
  const out = [];
  for (const r of rows) if (out.length === 0 || out[out.length - 1] !== r.year) out.push(r.year);
  if (out.length) out.unshift(out[0] - 100);   // the state before the first recorded change
  return out;
}
```

Replace the first five lines of `init` (`page.html:2246-2251`) with:

```js
  const [changeBook, styleBook, lawSet] = await Promise.all([
    fetch("/api/changes?from=-4004&to=1900").then(r => r.json()),
    fetch("/api/styles").then(r => r.json()),
    fetch("/api/laws").then(r => r.json()),
  ]);
  state.stops = deriveStops(changeBook);
  state.styles = styleBook.map(s => ({ name: s.name, id: s.id }));
  state.style = (styleBook.find(s => s.default) || styleBook[0]).id;
  state.anchor = lawSet.frame || null;
  state.idx = Math.max(0, state.stops.length - 4);
```

and replace the `encoders` / `projections` rows (`page.html:2257-2261`) with:

```js
  // The two charts are CLIENT renderers, not server answers: globe and
  // flat consume the same camera (the camera law, §32). They were never
  // a server fact -- /api/meta hardcoded this pair at lib.rs:879.
  radioRow($("projections"), [{ name: "globe", value: "globe" }, { name: "flat", value: "flat" }],
    state.projection,
    v => {
```
(keep the existing callback body from `page.html:2262` onward unchanged).

Leave the `encoders` radio row alone for now — Task 8 removes it, and doing it here would put two owner-visible changes in one reviewable unit.

- [ ] **Step 6: Delete the `/api/meta` handler**

Delete `lib.rs:867-884` (the whole `"/api/meta" => { … }` arm). `App.stops` and `App.anchor` stay — `stops` still feeds `presence` at `lib.rs:318-333`, and `anchor` feeds `laws()`. Confirm with `cargo build --release` that no `unused field` warning appears; if one does, that field genuinely has no reader left and should be deleted with a note in the commit message.

- [ ] **Step 7: Rebuild, restart, run every affected test**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
```bash
node --test crates/map-viewer/tests/stops.test.mjs      # PASS
cargo test --workspace                                   # no new failures
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
```

- [ ] **Step 8: THE GOLDEN GATE — this is the first task that can move a pixel**

Run (from `C:\Users\donov\.claude\jobs\c6946bce\tmp`): `node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check`
Expected: `ALL GOLDEN VIEWS HOLD`, 89/89.
**What is allowed to change: nothing.** The stop list is derived from the same numbers by the same rule; the default style is the same style, now flagged rather than positioned. If Task 1's guard complains about the stop set, the derivation is wrong — fix the derivation, never the fixture. If a colour probe drifts, the default style changed: check that `styleBook.find(s => s.default)` picks `canaan`, which `/api/styles` flags because `lib.rs:266` sorts the declared default first.

- [ ] **Step 9: Commit**

```bash
git add crates/map-viewer/src/page.html crates/map-viewer/src/lib.rs \
        crates/map-viewer/tests/stops.test.mjs contracts/map-api/fact/changes.feature \
        contracts/map-api/fixtures/timeline-stops.json contracts/runner/src/Steps.hs
git commit -m "/api/meta dies: the timeline is derived from changes(), the dress book from styles(), the frame from laws()"
```

---

### Task 5: `/api/region_times` dies — a subject's timeline is `standings(over)`

`/api/region_times` (`lib.rs:1165`) answers "which scrub stops carry a mapping for this subject". Its data is `App.presence`, built at startup (`lib.rs:318-333`) by calling `provider.subjects(at)` at every stop and recording which subjects appear. That is, exactly, *the intervals over which each entity stands* — which is `standings(over)`.

**`[STAGE-2 DEP]`:** this needs `GET /api/standings?over=<from>..<to>` returning `[{ "entity": "<key>", "from": <year>, "to": <year|null> }]` per spec §3 (`standings(at | over) → [(entity, interval)]`). **Read Stage 2's plan first.** If the delivered shape differs (different parameter spelling, different interval encoding), adapt the client derivation below to it and say so in the commit; do **not** add a second route. If `standings` does not exist, this task is blocked — report and stop.

**Files:**
- Modify: `crates/map-viewer/src/page.html` (`loadMappedTimes`)
- Modify: `crates/map-viewer/src/lib.rs` (delete the `"/api/region_times"` arm; delete `App.presence` and its startup probe)
- Create: `crates/map-viewer/tests/mapped-times.test.mjs`

**Interfaces:**
- Consumes: `GET /api/standings?over=…`; `state.stops` (Task 4).
- Produces: `stopsWithin(standings, stops, subjectKeys) -> number[]` in `page.html` — the stops at which EVERY named subject stands, ascending.

- [ ] **Step 1: Write the failing test**

Create `crates/map-viewer/tests/mapped-times.test.mjs`:

```js
// "The stops that carry a mapping for EVERY selected region" moves from
// a bespoke server route to a derivation over standings(over). The old
// route answered from a startup probe of subjects() at every stop; the
// new one answers from the intervals themselves, which is the same fact
// stated once instead of sampled 89 times.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const page = readFileSync(new URL("../src/page.html", import.meta.url), "utf8");
const body = page.match(/function stopsWithin\(standings, stops, keys\) \{[\s\S]*?\n\}/);
assert.ok(body, "stopsWithin must exist in page.html");
const stopsWithin = new Function("standings", "stops", "keys",
  body[0] + "\nreturn stopsWithin(standings, stops, keys);");

const STOPS = [-1000, -900, -800, -700, -600];
const STANDINGS = [
  { entity: "region:aaaa", from: -1000, to: -800 },   // right-open: -800 excluded
  { entity: "region:bbbb", from: -900, to: null },    // endures to the frame's edge
];

test("one subject: the stops inside its interval, right-open", () => {
  assert.deepEqual(stopsWithin(STANDINGS, STOPS, ["region:aaaa"]), [-1000, -900]);
});

test("an open interval reaches every later stop", () => {
  assert.deepEqual(stopsWithin(STANDINGS, STOPS, ["region:bbbb"]), [-900, -800, -700, -600]);
});

test("two subjects: the INTERSECTION, which is the question the panel asks", () => {
  assert.deepEqual(stopsWithin(STANDINGS, STOPS, ["region:aaaa", "region:bbbb"]), [-900]);
});

test("a subject with no standing yields nothing — and nothing is not everything", () => {
  assert.deepEqual(stopsWithin(STANDINGS, STOPS, ["region:cccc"]), []);
  assert.deepEqual(stopsWithin(STANDINGS, STOPS, ["region:aaaa", "region:cccc"]), []);
});

test("no subjects selected yields nothing, not every stop", () => {
  // The negative case that matters: a bug returning `stops` here would
  // light up the whole scrubber and read as 'everything is mapped'.
  assert.deepEqual(stopsWithin(STANDINGS, STOPS, []), []);
});
```

- [ ] **Step 2: Run and verify FAIL**

Run: `node --test crates/map-viewer/tests/mapped-times.test.mjs`
Expected: FAIL — `stopsWithin must exist in page.html`.

- [ ] **Step 3: Implement**

In `page.html`, immediately above `async function loadMappedTimes()` (`page.html:2050`):

```js
// A SUBJECT'S TIMELINE IS ITS STANDING. The old /api/region_times
// answered from a startup probe that called subjects() at all 89 stops
// and remembered where each subject turned up; standings(over) states
// the same fact once, as the intervals themselves. Intervals are
// RIGHT-OPEN (spec §2 equation 3): `to` is excluded, and a null `to`
// endures to the frame's edge.
function stopsWithin(standings, stops, keys) {
  if (keys.length === 0) return [];
  const spans = new Map();
  for (const s of standings) {
    if (!spans.has(s.entity)) spans.set(s.entity, []);
    spans.get(s.entity).push([s.from, s.to]);
  }
  const covers = (key, y) =>
    (spans.get(key) || []).some(([f, t]) => y >= f && (t === null || y < t));
  return stops.filter(y => keys.every(k => covers(k, y)));
}
```

Replace the body of `loadMappedTimes` from its `const sets = await Promise.all(...)` block through `state.subjectStops = new Set(common);` (`page.html:2058-2064`) with:

```js
  const lo = state.stops[0], hi = state.stops[state.stops.length - 1] + 1;
  const standings = await (await fetch(`/api/standings?over=${lo}..${hi}`)).json();
  const common = stopsWithin(standings, state.stops, state.selection);
  state.subjectStops = new Set(common);
```

The rest of `loadMappedTimes` (the chip rendering from `page.html:2065` on) is unchanged, except that `common.sort(...)` at `page.html:2066` is now redundant — `stopsWithin` filters `state.stops`, which is already ascending. Leave the sort in place; it is free and it documents the invariant.

- [ ] **Step 4: Delete the server side**

- Delete `lib.rs:1165-1174` (the `"/api/region_times"` arm).
- Delete the `presence` field from `App` (`lib.rs:43` region) and its startup probe (`lib.rs:317-334`, `let t_presence = …` through the `eprintln!("presence probed in …")`), plus the `presence,` line in the `App { … }` literal.
- **This deletes an 89-iteration startup probe.** Record the boot-time change in the commit message; `boot.log` at the repo root shows the before.

- [ ] **Step 5: Rebuild, restart, verify**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
```bash
node --test crates/map-viewer/tests/mapped-times.test.mjs   # PASS
cargo test --workspace                                       # no new failures
```

- [ ] **Step 6: Verify the panel by hand against the old answer, before trusting it**

Before this commit is believed, compare the two answers for one real subject at one real stop. The old route is gone, so take the comparison from git:

```bash
git stash && \
  curl -s "http://127.0.0.1:8090/api/region_times?subject=region:<pick-one-from-/api/subjects>" > /tmp/old.json
git stash pop
```
(If the viewer must be rebuilt to answer the old route, that is the honest cost of this check — do it once.) Then derive the same list from `/api/standings` and diff. **A mismatch here is a Stage 2 finding, not a client bug** — report it before proceeding.

- [ ] **Step 7: Golden gate**

Run the gate. Expected: `ALL GOLDEN VIEWS HOLD`. `loadMappedTimes` runs on every `setStop`, and the golden gate calls `setStop` 89 times, so a throw here would surface as a stalled `settled()` and a timeout — not as drift. If the gate hangs, look at the browser console for a `loadMappedTimes` exception first.

- [ ] **Step 8: Commit**

```bash
git add crates/map-viewer/src/page.html crates/map-viewer/src/lib.rs \
        crates/map-viewer/tests/mapped-times.test.mjs
git commit -m "/api/region_times dies: a subject's timeline is its standing, stated once instead of probed 89 times"
```

---

### Task 6: `/api/overlay` dies — a two-instant scene is `scene(…, over=…)`, and two scenes compose in the client

`/api/overlay` (`lib.rs:1179`) renders two subject-sides, tints the held side with the ramp's oldest paint and the current side with its newest (`lib.rs:1189-1191`, `app.overlay_tints`), composes them with `Snapshot::combine` — the monoid — and serves the result as an SVG picture shown as a flat panel over the live map (`page.html:2327-2329`).

Two of its three jobs already have contract homes. The two-instant *range* is `scene(…, over=…)`: `build_query` at `lib.rs:584-590` already turns a `to=` parameter into `TimeSelector::Over`, and `page.html:292` already sends it when "expose" is checked. What has no home is composing **two different subjects** — and spec §3 law 2 is exactly the licence to do that in the client: `scene(a ⊕ b) = scene(a) ⊕ scene(b)`, with paint order coming from the precedence law rather than composition order. Stage 1 made that law true. This task is the first thing to *use* it.

> **STOP. OWNER GATE — read before writing any code.**
>
> The compose panel changes appearance. Today, pressing **compose** replaces the live map with a flat, static picture drawn by the server. Afterwards it draws both sides in the live map itself: you can still spin and zoom while comparing, and the held side keeps its faded tint. The picture will not be pixel-identical to the old flat panel — the live map and the server's picture encoder are two different renderers, and they have always disagreed slightly (this is the known, open renderer paint-order divergence recorded in project memory).
>
> **The alternative, if you would rather not spend the change here:** retire the compose panel altogether — remove the hold/compose/drop buttons — and reach the same comparison through the existing "expose" range control, which already asks the server for a two-instant scene and already draws it live.
>
> This gate is a real fork. Take the answer before Step 1. Record it in the commit message.

**Files:**
- Modify: `crates/map-viewer/src/page.html` (the compose handler; a second scene slot in the retained renderer)
- Modify: `crates/map-viewer/src/lib.rs` (delete the `"/api/overlay"` arm; `App.overlay_tints` becomes `styles()`-visible or dies)
- Modify: `contracts/map-api/scene/scene.feature` (one scenario)

**Interfaces:**
- Consumes: `GET /api/scene?…&pieces=…` (Stage 1 Task 11); `styles()` (Task 3); the composition law, now green (Stage 1 Task 12).
- Produces: `gpu.held` — a second resident scene drawn beneath `gpu.scene`, or `null`. The golden gate never sets it, so `gpu.held === null` must be indistinguishable from today.

- [ ] **Step 1: Write the failing contract scenario — composition, now on two SUBJECTS**

Append to `contracts/map-api/scene/scene.feature`:

```gherkin
  Scenario: two subjects compose — the client's ⊕ is the server's ⊕
    When I render pieces fills, borders at year -1405 in style canaan for subject world as whole
    And I render pieces fills at year -1405 in style canaan for subject world as fillsOnly
    And I render pieces borders at year -1405 in style canaan for subject world as bordersOnly
    Then fillsOnly's resources united with bordersOnly's resources equal whole's resources
```

Adaptation note: Stage 1's Task 12 rewrote the composition scenarios and may already carry a step of this shape under a different wording. **If it does, do not add a second one** — cite it here instead and move to Step 2. The purpose of this scenario is to make the client's compose panel rest on a law that is checked, not on an assumption; an existing equivalent law serves that purpose.

- [ ] **Step 2: Run and verify green or red**

```bash
cd contracts/runner && cabal run contract-runner -- check ../map-api && \
  cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
```
If green, the licence to compose in the client is established and Step 3 may proceed. **If red, STOP** — the client must not compose scenes the server cannot compose. That is a Stage 1 regression and it is reported, not worked around.

- [ ] **Step 3: Write the failing client test**

Create `crates/map-viewer/tests/compose.test.mjs`:

```js
// The compose panel becomes a client-side ⊕ over two scene manifests.
// The law that matters for the golden gate: with nothing held, the
// draw list is EXACTLY today's draw list -- same resources, same order.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const page = readFileSync(new URL("../src/page.html", import.meta.url), "utf8");
const body = page.match(/function drawList\(scene, held\) \{[\s\S]*?\n\}/);
assert.ok(body, "drawList must exist in page.html");
const drawList = new Function("scene", "held", body[0] + "\nreturn drawList(scene, held);");

const SCENE = { wanted: ["a", "b"], features: [{ id: "a", piece: "fills" }, { id: "b", piece: "borders" }] };
const HELD  = { wanted: ["c"],      features: [{ id: "c", piece: "fills" }] };

test("nothing held: the draw list is the scene, unchanged and unwrapped", () => {
  assert.deepEqual(drawList(SCENE, null), SCENE.features);
});

test("something held: the held side draws FIRST, beneath", () => {
  const got = drawList(SCENE, HELD);
  assert.deepEqual(got.map(f => f.id), ["c", "a", "b"]);
});

test("the held side is marked held, and the current side is not", () => {
  const got = drawList(SCENE, HELD);
  assert.equal(got.find(f => f.id === "c").held, true);
  assert.ok(!got.find(f => f.id === "a").held);
});

test("an empty held scene is the identity — absence is the identity element (spec §3 law 1)", () => {
  assert.deepEqual(drawList(SCENE, { wanted: [], features: [] }), SCENE.features);
});
```

- [ ] **Step 4: Run and verify FAIL**

Run: `node --test crates/map-viewer/tests/compose.test.mjs`
Expected: FAIL — `drawList must exist in page.html`.

- [ ] **Step 5: Implement `drawList` and rewire the compose handler**

Add to `page.html` beside the other GPU draw helpers (near `gpuDraw`):

```js
// THE CLIENT'S ⊕ (spec §3 law 2). Two scene manifests draw as one, the
// held side beneath. Paint order inside each side comes from the
// PRECEDENCE LAW the server already stamped on the manifest -- this
// function never reorders within a side, only places one side under the
// other. With nothing held it is the identity, which is what keeps the
// golden gate's 89 stops exactly where they are.
function drawList(scene, held) {
  if (!held || held.features.length === 0) return scene.features;
  return [...held.features.map(f => ({ ...f, held: true })), ...scene.features];
}
```

Replace the compose handler (`page.html:2317-2330`) with:

```js
  $("compose").addEventListener("click", async () => {
    if (!state.held) return;
    // The held side is a SCENE, demanded exactly like the current one,
    // dressed in the held tint that styles() advertises. Composed in
    // the client because the algebra says it may be (law 2), and drawn
    // live because the retained renderer IS the renderer.
    const p = new URLSearchParams(state.held);
    p.set("style", state.heldStyle || state.style);
    gpu.held = await sceneFor(p);
    gpuSync();
  });
```

Adaptation note: `sceneFor(params)` — the existing scene-demand path in `page.html` around line 832 is a single function that mutates `gpu.scene`. Refactor it so the fetch-and-publish half returns a scene object and the caller assigns it; that refactor is this step's real work and it must leave `gpu.scene`'s path byte-identical. Then make `gpuDraw` iterate `drawList(gpu.scene, gpu.held)` rather than `gpu.scene.features`, and make the `drop` handler (`page.html:2312`) set `gpu.held = null`.

`state.heldStyle`: `app.overlay_tints` (`lib.rs:1189`) maps a base style to a (held, current) pair of style ids. Those ids must now be reachable from the client. Add them to `/api/styles` as a `held` field on each entry (`"held": "<16 hex>"`), extend `styles.feature`'s pinned fixture, and bump the changelog — additive, PATCH.

- [ ] **Step 6: Delete the server side**

Delete `lib.rs:1179-1214` (the `"/api/overlay"` arm). Keep `App.overlay_tints` — `/api/styles` now publishes it.

- [ ] **Step 7: Rebuild, restart, verify**

```bash
node --test crates/map-viewer/tests/compose.test.mjs        # PASS
node --test crates/map-viewer/tests/surface.test.mjs        # /api/overlay gone from the diff
cargo test --workspace
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
```

- [ ] **Step 8: THE GOLDEN GATE — the highest-risk gate in the plan so far**

Run the gate. Expected: `ALL GOLDEN VIEWS HOLD`.
**What is allowed to change: nothing.** `gpu.held` is `null` for every one of the gate's 178 probes, and `drawList(scene, null)` returns `scene.features` — the identity, pinned by the first test in Step 3. If a probe drifts, the `sceneFor` refactor in Step 5 changed the current side's path; revert the refactor and redo it smaller. Do **not** re-bless.

- [ ] **Step 9: Commit**

```bash
git add crates/map-viewer/src/page.html crates/map-viewer/src/lib.rs \
        crates/map-viewer/tests/compose.test.mjs contracts/map-api/scene/scene.feature \
        contracts/map-api/scene/styles.feature contracts/map-api/fixtures/styles.json \
        contracts/CHANGELOG.md
git commit -m "/api/overlay dies: two scenes compose in the client, because the algebra says they may

Owner gate answered: <record the decision from the gate above>"
```

---

### Task 7: `/api/scaffold`, `/api/features`, and the entity-id `pieces=` branch die — nothing consumes them

Three legacy surfaces with no client and no contract function. `/api/scaffold` (`lib.rs:993`) is land+water+relief as a served picture — `scene(pieces=ground,water)` answers it. `/api/features` (`lib.rs:1053`) is GeoJSON for a set of entity ids — `borders(entity, at)` answers it `[STAGE-3 DEP]`. The entity-id `pieces=` branch in `composed_scene` (`lib.rs:703-722`) is their sibling from commit `c72ea95`, and it is also a live hazard: it shadows Stage 1's PieceSet `pieces=` parameter on `/api/scene`, because `composed_scene` tests it before `build_query` ever sees it.

- [ ] **Step 1: FIRST — establish whether the collision is live. This gates everything.**

```bash
curl -s "http://127.0.0.1:8090/api/scene?year=-1405&zoom=90.0000&style=canaan&pieces=fills,borders" | head -c 400
```
Read the answer carefully.
- A **400** mentioning `center and zoom (alignment law)` or a manifest with near-zero features means the entity-id branch is still winning and **Stage 1's `?pieces=` parameter never reached `/api/scene`**. That is a Stage 1 defect. Record it, tell the owner, and continue — this task fixes it.
- A manifest whose entries carry only `fills` and `borders` pieces means Stage 1 already resolved the collision. Confirm by reading `composed_scene` in `lib.rs` and note how, then continue.

Either way, write the finding into this task's commit message. **This is the single most consequential check in the plan** — every scene-piece claim from Stage 1 onward depends on the parameter actually arriving.

- [ ] **Step 2: Write the failing test**

Append to `lib.rs`'s test module:

```rust
#[test]
fn the_pieces_parameter_selects_pieces_and_nothing_else_claims_that_name() {
    use map_types::piece::{Piece, PieceSet};
    let app = test_app();
    // A pieces= query with NO camera must be accepted: the entity-id
    // branch demanded center+zoom (the alignment law of a different
    // API), and its survival is exactly what this asserts against.
    let (status, _, body, _) = route(&app, "/api/scene", "year=-1405&pieces=fills,borders&style=canaan");
    assert_eq!(status, 200, "pieces= must not demand a camera: {}", String::from_utf8_lossy(&body));
    let man: serde_json::Value = serde_json::from_slice(&body).expect("a manifest");
    let seen: std::collections::BTreeSet<String> = man["features"]
        .as_array().expect("features")
        .iter()
        .map(|f| f["piece"].as_str().expect("every entry names its piece").to_string())
        .collect();
    assert_eq!(
        seen,
        ["borders", "fills"].iter().map(|s| s.to_string()).collect(),
        "the scene contains pieces nobody asked for"
    );
    // And the whole vocabulary still parses, so no piece became
    // unreachable when the shadowing branch went away.
    for p in Piece::ALL {
        let q = format!("year=-1405&style=canaan&pieces={}", p.name());
        assert_eq!(route(&app, "/api/scene", &q).0, 200, "piece {} is unreachable", p.name());
    }
    let _ = PieceSet::all();
}
```

Adaptation note: `man["features"]` — read `manifest_json()` in `crates/map-encoders/src/gpu.rs` for the actual array names. Stage 1 Task 10 stamps `piece` on "all three manifest resource types and all three JSON arrays"; assert over all three, not just one. A test that checks one array while two go unchecked is a check satisfiable by the failure mode.

- [ ] **Step 3: Run and verify FAIL**

Run: `cargo test -p map-viewer the_pieces_parameter_selects_pieces`
Expected: FAIL — either a 400 from the alignment law, or a manifest containing pieces nobody asked for.

- [ ] **Step 4: Delete**

- Delete `lib.rs:993-1029` (the `"/api/scaffold"` arm).
- Delete `lib.rs:1053-1085` (the `"/api/features"` arm).
- Delete `lib.rs:701-722` (the entity-id `pieces=` branch at the head of `composed_scene`, from the `// A PIECES render:` comment through the closing `}` of the `if let Some(ids)` block).
- If `canon.render_pieces` now has no callers, delete it too and its `map_canon` re-export; run `cargo build --release` and let the dead-code warnings name what else goes. **Do not delete anything a Stage 2 or Stage 3 route calls** — grep first.

- [ ] **Step 5: Rebuild, restart, verify green**

```bash
cargo test -p map-viewer the_pieces_parameter_selects_pieces      # PASS
cargo test --workspace
cargo test -p map-viewer served_surface_is_exactly_the_contract   # three fewer extras
```

- [ ] **Step 6: Golden gate**

Run the gate. Expected: `ALL GOLDEN VIEWS HOLD`.
**What is allowed to change: nothing** — but this is the first task that touches `composed_scene`, which every scene the gate renders flows through. If a probe drifts, the deletion took a line it should not have; `git diff` the arm boundaries and put back exactly what was outside the deleted block.

- [ ] **Step 7: Commit**

```bash
git add crates/map-viewer/src/lib.rs crates/map-canon/src/
git commit -m "The composable API dies: scaffold, features, and the entity-id pieces= branch that shadowed the real one

Finding on the pieces= collision: <record Step 1's answer verbatim>"
```

---

### Task 8: `/api/render` dies, and the inspection panel with it

`/api/render` (`lib.rs:1087`) serves a picture — SVG or GeoJSON. There is no contract function for "a picture": spec §3's scene tier answers with a manifest and content-addressed resources, and the client draws. The retained renderer already replaced the SVG display path (the comment at `page.html:313-319` records it); what remains is the **inspection panel**, which fetches `/api/render?encoder=geojson` and shows the result as a flat panel over the map (`page.html:320-329`).

The SVG and GeoJSON encoders themselves do **not** die. They live in `crates/map-encoders`, they are the verification oracle, and `map-cli` uses them (`crates/map-viewer/src/bin/map-cli.rs`, `make maps`, `make artifacts`). Only the route dies.

> **STOP. OWNER GATE — read before writing any code.**
>
> The **encoders** row in the workbench sidebar disappears. Today it offers "map" and "geojson"; picking "geojson" replaces the live map with a panel of raw GeoJSON text. Afterwards there is only the map.
>
> What you keep: every picture the command line makes. `make maps` still writes SVGs into `out/maps/`, `make artifacts` still writes plates into `out/artifacts/`, and the SVG encoder is still the thing the tests compare against. Nothing about the pictures you actually publish changes.
>
> What you lose: the ability to eyeball the GeoJSON of the current view from inside the workbench, without going to the command line.
>
> **If you want to keep in-workbench inspection**, say so — the honest replacement is a panel that shows the *scene manifest* (`/api/scene`'s own JSON answer, which the client already has in hand and which is a contract function). That is a small extra task, and it is the version this plan recommends if the panel is worth keeping at all.
>
> Take the answer before Step 1. Record it in the commit message.

**Files:**
- Modify: `crates/map-viewer/src/page.html` (delete `render()`'s fetch, the encoders row, `state.encoder`, `showPlate`, `frameStatus`, `inspectOff`, `clearPlateFrames` as they become dead)
- Modify: `crates/map-viewer/src/lib.rs` (delete the `"/api/render"` arm; delete `encode` if `/api/render` was its last caller)

- [ ] **Step 1: Write the failing test**

Add to `crates/map-viewer/tests/surface.test.mjs` — no change needed; `/api/render` is already absent from `CONTRACT_SURFACE`, so that test is the failing test for this task. Confirm it names `/api/render` today:

Run: `node --test crates/map-viewer/tests/surface.test.mjs`
Expected: FAIL, with `/api/render` listed as an extra. **That is this task's red.** Additionally append to `lib.rs`'s test module:

```rust
#[test]
fn the_picture_encoders_survive_their_route() {
    // The route dies; the encoders do not. map-cli and the verification
    // oracle both depend on them, and a deletion that took them with it
    // would be discovered by `make maps` weeks later.
    use map_encoders::{Encoder, GeoJsonEncoder, SvgEncoder};
    let scene = map_types::Snapshot::empty();
    assert!(GeoJsonEncoder.encode(&scene).is_ok());
    assert!(SvgEncoder::default().encode(&scene).is_ok());
}
```

Adaptation note: match the real trait and constructor names in `crates/map-encoders/src/lib.rs`; `Snapshot::empty()` is used at `lib.rs:1185` so it exists.

- [ ] **Step 2: Run and verify FAIL / PASS-for-the-right-reason**

Run: `cargo test -p map-viewer the_picture_encoders_survive_their_route`
Expected: PASS today (the encoders exist). That is fine — this is a *guard*, and its job starts in Step 4. Run it again after the deletion; if it fails then, the deletion went too far.

- [ ] **Step 3: Delete the client side**

In `page.html`:
- Delete the `encoders` radio row from `init` (`page.html:2253-2260`, the block from the `// The map is the map; encoders are INSPECTION panels` comment through the `v => { state.encoder = v || null; render(); });` line).
- Delete `encoder: …` from the `state` literal and the `if (state.encoder) p.set("encoder", …)` line (`page.html:297`).
- Reduce `render()` (`page.html:320-329`) to its live-map path:
  ```js
  // THE RETAINED RENDERER IS THE RENDERER, and now it is the only one.
  // The server's SVG and GeoJSON encoders live on where they always
  // belonged -- in map-cli and in the verification oracle -- but the
  // viewer never asks the server for a picture.
  function render() { gpuSync(); }
  ```
- Delete the three `if (state.encoder) return;  // an inspection panel is up` guards (`page.html:1912, 1960, 1991`).
- Delete `showPlate`, `frameStatus`, and `inspectOff` if nothing else calls them. **Grep before deleting each**: `clearPlateFrames` may still be used by other panels.
- Delete the `<div id="encoders">` element from the markup.

- [ ] **Step 4: Delete the server side**

- Delete `lib.rs:1087-1107` (the `"/api/render"` arm).
- `encode` (`lib.rs:632-689`) was called by `/api/render`, `/api/scaffold` (deleted in Task 7) and `/api/overlay` (deleted in Task 6). If it now has no callers, delete it. **Then re-run `the_picture_encoders_survive_their_route`** — that test is precisely the line between deleting a route's plumbing and deleting the encoders.
- The `X-Attribution`, `X-Scene-Pid` and `X-Query-Pid` headers were `/api/render`'s. `/api/scene` sets only `X-Resolved-View` (`lib.rs:1151`). Check whether `setStatus` in `page.html` still has a source for its attribution field; if not, either drop the status line's attribution or move it into the scene manifest. **Prefer moving it**: attribution is a property of the scene, and `manifest_json` already carries `scene.attribution` — confirm at `crates/map-encoders/src/gpu.rs`. Say which you did in the commit.

- [ ] **Step 5: Rebuild, restart, verify**

```bash
node --test crates/map-viewer/tests/surface.test.mjs      # /api/render gone
cargo test --workspace
cargo test -p map-viewer the_picture_encoders_survive_their_route   # still PASS
./target/release/map-cli plates                            # the CLI still makes pictures
PORT=8090 bash scripts/make-maps.sh                        # and so does make maps
```
The last two are the real proof that the encoders survived. Run them.

- [ ] **Step 6: THE GOLDEN GATE**

Run the gate. Expected: `ALL GOLDEN VIEWS HOLD`.
**What is allowed to change: nothing.** `state.encoder` was `null` for every golden probe (`page.html:321` returns early), so the gate never touched `/api/render`. If a probe drifts, the deletion in Step 3 reached past the inspection path — most likely one of the three `if (state.encoder) return;` guards was protecting something else. Read each deleted guard's surrounding function before re-deleting.

- [ ] **Step 7: Commit**

```bash
git add crates/map-viewer/src/page.html crates/map-viewer/src/lib.rs
git commit -m "/api/render dies: the viewer never asks the server for a picture

The SVG and GeoJSON encoders live on in map-cli and the oracle; make maps
and make artifacts verified after the deletion.
Owner gate answered: <record the decision>"
```

---

### Task 9: The four legacy flags die — `pieces=` is the only piece vocabulary, and both surface laws go green

`build_query` (`lib.rs:603-616`) still reads `labels=`, `topo=`, `relief=` and `journeys=`. Stage 1 kept them deliberately — its Task 11 says *"When `pieces=` is absent, the legacy `labels=`/`topo=`/`relief=`/`journeys=` flags decide, exactly as today, so the viewer page and the golden gate are untouched."* Stage 4 is where that bridge comes down. Both the client (`currentParams`, `page.html:306-309`) and the contract runner (`Steps.hs:26-30`, `sceneUrl`) emit the flags today.

**Files:**
- Modify: `crates/map-viewer/src/lib.rs` (`build_query`)
- Modify: `crates/map-viewer/src/page.html` (`currentParams`)
- Modify: `contracts/runner/src/Steps.hs` (`sceneUrl` emits `pieces=`)
- Re-bless: every `contracts/map-api/fixtures/scene-*.json` **only if the bodies actually change** — see Step 5

**Interfaces:**
- Consumes: `PieceSet::{parse, render}` (Stage 1 Task 8), `?pieces=` (Stage 1 Task 11, verified live in Task 7 Step 1).
- Produces: `/api/scene` accepts `pieces=` and nothing else; `PieceSet::default_for_viewer()` in `map-types` — the declared default set, named once instead of assembled from four flag defaults.

- [ ] **Step 1: Write the failing equivalence test — the flags and the piece set must name the SAME set before either can go**

Append to `crates/map-types/src/tests.rs`:

```rust
#[test]
fn the_viewer_default_is_a_declared_set_not_four_flag_defaults() {
    use crate::piece::{Piece, PieceSet};
    // The set the four legacy flags produced with all four at their
    // defaults (lib.rs:603-616 before Stage 4): labels on, water on,
    // relief OFF, journeys on, and GEOMETRY = fills + borders + claims.
    // Written out here ONCE, as the declared default, so that deleting
    // the flags cannot silently change what a bare query means.
    let expected = PieceSet::empty()
        .with(Piece::Water)
        .with(Piece::Fills)
        .with(Piece::Borders)
        .with(Piece::Claims)
        .with(Piece::Labels)
        .with(Piece::Journeys)
        .with(Piece::Markers)
        .with(Piece::Chrome)
        .with(Piece::Veil);
    assert_eq!(PieceSet::default_for_viewer(), expected);
    assert!(!PieceSet::default_for_viewer().contains(Piece::Ground),
            "relief has always been opt-in; making it default would repaint every stop");
    // And it round-trips, so the client can send it as a string.
    assert_eq!(PieceSet::parse(&PieceSet::default_for_viewer().render()),
               Ok(PieceSet::default_for_viewer()));
}
```

**Adaptation note, and it is load-bearing:** the `expected` set above is this plan's reading of `lib.rs:603-616` combined with Stage 1 Task 8's mapping of `LayerSet::GEOMETRY` onto fills/borders/claims, and of `Markers`/`Chrome`/`Veil` as always-present in v0.1 (the wart `scene.feature`'s preamble records). **Before writing this test, read Stage 1's delivered `build_query` and copy its actual default arms.** If the real default differs, the test's `expected` changes and the golden gate is what proves which reading is right. Do not adjust the golden fixture to match a guess.

- [ ] **Step 2: Run and verify FAIL**

Run: `cargo test -p map-types the_viewer_default_is_a_declared_set`
Expected: FAIL — no `default_for_viewer`.

- [ ] **Step 3: Write the failing viewer test**

Append to `lib.rs`'s test module:

```rust
#[test]
fn the_legacy_flags_are_gone_and_a_bare_query_is_the_declared_default() {
    use map_types::piece::{Piece, PieceSet};
    let app = test_app();
    let q = |s: &str| build_query(&app, &params(s), "", None).expect("query");

    // A bare query is the declared default -- one named thing.
    assert_eq!(q("year=-1405&style=canaan").pieces, PieceSet::default_for_viewer());

    // The four flags are INERT. Not "rejected" -- a stale bookmark must
    // still draw a map -- but they no longer decide anything, and the
    // negative case is the whole test: if labels= still worked, this
    // would pass by accident.
    for stale in ["labels=0", "topo=0", "relief=1", "journeys=0"] {
        let s = format!("year=-1405&style=canaan&{stale}");
        assert_eq!(q(&s).pieces, PieceSet::default_for_viewer(), "{stale} still decides pieces");
    }

    // pieces= is the only vocabulary, and it reaches every piece.
    for p in Piece::ALL {
        let s = format!("year=-1405&style=canaan&pieces={}", p.name());
        assert_eq!(q(&s).pieces, PieceSet::empty().with(p));
    }
    assert_eq!(q("year=-1405&style=canaan&pieces=none").pieces, PieceSet::empty());
    assert!(build_query(&app, &params("year=-1405&style=canaan&pieces=topografy"), "", None).is_none());
}
```

- [ ] **Step 4: Implement**

In `crates/map-types/src/piece.rs`:

```rust
impl PieceSet {
    /// THE DECLARED DEFAULT (spec §3 law 4: an omitted selection is the
    /// declared default, never an error and never an accident). This
    /// was four independent flag defaults scattered through
    /// `build_query`; it is now one named set, so "what does a bare
    /// scene request mean" has exactly one answer and one place to
    /// change it. Ground is out: hypsometric relief has always been
    /// opt-in, and 89 blessed views say so.
    pub fn default_for_viewer() -> Self {
        PieceSet::all().without(Piece::Ground)
    }
}
```

Note: `all().without(Ground)` must equal the explicit nine-piece set in the Step 1 test. If it does not — if Stage 1's `Piece::ALL` has a variant the viewer never renders — write the explicit set instead and say why in the doc comment. Never write `all().without(x).without(y)` chains to chase an equality; that is a tuned constant wearing a type's clothes.

In `crates/map-viewer/src/lib.rs`, replace `build_query`'s flag block (`lib.rs:603-616`) with:

```rust
    // PIECES, AND ONLY PIECES (spec §3). The four flags this replaces
    // -- labels=, topo=, relief=, journeys= -- were a private
    // vocabulary that could reach only four of ten pieces. A malformed
    // piece name is a rejected query, not a silently smaller scene.
    let pieces = match p.get("pieces") {
        Some(spec) => PieceSet::parse(spec).ok()?,
        None => PieceSet::default_for_viewer(),
    };
```

In `page.html`'s `currentParams` (`page.html:306-309`), replace the four `p.set(...)` flag lines with:

```js
  // The piece vocabulary, sent whole. The checkboxes still read the
  // same way; they now name pieces instead of four private flags.
  const pieces = ["water", "fills", "borders", "claims", "markers", "chrome", "veil"];
  if ($("labels").checked) pieces.push("labels");
  if ($("journeys").checked) pieces.push("journeys");
  if ($("relief").checked) pieces.push("ground");
  p.set("pieces", pieces.sort().join(","));
```

**The `topo` checkbox:** `lib.rs:608` maps `topo=0` to removing `LayerSet::TOPOGRAPHY`, which Stage 1 Task 11's test maps to `Piece::Water`. Read Stage 1's delivered mapping and honour it: if `topo` means Water, add `if (!$("topo").checked) pieces.splice(pieces.indexOf("water"), 1);` and **rename the checkbox's visible label to "water"** in the same commit, because a control named `topo` that toggles the sea is exactly the kind of private vocabulary this stage exists to end. That rename is owner-visible; mention it in the commit, and if the owner objects, keep the label and add a one-line comment saying the name is historical.

In `contracts/runner/src/Steps.hs`, replace `sceneUrl`'s flag emission (`Steps.hs:26-30`) with `pieces=`:

```haskell
sceneUrl :: Text -> PieceSet -> Year -> StyleName -> Text
sceneUrl base ps (Year y) (StyleName st) =
  base <> "/api/scene?year=" <> tshow y <> "&zoom=90.0000&style=" <> st
       <> "&pieces=" <> renderPieceSet ps
```
with `renderPieceSet` emitting the same comma-joined sorted names `PieceSet::render` produces, and `"none"` for the empty set. **This is the change that makes the contract suite test what the spec says rather than what v0.1 could reach**, so read every scene scenario's verdict after it.

- [ ] **Step 5: Rebuild, restart, and read the fixture diffs before re-blessing anything**

```bash
cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
```
Some scene fixtures will change: `sceneUrl` now asks for exactly the named pieces where it used to ask for four flags' worth. **For each changed fixture, read the diff and say in one sentence why it changed** before re-blessing. A diff you cannot explain is a bug. In particular, `scene-1405-full` was blessed against a URL with `relief=1` (Ground on) and the equivalent `pieces=` list must contain `ground` — if it does not, `sceneUrl`'s translation is wrong.

Then re-bless only the fixtures you explained, bump `contracts/VERSION`'s MINOR per spec §4 (editing an existing blessed fixture is a breaking change) — **this is the stage's `0.5.0` bump; take it here, and Task 15 declares it rather than performs it** — and write the changelog entry.

- [ ] **Step 6: THE GOLDEN GATE — the single most dangerous gate in this plan**

Run the gate. Expected: `ALL GOLDEN VIEWS HOLD`.
**What is allowed to change: nothing.** Every one of the gate's 178 probes renders with the checkbox defaults, which Step 4 translates into `PieceSet::default_for_viewer()` — provably the same set the four flag defaults produced, by the Step 1 test. If a probe drifts, the translation is wrong in exactly one direction and the Step 1 test's `expected` set names which piece. Fix the translation.
**If the owner has approved a visual change here, this is the only place in the plan where re-blessing is even conceivable — and it still requires the approval in writing, naming the probes.**

- [ ] **Step 7: Both surface laws go GREEN**

```bash
node --test crates/map-viewer/tests/surface.test.mjs                 # PASS
cargo test -p map-viewer served_surface_is_exactly_the_contract      # PASS
```
**These two passing are spec §6's first two dogfood-bar checkables, met.** If either is still red, this stage is not done and Task 15 must not run.

- [ ] **Step 8: Commit**

```bash
git add crates/map-types/src/piece.rs crates/map-types/src/tests.rs \
        crates/map-viewer/src/lib.rs crates/map-viewer/src/page.html \
        contracts/runner/src/Steps.hs contracts/map-api/fixtures/ \
        contracts/VERSION contracts/CHANGELOG.md
git commit -m "The four flags die: pieces= is the only vocabulary, and both surface laws go green

The dogfood bar's first two checkables are met:
  the viewer fetches exactly the contract surface
  the server serves exactly the contract surface"
```

---

## Part C — `TopographyDress`, dress-locality's show-piece

Spec §3's corollary: *"`TopographyDress` (multi-stop land/bathymetry ramps, band structure, intensity) is Ground's dress under laws 3/4 — injectable, omittable, swappable with impunity. Ground takes a data-source parameter later without touching the algebra (out of scope now; signature leaves room)."*

Today, Ground's dress is `Style.topo: AgeRamp` (`style.rs:289`, `style.rs:316`) — two stops, `newest` and `oldest`, whose own doc comment admits the shape is borrowed: *"reusing the ramp shape"* (`style.rs:419`). It is consumed at exactly one place: `canon_provider.rs:291-297`, where a Relief-layer entity's band position `t` (from `relief_positions`, `canon_provider.rs:1156`, area rank normalised to `[0,1]`) selects `mix(ramp.oldest, ramp.newest, t)`.

**Two honest notes on the spec, to be settled with the owner in Task 12's gate, not silently:**

1. **Bathymetry.** Spec §3 puts the bathymetry ramp in *Ground's* dress, while `Water` is a separate piece. That is coherent if Ground means *the physical relief surface* — elevation bands above sea level and depth bands below — and Water means *the hydrographic fill* (seas, lakes, rivers as named waterbodies). This plan adopts that reading. **There are no below-sea-level relief bands in the canon today**, so the bathymetry ramp is declarable and unpopulated: exactly the "signature leaves room" case, and exactly what spec §7 means by keeping topography *data-source selection* out of scope. This plan lands the type, populates land, and populates nothing for sea.
2. **Band structure is currently a rank, not an elevation.** `relief_positions` orders bands by measured ring area, largest first, and calls the largest band position 0. That is honest — the comment at `canon_provider.rs:1152-1155` says so — but it means "band structure" today can only mean *how many bands and where the ramp is sampled*, not *which elevations they represent*. `BandStructure` is therefore an enum with one populated variant and room for the elevation-driven one; adding that variant is data-source work and is out of scope.

### Task 10: `TopographyDress` — a first-class type with laws, and the two-stop identity that keeps every pixel

**Files:**
- Create: `crates/map-types/src/topography.rs`
- Modify: `crates/map-types/src/lib.rs` (module declaration)
- Modify: `crates/map-types/src/tests.rs`

**Interfaces:**
- Consumes: `map_types::style::{Paint, Rgba}`.
- Produces:
```rust
// map-types::topography
#[derive(Clone, Debug, PartialEq)]
pub struct Ramp { stops: Vec<(f64, Paint)> }       // sorted, first at 0.0, last at 1.0
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RampError { Empty, Unsorted, NotSpanning, PositionOutOfRange }
impl Ramp {
    pub fn new(stops: Vec<(f64, Paint)>) -> Result<Ramp, RampError>;
    pub fn two(low: Paint, high: Paint) -> Ramp;    // total: cannot fail
    pub fn sample(&self, t: f64) -> Paint;          // total: t clamped to [0,1]
    pub fn stops(&self) -> &[(f64, Paint)];
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandStructure { ByMeasuredArea }           // room for ByElevation(..) — out of scope
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Intensity(f64);                          // [0.0, 1.0]
impl Intensity { pub fn new(v: f64) -> Option<Intensity>; pub const FULL: Intensity; pub fn get(self) -> f64; }
#[derive(Clone, Debug, PartialEq)]
pub struct TopographyDress { pub land: Ramp, pub sea: Ramp, pub bands: BandStructure, pub intensity: Intensity }
impl TopographyDress {
    pub fn classical() -> TopographyDress;
    pub fn from_two_stop(oldest: Paint, newest: Paint) -> TopographyDress;
    pub fn band_paint(&self, t: f64, ground: Paint) -> Paint;
}
```
Tasks 11 and 12 consume all of it.

- [ ] **Step 1: Write the failing tests — the laws, and the pixel identity**

Append to `crates/map-types/src/tests.rs`:

```rust
#[test]
fn a_ramp_is_a_lawful_multi_stop_thing() {
    use crate::style::{Paint, Rgba};
    use crate::topography::{Ramp, RampError};
    let p = |r, g, b| Paint { fill: Rgba(r, g, b, 255) };

    // Construction refuses everything dishonest, BY NAME.
    assert_eq!(Ramp::new(vec![]), Err(RampError::Empty));
    assert_eq!(Ramp::new(vec![(0.5, p(1,1,1)), (0.2, p(2,2,2))]), Err(RampError::Unsorted));
    assert_eq!(Ramp::new(vec![(0.2, p(1,1,1)), (1.0, p(2,2,2))]), Err(RampError::NotSpanning));
    assert_eq!(Ramp::new(vec![(0.0, p(1,1,1)), (0.5, p(2,2,2))]), Err(RampError::NotSpanning));
    assert_eq!(Ramp::new(vec![(0.0, p(1,1,1)), (1.5, p(2,2,2))]), Err(RampError::PositionOutOfRange));

    // A ramp reproduces its own stops EXACTLY -- no interpolation
    // rounding at a declared stop.
    let r = Ramp::new(vec![(0.0, p(10,10,10)), (0.5, p(200,100,50)), (1.0, p(250,250,250))]).unwrap();
    assert_eq!(r.sample(0.0), p(10,10,10));
    assert_eq!(r.sample(0.5), p(200,100,50));
    assert_eq!(r.sample(1.0), p(250,250,250));

    // sample is TOTAL: outside [0,1] clamps, never panics, never wraps.
    assert_eq!(r.sample(-3.0), r.sample(0.0));
    assert_eq!(r.sample(99.0), r.sample(1.0));
    assert_eq!(r.sample(f64::NAN), r.sample(0.0));

    // Monotone in each channel between adjacent stops -- a ramp that
    // doubled back would be a gradient nobody declared.
    for i in 0..100 {
        let (a, b) = (i as f64 / 200.0, (i + 1) as f64 / 200.0);
        let (ca, cb) = (r.sample(a).fill, r.sample(b).fill);
        assert!(cb.0 >= ca.0 && cb.1 >= ca.1, "ramp reversed between {a} and {b}");
    }
}

#[test]
fn a_two_stop_dress_is_byte_identical_to_the_ramp_it_replaces() {
    // THE PIXEL LAW. Every existing template declares a two-stop topo
    // ramp, and 89 blessed views were painted through
    // `mix(oldest, newest, t)` in canon_provider. This asserts the new
    // sampler IS that function on two stops -- not approximately, at
    // every band position the provider can produce. If this passes, the
    // golden gate cannot move when Task 11 swaps the machinery.
    use crate::style::{Paint, Rgba};
    use crate::topography::TopographyDress;
    let oldest = Paint { fill: Rgba(234, 224, 200, 255) };   // canaan.ron
    let newest = Paint { fill: Rgba(201, 180, 144, 255) };
    let dress = TopographyDress::from_two_stop(oldest, newest);

    // The reference: canon_provider::mix, transcribed verbatim.
    let mix = |a: Paint, b: Paint, t: f64| -> Paint {
        let c = |x: u8, y: u8| -> u8 {
            (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round().clamp(0.0, 255.0) as u8
        };
        let (Rgba(ar, ag, ab, aa), Rgba(br, bg, bb, ba)) = (a.fill, b.fill);
        Paint { fill: Rgba(c(ar, br), c(ag, bg), c(ab, bb), c(aa, ba)) }
    };

    // relief_positions emits i/(n-1) for n bands; cover every n the
    // canon could plausibly hold, and every i within it.
    for n in 2..=64usize {
        for i in 0..n {
            let t = i as f64 / (n as f64 - 1.0);
            assert_eq!(
                dress.band_paint(t, Paint { fill: Rgba(0, 0, 0, 0) }),
                mix(oldest, newest, t),
                "n={n} i={i} t={t}"
            );
        }
    }
}

#[test]
fn intensity_is_a_bounded_type_and_full_is_the_identity() {
    use crate::style::{Paint, Rgba};
    use crate::topography::{Intensity, TopographyDress};
    assert_eq!(Intensity::new(-0.1), None);
    assert_eq!(Intensity::new(1.1), None);
    assert_eq!(Intensity::new(f64::NAN), None);
    assert_eq!(Intensity::FULL.get(), 1.0);

    let ground = Paint { fill: Rgba(237, 227, 205, 255) };   // canaan's land
    let d = TopographyDress::from_two_stop(
        Paint { fill: Rgba(0, 0, 0, 255) }, Paint { fill: Rgba(255, 255, 255, 255) });
    // At FULL intensity the ground colour is irrelevant -- the ramp wins
    // outright, which is what today's provider does.
    assert_eq!(d.band_paint(0.5, ground), d.band_paint(0.5, Paint { fill: Rgba(9, 9, 9, 9) }));
    // At zero intensity the dress vanishes and the ground stands.
    let mut faint = d.clone();
    faint.intensity = Intensity::new(0.0).unwrap();
    assert_eq!(faint.band_paint(0.5, ground), ground);
}
```

- [ ] **Step 2: Run and verify FAIL**

Run: `cargo test -p map-types topography ramp intensity`
Expected: FAIL — module `topography` not found.

- [ ] **Step 3: Write `crates/map-types/src/topography.rs`**

```rust
//! GROUND'S DRESS (spec §3's corollary to laws 3 and 4). Topography is
//! not a special case in the renderer and not a set of tuned constants:
//! it is a dress, made of first-class parts with laws, injectable and
//! omittable and swappable like any other dress.
//!
//! What this replaces: `Style.topo: AgeRamp` -- two stops, borrowed
//! from the temporal-depth type, whose own doc comment said so
//! ("reusing the ramp shape"). Two stops cannot express a hypsometric
//! tint; a `Ramp` can.
//!
//! What it deliberately leaves room for and does NOT do (spec §7):
//! `sea` is a declarable bathymetry ramp with nothing to sample,
//! because the canon carries no below-sea-level bands; and
//! `BandStructure` has one variant because band position is measured
//! area rank today, not elevation. Both are data-source questions, and
//! data-source selection is out of scope. The signatures leave room;
//! the code makes no promise it cannot keep.
//!
//! Laws (crates/map-types/src/tests.rs): a Ramp is sorted, spans
//! [0, 1], reproduces its own stops exactly, is total under sampling
//! (clamped, NaN-safe), and is monotone between adjacent stops. A
//! two-stop dress at full intensity is BYTE-IDENTICAL to the
//! `mix(oldest, newest, t)` that painted the 89 blessed views.

use crate::style::{Paint, Rgba};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RampError {
    /// A ramp with no stops has no colour to give.
    Empty,
    /// Positions must ascend: a ramp that doubles back is a gradient
    /// nobody declared.
    Unsorted,
    /// A ramp must span the whole band range -- first stop at 0.0,
    /// last at 1.0 -- so `sample` is total without inventing ends.
    NotSpanning,
    PositionOutOfRange,
}

/// A multi-stop colour ramp over the normalized band range [0, 1].
#[derive(Clone, Debug, PartialEq)]
pub struct Ramp {
    stops: Vec<(f64, Paint)>,
}

impl Ramp {
    pub fn new(stops: Vec<(f64, Paint)>) -> Result<Ramp, RampError> {
        if stops.is_empty() {
            return Err(RampError::Empty);
        }
        if stops.iter().any(|(p, _)| !(0.0..=1.0).contains(p)) {
            return Err(RampError::PositionOutOfRange);
        }
        if stops.windows(2).any(|w| w[1].0 <= w[0].0) {
            return Err(RampError::Unsorted);
        }
        if stops.first().expect("non-empty").0 != 0.0 || stops.last().expect("non-empty").0 != 1.0 {
            return Err(RampError::NotSpanning);
        }
        Ok(Ramp { stops })
    }

    /// The two-stop ramp, TOTAL: it is the shape every template
    /// declared before this type existed, so it must not be fallible.
    pub fn two(low: Paint, high: Paint) -> Ramp {
        Ramp { stops: vec![(0.0, low), (1.0, high)] }
    }

    pub fn stops(&self) -> &[(f64, Paint)] {
        &self.stops
    }

    /// Total. Outside [0, 1] clamps to the ends; NaN takes the low end.
    /// A dress that could panic on a band position is not a dress.
    pub fn sample(&self, t: f64) -> Paint {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        let mut lo = &self.stops[0];
        for s in &self.stops {
            if s.0 <= t {
                lo = s;
            } else {
                let span = s.0 - lo.0;
                let u = if span <= 0.0 { 0.0 } else { (t - lo.0) / span };
                return mix(lo.1, s.1, u);
            }
        }
        lo.1
    }
}

/// Where a band sits in the ramp. One variant today, because band
/// position is MEASURED (ring area rank, canon_provider::relief_positions)
/// and not read from elevation data -- and choosing an elevation source
/// is out of scope (spec §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandStructure {
    ByMeasuredArea,
}

/// How strongly the ramp speaks over the ground beneath it. Bounded by
/// construction: an out-of-range intensity is unrepresentable, not
/// clamped in a renderer somewhere.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Intensity(f64);

impl Intensity {
    pub const FULL: Intensity = Intensity(1.0);
    pub fn new(v: f64) -> Option<Intensity> {
        (v.is_finite() && (0.0..=1.0).contains(&v)).then_some(Intensity(v))
    }
    pub fn get(self) -> f64 {
        self.0
    }
}

/// GROUND'S DRESS. Injectable, omittable, swappable (spec §3 laws 3/4).
#[derive(Clone, Debug, PartialEq)]
pub struct TopographyDress {
    /// Land bands, low to high.
    pub land: Ramp,
    /// Sea-floor bands, shallow to deep. Declarable now; nothing
    /// samples it until a depth source exists (spec §7).
    pub sea: Ramp,
    pub bands: BandStructure,
    pub intensity: Intensity,
}

impl TopographyDress {
    /// THE DECLARED CLASSICAL DEFAULT (spec §3 law 4). What a style
    /// that says nothing about topography wears.
    pub fn classical() -> TopographyDress {
        TopographyDress::from_two_stop(
            Paint { fill: Rgba(234, 224, 200, 255) },
            Paint { fill: Rgba(201, 180, 144, 255) },
        )
    }

    /// The bridge from the two-stop `AgeRamp` every template declares
    /// today. Argument order matches `AgeRamp`'s meaning at
    /// `style.rs:419`: oldest = lowest band, newest = highest.
    pub fn from_two_stop(oldest: Paint, newest: Paint) -> TopographyDress {
        TopographyDress {
            land: Ramp::two(oldest, newest),
            sea: Ramp::two(oldest, oldest),
            bands: BandStructure::ByMeasuredArea,
            intensity: Intensity::FULL,
        }
    }

    /// The paint for a band at normalized position `t`, over the ground
    /// it sits on. At FULL intensity this is exactly the ramp -- which
    /// is exactly what `canon_provider::mix` produced, and the reason
    /// the golden gate does not move when this type lands.
    pub fn band_paint(&self, t: f64, ground: Paint) -> Paint {
        let band = self.land.sample(t);
        match self.intensity.get() {
            i if i >= 1.0 => band,
            i => mix(ground, band, i),
        }
    }
}

/// Channel-wise linear blend, rounded. Transcribed from
/// `canon_provider::mix` (canon_provider.rs:1225) so that the pixel law
/// in tests.rs compares two implementations of the same arithmetic, not
/// one implementation with itself.
fn mix(a: Paint, b: Paint, t: f64) -> Paint {
    let c = |x: u8, y: u8| -> u8 {
        (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round().clamp(0.0, 255.0) as u8
    };
    let (Rgba(ar, ag, ab, aa), Rgba(br, bg, bb, ba)) = (a.fill, b.fill);
    Paint { fill: Rgba(c(ar, br), c(ag, bg), c(ab, bb), c(aa, ba)) }
}
```

Add `pub mod topography;` to `crates/map-types/src/lib.rs`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p map-types topography ramp intensity`
Expected: PASS, 3 tests.
**If `a_two_stop_dress_is_byte_identical_to_the_ramp_it_replaces` fails at any `n`/`i`, stop.** That test is the entire pixel-safety argument for Task 11. A rounding difference here is a real difference on screen.

- [ ] **Step 5: Commit**

```bash
git add crates/map-types/src/topography.rs crates/map-types/src/lib.rs crates/map-types/src/tests.rs
git commit -m "TopographyDress: Ground's dress as a first-class type, with the two-stop identity that keeps every pixel"
```

---

### Task 11: The dress wired through templates and provider — and not one pixel moves

**Files:**
- Modify: `crates/map-types/src/style.rs` (`StyleSpec.topo` / `Style.topo` become `TopographyDress`; `topo_ramp` → `topography`)
- Modify: `crates/map-viewer/src/templates.rs` (the `topography:` block in the file schema)
- Modify: `templates/canaan.ron`, `templates/parchment.ron`, `templates/slate.ron`
- Modify: `crates/map-provider/src/canon_provider.rs` (`area_paint`'s `Relief` arm)
- Modify: `crates/map-provider/src/tests.rs` (two `topo:` literals at `tests.rs:19` and `tests.rs:110`)

**Interfaces:**
- Consumes: `TopographyDress` (Task 10).
- Produces: `Style::topography(&self) -> &TopographyDress` replacing `Style::topo_ramp`. Task 12 consumes it.

- [ ] **Step 1: Write the failing test — the templates keep their pixels**

Append to `crates/map-viewer/src/templates.rs`'s test module:

```rust
#[test]
fn every_shipped_template_paints_the_same_bands_it_painted_before() {
    // The bridge test. Each shipped .ron declared a two-stop topo ramp;
    // after the migration each declares a `topography:` block. This
    // asserts the BANDS COME OUT THE SAME -- at every position the
    // provider can produce -- for every template in the book. It is the
    // reason the golden gate can be trusted after this task.
    use map_types::style::{Paint, Rgba};
    let before: &[(&str, Rgba, Rgba)] = &[
        // (template, oldest, newest) -- copied from the .ron files as
        // they stood at commit e05ea3d. Read them; do not trust this list.
        ("canaan",    Rgba(234, 224, 200, 255), Rgba(201, 180, 144, 255)),
        ("parchment", Rgba(0, 0, 0, 0),         Rgba(0, 0, 0, 0)),   // FILL IN from parchment.ron
        ("slate",     Rgba(0, 0, 0, 0),         Rgba(0, 0, 0, 0)),   // FILL IN from slate.ron
    ];
    let loaded = super::load_templates(std::path::Path::new("templates"));
    assert_eq!(loaded.len(), before.len(), "the style book changed size");
    for (name, style, _) in &loaded {
        let (_, oldest, newest) = before.iter().find(|(n, _, _)| n == name)
            .unwrap_or_else(|| panic!("no before-values recorded for template {name}"));
        let reference = map_types::topography::TopographyDress::from_two_stop(
            Paint { fill: *oldest }, Paint { fill: *newest });
        for n in 2..=64usize {
            for i in 0..n {
                let t = i as f64 / (n as f64 - 1.0);
                assert_eq!(
                    style.topography().band_paint(t, Paint { fill: Rgba(0, 0, 0, 0) }),
                    reference.band_paint(t, Paint { fill: Rgba(0, 0, 0, 0) }),
                    "template {name} band {i}/{n}"
                );
            }
        }
    }
}
```

**Before running this, open `templates/parchment.ron` and `templates/slate.ron` and copy their real `topo:` values into the `before` table.** The two placeholder rows are the only place in this plan where a value must be read out of a file rather than transcribed here, and they are marked so nobody mistakes them for finished. A test that runs against `Rgba(0,0,0,0)` placeholders would pass by comparing two wrongs.

- [ ] **Step 2: Run and verify FAIL**

Run: `cargo test -p map-viewer every_shipped_template_paints_the_same_bands`
Expected: FAIL — no method `topography` on `Style`.

- [ ] **Step 3: Implement the type change**

In `crates/map-types/src/style.rs`:
- `StyleSpec.topo: AgeRamp` (`style.rs:289`) → `pub topography: TopographyDress`.
- `Style.topo: AgeRamp` (`style.rs:316`) → `topography: TopographyDress`, and **delete the "reusing the ramp shape" comment** — the shape is no longer borrowed.
- `topo_ramp` (`style.rs:419-421`) →
  ```rust
      /// GROUND'S DRESS (spec §3's corollary). Was `topo_ramp`, a
      /// two-stop AgeRamp borrowed from temporal depth; now a dress
      /// with its own laws.
      pub fn topography(&self) -> &crate::topography::TopographyDress {
          &self.topography
      }
  ```
- `Style` currently derives `Copy` (`style.rs:307`) and `templates.rs`'s module doc says the whole tree must stay `Copy`. **`TopographyDress` owns a `Vec` and is not `Copy`.** Two ways out, and the choice is architectural:
  - **(a)** Drop `Copy` from `Style` and fix the call sites the compiler names. `app.style_values` is a `BTreeMap<StyleId, Style>` read by reference in most places; `lib.rs:1121` does `.copied()` and `canon_provider` takes `&Style`. Expect a handful of `.copied()` → `.cloned()` or `&`-borrow changes.
  - **(b)** Make `Ramp` fixed-capacity (`[(f64, Paint); 8]` plus a length), keeping `Copy`.
  **Take (a).** (b) is a capacity constant nobody declared — the owner's standing law forbids it, and eight is exactly the kind of number that becomes a bug in a year. Record the choice and its reason in the commit message; if the compiler cascade turns out to reach beyond `map-types`/`map-viewer`/`map-provider`/`map-encoders`, stop and report before pressing on.
- In `canon(...)` (`style.rs:471-472`), `self.topo.newest.canon(c); self.topo.oldest.canon(c);` becomes a walk over every stop of both ramps plus the intensity and the band variant. **The style id is a content hash, so this matters:** if the walk visits the same bytes for a two-stop dress as the old two-line walk did, existing style ids do not change, `/api/styles` keeps its blessed fixture, and no cached scene key moves. **Prefer keeping the bytes identical for the two-stop case** and appending the new fields only when they differ from classical — and if that is not cleanly expressible, let the ids change, re-bless `styles.json`, and say so loudly, because every style id in every blessed fixture moves with it.

In `crates/map-viewer/src/templates.rs`:
- Replace the `topo: TRamp` field of the template struct with `topography: TTopography`, and add:
  ```rust
  #[derive(serde::Deserialize)]
  #[serde(deny_unknown_fields)]
  struct TTopography {
      /// low band -> high band; at least two stops, positions ascending
      /// from 0.0 to 1.0.
      land: Vec<(f64, [u8; 4])>,
      sea: Vec<(f64, [u8; 4])>,
      intensity: f64,
  }
  ```
- In `build`, replace `topo: ramp(&t.topo)` (`templates.rs:200`) with a conversion that turns each list into a `Ramp` through `Ramp::new` and **fails the template by name** on a `RampError`, exactly as the honesty laws already do (`parse_template` returns `Err(String)`; `load_templates` panics with the file name). A dishonest ramp must refuse to serve, loudly, like every other dishonest dress.

In `templates/canaan.ron`, replace the `topo:` line with:
```ron
    // GROUND'S DRESS: land bands low -> high. Two stops here reproduce
    // the reference plate's hypsometric wash exactly; the type holds
    // more, and templates/terrain.ron shows what more looks like.
    topography: (
        land: [(0.0, (234, 224, 200, 255)), (1.0, (201, 180, 144, 255))],
        sea:  [(0.0, (234, 224, 200, 255)), (1.0, (234, 224, 200, 255))],
        intensity: 1.0,
    ),
```
and do the same in `parchment.ron` and `slate.ron` **with each file's own two colours**, read from the file.

In `crates/map-provider/src/canon_provider.rs`, replace the `LayerKind::Relief` arm (`canon_provider.rs:289-298`):
```rust
            LayerKind::Relief => {
                // Bands tint along GROUND'S DRESS by MEASURED order: a
                // lower band always encloses more area than the band
                // above it, so area rank IS elevation rank -- no name
                // parsing, no hashing. The dress decides how that rank
                // becomes colour; this code decides nothing about looks.
                let t = self.relief_pos.get(entity).copied().unwrap_or(0.0);
                style.topography().band_paint(t, style.region_paint())
            }
```
and update `crates/map-provider/src/tests.rs:19` and `:110`'s `topo: AgeRamp { … }` literals to `topography: TopographyDress::from_two_stop(…)` with the same two paints.

- [ ] **Step 4: Run everything green**

```bash
cargo test -p map-types
cargo test -p map-viewer every_shipped_template_paints_the_same_bands
cargo test --workspace
```
Expected: all pass. **If `map-provider`'s own tests move, read each diff**: those tests pin provider output, and a moved expectation here is a moved pixel there.

- [ ] **Step 5: Rebuild, restart, and check the style ids did not move**

```bash
curl -s "http://127.0.0.1:8090/api/styles"
```
Compare against `contracts/map-api/fixtures/styles.json`. If the ids changed, the `canon` walk changed the hash: re-bless `styles.json` and every scene fixture that carries a style id, bump the changelog, and **say so in the commit message in one sentence a non-programmer can act on**: "every saved link that names a dress by its id now names a different dress; old links fall back to the default."

- [ ] **Step 6: THE GOLDEN GATE**

Run the gate. Expected: `ALL GOLDEN VIEWS HOLD`.
**What is allowed to change: nothing.** Relief is opt-in (`PieceSet::default_for_viewer()` excludes Ground), so the gate's probes do not even render relief bands — but the `Copy` removal touched every style call site, and that is what could drift. If a probe moves, it is not the ramp: `git diff crates/map-provider` and look for a `.copied()` that became a different style lookup.

- [ ] **Step 7: Commit**

```bash
git add crates/map-types/src/style.rs crates/map-viewer/src/templates.rs templates/ \
        crates/map-provider/src/canon_provider.rs crates/map-provider/src/tests.rs
git commit -m "Ground wears TopographyDress: the borrowed AgeRamp retires, and the bands paint identically

Style is no longer Copy: TopographyDress owns its stops, and a
fixed-capacity ramp would be a capacity constant nobody declared."
```

---

### Task 12: Default-totality made real, and the show-piece: `terrain.ron` dresses only Ground

Spec §3 law 4 has two halves and only one is currently true anywhere: *"an omitted dress is the declared classical default; **a style may dress any subset of pieces**."* Today `templates.rs` uses `#[serde(deny_unknown_fields)]` with every field required, so a template must dress everything or fail to load. Diagnosis §9.1 records law 4 as having no scenario at all, and Stage 1 Task 7 gave it its first one. This task makes the law true and then exhibits it.

**Files:**
- Modify: `crates/map-types/src/style.rs` (classical `Default` impls for the remaining dress sub-types)
- Modify: `crates/map-viewer/src/templates.rs` (template fields become optional)
- Create: `templates/terrain.ron`
- Modify: `contracts/map-api/scene/scene.feature`, `contracts/map-api/scene/styles.feature`
- Re-bless: `contracts/map-api/fixtures/styles.json`

> **STOP. OWNER GATE — before Step 5.**
>
> This adds a fourth dress to the workbench's style row, called **terrain**. It shows the land in graded elevation colours — low ground green, high ground brown, peaks pale — instead of one flat cream. It does not become the default; **canaan** stays the dress you see when you open the workbench, and the 89 golden views are untouched.
>
> What it is *for*: it is the proof that a dress can change one thing and nothing else. It dresses only the ground; every other part of the map — borders, labels, water, journeys — falls back to the standard look, automatically, because nobody wrote them into the file. That is the promise the whole refactor rests on, made visible.
>
> Two questions for you: **(1)** do you want a fourth dress in the row at all, or should this ship as a test fixture the workbench never shows? **(2)** the elevation colours below are a first attempt against the reference plate's palette — approve them, or send a reference and they get matched.

**Interfaces:**
- Consumes: `TopographyDress` (Task 10), `Style::topography` (Task 11), `Style::dresses` (Task 3).
- Produces: `Style::dresses` becomes honest — it returns the pieces the template actually declared, not `PieceSet::all()`.

- [ ] **Step 1: Write the failing test — a template may dress a subset**

Append to `crates/map-viewer/src/templates.rs`'s test module:

```rust
#[test]
fn a_template_may_dress_any_subset_and_the_rest_is_the_declared_classical_default() {
    // SPEC §3 LAW 4, both halves, as one whole-body assertion: a
    // template that declares ONLY topography must equal the classical
    // default in EVERY other field. Not "loads without error" -- equal,
    // field for field, so a wrong default cannot hide behind a
    // successful parse.
    use map_types::style::{Paint, Rgba};
    let bare = r#"( topography: (
        land: [(0.0, (10, 60, 20, 255)), (1.0, (240, 240, 235, 255))],
        sea:  [(0.0, (10, 30, 60, 255)), (1.0, (10, 30, 60, 255))],
        intensity: 1.0,
    ) )"#;
    let (dressed, is_default) = super::parse_template(bare).expect("a subset dress loads");
    assert!(!is_default, "a template that says nothing about being default is not default");

    let classical = super::parse_template("()").expect("the empty dress is the classical default");
    // Everything except topography matches the classical default...
    assert_eq!(dressed.region_paint(), classical.0.region_paint());
    assert_eq!(dressed.water_paint(),  classical.0.water_paint());
    assert_eq!(dressed.chrome(),       classical.0.chrome());
    assert_eq!(dressed.labeling(),     classical.0.labeling());
    assert_eq!(dressed.age_ramp(),     classical.0.age_ramp());
    assert_eq!(dressed.paper(),        classical.0.paper());
    // ...and topography does not.
    assert_ne!(dressed.topography(), classical.0.topography());
    assert_eq!(dressed.topography().land.sample(0.0), Paint { fill: Rgba(10, 60, 20, 255) });

    // DRESSES is now honest: this template reaches Ground and nothing else.
    use map_types::piece::{Piece, PieceSet};
    assert_eq!(dressed.dresses(), PieceSet::empty().with(Piece::Ground));
    assert_eq!(classical.0.dresses(), PieceSet::empty(),
               "a template that declares nothing dresses nothing -- it IS the default");
}
```

Adaptation note: `parse_template` returns `Result<(Style, bool), String>` (`templates.rs:270`), so `classical.0` is the `Style`. The accessor list above must match the real ones on `Style`; read `style.rs:405-440` and cover **every** accessor, not the six above — a whole-body assertion that checks six of twenty fields is a poke wearing a law's name. The six are the shape; expand them.

- [ ] **Step 2: Run and verify FAIL**

Run: `cargo test -p map-viewer a_template_may_dress_any_subset`
Expected: FAIL — `template schema: missing field boundaries`.

- [ ] **Step 3: Implement default-totality in the template schema**

In `crates/map-types/src/style.rs`, add classical `Default` impls for every dress sub-type that lacks one. Three already exist (`GlobeChrome`, `GhostDress`, `PatternGeometry`, `style.rs:225-269`) with the doc comment *"The classical reference values — what every template declared the day these became data."* Follow it exactly: add `Default` for `BoundaryStrokes`, `Labeling`, `LabelStyle`, `LabelScale`, `TypeVoice`, `MarkerStyle`, `DeltaEmphasis`, `AgeRamp`, and add `StyleSpec::classical()` assembling them plus `region`, `water`, `palette: None`, `paper`, `tint_alpha`, `river_width`, `topography: TopographyDress::classical()`. **Take every value from `templates/canaan.ron`**, which is the reference plate's own dress — and say so in the doc comment, so the origin of each number is on the page rather than in someone's memory.

In `crates/map-viewer/src/templates.rs`, make every field of the `Template` struct `Option<_>` and, in `build`, resolve each as `t.field.map(convert).unwrap_or(classical.field)`. Keep `deny_unknown_fields`: a *misspelled* field must still refuse to serve, loudly, by name — that is a different thing from an omitted one, and conflating them would make law 4 a licence for typos.

Then make `Style::dresses` honest. It needs to know which fields the template declared, which `Style` itself cannot know — so `build` computes it and `StyleSpec` carries it:
```rust
    /// Which pieces this template actually dressed (spec §3 law 4).
    /// Computed where the knowledge lives -- in the loader that saw the
    /// file -- never guessed from the values, because a template may
    /// legitimately declare a field whose value equals the default.
    pub dresses: crate::piece::PieceSet,
```
and in `templates.rs`'s `build`, accumulate it:
```rust
    let mut dresses = PieceSet::empty();
    if t.topography.is_some() { dresses = dresses.with(Piece::Ground); }
    if t.water.is_some() || t.river_width.is_some() { dresses = dresses.with(Piece::Water); }
    if t.region.is_some() || t.palette.is_some() { dresses = dresses.with(Piece::Fills); }
    if t.boundaries.is_some() { dresses = dresses.with(Piece::Borders).with(Piece::Claims); }
    if t.labeling.is_some() { dresses = dresses.with(Piece::Labels); }
    if t.marker.is_some() { dresses = dresses.with(Piece::Markers); }
    if t.delta.is_some() { dresses = dresses.with(Piece::Journeys); }
    if t.chrome.is_some() || t.paper.is_some() { dresses = dresses.with(Piece::Chrome); }
    if t.ghost.is_some() || t.tint_alpha.is_some() { dresses = dresses.with(Piece::Veil); }
```
and `Style::dresses` returns the carried set. **The `age` field maps to no piece** — it is temporal depth, which rides Fills; fold it into `Fills` and note that in the code, so the mapping is total over the template's fields and a reviewer can check it is.

- [ ] **Step 4: Verify the shipped templates are unchanged**

Run: `cargo test --workspace`
Then check `/api/styles`: all three shipped templates declare every field, so each must still report `dresses` = all ten. **The Task 3 test `every_loaded_style_declares_what_it_dresses` asserts exactly that and must still pass** — if it fails, the `dresses` accumulation above missed a field.

- [ ] **Step 5: Create the show-piece — but only after the owner gate above is answered**

Create `templates/terrain.ron`:

```ron
// THE SHOW-PIECE for spec §3 laws 3 and 4. This file declares ONE
// thing: Ground's dress. Everything else -- borders, labels, water,
// journeys, chrome, the veil -- is absent, and therefore is the
// declared classical default. That is default-totality (law 4) as a
// file you can read, and dress-locality (law 3) as a thing you can see:
// switch to this dress and only the ground changes.
//
// The ramp is hypsometric: low ground green, mid ground tan, high
// ground brown, peaks pale. Five stops -- a shape the two-stop AgeRamp
// this replaced could not express at all.
(
    topography: (
        land: [
            (0.00, (108, 140,  92, 255)),   // lowland green
            (0.30, (186, 178, 126, 255)),   // dry tableland
            (0.60, (176, 138,  92, 255)),   // hill brown
            (0.85, (146, 106,  74, 255)),   // upland
            (1.00, (238, 234, 228, 255)),   // peak, pale
        ],
        // Bathymetry is DECLARABLE and unsampled: the canon carries no
        // below-sea-level bands, and choosing a depth source is out of
        // scope (spec §7). One flat stop, honestly stated.
        sea:  [(0.0, (124, 157, 201, 255)), (1.0, (124, 157, 201, 255))],
        intensity: 1.0,
    ),
)
```

- [ ] **Step 6: The scenarios that prove the algebra — injectable, omittable, swappable**

Append to `contracts/map-api/scene/scene.feature`:

```gherkin
  Scenario: dress-locality, swappable — a new dress changes Ground and nothing else
    When I render pieces ground, fills, borders, labels at year -1405 in style canaan as plate
    And I render pieces ground, fills, borders, labels at year -1405 in style terrain as terrained
    Then plate and terrained differ only in dress, never in geometry

  Scenario: dress-locality, omittable — with Ground omitted the two dresses agree entirely
    When I render pieces fills, borders, labels at year -1405 in style canaan as plate
    And I render pieces fills, borders, labels at year -1405 in style terrain as terrained
    Then plate equals terrained

  Scenario: default-totality — what terrain does not dress, it wears classically
    When I GET /api/styles as book
    Then book equals fixture "styles"
```

The second scenario is the show-piece's real proof and it is a **negative** case: `terrain` dresses only Ground, so with Ground omitted the two renders must be *byte-identical*, not merely similar. A server that leaked the terrain dress into fills or labels fails it. A server that ignored `style=` entirely would pass it — which is why the first scenario, requiring the two to *differ*, stands beside it. Neither alone is a check; together they are.

- [ ] **Step 7: Re-bless `styles.json`, and confirm the default did not move**

```bash
cd contracts/runner
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api --bless styles
```
The blessed fixture now has four entries. **Verify `canaan` still carries `"default": true` and `terrain` carries `"dresses": ["ground"]`.**

**Task 3's `every_loaded_style_declares_what_it_dresses` now fails, and it is supposed to.** It asserted `d == PieceSet::all()` for every loaded style, with a comment saying *"Task 11 makes this vary; this assertion is what will notice when it does."* It has noticed. Rewrite it, keeping the whole-body shape rather than weakening it to a poke:

```rust
#[test]
fn every_loaded_style_declares_what_it_dresses() {
    use map_types::piece::{Piece, PieceSet};
    let app = test_app();
    // The whole book, pinned: which dress reaches which pieces is a
    // published fact (styles() serves it), so it is asserted entirely,
    // not sampled.
    let book: Vec<(&str, String)> = app
        .styles
        .iter()
        .map(|(name, id)| {
            (*name, app.style_values.get(id).expect("a loaded style").dresses().render())
        })
        .collect();
    let all = PieceSet::all().render();
    assert_eq!(
        book,
        vec![
            ("canaan", all.clone()),
            ("parchment", all.clone()),
            ("slate", all),
            ("terrain", PieceSet::empty().with(Piece::Ground).render()),
        ]
    );
}
```
Note the ordering: `app.styles` is default-first then alphabetical (`lib.rs:266`), and `canaan` is the declared default, so the expected order is canaan, parchment, slate, terrain. If the book's order differs, Step 7's `canaan_is_and_remains_the_declared_default_dress` test will say why before this one does. The default is declared in the `.ron` (`default: true`), not alphabetical (`templates.rs:280-283`, `lib.rs:262-266`) — and `terrain.ron` declares nothing, so it cannot claim it. Add a Rust test pinning it anyway, because the claim now matters to four files:
```rust
#[test]
fn canaan_is_and_remains_the_declared_default_dress() {
    let app = test_app();
    assert_eq!(app.styles.first().map(|(n, _)| *n), Some("canaan"),
               "the default dress moved -- 89 blessed views are painted in canaan");
}
```

- [ ] **Step 8: THE GOLDEN GATE**

Run the gate. Expected: `ALL GOLDEN VIEWS HOLD`.
**What is allowed to change: nothing.** A fourth template joins the book but does not become the default; `canaan` still declares every field, so its `Style` is byte-identical. If a probe drifts, either the default moved (Step 7's test says so) or the optional-field conversion changed one of canaan's values — diff `parse_template("<canaan source>")`'s output against the previous commit's.

- [ ] **Step 9: Commit**

```bash
git add crates/map-types/src/style.rs crates/map-viewer/src/templates.rs templates/terrain.ron \
        contracts/map-api/scene/scene.feature contracts/map-api/scene/styles.feature \
        contracts/map-api/fixtures/styles.json contracts/CHANGELOG.md
git commit -m "Default-totality made real, and TopographyDress shown: terrain dresses only Ground

Owner gate answered: <record the two answers>"
```

---

## Part D — bind the tiers, and close

### Task 13: Derivability — the last `@target` retires, and the two tiers stop being tested in isolation

Diagnosis §9.1 item 2: spec §4 requires *"every manifest entry must be traceable to `disposition` + `borders` answers — contract-tested by sampling"*, and there was no scenario. Stage 1 Task 7 wrote it as a declared `@target` waiting on Stages 2 and 3. **Stage 4 is the last stage; if derivability does not go green here, the contract never binds its two tiers, and diagnosis §9.1 calls this "arguably the most important omission on the list".**

**`[STAGE-2 DEP]` `[STAGE-3 DEP]`:** needs `GET /api/disposition?entity=&at=` and `GET /api/borders?entity=&at=`. Read both plans first; adapt the parameter spelling below to what they delivered.

**One deliberate strengthening of the spec's own word.** Spec §4 says "by sampling". A sample can miss, and a check that can miss the failure mode is not a check (§7.0). This task therefore asserts **totality** at blessed instants — *every* entry traces — and uses sampling only in the `@property` scenario over generated years, where whole-body pinning is impossible by construction. That satisfies the spec's requirement and the owner's decree together, rather than trading one for the other.

**Files:**
- Modify: `contracts/map-api/scene/scene.feature` (retire the `@target`)
- Modify: `contracts/runner/src/Steps.hs` (the tracing step)
- Create: `contracts/map-api/fixtures/derivability-1405.json`

- [ ] **Step 1: Rewrite the derivability scenario, whole-body**

Replace Stage 1's `@target` derivability scenario in `contracts/map-api/scene/scene.feature` with:

```gherkin
  Scenario: derivability — every manifest entry traces to a disposition and a border
    When I render pieces fills, borders at year -1405 in style canaan as plate
    Then every entry in plate traces to disposition and borders at year -1405
    And the trace of plate equals fixture "derivability-1405"

  @property
  Scenario: derivability holds at any year
    When I render pieces fills, borders at year <someYear> in style canaan as plate
    Then every entry in plate traces to disposition and borders at year <someYear>
```

The first scenario has two Thens on purpose: the first is the law (totality — nothing is free-floating geometry), the second pins the *whole trace* so that a change in which entry maps to which disposition is a change to a published promise, not a silent re-parenting.

- [ ] **Step 2: Implement the step**

In `contracts/runner/src/Steps.hs`:

```haskell
  -- DERIVABILITY (spec §4): the join between the scene tier and the
  -- fact tier. An entry that cannot be traced is free-floating
  -- geometry, which the architecture does not permit. Note this asserts
  -- TOTALITY, not a sample: the spec says "by sampling", but a sample
  -- that misses is not a check (diagnosis §7.0). The @property
  -- scenario is where sampling belongs -- over YEARS, not entries.
  , step "every entry in <name> traces to disposition and borders at year <year>" $ \nm y w -> do
      body <- bound nm w
      let entries = body ^.. key "features" . values
      when (null entries) $ failWith "an empty manifest traces vacuously -- that is not a pass"
      untraceable <- fmap catMaybes . forM entries $ \e -> do
        let ent = e ^? key "entity" . _String
        case ent of
          Nothing -> pure (Just (e ^? key "id" . _String, "entry names no entity"))
          Just eid -> do
            d <- getUrl (baseUrl w <> "/api/disposition?entity=" <> eid <> "&at=" <> tshow y) w
            b <- getUrl (baseUrl w <> "/api/borders?entity="     <> eid <> "&at=" <> tshow y) w
            pure $ if isJust d && isJust b then Nothing
                   else Just (Just eid, "no disposition or no borders")
      unless (null untraceable) $
        failWith $ "untraceable entries: " <> tshow (take 20 untraceable)
                   <> " (of " <> tshow (length untraceable) <> ")"
  , step "the trace of <name> equals fixture <fixture>" $ \nm fx w -> do
      body <- bound nm w
      let trace = sort [ (e ^? key "piece" . _String, e ^? key "entity" . _String)
                       | e <- body ^.. key "features" . values ]
      blessOrCompare fx (toJSON trace) w
```

Adaptation notes, all load-bearing:
- `key "entity"` — Stage 1 stamps `piece` on manifest entries; whether it also stamps `entity` is a Stage 1 question. **Read the delivered `manifest_json`.** If entries carry no entity id, this task has real work first: add it, additively, in `crates/map-encoders/src/gpu.rs`, with its own failing test — and note that this is the derivability clause's actual cost, discovered here rather than assumed away.
- `getUrl` must return `Nothing` on a non-2xx. Stage 0 item 4 in diagnosis §7 fixed exactly this ("an HTTP transport that returned error pages as success"); confirm it is still true, because this step's whole discrimination rests on it.
- The `when (null entries)` guard is the vacuity guard. Diagnosis §6.3 records a piece-attribution step that "passed vacuously on an empty manifest"; do not repeat it.

- [ ] **Step 3: Run and verify RED, then green**

```bash
cd contracts/runner
cabal run contract-runner -- check ../map-api        # totality: every step defined
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
```
Expected first: red, naming untraceable entries. **Read the list before fixing anything** — if it is short and specific, the finding is about those entities; if it is every entry, the manifest carries no entity ids and Step 2's first adaptation note applies.
Then bless `derivability-1405` and re-run to green.

- [ ] **Step 4: Confirm no `@target` remains**

```bash
grep -rn "@target" contracts/map-api/
```
Expected: **no matches.** Every law the corpus predicted would fail now holds.
 If any `@target` survives, name it in the commit message and carry it into Task 15's report as debt the v1.0 declaration must weigh.

- [ ] **Step 5: Commit**

```bash
git add contracts/map-api/scene/scene.feature contracts/runner/src/Steps.hs \
        contracts/map-api/fixtures/derivability-1405.json contracts/CHANGELOG.md
git commit -m "Derivability: the last @target retires, and the two tiers are checked against each other"
```

---

### Task 14: The four strata swept, the census stable, the CDC re-verified

Spec §6's remaining dogfood-bar checkables: *contract suite green against the server; atlas-edge CDC green against `:8080` and handed to the atlas session; census stable; golden gate green.* This task gathers the evidence. It writes no production code; it may only fix what it finds red, and if it finds something red that it cannot fix in one small change, **it stops and reports** rather than growing.

- [ ] **Step 1: Stratum 1 — Rust unit and law tests**

```bash
cargo test --workspace 2>&1 | tee /tmp/stage4-rust.out
grep -c "test result: ok" /tmp/stage4-rust.out
grep -c "test result: FAILED" /tmp/stage4-rust.out
```
Expected: the ok count is at least Stage 0's 19 (this stage adds tests, so it may be higher); the FAILED count is `0`. Record both numbers.

- [ ] **Step 2: Stratum 1 — the client-side laws**

```bash
node --test crates/map-viewer/tests/
```
Expected: every file passes, including `surface.test.mjs` (the dogfood bar), `stops.test.mjs`, `mapped-times.test.mjs`, `compose.test.mjs`, `golden.test.mjs`, and Stage 0's `limb.test.mjs`.

- [ ] **Step 3: Stratum 2 — the contract suite, and its static gates**

```bash
cd contracts/runner
cabal test
cabal run contract-runner -- check ../map-api
cabal run contract-runner -- check ../atlas-edge
cabal run contract-runner -- vocab ../map-api
cabal run contract-runner -- vocab ../atlas-edge
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api    | tee ../../map-api.out
cabal run contract-runner -- run --base-url http://127.0.0.1:8080 ../atlas-edge | tee ../../atlas-edge.out
```
Expected: `cabal test` green; both `check`s report `totality: every step has exactly one definition`; both `vocab`s report `vocabulary: every table matches its types`; **map-api all green with zero `@target`s**; **atlas-edge 6/6 green**.

Two things to check that a green tally does not tell you:
- The `vocab` exemption from diagnosis §9.2 — three features (`census`, `changes`, `subjects`) carry no `Vocabulary:` block and pass vacuously. Stage 1 Task 7 was to make that exemption explicit and counted. **Confirm the count is stated in the output.** If `vocab` still reports a blanket green over unexamined ground, that is §7.0's shape again and it goes into Task 15's report as debt.
- The hole-distinctness law (Stage 1 Task 5) must be running over this stage's new `@property` scenarios. Confirm `check` names how many property holes it examined.

- [ ] **Step 4: Stratum 2 — the atlas hand-off, confirmed not repeated**

Diagnosis §8.3 recommended handing the CDC suite over early, and **Stage 1 Task 17 Step 4 does it.** Do not repeat it. Confirm:
```bash
git log --oneline --all --grep="atlas-edge" | head
```
Read Stage 1's closing report for the binary path and the hand-off confirmation. Then re-verify the current suite is green against `:8080` (Step 3 above did) and record it. **If Stage 1's report shows the hand-off did not happen**, perform it now:
```bash
cd contracts/runner && cabal build && cabal list-bin contract-runner
# the atlas session runs, with no access to this workspace and no Rust toolchain:
#   contract-runner run --base-url http://127.0.0.1:8080 <path-to>/contracts/atlas-edge
```
and report to the owner: the binary path, the `contracts/atlas-edge/` directory, that all six pass today, and the request that the atlas session adopt it in their CI. **Do not edit the suite. Do not touch the atlas repo.**

- [ ] **Step 5: Stratum 3 — the census is STABLE**

Spec §6's checkable is "census stable", and this stage changed no identity, no ledger row and no border. The diff must be **empty**, at every instant Stages 1–3 sampled:
```bash
for Y in -1446 -1405 -1050 -586 59; do
  echo "=== $Y"
  curl -s "http://127.0.0.1:8090/api/census?year=$Y" -o "census-stage4-$Y.json"
  curl -s "http://127.0.0.1:8090/api/census?year=$Y&to=$Y" | head -c 200; echo
done
git show HEAD~40:crates/... # -- see note
```
Since Stage 3's closing census files are the reference, take them from that stage's commit rather than re-deriving:
```bash
git log --oneline --grep="Stage 3 closes" -1     # find Stage 3's closing commit
# then diff this stage's captures against the census artifacts that commit recorded
```
**Expected: an empty diff at every instant.** A non-empty diff means Stage 4 moved a fact while claiming to move only routes and dress — that is a bug, it stops the stage, and it goes to the owner before anything else.
Adaptation note: Stage 1 built `/api/census?year=A&to=B` as the diff instrument. Use it directly if Stage 3 recorded the reference years; the shell above is the fallback for when it did not.

- [ ] **Step 6: Stratum 4 — the golden gate, final**

Run (from `C:\Users\donov\.claude\jobs\c6946bce\tmp`): `node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -3`
Expected: `ALL GOLDEN VIEWS HOLD`, 89/89, **with Task 1's stop-set guard silent**. Note explicitly in the record that the guard ran and found nothing — a gate that examined the full stop set is a different claim from a gate that printed a green line.

- [ ] **Step 7: `contract()` reports the current 0.x**

```bash
curl -s "http://127.0.0.1:8090/api/contract"
cat contracts/VERSION
```
Expected: both say `0.5.0` (bumped in Task 9 Step 5). The route reads `include_str!("../../../contracts/VERSION")` at `lib.rs:892`, so a stale binary is the only way these can disagree — if they do, rebuild and restart, and note that the version is baked at compile time.

- [ ] **Step 8: Commit the evidence**

```bash
git add map-api.out atlas-edge.out census-stage4-*.json
git commit -m "Stage 4's evidence: four strata swept, census empty-diff, golden 89/89, contract() at 0.5.0"
```
If the repo's `.gitignore` excludes these, put them under `docs/notes/` with a dated name instead — the evidence must be in the repository, because Task 15 asks the owner to make a decision on it.

---

### Task 15: The dogfood bar declared at v0.5.0 — and v1.0 put to the owner

**This task makes no v1.0 declaration and must not.** Spec §4: *"v1.0.0 is a human act by the owner, tagged when the contract is ready for the atlas to lean on — never automatic."* Spec §7 lists v1.0 as out of scope. This task's entire purpose is to put the decision, and the evidence for it, in the owner's hands, and then stop.

**Files:**
- Modify: `contracts/CHANGELOG.md`
- Create: `docs/notes/2026-09-07-dogfood-bar.md`

- [ ] **Step 1: Check every dogfood-bar checkable, one line each, with its evidence**

Write `docs/notes/2026-09-07-dogfood-bar.md`. Every row cites the command that produced it — a claim without a command is an assertion, and this document exists to replace assertions with evidence:

```markdown
# The dogfood bar (spec §6, Stage 4 exit)

| checkable | verdict | evidence |
|---|---|---|
| viewer entirely on contract functions | | `node --test crates/map-viewer/tests/surface.test.mjs` |
| 13 legacy routes deleted | | `cargo test -p map-viewer served_surface_is_exactly_the_contract` + the route census below |
| contract suite green against the server | | `contract-runner run --base-url http://127.0.0.1:8090 ../map-api` |
| atlas-edge CDC green against :8080 and handed to the atlas session | | `contract-runner run --base-url http://127.0.0.1:8080 ../atlas-edge`; hand-off in Stage 1 Task 17 |
| census stable | | Task 14 Step 5's empty diff at -1446, -1405, -1050, -586, 59 |
| golden gate green | | `node golden.js --check` — 89/89, stop-set guard silent |
| `contract()` reports the current 0.x | | `curl /api/contract` → 0.5.0 |
```

**On "13 legacy routes deleted", state the reconciliation plainly and put the question to the owner.** The spec's 13 was the `/api/*` surface the day the spec was written (`a81fbf3`); Stage 0 added two. Six were deleted (`meta`, `overlay`, `region_times`, `render`, `scaffold`, `features`) plus the entity-id `pieces=` branch; nine survive as named contract functions with laws (`resource`, `resources`, `contract`, `census`, `subjects`, `changes`, `transition`, `entities`, `scene`) alongside the ones Stages 1–3 added. **Ask the owner directly: is the bar met?** The plan's reading is that it is — the legacy *surface* is retired, which is what the stage was for — but the number in the spec is not the number on the ground, and that is the owner's to accept or refuse.

- [ ] **Step 2: Write the v0.5.0 changelog entry as a declaration**

Prepend to `contracts/CHANGELOG.md` a `## 0.5.0` section. Write it as the owner's statement of what changed, not as a commit log: six legacy routes deleted and the viewer moved wholly onto contract functions; `styles()` added; the four legacy piece flags replaced by `pieces=`; `TopographyDress` replacing the borrowed two-stop ramp; default-totality made real so a style may dress any subset; derivability green and no `@target` remaining; the golden gate repaired to refuse a changed stop set.

- [ ] **Step 3: Put v1.0 to the owner — with what they need to decide, and what they would be deciding against**

Report, in plain language:

**What the contract now guarantees.** All four scene-algebra laws hold, checked. Both tiers are bound by derivability. The viewer runs on nothing but contract functions. 89 blessed views are unchanged through the whole stage.

**What v1.0 would mean.** Spec §4: the contract is ready for the atlas to lean on. After it, a breaking change to any feature or blessed fixture is a MAJOR bump with everything that implies for a consumer. Before it, the 0.x regime lets breaking changes ride a MINOR.

**What is still open, and would be frozen by declaring.** From spec §7 and the diagnosis:
- the write API — mutation-ready types only, no write path exists;
- topography data-source selection — `TopographyDress.sea` is declarable and unsampled, `BandStructure` has one variant;
- the spec's true `⊕` — composition is checked as resource-set union, never as an operator with associativity and identity, because there is no server-side combine (diagnosis §6.1, §8.5);
- `@deprecated` warts frozen at v0.1 that no stage has revisited (spec §8);
- any `@target` still standing (Task 13 Step 4 should have found none — if it found one, name it here);
- any vacuous `vocab` exemption still unstated (Task 14 Step 3).

**The question, asked plainly:** *is the contract ready for the Bible atlas to build against, knowing that after you say yes, changing it breaks them?* And name the one piece of evidence they cannot get from a table: nobody has yet built anything against this contract except us.

- [ ] **Step 4: Commit — and STOP**

```bash
git add contracts/CHANGELOG.md docs/notes/2026-09-07-dogfood-bar.md
git commit -m "Stage 4 closes at v0.5.0: the dogfood bar met, and v1.0 put to the owner

v1.0 is the owner's declaration and is NOT taken here."
```

**Do not tag v1.0. Do not move `contracts/VERSION` to `1.0.0`. Do not begin any of the out-of-scope work below.** Report to the owner and stop.

---

## After Stage 4: what remains, and what this plan does not authorise

This is the last planned stage. Everything below is outside it. **This plan does not authorise crossing any of these lines**, and a subagent that finds itself doing any of it has left the plan.

### 1. The v1.0 declaration — the owner's act, and only the owner's

Spec §4: *"v1.0.0 is a human act by the owner, tagged when the contract is ready for the atlas to lean on — never automatic."* Spec §7 lists it as out of scope. Spec §5 Stage 4: *"Pre-release continues until the owner's v1.0 declaration."*

The criteria are the owner's, not this plan's. What Task 15 hands them is: the dogfood-bar table with its commands, the four strata's numbers, the empty census diff, the 89 golden views, and the list of what declaring would freeze. What no artifact can give them is the one fact that matters most — **nothing outside this workspace has been built against this contract yet.** The atlas session has the CDC suite for the edge *we consume*; nobody has yet consumed *ours*. That asymmetry is the strongest argument for waiting, and the owner should hear it stated rather than discover it.

### 2. Spec §7's out-of-scope list, unchanged

- **The write API.** Mutation-ready types only; no mutation code, no route, no test. The fact graph's six node types were designed to take writes; none of that is exercised.
- **Topography data-source selection.** `TopographyDress.sea` is a declarable bathymetry ramp with nothing to sample; `BandStructure` has one variant (`ByMeasuredArea`) because band position is measured ring-area rank, not elevation. Spec §3 says Ground *"takes a data-source parameter later without touching the algebra"* — the signature leaves room, and this plan wrote no code toward it.
- **Atlas-side implementation.** Theirs. The atlas repo is a read-only path dependency; the CDC suite was handed over, and whether they adopt it is their session's call.
- **The upstream PRs themselves.** The upstreamability law is contract-tested on our side (Stage 1 Task 16); actually contributing our node types into the Bible atlas is a future act and touches their repo.

### 3. Debt parked by earlier stages, still standing

Carried forward so it is not rediscovered as a surprise:

- **The spec's true `⊕`.** Composition is checked as resource-set union, not as an associative operator with an identity, because there is no server-side combine (diagnosis §6.1). Stage 1's own self-review names this as deliberately left open, and diagnosis §8.5 argues against building a combine before the packaging bug it would have sat on top of was fixed. That bug is fixed now, and Task 6 uses a *client-side* ⊕. **Whether the server should grow one is now a live question with better evidence behind it than it had at Stage 1** — and it is the first thing to reconsider after v1.0, not before.
- **Feature preambles are unverified prose** (diagnosis §9.2). The drift law protects the `Vocabulary:` table; nothing protects the English above it, and three atlas-edge preambles drifted from their own projections inside a single stage. Since the atlas-edge suite is an artifact handed to another team, its prose is part of its interface. No stage has proposed a law for this, and inventing one is not obviously a good idea.
- **The `vocab` exemption.** Three features carry no `Vocabulary:` block and pass vacuously (diagnosis §9.2). Stage 1 Task 7 was to make the exemption explicit and counted; Task 14 Step 3 checks it and reports if it is still blanket.
- **The renderer paint-order divergence** (project memory: renderer-paint-order-divergence). SVG and WebGL stack epoch aggregates differently and SVG is viewport-culled. Pre-existing, open, and untouched by this stage — but note that Task 8 deleted the *route* that served the SVG side to the client, so the divergence is now only observable through `map-cli`. It did not go away; it got quieter, which is worse.
- **The whole-branch review as a standing gate.** Diagnosis §7.0's third lesson: per-task review cannot catch a whole-corpus defect, and the defect that got through was found only by a review whose unit was the entire branch. **Stage 4 should get one before Task 15's report**, and it is the one thing in this plan that no task can perform on itself.

### 4. What a v1.0 declaration would freeze, in one sentence

After v1.0, a breaking change to any feature or blessed fixture is a MAJOR bump. The items in §2 and §3 above are all things that are cheap to change now and expensive to change then — which is the whole content of the decision.

---

## Self-Review (performed at write time)

**1. Spec coverage.**

Spec §5 Stage 4's four named deliverables:
- *"The viewer consumes only contract functions"* — Task 2 writes the law, Tasks 4/5/6/8 move each consumer, Task 9 turns it green. ✓
- *"legacy routes deleted"* — Tasks 4, 5, 6, 7, 8 delete six routes plus the entity-id `pieces=` branch; Task 2's Rust law pins the surviving surface whole. The spec's count of "13" is reconciled in the route-census table above and put to the owner in Task 15 Step 1. ✓
- *"`TopographyDress` lands as dress-locality's show-piece"* — Tasks 10 (the type and its laws), 11 (wired, pixel-identical), 12 (the show-piece and the two scenarios that prove injectable/omittable/swappable). ✓
- *"Pre-release continues until the owner's v1.0 declaration"* — Task 9 Step 5 bumps to `0.5.0`, Task 15 declares it and explicitly refuses 1.0; the closing section states what the plan does not authorise. ✓

Spec §6's dogfood-bar checkables, each with a task: viewer on contract functions (2, 9); 13 legacy routes deleted (4–8, reconciled in 15); contract suite green (14 Step 3); atlas-edge CDC green against `:8080` and handed over (14 Step 4 — verified, not repeated, per diagnosis §8.3 and Stage 1 Task 17 Step 4); census stable (14 Step 5); golden gate green (14 Step 6, on the instrument Task 1 repaired); `contract()` reports the current 0.x (14 Step 7). ✓

Spec §3's algebra: law 1 omission-totality is exercised by Task 6's identity case and Task 9's `pieces=none`; law 2 composition is the licence Task 6 rests on and is re-checked before use; law 3 dress-locality is Task 12's first scenario; law 4 default-totality is Task 12's whole-body template assertion plus `styles()`'s `dresses` field. The `TopographyDress` corollary is Tasks 10–12, with the bathymetry and band-structure readings flagged for the owner rather than assumed. ✓

Spec §4's derivability — the clause diagnosis §9.1 calls the most important omission — is Task 13, strengthened from the spec's "by sampling" to totality-at-blessed-instants plus sampling over years, with the reason stated. ✓

Spec §7 is respected: no write API, no topography data-source selection (`sea` declarable and unsampled, `BandStructure` one variant), no atlas-repo edits, no upstream PRs, no v1.0. The closing section names each and says the plan does not authorise it. ✓

**Gaps I am leaving open, deliberately, with the reason:**
- The spec's true `⊕` is still untested as an operator. Diagnosis §8.5 argues against building a combine; Task 6 uses a client-side ⊕ under law 2 instead, and the closing section records that the question is now better-evidenced than it was and belongs after v1.0.
- Feature preambles remain unverified prose. A standing constraint makes it a reviewer's job; no law is invented for it.
- Tasks 4, 5, 7, 13 take dependencies on Stages 2 and 3, whose plans did not exist at write time. Each is marked `[STAGE-2 DEP]` / `[STAGE-3 DEP]` with the exact shape assumed and the fallback if it differs. **Reconcile all four against the real plans before Task 4 starts** — that reconciliation is the first thing an executor should do, and it is not optional.

**2. Placeholder scan.** No "TBD", no "add error handling", no "handle edge cases", no "similar to Task N". Two places carry a deliberate, marked hole and both are marked as such in-line: Task 11 Step 1's `before` table has two `Rgba(0,0,0,0)` rows with a `// FILL IN from <file>.ron` comment and a paragraph explaining that a test run against the placeholders would compare two wrongs — the values must be read out of files this plan is forbidden to edit and would be transcription, not instruction. Task 14 Step 5's census reference is expressed as "take Stage 3's closing artifacts" with a fallback shell, because Stage 3's plan did not exist. Eleven places say "adapt to what the file already does"; each names the exact file, the exact thing to read, and what to do if it differs. Two owner gates (Tasks 6 and 8) and one two-question gate (Task 12) are decisions this plan must not make, and naming them as gates is the point.

**3. Type consistency.** Checked and two inconsistencies found and fixed inline:
- Task 2's `CONTRACT_SURFACE` omitted `/api/laws`, which Task 4 makes the client fetch for the chronology frame. Added, and Task 2 Step 2's expected-failure list corrected to match.
- Task 3's `every_loaded_style_declares_what_it_dresses` asserts `PieceSet::all()` for every style, which Task 12's `terrain.ron` (dressing only Ground) breaks. Task 12 Step 7 now rewrites that test explicitly, keeping the whole-body shape rather than weakening it — and Task 3's version carries a comment saying it is expected to notice.

Names that match across tasks: `stopSetComplaints` (1, 4, 14); `deriveStops` (4); `stopsWithin` (5); `drawList` (6); `CONTRACT_SURFACE` / `served_surface_is_exactly_the_contract` (2, 3, 7, 8, 9, 15); `PieceSet::{empty, all, with, without, contains, iter, parse, render, default_for_viewer}` — the first eight are Stage 1 Task 8's, `default_for_viewer` is added in Task 9 and used in 9 and 11; `Piece::{ALL, name}` (3, 7, 9, 12); `Style::{dresses, topography, region_paint, water_paint, chrome, labeling, age_ramp, paper}` — `dresses` introduced in 3 and re-bodied in 12 (same name, same signature, both stated), `topography` replaces `topo_ramp` in 11 and is used in 11 and 12; `Ramp::{new, two, sample, stops}` and `RampError::{Empty, Unsorted, NotSpanning, PositionOutOfRange}` (10, 11, 12); `Intensity::{new, FULL, get}` (10, 12); `TopographyDress::{classical, from_two_stop, band_paint}` with fields `land`/`sea`/`bands`/`intensity` (10, 11, 12); `BandStructure::ByMeasuredArea` (10, closing section); `sceneUrl` keeps Stage 0's name and signature while its body changes in Task 9; `sceneFor` is introduced in Task 6 Step 5 as a named refactor of the existing scene-demand path.

**The one number this plan disputes with the spec, and the evidence for it.** Spec §6's checkable reads "13 legacy routes deleted". The route census above shows 15 routes today and — verified at `a81fbf3`, the spec's own commit — exactly 13 the day the spec was written. Six of them are deleted here; the rest are contract functions the spec itself requires the server to keep serving, so the literal reading of the checkable would delete `/api/scene`. The plan's reading is that the legacy *surface* is retired rather than the *count* deleted, and Task 15 Step 1 puts that reading to the owner instead of quietly adopting it. Nothing else in the spec or the diagnosis conflicts with what the code shows.
