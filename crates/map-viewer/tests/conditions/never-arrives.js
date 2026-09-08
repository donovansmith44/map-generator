// A map that never loaded.
//
// Once the page has drawn its first scene, every later /api/scene
// request is answered with a refusal. gpuSyncInner has already
// advanced `gpu.sceneKey` to the scene it was about to fetch and
// returns silently on `!res.ok` — so the page settles completely,
// fully resident, with no request in flight and NO ERROR ANYWHERE,
// showing the scene it happened to have. Nothing the gate's settle
// predicate reads is false. The picture is simply not this stop's
// picture, and at the first stop it is not any stop's picture.
//
// A refusal rather than a hang, deliberately: a hung fetch leaves
// `gpu.syncing` true, which the settle predicate already catches, and
// a law about a map that never arrived would then be quietly testing a
// map that never stopped moving instead.
(() => {
  const realFetch = window.fetch;
  let armed = false;
  window.fetch = function (input) {
    const url = typeof input === 'string' ? input : (input && input.url) || '';
    if (armed && url.includes('/api/scene')) {
      return Promise.resolve(new Response('', { status: 503, statusText: 'condition: never-arrives' }));
    }
    return realFetch.apply(this, arguments);
  };
  const arm = () => {
    if (typeof gpu === 'undefined' || !gpu.scene) { requestAnimationFrame(arm); return; }
    armed = true;
  };
  requestAnimationFrame(arm);
})();
