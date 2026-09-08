// THE GOLDEN VIEWS: the owner's no-regression mandate, as a gate.
// Every era stop of the timeline, probed at the Levant camera (the
// beloved framing around the twelve tribes) and the hemisphere, 25
// colour samples each — blessed from an approved build, checked
// before any renderer or canon change ships.
//
//   node golden.js           bless: overwrite fixtures/golden-views.json
//   node golden.js --check   gate: fail loudly on any drifted probe
//
// Requires playwright-core and the cached Chromium (the project's
// verification harness convention); run from a directory where
// `require('playwright-core')` resolves. Timings are SwiftShader —
// comparative only; the colours are exact.
//
// THE SETTLE LAW (R89). This gate once slept: 1500 ms after load, 700
// ms after each camera move, 900 ms before conceding a drift. Three
// runs against ONE unchanged binary answered HOLD, then REGRESSION 4,
// then REGRESSION 3, then REGRESSION 1 — no two failing sets alike,
// every drifted probe a label glyph caught mid-fade. A gate that
// answers differently each run is not a gate.
//
// The sleeps are gone. Nothing here waits a duration; everything
// OBSERVES a condition the page already keeps for its own animation
// (see `QUIESCENT` below), and then samples until the picture has
// demonstrably stopped changing. The constants that remain are
// ceilings on pathology — they exist to make a wedged page fail
// loudly, and none of them is the mechanism by which the gate passes.
const path = require('path');
const fs = require('fs');
// resolve playwright-core from the CALLER's directory (the harness
// convention keeps node_modules in the job tmp dir, not the repo)
const { chromium } = require(require.resolve('playwright-core', { paths: [process.cwd(), __dirname] }));

const CAMS = [
  ['levant', 31.8, 35.3, 8],
  ['hemisphere', 31.8, 35.3, 90],
];
const GRID = [];
for (const u of [0.2, 0.35, 0.5, 0.65, 0.8]) {
  for (const v of [0.2, 0.35, 0.5, 0.65, 0.8]) GRID.push([u, v]);
}
const FIXTURE = path.join(__dirname, 'fixtures', 'golden-views.json');
const TOL = 24; // per-channel; label-glyph antialiasing stays under it

// ---------------------------------------------------------------
// Declared bounds. Each is a CEILING ON PATHOLOGY with its reason —
// never a duration tuned until the test went green.
// ---------------------------------------------------------------

// The page must come up (renderer live, timeline loaded) or it is
// broken, not slow: boot is a handful of local fetches against a
// server already running. A minute is two orders of magnitude of
// headroom over the ~1.2 s observed; past it, say so and stop.
const BOOT_CEILING_MS = 60000;

// A settle is bounded by the page's own declared durations (the 450 ms
// scene and label fade law) plus whatever the local server takes to
// publish and the GPU to accept a scene. Observed worst case on this
// machine under SwiftShader: ~9.4 s. Two minutes is not a wait we
// expect to spend — it is the line past which a page is WEDGED (a lost
// fetch, a morph whose owner never redrew) and the honest answer is a
// loud failure, not a longer sleep.
const SETTLE_CEILING_MS = 120000;

// How many samples we will take looking for a still picture. Under
// quiescence the answer is always 2 — consecutive samples come back
// byte-identical (measured across every stop/camera probed). Eight is
// four times that headroom; reaching it means the picture never
// stopped moving, which is reported, never swallowed.
const CONVERGE_ATTEMPTS = 8;

// Consecutive samples must agree EXACTLY, not within TOL. This is
// deliberate and it is the whole point: a label fading at the declared
// 450 ms law moves a probe by roughly dt/450 of its contrast — under
// ten units per frame — so "two consecutive samples within TOL=24"
// is a criterion the failure mode itself satisfies, which makes it no
// criterion at all. Zero is the only threshold a running fade cannot
// counterfeit.
const STABLE_DELTA = 0;

