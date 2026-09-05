# Parity at the limb: why fills break at the globe's edge

**Date:** 2026-09-05
**Component:** `crates/map-viewer/src/page.html` — the retained WebGL renderer
**Status:** root cause confirmed by experiment; **FIXED later this same day**
on the `limb-laws` branch, by the law-first route this note's *The real fix*
section called for: a spherical containment law (`inside_ring`, map-types),
a crossing-azimuth rebuild of `clip_ring_front` on top of it, a JS port
(`crates/map-viewer/src/limb.js`) held to the Rust by shared fixtures, and
fill parity passes that draw clipped rings with the discard disabled for
fills only. The earlier history below is kept as written — it is the record
of how the fix was found. The failed first attempt remains at
`docs/notes/patches/parity-attempt.patch`.

---

## The reported symptoms

1. Spinning the globe shows "distorted shapes of countries **through** the globe"
   — landmasses appearing where ocean should be, at the rim.
2. The centre of the view is always fine. Trouble is exclusive to the edges.
3. Clicking in certain places changes the map unpredictably.

Symptoms 1 and 2 share one root cause. Symptom 3 is a separate defect with the
same trigger (see *Adjacent defects*).

---

## Background: how the retained renderer draws the map

The camera is **not** baked into the geometry. In `gpuSyncInner` the camera
centre is deliberately dropped from the scene's cache key:

```js
for (const k of ["encoder", "smooth", "projection", "center"]) p.delete(k);
```

So rotating the globe refetches nothing and re-clips nothing. The server ships
the whole world once as unit vectors on a sphere; rotation only changes three
uniforms (`uEast`, `uNorth`, `uCenter`). That is what made rotation fast.

The consequence is the whole story: the browser must solve **per frame, on the
GPU**, a problem the server solves properly on the CPU — *where does the visible
hemisphere end?*

### How the server solves it (the reference)

`clip_ring_front` in `crates/map-encoders/src/lib.rs:562` does true geometric
clipping. Where an outline crosses the horizon it computes the exact crossing
point, then **walks the horizon arc** to the re-entry point, accumulating the
sweep angle so it walks the correct way round. The result is a genuinely closed
ring lying entirely on the visible side. A ring with no front-facing vertex
returns empty and is dropped. Fill that and it is correct by construction.

### How the GPU approximates it

Two mechanisms, neither of which produces a closed correct ring:

1. **Vertex fold** (`page.html:692`) — a vertex behind the horizon is pushed
   radially onto the horizon circle.
   ```glsl
   vFront = dot(p, uCenter);
   vec2 en = vec2(dot(p, uEast), dot(p, uNorth));
   if (vFront < 0.0) { float l = length(en); if (l > 1e-6) en /= l; }
   ```
2. **Fragment discard** (`page.html:721`)
   ```glsl
   if (vFront < uFrontMin) discard;   // behind the limb
   ```

---

## How fills work: stencil even-odd parity

The project fills by the **even-odd rule** — a pixel is inside when a ray from
it crosses the outline an odd number of times. The SVG path states this
literally as `fill-rule="evenodd"`. Holes fall out for free: a ray from inside a
lake crosses the lake shore and the coastline, two crossings, even, unfilled. No
code needs to know which loop is a hole.

The GPU reproduces it with the **stencil parity** technique
(`page.html:1601-1618`). Each ring is drawn as a `TRIANGLE_FAN` anchored at
vertex 0.

**Pass 1 — count coverage, paint nothing:**

```js
gl.colorMask(false, false, false, false);
gl.stencilFunc(gl.ALWAYS, 0, 0xff);
gl.stencilOp(gl.KEEP, gl.KEEP, gl.INVERT);   // every fragment FLIPS the bit
for (const r of rings) { gl.bindVertexArray(r.vao); gl.drawArrays(gl.TRIANGLE_FAN, 0, r.count); }
```

**Pass 2 — paint where parity is odd, clearing as it goes:**

```js
gl.colorMask(true, true, true, true);
gl.stencilFunc(gl.NOTEQUAL, 0, 0xff);
gl.stencilOp(gl.KEEP, gl.KEEP, gl.ZERO);
for (const r of rings) { gl.bindVertexArray(r.vao); gl.drawArrays(gl.TRIANGLE_FAN, 0, r.count); }
```

### The load-bearing invariant

**The fan triangles are not the shape.** For a non-convex outline they spill
into regions that must end up empty. That is not a flaw — it is the mechanism.
The spill **cancels**: every edge radiating from vertex 0 is drawn twice, once
as the trailing edge of one triangle and once as the leading edge of the next,
so those flips undo each other. Only the ring's own outline edges are drawn
once. What survives is exactly the shape.

