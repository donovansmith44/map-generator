// A map that draws nothing.
//
// Everything arrives: the scene is the scene this camera asked for,
// every wanted resource is resident, no label is mid-fade. The page is
// simply painted empty — the draw runs and its result is wiped. That
// is what makes this the right partner for never-arrives.js: the two
// look identical through a probe (all-black), and differ entirely in
// what the page has. A gate that cannot tell them apart is a gate that
// blessed a black baseline once already.
(() => {
  const arm = () => {
    if (typeof gpuDraw !== 'function' || typeof gpu === 'undefined' || !gpu.gl) {
      requestAnimationFrame(arm);
      return;
    }
    const real = gpuDraw;
    gpuDraw = function () {
      real.apply(this, arguments);
      const gl = gpu.gl;
      gl.disable(gl.SCISSOR_TEST);
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
    };
  };
  requestAnimationFrame(arm);
})();
