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
      const probes = await page.evaluate(async (grid) => {
        const snap = document.createElement('canvas');
        snap.width = gpu.canvas.width; snap.height = gpu.canvas.height;
        const c = snap.getContext('2d');
        await new Promise(r => requestAnimationFrame(() => { gpuDraw(); c.drawImage(gpu.canvas, 0, 0); r(); }));
        return grid.map(([u, v]) => {
          const d = c.getImageData(Math.round(snap.width * u), Math.round(snap.height * v), 1, 1).data;
          return [d[0], d[1], d[2]];
        });
      }, GRID);
      cams[name][stops[i]] = probes;
      if (check) {
        const w = (want.cams[name] || {})[stops[i]];
        if (!w) { console.log(`NEW STOP ${stops[i]} (${name}) — bless to adopt`); continue; }
        w.forEach((exp, j) => {
          const g = probes[j];
          const d = Math.max(Math.abs(exp[0] - g[0]), Math.abs(exp[1] - g[1]), Math.abs(exp[2] - g[2]));
          if (d > TOL) { drifted++; console.log(`DRIFT year ${stops[i]} ${name}[${j}] want ${exp} got ${g}`); }
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