Worked example — a square with a V notch cut into the top:

```
v0=(0,0)  v1=(6,0)  v2=(6,6)  v3=(3,2)  v4=(0,6)

fan: T1=(v0,v1,v2)  T2=(v0,v2,v3)  T3=(v0,v3,v4)

point P=(3,1)   in the body   covered by T1 only      -> 1, odd  -> filled
point Q=(4,3.5) in the notch  covered by T1 AND T2    -> 2, even -> empty
```

Q is the whole argument. T1 wrongly spills into the notch, T2 wrongly spills
into the same place, and the two errors annihilate.

> **The invariant: parity is only correct if every triangle is drawn in full.**

---

## The bug

`discard` violates that invariant in two compounding ways.

### 1. A discarded fragment never reaches the stencil stage

It is not "flipped to zero" — it skips per-fragment operations entirely, so its
`INVERT` in pass 1 **does not happen**. One half of a cancelling pair goes
missing and the surviving flip is stranded at 1. The pixel ends the frame odd,
so pass 2 paints it: land where there should be ocean.

There is no temporal element. The stencil buffer is cleared every frame
(`gl.clear(... | gl.STENCIL_BUFFER_BIT)`); the failure is entirely within one
frame's pass 1.

### 2. The discard boundary is a chord where the horizon is an arc

`vFront` is computed per vertex and **smoothly interpolated** across each
triangle, so the locus where it crosses zero is a *straight line in screen
space*. The horizon it stands in for is a *circle*. These agree only for tiny
triangles — and fan triangles reach from vertex 0, potentially clear across the
disc, out to the rim. The cut therefore falls well inside the true horizon, and
the crescent between chord and arc receives inverted parity.

This is why the artifact **cannot** appear at the centre of view and **must**
concentrate at the rim: near the centre `vFront` is comfortably positive
everywhere, nothing is discarded, nothing breaks.

### Why the densify commit could not have fixed it

`a2546ee` ("No edge outruns the limb") shortened **outline** edges so a folded
outline hugs the horizon within half a pixel. Real improvement, adjacent
problem. The broken interpolation is across the fan's **interior** triangles
(apex to rim), which stay enormous no matter how finely the outline is
subdivided.

---

## Evidence

