# Geometry guard dependencies

The F-283 compiler guard uses [geo 0.31.0](https://github.com/georust/geo/tree/geo-0.31.0),
licensed under MIT or Apache-2.0, for spherical distance, bearing, interpolation,
densification, length, buffering, polygon difference and line clipping.
Its buffer/overlay implementation uses [i_overlay](https://github.com/iShape-Rust/iOverlay)
under MIT. The spatial segment index uses [rstar 0.12.2](https://github.com/georust/rstar),
licensed under MIT or Apache-2.0. These libraries retain their upstream notices.

Natural Earth source attribution and its public-domain declaration remain in
[data/natural-earth/LICENSE.md](data/natural-earth/LICENSE.md). The geometric
quarantine catalogue records original source hashes and paths as negative
admission evidence; it does not supply map witnesses or confer a licence on
any excluded source. See [the quarantine decision](docs/errata/quarantine.md).
