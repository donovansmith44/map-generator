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
const collection = { ...served, features: [polygon, point], waters: [] };
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
            const result = await page.evaluate(async ({ collection, fullCollection }) => {
                const map = new maplibregl.Map({ container: 'map', style: { version: 8, sources: {}, layers: [] }, center: [35, 31], zoom: 3, fadeDuration: 0 });
                await new Promise(resolve => map.once('load', resolve));
                let pending = null, target = null, input = null, gl = null, tracking = false;
                const groups = new Map();
                let allocations = 0, releases = 0, coldCount = 0, glyphUploads = 0;
                let expectedDestination = collection;
                let original = JSON.stringify(collection);
                const paint = () => [{ fill: [190, 208, 140, 210] }, { color: [60, 70, 40, 255], width: 1, pattern: 'Solid' }, { color: [30, 30, 30, 255], size: 5 },
                    [{ color: [25, 25, 25, 255], halo: [255, 255, 255, 255], size: 14, halo_width_em: 0.24 },
                        { family: 'sans-serif', weight: 400, italic: false, uppercase: false, tracking_em: 0, advance_em: 0.5 }]];
                const cold = destination => {
                    if (destination !== expectedDestination || JSON.stringify(destination) !== original) throw new Error('Settlement must pass the exact unchanged ordinary source collection.');
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
                    const upload = gl.texImage2D.bind(gl);
                    gl.texImage2D = (...args) => { if (tracking && args.length === 6) glyphUploads++; return upload(...args); };
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
                await run(() => { target = transitionTarget(gl, () => input, paint, () => [{ fill: [30, 110, 210, 255] }, null], cold); });
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
                    expectedDestination = fullCollection;
                    original = JSON.stringify(fullCollection);
                    const previousGlyphUploads = glyphUploads;
                    drawTransition(target, { before: fullCollection, after: fullCollection, change: { script: { steps: [] } } }, [], 0.25);
                    if (coldCount !== 40) throw new Error('The complete served scene animates through its water look.');
                    if (glyphUploads - previousGlyphUploads > 1) throw new Error('One frame batches its glyph atlas upload once; actual uploads ' + (glyphUploads - previousGlyphUploads));
                    drawTransition(target, { before: fullCollection, after: fullCollection, change: { script: { steps: [] } } }, [], 1);
                    for (const kind of ['Buffer', 'VertexArray', 'Texture', 'Framebuffer', 'Renderbuffer']) if (groups.get(kind).size !== 0) throw new Error('Full water scene settlement retained an overlay.');
                    disposeTransition(target); disposeTransition(target);
                    const create = gl.createProgram; gl.createProgram = () => null;
                    try {
                        let refused = false;
                        try { transitionTarget(gl, () => input, paint, () => [{ fill: [30, 110, 210, 255] }, null], cold); } catch (error) { refused = error.message.includes('program allocation'); }
                        if (!refused) throw new Error('Factory must refuse failed allocation.');
                    } finally { gl.createProgram = create; }
                });
                for (const [kind, live] of groups) if (live.size !== 0) throw new Error('Disposal retained ' + kind);
                const result = { allocations, releases, coldCount, crossfades: target.crossfades, error: gl.getError() };
                map.remove(); return result;
            }, { collection, fullCollection: served });
            assert.equal(result.allocations, result.releases, 'All target allocations are released, including failed factory shaders.');
            assert.equal(result.coldCount, 41, 'Every endpoint uses the exact ordinary destination callback.');
            assert.equal(result.crossfades, 41, 'Missing correspondence is counted as crossfades without a historical morph claim.');
            assert.equal(result.error, 0, 'Disposal leaves no GL error.');
            assert.deepEqual(errors, [], 'The browser completes without page errors.');
        } finally { await browser.close(); }
    });
}

