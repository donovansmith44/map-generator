// A change to the picture, at exactly one probe of exactly one camera.
//
//   CONDITION = { "probe": 0..24, "camera": "levant", "by": "far"|"under" }
//
// "far"   — the block is repainted to the far end of its own range, so
//           the change is at least 127 per channel: unmissable, and
//           independent of what was there.
// "under" — the block moves toward the middle of the range by HALF the
//           gate's own declared tolerance, so the change is real, is
//           never clamped away, and is strictly smaller than the
//           tolerance whatever that tolerance is. Read off `GOLDEN.tol`,
//           which the gate publishes precisely so a condition need not
//           restate its constants: a hardcoded 12 was this file's own
//           README violated ("a restated constant is a constant that
//           drifts"), and it would have made this law spuriously red the
//           day TOL dropped below it.
//
// The observation is made THE WAY THE GATE MAKES IT: the block is read
// back through a 2D snapshot, exactly as `sampleOnce` reads it, so the
// target colour is stated in the units the gate compares in and the
// change it will measure is the change declared here — not a value
// that has to survive premultiplied alpha and compositing on its way
// to being compared. The write is an opaque scissor clear over the
// same nine pixels, which is why the block's mean afterwards IS the
// target.
//
// The probe's location and the camera's zoom come from `GOLDEN`, the
// gate's own published constants; nothing here restates the grid.
(() => {
  const C = typeof CONDITION !== 'undefined' ? CONDITION : null;
  if (!C) throw new Error('condition repaint: needs --condition-arg {"probe":..,"camera":..,"by":..}');
  // strictly under the gate's tolerance, derived from it, never restated
  const UNDER_TOLERANCE = Math.max(1, Math.floor(GOLDEN.tol / 2));

  const arm = () => {
    if (typeof gpuDraw !== 'function' || typeof gpu === 'undefined' || !gpu.gl
        || typeof state === 'undefined') {
      requestAnimationFrame(arm);
      return;
    }
    const spec = GOLDEN.cams.find(c => c[0] === C.camera);
    if (!spec) throw new Error('condition repaint: no camera named ' + C.camera);
    const cell = GOLDEN.grid[C.probe];
    if (!cell) throw new Error('condition repaint: no probe ' + C.probe);
    const scratch = document.createElement('canvas');
    scratch.width = 3; scratch.height = 3;
    const s2d = scratch.getContext('2d', { willReadFrequently: true });

    const real = gpuDraw;
    gpuDraw = function () {
      real.apply(this, arguments);
      const cam = state.cameraNow && state.cameraNow();
      // this camera only: the law says the drift lands at one probe of
      // ONE camera and nowhere else, so the other camera must be left
      // exactly as it was
      if (!cam || cam.zoom === null || Math.abs(cam.zoom - spec[3]) > 1e-9) return;

      const gl = gpu.gl, canvas = gpu.canvas;
      const x = Math.max(1, Math.round(canvas.width * cell[0]));
      const y = Math.max(1, Math.round(canvas.height * cell[1]));

      // what the gate would see here, in the gate's own units
      s2d.clearRect(0, 0, 3, 3);
      s2d.drawImage(canvas, x - 1, y - 1, 3, 3, 0, 0, 3, 3);
      const d = s2d.getImageData(0, 0, 3, 3).data;
      const mean = [0, 1, 2].map(k => {
        let t = 0;
        for (let p = 0; p < 9; p++) t += d[p * 4 + k];
        return Math.round(t / 9);
      });

      // the target, per channel
      const target = mean.map(m => C.by === 'far'
        ? (m < 128 ? 255 : 0)
        // toward the middle, so the shift is never clamped and is
        // therefore exactly UNDER_TOLERANCE
        : (m < 128 ? m + UNDER_TOLERANCE : m - UNDER_TOLERANCE));

      gl.enable(gl.SCISSOR_TEST);
      // GL rows count from the bottom; the gate reads rows y-1..y+1
      // from the top
      gl.scissor(x - 1, canvas.height - y - 2, 3, 3);
      gl.clearColor(target[0] / 255, target[1] / 255, target[2] / 255, 1);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.disable(gl.SCISSOR_TEST);
    };
  };
  requestAnimationFrame(arm);
})();
