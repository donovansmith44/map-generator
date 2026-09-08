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
//
// THE GATE'S OWN LAWS (R99). This gate is the instrument standing
// between a refactor and the maps the owner loves, so it answers to
// the standard it enforces: it must be able to FAIL, and it must
// always say WHICH WAY. Its required behaviour is written as laws in
// contracts/golden-gate/golden-gate.feature and executed by the
// Haskell contract runner, which drives this file as a subprocess.
// The three capabilities below exist for that, and each is a general
// input to the gate rather than a switch per law:
//
//   --condition FILE      run the page under a DECLARED CONDITION: the
//   --condition-arg JSON  file is installed in the page's own world
//                         before any of the page's scripts (a browser
//                         extension's content script, in effect), and
//                         the JSON is handed to it as `CONDITION`.
//                         ONE mechanism, not one flag per situation: a
//                         renderer that dies, a page that never
//                         settles, a scene that never arrives and a
//                         repainted probe are four condition FILES,
//                         and this gate knows about none of them. See
//                         tests/conditions/ for the ones the laws use.
//                         The gate publishes its own constants as
//                         `GOLDEN` (cameras, probe grid, tolerance) so
//                         a condition can find a probe without
//                         restating them.
//
//   --stops Y1,Y2,...     judge only these stops, BY YEAR. A full run
//                         is ~25 minutes; a law about detection does
//                         not need 89 eras to be true, and a property
//                         quantified over 25 probes and 2 cameras
//                         cannot pay 25 minutes an iteration. This is
//                         a real capability, not a test hack — but a
//                         short run must never be mistakable for a
//                         complete one, so a subset is stated in the
//                         banner, in the verdict, and in the stop
//                         accounting below, which fails a run that did
//                         not judge every stop it was asked for.
//
//   --baseline FILE       the blessed views to judge against (or, in
//                         bless mode, to write). The baseline is an
//                         INPUT to the gate, not a constant inside it;
//                         that is what lets the law "a view that will
//                         not hold still is never blessed" be checked
//                         by actually running a bless, without staking
//                         the owner's own fixture on the answer.
//
// Every run ends with one machine-readable line, `GATE VERDICT {...}`,
// naming the verdict, the stops judged, the stops missed, and the
// condition (if any) it ran under. "Always says which way" is that
// line: a gate that only exits non-zero says a run failed, not what
// failed, and the laws are about the difference.
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

// How many individual drifted probes the verdict line enumerates. Not
// a threshold on anything: the drifted COUNT is always exact and always
// whole, and this bounds only how many are spelled out beside it. Forty
// is more than any law here reads and far less than a blank map's
// forty-six per stop.
const DRIFT_REPORT_CAP = 40;

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
  // A LOST CONTEXT IS A DEAD RENDERER, and the page's own flags do not
  // say so: gpu.enabled is only ever assigned by gpuSetEnabled and
  // gpu.canvas survives the loss, so a context that dies mid-run leaves
  // both conjuncts above true and the page merrily "drawing" nothing.
  // Observed, not timed, and read-only: WebGL keeps the fact itself, so
  // page.html is untouched. Before this, a dead renderer surfaced only
  // as a two-minute settle-ceiling breach whose blocker list said
  // "label:N" -- the shape of a page that is merely slow.
  if (!gpu.gl || gpu.gl.isContextLost()) return ['renderer-down'];

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

// THE SETTLE STATE, in one in-page expression: '' while the page is
// still moving, 'settled' when it has stopped, 'renderer-down' when
// there is no renderer left to wait for.
//
// The third answer is the point. A dead renderer used to surface only
// as a SETTLE CEILING breach — after burning the full two minutes, in
// exactly the shape of a page that is merely slow. Death is not
// slowness, and the law says the gate must SAY the renderer is down;
// so renderer-down is terminal here and reported at once, while the
// ceiling stays what it was: a bound on a page that is still alive and
// still not finished.
const SETTLE_STATE = `(() => {
  const why = ${QUIESCENT};
  return why.length === 0 ? 'settled' : (why[0] === 'renderer-down' ? 'renderer-down' : '');
})()`;

