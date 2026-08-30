//! The arrangement builder: witnesses in, one lawful partition out.
//!
//! Pipeline (the spherical adaptation of standard arrangement
//! construction with deterministic snap-rounding):
//!   normalize → candidate nodes (endpoints + arc intersections)
//!   → deterministic clustering → split every segment at every node
//!   on it → dedupe coincident atomic arcs (same border drawn twice
//!   becomes ONE edge) → half-edge assembly by angular order → face
//!   cycles → group cycles into cells by containment signature →
//!   classify against witnesses → absorb slivers semantically →
//!   validate (including the 4π law).

use std::collections::BTreeMap;

use map_types::UnitVec;

use crate::{
    bearing, cycle_area, winding, FaceKind, Fnv, PEdge, PFace, PHalf, Partition,
    PartitionConfig, RiverPath,
};

pub struct WitnessRegion {
    pub id: String,
    pub kind: FaceKind,
    /// outer rings (and hole rings; orientation is normalized away).
    pub rings: Vec<Vec<UnitVec>>,
    /// subdivision: this claim counts only where the parent claims —
    /// a tribe exists within its land, never beyond it.
    pub parent: Option<String>,
}

pub struct WitnessPolyline {
    pub id: String,
    pub pts: Vec<UnitVec>,
}

/// An OPEN border curve: part of the arrangement (it bounds cells)
/// without claiming anything itself. A region's land border.
pub struct WitnessBorder {
    pub id: String,
    pub pts: Vec<UnitVec>,
}

/// A claim by SEED: the face containing the seed point IS the region.
/// Its water-side boundary is therefore the water's own edge by
/// construction — flush cannot fail, it can only refuse to build
/// (a dangling border leaks the cell, the Background face vanishes,
/// and validation fails loud).
pub struct WitnessSeed {
    pub id: String,
    pub kind: FaceKind,
    pub seed: UnitVec,
}

#[derive(Debug)]
pub enum BuildError {
    /// arc endpoints too close to antipodal for a unique minor arc.
    AntipodalArc { witness: String },
    /// a witness ring with fewer than three distinct points.
    DegenerateRing { witness: String },
    /// the assembled subdivision failed its own laws.
    TopologyFailure(Vec<String>),
}

fn dist(a: &UnitVec, b: &UnitVec) -> f64 {
    a.angle_to(b)
}

fn norm3(x: f64, y: f64, z: f64) -> Option<UnitVec> {
    UnitVec::normalize(x, y, z).ok()
}

/// Deterministic byte key for a point (used to fix all orderings).
fn key_of(v: &UnitVec) -> [u8; 24] {
    let q = |x: f64| ((x * 1e9).round() as i64).to_be_bytes();
    let mut k = [0u8; 24];
    k[..8].copy_from_slice(&q(v.x()));
    k[8..16].copy_from_slice(&q(v.y()));
    k[16..].copy_from_slice(&q(v.z()));
    k
}

struct Seg {
    a: UnitVec,
    b: UnitVec,
    witness: String,
}

/// union-find with deterministic behavior
struct Uf(Vec<usize>);
impl Uf {
    fn new(n: usize) -> Self {
        Uf((0..n).collect())
    }
    fn find(&mut self, i: usize) -> usize {
        if self.0[i] != i {
            let r = self.find(self.0[i]);
            self.0[i] = r;
        }
        self.0[i]
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        // deterministic: smaller index wins
        if ra < rb {
            self.0[rb] = ra;
        } else if rb < ra {
            self.0[ra] = rb;
        }
    }
}

pub fn build(
    regions: &[WitnessRegion],
    polylines: &[WitnessPolyline],
    cfg: &PartitionConfig,
) -> Result<Partition, BuildError> {
    build_with(regions, &[], &[], polylines, cfg)
}

