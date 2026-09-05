# The limb: clipping, filling, and clicking

**Date:** 2026-09-05
**Component:** `crates/map-viewer/src/page.html` — the retained WebGL renderer
**Companion:** `2026-09-05-parity-at-the-limb.md` (why fills break at the rim)

**Status (final, end of 2026-09-05, `limb-laws` branch)**

| Bug | State |
|---|---|
| Click falls through at the rim | **FIXED.** The hit test unprojects the click to a unit vector and asks `insideRing` against the original rings, even-odd across a region's rings; `it.whole` sentinel items are unhittable. Verified with distinct-key + ocean-null sampling and an SVG DOM differential. |
| Fill parity wrong in a crescent at the rim | **FIXED.** The "one unsolved geometry problem" (Part 3) was solved as `inside_ring` in map-types; `clip_ring_front` was rebuilt on crossing azimuths sorted on the limb, ported to `crates/map-viewer/src/limb.js`, and wired into the parity passes with the discard disabled for fills only. All seven battery cameras clean, mid-drag == at-rest. |

The analysis below is kept as written — it is the map that led to the fix.

Nothing from this note is in the tree. The whole attempt lives at
`docs/notes/patches/clip-and-hittest-attempt.patch`; `page.html`
is clean at `a2546ee`. The *analysis* below — the mechanism of the click
bug, the visible-ring machinery, the fill diagnosis — remains accurate
and is the starting point for the next attempt.

This note is written assuming no graphics background. Every function used is
given with its signature and what each parameter means.

---

## Part 0 — Vocabulary you need first

**The limb** is the circular outline of the globe — the horizon. Everything
inside it faces you; everything behind it is on the far side of the planet.
"Limb" is the codebase's word for it throughout.

**A ring** is one closed outline: the coast of a landmass, the shore of a lake.
A country is one or more rings. Stored as points **on the unit sphere** —
`(x, y, z)` with `x² + y² + z² = 1` — not as latitude/longitude, because
rotation is then just a dot product.

**A unit vector** is a direction of length 1. Every point on the globe is one.

**The camera basis** is three perpendicular unit vectors describing where you
are looking from:

| | meaning |
|---|---|
| `C` | **centre** — the point of the globe facing you dead-on |
| `E` | **east** — screen-right, at that point |
| `N` | **north** — screen-up, at that point |

**The dot product** `dot(a, b) = a.x*b.x + a.y*b.y + a.z*b.z`. For two unit
vectors this is the cosine of the angle between them. The one fact used
everywhere in this file:

```
dot(p, C) > 0   ->  p faces the camera   (visible)
dot(p, C) = 0   ->  p is exactly on the limb
dot(p, C) < 0   ->  p is on the far side (hidden)
```

That single sign test is the entire notion of "visible" in this renderer.

**Projecting** a point to the screen, once you have the basis, is two dot
products: `screen_x = dot(p, E)`, `screen_y = dot(p, N)`. That is orthographic
projection — like photographing the globe from very far away.

---

## Part 1 — The click bug (attempt reverted; analysis stands)

### What you saw

Clicking a country near the edge of the globe either did nothing or selected
some *other* country underneath it.

### The old code

`gpuHitTest(e)` — takes the browser's mouse event, returns a selection key
string like `"region:4b413fa2bf7ab2ee"`, or `null` for a miss.

```js
let inside = false, behind = false;
for (const id of it.ids) {                     // it.ids: the region's rings
  const r = gpu.cache.get(id);                 // r.verts is a Float32Array of xyz
  const v = r.verts, n = v.length / 3;         // n = number of points
  const pts = new Float64Array(n * 2);
  for (let j = 0; j < n; j++) {
    const pr = proj.project(v[j*3], v[j*3+1], v[j*3+2]);
    if (pr[2] < 0) { behind = true; break; }   // <-- THE BUG
    pts[j*2] = pr[0];
    pts[j*2+1] = pr[1];
  }
  if (behind) break;
  /* ...even-odd crossing test in 2D... */
}
if (!behind && inside) return it.key;
```

