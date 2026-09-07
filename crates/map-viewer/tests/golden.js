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
  await page.waitForTimeout(1500);
  await page.evaluate(() => { if (!gpu.enabled) document.getElementById('gpu').click(); });
  await page.waitForFunction("typeof gpu !== 'undefined' && gpu.enabled && !!gpu.canvas", { timeout: 20000 });
  const settled = () => page.waitForFunction(() => {
    const s = gpu.incoming || gpu.scene;
    return s && s === gpu.scene &&
      s.wanted.every(id => { const r = gpu.cache.get(id); return r && r.state === 'gpu'; });
  }, { timeout: 120000 });
  await settled();

  const stops = await page.evaluate(() => state.stops);
  const cams = {};
  for (const [name] of CAMS) cams[name] = {};
  let drifted = 0;
  const want = check ? JSON.parse(fs.readFileSync(FIXTURE, 'utf8')) : null;

  for (let i = 0; i < stops.length; i++) {
    await page.evaluate(i => setStop(i), i);
    await settled();
    for (const [name, lat, lon, zoom] of CAMS) {
      await page.evaluate(([la, lo, z]) => state.setCamera(la, lo, z), [lat, lon, zoom]);
      await settled();
      await page.waitForTimeout(700); // label fades finish (450ms law)
      // 3x3 block means, not single pixels: a lone pixel sits on
      // antialiased edges and label glyphs whose fade timing jitters
      // run to run — the mean is the stable signal of the LOOK.
      const sample = () => page.evaluate(async (grid) => {
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
      let probes = await sample();
      cams[name][stops[i]] = probes;
      if (check) {
        const w = (want.cams[name] || {})[stops[i]];
        if (!w) { console.log(`NEW STOP ${stops[i]} (${name}) — bless to adopt`); continue; }
        const off = j => {
          const d = Math.max(Math.abs(w[j][0] - probes[j][0]), Math.abs(w[j][1] - probes[j][1]), Math.abs(w[j][2] - probes[j][2]));
          return d > TOL;
        };
        // one settle-and-retry before declaring drift: a label mid-
        // fade is a transient, not a regression
        if (w.some((_, j) => off(j))) {
          await page.waitForTimeout(900);
          probes = await sample();
          cams[name][stops[i]] = probes;
        }
        w.forEach((exp, j) => {
          if (off(j)) { drifted++; console.log(`DRIFT year ${stops[i]} ${name}[${j}] want ${exp} got ${probes[j]}`); }
        });
      }
    }
    if (i % 10 === 0) console.log(`...${i + 1}/${stops.length} stops`);
  }

  if (check) {
    console.log(drifted === 0 ? 'ALL GOLDEN VIEWS HOLD' : `REGRESSION: ${drifted} probes drifted`);
    process.exit(drifted === 0 ? 0 : 1);
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
