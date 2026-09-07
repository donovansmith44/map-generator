#!/usr/bin/env python3
"""Re-runnable mutation evidence for the parameterization sweep.

WHY THIS EXISTS
---------------
sweep-report.md claims that every law the sweep added would go RED if its
implementation broke -- the property that test-first ordering is a proxy for,
asserted directly.  Round-1 review verified all twelve mutations by READING the
assertions, which is the right way to check a claim once.  This file makes the
claim re-runnable instead of testimonial, so a later reviewer (or a later
change to the generators) can re-derive it in one command rather than trusting
this report.

    python contracts/runner/mutation-evidence.py

Each batch is applied to the working tree, `cabal test` is run, the tree is
restored, and the observed failures are checked against the EXPECTED ones.

(It lives here rather than beside sweep-report.md because `.superpowers/sdd/`
is `*`-ignored by repo policy -- no file under it has ever been tracked -- and
an artifact that exists to be RE-RUN by a later reviewer has to be committed.
Here it also sits next to the sources it mutates.)

WHAT MAKES THIS HONEST RATHER THAN DECORATIVE
---------------------------------------------
A mutation script that silently fails to mutate -- because the code moved and
its search text no longer matches -- would run a pristine suite, see 0 failures,
and be unable to tell that from "the mutation was caught".  That is a check
satisfiable by its own failure mode, which is the thing this project forbids
(MEMORY: verify-distinct-not-nonnull).  So:

  * every mutation asserts its search text occurs EXACTLY ONCE before editing,
    and the script aborts if it does not -- the artifact rots LOUDLY;
  * the pass condition is not "some tests failed" but "exactly the expected set
    of tests failed, and no others" -- an over-broad mutation that reddens the
    whole suite fails this script just as a caught-nothing mutation does;
  * the restore runs in a `finally`, and the script refuses to start unless the
    two source files are clean in git, so an interrupted run cannot leave a
    mutation committed by accident.

Expected result: BATCH A caught (4/4), BATCH B caught (7/7),
                 BATCH C caught (7/7), tree restored.
"""

import os
import re
import subprocess
import sys

RUNNER = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(RUNNER, "..", ".."))

SRC = {name: os.path.join(RUNNER, "src", name) for name in ("Prop.hs", "Run.hs", "Steps.hs")}

# ---------------------------------------------------------------------------
# BATCH A -- the CORRELATIONS.  Both mutations replace a correlation-by-
# construction with an independent draw: exactly the accident that produced the
# composition bug (7cd31bd), approached from the other side.
# ---------------------------------------------------------------------------
BATCH_A = [
    (
        "A1: genStylePair's rotation may be ZERO, so the two styles can collide",
        "Prop.hs",
        "  d <- chooseInt (1, n - 1)\n",
        "  d <- chooseInt (0, n - 1)\n",
    ),
    (
        "A2: genNestedPieces draws its subset INDEPENDENTLY instead of from inside "
        "the superset",
        "Prop.hs",
        "  sub <- sublistOf (Set.toList super)\n"
        "  pure (PieceSet (Set.fromList sub), PieceSet super)",
        "  PieceSet sub <- genPieces\n"
        "  pure (PieceSet sub, PieceSet super)",
    ),
]

EXPECT_A = [
    "<someSubset> is nested inside <someSuperset> at EVERY iteration",
    "<someStyle> and <someOtherStyle> are DISTINCT at every iteration",
    "the subtractive law runs GREEN over 100 iterations",
    "the dress-locality law runs GREEN over 100 iterations",
]

