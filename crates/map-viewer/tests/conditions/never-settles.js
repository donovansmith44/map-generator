// A map that never stops moving.
//
// The glyph atlas is declared permanently dirty, so the page's own
// upload path re-uploads it every frame and never finishes. This is a
// real page state (an atlas whose upload never completes), not a flag
// invented for the gate: `glyphs.dirty` is the page's own field and
// the gate's settle predicate reads it because the page's renderer
// does.
//
// A getter rather than a re-assignment on every frame: the page clears
// the flag as part of the same frame it uploads in, so a writer would
// race the reader and the condition would hold only most of the time.
// A property that cannot be false is not a race.
(() => {
  const arm = () => {
    if (typeof glyphs === 'undefined' || !glyphs) { requestAnimationFrame(arm); return; }
    Object.defineProperty(glyphs, 'dirty', { get: () => true, set: () => {}, configurable: true });
  };
  requestAnimationFrame(arm);
})();
