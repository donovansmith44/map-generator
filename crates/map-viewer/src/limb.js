// ------------------------------------------------------ the limb laws
//
// A pure port of the spherical containment law (map-types geom.rs
// inside_ring) and the limb clip built on it (map-encoders
// clip_ring_front). PORTED, NOT REDESIGNED: one algorithm, two
// languages, held together by the shared fixtures in
// crates/map-viewer/tests/fixtures/limb.json — change one side and
// the cross-language test tells on you.
//
// Rings are flat arrays of xyz triples (Float32Array or number[]),
// cyclic with no repeated closing point. Points are [x, y, z] unit
// vectors. No DOM, no state: loadable in Node for tests and in the
// page for the renderer.
//
// THE CONTAINMENT LAW: a closed SIMPLE ring divides the sphere into
// two components; the INTERIOR is the smaller-area component (an
// exact half-sphere tie goes to the left of traversal). Membership is
// decided by even-odd crossings of the geodesic from the query point
// to a reference of known status, with the same half-open sign
// convention as the classic 2D ray cast. The whole-sphere sentinel
// (coversSphere) contains everything, by decree.

"use strict";

// The whole-sphere sentinel (RegionPart's empty-cycle convention): at
// most five stored points containing a near-antipodal pair. Its
// interior is everything.
function coversSphere(ring) {
  const n = ring.length / 3;
  if (n > 5) return false;
  for (let i = 0; i < n; i++) {
    for (let j = 0; j < n; j++) {
      const d = ring[i * 3] * ring[j * 3] + ring[i * 3 + 1] * ring[j * 3 + 1] + ring[i * 3 + 2] * ring[j * 3 + 2];
      if (d < -0.99) return true;
    }
  }
  return false;
}

// Signed-side test with the half-open convention: exactly on the
// great circle counts as the negative side.
function side(nx, ny, nz, x, y, z) {
  return nx * x + ny * y + nz * z > 0;
}

// Does the geodesic p->r cross the ring edge a->b (indices into the
// flat array)? Both arcs are the shorter great-circle arcs. n1 is
// cross(p, r), passed in once per query. The two circles meet at the
// antipodal pair +/-(n1 x n2); the straddle tests guarantee each arc
// crosses the OTHER's circle exactly once, at whichever of +/-x lies
// nearer that arc's midpoint — the arcs cross each other iff the
// midpoint dots agree in sign (robust even at vertex-exact hits).
function arcsCross(p, r, n1x, n1y, n1z, v, ia, ib) {
  const ax = v[ia], ay = v[ia + 1], az = v[ia + 2];
  const bx = v[ib], by = v[ib + 1], bz = v[ib + 2];
  if (side(n1x, n1y, n1z, ax, ay, az) === side(n1x, n1y, n1z, bx, by, bz)) return false;
  const n2x = ay * bz - az * by, n2y = az * bx - ax * bz, n2z = ax * by - ay * bx;
  if (
    side(n2x, n2y, n2z, p[0], p[1], p[2]) === side(n2x, n2y, n2z, r[0], r[1], r[2])
  ) return false;
  const xx = n1y * n2z - n1z * n2y, xy = n1z * n2x - n1x * n2z, xz = n1x * n2y - n1y * n2x;
  const dAb = xx * (ax + bx) + xy * (ay + by) + xz * (az + bz);
  const dPr = xx * (p[0] + r[0]) + xy * (p[1] + r[1]) + xz * (p[2] + r[2]);
  return dAb * dPr > 0;
}

// Crossing parity of the geodesic p->r against every ring edge
// (excluding index `skip`, for the labelling walk that starts ON that
// edge). True = odd = p and r in different components.
function crossingParity(p, r, v, skip) {
  const n = v.length / 3;
  const n1x = p[1] * r[2] - p[2] * r[1];
  const n1y = p[2] * r[0] - p[0] * r[2];
  const n1z = p[0] * r[1] - p[1] * r[0];
  let odd = false;
  for (let i = 0; i < n; i++) {
    if (i === skip) continue;
    if (arcsCross(p, r, n1x, n1y, n1z, v, i * 3, ((i + 1) % n) * 3)) odd = !odd;
  }
  return odd;
}