pub fn build_with(
    regions: &[WitnessRegion],
    borders: &[WitnessBorder],
    seeds: &[WitnessSeed],
    polylines: &[WitnessPolyline],
    cfg: &PartitionConfig,
) -> Result<Partition, BuildError> {
    let mut diagnostics = Vec::new();

    // ---- 1. normalize witness rings into segments
    let mut segs: Vec<Seg> = Vec::new();
    let mut rings_norm: Vec<(String, FaceKind, Vec<UnitVec>, Option<String>)> = Vec::new();
    for w in regions {
        for ring in &w.rings {
            let mut pts: Vec<UnitVec> = Vec::with_capacity(ring.len());
            for p in ring {
                if pts.last().map_or(true, |q| dist(q, p) > 1e-12) {
                    pts.push(*p);
                }
            }
            while pts.len() > 1 && dist(&pts[0], pts.last().unwrap()) <= 1e-12 {
                pts.pop();
            }
            if pts.len() < 3 {
                return Err(BuildError::DegenerateRing { witness: w.id.clone() });
            }
            // orientation-normalize: interior on the left
            if cycle_area(&pts) < 0.0 {
                pts.reverse();
            }
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                let (cx, cy, cz) = a.cross_raw(&b);
                if (cx * cx + cy * cy + cz * cz).sqrt() < 1e-12 {
                    // a vanishing cross product is either a duplicate
                    // point (skip: clustering merges it) or a true
                    // antipode (reject: no unique minor arc)
                    if a.angle_to(&b) > 1.0 {
                        return Err(BuildError::AntipodalArc { witness: w.id.clone() });
                    }
                    continue;
                }
                segs.push(Seg { a, b, witness: w.id.clone() });
            }
            rings_norm.push((w.id.clone(), w.kind.clone(), pts, w.parent.clone()));
        }
    }
    for w in borders {
        let mut pts: Vec<UnitVec> = Vec::with_capacity(w.pts.len());
        for p in &w.pts {
            if pts.last().map_or(true, |q| dist(q, p) > 1e-12) {
                pts.push(*p);
            }
        }
        for pair in pts.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let (cx, cy, cz) = a.cross_raw(&b);
            if (cx * cx + cy * cy + cz * cz).sqrt() < 1e-12 {
                if a.angle_to(&b) > 1.0 {
                    return Err(BuildError::AntipodalArc { witness: w.id.clone() });
                }
                continue;
            }
            segs.push(Seg { a, b, witness: w.id.clone() });
        }
    }
    // parent-chain depth per witness (most specific = deepest)
    let wit_depth: BTreeMap<String, usize> = {
        let par: BTreeMap<&String, &Option<String>> =
            rings_norm.iter().map(|(w, _, _, p)| (w, p)).collect();
        rings_norm
            .iter()
            .map(|(w, _, _, _)| {
                let mut d = 0usize;
                let mut cur = par.get(w).copied();
                while let Some(Some(pid)) = cur {
                    d += 1;
                    cur = par.get(pid).copied();
                    if d > 8 {
                        break;
                    }
                }
                (w.clone(), d)
            })
            .collect()
    };

    // total ring area per witness: the measure of specificity
    let wit_area: BTreeMap<String, f64> = {
        let mut m: BTreeMap<String, f64> = BTreeMap::new();
        for (w, _, ring, _) in &rings_norm {
            *m.entry(w.clone()).or_insert(0.0) += cycle_area(ring).abs();
        }
        m
    };

    // deterministic processing order regardless of caller order
    segs.sort_by(|s, t| key_of(&s.a).cmp(&key_of(&t.a)).then(key_of(&s.b).cmp(&key_of(&t.b))));

    // ---- 2. candidate nodes: endpoints + pairwise arc intersections
    let mut cands: Vec<UnitVec> = Vec::new();
    for s in &segs {
        cands.push(s.a);
        cands.push(s.b);
    }
    for i in 0..segs.len() {
        for j in i + 1..segs.len() {
            if let Some(p) = arc_intersection(&segs[i], &segs[j], cfg.tau_vertex) {
                cands.push(p);
            }
        }
    }
    cands.sort_by(|a, b| key_of(a).cmp(&key_of(b)));
    cands.dedup_by(|a, b| key_of(a) == key_of(b));

    // ---- 3. deterministic clustering
    let mut uf = Uf::new(cands.len());
    for i in 0..cands.len() {
        for j in i + 1..cands.len() {
            if dist(&cands[i], &cands[j]) <= cfg.tau_vertex {
                uf.union(i, j);
            }
        }
    }
    let mut clusters: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..cands.len() {
        let r = uf.find(i);
        clusters.entry(r).or_default().push(i);
    }
    // representative: normalized sum in deterministic (sorted) order
    let mut reps: Vec<UnitVec> = Vec::new();
    let mut rep_of_cand: Vec<usize> = vec![0; cands.len()];
    for (_, members) in &clusters {
        let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
        for &m in members {
            x += cands[m].x();
            y += cands[m].y();
            z += cands[m].z();
        }
        let rep = norm3(x, y, z).expect("cluster of unit vectors has a mean direction");
        let id = reps.len();
        reps.push(rep);
        for &m in members {
            rep_of_cand[m] = id;
        }
    }
    let rep_index = |p: &UnitVec, cands: &[UnitVec], rep_of: &[usize]| -> usize {
        // exact candidate lookup by key
        let k = key_of(p);
        let i = cands.binary_search_by(|c| key_of(c).cmp(&k)).expect("endpoint is a candidate");
        rep_of[i]
    };

    // ---- 4. split every segment at every node on it
    // atomic arcs collected as (rep_a, rep_b) with witness provenance
    let mut atomic: BTreeMap<(usize, usize), Vec<String>> = BTreeMap::new();
    for s in &segs {
        let ra = rep_index(&s.a, &cands, &rep_of_cand);
        let rb = rep_index(&s.b, &cands, &rep_of_cand);
        if ra == rb {
            continue; // collapsed by clustering
        }
        // nodes on this segment: any rep within tau_edge of the arc,
        // strictly between the endpoints
        let (nx, ny, nz) = s.a.cross_raw(&s.b);
        let nn = (nx * nx + ny * ny + nz * nz).sqrt();
        let n = (nx / nn, ny / nn, nz / nn);
        let full = dist(&s.a, &s.b);
        let mut on: Vec<(f64, usize)> = vec![(0.0, ra), (full, rb)];
        for (ri, rp) in reps.iter().enumerate() {
            if ri == ra || ri == rb {
                continue;
            }
            let off = (rp.x() * n.0 + rp.y() * n.1 + rp.z() * n.2).abs().asin();
            if off > cfg.tau_edge {
                continue;
            }
            // project onto the supporting great circle
            let d = rp.x() * n.0 + rp.y() * n.1 + rp.z() * n.2;
            let Some(proj) = norm3(rp.x() - d * n.0, rp.y() - d * n.1, rp.z() - d * n.2) else {
                continue;
            };
            let sa = dist(&s.a, &proj);
            let sb = dist(&proj, &s.b);
            if (sa + sb - full).abs() <= cfg.tau_edge && sa > cfg.tau_vertex * 0.5 && sb > cfg.tau_vertex * 0.5
            {
                on.push((sa, ri));
            }
        }
        on.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        on.dedup_by_key(|x| x.1);
        for w in on.windows(2) {
            let (u, v) = (w[0].1, w[1].1);
            if u == v {
                continue;
            }
            let k = (u.min(v), u.max(v));
            let entry = atomic.entry(k).or_default();
            if !entry.contains(&s.witness) {
                entry.push(s.witness.clone());
            }
        }
    }

    // ---- 5–8: assemble and extract faces, HEALING to fixpoint: an
    // edge that ends with the same face on both sides bounds nothing
    // (an antenna stub or a bridge left by partial witness merges) —
    // it is removed deterministically and the arrangement reassembled.
    let (vertices, mut edges, halves, faces) = 'outer: loop {
        let (vertices, mut edges, halves, mut faces, face_rep, wrap_face, edge_keys) = loop {
        // canonical vertices = reps used by atomic arcs
        let mut used: Vec<usize> = atomic.keys().flat_map(|&(a, b)| [a, b]).collect();
        used.sort();
        used.dedup();
        let mut vid_of_rep: BTreeMap<usize, usize> = BTreeMap::new();
        let mut vertices: Vec<UnitVec> = Vec::new();
        for r in used {
            vid_of_rep.insert(r, vertices.len());
            vertices.push(reps[r]);
        }
        let rep_of_vid: BTreeMap<usize, usize> =
            vid_of_rep.iter().map(|(&r, &v)| (v, r)).collect();

        // half-edge assembly
        let mut edges: Vec<PEdge> = Vec::new();
        let mut halves: Vec<PHalf> = Vec::new();
        let mut edge_keys: Vec<(usize, usize)> = Vec::new();
        for (&(ra, rb), witnesses) in &atomic {
            edge_keys.push((ra.min(rb), ra.max(rb)));
            let (a, b) = (vid_of_rep[&ra], vid_of_rep[&rb]);
            let e = edges.len();
            let h_ab = halves.len();
            let h_ba = h_ab + 1;
            halves.push(PHalf { origin: a, edge: e, twin: h_ba, next: usize::MAX, prev: usize::MAX, face: usize::MAX });
            halves.push(PHalf { origin: b, edge: e, twin: h_ab, next: usize::MAX, prev: usize::MAX, face: usize::MAX });
            edges.push(PEdge { a, b, half_ab: h_ab, half_ba: h_ba, river: false, provenance: witnesses.clone() });
        }
        // outgoing half-edges per vertex, sorted by bearing
        let mut out_at: Vec<Vec<usize>> = vec![Vec::new(); vertices.len()];
        for (hi, h) in halves.iter().enumerate() {
            out_at[h.origin].push(hi);
        }
        let dest_of = |h: &PHalf, halves: &[PHalf]| halves[h.twin].origin;
        for v in 0..vertices.len() {
            let p = vertices[v];
            out_at[v].sort_by(|&x, &y| {
                let bx = bearing(&p, &vertices[dest_of(&halves[x], &halves)]);
                let by = bearing(&p, &vertices[dest_of(&halves[y], &halves)]);
                bx.partial_cmp(&by).unwrap()
            });
        }
        // next(h): at the destination vertex, the clockwise neighbor
        // of twin(h) — interior stays left.
        for hi in 0..halves.len() {
            let d = dest_of(&halves[hi], &halves);
            let ring = &out_at[d];
            let pos =
                ring.iter().position(|&x| x == halves[hi].twin).expect("twin is outgoing at dest");
            let nxt = ring[(pos + ring.len() - 1) % ring.len()];
            halves[hi].next = nxt;
            halves[nxt].prev = hi;
        }

        // face cycles
        let mut cycle_of_half: Vec<usize> = vec![usize::MAX; halves.len()];
        let mut cycles: Vec<Vec<usize>> = Vec::new();
        for h0 in 0..halves.len() {
            if cycle_of_half[h0] != usize::MAX {
                continue;
            }
            let mut cy = Vec::new();
            let mut h = h0;
            loop {
                cycle_of_half[h] = cycles.len();
                cy.push(h);
                h = halves[h].next;
                if h == h0 {
                    break;
                }
            }
            cycles.push(cy);
        }
        let cycle_pts: Vec<Vec<UnitVec>> = cycles
            .iter()
            .map(|cy| cy.iter().map(|&h| vertices[halves[h].origin]).collect())
            .collect();
        let cycle_signed: Vec<f64> = cycle_pts.iter().map(|p| cycle_area(p)).collect();

        // representative point just left of each cycle's first arc
        let eps = (cfg.tau_vertex * 0.4).max(1e-7);
        let cycle_rep: Vec<UnitVec> = cycles
            .iter()
            .map(|cy| {
                let h = cy[0];
                let a = vertices[halves[h].origin];
                let b = vertices[dest_of(&halves[h], &halves)];
                let m = norm3(a.x() + b.x(), a.y() + b.y(), a.z() + b.z()).unwrap_or(a);
                let (nx, ny, nz) = a.cross_raw(&b);
                let nn = (nx * nx + ny * ny + nz * nz).sqrt();
                norm3(
                    m.x() * eps.cos() + nx / nn * eps.sin(),
                    m.y() * eps.cos() + ny / nn * eps.sin(),
                    m.z() * eps.cos() + nz / nn * eps.sin(),
                )
                .unwrap_or(m)
            })
            .collect();

        // group cycles into faces by containment signature
        let nc = cycles.len();
        let mut signature: Vec<Vec<usize>> = Vec::with_capacity(nc);
        for i in 0..nc {
            let mut sig = Vec::new();
            for j in 0..nc {
                if winding(&cycle_pts[j], &cycle_rep[i]) == 1 {
                    sig.push(j);
                }
            }
            signature.push(sig);
        }
        let mut face_of_sig: BTreeMap<Vec<usize>, usize> = BTreeMap::new();
        let mut faces: Vec<PFace> = Vec::new();
        let mut cycle_face: Vec<usize> = vec![usize::MAX; nc];
        for i in 0..nc {
            let f = *face_of_sig.entry(signature[i].clone()).or_insert_with(|| {
                faces.push(PFace {
                    cycles: Vec::new(),
                    kind: FaceKind::Background,
                    claims: Vec::new(),
                    conflicts: Vec::new(),
                    area: 0.0,
                });
                faces.len() - 1
            });
            faces[f].cycles.push(cycles[i].clone());
            cycle_face[i] = f;
            for &h in &cycles[i] {
                halves[h].face = f;
            }
        }
        // the healing check: every edge must separate two faces
        let bad: Vec<usize> = (0..edges.len())
            .filter(|&e| halves[edges[e].half_ab].face == halves[edges[e].half_ba].face)
            .collect();
        if !bad.is_empty() {
            for &e in &bad {
                let key = {
                    let (ra, rb) = (rep_of_vid[&edges[e].a], rep_of_vid[&edges[e].b]);
                    (ra.min(rb), ra.max(rb))
                };
                let (la, lo) = vertices[edges[e].a].to_lat_lon_deg();
                diagnostics.push(format!(
                    "  healed edge at ({la:.3},{lo:.3}) witnesses {:?}",
                    edges[e].provenance
                ));
                atomic.remove(&key);
            }
            diagnostics.push(format!(
                "healed {} edge(s) that bounded no area (antenna/bridge)",
                bad.len()
            ));
            continue;
        }

        // face areas: sum of signed cycle areas; the global wrap face
        // comes out ≤ 0 and gains 4π.
        let tau = 4.0 * std::f64::consts::PI;
        let mut sig_area: BTreeMap<usize, f64> = BTreeMap::new();
        for i in 0..nc {
            *sig_area.entry(cycle_face[i]).or_insert(0.0) += cycle_signed[i];
        }
        let mut wrap_face = usize::MAX;
        for (f, area) in sig_area {
            faces[f].area = if area > 1e-12 {
                area
            } else {
                wrap_face = f;
                area + tau
            };
        }
        // per-face representative point (first cycle's rep)
        let face_rep: Vec<UnitVec> = (0..faces.len())
            .map(|fi| {
                let ci = (0..nc).find(|&i| cycle_face[i] == fi).expect("face has a cycle");
                cycle_rep[ci]
            })
            .collect();
        break (vertices, edges, halves, faces, face_rep, wrap_face, edge_keys);
    };

    // ---- 9. classify faces against witness rings
    for (fi, face) in faces.iter_mut().enumerate() {
        let p = face_rep[fi];
        let mut kinds: Vec<(String, FaceKind, Option<String>)> = Vec::new();
        for (wid, kind, ring, par) in &rings_norm {
            if winding(ring, &p) == 1 && !kinds.iter().any(|(w, _, _)| w == wid) {
                kinds.push((wid.clone(), kind.clone(), par.clone()));
            }
        }
        // SUBDIVISION RULE: a claim with a parent counts only where
        // the parent also claims — iterate to fixpoint so grandchild
        // chains resolve too.
        loop {
            let present: std::collections::BTreeSet<String> =
                kinds.iter().map(|(w, _, _)| w.clone()).collect();
            let before = kinds.len();
            kinds.retain(|(_, _, par)| par.as_ref().map_or(true, |p| present.contains(p)));
            if kinds.len() == before {
                break;
            }
        }
        if kinds.is_empty() {
            face.kind = FaceKind::Background;
            continue;
        }
        // precedence: water over land; among land, the MOST SPECIFIC
        // claim first — deeper in the parent chain, then the SMALLER
        // witness (a region names the face over the country that
        // contains it: specificity is measured, never curated); id
        // order only as the final deterministic tie.
        kinds.sort_by(|a, b| {
            let rank = |k: &FaceKind| match k {
                FaceKind::Lake => 0,
                FaceKind::Sea => 1,
                FaceKind::LandClaim => 2,
                FaceKind::Background => 3,
            };
            let da = wit_depth.get(&a.0).copied().unwrap_or(0);
            let db = wit_depth.get(&b.0).copied().unwrap_or(0);
            let aa = wit_area.get(&a.0).copied().unwrap_or(0.0);
            let ab = wit_area.get(&b.0).copied().unwrap_or(0.0);
            rank(&a.1)
                .cmp(&rank(&b.1))
                .then(db.cmp(&da))
                .then(aa.partial_cmp(&ab).unwrap_or(std::cmp::Ordering::Equal))
                .then(a.0.cmp(&b.0))
        });
        face.kind = kinds[0].1.clone();
        face.claims = kinds.iter().map(|(w, _, _)| w.clone()).collect();
        let winner_kind = kinds[0].1.clone();
        face.conflicts = kinds
            .iter()
            .filter(|(_, k, _)| *k != winner_kind && *k != FaceKind::LandClaim)
            .map(|(w, _, _)| w.clone())
            .collect();
    }

    // ---- 9b. seed claims: the face containing the seed IS the
    // region — its boundary is whatever curves enclose it, water
    // included, so flush is definitional.
    for sd in seeds {
        for (fi, face) in faces.iter_mut().enumerate() {
            let rings: Vec<Vec<UnitVec>> = {
                let cy_pts: Vec<Vec<UnitVec>> = {
                    let mut v = Vec::new();
                    for cy in &face.cycles {
                        v.push(
                            cy.iter().map(|&h| vertices[halves[h].origin]).collect::<Vec<_>>(),
                        );
                    }
                    v
                };
                cy_pts
            };
            let signed: f64 = rings.iter().map(|r| cycle_area(r)).sum();
            let w: i32 = rings.iter().map(|r| winding(r, &sd.seed)).sum();
            let target = if signed <= 1e-12 { 0 } else { 1 };
            if w == target {
                face.kind = sd.kind.clone();
                if !face.claims.contains(&sd.id) {
                    face.claims.push(sd.id.clone());
                }
                let _ = fi;
                break;
            }
        }
    }

    // ---- 10. sliver absorption (semantic, deterministic)
    let face_len: Vec<f64> = (0..faces.len())
        .map(|fi| {
            faces[fi]
                .cycles
                .iter()
                .flat_map(|cy| cy.iter())
                .map(|&h| {
                    let e = &edges[halves[h].edge];
                    dist(&vertices[e.a], &vertices[e.b])
                })
                .sum()
        })
        .collect();
    let _ = face_len;
    for fi in 0..faces.len() {
        // On a sphere exactly ONE unclaimed face is legitimate: the
        // wrap (the outside world). Any other background cell is an
        // enclosed pocket — a witness disagreement — and is absorbed
        // structurally, no size threshold involved. Claimed fragments
        // below the sliver floor absorb as before.
        let pocket = faces[fi].kind == FaceKind::Background && fi != wrap_face;
        if !pocket && faces[fi].area >= cfg.sliver_area {
            continue;
        }
        // neighbor with the longest shared boundary
        let mut shared: BTreeMap<usize, f64> = BTreeMap::new();
        for cy in &faces[fi].cycles {
            for &h in cy {
                let nb = halves[halves[h].twin].face;
                if nb != fi {
                    let e = &edges[halves[h].edge];
                    *shared.entry(nb).or_insert(0.0) += dist(&vertices[e.a], &vertices[e.b]);
                }
            }
        }
        // pure longest-shared-boundary: whoever drew most of the
        // pocket's boundary owns the disagreement. If that is the
        // outside world, the bg-bg artifact-ink pass then erases the
        // orphaned ring and merges the pocket into the wrap properly.
        let pick = shared
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap().then(b.0.cmp(a.0)));
        if let Some((&nb, _)) = pick {
            let (kind, claims) = (faces[nb].kind.clone(), faces[nb].claims.clone());
            diagnostics.push(format!(
                "sliver face {fi} ({:.3e} sr) absorbed semantically into face {nb}",
                faces[fi].area
            ));
            faces[fi].kind = kind;
            faces[fi].claims = claims;
        }
    }

        // ---- 10b. structural: an edge with unclaimed cells on BOTH
        // sides bounds nothing anyone claims — floating artifact ink.
        // Remove it and reassemble; the bubbles merge into the wrap.
        let bad2: Vec<(usize, usize)> = (0..edges.len())
            .filter(|&e| {
                let f1 = halves[edges[e].half_ab].face;
                let f2 = halves[edges[e].half_ba].face;
                faces[f1].kind == FaceKind::Background && faces[f2].kind == FaceKind::Background
            })
            .map(|e| edge_keys[e])
            .collect();
        if bad2.is_empty() {
            let _ = (face_rep, wrap_face);
            break 'outer (vertices, edges, halves, faces);
        }
        diagnostics.push(format!(
            "removed {} edge(s) bounding only unclaimed cells (artifact ink)",
            bad2.len()
        ));
        for k in bad2 {
            atomic.remove(&k);
        }
    };

    // ---- 11. rivers: node the overlay into the same vertex pool
    let mut rivers = Vec::new();
    for pl in polylines {
        let mut pts: Vec<UnitVec> = Vec::new();
        for p in &pl.pts {
            // snap to a canonical vertex when within tau_vertex
            let mut best: Option<(f64, usize)> = None;
            for (vi, v) in vertices.iter().enumerate() {
                let d = dist(p, v);
                if d <= cfg.tau_vertex && best.map_or(true, |(bd, _)| d < bd) {
                    best = Some((d, vi));
                }
            }
            let q = match best {
                Some((_, vi)) => vertices[vi],
                None => *p,
            };
            if pts.last().map_or(true, |last| dist(last, &q) > 1e-12) {
                pts.push(q);
            }
        }
        if pts.len() >= 2 {
            rivers.push(RiverPath { id: pl.id.clone(), pts });
        }
    }
    // mark border edges that ARE rivers: an edge whose midpoint lies
    // within tau_edge of any river polyline carries the river attr,
    // so a border river is one canonical edge, styled twice.
    for e in edges.iter_mut() {
        let a = vertices[e.a];
        let b = vertices[e.b];
        let m = match norm3(a.x() + b.x(), a.y() + b.y(), a.z() + b.z()) {
            Some(m) => m,
            None => continue,
        };
        'rl: for r in &rivers {
            for w in r.pts.windows(2) {
                if point_near_arc(&m, &w[0], &w[1], cfg.tau_edge * 2.0) {
                    e.river = true;
                    break 'rl;
                }
            }
        }
    }

    let part = Partition { vertices, edges, halves, faces, rivers, diagnostics };
    let mut violations = part.validate();
    if violations.is_empty() {
        Ok(part)
    } else {
        // a refused build carries its own story out
        violations.extend(part.diagnostics.iter().cloned());
        Err(BuildError::TopologyFailure(violations))
    }
}

