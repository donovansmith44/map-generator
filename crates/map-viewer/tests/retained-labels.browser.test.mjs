import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
const require = createRequire(import.meta.url);
const { chromium, webkit } = require(require.resolve('playwright-core', { paths: [process.cwd()] }));
const source = fileURLToPath(new URL('../src/retained-labels.js', import.meta.url));
for (const engine of [chromium, webkit]) test(`${engine.name()} generated atlas overflow preserves prepared label pixels`, async () => {
    const browser = await engine.launch(engine === chromium ? { args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] }
        : process.env.FSHARP_WEBKIT_EXECUTABLE ? { executablePath: process.env.FSHARP_WEBKIT_EXECUTABLE } : {});
    try {
        const page = await browser.newPage(), errors = [];
        page.on('pageerror', error => errors.push(error.message));
        await page.addScriptTag({ path: source });
        const result = await page.evaluate(() => {
            const canvas = document.createElement('canvas');
            canvas.width = canvas.height = 2048;
            const ctx = canvas.getContext('2d');
            const glyphs = { canvas, ctx, map: new Map(), x: 0, y: 0, rowH: 0, dirty: false };
            const voice = { family: 'sans-serif', weight: 400, italic: false, tracking_em: 0 };
            const add = (text, size) => retainedGlyphEntry(glyphs, text, voice, size, [0,0,0,255], [255,255,255,255]);
            const first = add('Prepared first label', 100);
            const pixels = () => Array.from(ctx.getImageData(0, 0, Math.ceil(first.w), Math.ceil(first.h)).data);
            const before = pixels();
            let omitted = 0;
            for (let index = 0; index < 300; index++) if (add('Later label ' + index, 100) === null) omitted++;
            const sameEntry = add('Prepared first label', 100) === first;
            const samePixels = JSON.stringify(before) === JSON.stringify(pixels());
            const placement = [glyphs.x, glyphs.y, glyphs.rowH, glyphs.map.size];
            const wide = add('oversize'.repeat(1000), 20), tall = add('x', 2000);
            return { omitted, sameEntry, samePixels, ink: before.some(value => value !== 0), wide, tall,
                placement, finalPlacement: [glyphs.x, glyphs.y, glyphs.rowH, glyphs.map.size],
                finalPixels: JSON.stringify(before) === JSON.stringify(pixels()) };
        });
        assert.ok(result.ink, 'The prepared label has actual rasterized pixels.');
        assert.ok(result.omitted > 0, 'Generated labels exceed atlas capacity.');
        assert.equal(result.sameEntry, true, 'Overflow preserves earlier UV entries.');
        assert.equal(result.samePixels, true, 'Overflow preserves earlier rasterized names.');
        assert.equal(result.wide, null, 'A label wider than the atlas is omitted.');
        assert.equal(result.tall, null, 'A label taller than the atlas is omitted.');
        assert.deepEqual(result.finalPlacement, result.placement, 'Oversize labels preserve accepted placement.');
        assert.equal(result.finalPixels, true, 'Oversize labels preserve earlier rasterized names.');
        assert.deepEqual(errors, [], 'The browser has no page errors.');
    } finally { await browser.close(); }
});