// The ring's bounding cap: vertex-mean axis and the cosine of the
// angular radius. null when the vertices cancel to no centre.
function ringCap(v) {
  const n = v.length / 3;
  let ax = 0, ay = 0, az = 0;
  for (let i = 0; i < n; i++) { ax += v[i * 3]; ay += v[i * 3 + 1]; az += v[i * 3 + 2]; }
  const l = Math.hypot(ax, ay, az);
  if (l < 1e-9) return null;
  ax /= l; ay /= l; az /= l;
  let cosR = 1;
  for (let i = 0; i < n; i++) {
    const d = ax * v[i * 3] + ay * v[i * 3 + 1] + az * v[i * 3 + 2];
    if (d < cosR) cosR = d;
  }
  return { ax, ay, az, cosR: Math.max(-1, Math.min(1, cosR)) };
}

// A reference point provably outside the cap, never near-antipodal to
// p. The axis's antipode serves unless p sits near the axis; the
// fallback steps off along a perpendicular, halfway between the cap
// rim and the antipode — still outside the cap by construction.
function outsideReference(p, ax, ay, az, cosR) {
  if (p[0] * -ax + p[1] * -ay + p[2] * -az > -0.999) return [-ax, -ay, -az];
  let ex = -ay, ey = ax, ez = 0;
  const el = Math.hypot(ex, ey, ez);
  if (el < 1e-9) { ex = 0; ey = 1; ez = 0; } else { ex /= el; ey /= el; ez /= el; }
  const phi = (Math.acos(cosR) + Math.PI) / 2;
  const c = Math.cos(phi), s = Math.sin(phi);
  const x = ax * c + ex * s, y = ay * c + ey * s, z = az * c + ez * s;
  const l = Math.hypot(x, y, z) || 1;
  return [x / l, y / l, z / l];
}

