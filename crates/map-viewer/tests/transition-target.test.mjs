import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);

test('full transition target refuses failed shader creation without retaining resources', () => {
    const target = require('../src/transition-target.js');
    let deleted = 0;
    const gl = {
        VERTEX_SHADER: 1,
        createShader() { return null; },
        deleteShader() { deleted++; },
    };
    const input = { shaderData: { vertexShaderPrelude: '', define: '', variantName: 'failed' } };
    assert.throws(() => target.transitionTarget(gl, () => input, () => [null, null, null, null], () => [null, null], () => {}, reference => reference), /shader allocation/i,
        'The target refuses the failed shader allocation.');
    assert.equal(deleted, 0, 'No null resource is treated as an allocation.');
});