// ---------------------------------------------------------------
// THE SETTLE PREDICATE: is the page done moving?
//
// Every conjunct reads a field the page already maintains to drive its
// own animation. Nothing is added to page.html and nothing is timed.
// (page.html is `include_str!`-embedded in the viewer binary, so an
// accessor added there would be invisible to a running server anyway;
// its top-level `const`s are directly reachable from evaluate, so no
// accessor is needed.)
//
// Returns [] when settled, or the list of blockers — so a ceiling
// breach can say precisely WHAT never finished.
// ---------------------------------------------------------------
const QUIESCENT = `(() => {
  const why = [];
  if (typeof gpu === 'undefined' || !gpu.enabled || !gpu.canvas) return ['renderer-down'];

  // 1. No scene request in flight, and none queued behind one.
  //    gpuSync sets these synchronously, so they are already true by
  //    the time setStop() resolves — the window the old gate slept
  //    through.
  if (gpu.syncing) why.push('syncing');
  if (gpu.syncPending) why.push('syncPending');

  // 2. No scene acquired but not yet activated (the atomic swap, §45).
  if (gpu.incoming) why.push('incoming');

  // 3. The active scene's every wanted resource is resident on the GPU.
  //    (The old gate's ONLY condition.)
  const s = gpu.scene;
  if (!s) why.push('no-scene');
  else {
    const missing = s.wanted.filter(id => {
      const r = gpu.cache.get(id);
      return !(r && r.state === 'gpu');
    }).length;
    if (missing) why.push('resident:' + missing);
  }

  // 4. The demand envelope is satisfied: gpuTick, on its next frame,
  //    will NOT ask for a new scene. This is the page's own re-demand
  //    test, mirrored. It is what makes a camera move safe to sample:
  //    state.setCamera mutates state synchronously but the re-demand
  //    it provokes happens a frame later, so without this conjunct the
  //    gate can find the OLD scene perfectly settled and sample it.
  const cam = state.cameraNow && state.cameraNow();
  if (!cam || cam.zoom === null) why.push('no-camera');
  else if (!gpu.syncCam) why.push('no-sync-camera');
  else {
    const ref = gpu.syncCam;
    const walked = Math.abs(cam.lat - ref.lat) + Math.abs(cam.lon - ref.lon);
    if (walked > cam.zoom * 0.4 || Math.abs(Math.log(cam.zoom / ref.zoom)) > 0.3) why.push('envelope');
  }

  // 5. Scene lifecycle fades finished (§56): nothing being born,
  //    nothing dying, no refinement morph mid-ease (§R6).
  if ((gpu.fading || []).length) why.push('fading:' + gpu.fading.length);
  const items = gpu.drawItems || [];
  const births = items.filter(it => it.birth !== undefined && it.birth !== null).length;
  if (births) why.push('birth:' + births);
  const morphs = items.filter(it => it.morph).length;
  if (morphs) why.push('morph:' + morphs);

  // 6. NO LABEL IS MID-FADE. The conjunct the old settle() lacked, and
  //    the cause of every observed flake: label alpha walks toward its
  //    target over LABEL_FADE_MS, a separate and LATER animation than
  //    the resource upload condition (3) waited on.
  let mid = 0;
  for (const st of labelState.values()) if (st.target ? st.alpha < 1 : st.alpha > 0) mid++;
  if (mid) why.push('label:' + mid);

  // 7. The glyph atlas has no upload pending.
  if (glyphs.dirty) why.push('glyph-atlas');

  return why;
})()`;

