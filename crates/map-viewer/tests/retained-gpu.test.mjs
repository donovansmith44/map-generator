import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const require = createRequire(import.meta.url);
const renderer = require('../src/retained-gpu.js');
const loc = { aPos: 0, aPrev: 1, aArc: 2, uMode: 3, uDash: 4, uColor: 5, uFrontMin: 6 };
const vertices = () => Float32Array.from([1, 0, 0, 0, 1, 0, 0, 0, 1]);

function context(failAt = 0) {
  let issued = 0, uploads = 0;
  const buffers = new Set(), vaos = new Set(), pointers = new Map(), draws = [], uniforms = [];
  const allocation = pool => { const handle = ++issued; pool.add(handle); return handle; };
  const gl = {
    ARRAY_BUFFER: 1, STATIC_DRAW: 2, FLOAT: 3, STENCIL_TEST: 4,
    ALWAYS: 5, KEEP: 6, INVERT: 7, NOTEQUAL: 8, ZERO: 9, TRIANGLE_FAN: 10, TRIANGLES: 11, STENCIL_BUFFER_BIT: 12, STENCIL: 13,
    buffers, vaos, pointers, draws, uniforms, bound: null, array: null, mask: null, stencil: false, cleared: null,
    createBuffer() { return allocation(buffers); }, createVertexArray() { return allocation(vaos); },
    deleteBuffer(handle) { if (handle !== null) assert.equal(buffers.delete(handle), true, 'each buffer is deleted exactly once'); },
    deleteVertexArray(handle) { if (handle !== null) assert.equal(vaos.delete(handle), true, 'each VAO is deleted exactly once'); },
    bindBuffer(_target, handle) { this.array = handle; }, bindVertexArray(handle) { this.bound = handle; },
    bufferData() { if (++uploads === failAt) throw Error('upload failed'); }, enableVertexAttribArray() {},
    vertexAttribPointer(attribute) { pointers.set(attribute, this.array); },
    uniform1i(location, value) { uniforms.push([location, value]); }, uniform1f(location, value) { uniforms.push([location, value]); },
    uniform2f() {}, uniform4f(...args) { this.colour = args.slice(1); },
    enable() { this.stencil = true; }, disable() { this.stencil = false; },
    colorMask(...mask) { this.mask = mask; }, stencilFunc() {}, stencilOp() {},
    drawArrays(mode, _first, count) { draws.push([mode, count]); },
    clearStencil(value) { this.clearValue = value; }, clear(bits) { this.cleared = bits; },
    clearBufferiv(buffer, index, value) { this.bufferClear = { buffer, index, value: Array.from(value) }; },
  };
  return gl;
}

for (const direction of ['forward', 'reverse']) {
  test(`twenty ${direction} changes release every geometry previous and clipped resource`, () => {
    const gl = context();
    for (let cycle = 0; cycle < 20; cycle++) {
      const points = vertices();
      if (direction === 'reverse') points.reverse();
      const shape = renderer.uploadGeometry(gl, loc, points);
      const previous = renderer.uploadPrevious(gl, loc, shape, points);
      const lobe = renderer.uploadLobe(gl, loc, points);
      shape.clipScratch = [lobe]; shape.clipKey = String(cycle);
      assert.equal(gl.buffers.size, 4, 'a geometry, previous positions and one clip own four buffers');
      assert.equal(gl.vaos.size, 2, 'the geometry and its clip own two VAOs');
      renderer.endPrevious(gl, loc, shape, previous);
      assert.equal(gl.pointers.get(loc.aPrev), shape.buf, 'ending a morph restores the original position attribute');
      renderer.disposeGeometry(gl, shape);
      assert.equal(gl.buffers.size, 0, 'settling releases every per-change buffer');
      assert.equal(gl.vaos.size, 0, 'settling releases every per-change VAO');
      assert.equal(gl.bound, null, 'the resource lifecycle leaves no VAO bound');
      assert.equal(shape.clipScratch, null, 'disposal drops the clipped draw list');
      assert.equal(shape.clipKey, null, 'disposal drops the clipped camera key');
    }
  });
}

for (const failAt of [1, 2]) {
  test(`geometry upload failure ${failAt} releases partial allocations`, () => {
    const gl = context(failAt);
    assert.throws(() => renderer.uploadGeometry(gl, loc, vertices()), /upload failed/, 'the original upload failure reaches its caller');
    assert.equal(gl.buffers.size, 0, 'a failed geometry upload retains no buffer');
    assert.equal(gl.vaos.size, 0, 'a failed geometry upload retains no VAO');
    assert.equal(gl.bound, null, 'a failed upload leaves no VAO bound');
  });
}

test('previous-position upload failure releases its buffer and restores the original attribute', () => {
  const gl = context(3);
  const shape = renderer.uploadGeometry(gl, loc, vertices());
  assert.throws(() => renderer.uploadPrevious(gl, loc, shape, vertices()), /upload failed/, 'the previous-position failure reaches its caller');
  assert.equal(gl.buffers.size, 2, 'only the original geometry buffers remain');
  assert.equal(gl.pointers.get(loc.aPrev), shape.buf, 'failed previous positions restore the original attribute');
  renderer.disposeGeometry(gl, shape);
  assert.equal(gl.buffers.size, 0, 'disposing the surviving geometry releases its buffers');
  assert.equal(gl.vaos.size, 0, 'disposing the surviving geometry releases its VAO');
});