for (const engine of [chromium, webkit]) {
    test(`${engine.name()} projection follows the actual camera and preserves sentinel caller state`, async () => {
        const browser = await engine.launch(engine === chromium ? { args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] }
            : process.env.FSHARP_WEBKIT_EXECUTABLE ? { executablePath: process.env.FSHARP_WEBKIT_EXECUTABLE } : {});
        try {
            const page = await browser.newPage();
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
                const feature = collection.features.find(feature => feature.geometry.type === 'Point');
                const scene = { ...collection, features: [feature] };
                const frame = { before: scene, after: scene, change: { script: { steps: [] } } };
                const map = new maplibregl.Map({ container: 'map', style: { version: 8, sources: {}, layers: [] }, center: feature.geometry.coordinates, zoom: 3, fadeDuration: 0 });
                await new Promise(resolve => map.once('load', resolve));
                const marker = { color: [230, 30, 70, 255], size: 10 };
                map.addSource('ordinary', { type: 'geojson', data: scene });
                map.addLayer({ id: 'ordinary', source: 'ordinary', type: 'circle', layout: { visibility: 'none' }, paint: { 'circle-radius': marker.size / 2, 'circle-color': `rgb(${marker.color.slice(0, 3).join(',')})` } });
                let pending = null, input = null, gl = null, target = null;
                map.addLayer({ id: 'projection-proof', type: 'custom', renderingMode: '2d', onAdd(_map, context) { gl = context; },
                    render(_context, args) {
                        input = args;
                        if (!pending) return;
                        const operation = pending; pending = null;
                        try { operation.resolve(operation.run()); } catch (error) { operation.reject(error); }
                    } });
                const run = action => new Promise((resolve, reject) => { pending = { run: action, resolve, reject }; map.triggerRepaint(); });
                await run(() => { target = transitionTarget(gl, () => input, () => [null, null, marker, null], () => [null, null], destination => { if (destination !== scene) throw new Error('Cold renderer must receive the exact source destination.'); map.getSource('ordinary').setData(destination); map.setLayoutProperty('ordinary', 'visibility', 'visible'); }); });
                const sample = () => run(() => {
                    const projected = map.project(feature.geometry.coordinates);
                    gl.enable(gl.SCISSOR_TEST); gl.scissor(0, 0, 2, 2);
                    gl.enable(gl.CULL_FACE); gl.cullFace(gl.FRONT_AND_BACK);
                    gl.blendEquationSeparate(gl.FUNC_REVERSE_SUBTRACT, gl.FUNC_SUBTRACT);
                    gl.colorMask(false, true, false, true);
                    const before = ['SCISSOR_BOX', 'CULL_FACE_MODE', 'BLEND_EQUATION_RGB', 'BLEND_EQUATION_ALPHA', 'COLOR_WRITEMASK'].map(name => [name, gl.getParameter(gl[name])]);
                    const inputError = gl.getError();
                    if (inputError !== gl.NO_ERROR) throw new Error('Sentinel input GL error: ' + inputError);
                    drawTransition(target, frame, [], 0.25);
                    for (const [name, value] of before) {
                        const actual = gl.getParameter(gl[name]);
                        if ((Array.isArray(value) || ArrayBuffer.isView(value)) ? !Array.from(value).every((element, index) => element === actual[index]) : actual !== value) throw new Error('Caller state changed: ' + name);
                    }
                    if (!gl.isEnabled(gl.SCISSOR_TEST) || !gl.isEnabled(gl.CULL_FACE)) throw new Error('Caller capabilities changed.');
                    const ratio = gl.drawingBufferWidth / gl.canvas.clientWidth;
                    const pixel = new Uint8Array(4);
                    gl.readPixels(Math.round(projected.x * ratio), Math.round(gl.drawingBufferHeight - projected.y * ratio), 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
                    gl.disable(gl.SCISSOR_TEST); gl.disable(gl.CULL_FACE); gl.blendEquation(gl.FUNC_ADD); gl.colorMask(true, true, true, true);
                    if (pixel[3] === 0) throw new Error('The served point must be painted at its actual projected camera position.');
                    const error = gl.getError();
                    if (error !== gl.NO_ERROR) throw new Error('Projection GL error ' + error + ', painted alpha ' + pixel[3]);
                    return { x: projected.x, y: projected.y, pixel: Array.from(pixel) };
                });
                const first = await sample();
                await new Promise(resolve => { map.once('moveend', resolve); map.panBy([60, 0], { duration: 0 }); });
                const moved = await sample();
                await run(() => drawTransition(target, frame, [], 1));
                await new Promise(resolve => map.once('idle', resolve));
                const coldPixel = await run(() => {
                    const projected = map.project(feature.geometry.coordinates);
                    const ratio = gl.drawingBufferWidth / gl.canvas.clientWidth;
                    const pixel = new Uint8Array(4);
                    gl.readPixels(Math.round(projected.x * ratio), Math.round(gl.drawingBufferHeight - projected.y * ratio), 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
                    return Array.from(pixel);
                });
                await run(() => disposeTransition(target));
                map.remove(); return { first, moved, coldPixel };
            }, collection);
            assert.notEqual(result.first.x, result.moved.x, 'Camera motion changes the projected position of the same served point.');
            assert.equal(result.first.pixel[3], 255, 'The initial point is visible through the official projection.');
            assert.equal(result.moved.pixel[3], 255, 'The point remains visible at its moved projected position.');
            assert.deepEqual(result.coldPixel, result.moved.pixel, 'Fraction one presents the same point through the ordinary cold renderer.');
        } finally { await browser.close(); }
    });
}

