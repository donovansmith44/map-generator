// Drive the workbench in real Edge and MEASURE jank: rAF frame-time
// distribution during a wheel-zoom gesture and a drag, on both charts.
// Setup: npm i puppeteer-core (anywhere on the node path); the
// workbench must be serving on 127.0.0.1:8090.
const puppeteer = require('puppeteer-core');

(async () => {
  const browser = await puppeteer.launch({
    executablePath: 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
    headless: 'new',
    args: ['--window-size=1500,1000', '--disable-features=CalculateNativeWinOcclusion'],
  });
  const page = await browser.newPage();
  await page.setViewport({ width: 1500, height: 950 });
  await page.goto('http://127.0.0.1:8090/', { waitUntil: 'networkidle2', timeout: 60000 });
  await new Promise(r => setTimeout(r, 2500));

  async function measure(label, gesture) {
    await page.evaluate(() => {
      window.__frames = [];
      window.__go = true;
      let last = performance.now();
      const loop = ts => {
        window.__frames.push(ts - last);
        last = ts;
        if (window.__go) requestAnimationFrame(loop);
      };
      requestAnimationFrame(loop);
    });
    await gesture();
    const motionFrames = await page.evaluate(() => window.__frames.length);
    await new Promise(r => setTimeout(r, 900)); // let it settle + sharpen
    const frames = await page.evaluate(() => { window.__go = false; return window.__frames.slice(1); });
    const motion = frames.slice(0, motionFrames - 1);
    const settle = frames.slice(motionFrames - 1);
    const stat = a => ({
      long: a.filter(f => f > 34).length,
      worst: a.length ? Math.max(...a) : 0,
    });
    const m = stat(motion), s = stat(settle);
    console.log(
      `${label}: MOTION ${motion.length}f worst=${m.worst.toFixed(0)}ms long=${m.long}` +
      ` | SETTLE worst=${s.worst.toFixed(0)}ms long=${s.long}`);
    return { long: m.long, worst: m.worst };
  }

  const plate = await page.$('#plate');
  const box = await plate.boundingBox();
  const cx = box.x + box.width / 2, cy = box.y + box.height / 2;

  // wheel zoom in: 10 notches, 60ms apart (a human-ish roll)
  await page.mouse.move(cx + 100, cy - 50);
  const wheelIn = async () => {
    for (let i = 0; i < 10; i++) {
      await page.mouse.wheel({ deltaY: -100 });
      await new Promise(r => setTimeout(r, 60));
    }
  };
  const wheelOut = async () => {
    for (let i = 0; i < 10; i++) {
      await page.mouse.wheel({ deltaY: 100 });
      await new Promise(r => setTimeout(r, 60));
    }
  };
  const dragGesture = async () => {
    await page.mouse.move(cx, cy);
    await page.mouse.down();
    for (let i = 1; i <= 25; i++) {
      await page.mouse.move(cx + i * 8, cy + i * 3);
      await new Promise(r => setTimeout(r, 16));
    }
    await page.mouse.up();
  };

  console.log('== GLOBE ==');
  const g1 = await measure('wheel-in ', wheelIn);
  const g2 = await measure('wheel-out', wheelOut);
  const g3 = await measure('drag     ', dragGesture);

  // switch to flat
  await page.evaluate(() => {
    const b = [...document.querySelectorAll('button, label, input')].find(
      el => (el.textContent || el.value || '').toLowerCase().includes('flat'));
    if (b) b.click();
  });
  await new Promise(r => setTimeout(r, 1200));
  console.log('== FLAT ==');
  const f1 = await measure('wheel-in ', wheelIn);
  const f2 = await measure('wheel-out', wheelOut);
  const f3 = await measure('drag     ', dragGesture);

  const totalLong = [g1, g2, g3, f1, f2, f3].reduce((a, m) => a + m.long, 0);
  console.log(`TOTAL long frames: ${totalLong}`);
  await browser.close();
})().catch(e => { console.error('PROBE ERROR:', e.message); process.exit(1); });
