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
- `crates/map-adapters` — phase 2: source adapters behind the
  `TimelineSource` seam. First source: historical-basemaps, with
  shared-arc extraction, narrated epoch deltas, typed exemptions, and
  the ring-for-ring fidelity law.
- `crates/map-provider` — phase 3: the reference `MapProvider`.
  Materialization = select ∘ simplify ∘ style, a pure function of the
  timeline; determinism, composition, accumulation, and selection laws
  run against it.
- `crates/map-encoders` — terminal encoders (law 11): SVG and GeoJSON
  backends behind `SceneEncoder`, deterministic byte for byte.
- `crates/map-viewer` — phase 4: the workbench. One page over the
  public contract only (`dyn MapProvider` + `SceneEncoder`, zero
  privileged access). `cargo run -p map-viewer --release`, then
  http://127.0.0.1:8090/ — scrubber stops at change events, subject
  picker, style/encoder switchers, long-exposure toggle, overlay
  scratchpad.
- `data/historical-basemaps` — the vendored first source (12 world
  border files, 4000 BC to AD 100, license included), for the offline
  story.
- `docs/map-system-handoff.md` — the founding spec.

## Working covenant (short form)

Types first; laws as tests; justification everywhere; honesty renders;
deltas, not states; reversibility by interface; algebraic composition
over inheritance; Bible-driven authority. See the spec for the binding
form.

## Make targets

`make build` · `make test` · `make demo` (detached workbench on 8090) ·
`make stop` · `make maps` (renders the canonical Bible map set into
`out/maps/`) · `make clean`. Without make: `bash scripts/demo.sh` and
`bash scripts/make-maps.sh`.
