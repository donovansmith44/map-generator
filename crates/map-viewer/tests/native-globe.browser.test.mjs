import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
const require = createRequire(import.meta.url);
const { chromium } = require(require.resolve('playwright-core', { paths: [process.cwd()] }));
const vendor = fileURLToPath(new URL('../src/', import.meta.url));

test('the retained native renderer and its pixel picker agree on original globe controls', async () => {
    // Arrange
    const fixtures = JSON.parse(readFileSync(new URL('fixtures/native-globe.json', import.meta.url)));
    const browser = await chromium.launch({ executablePath: process.env.PROOF_CHROMIUM, args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
    try {
        const page = await browser.newPage();
        await page.setContent('<canvas width="600" height="600" style="width:600px;height:600px"></canvas>');
        for (const script of ['retained-gpu.js', 'limb.js', 'retained-labels.js', 'transition-target.js']) await page.addScriptTag({ path: vendor + script });
        // Act
        const actual = await page.evaluate(fixtures => {
            const canvas = document.querySelector('canvas');
            const gl = canvas.getContext('webgl2', { stencil: true, antialias: false, preserveDrawingBuffer: true });
            const failures = [];
            let probes = 0;
            const dot = (a, b) => a.reduce((sum, value, index) => sum + value * b[index], 0);
            const xyz = p => [p.x, p.y, p.z];
            const key = reference => reference.store + ':' + reference.key;
            for (const fixture of fixtures) {
                const C = xyz(fixture.camera);
                const length = Math.hypot(C[0], C[1]);
                const E = length ? [-C[1] / length, C[0] / length, 0] : [0, 1, 0];
                const N = [C[1] * E[2] - C[2] * E[1], C[2] * E[0] - C[0] * E[2], C[0] * E[1] - C[1] * E[0]];
                const scale = 0.8;
                const matrix = [E[1] * scale, N[1] * scale, -C[1], 0, E[2] * scale, N[2] * scale, -C[2], 0, E[0] * scale, N[0] * scale, -C[0], 0, 0, 0, 0, 1];
                const projection = { shaderData: { variantName: 'globe', define: '', vertexShaderPrelude: `
const float PI = 3.141592653589793;
#define GLOBE_RADIUS 6371008.8
uniform mat4 u_projection_matrix;
uniform mat4 u_projection_fallback_matrix;
uniform vec4 u_projection_tile_mercator_coords;
uniform vec4 u_projection_clipping_plane;
uniform float u_projection_transition;
vec4 interpolateProjection(vec2 tile, vec3 point, float elevation) {
    vec4 position = u_projection_matrix * vec4(point, 1.0);
    position.z = (1.0 - dot(point, u_projection_clipping_plane.xyz)) * position.w;
    return position;
}` }, defaultProjectionData: { mainMatrix: matrix, fallbackMatrix: matrix, tileMercatorCoords: [0, 0, 1, 1], clippingPlane: [C[1], C[2], C[0], 0], projectionTransition: 1 } };
                const arcs = new Map(fixture.arcs.map(arc => [key(arc.reference), arc.points]));
                const ring = cycle => cycle.flatMap(edge => {
                    const points = arcs.get(key(edge.boundary));
                    return (edge.orientation === 'Reverse' ? [...points].reverse() : points).slice(0, -1);
                });
                const geometry = { type: 'MultiPolygon', coordinates: fixture.regions.parts.map(part => [ring(part.cycle), ...part.holes.map(ring)]) };
                const collection = { features: [{ id: fixture.name, geometry, properties: {} }], waters: [] };
                const target = globalThis.transitionTarget(gl, () => projection, () => [{ fill: [255, 255, 255, 255] }, null, null, null], () => [], () => {}, r => typeof r === 'string' ? r : r.id);
                gl.clearColor(0, 0, 0, 0); gl.clearStencil(0); gl.clear(gl.COLOR_BUFFER_BIT | gl.STENCIL_BUFFER_BIT);
                globalThis.drawStatic(target, collection);
                for (const probe of fixture.probes) {
                    const p = xyz(probe.point);
                    if (dot(C, p) <= 0.02) continue;
                    const x = Math.floor(canvas.width / 2 * (1 + scale * dot(p, E)));
                    const y = Math.floor(canvas.height / 2 * (1 + scale * dot(p, N)));
                    const pixel = new Uint8Array(4);
                    gl.readPixels(x, y, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
                    const visible = pixel[3] > 127;
                    const picked = globalThis.pickTransition(target, { x, y: canvas.height - 1 - y }, new Set([fixture.name]));
                    probes++;
                    if (visible !== probe.inside || (picked === fixture.name) !== probe.inside) failures.push({ name: fixture.name, expected: probe.inside, visible, picked });
                }
                if (fixture.transition) {
                    const body = points => ({ features: [{ id: fixture.name, properties: {}, geometry: { type: 'MultiPolygon', coordinates: [[points]] } }], waters: [] });
                    const render = (from, to, fraction) => {
                        globalThis.resetTransition(target);
                        gl.clear(gl.COLOR_BUFFER_BIT | gl.STENCIL_BUFFER_BIT);
                        const frame = { before: body(from), after: body(to), change: { script: { steps: [{ step: { ...fixture.transition, from_pts: from, to_pts: to } }] } } };
                        target.cold = destination => globalThis.drawStatic(target, destination);
                        globalThis.drawTransition(target, frame, [], fraction);
                        const pixels = new Uint8Array(canvas.width * canvas.height * 4);
                        gl.readPixels(0, 0, canvas.width, canvas.height, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
                        return pixels;
                    };
                    const same = (a, b) => a.every((value, index) => value === b[index]);
                    let previous;
                    for (const sample of fixture.animationProbes) {
                        const pixels = render(fixture.transition.from_pts, fixture.transition.to_pts, sample.fraction);
                        const inverse = render(fixture.transition.to_pts, fixture.transition.from_pts, 1 - sample.fraction);
                        const cold = render(sample.points, sample.points, 1);
                        if (!same(pixels, inverse)) failures.push({ name: fixture.name, fraction: sample.fraction, error: 'inverse raster differs' });
                        if (!same(pixels, cold)) failures.push({ name: fixture.name, fraction: sample.fraction, error: 'cold raster differs' });
                        if (previous && same(previous, pixels)) failures.push({ name: fixture.name, fraction: sample.fraction, error: 'frame did not change' });
                        previous = pixels;
                        for (const probe of sample.probes) {
                            const p = xyz(probe.point);
                            const x = Math.floor(canvas.width / 2 * (1 + scale * dot(p, E)));
                            const y = Math.floor(canvas.height / 2 * (1 + scale * dot(p, N)));
                            const visible = pixels[(y * canvas.width + x) * 4 + 3] > 127;
                            probes++;
                            if (visible !== probe.inside) failures.push({ name: fixture.name, fraction: sample.fraction, expected: probe.inside, visible });
                        }
                    }
                }
                const error = gl.getError();
                if (error) failures.push({ name: fixture.name, error });
                globalThis.disposeTransition(target);
            }
            return { probes, failures };
        }, fixtures);
        // Assert
        assert.deepEqual(actual, { probes: 37, failures: [] });
    } finally { await browser.close(); }
});

test('native fills and pixel picks stay aligned with the MapLibre globe camera', async () => {
    // Arrange
    const fixtures = JSON.parse(readFileSync(new URL('fixtures/native-globe.json', import.meta.url)));
    const maplibre = readFileSync(process.env.MG_MAPLIBRE_MODULE ?? new URL('../node_modules/maplibre-gl/dist/maplibre-gl.mjs', import.meta.url), 'utf8');
    const browser = await chromium.launch({ executablePath: process.env.PROOF_CHROMIUM, args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
    try {
        const page = await browser.newPage({ viewport: { width: 600, height: 600 } });
        await page.route('http://renderer.test/**', route => route.fulfill({ contentType: route.request().url().endsWith('.mjs') ? 'text/javascript' : 'text/html',
            body: route.request().url().endsWith('.mjs') ? maplibre : '<div id="map" style="position:absolute;inset:0"></div>' }));
        await page.goto('http://renderer.test/');
        for (const script of ['retained-gpu.js', 'limb.js', 'retained-labels.js', 'transition-target.js']) await page.addScriptTag({ path: vendor + script });
        // Act
        const actual = await page.evaluate(async fixtures => {
            const { Map: GlobeMap, setWorkerCount } = await import('/maplibre.mjs');
            setWorkerCount(1);
            const map = new GlobeMap({ container: 'map', style: { version: 8, sources: {}, layers: [] }, center: [0, 0], zoom: 1, attributionControl: false });
            await new Promise(resolve => map.on('load', resolve));
            map.setProjection({ type: 'globe' });
            const geographic = point => [Math.atan2(point.y, point.x) * 180 / Math.PI, Math.atan2(point.z, Math.hypot(point.x, point.y)) * 180 / Math.PI];
            const failures = [];
            let probes = 0;
            for (const fixture of fixtures) {
                const center = geographic(fixture.camera);
                map.jumpTo({ center, zoom: 1 });
                const key = reference => reference.store + ':' + reference.key;
                const arcs = new Map(fixture.arcs.map(arc => [key(arc.reference), arc.points]));
                const ring = cycle => cycle.flatMap(edge => {
                    const points = arcs.get(key(edge.boundary));
                    return (edge.orientation === 'Reverse' ? [...points].reverse() : points).slice(0, -1);
                });
                const collection = { features: [{ id: fixture.name, properties: {}, geometry: { type: 'MultiPolygon', coordinates: fixture.regions.parts.map(part => [ring(part.cycle), ...part.holes.map(ring)]) } }], waters: [] };
                await new Promise((resolve, reject) => {
                    let target, input, completed = false;
                    map.addLayer({ id: 'native-fixture', type: 'custom', renderingMode: '2d',
                        onAdd(_map, gl) { this.gl = gl; },
                        render(gl, projection) {
                            if (completed) return;
                            input = projection;
                            if (input.defaultProjectionData.projectionTransition < 0.999) { map.triggerRepaint(); return; }
                            try {
                                target ??= globalThis.transitionTarget(gl, () => input, () => [{ fill: [255, 255, 255, 255] }, null, null, null], () => [], () => {}, r => r.id);
                                globalThis.drawStatic(target, collection);
                                for (const probe of fixture.probes) {
                                    const point = map.project(geographic(probe.point));
                                    if (point.x < 0 || point.x >= 600 || point.y < 0 || point.y >= 600) continue;
                                    const front = fixture.camera.x * probe.point.x + fixture.camera.y * probe.point.y + fixture.camera.z * probe.point.z;
                                    if (front <= 0.1) continue;
                                    const pixel = new Uint8Array(4);
                                    gl.readPixels(Math.floor(point.x), gl.drawingBufferHeight - 1 - Math.floor(point.y), 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
                                    const visible = pixel[3] > 127;
                                    const picked = globalThis.pickTransition(target, point, new Set([fixture.name]));
                                    probes++;
                                    if (visible !== probe.inside || (picked === fixture.name) !== probe.inside) failures.push({ name: fixture.name, expected: probe.inside, visible, picked, point: probe.point, plane: input.defaultProjectionData.clippingPlane });
                                }
                                const error = gl.getError();
                                if (error) failures.push({ name: fixture.name, error });
                                completed = true;
                                queueMicrotask(resolve);
                            } catch (error) { completed = true; reject(error); }
                        },
                        onRemove() { if (target) globalThis.disposeTransition(target); },
                    });
                    map.triggerRepaint();
                });
                map.removeLayer('native-fixture');
            }
            map.remove();
            return { probes, failures };
        }, fixtures);
        // Assert
        assert.ok(actual.probes >= 12);
        assert.deepEqual(actual.failures, []);
    } finally { await browser.close(); }
});

test('exact camera motion clips and uploads bounded real-GL scratch', async t => {
    const browser = await chromium.launch({ executablePath: process.env.PROOF_CHROMIUM, args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
    try {
        const page = await browser.newPage();
        await page.setContent('<canvas></canvas>');
        for (const script of ['limb.js', 'retained-gpu.js']) await page.addScriptTag({ path: vendor + script });
        const result = await page.evaluate(() => {
            const gl = document.querySelector('canvas').getContext('webgl2');
            const loc = { aPos: 0, aPrev: 1, aArc: 2 };
            const ring = uploadGeometry(gl, loc, Float32Array.from([0.6, -0.8, 0, 0.6, 0, 0.8, -0.6, 0.8, 0, -0.6, 0, -0.8]));
            const times = [], pieces = new Set();
            for (let step = 0; step < 240; step++) {
                const angle = step * 0.00001, C = [Math.cos(angle), Math.sin(angle), 0];
                const start = performance.now();
                const draws = visibleFillDraws(gl, loc, ring, C);
                times.push(performance.now() - start);
                if (visibleFillDraws(gl, loc, ring, [...C]) !== draws) throw Error('unchanged direction reallocated');
                for (const draw of draws) pieces.add(draw);
            }
            disposeGeometry(gl, ring);
            times.sort((a, b) => a - b);
            return { changes: times.length, medianMs: times[120], p95Ms: times[228],
                released: [...pieces].every(piece => piece.vao === null && piece.buf === null), error: gl.getError() };
        });
        assert.equal(result.changes, 240);
        assert.equal(result.released, true);
        assert.equal(result.error, 0);
        t.diagnostic(`Four-vertex ring, Chromium SwiftShader, CPU clip + upload submission: median ${result.medianMs} ms, p95 ${result.p95Ms} ms across ${result.changes} exact camera changes; not a full-map frame budget.`);
    } finally { await browser.close(); }
});
