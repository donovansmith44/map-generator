import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
const require = createRequire(import.meta.url);
const { chromium } = require(require.resolve('playwright-core', { paths: [process.cwd()] }));
const vendor = fileURLToPath(new URL('../src/', import.meta.url));

test('native static fills ignore incoming stencil contents and preserve the caller clip buffer', async () => {
    // Arrange
    const fixtures = JSON.parse(readFileSync(new URL('fixtures/native-globe.json', import.meta.url)));
    const browser = await chromium.launch({ executablePath: process.env.PROOF_CHROMIUM, args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
    try {
        const page = await browser.newPage();
        await page.setContent('<canvas width="128" height="128"></canvas>');
        for (const script of ['libtess.js', 'retained-gpu.js', 'limb.js', 'retained-labels.js', 'transition-target.js']) await page.addScriptTag({ path: vendor + script });
        // Act
        const actual = await page.evaluate(fixture => {
            const gl = document.querySelector('canvas').getContext('webgl2', { stencil: true, antialias: false });
            let allocated = 0, released = 0;
            const allocate = gl.createFramebuffer.bind(gl), release = gl.deleteFramebuffer.bind(gl);
            gl.createFramebuffer = () => { allocated++; return allocate(); };
            gl.deleteFramebuffer = value => { if (value) released++; return release(value); };
            const matrix = [1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1];
            const projection = { shaderData: { variantName: 'stencil-law', define: '', vertexShaderPrelude: `
const float PI = 3.141592653589793;
#define GLOBE_RADIUS 6371008.8
uniform mat4 u_projection_matrix;
uniform mat4 u_projection_fallback_matrix;
uniform vec4 u_projection_tile_mercator_coords;
uniform vec4 u_projection_clipping_plane;
uniform float u_projection_transition;
vec4 interpolateProjection(vec2 tile, vec3 point, float elevation) { return u_projection_matrix * vec4(point, 1.0); }
` }, defaultProjectionData: { mainMatrix: matrix, fallbackMatrix: matrix, tileMercatorCoords: [0, 0, 1, 1], clippingPlane: [0, 1, 0, 0], projectionTransition: 1 } };
            const key = reference => reference.store + ':' + reference.key;
            const arcs = new Map(fixture.arcs.map(arc => [key(arc.reference), arc.points]));
            const ring = cycle => cycle.flatMap(edge => (edge.orientation === 'Reverse' ? [...arcs.get(key(edge.boundary))].reverse() : arcs.get(key(edge.boundary))).slice(0, -1));
            const collection = { features: [{ id: fixture.name, properties: {}, geometry: { type: 'MultiPolygon', coordinates: fixture.regions.parts.map(part => [ring(part.cycle), ...part.holes.map(ring)]) } }], waters: [] };
            const target = globalThis.transitionTarget(gl, () => projection, () => [{ fill: [255, 0, 0, 255] }, null, null, null], () => [], () => {}, r => r.id);
            const render = stencil => {
                gl.stencilMask(255); gl.clearStencil(stencil); gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT | gl.STENCIL_BUFFER_BIT);
                globalThis.drawStatic(target, collection);
                const pixels = new Uint8Array(128 * 128 * 4); gl.readPixels(0, 0, 128, 128, gl.RGBA, gl.UNSIGNED_BYTE, pixels); return pixels;
            };
            const clean = render(0), incoming = render(255), retained = allocated === 1;
            const compile = (kind, source) => { const shader = gl.createShader(kind); gl.shaderSource(shader, source); gl.compileShader(shader); return shader; };
            const program = gl.createProgram();
            const vertex = compile(gl.VERTEX_SHADER, '#version 300 es\nvoid main(){vec2 p=vec2((gl_VertexID<<1)&2,gl_VertexID&2);gl_Position=vec4(p*2.0-1.0,0,1);}');
            const fragment = compile(gl.FRAGMENT_SHADER, '#version 300 es\nprecision highp float;out vec4 color;void main(){color=vec4(1,0,0,1);}');
            gl.attachShader(program, vertex); gl.attachShader(program, fragment); gl.linkProgram(program); gl.useProgram(program); gl.bindVertexArray(null);
            gl.clear(gl.COLOR_BUFFER_BIT); gl.disable(gl.BLEND); gl.enable(gl.STENCIL_TEST); gl.stencilFunc(gl.EQUAL, 255, 255); gl.stencilOp(gl.KEEP, gl.KEEP, gl.KEEP); gl.drawArrays(gl.TRIANGLES, 0, 3);
            const sentinel = new Uint8Array(128 * 128 * 4); gl.readPixels(0, 0, 128, 128, gl.RGBA, gl.UNSIGNED_BYTE, sentinel);
            const result = { independent: clean.every((value, i) => value === incoming[i]), callerClipPreserved: sentinel.every((value, i) => value === (i % 4 === 0 || i % 4 === 3 ? 255 : 0)), visible: clean.some(value => value !== 0), retained, error: gl.getError() };
            gl.useProgram(null); gl.deleteProgram(program); gl.deleteShader(vertex); gl.deleteShader(fragment);
            gl.canvas.width = 160; globalThis.drawStatic(target, collection); globalThis.disposeTransition(target);
            return { ...result, allocated, released, finalError: gl.getError() };
        }, fixtures[0]);
        // Assert
        assert.deepEqual(actual, { independent: true, callerClipPreserved: true, visible: true, retained: true, error: 0, allocated: 2, released: 2, finalError: 0 });
    } finally { await browser.close(); }
});

test('the retained native renderer and its pixel picker agree on original globe controls', async () => {
    // Arrange
    const fixtures = JSON.parse(readFileSync(new URL('fixtures/native-globe.json', import.meta.url)));
    const browser = await chromium.launch({ executablePath: process.env.PROOF_CHROMIUM, args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
    try {
        const page = await browser.newPage();
        await page.setContent('<canvas width="600" height="600" style="width:600px;height:600px"></canvas>');
        for (const script of ['libtess.js', 'retained-gpu.js', 'limb.js', 'retained-labels.js', 'transition-target.js']) await page.addScriptTag({ path: vendor + script });
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
        for (const script of ['libtess.js', 'retained-gpu.js', 'limb.js', 'retained-labels.js', 'transition-target.js']) await page.addScriptTag({ path: vendor + script });
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

test('native even-odd coverage agrees with an independent oracle through near-coincident ring vertices', async () => {
    // Arrange
    const maplibre = readFileSync(process.env.MG_MAPLIBRE_MODULE ?? new URL('../node_modules/maplibre-gl/dist/maplibre-gl.mjs', import.meta.url), 'utf8');
    const browser = await chromium.launch({ executablePath: process.env.PROOF_CHROMIUM, args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
    try {
        const page = await browser.newPage({ viewport: { width: 1392, height: 753 } });
        await page.route('http://renderer.test/**', route => route.fulfill({ contentType: route.request().url().endsWith('.mjs') ? 'text/javascript' : 'text/html',
            body: route.request().url().endsWith('.mjs') ? maplibre : '<div id="map" style="position:absolute;inset:0"></div>' }));
        await page.goto('http://renderer.test/');
        for (const script of ['libtess.js', 'retained-gpu.js', 'limb.js', 'retained-labels.js', 'transition-target.js']) await page.addScriptTag({ path: vendor + script });
        // Act
        const actual = await page.evaluate(async () => {
            const { Map: GlobeMap, setWorkerCount } = await import('/maplibre.mjs');
            setWorkerCount(1);
            const map = new GlobeMap({ container: 'map', style: { version: 8, sources: {}, layers: [] }, center: [36, 32.75], zoom: 5.5, attributionControl: false, antialias: false });
            await new Promise(resolve => map.on('load', resolve));
            map.setProjection({ type: 'globe' });
            const points = Array.from({ length: 16 }, (_, index) => {
                const angle = index * Math.PI * 2 / 16, radius = 40 * (0.6 + 0.3 * Math.sin(5 * angle) + 0.1 * Math.sin(13 * angle));
                return [0, 0.0001].map(offset => {
                    const longitude = (36 + radius * Math.cos(angle) + offset) * Math.PI / 180, latitude = (32.75 + radius * 0.4 * Math.sin(angle) + offset) * Math.PI / 180;
                    return { x: Math.cos(latitude) * Math.cos(longitude), y: Math.cos(latitude) * Math.sin(longitude), z: Math.sin(latitude) };
                });
            }).flat();
            let target, completed = false;
            const result = await new Promise((resolve, reject) => {
                map.addLayer({ id: 'native-coverage-law', type: 'custom', renderingMode: '2d', render(gl, projection) {
                    if (completed) return;
                    if (projection.defaultProjectionData.projectionTransition < 0.999) { map.triggerRepaint(); return; }
                    try {
                        const link = gl.linkProgram.bind(gl);
                        gl.linkProgram = program => { gl.transformFeedbackVaryings(program, ['gl_Position'], gl.INTERLEAVED_ATTRIBS); link(program); };
                        target = globalThis.transitionTarget(gl, () => projection, () => [{ fill: [255, 255, 255, 255] }, null, null, null], () => [], () => {}, reference => reference.id);
                        gl.linkProgram = link;
                        const render = ring => {
                            globalThis.resetTransition(target);
                            gl.clearColor(0, 0, 0, 0); gl.clearStencil(0); gl.clear(gl.COLOR_BUFFER_BIT | gl.STENCIL_BUFFER_BIT);
                            globalThis.drawStatic(target, { features: [{ id: 'coverage-law', properties: {}, geometry: { type: 'MultiPolygon', coordinates: [[ring]] } }], waters: [] });
                            const pixels = new Uint8Array(gl.drawingBufferWidth * gl.drawingBufferHeight * 4);
                            gl.readPixels(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
                            return pixels;
                        };
                        const original = render(points), program = [...target.programs.values()][0], ring = target.staticScene[0].items[0].rings[0];
                        const positions = new Float32Array(points.length * 4), feedback = gl.createBuffer();
                        const previousProgram = gl.getParameter(gl.CURRENT_PROGRAM), previousArray = gl.getParameter(gl.VERTEX_ARRAY_BINDING);
                        gl.useProgram(program.value); gl.uniform1f(program.loc.uScreen, 0); gl.uniform1f(program.loc.uLabel, 0); gl.uniform1f(program.loc.uFill, 1); gl.uniform1f(program.loc.uMorph, 1);
                        gl.bindVertexArray(ring.vao); gl.bindBuffer(gl.TRANSFORM_FEEDBACK_BUFFER, feedback); gl.bufferData(gl.TRANSFORM_FEEDBACK_BUFFER, positions.byteLength, gl.DYNAMIC_READ);
                        gl.bindBufferBase(gl.TRANSFORM_FEEDBACK_BUFFER, 0, feedback); gl.enable(gl.RASTERIZER_DISCARD); gl.beginTransformFeedback(gl.POINTS);
                        gl.drawArrays(gl.POINTS, ring.first, ring.count); gl.endTransformFeedback(); gl.disable(gl.RASTERIZER_DISCARD); gl.getBufferSubData(gl.TRANSFORM_FEEDBACK_BUFFER, 0, positions);
                        gl.bindBufferBase(gl.TRANSFORM_FEEDBACK_BUFFER, 0, null); gl.bindBuffer(gl.TRANSFORM_FEEDBACK_BUFFER, null); gl.deleteBuffer(feedback); gl.useProgram(previousProgram); gl.bindVertexArray(previousArray);
                        const rotated = render([...points.slice(31), ...points.slice(0, 31)]);
                        const oracle = document.createElement('canvas'); oracle.width = gl.drawingBufferWidth; oracle.height = gl.drawingBufferHeight;
                        const outline = document.createElement('canvas'); outline.width = oracle.width; outline.height = oracle.height;
                        const context = oracle.getContext('2d'), boundary = outline.getContext('2d');
                        context.fillStyle = 'white'; boundary.strokeStyle = 'white'; boundary.lineWidth = 4;
                        for (const drawing of [context, boundary]) {
                            drawing.beginPath();
                            for (let index = 0; index < positions.length; index += 4) {
                                const x = (positions[index] / positions[index + 3] + 1) * oracle.width / 2, y = (1 - positions[index + 1] / positions[index + 3]) * oracle.height / 2;
                                if (index) drawing.lineTo(x, y); else drawing.moveTo(x, y);
                            }
                            drawing.closePath();
                        }
                        context.fill('evenodd'); boundary.stroke();
                        const expected = context.getImageData(0, 0, oracle.width, oracle.height).data, excluded = boundary.getImageData(0, 0, oracle.width, oracle.height).data;
                        let rotationMismatches = 0, oracleMismatches = 0;
                        for (let y = 0; y < oracle.height; y++) for (let x = 0; x < oracle.width; x++) {
                            const gpu = ((oracle.height - 1 - y) * oracle.width + x) * 4 + 3, cpu = (y * oracle.width + x) * 4 + 3;
                            if (original[gpu] !== rotated[gpu]) rotationMismatches++;
                            if (excluded[cpu] === 0 && (original[gpu] > 127) !== (expected[cpu] > 127)) oracleMismatches++;
                        }
                        completed = true; queueMicrotask(() => resolve({ rotationMismatches, oracleMismatches, visible: original.some(value => value !== 0), projectionFinite: positions.every(Number.isFinite), error: gl.getError() }));
                    } catch (error) { completed = true; reject(error); }
                }, onRemove() { if (target) globalThis.disposeTransition(target); } });
                map.triggerRepaint();
            });
            map.remove(); return result;
        });
        // Assert
        assert.deepEqual(actual, { rotationMismatches: 0, oracleMismatches: 0, visible: true, projectionFinite: true, error: 0 });
    } finally { await browser.close(); }
});