`proj.project(x, y, z)` takes the three components of a point on the sphere and
returns `[pageX, pageY, front]`, where `front` is that `dot(p, C)` sign test —
negative means the point is round the back.

### Why it failed

Read the marked line as English: *"if any single point of this country is round
the back of the globe, give up on the whole country."*

Russia has thousands of points. Rotate until one of them slips behind the
horizon and Russia becomes unclickable — while still being fully drawn and
sitting right under your cursor. The loop then continues to the next region in
paint order, so your click silently lands on whatever is beneath. Hence
"unpredictable."

The check exists for a real reason: a hidden point cannot be projected sensibly.
Two points on opposite sides of the planet can land on the same screen pixel.
The old code's response was to bail out. The correct response is to remove the
hidden points first.

### The fix

```js
let inside = false;
for (const id of it.ids) {
  const r = gpu.cache.get(id);
  if (!r || r.state !== "gpu") continue;
  const v = visibleRing(r, proj, camKey);   // <-- clipped to the visible half
  if (!v) continue;                         // wholly hidden: no contribution
  const n = v.length / 3;
  const pts = new Float64Array(n * 2);
  for (let j = 0; j < n; j++) {
    const pr = proj.project(v[j*3], v[j*3+1], v[j*3+2]);
    pts[j*2]     = pr[0];                   // no `behind` check needed:
    pts[j*2 + 1] = pr[1];                   // every point here already faces us
  }
  for (let j = 0; j < n; j++) {
    const jx = pts[j*2],           jy = pts[j*2 + 1];
    const kx = pts[((j+1)%n)*2],   ky = pts[((j+1)%n)*2 + 1];
    if ((jy > y) !== (ky > y) && x < jx + (y - jy)/(ky - jy) * (kx - jx)) inside = !inside;
  }
}
if (inside) return it.key;
```

`visibleRing` hands back a ring guaranteed to be entirely front-facing, so the
`behind` bail-out has nothing left to guard against and is deleted. Countries at
the rim are now clickable.

### That last loop, line by line

This is the **even-odd** test — the same rule the fill obeys, in 2D. Imagine
firing a ray from your cursor straight left and counting how many edges it
crosses. Odd = inside.

```js
for (let j = 0; j < n; j++) {
  const k = (j + 1) % n;              // next point; % n wraps back to 0 so the
                                      // ring closes without storing a duplicate
```

```js
  if ((jy > y) !== (ky > y) &&
```
Does this edge straddle the cursor's horizontal line? One endpoint above and one
below means yes. `!==` on two booleans is "exactly one of these is true."

```js
      x < jx + (y - jy) / (ky - jy) * (kx - jx))
```
Where does the edge cross that line? `(y - jy)/(ky - jy)` is how far down the
edge the crossing sits, as a fraction from 0 to 1; multiplying by the edge's
horizontal run and adding the start gives the crossing's x. If the cursor is to
the *left* of it, the leftward ray crosses this edge.

```js
    inside = !inside;                 // toggle: this is the odd/even count
}
```

No counter needed — flipping a boolean *is* counting parity. Note `inside` is
**not** reset between rings, which is what makes lakes work: a click inside a
lake inside an island toggles twice and comes out false.

### The regression — how "verified" was wrong

The verification sampled clicks from the view centre to the rim and saw
non-null keys at 0.88 of the radius, where the old code returned `null`.
Called success. **The evidence of failure was in the same output:**

```
{"f":0.24,"key":"region:4b413fa2bf7ab2ee"}
{"f":0.60,"key":"region:4b413fa2bf7ab2ee"}
{"f":0.88,"key":"region:4b413fa2bf7ab2ee"}
```

Every hit is the **same key** — and `4b413fa2bf7ab2ee` had already been
identified by the instrumentation as the whole-sphere ocean sentinel
(`whole: true`, 6838 rings blanketing the planet). One region was
swallowing every click. In the live app, clicking any landmass selected
"the whole world" as the subject and blanked the map.

**Mechanism:** the sentinel's rings cover the globe, so some are always
behind the horizon. The old `behind` bail-out — the very line being
fixed — was incidentally the only thing keeping the sentinel out of the
hit test. Deleting the bail-out made the sentinel eligible, it is tested
early and contains essentially every point, so it captured everything.

