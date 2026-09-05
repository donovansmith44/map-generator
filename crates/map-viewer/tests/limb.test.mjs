// The JS side of the cross-language limb contract. The fixture file
// is blessed by the Rust implementation
// (cargo test -p map-encoders bless_limb_fixtures -- --ignored);
// this test holds crates/map-viewer/src/limb.js — the line-for-line
// port compiled into the viewer page — to the same answers.
//
// Run: node --test crates/map-viewer/tests/

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const limb = require("../src/limb.js");
const fixtures = JSON.parse(
  readFileSync(new URL("./fixtures/limb.json", import.meta.url), "utf8"),
);

// Clip points come back as Float32Array (the GPU's own precision);
// the Rust side computed in f64. A hair over f32 epsilon.
const CLIP_TOL = 2e-6;

for (const cs of fixtures.cases) {
  test(`insideRing agrees with Rust: ${cs.name}`, () => {
    for (const probe of cs.probes) {
      assert.equal(
        limb.insideRing(probe.p, cs.ring),
        probe.inside,
        `probe ${JSON.stringify(probe.p)}`,
      );
    }
  });

  test(`clipRingFront agrees with Rust: ${cs.name}`, () => {
    const { E, N } = limb.limbBasis(cs.c);
    const v = Float64Array.from(cs.ring);
    const got = limb.clipRingFront(v, cs.c, E, N);
    if (cs.clip.kind === "same") {
      assert.equal(got, v, "wholly front hands back the same object");
    } else if (cs.clip.kind === "none") {
      assert.equal(got, null, "wholly hidden clips to nothing");
    } else {
      assert.ok(Array.isArray(got), "straddler clips to an array of loops");
      assert.equal(got.length, cs.clip.loops.length, "same lobe count as Rust");
      for (let l = 0; l < got.length; l++) {
        const want = cs.clip.loops[l];
        assert.equal(got[l].length, want.length, `loop ${l} point count`);
        for (let i = 0; i < want.length; i++) {
          assert.ok(
            Math.abs(got[l][i] - want[i]) < CLIP_TOL,
            `loop ${l} coord ${i}: js ${got[l][i]} vs rust ${want[i]}`,
          );
        }
      }
    }
  });
}

// The traversal-invariance law holds in the port too: reversing a
// ring never changes membership.
test("insideRing is traversal-invariant", () => {
  for (const cs of fixtures.cases) {
    const n = cs.ring.length / 3;
    const rev = new Float64Array(cs.ring.length);
    for (let i = 0; i < n; i++) {
      rev[i * 3] = cs.ring[(n - 1 - i) * 3];
      rev[i * 3 + 1] = cs.ring[(n - 1 - i) * 3 + 1];
      rev[i * 3 + 2] = cs.ring[(n - 1 - i) * 3 + 2];
    }
    for (const probe of cs.probes) {
      assert.equal(
        limb.insideRing(probe.p, rev),
        probe.inside,
        `${cs.name}: reversed ring, probe ${JSON.stringify(probe.p)}`,
      );
    }
  }
});