/// Intersection of two minor arcs, when it exists and is not merely
/// an endpoint touch (those are handled by clustering).
fn arc_intersection(s: &Seg, t: &Seg, tol: f64) -> Option<UnitVec> {
    let (n1x, n1y, n1z) = s.a.cross_raw(&s.b);
    let (n2x, n2y, n2z) = t.a.cross_raw(&t.b);
    // p = n1 × n2
    let px = n1y * n2z - n1z * n2y;
    let py = n1z * n2x - n1x * n2z;
    let pz = n1x * n2y - n1y * n2x;
    let nn = (px * px + py * py + pz * pz).sqrt();
    if nn < 1e-12 {
        return None; // near-coincident great circles: merging handles them
    }
    let p = UnitVec::normalize(px, py, pz).ok()?;
    let q = UnitVec::normalize(-px, -py, -pz).ok()?;
    for cand in [p, q] {
        if on_arc(&cand, &s.a, &s.b, tol) && on_arc(&cand, &t.a, &t.b, tol) {
            // strictly interior on at least one arc, else clustering owns it
            let end_touch = |x: &UnitVec, s: &Seg| x.angle_to(&s.a) <= tol || x.angle_to(&s.b) <= tol;
            if !(end_touch(&cand, s) && end_touch(&cand, t)) {
                return Some(cand);
            }
        }
    }
    None
}

fn on_arc(p: &UnitVec, a: &UnitVec, b: &UnitVec, tol: f64) -> bool {
    let full = a.angle_to(b);
    (p.angle_to(a) + p.angle_to(b) - full).abs() <= tol
}

fn point_near_arc(p: &UnitVec, a: &UnitVec, b: &UnitVec, tol: f64) -> bool {
    let (nx, ny, nz) = a.cross_raw(&b);
    let nn = (nx * nx + ny * ny + nz * nz).sqrt();
    if nn < 1e-12 {
        return false;
    }
    let off = ((p.x() * nx + p.y() * ny + p.z() * nz) / nn).abs().asin();
    off <= tol && on_arc(p, a, b, tol * 4.0)
}

/// Deterministic content hash of a witness set — used by tests to
/// state order-invariance without exposing internals.
pub fn witness_hash(regions: &[WitnessRegion]) -> u64 {
    let mut keys: Vec<Vec<u8>> = regions
        .iter()
        .map(|w| {
            let mut h = Fnv::new();
            h.bytes(w.id.as_bytes());
            h.finish().to_be_bytes().to_vec()
        })
        .collect();
    keys.sort();
    let mut h = Fnv::new();
    for k in keys {
        h.bytes(&k);
    }
    h.finish()
}
