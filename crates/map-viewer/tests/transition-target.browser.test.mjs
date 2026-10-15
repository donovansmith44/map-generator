import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
const require = createRequire(import.meta.url);
const { chromium, webkit } = require(require.resolve('playwright-core', { paths: [process.cwd()] }));
const viewer = fileURLToPath(new URL('..', import.meta.url));
const served = JSON.parse(readFileSync(process.env.MG_TRANSITION_FIXTURE, 'utf8'));
const polygon = served.features.find(feature => feature.geometry.type === 'MultiPolygon' && feature.geometry.coordinates.some(part => part.length > 1));
const point = served.features.find(feature => feature.geometry.type === 'Point' && feature.properties.label?.anchor.type === 'Point');
assert.ok(polygon, 'The served fixture has a polygon with holes.');
assert.ok(point, 'The served fixture has an anchored label.');
const collection = { ...served, features: [polygon, point] };
for (const engine of [chromium, webkit]) {
    test(`${engine.name()} full transition target releases twenty forward reverse cycles and settles exact data`, async () => {
        const browser = await engine.launch(engine === chromium ? { args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] }
            : process.env.FSHARP_WEBKIT_EXECUTABLE ? { executablePath: process.env.FSHARP_WEBKIT_EXECUTABLE } : {});
        try {
            const page = await browser.newPage(), errors = [];
            page.on('pageerror', error => errors.push(error.message));
            await page.context().route('http://canonical.test/**', async route => {
                const path = new URL(route.request().url()).pathname;
                if (path === '/') await route.fulfill({ contentType: 'text/html', body: '<div id="map" style="width:320px;height:320px"></div>' });
                else await route.fulfill({ contentType: 'text/javascript', path: viewer + '/node_modules/maplibre-gl/dist/' + path.slice(1) });
            });
            await page.goto('http://canonical.test/');
            await page.addScriptTag({ type: 'module', content: "import * as maplibregl from '/maplibre-gl.mjs';globalThis.maplibregl=maplibregl;" });
            await page.waitForFunction(() => globalThis.maplibregl?.Map);
            for (const path of ['src/retained-gpu.js', 'src/retained-labels.js', 'src/transition-target.js']) await page.addScriptTag({ path: viewer + '/' + path });
            const result = await page.evaluate(async collection => {
                const map = new maplibregl.Map({ container: 'map', style: { version: 8, sources: {}, layers: [] }, center: [35, 31], zoom: 3, fadeDuration: 0 });
                await new Promise(resolve => map.once('load', resolve));
                let pending = null, target = null, input = null, gl = null, tracking = false;
                const groups = new Map();
                let allocations = 0, releases = 0, coldCount = 0;
                const original = JSON.stringify(collection);
                const paint = () => [{ fill: [190, 208, 140, 210] }, { color: [60, 70, 40, 255], width: 1, pattern: 'Solid' }, { color: [30, 30, 30, 255], size: 5 },
                    [{ color: [25, 25, 25, 255], halo: [255, 255, 255, 255], size: 14, halo_width_em: 0.24 },
                        { family: 'sans-serif', weight: 400, italic: false, uppercase: false, tracking_em: 0, advance_em: 0.5 }]];
                const cold = destination => {
                    if (destination !== collection || JSON.stringify(destination) !== original) throw new Error('Settlement must pass the exact unchanged ordinary source collection.');
                    coldCount++;
                };
                const names = ['CURRENT_PROGRAM', 'VERTEX_ARRAY_BINDING', 'ARRAY_BUFFER_BINDING', 'DRAW_FRAMEBUFFER_BINDING', 'READ_FRAMEBUFFER_BINDING', 'RENDERBUFFER_BINDING', 'VIEWPORT', 'COLOR_CLEAR_VALUE', 'ACTIVE_TEXTURE', 'TEXTURE_BINDING_2D', 'COLOR_WRITEMASK', 'STENCIL_WRITEMASK', 'STENCIL_FUNC', 'STENCIL_REF', 'STENCIL_VALUE_MASK', 'STENCIL_FAIL', 'STENCIL_PASS_DEPTH_FAIL', 'STENCIL_PASS_DEPTH_PASS', 'LINE_WIDTH', 'UNPACK_PREMULTIPLY_ALPHA_WEBGL'];
                const snapshot = () => names.map(name => [name, gl.getParameter(gl[name])]);
                const verify = before => {
                    for (const [name, value] of before) {
                        const actual = gl.getParameter(gl[name]);
                        const equal = Array.isArray(value) || ArrayBuffer.isView(value) ? Array.from(value).every((element, index) => element === actual[index]) : actual === value;
                        if (!equal) throw new Error('Shared GL state changed: ' + name);
                    }
                };
                map.addLayer({ id: 'canonical-transition', type: 'custom', renderingMode: '2d', onAdd(_map, context) {
                    gl = context;
                    for (const kind of ['Buffer', 'VertexArray', 'Shader', 'Program', 'Texture', 'Framebuffer', 'Renderbuffer']) {
                        const live = new Set(); groups.set(kind, live);
                        const create = gl['create' + kind].bind(gl), drop = gl['delete' + kind].bind(gl);
                        gl['create' + kind] = (...args) => { const value = create(...args); if (tracking && value) { live.add(value); allocations++; } return value; };
                        gl['delete' + kind] = value => { if (live.delete(value)) releases++; drop(value); };
                    }
                }, render(_context, args) {
                    input = args;
                    if (!pending) return;
                    const operation = pending; pending = null; tracking = true;
                    try { operation.run(); operation.resolve(); } catch (error) { operation.reject(error); } finally { tracking = false; }
                } });
                const run = action => new Promise((resolve, reject) => { pending = { run: action, resolve, reject }; map.triggerRepaint(); });
                await run(() => { target = transitionTarget(gl, () => input, paint, cold); });
                const frame = { before: collection, after: collection, change: { script: { steps: [] } } };
                for (let cycle = 0; cycle < 20; cycle++) for (const direction of ['forward', 'reverse']) await run(() => {
                    const before = snapshot();
                    drawTransition(target, frame, [], direction === 'forward' ? 0.25 : 0.75);
                    verify(before);
                    const program = target.programs.get(input.shaderData.variantName);
                    const fraction = direction === 'forward' ? 0.25 : 0.75;
                    if (gl.getUniform(program.value, program.loc.uMix) !== fraction) throw new Error('Target must consume the provided fraction without applying easing.');
                    if (groups.get('Texture').size < 3) throw new Error('Target must own two scenes and the source glyph atlas.');
                    if (gl.getError() !== gl.NO_ERROR) throw new Error('Transition draw generated a GL error.');
                    drawTransition(target, frame, [], 1); verify(before);
                    for (const kind of ['Buffer', 'VertexArray', 'Texture', 'Framebuffer', 'Renderbuffer']) if (groups.get(kind).size !== 0) throw new Error('Settlement retained ' + kind);
                });
                await run(() => {
                    disposeTransition(target); disposeTransition(target);
                    const create = gl.createProgram; gl.createProgram = () => null;
                    try {
                        let refused = false;
                        try { transitionTarget(gl, () => input, paint, cold); } catch (error) { refused = error.message.includes('program allocation'); }
                        if (!refused) throw new Error('Factory must refuse failed allocation.');
                    } finally { gl.createProgram = create; }
                });
                for (const [kind, live] of groups) if (live.size !== 0) throw new Error('Disposal retained ' + kind);
                const result = { allocations, releases, coldCount, crossfades: target.crossfades, error: gl.getError() };
                map.remove(); return result;
            }, collection);
            assert.equal(result.allocations, result.releases, 'All target allocations are released, including failed factory shaders.');
            assert.equal(result.coldCount, 40, 'Every endpoint uses the exact ordinary destination callback.');
            assert.equal(result.crossfades, 40, 'Missing correspondence is counted as crossfades without a historical morph claim.');
            assert.equal(result.error, 0, 'Disposal leaves no GL error.');
            assert.deepEqual(errors, [], 'The browser completes without page errors.');
        } finally { await browser.close(); }
    });
}
