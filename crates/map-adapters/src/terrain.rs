//! Relief (phase 5): the earth's own shape as explorable regions.
//! A public-domain global elevation grid (ETOPO, NOAA) is contoured by
//! marching squares into hypsometric bands — each band ONE multi-part
//! region (class Terrain), so "the highlands" is as clickable as any
//! kingdom. Validity opens at the configured start: under the biblical
//! frame, "let the dry land appear" (GEN 1:9) — the DATE claim is
//! Scripture's, the SHAPE is modern measurement, both disclosed.

use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use atlas_graph_types::covenant::{ContentHash, SourceId, TimePoint};
use atlas_graph_types::covenant::{Ground, Justification};
use atlas_graph_types::covenant::{BibleLocus, LocusRange, VerseRef};

use map_types::{
    Boundary, BoundaryHistory, BoundaryId, BoundarySource, EdgeCharacter, Interval, Orientation,
    RegionClass, RegionGeom, RegionHistory, RegionId, RegionPart, UnitVec, WorldTimeline,
};

const TERRAIN_PROVENANCE: &str = "etopo1 via NOAA ERDDAP (public domain; modern relief)";

/// The hypsometric bands, lowest to highest (meters above sea level).
pub const BANDS: [(i16, &str); 5] = [
    (200, "hills (above 200 m)"),
    (500, "uplands (above 500 m)"),
    (1000, "highlands (above 1000 m)"),
    (2000, "mountains (above 2000 m)"),
    (3500, "high peaks (above 3500 m)"),
];

/// A lat/lon elevation grid: row-major, `rows` latitudes from lat0
/// upward by `step`, `cols` longitudes from lon0 eastward by `step`.
pub struct ElevationGrid {
    pub rows: usize,
    pub cols: usize,
    pub lat0: f64,
    pub lon0: f64,
    pub step: f64,
    pub data: Vec<i16>,
}

impl ElevationGrid {
    /// The vendored binary: little-endian i16 meters, 721x1441 at a
    /// quarter degree from (-90,-180).
    pub fn from_etopo_bin(bytes: &[u8]) -> Option<ElevationGrid> {
        let (rows, cols) = (721usize, 1441usize);
        if bytes.len() != rows * cols * 2 {
            return None;
        }
        let data = bytes
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();
        Some(ElevationGrid { rows, cols, lat0: -90.0, lon0: -180.0, step: 0.25, data })
    }

    fn at(&self, i: isize, j: isize) -> f64 {
        // Outside the grid counts as far below every threshold, so
        // contours close along the grid boundary.
        if i < 0 || j < 0 || i as usize >= self.rows || j as usize >= self.cols {
            return -30000.0;
        }
        f64::from(self.data[i as usize * self.cols + j as usize])
    }
}

