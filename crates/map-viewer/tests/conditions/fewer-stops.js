// A map that offers fewer stops than the baseline holds.
//
// The timeline arrives and then loses the stops named in CONDITION.drop.
// Nothing else changes: the stops that remain render exactly as they
// always did and hold against the baseline, which is the point — the
// run is clean in every respect except that it never looked at part of
// what it was asked about.
//
//   CONDITION = { "drop": [-1405, ...] }   years to remove
(() => {
  const drop = (typeof CONDITION !== 'undefined' && CONDITION && CONDITION.drop) || [];
  const arm = () => {
    if (typeof state === 'undefined' || !Array.isArray(state.stops) || !state.stops.length) {
      requestAnimationFrame(arm);
      return;
    }
    state.stops = state.stops.filter(y => !drop.includes(y));
  };
  requestAnimationFrame(arm);
})();