// The general path, no cap assumption: label the two components by
// area (Gauss-Bonnet turning angles) and decide membership by parity
// against the labelled side.
function insideRingGeneral(p, ring) {
  // Drop consecutive duplicates (and trailing repeats of the first).
  const raw = ring, rn = raw.length / 3;
  const pts = [];
  for (let i = 0; i < rn; i++) {
    const x = raw[i * 3], y = raw[i * 3 + 1], z = raw[i * 3 + 2];
    const m = pts.length - 3;
    if (m < 0 || pts[m] * x + pts[m + 1] * y + pts[m + 2] * z < 1 - 1e-12) pts.push(x, y, z);
  }
  while (
    pts.length > 3 &&
    pts[0] * pts[pts.length - 3] + pts[1] * pts[pts.length - 2] + pts[2] * pts[pts.length - 1] >= 1 - 1e-12
  ) pts.length -= 3;
  const n = pts.length / 3;
  if (n < 3) return false;

  // Area of the LEFT-of-traversal component: 2 pi minus the summed
  // signed turning angles (left turns positive about the outward
  // normal).
  let turn = 0;
  for (let i = 0; i < n; i++) {
    const u = ((i + n - 1) % n) * 3, vi = i * 3, w = ((i + 1) % n) * 3;
    const ux = pts[u], uy = pts[u + 1], uz = pts[u + 2];
    const vx = pts[vi], vy = pts[vi + 1], vz = pts[vi + 2];
    const wx = pts[w], wy = pts[w + 1], wz = pts[w + 2];
    const duv = ux * vx + uy * vy + uz * vz;
    const dvw = vx * wx + vy * wy + vz * wz;
    let ix = vx * duv - ux, iy = vy * duv - uy, iz = vz * duv - uz;
    let ox = wx - vx * dvw, oy = wy - vy * dvw, oz = wz - vz * dvw;
    const il = Math.hypot(ix, iy, iz), ol = Math.hypot(ox, oy, oz);
    if (il < 1e-9 || ol < 1e-9) continue; // antipodal edge pair: no defined tangent
    ix /= il; iy /= il; iz /= il; ox /= ol; oy /= ol; oz /= ol;
    const cx = iy * oz - iz * oy, cy = iz * ox - ix * oz, cz = ix * oy - iy * ox;
    turn += Math.atan2(cx * vx + cy * vy + cz * vz, ix * ox + iy * oy + iz * oz);
  }
  const TAU = Math.PI * 2;
  const leftArea = TAU - turn;
  const leftIsInterior = leftArea <= TAU;

  // The longest edge anchors the labelling: the geodesic from its
  // midpoint m to the reference meets that edge's great circle only
  // at m itself, so skipping the edge in the count is exact, and the
  // departure side is decided by which side of the edge's circle the
  // reference lies on.
  let eBest = 0, cBest = 2;
  for (let i = 0; i < n; i++) {
    const a = i * 3, b = ((i + 1) % n) * 3;
    const c = pts[a] * pts[b] + pts[a + 1] * pts[b + 1] + pts[a + 2] * pts[b + 2];
    if (c < cBest) { cBest = c; eBest = i; }
  }
  const a = eBest * 3, b = ((eBest + 1) % n) * 3;
  let mx = pts[a] + pts[b], my = pts[a + 1] + pts[b + 1], mz = pts[a + 2] + pts[b + 2];
  const ml = Math.hypot(mx, my, mz);
  mx /= ml; my /= ml; mz /= ml;
  let hx = pts[a + 1] * pts[b + 2] - pts[a + 2] * pts[b + 1];
  let hy = pts[a + 2] * pts[b] - pts[a] * pts[b + 2];
  let hz = pts[a] * pts[b + 1] - pts[a + 1] * pts[b];
  const hl = Math.hypot(hx, hy, hz);
  hx /= hl; hy /= hl; hz /= hl;

  // A reference not near-antipodal to p or m and decisively off the
  // labelling edge's great circle.
  const candidates = [
    [0, 0, 1], [0, 0, -1], [1, 0, 0], [0, 1, 0], [-1, 0, 0], [0, -1, 0],
  ];
  let r = candidates[0];
  for (const cand of candidates) {
    if (
      p[0] * cand[0] + p[1] * cand[1] + p[2] * cand[2] > -0.999 &&
      mx * cand[0] + my * cand[1] + mz * cand[2] > -0.999 &&
      Math.abs(hx * cand[0] + hy * cand[1] + hz * cand[2]) > 1e-6
    ) { r = cand; break; }
  }

  // Parity of the LEFT class: crossings from m to r over the other
  // edges, plus one when r lies to the edge's right.
  let leftParity = crossingParity([mx, my, mz], r, new Float64Array(pts), eBest);
  if (!side(hx, hy, hz, r[0], r[1], r[2])) leftParity = !leftParity;
  const pParity = crossingParity(p, r, new Float64Array(pts), -1);
  return (pParity === leftParity) === leftIsInterior;
}

// Is p inside the closed ring, under the smaller-component law?
function insideRing(p, ring) {
  if (ring.length / 3 < 3) return false;
  if (coversSphere(ring)) return true;
  const cap = ringCap(ring);
  if (cap && cap.cosR > 0) {
    const r = outsideReference(p, cap.ax, cap.ay, cap.az, cap.cosR);
    return crossingParity(p, r, ring, -1);
  }
  return insideRingGeneral(p, ring);
}

// ------------------------------------------------------ the limb clip

// Radians between successive points walked along the limb — the same
// curve the server closes a clipped ring on.
const LIMB_STEP = 0.06;