# ---------------------------------------------------------------------------
# BATCH B -- the SKIP DISCIPLINE and the three new steps.  Each mutation makes
# one law unable to fail: a comparison that always holds, or a precondition
# miss reported as a pass.
# ---------------------------------------------------------------------------
BATCH_B = [
    (
        "B1: lawTally's all-skipped guard disabled -- a law that never ran reports green",
        "Run.hs",
        "  | iterations > 0 && skipped >= iterations =",
        "  | False =",
    ),
    (
        "B2: the identity law compares nothing",
        "Steps.hs",
        "          if combined == rc then Right w",
        "          if True then Right w",
    ),
    (
        "B3: the singleton fold compares nothing",
        "Steps.hs",
        "                  if stacked == expected then Right w",
        "                  if True then Right w",
    ),
    (
        "B4: the empty-list law accepts any body",
        "Steps.hs",
        "          | v == Array V.empty -> Right w",
        "          | True || v == Array V.empty -> Right w",
    ),
    (
        "B5: the empty piece set PASSES the fold instead of skipping it",
        "Steps.hs",
        "            | Set.null ps -> pure (StepSkipped",
        "            | False -> pure (StepSkipped",
    ),
]

EXPECT_B = [
    "the identity law fails, naming the ids that differ",
    "the singleton fold reports Left, naming the ids that went missing",
    "the singleton fold SKIPS an empty piece set",
    "the empty-list law fails on a NON-empty list",
    "the empty-list law fails on a body that is not a list at all",
    "a plain scenario whose ONLY run skipped is Failed",
    "a property whose iterations ALL skip is Failed",
]

# ---------------------------------------------------------------------------
# BATCH C -- FIX ROUND 1.  The round-1 review found four laws pinned by tests
# that the very mutation they exist to catch would survive.  These are those
# mutations, so the claim "the gap is closed" is re-runnable rather than
# testimonial -- which is the whole reason this file exists.
#
# C1 is the one that matters most: it is the reading the controller ratified
# (R81), and it is the precise failure mode the live server exhibits (reversing
# a span swaps fade_in/fade_out while rise/fall stand still, which the union
# reading calls green).
# ---------------------------------------------------------------------------
BATCH_C = [
    (
        "C1: the plan-vs-timeline law reads the UNION of the two fade kinds "
        "instead of matching them kind for kind",
        "Steps.hs",
        "            else if ins == rises && outs == falls then StepOk w",
        "            else if Set.union ins outs == Set.union rises falls then StepOk w",
    ),
    (
        "C2: the two-sided culling law checks only what LEAKED -- so a server "
        "that culled the whole world is green (characterization C5's trap)",
        "Steps.hs",
        "            _ | null missing && null leaked -> StepOk w",
        "            _ | null leaked -> StepOk w",
    ),
    (
        "C3: the endpoint-fade law drops its fade-out half entirely",
        "Steps.hs",
        "              badOut = outAbsent ++ outStayed",
        "              badOut = []",
    ),
    (
        "C4: the endpoint-fade law drops the 'was already in the earlier scene' "
        "disjunct of its fade-in half",
        "Steps.hs",
        "              badIn  = inAbsent ++ inAlready",
        "              badIn  = inAbsent",
    ),
    (
        "C5: the bogus-id law treats ANY other status -- a 500, a dead route -- "
        "as the law being met (the round-1 bug, restored)",
        "Steps.hs",
        "  | otherwise =
"
        "      StepFailed (\"the batch answered HTTP \" <> tshow code <> \", which is an error, not a \
"
        "                  \refusal: a server that fell over has not met this law\")",
        "  | otherwise = StepOk w",
    ),
    (
        "C6: the vertex ladder checks only its FIRST rung, so ultra-vs-fine is "
        "never weighed",
        "Steps.hs",
        "            _ | null (rungBad l1) && null (rungBad l2) -> StepOk w",
        "            _ | null (rungBad l1) -> StepOk w",
    ),
]

EXPECT_C = [
    "the plan-vs-timeline law DISTINGUISHES the fade kinds",
    "the two-sided culling law fails on the KEEPS half too",
    "a fade-out region that was never in the earlier scene is red",
    "a fade-out region still standing in the later scene is red",
    "a fade-in region that was ALREADY in the earlier scene is red",
    "a 5xx is NOT the law being met",
    "the vertex ladder's SECOND rung is load-bearing",
]


