# Declared conditions for the golden gate

The gate (`../golden.js`) judges a map. Its own laws
(`contracts/golden-gate/golden-gate.feature`) judge the gate, and they need it
to run under conditions that do not arise on their own: a picture that changed,
a renderer that died, a page that never settles, a scene that never arrived, a
run that offers fewer stops than the baseline holds.

There is **one** mechanism for all of them, and the gate knows about none of
them individually:

```
node golden.js --check --condition conditions/blank.js --condition-arg '{"..."}'
```

A condition is a plain script installed in the page's own world *before* any of
the page's scripts run — the position a browser extension's content script
occupies. It may therefore do anything a script in that page could do, which is
why one flag covers eight situations rather than eight flags covering one each.
Its declared argument arrives as the global `CONDITION`.

Two facts a condition can rely on:

* `GOLDEN` — the gate's own constants (`cams`, `grid`, `tol`), published into
  every run whether or not a condition is installed. A condition that needed to
  locate probe 7 of the levant camera would otherwise restate the gate's grid,
  and a restated constant is a constant that drifts.
* the page's own top-level bindings (`gpu`, `state`, `glyphs`, `labelState`,
  `gpuDraw`). A condition runs before they exist, so anything that needs them
  waits for them on the page's own animation frames. That waiting is the
  condition's business, not the gate's.

A run under a condition says so in its banner and in its `GATE VERDICT` line.
Nothing here can make the gate report a *pass* it would not otherwise report:
these scripts change the map the gate looks at, never the gate's judgement of
it. A bless under a condition may not write the default baseline at all — the
gate refuses that outright.