**For the next attempt, two requirements:**

1. Skip `it.whole` items in the hit test. A sentinel is a backdrop, not
   a subject. (Likely also `it.ids.length === 0` fills, which are pure
   page dressing.)
2. Verification must assert **distinct keys at distinct sample points**,
   and that a click on open ocean returns `null`. "Non-null where it was
   null before" is satisfiable by exactly this failure.

A healthy baseline, measured post-rollback at the same camera
(`lat 20, lon 60, zoom 90`): `f=0.24 -> region:f4ed2527d043aa05`, most
other samples `null`. Distinct real regions, misses on ocean.

---

## Part 2 — The visible ring

Both bugs want the same missing thing: **a ring cut down to the half of the
globe facing the camera.** The server has always had this. The GPU never did.

### `capOf(verts)` — the cheap pre-filter

```js
function capOf(verts)
//   verts : Float32Array   xyz triples for one ring
//   returns { ax, ay, az, cosR } | null
```

A **spherical cap** is a circular patch of the globe — think of the area a
spotlight makes. It is described by an axis (which way the spotlight points) and
a radius. `ax, ay, az` are the axis; `cosR` is the **cosine** of the angular
radius, stored as a cosine so the test below needs no trigonometry.

```js
let ax = 0, ay = 0, az = 0;
for (let i = 0; i < n; i++) { ax += verts[i*3]; ay += verts[i*3+1]; az += verts[i*3+2]; }
const l = Math.hypot(ax, ay, az);
if (l < 1e-9) return null;
ax /= l; ay /= l; az /= l;
```
Average all the points and renormalise to length 1 — roughly "the middle of this
ring." `Math.hypot(x,y,z)` is `sqrt(x²+y²+z²)`, the length. The `l < 1e-9` guard
catches a ring so spread out that its points cancel to nearly nothing, leaving
no meaningful centre.

```js
let cosR = 1;
for (let i = 0; i < n; i++) {
  const d = ax*verts[i*3] + ay*verts[i*3+1] + az*verts[i*3+2];
  if (d < cosR) cosR = d;
}
```
The point furthest from that centre. Smallest cosine = largest angle. Now the
cap provably contains the whole ring.

**Why it matters:** without it, every ring pays a full scan every frame. Measured
at **557 ms per frame**. With it, the common cases cost one dot product and the
p95 frame time roughly halved (1117 ms -> 599 ms in the software renderer).

### `visibleRing(r, proj, camKey)` — the dispatcher

```js
function visibleRing(r, proj, camKey)
//   r      : cache entry  — .verts (Float32Array), .vao (GPU buffer), .cap
//   proj   : projector    — .C/.E/.N camera basis, .mode (0 globe, 1 flat)
//   camKey : string       — camera identity, so a cached clip can be reused
//   returns r.verts (wholly visible) | null (wholly hidden) | Float32Array (clipped)
```

```js
const cap = r.cap;
if (cap && cap.cosR > 0) {
  const t = cap.ax*proj.C[0] + cap.ay*proj.C[1] + cap.az*proj.C[2];
  const sinR = Math.sqrt(1 - cap.cosR * cap.cosR);
  if (t >=  sinR) return r.verts;   // whole cap in front  -> nothing to do
  if (t <= -sinR) return null;      // whole cap behind    -> drop it
}
```

`t` is the cosine of the angle between the cap's axis and the camera centre.
`sinR` converts the cap's radius from cosine to sine via `sin² + cos² = 1`. The
comparison is trigonometry collapsed into arithmetic: the cap lies entirely in
front when *its own centre, minus its radius,* is still on the near side. The
guard `cosR > 0` skips caps wider than a hemisphere, where the reasoning does not
hold — those fall through to the exact path.

Three outcomes, deliberately distinguished by **return type**:

| return | meaning | caller does |
|---|---|---|
| `r.verts` (same object) | wholly visible | keep using the ring's own GPU buffer |
| `null` | wholly hidden | skip entirely |
| a new `Float32Array` | straddles the limb | use the clipped points |

