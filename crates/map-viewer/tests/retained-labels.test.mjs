import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const { retainedGlyphEntry } = createRequire(import.meta.url)('../src/retained-labels.js');
const voice = { family: 'sans-serif', weight: 400, italic: false, tracking_em: 0 };
const color = [0, 0, 0, 255], halo = [255, 255, 255, 255];
function atlas() {
    let clears = 0;
    const ctx = { measureText: text => ({ width: text.length * 40 }), strokeText() {}, fillText() {}, clearRect() { clears++; } };
    return { map: new Map(), x: 0, y: 0, rowH: 0, dirty: false, ctx, clears: () => clears };
}
globalThis.window = { devicePixelRatio: 1 };
test('generated capacity overflow preserves all earlier glyph entries', () => {
    const glyphs = atlas(), entries = [];
    for (let index = 0; index < 1000; index++) {
        const entry = retainedGlyphEntry(glyphs, 'label-' + index, voice, 100, color, halo);
        if (entry !== null) entries.push(entry);
    }
    assert.equal(glyphs.clears(), 0, 'Overflow does not clear previously prepared glyph pixels.');
    assert.ok(entries.length < 1000, 'The generated labels exceed the retained atlas capacity.');
    assert.equal(glyphs.map.size, entries.length, 'Every prepared glyph remains in the atlas.');
    assert.equal(retainedGlyphEntry(glyphs, 'label-0', voice, 100, color, halo), entries[0], 'An earlier label keeps its original UV entry.');
});
test('generated oversize labels preserve the accepted placement and pixels', () => {
    for (const [text, size] of [['wide'.repeat(100), 20], ['x', 2000]]) {
        const glyphs = atlas();
        retainedGlyphEntry(glyphs, 'first', voice, 20, color, halo);
        const before = [glyphs.x, glyphs.y, glyphs.rowH, glyphs.map.size];
        assert.equal(retainedGlyphEntry(glyphs, text, voice, size, color, halo), null, 'An oversize glyph is omitted.');
        assert.equal(glyphs.clears(), 0, 'An oversize glyph does not clear accepted pixels.');
        assert.deepEqual([glyphs.x, glyphs.y, glyphs.rowH, glyphs.map.size], before, 'An omitted glyph preserves the accepted placement.');
    }
});
