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
    ALWAYS: 5, KEEP: 6, INVERT: 7, NOTEQUAL: 8, ZERO: 9, TRIANGLE_FAN: 10, TRIANGLES: 11, STENCIL_BUFFER_BIT: 12,
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
  assert.throws(() => renderer.drawStencilFill(gl, loc, [{ vao: 10, count: 3 }], { mode: 0, whole: false, color: [1, 2, 3, 4], frontMin: 0.2 }), /draw failed/, 'the draw failure reaches its caller');
  assert.equal(gl.cleared, gl.STENCIL_BUFFER_BIT, 'incomplete parity is cleared after a failed draw');
  assert.equal(gl.clearValue, 0, 'failed parity is cleared back to its initial zero state');
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
