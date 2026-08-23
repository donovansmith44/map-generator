//! Shared-arc extraction — the priced-in work of §F.1. Sources give
//! each region its own closed outline, duplicating every shared border;
//! the core wants each border stretch stored ONCE. This module finds
//! the sharing that actually exists (exact quantized-vertex agreement,
//! the disclosed method) and shares it; stretches whose vertices don't
//! agree stay private to their ring — honest about the source, never
//! invented.
//!
//! The fidelity law leans on one guarantee: every input ring is exactly
//! the concatenation of its extracted arcs.

use std::collections::{BTreeMap, BTreeSet};

use crate::quantize::{canonical_rotation, QPoint};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArcDir {
    Forward,
    Reverse,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Extraction {
    /// Canonical arc polylines. An arc that closes on itself (a ring
    /// wholly one arc) repeats its first point at the end, so cycle
    /// continuity holds by the same end-meets-start rule as open arcs.
    pub arcs: Vec<Vec<QPoint>>,
    /// Per input ring, in input order: the ordered walk of arcs whose
    /// oriented concatenation reproduces the ring.
    pub cycles: Vec<Vec<(usize, ArcDir)>>,
}

type EdgeKey = (QPoint, QPoint);

fn edge_key(a: QPoint, b: QPoint) -> EdgeKey {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Split every ring into maximal runs of edges with identical user
/// sets, then deduplicate runs by canonical polyline.
pub fn extract(rings: &[Vec<QPoint>]) -> Extraction {
    // Which rings use each undirected edge?
    let mut users: BTreeMap<EdgeKey, BTreeSet<usize>> = BTreeMap::new();
    for (r, pts) in rings.iter().enumerate() {
        for i in 0..pts.len() {
            let key = edge_key(pts[i], pts[(i + 1) % pts.len()]);
            users.entry(key).or_default().insert(r);
        }
    }

    let mut arcs: Vec<Vec<QPoint>> = Vec::new();
    let mut index: BTreeMap<Vec<QPoint>, usize> = BTreeMap::new();
    let mut cycles: Vec<Vec<(usize, ArcDir)>> = Vec::new();

    fn store(
        arcs: &mut Vec<Vec<QPoint>>,
        index: &mut BTreeMap<Vec<QPoint>, usize>,
        canonical: Vec<QPoint>,
    ) -> usize {
        let next = arcs.len();
        *index.entry(canonical.clone()).or_insert_with(|| {
            arcs.push(canonical);
            next
        })
    }

    // Canonical form of an open walk: the lexicographically smaller of
    // the walk and its reverse, so both sharers land on the same arc.
    fn intern(
        arcs: &mut Vec<Vec<QPoint>>,
        index: &mut BTreeMap<Vec<QPoint>, usize>,
        walked: Vec<QPoint>,
    ) -> (usize, ArcDir) {
        let mut reversed = walked.clone();
        reversed.reverse();
        let (canonical, dir) =
            if walked <= reversed { (walked, ArcDir::Forward) } else { (reversed, ArcDir::Reverse) };
        (store(arcs, index, canonical), dir)
    }

    for pts in rings {
        let n = pts.len();
        let edge_users =
            |i: usize| users.get(&edge_key(pts[i % n], pts[(i + 1) % n])).unwrap().clone();
        // A junction is a vertex where the sharing situation changes.
        let junctions: Vec<usize> =
            (0..n).filter(|&i| edge_users((i + n - 1) % n) != edge_users(i)).collect();

        if junctions.is_empty() {
            // The whole ring is one arc, closed on itself. Canonical
            // form minimizes over every rotation AND both directions,
            // so two sharers walking opposite ways from arbitrary
            // phases still land on the same arc.
            let forward = canonical_rotation(pts);
            let mut rev = pts.clone();
            rev.reverse();
            let backward = canonical_rotation(&rev);
            let (mut closed, dir) = if forward <= backward {
                (forward, ArcDir::Forward)
            } else {
                (backward, ArcDir::Reverse)
            };
            closed.push(closed[0]);
            let id = store(&mut arcs, &mut index, closed);
            cycles.push(vec![(id, dir)]);
            continue;
        }

        // Walk junction to junction, starting at the first one.
        let mut cycle = Vec::with_capacity(junctions.len());
        for (j, &start) in junctions.iter().enumerate() {
            let end = junctions[(j + 1) % junctions.len()];
            let mut walked = Vec::new();
            let mut i = start;
            loop {
                walked.push(pts[i]);
                if i == end && walked.len() > 1 {
                    break;
                }
                i = (i + 1) % n;
                if i == start {
                    walked.push(pts[i]); // wrapped fully: close on start
                    break;
                }
            }
            cycle.push(intern(&mut arcs, &mut index, walked));
        }
        cycles.push(cycle);
    }

    Extraction { arcs, cycles }
}

/// The fidelity witness: rebuild a ring from its cycle. Each arc
/// contributes its oriented points minus the trailing junction (owned
/// by the next arc).
pub fn reconstruct_ring(ext: &Extraction, ring: usize) -> Vec<QPoint> {
    let mut out = Vec::new();
    for &(arc, dir) in &ext.cycles[ring] {
        let pts = &ext.arcs[arc];
        match dir {
            ArcDir::Forward => out.extend_from_slice(&pts[..pts.len() - 1]),
            ArcDir::Reverse => out.extend(pts[1..].iter().rev()),
        }
    }
    out
}