test('clip upload failure releases its partial VAO and buffer', () => {
  const gl = context(1);
  assert.throws(() => renderer.uploadLobe(gl, loc, vertices()), /upload failed/, 'the clipping upload failure reaches its caller');
  assert.equal(gl.buffers.size, 0, 'failed clipping retains no buffer');
  assert.equal(gl.vaos.size, 0, 'failed clipping retains no VAO');
});

for (const target of ['createBuffer', 'createVertexArray']) {
  test(`a refused ${target} allocation releases every earlier allocation`, () => {
    const gl = context(); gl[target] = () => null;
    assert.throws(() => renderer.uploadGeometry(gl, loc, vertices()), /allocation failed/, 'allocation refusal reaches the caller');
    assert.equal(gl.buffers.size, 0, 'allocation refusal leaves no buffer');
    assert.equal(gl.vaos.size, 0, 'allocation refusal leaves no VAO');
  });
}

for (const mode of [0, 1]) {
  test(`whole region parity in projection ${mode} preserves its two passes and restores state`, () => {
    const gl = context();
    renderer.drawStencilFill(gl, loc, [{ vao: 10, count: 4 }, { vao: 11, count: 3 }], { mode, whole: true, pageVao: 12, discVao: 13, discCount: 98, color: [128, 64, 32, 192], frontMin: 0.01 });
    assert.equal(gl.draws.length, 6, 'each pass draws the whole-page sentinel and both rings');
    assert.deepEqual(gl.draws[0], mode === 0 ? [gl.TRIANGLE_FAN, 98] : [gl.TRIANGLES, 3], 'the original projection selects its original sentinel');
    assert.deepEqual(gl.mask, [true, true, true, true], 'the parity passes restore colour writes');
    assert.equal(gl.stencil, false, 'the parity passes disable their stencil test');
    assert.deepEqual(gl.uniforms.at(-1), [loc.uFrontMin, 0.01], 'the parity passes restore the stroke front threshold');
    assert.deepEqual(gl.colour, [128 / 255, 64 / 255, 32 / 255, 192 / 255], 'the fill keeps its original colour and alpha');
  });
}

test('failed parity drawing clears incomplete parity and restores drawing state', () => {
  const gl = context(); gl.drawArrays = () => { throw Error('draw failed'); };
  gl.clearStencil(113);
  assert.throws(() => renderer.drawStencilFill(gl, loc, [{ vao: 10, count: 3 }], { mode: 0, whole: false, color: [1, 2, 3, 4], frontMin: 0.2 }), /draw failed/, 'the draw failure reaches its caller');
  assert.deepEqual(gl.bufferClear, { buffer: gl.STENCIL, index: 0, value: [0] }, 'incomplete parity is cleared back to zero after a failed draw');
  assert.equal(gl.clearValue, 113, 'failure cleanup preserves the caller clear value');
  assert.deepEqual(gl.mask, [true, true, true, true], 'failed parity restores colour writes');
  assert.equal(gl.stencil, false, 'failed parity disables its stencil test');
  assert.deepEqual(gl.uniforms.at(-1), [loc.uFrontMin, 0.2], 'failed parity restores the stroke front threshold');
});