// Where the segment ia->ib pierces the limb plane dot(p, C) = 0,
// returned as a unit vector. da, db are the endpoints' signed
// distances and must straddle the plane.
function frontCrossing(v, ia, ib, da, db) {
  const t = da / (da - db);
  const x = v[ia] + t * (v[ib] - v[ia]);
  const y = v[ia + 1] + t * (v[ib + 1] - v[ia + 1]);
  const z = v[ia + 2] + t * (v[ib + 2] - v[ia + 2]);
  const l = Math.hypot(x, y, z) || 1;
  return [x / l, y / l, z / l];
}

// Clip one ring to the front hemisphere, closing along the limb.
//   v      flat xyz triples, cyclic (no repeated closing point)
//   C,E,N  the camera basis — the SAME vectors the shader receives as
//          uCenter/uEast/uNorth, so a limb point here lands exactly on
//          the limb circle there
// Returns v itself when wholly visible (the caller keeps using the
// ring's own GPU buffer), null when nothing is visible, or an array
// of Float32Array loops (a ring may clip into several lobes; even-odd
// filling is indifferent).
//
// THE LIMB RULE, same as the server's: crossings carry azimuths
// measured ON the limb (perfectly conditioned); sorted, they cut the
// limb into arcs, and an arc is kept exactly when its midpoint lies
// inside the original ring. The old sweep over hidden vertices was
// meaningless near the camera antipode — the Pacific inversion.
function clipRingFront(v, C, E, N) {
  const n = v.length / 3;
  if (n < 3) return null;
  const az = (x, y, z) => Math.atan2(x * N[0] + y * N[1] + z * N[2], x * E[0] + y * E[1] + z * E[2]);
  const limbPoint = th => {
    const c1 = Math.cos(th), s1 = Math.sin(th);
    return [E[0] * c1 + N[0] * s1, E[1] * c1 + N[1] * s1, E[2] * c1 + N[2] * s1];
  };

  const d = new Float64Array(n);
  let anyFront = false, anyBehind = false;
  for (let i = 0; i < n; i++) {
    d[i] = v[i * 3] * C[0] + v[i * 3 + 1] * C[1] + v[i * 3 + 2] * C[2];
    if (d[i] >= 0) anyFront = true; else anyBehind = true;
  }
  if (!anyBehind) return v;
  if (!anyFront) {
    // A theorem of the smaller-component law, not a heuristic: the
    // ring-free front hemisphere (area 2 pi) always lies in the
    // LARGER component, so a wholly hidden ring shows nothing. (The
    // whole-sphere sentinel is the caller's business.)
    return null;
  }

  // Visible chains: maximal front runs, bracketed by their crossings.
  let start = 0;
  while (!(d[start] >= 0 && d[(start + n - 1) % n] < 0)) start++;
  const chains = []; // { entry:[xyz], verts:[indices...], exit:[xyz] }
  let cur = null;
  for (let j = 0; j < n; j++) {
    const idx = (start + j) % n;
    const prv = (idx + n - 1) % n;
    if (d[idx] >= 0) {
      if (!cur) cur = { entry: frontCrossing(v, prv * 3, idx * 3, d[prv], d[idx]), verts: [] };
      cur.verts.push(idx);
    } else if (cur) {
      cur.exit = frontCrossing(v, prv * 3, idx * 3, d[prv], d[idx]);
      chains.push(cur);
      cur = null;
    }
  }

  // Crossings sorted by limb azimuth; the arcs between consecutive
  // crossings are inside the ring exactly when their midpoints are.
  const crossings = [];
  for (let k = 0; k < chains.length; k++) {
    const ch = chains[k];
    crossings.push({ az: az(ch.entry[0], ch.entry[1], ch.entry[2]), chain: k, isEntry: true });
    crossings.push({ az: az(ch.exit[0], ch.exit[1], ch.exit[2]), chain: k, isEntry: false });
  }
  crossings.sort((x, y) => x.az - y.az);
  const m = crossings.length;
  const TAU = Math.PI * 2;
  const gap = j => {
    const hi = crossings[(j + 1) % m].az + (j + 1 === m ? TAU : 0);
    return hi - crossings[j].az;
  };
  const arcInside = new Array(m);
  for (let j = 0; j < m; j++) {
    const g = gap(j);
    const mid = limbPoint(crossings[j].az + g / 2);
    arcInside[j] = g > 1e-12 && insideRing(mid, v);
  }
  // Two-regularity repair for tangential grazes: every crossing needs
  // one interior arc beside it; starve cases get the narrower one.
  for (let j = 0; j < m; j++) {
    const before = (j + m - 1) % m;
    if (!arcInside[j] && !arcInside[before]) {
      if (gap(j) <= gap(before)) arcInside[j] = true; else arcInside[before] = true;
    }
  }

  // Stitch: chains and interior limb arcs alternate around each loop.
  const slot = (k, isEntry) => crossings.findIndex(x => x.chain === k && x.isEntry === isEntry);
  const used = new Array(chains.length).fill(false);
  const out = [];
  for (let k0 = 0; k0 < chains.length; k0++) {
    if (used[k0]) continue;
    const startSlot = slot(k0, true);
    const run = [];
    let k = k0, enterAtEntry = true;
    for (let guard = 0; guard < chains.length; guard++) {
      used[k] = true;
      const ch = chains[k];
      let leave;
      if (enterAtEntry) {
        run.push(ch.entry[0], ch.entry[1], ch.entry[2]);
        for (const idx of ch.verts) run.push(v[idx * 3], v[idx * 3 + 1], v[idx * 3 + 2]);
        run.push(ch.exit[0], ch.exit[1], ch.exit[2]);
        leave = slot(k, false);
      } else {
        run.push(ch.exit[0], ch.exit[1], ch.exit[2]);
        for (let q = ch.verts.length - 1; q >= 0; q--) {
          const idx = ch.verts[q];
          run.push(v[idx * 3], v[idx * 3 + 1], v[idx * 3 + 2]);
        }
        run.push(ch.entry[0], ch.entry[1], ch.entry[2]);
        leave = slot(k, true);
      }
      const before = (leave + m - 1) % m;
      let next, a0, sweep;
      if (arcInside[leave]) {
        next = (leave + 1) % m; a0 = crossings[leave].az; sweep = gap(leave);
      } else {
        next = before; a0 = crossings[leave].az; sweep = -gap(before);
      }
      const steps = Math.max(1, Math.ceil(Math.abs(sweep) / LIMB_STEP));
      for (let s = 1; s < steps; s++) {
        const q = limbPoint(a0 + sweep * s / steps);
        run.push(q[0], q[1], q[2]);
      }
      if (next === startSlot) break;
      k = crossings[next].chain;
      enterAtEntry = crossings[next].isEntry;
    }
    out.push(new Float32Array(run));
  }
  return out;
}

// The camera basis the projectors and the server both derive from a
// centre vector: east = normalize(-Cy, Cx, 0) (pole fallback: +Y),
// north = C x E. Exposed so tests build the SAME basis.
function limbBasis(C) {
  let E = [-C[1], C[0], 0];
  const el = Math.hypot(E[0], E[1], E[2]);
  E = el < 1e-9 ? [0, 1, 0] : [E[0] / el, E[1] / el, E[2] / el];
  let N = [C[1] * E[2] - C[2] * E[1], C[2] * E[0] - C[0] * E[2], C[0] * E[1] - C[1] * E[0]];
  const nl = Math.hypot(N[0], N[1], N[2]);
  N = nl < 1e-9 ? [0, 0, 1] : [N[0] / nl, N[1] / nl, N[2] / nl];
  return { E, N };
}

if (typeof module !== "undefined" && module.exports) {
  module.exports = {
    coversSphere,
    insideRing,
    insideRingGeneral,
    ringCap,
    clipRingFront,
    frontCrossing,
    limbBasis,
    LIMB_STEP,
  };
}
