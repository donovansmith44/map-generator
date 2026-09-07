# The pipeline

Execution order of planned work. A stage is planned when its file exists,
queued when it only has an entry here.

1. **Stage 1** — entity registry + pieces made first-class — `2026-09-07-contract-stage1.md`
2. **Stage 2** — the ledger, judged by an empty census diff — `2026-09-07-contract-stage2.md`
3. **Stage 3** — one arrangement, tolerances declared not tuned — `2026-09-07-contract-stage3.md`
4. **Stage 4** — the dogfood bar, legacy routes retired — `2026-09-07-contract-stage4.md`
5. **Mutation testing** — QUEUED (owner, 2026-09-07). Planned after Stages 1–4 land.

## 5. Mutation testing — why it is queued, and what it must cover

Stage 0 found, by hand and by review, at least six checks that were
satisfiable by their own failure mode — a sort test whose example happened
to agree in both orders, a URL test passing on duplicated flags, an equality
test whose fake returned one body for every URL, a `@target` classification
with no test at all, a composition property whose two holes always drew the
same value, and the golden gate itself, which skips stops missing from the
run. Every one was caught by a person asking "would this fail if the code
were wrong?" Mutation testing automates exactly that question: mutate the
code, require a test to go red, and every surviving mutant is a check that
cannot fail.

Scope when planned (against the codebase as Stages 1–4 leave it):

- **Rust workspace** — `cargo-mutants` is the obvious tool; the census,
  disposition, arrangement and piece-attribution code are the high-value
  targets, since census diffs and the golden gate are blind to parts of them.
- **Haskell runner** — tooling is thin (no maintained mutation framework);
  expect a small owned harness in the house style: a mutation is a first-class
  value applied to a pure function under test, not a source-text hack. The
  step library, `Check.classify`, the correlated-hole generators and
  `hardReds` are the targets — the places where a silent weakening lies.
- **The contract itself** — mutate *responses*, not code: a harness that
  perturbs a blessed fixture or a live body (drop a row, move a coordinate,
  swap two ids) and requires the suite to go red. This is the whole-body
  law's own mutation test, and no off-the-shelf tool does it.

Exit bar, same as everything else here: a mutation run is a check too — it
must be able to fail, its kill-rate floor is declared with a reason, and a
surviving mutant is a finding to fix or rule on, never to wave through.