// THE ARRIVAL PREDICATE: is the picture on screen the picture this
// camera asked for?
//
// gpuSyncInner sets `gpu.sceneKey` to the key it is ABOUT to fetch and
// then returns silently on `!res.ok` — so a scene that fails to arrive
// leaves the key advanced and the previous scene still drawing, fully
// resident, perfectly quiescent. Nothing in the settle predicate can
// see that: every conjunct is satisfied by a page contentedly showing
// the wrong map. This reads the page's own two names for the scene and
// reports the mismatch; a stale picture judged against this stop's
// blessed colours is drift that is not drift, and an all-black one
// blessed as a view is how a black baseline gets written.
const ARRIVED = `(() => {
  if (typeof gpu === 'undefined' || !gpu.scene) return 'no-scene';
  if (gpu.scene.key !== gpu.sceneKey) return 'stale-scene';
  return '';
})()`;

// ---------------------------------------------------------------
// The command line.
// ---------------------------------------------------------------
const ARGS = process.argv.slice(2);
const has = n => ARGS.includes(n);
const valueOf = (n, dflt) => {
  const i = ARGS.indexOf(n);
  if (i < 0) return dflt;
  if (i + 1 >= ARGS.length) { console.error(`${n} needs a value`); process.exit(2); }
  return ARGS[i + 1];
};

const check = has('--check');
const BASELINE = path.resolve(valueOf('--baseline', FIXTURE));
const CONDITION = valueOf('--condition', null);
const CONDITION_ARG_TEXT = valueOf('--condition-arg', null);
// Parsed HERE, at the start, and never again: a malformed argument is a
// mistake in the invocation, and the honest moment to say so is before
// a browser is launched — not in the verdict line at the end of a run
// that has already spent a minute looking at the wrong thing.
let CONDITION_ARG = null;
if (CONDITION_ARG_TEXT !== null) {
  try {
    CONDITION_ARG = JSON.parse(CONDITION_ARG_TEXT);
  } catch (e) {
    console.error(`--condition-arg is not JSON: ${CONDITION_ARG_TEXT}`);
    process.exit(2);
  }
}
const STOPS_ARG = valueOf('--stops', null);
const REQUESTED = STOPS_ARG === null ? null : STOPS_ARG.split(',').map(s => {
  const y = Number(s.trim());
  if (!Number.isFinite(y)) { console.error(`--stops: '${s}' is not a year`); process.exit(2); }
  return y;
});
if (REQUESTED !== null && REQUESTED.length === 0) {
  console.error('--stops: an empty subset judges nothing'); process.exit(2);
}

// A DRIVEN OR SUBSETTED BLESS NEVER TOUCHES THE DEFAULT BASELINE. The
// owner's golden-views.json is the one artefact in this repository
// that a wrong answer here would destroy silently, and both new inputs
// are ways to make the gate see something other than the real map. The
// refusal is structural rather than trusted to the caller: a bless
// under a condition, or of a subset of stops, must name its own file.
if (!check && BASELINE === FIXTURE && (CONDITION || REQUESTED)) {
  console.error('REFUSING TO BLESS the default baseline under --condition/--stops: '
                + 'a driven or partial run may not overwrite ' + FIXTURE
                + ' (pass --baseline to write elsewhere)');
  process.exit(2);
}

// ---------------------------------------------------------------
// The verdict. ONE line, machine-readable, at the end of every run —
// including a run that died. A gate that only exits non-zero says
// THAT a run failed; the laws are about WHICH WAY it failed, and a
// caller that has to grep prose for the difference is reading tea
// leaves.
// ---------------------------------------------------------------
const VERDICTS = {
  HOLD: 0,            // every judged probe matches the baseline
  BLESSED: 0,         // bless mode wrote the baseline
  DRIFT: 1,           // a probe moved further than the tolerance
  'NOT-STILL': 1,     // the view would not hold still
  'RENDERER-DOWN': 1, // there is no renderer to judge
  'NOT-ARRIVED': 1,   // the picture is not the picture asked for
  'MISSING-STOPS': 1, // the baseline knows a stop this run never judged
  ERROR: 1,           // anything else, named in `detail`
};