test('the original viewer loads and calls the extracted single implementation', () => {
  const page = readFileSync(new URL('../src/page.html', import.meta.url), 'utf8');
  const host = readFileSync(new URL('../src/lib.rs', import.meta.url), 'utf8');
  assert.match(page, /<script src="\/retained-gpu.js"><\/script>/, 'the viewer loads its source-owned resource module');
  assert.match(host, /"\/retained-gpu.js" =>/, 'the original viewer serves the extracted module');
  assert.match(page, /uploadGeometry\(gl, gpu.loc, verts\)/, 'the original geometry acquisition uses the extracted upload');
  assert.match(page, /drawStencilFill\(gl, loc, draws,/, 'the original fill uses the extracted parity path');
  assert.doesNotMatch(page, /gl\.stencilFunc\(gl\.NOTEQUAL/, 'the original viewer retains no second odd-parity fill implementation');
});

const limb = require('../src/limb.js');
const crossingVertices = () => Float32Array.from([0.6, -0.8, 0, 0.6, 0, 0.8, -0.6, 0.8, 0, -0.6, 0, -0.8]);

test('visible fill cache reuses exact direction, replaces tiny changes and owns disposal', () => {
  const gl = context(), ring = renderer.uploadGeometry(gl, loc, crossingVertices());
  const C = [1, 0, 0];
  const first = renderer.visibleFillDraws(gl, loc, ring, C);
  assert.ok(first.length > 0);
  assert.notEqual(first[0], ring);
  assert.equal(renderer.visibleFillDraws(gl, loc, ring, [...C]), first);
  assert.deepEqual(renderer.visibleFillDraws(gl, loc, ring, null), [ring]);
  assert.equal(renderer.visibleFillDraws(gl, loc, ring, C), first, 'flat mode borrows the original without evicting bounded scratch');
  const oldVaos = first.map(piece => piece.vao);
  const next = renderer.visibleFillDraws(gl, loc, ring, [Math.cos(1e-7), Math.sin(1e-7), 0]);
  assert.notEqual(next, first, 'even a sub-rounding direction change reclips');
  for (const vao of oldVaos) assert.equal(gl.vaos.has(vao), false);
  renderer.disposeGeometry(gl, ring);
  renderer.disposeGeometry(gl, ring);
  assert.equal(gl.buffers.size + gl.vaos.size, 0);
});

test('visible fills clip only the selected shared-buffer range', () => {
  const gl = context(), values = crossingVertices();
  const backing = renderer.uploadGeometry(gl, loc, Float32Array.from([1, 0, 0, ...values, -1, 0, 0]));
  const range = { vao: backing.vao, verts: backing.verts, first: 1, count: values.length / 3 };
  const C = [1, 0, 0], { E, N } = limb.limbBasis(C);
  const expected = limb.clipRingFront(values, C, E, N);
  const draws = renderer.visibleFillDraws(gl, loc, range, C);
  assert.deepEqual(draws.map(draw => Array.from(draw.verts)), expected.map(points => Array.from(points)));
  renderer.dropClipScratch(gl, range);
  assert.equal(gl.vaos.has(backing.vao), true, 'range cleanup leaves the shared owner alive');
  renderer.disposeGeometry(gl, backing);
  assert.equal(gl.buffers.size + gl.vaos.size, 0);
});

test('visible fills preserve cap fast paths and send exact tangency through clipping', () => {
  const gl = context(), ring = renderer.uploadGeometry(gl, loc, vertices());
  ring.cap = { ax: 1, ay: 0, az: 0, cosR: 1 };
  const original = limb.clipRingFront;
  let clips = 0;
  limb.clipRingFront = (...args) => { clips++; return original(...args); };
  try {
    assert.deepEqual(renderer.visibleFillDraws(gl, loc, ring, [1, 0, 0]), [ring]);
    assert.deepEqual(renderer.visibleFillDraws(gl, loc, ring, [-1, 0, 0]), []);
    assert.equal(clips, 0);
    renderer.visibleFillDraws(gl, loc, ring, [0, 1, 0]);
    assert.equal(clips, 1, 'a tangent cap cannot decide the visible ring');
    delete ring.cap;
    renderer.dropClipScratch(gl, ring);
    assert.deepEqual(renderer.visibleFillDraws(gl, loc, ring, [1, 0, 0]), [ring]);
    assert.deepEqual(renderer.visibleFillDraws(gl, loc, ring, [-1, -1, -1]), []);
  } finally {
    limb.clipRingFront = original;
    renderer.disposeGeometry(gl, ring);
  }
});

test('partial multi-lobe upload failure frees completed lobes and leaves cache retryable', () => {
  const gl = context(4), ring = renderer.uploadGeometry(gl, loc, crossingVertices());
  const original = limb.clipRingFront;
  limb.clipRingFront = () => [vertices(), vertices()];
  try {
    assert.throws(() => renderer.visibleFillDraws(gl, loc, ring, [1, 0, 0]), /upload failed/);
    assert.equal(gl.buffers.size, 2);
    assert.equal(gl.vaos.size, 1);
    assert.equal(ring.clipKey, null);
    assert.equal(ring.clipScratch, null);
    assert.equal(renderer.visibleFillDraws(gl, loc, ring, [1, 0, 0]).length, 2);
  } finally {
    limb.clipRingFront = original;
    renderer.disposeGeometry(gl, ring);
  }
  assert.equal(gl.buffers.size + gl.vaos.size, 0);
});

test('optional cap decisions match the uncapped fixtures and replacement contexts own distinct lobes', () => {
  const fixtures = JSON.parse(readFileSync(new URL('./fixtures/limb.json', import.meta.url), 'utf8'));
  for (const fixture of fixtures.cases) {
    const points = Float32Array.from(fixture.ring);
    const firstGl = context(), secondGl = context();
    const ring = renderer.uploadGeometry(firstGl, loc, points);
    const replacement = renderer.uploadGeometry(secondGl, { ...loc, aPos: 4 }, points);
    ring.cap = limb.ringCap(points);
    const capped = renderer.visibleFillDraws(firstGl, loc, ring, fixture.c);
    const uncapped = renderer.visibleFillDraws(secondGl, { ...loc, aPos: 4 }, replacement, fixture.c);
    assert.deepEqual(capped.map(draw => Array.from(draw.verts)), uncapped.map(draw => Array.from(draw.verts)), fixture.name);
    assert.notEqual(ring.clipScratch, replacement.clipScratch, 'recreated resources never share context-owned scratch');
    renderer.disposeGeometry(firstGl, ring);
    renderer.disposeGeometry(secondGl, replacement);
    assert.equal(firstGl.buffers.size + firstGl.vaos.size + secondGl.buffers.size + secondGl.vaos.size, 0);
  }
});