for (const engine of [chromium, webkit]) {
    test(`${engine.name()} served water uses the shared crossfade path and exact cold water look`, async () => {
        const browser = await engine.launch(engine === chromium ? { args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] }
            : process.env.FSHARP_WEBKIT_EXECUTABLE ? { executablePath: process.env.FSHARP_WEBKIT_EXECUTABLE } : {});
        try {
            const page = await browser.newPage();
            await page.context().route('http://canonical.test/**', async route => {
                const path = new URL(route.request().url()).pathname;
                if (path === '/') await route.fulfill({ contentType: 'text/html', body: '<div id="map" style="width:320px;height:320px"></div>' });
                else await route.fulfill({ contentType: 'text/javascript', path: viewer + '/node_modules/maplibre-gl/dist/' + path.slice(1) });
            });
            await page.goto('http://canonical.test/');
            await page.addScriptTag({ type: 'module', content: "import * as maplibregl from '/maplibre-gl.mjs';globalThis.maplibregl=maplibregl;" });
            await page.waitForFunction(() => globalThis.maplibregl?.Map);
            for (const path of ['src/retained-gpu.js', 'src/retained-labels.js', 'src/transition-target.js']) await page.addScriptTag({ path: viewer + '/' + path });
            const result = await page.evaluate(async source => {
                const after = { ...source, features: [] }, before = { ...after, waters: [] };
                const water = after.waters[0];
                if (!water) throw new Error('The served response must contain water.');
                const map = new maplibregl.Map({ container: 'map', style: { version: 8, sources: {}, layers: [] }, center: water.geometry.coordinates[0][0][0], zoom: 4, fadeDuration: 0 });
                await new Promise(resolve => map.once('load', resolve));
                const look = [{ fill: [30, 110, 210, 255] }, null];
                const waterLook = value => {
                    if (!after.waters.includes(value)) throw new Error('Water look must receive the original served WaterBody.');
                    return look;
                };
                const ordinary = { type: 'FeatureCollection', features: after.waters.map(value => ({ type: 'Feature', properties: {}, geometry: value.geometry })) };
                map.addSource('ordinary-water', { type: 'geojson', data: ordinary });
                map.addLayer({ id: 'ordinary-water', source: 'ordinary-water', type: 'fill', layout: { visibility: 'none' }, paint: { 'fill-color': `rgb(${look[0].fill.slice(0, 3).join(',')})` } });
                let gl, input, pending, target, coldCount = 0;
                map.addLayer({ id: 'water-target', type: 'custom', renderingMode: '2d', onAdd(_map, context) { gl = context; }, render(_gl, args) {
                    input = args;
                    if (!pending) return;
                    const operation = pending; pending = null;
                    try { operation.resolve(operation.run()); } catch (error) { operation.reject(error); }
                } });
                const run = action => new Promise((resolve, reject) => { pending = { run: action, resolve, reject }; map.triggerRepaint(); });
                await run(() => {
                    target = transitionTarget(gl, () => input, () => [null, null, null, null], waterLook, destination => {
                        if (destination !== after) throw new Error('Cold receives the unchanged full destination.');
                        coldCount++;
                        map.setLayoutProperty('ordinary-water', 'visibility', 'visible');
                    });
                });
                const frame = { before, after, change: { script: { steps: [] } } };
                const sample = (fraction, drawing = frame) => run(() => {
                    drawTransition(target, drawing, [], fraction);
                    if (coldCount !== 0) throw new Error('Served water must animate rather than immediately settle cold.');
                    const pixels = new Uint8Array(gl.drawingBufferWidth * gl.drawingBufferHeight * 4);
                    gl.readPixels(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
                    const maximum = pixels.filter((_value, index) => index % 4 === 3).reduce((value, next) => Math.max(value, next), 0);
                    if (gl.getError() !== gl.NO_ERROR) throw new Error('Water crossfade has a GL error.');
                    return maximum;
                });
                const quarter = await sample(0.25), threeQuarters = await sample(0.75);
                await run(() => drawTransition(target, frame, [], 1));
                await new Promise(resolve => map.once('idle', resolve));
                const coldAlpha = await run(() => {
                    const pixels = new Uint8Array(gl.drawingBufferWidth * gl.drawingBufferHeight * 4);
                    gl.readPixels(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
                    return pixels.filter((_value, index) => index % 4 === 3).reduce((value, next) => Math.max(value, next), 0);
                });
                await run(() => {
                    if (target.geometry.length !== 0 || target.surfaces.length !== 0) throw new Error('Water settlement retains geometry or scene resources.');
                    disposeTransition(target);
                });
                map.remove(); return { quarter, threeQuarters, coldAlpha, coldCount };
            }, served);
            assert.equal(result.quarter, 64, 'Water appears at the supplied quarter fraction.');
            assert.equal(result.threeQuarters, 191, 'Water appears at the supplied three-quarter fraction.');
            assert.equal(result.coldAlpha, 255, 'The ordinary cold renderer presents the full water look.');
            assert.equal(result.coldCount, 1, 'The full destination settles cold once.');
        } finally { await browser.close(); }
    });
}

for (const engine of [chromium, webkit]) {
    test(`${engine.name()} current camera refreshes water and feature looks without rebuilding geometry`, async () => {
        const browser = await engine.launch(engine === chromium ? { args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] }
            : process.env.FSHARP_WEBKIT_EXECUTABLE ? { executablePath: process.env.FSHARP_WEBKIT_EXECUTABLE } : {});
        try {
            const page = await browser.newPage();
            await page.context().route('http://canonical.test/**', async route => {
                const path = new URL(route.request().url()).pathname;
                if (path === '/') await route.fulfill({ contentType: 'text/html', body: '<div id="map" style="width:320px;height:320px"></div>' });
                else await route.fulfill({ contentType: 'text/javascript', path: viewer + '/node_modules/maplibre-gl/dist/' + path.slice(1) });
            });
            await page.goto('http://canonical.test/');
            await page.addScriptTag({ type: 'module', content: "import * as maplibregl from '/maplibre-gl.mjs';globalThis.maplibregl=maplibregl;" });
            await page.waitForFunction(() => globalThis.maplibregl?.Map);
            for (const path of ['src/retained-gpu.js', 'src/retained-labels.js', 'src/transition-target.js']) await page.addScriptTag({ path: viewer + '/' + path });
            const result = await page.evaluate(async source => {
                const water = source.waters.find(value => value.minZoom !== null);
                if (!water) throw new Error('The served fixture must provide a minimum water zoom.');
                const map = new maplibregl.Map({ container: 'map', style: { version: 8, sources: {}, layers: [] }, center: water.geometry.coordinates[0][0][0], zoom: water.minZoom, fadeDuration: 0 });
                await new Promise(resolve => map.once('load', resolve));
                const point = source.features.find(value => {
                    if (value.geometry.type !== 'Point') return false;
                    const screen = map.project(value.geometry.coordinates);
                    return screen.x > 16 && screen.x < 304 && screen.y > 16 && screen.y < 304;
                });
                if (!point) throw new Error('The source fixture must include a served point in the water camera viewport.');
                await new Promise(resolve => { map.once('moveend', resolve); map.setZoom(water.minZoom - 1); });
                const scene = { ...source, features: [point], waters: [water] };
                const frame = { before: scene, after: scene, change: { script: { steps: [] } } };
                let gl, input, target, pending, buffers = 0;
                const visible = () => map.getZoom() >= water.minZoom;
                const marker = { color: [240, 60, 50, 255], size: 10 };
                map.addLayer({ id: 'camera-look-target', type: 'custom', renderingMode: '2d', onAdd(_map, context) {
                    gl = context;
                    const allocate = gl.createBuffer.bind(gl);
                    gl.createBuffer = () => { buffers++; return allocate(); };
                }, render(_context, args) {
                    input = args;
                    if (!pending) return;
                    const operation = pending; pending = null;
                    try { operation.resolve(operation.run()); } catch (error) { operation.reject(error); }
                } });
                const run = action => new Promise((resolve, reject) => { pending = { run: action, resolve, reject }; map.triggerRepaint(); });
                await run(() => { target = transitionTarget(gl, () => input, () => [null, null, visible() ? marker : null, null],
                    () => [visible() ? { fill: [30, 110, 210, 255] } : null, null], () => {}); });
                const sample = () => run(() => {
                    drawTransition(target, frame, [], 0.5);
                    const pixels = new Uint8Array(gl.drawingBufferWidth * gl.drawingBufferHeight * 4);
                    gl.readPixels(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
                    let waterPixels = 0, pointPixels = 0;
                    for (let i = 0; i < pixels.length; i += 4) if (pixels[i + 3] > 0) {
                        if (pixels[i + 2] > pixels[i]) waterPixels++;
                        if (pixels[i] > pixels[i + 2]) pointPixels++;
                    }
                    if (gl.getError() !== gl.NO_ERROR) throw new Error('Camera look refresh has no GL error.');
                    return { waterPixels, pointPixels, buffers };
                });
                const hidden = await sample();
                await new Promise(resolve => { map.once('moveend', resolve); map.setZoom(water.minZoom); });
                const shown = await sample();
                await run(() => disposeTransition(target)); map.remove(); return { hidden, shown };
            }, served);
            assert.equal(result.hidden.waterPixels, 0, 'The served minimum zoom hides water initially.');
            assert.equal(result.hidden.pointPixels, 0, 'The same camera-dependent look hides the point initially.');
            assert.ok(result.shown.waterPixels > 0, 'The current camera reveals served water on the same frame.');
            assert.ok(result.shown.pointPixels > 0, 'The current camera reveals the served point on the same frame.');
            assert.equal(result.shown.buffers, result.hidden.buffers, 'Changing only the camera look retains the existing geometry buffers.');
        } finally { await browser.close(); }
    });
}

for (const engine of [chromium, webkit]) {
    test(`${engine.name()} served matched morph uses the canonical GPU buffers and exact cold destination`, async () => {
        const feature = served.features.find(feature => feature.geometry.type === 'MultiPolygon' && feature.geometry.coordinates.length === 1 && feature.geometry.coordinates[0].length === 1);
        assert.ok(feature, 'The served fixture contains an admitted single-ring region.');
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
            const result = await page.evaluate(async feature => {
                const points = feature.geometry.coordinates[0][0];
                const bounds = points.reduce((bounds, point) => bounds.extend(point), new maplibregl.LngLatBounds(points[0], points[0]));
                const map = new maplibregl.Map({ container: 'map', style: { version: 8, sources: {}, layers: [] }, bounds, fitBoundsOptions: { padding: 30 }, fadeDuration: 0 });
                await new Promise(resolve => map.once('load', resolve));
                let pending = null, target = null, input = null, gl = null, tracking = false, cold = 0, allocations = 0, releases = 0;
                const live = new Set(), selected = [];
                const before = { type: 'FeatureCollection', features: [], waters: [] };
                const after = { type: 'FeatureCollection', features: [feature], waters: [] };
                const original = JSON.stringify(after);
                const vector = ([lon, lat]) => { lat *= Math.PI / 180; lon *= Math.PI / 180; return { x: Math.cos(lat) * Math.cos(lon), y: Math.cos(lat) * Math.sin(lon), z: Math.sin(lat) }; };
                const to_pts = points.map(vector), from_pts = to_pts.map(() => to_pts[0]);
                const frame = { before, after, change: { script: { steps: [{ step: { step: 'morph', entity: feature.id, boundary: { kind: 'ring', parts: [] }, from_pts, to_pts }, clip: null }] } } };
                map.addLayer({ id: 'matched-morph', type: 'custom', renderingMode: '2d', onAdd(_map, context) {
                    gl = context;
                    for (const kind of ['Buffer', 'VertexArray', 'Shader', 'Program', 'Texture', 'Framebuffer', 'Renderbuffer']) {
                        const create = gl['create' + kind].bind(gl), drop = gl['delete' + kind].bind(gl);
                        gl['create' + kind] = (...args) => { const value = create(...args); if (tracking && value) { live.add(value); allocations++; } return value; };
                        gl['delete' + kind] = value => { if (live.delete(value)) releases++; drop(value); };
                    }
                }, render(_context, args) {
                    input = args;
                    if (!pending) return;
                    const operation = pending; pending = null; tracking = true;
                    try { operation.resolve(operation.run()); } catch (error) { operation.reject(error); } finally { tracking = false; }
                } });
                const run = action => new Promise((resolve, reject) => { pending = { run: action, resolve, reject }; map.triggerRepaint(); });
                await run(() => { target = transitionTarget(gl, () => input, properties => { selected.push(properties); return [{ fill: [190, 208, 140, 255] }, null, null, null]; }, () => [null, null], destination => {
                    if (destination !== after || JSON.stringify(destination) !== original) throw new Error('Cold destination must remain the original unchanged served collection.');
                    cold++;
                }); });
                const sample = (fraction, drawing = frame) => run(() => {
                    drawTransition(target, drawing, [], fraction);
                    const pixels = new Uint8Array(gl.drawingBufferWidth * gl.drawingBufferHeight * 4);
                    gl.readPixels(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
                    const program = target.programs.get(input.shaderData.variantName);
                    let opaque = 0, visible = 0;
                    for (let index = 3; index < pixels.length; index += 4) {
                        if (pixels[index] >= 250) opaque++;
                        if (pixels[index] > 0) visible++;
                    }
                    return { opaque, visible, fraction: gl.getUniform(program.value, program.loc.uMorph) };
                });
                const early = await sample(0.25), late = await sample(0.75);
                const crossfades = target.crossfades;
                const clipped = await sample(0.5, { ...frame, change: { script: { steps: [{ ...frame.change.script.steps[0], clip: 'source-subset' }] } } });
                const fallbackCrossfades = target.crossfades;
                await run(() => drawTransition(target, frame, [], 1));
                for (let cycle = 0; cycle < 20; cycle++) await run(() => {
                    drawTransition(target, frame, [], cycle % 2 === 0 ? 0.25 : 0.75);
                    drawTransition(target, frame, [], 1);
                    if (target.geometry.length !== 0 || target.extras.length !== 0) throw new Error('Settlement retained morph buffers.');
                });
                await run(() => disposeTransition(target));
                const result = { early, late, clipped, crossfades, fallbackCrossfades, cold, allocations, releases, live: live.size, sameProperties: selected.every(value => value === feature.properties), error: gl.getError() };
                map.remove(); return result;
            }, feature);
            assert.ok(result.early.opaque > 0, 'An early matched border paints opaque land rather than fading the full destination.');
            assert.ok(result.late.opaque > result.early.opaque, 'The served paired border expands across the source subset as time advances.');
            assert.equal(result.early.fraction, 0.25, 'The GPU receives the supplied early fraction without another easing.');
            assert.equal(result.late.fraction, 0.75, 'The GPU receives the supplied later fraction without another easing.');
            assert.equal(result.crossfades, 0, 'An admitted matched morph is not counted as missing correspondence.');
            assert.equal(result.fallbackCrossfades, 1, 'Unsupported clipped correspondence is counted through the existing crossfade.');
            assert.ok(result.clipped.visible > result.early.opaque, 'The fallback retains the full served destination instead of omitting its feature.');
            assert.equal(result.clipped.opaque, 0, 'The unsupported subset retains the ordinary half-opacity scene crossfade.');
            assert.equal(result.cold, 21, 'Every endpoint uses the ordinary cold destination callback.');
            assert.equal(result.sameProperties, true, 'The one painter receives the original served entity properties.');
            assert.equal(result.allocations, result.releases, 'The matched path frees every allocation after twenty further moves.');
            assert.equal(result.live, 0, 'No matched-path GL allocation survives disposal.');
            assert.equal(result.error, 0, 'The matched path leaves no GL error.');
            assert.deepEqual(errors, [], 'The matched path produces no browser errors.');
        } finally { await browser.close(); }
    });
}