const say = (verdict, extra) => {
  const body = Object.assign({
    verdict,
    mode: check ? 'check' : 'bless',
    baseline: BASELINE,
    condition: CONDITION,
    conditionArg: CONDITION_ARG,
    subset: REQUESTED,
  }, extra);
  console.log('GATE VERDICT ' + JSON.stringify(body));
  return VERDICTS[verdict];
};

(async () => {
  // The banner: what this run actually is, before it is anything else.
  // A subsetted or driven run announces itself here and in the verdict
  // both, because the one thing a short run must never do is read like
  // a complete one.
  console.log(`gate: ${check ? 'check' : 'bless'} against ${BASELINE}`);
  if (CONDITION) console.log(`CONDITION: ${CONDITION}${CONDITION_ARG_TEXT ? ' ' + CONDITION_ARG_TEXT : ''}`);
  if (REQUESTED) console.log(`SUBSET: ${REQUESTED.length} declared stop(s): ${REQUESTED.join(', ')}`);

  const browser = await chromium.launch({
    headless: true,
    executablePath: 'C:/Users/donov/AppData/Local/ms-playwright/chromium-1234/chrome-win64/chrome.exe',
    args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
  });
  const page = await browser.newPage({ viewport: { width: 1100, height: 1100 } });
  page.on('pageerror', e => console.log('PAGE ERROR:', e.message));

  // The gate's own constants, published into the page BEFORE anything
  // else runs there. A condition that has to locate probe 7 of the
  // levant camera would otherwise restate the grid and the camera
  // table, and a restated constant is a constant that drifts.
  await page.addInitScript(`globalThis.GOLDEN = ${JSON.stringify({ cams: CAMS, grid: GRID, tol: TOL })};`);
  if (CONDITION_ARG !== null) await page.addInitScript(`globalThis.CONDITION = ${JSON.stringify(CONDITION_ARG)};`);
  if (CONDITION) await page.addInitScript({ path: CONDITION });

  // Everything that can go wrong from here on ends in ONE verdict.
  let code = 1;
  try {
    code = await run(page);
  } catch (e) {
    const msg = String((e && e.message) || e);
    // A refusal raised by `halt` already KNOWS which way it failed and
    // says so on the error itself; nothing here re-derives it from
    // prose. Only the two ceilings, which are thrown by the waiting
    // code rather than by a judgement, are classified from their
    // message -- and anything else is an ERROR that quotes itself
    // rather than being filed under a verdict it did not earn.
    const verdict = e && e.verdict ? e.verdict
                  : /SETTLE CEILING/.test(msg) ? 'NOT-STILL'
                  : /BOOT CEILING/.test(msg) ? 'NOT-ARRIVED'
                  : 'ERROR';
    code = say(verdict, { detail: msg });
  } finally {
    await browser.close();
  }
  process.exit(code);
})();