/// Marching squares at one threshold: closed rings (lat, lon), each a
/// contour of the region at-or-above the threshold. Deterministic:
/// cells scanned in order, segments chained by grid-edge identity.
pub fn contour_rings(grid: &ElevationGrid, threshold: f64) -> Vec<Vec<(f64, f64)>> {
    // A grid EDGE holds at most one crossing point per threshold; key
    // edges as (i, j, horizontal?) of their lower/left corner.
    type EdgeKey = (isize, isize, bool);
    let point_on = |a_i: isize, a_j: isize, b_i: isize, b_j: isize| -> (f64, f64) {
        let (va, vb) = (grid.at(a_i, a_j), grid.at(b_i, b_j));
        let t = ((threshold - va) / (vb - va)).clamp(0.0, 1.0);
        let lat = grid.lat0 + grid.step * (a_i as f64 + t * (b_i - a_i) as f64);
        let lon = grid.lon0 + grid.step * (a_j as f64 + t * (b_j - a_j) as f64);
        (lat, lon)
    };
    // Per cell, emit directed segments from-edge -> to-edge such that
    // the ABOVE side stays on the left; chain by edge key.
    let mut next: BTreeMap<EdgeKey, (EdgeKey, (f64, f64), (f64, f64))> = BTreeMap::new();
    for i in -1..grid.rows as isize {
        for j in -1..grid.cols as isize {
            let above = |v: f64| v >= threshold;
            let (v00, v01, v10, v11) =
                (grid.at(i, j), grid.at(i, j + 1), grid.at(i + 1, j), grid.at(i + 1, j + 1));
            let case = (above(v00) as u8)
                | (above(v01) as u8) << 1
                | (above(v11) as u8) << 2
                | (above(v10) as u8) << 3;
            if case == 0 || case == 15 {
                continue;
            }
            // Edges of this cell: S = bottom horizontal, E = right
            // vertical, N = top horizontal, W = left vertical.
            let s_key = (i, j, true);
            let n_key = (i + 1, j, true);
            let w_key = (i, j, false);
            let e_key = (i, j + 1, false);
            let s_pt = point_on(i, j, i, j + 1);
            let n_pt = point_on(i + 1, j, i + 1, j + 1);
            let w_pt = point_on(i, j, i + 1, j);
            let e_pt = point_on(i, j + 1, i + 1, j + 1);
            let mut emit = |from: EdgeKey, fp: (f64, f64), to: EdgeKey, tp: (f64, f64)| {
                next.insert(from, (to, fp, tp));
            };
            match case {
                1 => emit(w_key, w_pt, s_key, s_pt),
                2 => emit(s_key, s_pt, e_key, e_pt),
                3 => emit(w_key, w_pt, e_key, e_pt),
                4 => emit(e_key, e_pt, n_key, n_pt),
                5 => {
                    // saddle: resolve by center mean
                    if (v00 + v01 + v10 + v11) / 4.0 >= threshold {
                        emit(w_key, w_pt, n_key, n_pt);
                        emit(e_key, e_pt, s_key, s_pt);
                    } else {
                        emit(w_key, w_pt, s_key, s_pt);
                        emit(e_key, e_pt, n_key, n_pt);
                    }
                }
                6 => emit(s_key, s_pt, n_key, n_pt),
                7 => emit(w_key, w_pt, n_key, n_pt),
                8 => emit(n_key, n_pt, w_key, w_pt),
                9 => emit(n_key, n_pt, s_key, s_pt),
                10 => {
                    if (v00 + v01 + v10 + v11) / 4.0 >= threshold {
                        emit(n_key, n_pt, e_key, e_pt);
                        emit(s_key, s_pt, w_key, w_pt);
                    } else {
                        emit(n_key, n_pt, w_key, w_pt);
                        emit(s_key, s_pt, e_key, e_pt);
                    }
                }
                11 => emit(n_key, n_pt, e_key, e_pt),
                12 => emit(e_key, e_pt, w_key, w_pt),
                13 => emit(e_key, e_pt, s_key, s_pt),
                14 => emit(s_key, s_pt, w_key, w_pt),
                _ => unreachable!(),
            }
        }
    }
    // Chain segments into rings. Orientation consistency (above side
    // on the left) makes every chain a perfect cycle; a chain that
    // fails to close would be a false boundary, so it is dropped, not
    // patched.
    let mut rings = Vec::new();
    while let Some((&start, _)) = next.iter().next() {
        let mut ring = Vec::new();
        let mut key = start;
        let closed = loop {
            let Some((to, fp, _tp)) = next.remove(&key) else { break false };
            ring.push(fp);
            key = to;
            if key == start {
                break true;
            }
        };
        if closed && ring.len() >= 8 {
            rings.push(ring);
        }
    }
    rings
}

fn hash_id(tag: &str) -> ContentHash {
    let mut h = DefaultHasher::new();
    tag.hash(&mut h);
    ContentHash(h.finish())
}

