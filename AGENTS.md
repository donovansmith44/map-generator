# Working in map-generator

This repository merges into the Bible Explorer (`your-word-is-truth-interactive-atlas`) after the NAMES and MAPS specs. Until then:

- **The work queue, locks and GO files live in the atlas repo:** `~/src/bible-atlas/.superpowers/{QUEUE,LOCKS,GO-codex,GO-claude}.md` and `~/src/bible-atlas/AGENTS.md`. Read them first. That `AGENTS.md`'s rules apply here too: lanes, one worktree per item, branches `lane/<agent>/<item-id>`, reviews, "never".
- **The binding covenant here:** `docs/map-system-handoff.md` §A (types first, laws as tests, justification everywhere, honesty renders, deltas not states, algebraic composition, Bible-driven authority) and the atlas's `docs/PRINCIPLES.md`.
- **Map errors are the priority.** The register lives in `docs/errata/` (queue items CX-M2 and CX-M3).
  - A fix is data with its justification, Scripture grounds first. Never a tuned constant or a styling trick.
  - Every fix starts from a failing law or golden stop.
- **The machine:** WSL2. Run `. ~/.bible-atlas-env` before `cargo`, `node` or `cabal` (Rust 1.97.1, Node 24, GHC 9.6.7 + cabal). Worktrees go in `~/w/mg-<item-id>`, target directories in `~/mut/<agent>-<item-id>`.
- **Until CX-M0 lands:** `cargo build` fails in WSL (the `atlas-graph-types` path dependency only resolves on Windows) and `make demo` launches a Windows `.exe` through PowerShell. CX-M0 fixes both.
- **Gates:**
  - `cargo test --workspace`
  - `make contract-gates`
  - with the workbench running on 8090: `node crates/map-viewer/tests/golden.js --check`
  - The golden views are re-blessed only with the owner's recorded approval.
- **Consult before changing the types.** If `map-types` doesn't compile against the atlas's `graph-types`, write it up and ask. Don't fix the types unilaterally.
- **Ports:** the workbench uses 8090. Never touch 8080.
