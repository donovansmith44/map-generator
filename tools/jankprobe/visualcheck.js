// Visual smoke test: wheel + drag on both charts, screenshot after each
// settle — the stash swap must never leave a blank/stale/misplaced map.
// Setup: npm i puppeteer-core; workbench serving on 127.0.0.1:8090.
const path = require('path');
const puppeteer = require('puppeteer-core');

(async () => {
  const browser = await puppeteer.launch({
    executablePath: 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
    headless: 'new',
    args: ['--window-size=1500,1000'],
  });
  const page = await browser.newPage();
  await page.setViewport({ width: 1500, height: 950 });
  page.on('pageerror', e => console.log('PAGE ERROR:', e.message));
  await page.goto('http://127.0.0.1:8090/', { waitUntil: 'networkidle2', timeout: 60000 });
  await new Promise(r => setTimeout(r, 2500));

  const plate = await page.$('#plate');
  const box = await plate.boundingBox();
  const cx = box.x + box.width / 2, cy = box.y + box.height / 2;
  await page.mouse.move(cx, cy);

  async function shot(name) {
    await new Promise(r => setTimeout(r, 1200)); // settle + sharpen
    await page.screenshot({ path: path.join(__dirname, name), clip: box });
    // sanity: the displayed svg exists and its transform is ~identity at rest
    const info = await page.evaluate(() => {
      const layers = document.querySelectorAll('#plate .layer');
      const svg = layers[layers.length - 1]?.querySelector('svg');
      return svg ? { tf: svg.style.transform || '(none)', paths: svg.querySelectorAll('path').length } : null;
    });
    console.log(name, JSON.stringify(info));
  }

  async function wheel(dir, n) {
    for (let i = 0; i < n; i++) { await page.mouse.wheel({ deltaY: dir * 100 }); await new Promise(r => setTimeout(r, 60)); }
  }
  async function dragMove() {
    await page.mouse.down();
    for (let i = 1; i <= 15; i++) { await page.mouse.move(cx + i * 8, cy + i * 3); await new Promise(r => setTimeout(r, 16)); }
    await page.mouse.up();
    await page.mouse.move(cx, cy);
  }

  await shot('vc-globe-start.png');
  await wheel(-1, 6); await shot('vc-globe-zoomed.png');
  await dragMove(); await shot('vc-globe-dragged.png');
  await wheel(1, 6); await shot('vc-globe-back.png');

  await page.evaluate(() => {
    const b = [...document.querySelectorAll('button, label, input')].find(
      el => (el.textContent || el.value || '').toLowerCase().includes('flat'));
    if (b) b.click();
  });
  await shot('vc-flat-start.png');
  await wheel(-1, 6); await shot('vc-flat-zoomed.png');
  await dragMove(); await shot('vc-flat-dragged.png');
  await wheel(1, 6); await shot('vc-flat-back.png');

  await browser.close();
})().catch(e => { console.error('CHECK ERROR:', e.message); process.exit(1); });