/// Build the relief timeline: one Terrain-class region per band, its
/// parts the contour rings (even-odd nesting makes valleys holes for
/// free), valid from `from` to the open edge of knowledge.
pub fn ingest_terrain(
    grid: &ElevationGrid,
    land: &[Vec<(f64, f64)>],
    from: TimePoint,
) -> WorldTimeline {
    let gen_1_9 = || {
        let v = BibleLocus::whole(VerseRef { book: 1, chapter: 1, verse: 9 });
        LocusRange::new(v.clone(), v).expect("a verse is a range")
    };
    let justification = Justification {
        text: Some(
            "\"Let the dry land appear\" — the date is the Word's; the elevations \
             are modern measurement (see provenance)."
                .to_string(),
        ),
        grounds: [
            Ground::Scripture(gen_1_9()),
            Ground::Source(SourceId::new("etopo1")),
        ]
        .into(),
    };
    let valid = Interval { from, to: None };

    let mut tl = WorldTimeline::default();
    // THE DRY LAND (GEN 1:10): the base of the relief stack — Natural
    // Earth land clipped to the working frame, the same derivation as
    // the sea witness, so coast and stage agree by construction. The
    // elevation bands rise from it.
    {
        let mut parts = Vec::new();
        for (ri, ring) in land.iter().enumerate() {
            let mut pts: Vec<UnitVec> =
                ring.iter().map(|(la, lo)| UnitVec::from_lat_lon_deg(*la, *lo)).collect();
            if pts.len() >= 3 {
                pts.push(pts[0]);
                let bid = BoundaryId(hash_id(&format!("terrain/land/ring{ri}")));
                tl.boundaries.insert(
                    bid,
                    BoundaryHistory {
                        versions: vec![(
                            valid,
                            Boundary {
                                pts,
                                character: EdgeCharacter::Line,
                                source: BoundarySource::Imported {
                                    source: SourceId::new("natural-earth"),
                                },
                                justification: justification.clone(),
                                provenance: TERRAIN_PROVENANCE.to_string(),
                            },
                        )],
                    },
                );
                parts.push(RegionPart { cycle: vec![(bid, Orientation::Forward)], holes: vec![] });
            }
        }
        if !parts.is_empty() {
            let rid = RegionId(hash_id("terrain/land"));
            tl.regions.insert(
                rid,
                RegionHistory {
                    class: RegionClass::Terrain(0),
                    label_history: vec![(valid, "the dry land".to_string())],
                    geom_history: vec![(valid, RegionGeom { parts })],
                },
            );
        }
    }
    for (band, (threshold, label)) in BANDS.iter().enumerate() {
        let rings = contour_rings(grid, f64::from(*threshold));
        let mut parts = Vec::new();
        for (ri, ring) in rings.iter().enumerate() {
            let mut pts: Vec<UnitVec> =
                ring.iter().map(|(la, lo)| UnitVec::from_lat_lon_deg(*la, *lo)).collect();
            pts.push(pts[0]); // closed arc form
            let bid = BoundaryId(hash_id(&format!("terrain/band{band}/ring{ri}")));
            tl.boundaries.insert(
                bid,
                BoundaryHistory {
                    versions: vec![(
                        valid,
                        Boundary {
                            pts,
                            character: EdgeCharacter::Line,
                            source: BoundarySource::Imported {
                                source: SourceId::new("etopo1"),
                            },
                            justification: justification.clone(),
                            provenance: TERRAIN_PROVENANCE.to_string(),
                        },
                    )],
                },
            );
            parts.push(RegionPart {
                cycle: vec![(bid, Orientation::Forward)],
                holes: vec![],
            });
        }
        if parts.is_empty() {
            continue;
        }
        let rid = RegionId(hash_id(&format!("terrain/band{band}")));
        tl.regions.insert(
            rid,
            RegionHistory {
                class: RegionClass::Terrain(band as u8),
                label_history: vec![(valid, (*label).to_string())],
                geom_history: vec![(valid, RegionGeom { parts })],
            },
        );
    }
    tl
}