def read(path):
    with open(path, encoding="utf-8", newline="") as fh:
        return fh.read()


def write(path, text):
    with open(path, "w", encoding="utf-8", newline="") as fh:
        fh.write(text)


def require_clean():
    """Refuse to run against a dirty tree: the restore step below rewrites these
    files wholesale, so an uncommitted edit of theirs could be lost."""
    out = subprocess.run(
        ["git", "-C", REPO, "status", "--porcelain", "--"] + list(SRC.values()),
        capture_output=True, text=True, check=True,
    ).stdout.strip()
    if out:
        sys.exit(
            "refusing to run: these files have uncommitted changes, and this "
            "script restores them by overwriting.\n" + out
        )


def apply_batch(mutations):
    """Apply every mutation, asserting each search text is present exactly once.
    A mutation whose target has moved aborts the run rather than quietly doing
    nothing -- see the module docstring."""
    for label, filename, old, new in mutations:
        path = SRC[filename]
        text = read(path)
        hits = text.count(old)
        if hits != 1:
            raise SystemExit(
                "MUTATION ROTTED: %s\n  in %s\n  search text found %d times, expected "
                "exactly 1.\n  The code moved; update this script's search text before "
                "trusting any result from it." % (label, filename, hits)
            )
        write(path, text.replace(old, new, 1))
        print("    applied  %s" % label)


def failing_tests():
    """Run the suite and return (list of failing test names, total examples)."""
    proc = subprocess.run(
        ["cabal", "test"], cwd=RUNNER, capture_output=True, text=True,
        encoding="utf-8", errors="replace",
    )
    out = proc.stdout + proc.stderr
    fails = re.findall(r"^\s*\d+\) (.+)$", out, re.M)
    total = re.search(r"(\d+) examples?, (\d+) failures?", out)
    return fails, (total.group(0) if total else "??")


def run_batch(name, mutations, expected, originals):
    print("\n=== %s: applying %d mutation(s) ===" % (name, len(mutations)))
    try:
        apply_batch(mutations)
        fails, total = failing_tests()
        print("    %s" % total)
    finally:
        for path, text in originals.items():
            write(path, text)
        print("    tree restored")

    caught, missed, extra = [], [], list(fails)
    for want in expected:
        hit = [f for f in fails if want in f]
        (caught if hit else missed).append(want)
        for f in hit:
            if f in extra:
                extra.remove(f)

    for w in caught:
        print("    [caught]   %s" % w)
    for w in missed:
        print("    [MISSED]   %s   <-- mutation NOT detected" % w)
    for f in extra:
        print("    [EXTRA]    %s   <-- unexpected collateral failure" % f)

    ok = not missed and not extra
    print("    %s: %d/%d expected laws went red%s"
          % ("PASS" if ok else "FAIL", len(caught), len(expected),
             "" if ok else "  (see MISSED/EXTRA above)"))
    return ok


def main():
    require_clean()
    originals = {path: read(path) for path in SRC.values()}
    try:
        a = run_batch("BATCH A (correlations)", BATCH_A, EXPECT_A, originals)
        b = run_batch("BATCH B (skip discipline + the three new steps)",
                      BATCH_B, EXPECT_B, originals)
        c = run_batch("BATCH C (fix round 1: the discriminating cases)",
                      BATCH_C, EXPECT_C, originals)
    finally:
        for path, text in originals.items():
            write(path, text)

    print("\n=== baseline: the restored tree must be green again ===")
    fails, total = failing_tests()
    print("    %s" % total)
    baseline_ok = not fails
    if not baseline_ok:
        print("    FAIL: the tree did not restore cleanly -- %s" % fails)

    ok = a and b and c and baseline_ok
    print("\n%s" % ("ALL MUTATIONS CAUGHT, TREE CLEAN" if ok else "MUTATION EVIDENCE FAILED"))
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