The caller distinguishes the first from the third with `vr === r.verts` — an
identity check, not a copy.

### `clipRingFront(v, C, E, N)` — the real work

```js
function clipRingFront(v, C, E, N)
//   v : Float32Array  xyz triples, cyclic (no repeated closing point)
//   C : number[3]     camera centre    — the same vector the shader gets as uCenter
//   E : number[3]     camera east      — uEast
//   N : number[3]     camera north     — uNorth
//   returns v | null | Float32Array
```

Reusing the shader's own `C/E/N` is deliberate: a point this function places on
the limb lands exactly on the limb circle when the shader draws it. Two
implementations of one geometry, guaranteed to agree.

**Step 1 — classify every point.**

```js
const d = new Float64Array(n);
let anyFront = false, anyBehind = false;
for (let i = 0; i < n; i++) {
  d[i] = v[i*3]*C[0] + v[i*3+1]*C[1] + v[i*3+2]*C[2];
  if (d[i] >= 0) anyFront = true; else anyBehind = true;
}
if (!anyFront)  return null;   // nothing visible
if (!anyBehind) return v;      // nothing hidden — hand back the original
```

`d[i]` is that point's signed distance to the plane through the globe's centre
perpendicular to your view. Positive = facing you. `Float64Array` (not 32) keeps
precision where the sign decides everything.

**Step 2 — walk the ring, starting from a visible point.**

```js
let start = 0;
while (d[start] < 0) start++;
let i = start, walked = 0;
while (walked < n) {
  const k = i % n;
  if (d[k] >= 0) { out.push(v[k*3], v[k*3+1], v[k*3+2]); i++; walked++; continue; }
```
Visible points are copied straight through. `i % n` wraps around the ring;
`walked` counts so we stop after exactly one lap.

**Step 3 — a hidden stretch: leave the hemisphere, cross the limb, come back.**

```js
const p = (i + n - 1) % n;                       // the last visible point
const exit = frontCrossing(v, p*3, k*3, d[p], d[k]);
```

```js
function frontCrossing(v, ia, ib, da, db)
//   v      : the vertex array
//   ia, ib : indices INTO that array (already multiplied by 3)
//   da, db : the two signed distances, which must have opposite signs
//   returns [x, y, z] — a unit vector exactly on the limb
```
```js
const t = da / (da - db);
```
This is the only interesting line. `da` is positive, `db` negative, so `da - db`
is their total spread and `t` is the fraction along the edge where the sign
flips — where the outline crosses the horizon. `t = 0.25` means a quarter of the
way. Then interpolate and renormalise back onto the sphere:
```js
const x = v[ia] + t * (v[ib] - v[ia]);           // ... y, z likewise
const l = Math.hypot(x, y, z) || 1;
return [x/l, y/l, z/l];
```
The `|| 1` guards a zero length so we never divide by zero.

**Step 4 — walk the limb from the exit round to the re-entry.** This is the step
that is still wrong; see Part 3.

```js
const steps = Math.max(1, Math.ceil(Math.abs(sweep) / LIMB_STEP));
for (let s = 1; s < steps; s++) {
  const th = a0 + sweep * s / steps;
  const c1 = Math.cos(th), s1 = Math.sin(th);
  out.push(E[0]*c1 + N[0]*s1, E[1]*c1 + N[1]*s1, E[2]*c1 + N[2]*s1);
}
```
`E*cos(th) + N*sin(th)` walks the limb circle: at `th = 0` you are at east, at
`th = π/2` at north. Every point produced is a unit vector exactly on the limb.
`LIMB_STEP = 0.06` radians matches the server's step, so both close a ring on the
same curve.

---

## Part 3 — Why the fill is still broken

### What was tried

Draw the **clipped** rings in the parity passes and delete the `discard`. With
every vertex front-facing there is nothing to discard, so the crescent artifact
described in the companion note disappears at its source.

The rim slivers did vanish. But at Pacific-centred views the **entire ocean**
rendered as bare paper. Reverted.

### What the instrumentation found