async function run(page) {
  await page.goto('http://localhost:8090/', { waitUntil: 'load' });
  // The page is ready when the renderer is live and the timeline is
  // loaded — an observed condition, not a slept 1500 ms. (Note the
  // three-argument waitForFunction: passing {timeout} as the SECOND
  // argument makes it the page function's ARGUMENT and silently leaves
  // the default timeout in force. The old gate did that.)
  try {
    await page.waitForFunction(
      "typeof gpu !== 'undefined' && gpu.enabled && !!gpu.canvas && typeof labelState !== 'undefined'" +
      " && Array.isArray(state.stops) && state.stops.length > 0",
      null, { timeout: BOOT_CEILING_MS });
  } catch (e) {
    throw new Error(`BOOT CEILING (${BOOT_CEILING_MS} ms): the page never came up`);
  }

  const blockers = () => page.evaluate(QUIESCENT);

  // Wait for the page to stop moving. Polls the predicate in-page on
  // the page's own animation frames; on a ceiling breach it reports
  // exactly which conjunct never cleared. A dead renderer short-
  // circuits: see SETTLE_STATE.
  const settle = async (where) => {
    let state;
    try {
      state = await page.waitForFunction(`(${SETTLE_STATE}) || null`, null,
        { timeout: SETTLE_CEILING_MS, polling: 'raf' }).then(h => h.jsonValue());
    } catch (e) {
      const why = await blockers().catch(() => ['<unreadable>']);
      throw new Error(`SETTLE CEILING (${SETTLE_CEILING_MS} ms) at ${where}; never finished: ${why.join(', ')}`);
    }
    return state;
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

  // The three ways a view can refuse to be judged, checked in one
  // place so every sample site asks the same question: the renderer is
  // gone, the page never stopped moving, or the picture on screen is
  // not the one this camera asked for.
  const halt = (verdict, where, detail) => {
    const e = new Error(`${verdict} at ${where}: ${detail}`);
    e.verdict = verdict;
    e.where = where;
    throw e;
  };

  const settleOrHalt = async (where) => {
    if (await settle(where) === 'renderer-down') {
      halt('RENDERER-DOWN', where, 'the WebGL context is gone');
    }
  };

  // Sample until the picture holds still: consecutive whole-grid
  // samples that agree exactly, with quiescence still true across the
  // pair. If it never holds still, that is an honest failure —
  // never a silent pass and never a silent drift.
  const stableSample = async (where) => {
    let prev = await sampleOnce();
    for (let attempt = 2; attempt <= CONVERGE_ATTEMPTS; attempt++) {
      const next = await sampleOnce();
      const delta = gridDelta(prev, next);
      // A late scene swap (healResidency) can restart an animation
      // under us; if quiescence broke, wait it out and start over.
      const why = await blockers();
      if (why.length) { await settleOrHalt(where); prev = await sampleOnce(); continue; }
      if (delta <= STABLE_DELTA) return next;
      prev = next;
    }
    halt('NOT-STILL', where, `${CONVERGE_ATTEMPTS} samples, picture still moving`);
  };

  await settleOrHalt('boot');

  // A bless has nothing to read: it is what CREATES a baseline, and a
  // first bless has none. A check has nothing to do without one, and
  // says so rather than reporting that zero views held.
  if (check && !fs.existsSync(BASELINE)) {
    return say('ERROR', { detail: `there is no baseline at ${BASELINE} to judge against` });
  }
  const want = fs.existsSync(BASELINE)
    ? JSON.parse(fs.readFileSync(BASELINE, 'utf8'))
    : { cams: {} };
  const pageStops = await page.evaluate(() => state.stops);
  // The stops the BASELINE knows — the same for both cameras in every
  // blessed fixture, and read per camera below so a fixture where they
  // ever differ is still accounted for honestly.
  const baselineStops = [...new Set(CAMS.flatMap(([name]) =>
    Object.keys(want.cams[name] || {}).map(Number)))];

  // WHICH STOPS THIS RUN OWES AN ANSWER FOR. With no subset declared
  // that is everything either side knows about; with one, exactly the
  // stops asked for. A requested stop that exists in neither the map
  // nor the baseline is a typo in the request, not a finding about the
  // gate, so it stops the run before it starts.
  const targets = REQUESTED === null
    ? [...new Set([...pageStops, ...baselineStops])]
    : REQUESTED;
  const nowhere = targets.filter(y => !pageStops.includes(y) && !baselineStops.includes(y));
  if (nowhere.length) {
    return say('ERROR', { detail: `requested stop(s) ${nowhere.join(', ')} are in neither the map nor the baseline` });
  }

  const cams = {};
  for (const [name] of CAMS) cams[name] = {};
  const drift = [];
  const judged = [];
  const newStops = [];

  for (let i = 0; i < pageStops.length; i++) {
    if (!targets.includes(pageStops[i])) continue;
    await page.evaluate(i => setStop(i), i);
    await settleOrHalt(`year ${pageStops[i]}`);
    for (const [name, lat, lon, zoom] of CAMS) {
      const where = `year ${pageStops[i]} ${name}`;
      await page.evaluate(([la, lo, z]) => state.setCamera(la, lo, z), [lat, lon, zoom]);
      await settleOrHalt(where);
      // THE PICTURE MUST BE THIS STOP'S PICTURE. Checked after the
      // settle, never as a settle conjunct: settled is exactly when
      // "the scene on screen is the scene asked for" stops being a
      // transient and starts being a fact.
      const arrival = await page.evaluate(ARRIVED);
      if (arrival) halt('NOT-ARRIVED', where, arrival);
      // 3x3 block means, not single pixels: a lone pixel sits on
      // antialiased edges, and the block mean is the stable signal of
      // the LOOK. (It is no longer asked to paper over fade timing —
      // settle() has already established that nothing is fading.)
      const probes = await stableSample(where);
      cams[name][pageStops[i]] = probes;
      if (check) {
        const w = (want.cams[name] || {})[pageStops[i]];
        if (!w) { newStops.push(`${pageStops[i]} (${name})`); continue; }
        w.forEach((exp, j) => {
          const d = Math.max(Math.abs(exp[0] - probes[j][0]), Math.abs(exp[1] - probes[j][1]), Math.abs(exp[2] - probes[j][2]));
          if (d > TOL) {
            drift.push({ stop: pageStops[i], camera: name, probe: j, want: exp, got: probes[j] });
            console.log(`DRIFT year ${pageStops[i]} ${name}[${j}] want ${exp} got ${probes[j]}`);
          }
        });
      }
    }
    judged.push(pageStops[i]);
    if (judged.length % 10 === 1) console.log(`...${judged.length}/${targets.length} stops`);
  }

  // R101: THE GATE'S SHARE OF R54 CLOSES HERE. A stop the baseline
  // holds and this run never judged is not a footnote: it is the
  // difference between "89 views hold" and "the run looked at three of
  // them and said nothing about the rest". The comparison is against
  // the stops this run OWED an answer for — everything the baseline
  // knows on a full run, and exactly the declared subset on a short
  // one — so a subset is judged completely or not at all, and can
  // never buy a green by simply looking at less.
  const missing = targets.filter(y => baselineStops.includes(y) && !judged.includes(y));
  for (const [name] of CAMS) {
    console.log(`stops ${name}: judged ${Object.keys(cams[name]).length}`
      + `, owed ${targets.filter(y => (want.cams[name] || {})[y] !== undefined).length}`);
  }
  for (const s of newStops) console.log(`NEW STOP ${s} — bless to adopt`);

  const accounting = {
    judged, missing, newStops,
    full: REQUESTED === null && missing.length === 0,
    baselineStops: baselineStops.length,
  };

  if (check) {
    if (missing.length) {
      return say('MISSING-STOPS', Object.assign({
        detail: `the baseline holds ${missing.length} stop(s) this run never judged: ${missing.join(', ')}`,
      }, accounting));
    }
    if (drift.length) {
      // The COUNT is whole; the LIST is bounded. A blank map drifts
      // every probe of every stop it judged, and the verdict is a
      // line. Truncation can only ever make a caller's law red — a
      // caller counting drifts sees fewer than `driftCount` and must
      // say so — never green, which is the direction that matters.
      return say('DRIFT', Object.assign({
        driftCount: drift.length,
        drift: drift.slice(0, DRIFT_REPORT_CAP),
        detail: `${drift.length} probe(s) drifted`,
      }, accounting));
    }
    console.log('ALL GOLDEN VIEWS HOLD');
    return say('HOLD', accounting);
  }

  // Bless. Reached only when every view held still, arrived, and was
  // judged — the refusals above are throws, so there is no path from a
  // pathological run to a written baseline.
  if (missing.length) {
    return say('MISSING-STOPS', Object.assign({
      detail: `refusing to bless: ${missing.length} requested stop(s) were never judged`,
    }, accounting));
  }
  fs.writeFileSync(BASELINE, JSON.stringify({
    note: 'The golden views: every era stop at the Levant and hemisphere cameras, ' +
          'blessed by the owner (no regression, only progression). Re-bless ONLY ' +
          'after an approved visual change: node golden.js. Gate: node golden.js --check.',
    grid: GRID, tol: TOL, cams,
  }, null, 0));
  console.log('blessed', judged.length, 'stops ->', BASELINE);
  return say('BLESSED', Object.assign({ blessed: true }, accounting));
}
