# map-generator

A historical map-generation engine: snapshots of arbitrary chunks of the
world at arbitrary points in time, with time-accurate borders, honest
uncertainty, style layers, clean overlay composition, and animated
transitions between snapshots.

Separate from — and a service to — the Bible Atlas and future offline
exploration tools. Authority flows from Scripture through the atlas
graph into the map (contract set C1–C6).

The founding spec is `docs/map-system-handoff.md`. The compiled types
crate (`crates/map-types`) is the authority; where prose and crate
disagree, the crate wins.

## Layout

- `crates/map-types` — phase 1: the domain types and the design laws as
  tests. Depends only on `atlas-graph-types` (path dep on the sibling
  Bible Atlas checkout, read-only; contract C1).
- `docs/map-system-handoff.md` — the founding spec.

## Working covenant (short form)

Types first; laws as tests; justification everywhere; honesty renders;
deltas, not states; reversibility by interface; algebraic composition
over inheritance; Bible-driven authority. See the spec for the binding
form.
