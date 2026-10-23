import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
const require = createRequire(import.meta.url);
const { chromium } = require(require.resolve('playwright-core', { paths: [process.cwd()] }));
const source = fileURLToPath(new URL('../src/retained-gpu.js', import.meta.url));

test('shared GPU spherical interpolation preserves canonical quarter angles and degenerate endpoints', async () => {
  // Arrange
  const browser = await chromium.launch({ executablePath: process.env.PROOF_CHROMIUM, args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  try {
    const page = await browser.newPage();
    await page.setContent('<canvas></canvas>');
    await page.addScriptTag({ path: source });
    // Act
    const actual = await page.evaluate(() => {
      const gl = document.querySelector('canvas').getContext('webgl2');
      const compile = (kind, source) => {
        const shader = gl.createShader(kind);
        gl.shaderSource(shader, source); gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader));
        return shader;
      };
      const program = gl.createProgram();
      gl.attachShader(program, compile(gl.VERTEX_SHADER, `#version 300 es
precision highp float;
uniform vec3 from; uniform vec3 to; uniform float fraction;
out vec3 result;
${sphericalInterpolationSource()}
void main() { result = sphericalInterpolate(from, to, fraction); gl_Position = vec4(0,0,0,1); }`));
      gl.attachShader(program, compile(gl.FRAGMENT_SHADER, `#version 300 es
precision highp float; out vec4 color; void main() { color = vec4(1); }`));
      gl.transformFeedbackVaryings(program, ['result'], gl.INTERLEAVED_ATTRIBS); gl.linkProgram(program);
      if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
      gl.useProgram(program);
      const buffer = gl.createBuffer(); gl.bindBuffer(gl.TRANSFORM_FEEDBACK_BUFFER, buffer); gl.bufferData(gl.TRANSFORM_FEEDBACK_BUFFER, 12, gl.STREAM_READ);
      gl.bindBufferBase(gl.TRANSFORM_FEEDBACK_BUFFER, 0, buffer); gl.enable(gl.RASTERIZER_DISCARD);
      const sample = (from, to, fraction) => {
        gl.uniform3fv(gl.getUniformLocation(program, 'from'), from); gl.uniform3fv(gl.getUniformLocation(program, 'to'), to); gl.uniform1f(gl.getUniformLocation(program, 'fraction'), fraction);
        gl.beginTransformFeedback(gl.POINTS); gl.drawArrays(gl.POINTS, 0, 1); gl.endTransformFeedback();
        const result = new Float32Array(3); gl.getBufferSubData(gl.TRANSFORM_FEEDBACK_BUFFER, 0, result); return Array.from(result);
      };
      return { start: sample([1,0,0],[-0.5,Math.sqrt(3)/2,0],0), end: sample([1,0,0],[-0.5,Math.sqrt(3)/2,0],1),
        quarter: sample([1,0,0],[-0.5,Math.sqrt(3)/2,0],0.25), threeQuarters: sample([1,0,0],[-0.5,Math.sqrt(3)/2,0],0.75),
        coincident: sample([1,0,0],[1,0,0],0.25), nearCoincident: sample([1,0,0],[Math.cos(1e-4),Math.sin(1e-4),0],0.25),
        nearAntipodal: sample([1,0,0],[-Math.cos(1e-4),Math.sin(1e-4),0],0.5),
        nearAntipodalReverse: sample([-Math.cos(1e-4),Math.sin(1e-4),0],[1,0,0],0.5), error: gl.getError() };
    });
    // Assert
    assert.deepEqual(actual.start, [1,0,0], 'The zero fraction is the supplied starting point.');
    assert.ok(Math.abs(actual.end[0] + 0.5) < 2e-6 && Math.abs(actual.end[1] - Math.sqrt(3)/2) < 2e-6, 'The unit fraction is the supplied ending point.');
    assert.ok(Math.abs(actual.quarter[0] - Math.sqrt(3)/2) < 2e-6 && Math.abs(actual.quarter[1] - 0.5) < 2e-6, 'A quarter of 120 degrees is 30 degrees.');
    assert.ok(Math.abs(actual.threeQuarters[0]) < 2e-6 && Math.abs(actual.threeQuarters[1] - 1) < 2e-6, 'Three quarters of 120 degrees is 90 degrees.');
    assert.deepEqual(actual.coincident, [1,0,0], 'Coincident canonical points remain the same point.');
    assert.ok(Math.abs(actual.nearCoincident[1] - 2.5e-5) < 1e-7, 'Float dot rounding does not erase a small valid angle.');
    assert.ok(Math.abs(actual.nearAntipodal[0] - 5e-5) < 2e-6 && Math.abs(actual.nearAntipodal[1] - 1) < 2e-6, 'A valid near-antipodal path retains its supplied direction.');
    assert.ok(actual.nearAntipodal.every((value, index) => Math.abs(value - actual.nearAntipodalReverse[index]) < 2e-6), 'A valid near-antipodal path is reversible.');
    assert.equal(actual.error, 0, 'GPU interpolation produces no WebGL error.');
  } finally { await browser.close(); }
});