- **Angle-dependent, not motion-dependent.** Mid-drag frames were captured along
  with the live camera, then those exact cameras were replayed at rest. The
  frames are identical — both show the slivers. Motion is irrelevant; the
  artifact is a property of the camera angle. (Earlier "it's clean after
  release" was a red herring: the release camera was a benign angle.)
- **Worst reproduction:** `lat 24.03, lon 18.02, zoom 90` — blue wedges slicing
  through South America at the left rim.
- **Causal proof.** Disabling the discard across the two fill passes only, same
  camera, nothing else changed, removed the slivers entirely.
- **Compositing ruled out.** In `gpu-primary` mode both `.layer` and `.portrait`
  are `visibility: hidden` (CSS at `page.html:149`), so every artifact is
  genuinely GL-rendered, not the old SVG layer bleeding through.

---

## The attempt that failed

Saved at `docs/notes/patches/parity-attempt.patch`.

Three changes: drop the discard across both fill passes; cull rings wholly
behind the horizon so a far-side country cannot fold into a phantom polygon;
restore the discard for strokes and markers.

Two cull variants were tried:

1. **Bounding spherical cap** — rejected: a continent's cap spans more than a
   hemisphere, so the test can never reject it even when every vertex is hidden.
2. **Exact per-vertex test** with early exit (matching the server's
   `position(front)` law) — correct, cheap, and *still not sufficient*.

**Result: a regression.** At Pacific-centred views (`lat 20, lon -150`) the
ocean rendered as bare paper. Both cull variants failed identically, which
falsifies the "un-culled far-side rings" hypothesis.

**What that tells us:** the failure is not wholly-behind rings. It is
**straddling** rings. The vertex fold is not a faithful stand-in for the limb
arc — it places each hidden vertex at its own azimuth but never inserts the true
crossing points, and for a ring whose hidden portion sweeps a wide azimuth range
the folded polygon encloses the wrong area. The discard was *masking* that
error. Remove the mask and it inverts parity across the entire disc.

> **Conclusion: the discard cannot be removed without also making the folded
> ring geometrically correct.** They are one change, not two.

---

## The real fix

**Update, later this day:** this port was attempted
(`clipRingFront`/`visibleRing`, preserved in
`docs/notes/patches/clip-and-hittest-attempt.patch`) and hit a
blocker: `clip_ring_front`'s own limb-arc direction rule — accumulating
azimuth over *hidden* vertices — is ill-conditioned near the antipode,
and a faithful port faithfully reproduces the server's Pacific ocean
inversion. The robust rule and the missing primitive it needs (a
spherical point-in-ring test) are worked out in
`2026-09-05-limb-clipping-and-hit-testing.md`, Part 3. The design below
still stands; that flaw must be fixed first, in whichever language keeps
the clip.

Make "a ring clipped to the visible hemisphere" a **value**, the way the server
already does — the asymmetry between the two renderers *is* the bug. On the GPU
the concept exists only smeared across a vertex fold plus a fragment discard, so
nothing can be asserted about it.

A `VisibleRing`:

- constructed only by a clip, never by hand;
- guaranteed closed and entirely front-facing, with the limb arc spliced in and
  the true crossing points inserted (port `clip_ring_front`);
- constructor returns an optional — *wholly behind* becomes a case the type
  forces you to handle rather than one you can forget.

Filling then becomes **total**: any `VisibleRing` fills correctly by parity with
no draw-time clipping whatsoever, and the discard disappears because nothing is
left to discard.

The cost question is where the design work sits: clipping is view-dependent, and
the retained renderer's whole premise is that camera motion does no work. The
plausible shape is to re-clip on the same cadence the scene already re-demands
on (`gpuSyncInner` refreshes when the view walks ~40% of its own radius) and
accept slightly stale limb geometry between refreshes — the same trade already
accepted elsewhere in the pipeline.

---

## Types that would have prevented this

1. **`flat` instead of smooth interpolation.** The smallest exact fix, and a
   real type distinction in GLSL:
   ```glsl
   out float vFront;      // smooth (default) — interpolated: a chord
   flat out int vSide;    // per-vertex, never interpolated
   ```
   `vFront` conflates a per-vertex *classification* with a per-fragment
   *boundary*. Only the first is meaningful. `flat` makes the second
   inexpressible, so the chord-versus-arc error becomes unwritable rather than
   merely discouraged.

2. **`VisibleRing` as above** — make the clipped ring a value with laws.

3. **Pipeline state as a capability, not ambient global.** `uFrontMin` is set
   hundreds of lines from where it is read, and nothing at the call site says
   which mode is active. A `ParityPass` that owns the stencil configuration and
   *cannot be constructed with discard enabled*, plus a `ClippedPass` for
   strokes with discard but no stencil, makes the illegal combination
   unrepresentable.

4. **A differential property test.** Even-odd fill is one law with two
   implementations. Rendering both paths at N random cameras and asserting
   agreement would have caught this on commit. It has a shelf life given the SVG
   path is slated for retirement, but it is exactly the right harness to hold
   *during* that retirement.

---

## Adjacent defects (separate, still open)

- **Click falls through at the rim.** `gpuHitTest` (`page.html:1503`) abandons a
  whole region if *any* vertex is behind the horizon:
  ```js
  if (pr[2] < 0) { behind = true; break; }
  ```
  A country straddling the rim becomes unclickable and the click lands on
  whatever is beneath it in paint order. This is the "clicking changes things
  unpredictably" symptom. The parity fix does not touch it.

  A fix was attempted later this day and **reverted**: removing the bail-out
  let the whole-sphere ocean sentinel capture every click, selecting "the
  whole world" and blanking the map — the `behind` bail-out was incidentally
  the only thing keeping the sentinel out of the hit test. Details, the
  mechanism, and the requirements for the next attempt are in
  `2026-09-05-limb-clipping-and-hit-testing.md` (*The regression*). Still
  open.

- **Initial zoom.** On load the globe overflows the viewport top and bottom.

- **SVG/server ocean inversion** at Pacific-centred views (`lon ≈ ±150`). Noted
  only for completeness — that renderer is slated for retirement.

---

## Reproduction

The workbench serves a pre-compiled canon, so a code change needs all three
steps (see the render-pipeline note): `cargo build --release`, then
`./target/release/map-compile.exe build`, then restart `map-viewer.exe`
detached. For viewer-only changes `cargo build --release -p map-viewer` plus a
restart is enough — `page.html` is `include_str!`'d into the binary, so editing
it without rebuilding changes nothing.

Headless capture (the MCP Playwright build cannot find Chrome; drive
`playwright-core` against the cached Chromium):

```js
chromium.launch({
  headless: true,
  executablePath: 'C:/Users/donov/AppData/Local/ms-playwright/chromium-1234/chrome-win64/chrome.exe',
  args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
// then: click '#gpu', wait ~7s for the scene to settle, and drive the camera:
await page.evaluate(() => state.setCamera(24.03, 18.02, 90));
```

`state.setCamera(lat, lon, zoom)` sets target and eased view together, so a
replayed camera is identical to the live one. **`zoom` is degrees of half-extent:
90 is the whole hemisphere, small values are zoomed in.**
