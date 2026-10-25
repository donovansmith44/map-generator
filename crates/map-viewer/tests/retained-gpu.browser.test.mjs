import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const { chromium, webkit } = require(require.resolve('playwright-core', { paths: [process.cwd()] }));
const source = fileURLToPath(new URL('../src/retained-gpu.js', import.meta.url));

for (const engine of [chromium, webkit]) {
    test(`${engine.name()} releases retained GPU resources across twenty back-and-forth moves`, async () => {
        const browser = await engine.launch(engine === chromium
            ? { args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] }
            : process.env.FSHARP_WEBKIT_EXECUTABLE ? { executablePath: process.env.FSHARP_WEBKIT_EXECUTABLE } : {});
        try {
            const page = await browser.newPage();
            const errors = [];
            page.on('pageerror', error => errors.push(error.message));
            await page.setContent('<canvas width="192" height="192"></canvas>');
            await page.addScriptTag({ path: fileURLToPath(new URL('../src/libtess.js', import.meta.url)) });
            await page.addScriptTag({ path: source });
            const result = await page.evaluate(() => {
                const gl = document.querySelector('canvas').getContext('webgl2', { stencil: true, antialias: false });
                if (!gl) throw new Error('The browser must provide WebGL2 for the retained renderer');
                const buffers = new Set();
                const arrays = new Set();
                let allocations = 0;
                let releases = 0;
                for (const [create, drop, live] of [['createBuffer', 'deleteBuffer', buffers], ['createVertexArray', 'deleteVertexArray', arrays]]) {
                    const allocate = gl[create].bind(gl);
                    const release = gl[drop].bind(gl);
                    gl[create] = () => { const value = allocate(); live.add(value); allocations++; return value; };
                    gl[drop] = value => {
                        if (value && !live.delete(value)) throw new Error('A GPU resource was released twice');
                        if (value) releases++;
                        release(value);
                    };
                }
                const compile = (type, code) => {
                    const shader = gl.createShader(type);
                    gl.shaderSource(shader, code);
                    gl.compileShader(shader);
                    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader));
                    return shader;
                };
                const vertex = compile(gl.VERTEX_SHADER, '#version 300 es\nlayout(location=0) in vec3 aPos; void main(){gl_Position=vec4(aPos,1.0);}');
                const fragment = compile(gl.FRAGMENT_SHADER, '#version 300 es\nprecision mediump float; uniform vec4 uColor; out vec4 color; void main(){color=uColor;}');
                const program = gl.createProgram();
                gl.attachShader(program, vertex);
                gl.attachShader(program, fragment);
                gl.transformFeedbackVaryings(program, ['gl_Position'], gl.INTERLEAVED_ATTRIBS);
                gl.linkProgram(program);
                if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
                gl.useProgram(program);
                const loc = { aPos: 0, aPrev: 1, aArc: 2, uColor: gl.getUniformLocation(program, 'uColor'), uMode: null, uDash: null, uFrontMin: null };
                const cycles = [];
                const outer = [[-0.8, -0.8, 0], [0.8, -0.8, 0], [0.8, 0.8, 0], [-0.8, 0.8, 0]];
                const hole = new Float32Array([-0.2, -0.2, 0, 0.2, -0.2, 0, 0.2, 0.2, 0, -0.2, 0.2, 0]);
                for (let move = 0; move < 20; move++) {
                    gl.clearColor(0, 0, 0, 0);
                    gl.clearStencil(0);
                    gl.clear(gl.COLOR_BUFFER_BIT | gl.STENCIL_BUFFER_BIT);
                    gl.clearColor(0.125, 0.25, 0.5, 0.75);
                    gl.clearStencil(113);
                    const clearBefore = [Array.from(gl.getParameter(gl.COLOR_CLEAR_VALUE)), gl.getParameter(gl.STENCIL_CLEAR_VALUE)];
                    const vertices = new Float32Array((move % 2 === 0 ? outer : [...outer].reverse()).flat());
                    const ring = uploadGeometry(gl, loc, vertices);
                    const lobe = uploadLobe(gl, loc, hole);
                    const previous = uploadPrevious(gl, loc, ring, vertices);
                    const foreignFeedback = gl.createTransformFeedback(), foreignIndexed = gl.createBuffer(), foreignGeneric = gl.createBuffer();
                    gl.bindTransformFeedback(gl.TRANSFORM_FEEDBACK, foreignFeedback);
                    gl.bindBufferBase(gl.TRANSFORM_FEEDBACK_BUFFER, 0, foreignIndexed);
                    gl.bindBuffer(gl.TRANSFORM_FEEDBACK_BUFFER, foreignGeneric);
                    const options = { key: [move], mode: 1, whole: false, pageVao: null, discVao: null, discCount: 0, color: [255, 0, 0, 255], frontMin: 0 };
                    drawStencilFill(gl, loc, [ring, lobe], options);
                    const firstAllocations = allocations;
                    drawStencilFill(gl, loc, [ring, lobe], options);
                    const cacheReused = allocations === firstAllocations;
                    const feedbackRestored = gl.getParameter(gl.TRANSFORM_FEEDBACK_BINDING) === foreignFeedback
                        && gl.getParameter(gl.TRANSFORM_FEEDBACK_BUFFER_BINDING) === foreignGeneric
                        && gl.getIndexedParameter(gl.TRANSFORM_FEEDBACK_BUFFER_BINDING, 0) === foreignIndexed && !gl.isEnabled(gl.RASTERIZER_DISCARD);
                    gl.bindTransformFeedback(gl.TRANSFORM_FEEDBACK, null); gl.bindBuffer(gl.TRANSFORM_FEEDBACK_BUFFER, null);
                    gl.deleteTransformFeedback(foreignFeedback); gl.deleteBuffer(foreignIndexed); gl.deleteBuffer(foreignGeneric);
                    const filled = new Uint8Array(4);
                    const empty = new Uint8Array(4);
                    gl.readPixels(45, 96, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, filled);
                    gl.readPixels(96, 96, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, empty);
                    const active = { buffers: buffers.size, arrays: arrays.size };
                    endPrevious(gl, loc, ring, previous);
                    ring.clipScratch = [lobe];
                    disposeGeometry(gl, ring);
                    const clearAfter = [Array.from(gl.getParameter(gl.COLOR_CLEAR_VALUE)), gl.getParameter(gl.STENCIL_CLEAR_VALUE)];
                    cycles.push({ cacheReused, feedbackRestored, active, remaining: { buffers: buffers.size, arrays: arrays.size }, filled: Array.from(filled), empty: Array.from(empty), clearBefore, clearAfter, error: gl.getError() });
                }
                gl.deleteProgram(program);
                gl.deleteShader(vertex);
                gl.deleteShader(fragment);
                return { cycles, allocations, releases };
            });
            assert.equal(result.cycles.length, 20, 'Every back-and-forth move is exercised');
            for (const [index, cycle] of result.cycles.entries()) {
                assert.equal(cycle.cacheReused, true, `Move ${index} reuses an identical projected mesh without allocations`);
                assert.equal(cycle.feedbackRestored, true, `Move ${index} preserves foreign transform-feedback bindings and discard state`);
                assert.deepEqual(cycle.active, { buffers: 5, arrays: 3 }, `Move ${index} holds only its declared resources`);
                assert.deepEqual(cycle.remaining, { buffers: 0, arrays: 0 }, `Move ${index} leaves no retained buffer or vertex array`);
                assert.deepEqual(cycle.filled, [255, 0, 0, 255], `Move ${index} fills the ring`);
                assert.deepEqual(cycle.empty, [0, 0, 0, 0], `Move ${index} preserves the hole`);
                assert.deepEqual(cycle.clearAfter, cycle.clearBefore, `Move ${index} preserves the foreign clear values`);
                assert.equal(cycle.error, 0, `Move ${index} causes no WebGL error`);
            }
            assert.equal(result.releases, result.allocations, 'Every allocated buffer and vertex array is released');
            assert.deepEqual(errors, [], 'The browser reports no rendering exception');
        } finally {
            await browser.close();
        }
    });
}