(async () => {
  const check = process.argv.includes('--check');
  const browser = await chromium.launch({
    headless: true,
    executablePath: 'C:/Users/donov/AppData/Local/ms-playwright/chromium-1234/chrome-win64/chrome.exe',
    args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
  });
  const page = await browser.newPage({ viewport: { width: 1100, height: 1100 } });
  page.on('pageerror', e => console.log('PAGE ERROR:', e.message));
  await page.goto('http://localhost:8090/', { waitUntil: 'load' });
  // The page is ready when the renderer is live and the timeline is
  // loaded — an observed condition, not a slept 1500 ms. (Note the
  // three-argument waitForFunction: passing {timeout} as the SECOND
  // argument makes it the page function's ARGUMENT and silently leaves
  // the default timeout in force. The old gate did that.)
  await page.waitForFunction(
    "typeof gpu !== 'undefined' && gpu.enabled && !!gpu.canvas && typeof labelState !== 'undefined'" +
    " && Array.isArray(state.stops) && state.stops.length > 0",
    null, { timeout: BOOT_CEILING_MS });

  const blockers = () => page.evaluate(QUIESCENT);

  // Wait for the page to stop moving. Polls the predicate in-page on
  // the page's own animation frames; on a ceiling breach it reports
  // exactly which conjunct never cleared.
  const settle = async (where) => {
    try {
      await page.waitForFunction(`(${QUIESCENT}).length === 0`, null,
        { timeout: SETTLE_CEILING_MS, polling: 'raf' });
    } catch (e) {
      const why = await blockers().catch(() => ['<unreadable>']);
      throw new Error(`SETTLE CEILING (${SETTLE_CEILING_MS} ms) at ${where}; never finished: ${why.join(', ')}`);
    }
  };

  const sampleOnce = () => page.evaluate(async (grid) => {
    const snap = document.createElement('canvas');
    snap.width = gpu.canvas.width; snap.height = gpu.canvas.height;
    const c = snap.getContext('2d');
    await new Promise(r => requestAnimationFrame(() => { gpuDraw(); c.drawImage(gpu.canvas, 0, 0); r(); }));
    return grid.map(([u, v]) => {
      const x = Math.max(1, Math.round(snap.width * u));
      const y = Math.max(1, Math.round(snap.height * v));
      const d = c.getImageData(x - 1, y - 1, 3, 3).data;
      let r = 0, g = 0, b = 0;
      for (let p = 0; p < 9; p++) { r += d[p * 4]; g += d[p * 4 + 1]; b += d[p * 4 + 2]; }
      return [Math.round(r / 9), Math.round(g / 9), Math.round(b / 9)];
    });
  }, GRID);

  // The whole grid's worst per-channel disagreement — the comparison
  // is over all 25 probes at once, never one probe in isolation.
  const gridDelta = (a, b) => a.reduce((m, p, j) => Math.max(m,
    Math.abs(p[0] - b[j][0]), Math.abs(p[1] - b[j][1]), Math.abs(p[2] - b[j][2])), 0);

  // Sample until the picture holds still: consecutive whole-grid
  // samples that agree exactly, with quiescence still true across the
  // pair. If it never holds still, that is an honest failure —
  // never a silent pass and never a silent drift.
  let unconverged = 0;
  const stableSample = async (where) => {
    let prev = await sampleOnce();
    for (let attempt = 2; attempt <= CONVERGE_ATTEMPTS; attempt++) {
      const next = await sampleOnce();
      const delta = gridDelta(prev, next);
      // A late scene swap (healResidency) can restart an animation
      // under us; if quiescence broke, wait it out and start over.
      const why = await blockers();
      if (why.length) { await settle(where); prev = await sampleOnce(); continue; }
      if (delta <= STABLE_DELTA) return next;
      prev = next;
    }
    unconverged++;
    console.log(`NO CONVERGENCE ${where}: ${CONVERGE_ATTEMPTS} samples, picture still moving`);
    return prev;
  };

  await settle('boot');

  const stops = await page.evaluate(() => state.stops);
  const cams = {};
  for (const [name] of CAMS) cams[name] = {};
  let drifted = 0;
  const want = check ? JSON.parse(fs.readFileSync(FIXTURE, 'utf8')) : null;

  for (let i = 0; i < stops.length; i++) {
    await page.evaluate(i => setStop(i), i);
    await settle(`year ${stops[i]}`);
    for (const [name, lat, lon, zoom] of CAMS) {
      await page.evaluate(([la, lo, z]) => state.setCamera(la, lo, z), [lat, lon, zoom]);
      await settle(`year ${stops[i]} ${name}`);
      // 3x3 block means, not single pixels: a lone pixel sits on
      // antialiased edges, and the block mean is the stable signal of
      // the LOOK. (It is no longer asked to paper over fade timing —
      // settle() has already established that nothing is fading.)
      const probes = await stableSample(`year ${stops[i]} ${name}`);
      cams[name][stops[i]] = probes;
      if (check) {
        const w = (want.cams[name] || {})[stops[i]];
        if (!w) { console.log(`NEW STOP ${stops[i]} (${name}) — bless to adopt`); continue; }
        w.forEach((exp, j) => {
          const d = Math.max(Math.abs(exp[0] - probes[j][0]), Math.abs(exp[1] - probes[j][1]), Math.abs(exp[2] - probes[j][2]));
          if (d > TOL) { drifted++; console.log(`DRIFT year ${stops[i]} ${name}[${j}] want ${exp} got ${probes[j]}`); }
        });
      }
    }
    if (i % 10 === 0) console.log(`...${i + 1}/${stops.length} stops`);
  }

  if (check) {
    // R54 is NOT repaired here (Stage 4 owns it): the gate still does
    // not FAIL on a stop the fixture expects but the run never
    // visited. It now at least says so out loud.
    for (const [name] of CAMS) {
      const ran = Object.keys(cams[name]).length;
      const expected = Object.keys(want.cams[name] || {}).length;
      console.log(`stops ${name}: ran ${ran}, expected ${expected}` +
        (ran === expected ? '' : '  <-- MISMATCH (R54: reported only, not gated)'));
    }
    if (unconverged) {
      console.log(`DID NOT CONVERGE: ${unconverged} view(s) never held still`);
      await browser.close();
      process.exit(1);
    }
    console.log(drifted === 0 ? 'ALL GOLDEN VIEWS HOLD' : `REGRESSION: ${drifted} probes drifted`);
    await browser.close();
    process.exit(drifted === 0 ? 0 : 1);
  }
  if (unconverged) {
    console.error(`REFUSING TO BLESS: ${unconverged} view(s) never held still`);
    await browser.close();
    process.exit(1);
  }
  fs.writeFileSync(FIXTURE, JSON.stringify({
    note: 'The golden views: every era stop at the Levant and hemisphere cameras, ' +
          'blessed by the owner (no regression, only progression). Re-bless ONLY ' +
          'after an approved visual change: node golden.js. Gate: node golden.js --check.',
    grid: GRID, tol: TOL, cams,
  }, null, 0));
  console.log('blessed', stops.length, 'stops ->', FIXTURE);
  await browser.close();
})().catch(e => { console.error(e); process.exit(1); });
