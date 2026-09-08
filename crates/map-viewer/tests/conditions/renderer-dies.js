// A renderer that has died.
//
// A GENUINE context loss, taken through WebGL's own extension, once
// the page has a renderer to lose — not a flag flipped to make the
// gate's predicate say what the law wants to hear. That distinction is
// the whole finding: `gpu.enabled` is only ever assigned by
// gpuSetEnabled and `gpu.canvas` survives the loss, so after this the
// page's own flags still claim a live renderer. A gate that believes
// them keeps sampling and calls the result drift.
(() => {
  const arm = () => {
    if (typeof gpu === 'undefined' || !gpu.gl
        || typeof state === 'undefined' || !state.stops || !state.stops.length) {
      requestAnimationFrame(arm);
      return;
    }
    const ext = gpu.gl.getExtension('WEBGL_lose_context');
    if (!ext) throw new Error('condition renderer-dies: WEBGL_lose_context is unavailable');
    ext.loseContext();
  };
  requestAnimationFrame(arm);
})();