At `lat 20, lon -150` the view centre is open ocean, so no land ring should
contain it. Testing each clipped ring for containment of that exact point:

```
centre: [600, 600]
containedBy: [ { id: "c329d4288f", orig: 4297, clip: 1378, straddler: true } ]
netParity: 1
```

One ring — 4297 points, straddling the limb — **wrongly contains the middle of
the Pacific.** The ocean is drawn as the whole disc with land-shaped holes
punched by parity, so the disc contributes 1, this ring wrongly contributes 1,
and `1 XOR 1 = 0`: the ocean does not paint and you see the layer beneath.

### The root cause

Step 4 above decides *which way round* the limb to travel by accumulating the
azimuth of the **hidden** points:

```js
sweep += wrapPi(a - prev);
```

Azimuth about the view axis is the angle you get from
`atan2(dot(p,N), dot(p,E))`. For a point near the **antipode** — the spot
directly opposite the camera — both of those dot products are nearly zero, and
`atan2(≈0, ≈0)` is numerically meaningless: it swings wildly on rounding error.
A long hidden stretch passing near there accumulates junk winding, and the arc
gets drawn the wrong way round — sometimes right around the disc, enclosing
everything.

**This flaw is inherited.** `clipRingFront` is a faithful port of
`clip_ring_front` in `crates/map-encoders/src/lib.rs:562`, sweep accumulation
included. That is why the SVG renderer *also* inverts the Pacific: one algorithm,
one bug, two renderers. Porting it faithfully reproduced it faithfully.

### The fix that is actually needed

Stop deriving the arc direction from hidden points. Every **crossing** lies
exactly on the limb, where `dot(p,E)` and `dot(p,N)` are on the unit circle and
`atan2` is perfectly well-conditioned. So:

1. Collect all crossings with their azimuths on the limb.
2. Pair each exit with its re-entry by ring order (already correct today).
3. Choose the arc direction from the crossings' own azimuths, never from hidden
   vertices.

The wrinkle is that the project fills by **even-odd**, which does not require
rings to be consistently wound, so "keep the interior on the left" is not
available as a tie-breaker. The robust rule is to test whether a candidate arc's
midpoint lies inside the original spherical ring — a spherical point-in-polygon
test that does not exist in the codebase yet. That is the one missing piece, and
it is a real geometry task rather than a patch.

Fixing it in `clip_ring_front` **and** porting would repair both renderers at
once. Given the SVG path is being retired, fixing the JS and letting the Rust
keep its bug is also defensible.

---

## What shipped

**Nothing.** The whole attempt — `capOf`, `camKeyOf`, `clipRingFront`,
`visibleRing`, the rewritten hit test — was reverted after the click
regression (see *The regression* in Part 1): the hit-test change let the
whole-sphere ocean sentinel capture every click, which selected "the
whole world" and blanked the map. `page.html` is clean at `a2546ee`; the
code is preserved at
`docs/notes/patches/clip-and-hittest-attempt.patch`.

The next attempt should carry the patch forward with two changes: the
hit test skips `it.whole` sentinel items, and the fill keeps the old
discard path until `clipRingFront`'s limb-arc direction is made robust
(Part 3). Both open bugs — clicks falling through at the rim, and the
fill crescent — remain live in the tree today.

---

## Things that cost time, recorded so they do not again

- `page.html` is `include_str!`'d into the binary (`lib.rs:29`). Editing it does
  nothing until `cargo build --release -p map-viewer` **and** a restart.
- `zoom` is **degrees of half-extent**: `90` is the whole hemisphere (fully
  zoomed out), small values are zoomed in. `setCamera(lat, lon, 1)` is a
  1-degree window, not a globe.
- Frame timings from the headless software renderer are dominated by
  rasterisation, not JavaScript. The pre-change baseline measured *slower*
  (966 ms) than the un-optimised fix (557 ms). Use these numbers only to compare
  runs in the same environment; they say nothing about real GPU performance.
- `state.setCamera(lat, lon, zoom)` sets the target and the eased view together,
  so a replayed camera is bit-identical to a live one. That is what proved the
  artifact is camera-angle dependent, not motion dependent.
