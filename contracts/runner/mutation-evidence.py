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
                 BATCH C caught (8/8), BATCH D caught (12/12),
                 BATCH E caught (22/22 laws, 20 mutations),
                 tree restored.

KNOWN ISSUE, and why `main()` may appear to hang on BATCH A
-----------------------------------------------------------
Under BATCH A every test still RUNS and finishes; what does not finish in
any reasonable time is hspec's FAILURE REPORT.  Its `shouldBe` formatter
diffs the two values character by character, and BATCH A's mutations
redden assertions whose values are enormous -- a 200-element `[True,...]`
list, and a `LawRun` carrying a full shrunk counterexample.  Measured:
the suite completes all 273 examples and then sits in the report for
6m41s+ without producing a summary line.

This is NOT caused by fix round 1: it reproduces identically at c86fd0a
(the step-phase commit), where the run likewise finishes every test and
then hangs printing a `LawRun` diff.  Batches B-D, whose failing
assertions are all short, run to completion in the ordinary time, so any
phase can be run on its own:

    python mutation-evidence.py D          # one batch
    python mutation-evidence.py solo       # the per-mutation runs
    python mutation-evidence.py D solo     # both, in that order
    python mutation-evidence.py            # every phase, A through solo

The real fix for BATCH A is to make the reddened assertions report SMALL
values (a count and the first few offenders rather than a 200-element
list), which is a change to Spec.hs's assertions and belongs with whoever
owns them.  Recorded rather than worked around.

WHY THERE IS A `solo` PHASE AT ALL
----------------------------------
`run_batch` applies a whole batch and then requires the failing set to
equal EXPECT exactly.  That establishes what the UNION of the batch's
mutations reddens -- not that each one is individually caught.  In BATCH D
the gap is concrete: D5 stops `check` reporting background steps at all,
which makes `backgroundViolation` unreachable and D7's and D8's mutated
lines dead code.  D5 alone reddens four laws, including all three of
D7/D8's tests, so if D7 and D8 were both no-ops the batch would still
print PASS.

`SOLO` runs those mutations one at a time with their own expectations, so
the claim "D7 is caught" is produced by this file rather than asserted
about it.
"""

import os
import re
import subprocess
import sys

RUNNER = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(RUNNER, "..", ".."))

SRC = {name: os.path.join(RUNNER, "src", name)
       for name in ("Prop.hs", "Run.hs", "Steps.hs", "Vocab.hs", "Check.hs",
                    # R99 (BATCH E) mutates these two as well: the gate's own
                    # laws rest on a capture type and on how one invocation
                    # becomes one command line.
                    "Capture.hs", "World.hs")}
SRC["Ast.hs"] = os.path.join(RUNNER, "src", "Gherkin", "Ast.hs")
SRC["Parse.hs"] = os.path.join(RUNNER, "src", "Gherkin", "Parse.hs")

# ---------------------------------------------------------------------------
# BATCH A -- the CORRELATIONS.  Both mutations replace a correlation-by-
# construction with an independent draw: exactly the accident that produced the
# composition bug (7cd31bd), approached from the other side.
# ---------------------------------------------------------------------------
BATCH_A = [
    (
        # The search text carries the `pure` line as well as the draw.
        # Until the step phase, "d <- chooseInt (1, n - 1)" occurred once
        # in this file; `genDetailPair` rotates by the identical
        # expression, so the bare line now matches twice and this script
        # correctly refused to run rather than mutating the wrong
        # generator. That refusal is the artifact working -- see the
        # module docstring.
        "A1: genStylePair's rotation may be ZERO, so the two styles can collide",
        "Prop.hs",
        "  d <- chooseInt (1, n - 1)\n"
        "  pure (styleAt i, styleAt (i + d))",
        "  d <- chooseInt (0, n - 1)\n"
        "  pure (styleAt i, styleAt (i + d))",
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

# NOT ADDED, and the reason is worth keeping: the step phase's three new
# correlated pairs (detail, center, year) cannot be mutated the way A1
# mutates the style pair.  A1 works because there are three styles, so a
# rotation that "may be zero" collides one draw in three.  The centre
# pair rotates modulo 647,316,000 and the year pair modulo 4104, so the
# same mutation collides one draw in 647 million -- it was applied,
# measured, and caught NOTHING (273 examples, 0 failures), which makes it
# a mutation in name only.  Mutating those generators honestly means
# pinning the rotation to zero outright, which is a different shape of
# edit; left undone rather than committed as decoration.  Their
# distinctness is pinned directly instead, over 200 iterations each, by
# the three "<someX> and <someOtherX> are DISTINCT at every iteration"
# tests.

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
        # The 4xx guard, widened to swallow every non-2xx status -- which
        # is exactly what the old `(Left _, _) -> StepOk w` did, since
        # transportRaw's Left was produced for all of them. A 500 whose
        # body happens not to name the id then lands in the "refused but
        # nameless" red rather than green, so the guard is widened to
        # accept the body unconditionally too.
        r'''  | code >= 400 && code < 500 =
      if TE.encodeUtf8 bogusResourceId `BS.isInfixOf` body''',
        r'''  | code < 200 || code >= 300 =
      if True''',
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
    # C5 widens the 4xx guard AND drops the body check, so it also
    # retires the law that a refusal must NAME what it refused. Listed
    # because this script fails on an unexpected collateral failure just
    # as it fails on an uncaught mutation -- an EXTRA is either a law
    # this mutation legitimately breaks (say so, as here) or evidence
    # that the mutation is broader than its label claims.
    "a 4xx that does NOT name it is a different red",
    "the vertex ladder's SECOND rung is load-bearing",
]

# ---------------------------------------------------------------------------
# BATCH D -- R97's Background.  The feature could have been built two ways: a
# first-class AST field (what it is) or an expansion performed at parse time.
# These mutations are the ways the chosen design can be got WRONG, one per
# consumption boundary, plus the two that would have made the other design
# look equivalent.
# ---------------------------------------------------------------------------
BATCH_D = [
    (
        # NOT "a separately-drawn background": `holeSeed` is a pure
        # function of (group name, iteration index), so a hypothetical
        # implementation that drew the background's bindings separately
        # AT THE SAME INDEX would produce identical values and neither
        # the one-binding pin nor the correlated-pair pin would notice.
        # The pin that rules a separate draw out is the shrinking one,
        # where the hole exists ONLY in the background -- see the report.
        "D1: the property runner reads ftScenarios, so the background never runs at all",
        "Prop.hs",
        "        Right f -> mapM (run1 (ftTitle f)) (runnableScenarios f)",
        "        Right f -> mapM (run1 (ftTitle f)) (ftScenarios f)",
    ),
    (
        "D2: the plain runner reads ftScenarios, so a background never runs",
        "Run.hs",
        "                        (runnableScenarios f)",
        "                        (ftScenarios f)",
    ),
    (
        "D3: `Background:` is not an anchor, so a Vocabulary block is spliced "
        "INSIDE the background, between its header and its steps",
        "Vocab.hs",
        '  in "@" `T.isPrefixOf` s || "Scenario: " `T.isPrefixOf` s || s == "Background:"',
        '  in "@" `T.isPrefixOf` s || "Scenario: " `T.isPrefixOf` s',
    ),
    (
        "D4: the derived vocabulary ignores background steps, so a feature "
        "whose shared render lives in the background loses its whole table",
        "Vocab.hs",
        "  | sc <- runnableScenarios f",
        "  | sc <- ftScenarios f",
    ),
    (
        "D5: check stops reporting background steps at all -- an undefined "
        "step in a background is silently skipped",
        "Check.hs",
        "  | st <- ftBackground f",
        "  | st <- ([] :: [Step])",
    ),
    (
        "D7: a background step is classified under only the FIRST scenario's "
        "tags, so a hole that a later non-@property scenario would run "
        "unsubstituted is called clean",
        "Check.hs",
        "      | otherwise            = map scTags (ftScenarios f)",
        "      | otherwise            = take 1 (map scTags (ftScenarios f))",
    ),
    (
        "D8: a background in a feature with NO scenarios gets no treatment at "
        "all, so every step in it is silently clean",
        "Check.hs",
        "      | null (ftScenarios f) = [[]]",
        "      | null (ftScenarios f) = []",
    ),
    (
        "D9: the empty-Background: guard is removed, so a header with no "
        "steps under it parses as no background at all and the line the "
        "author wrote silently means nothing",
        "Parse.hs",
        "            Right ([], _) -> err n emptyBackground\n",
        "",
    ),
    (
        "D10: the traversal check and the corpus-wide pin SHARE reads "
        "ftScenarios, so a degenerate hole written only in a background is "
        "invisible to the hole-distinctness law",
        "Check.hs",
        "  | sc <- runnableScenarios f",
        "  | sc <- ftScenarios f",
    ),
    (
        "D6: the background is APPENDED after each scenario's own steps "
        "instead of prepended, so setup runs after the law that needs it",
        "Ast.hs",
        "  [ sc { scSteps = ftBackground f ++ scSteps sc } | sc <- ftScenarios f ]",
        "  [ sc { scSteps = scSteps sc ++ ftBackground f } | sc <- ftScenarios f ]",
    ),
]

EXPECT_D = [
    "one binding per iteration",
    "a CORRELATED PAIR split across the boundary still correlates",
    "shrinking reaches a background hole",
    "the runner really fetches the background FIRST",
    "a fresh Vocabulary block goes ABOVE a Background",
    "an undefined step in a Background is reported ONCE",
    "runnableScenarios prepends the background to EVERY scenario",
    "a background hole is a BAD-VALUE when ANY scenario would run it",
    "a Background in a feature with NO scenarios is still checked",
    "the treatment it gets there is the UNTAGGED one",
    "an EMPTY Background: is a parse error naming the file and line",
    "a degenerate hole written ONLY in a background is caught by the",
]

# ---------------------------------------------------------------------------
# WHAT A BATCH TOTAL DOES AND DOES NOT PROVE.
#
# `run_batch` applies a whole batch at once and then requires the failing
# set to equal EXPECT exactly.  That establishes the UNION of the batch's
# mutations reddens exactly those laws -- it does NOT establish that each
# mutation is individually caught, and in BATCH D that gap is concrete
# rather than theoretical:
#
#   D5 (check stops reporting background steps at all) makes
#   `backgroundViolation` unreachable, so D7's and D8's mutated lines
#   become dead code.  Applied ALONE, D5 reddens four laws -- the ORPHAN
#   pin and all three of the tests D7 and D8 exist to prove.  If D7 and
#   D8 were both no-ops, this batch would still print PASS.
#
# So the per-mutation evidence for the Check.hs subset comes from the
# SOLO phase below (`SOLO_RUNS` / `run_solo`), which this file runs and
# prints -- not from the batch total, and not from a table typed into a
# report.  The same subsumption question is worth asking of batches A-C;
# reviewed and deferred with reasons, see the task report.
# ---------------------------------------------------------------------------


def read(path):
    with open(path, encoding="utf-8", newline="") as fh:
        return fh.read()


def write(path, text):
    with open(path, "w", encoding="utf-8", newline="") as fh:
        fh.write(text)


def line_ending(text):
    """This file's dominant line ending, as the two characters it is written
    with.  Not cosmetic: the search strings below are written with "\\n", and
    this repository has MIXED endings with no `.gitattributes` -- on a machine
    with `core.autocrlf=true` a fresh clone checks some of these sources out as
    CRLF while the authoring tree holds them as LF.  `read()` uses newline=""
    and therefore PRESERVES whatever is on disk rather than normalizing it, so
    every multi-line search string is ending-sensitive and a clone would find
    zero occurrences of a string the authoring tree finds once.

    That is a REPRODUCIBILITY defect and it is fixed here rather than papered
    over: the search and replacement text is translated into the file's own
    ending before matching, so the mutation lands identically either way, and
    the file is written back in exactly the ending it arrived in -- no
    normalize-and-continue, and no silent rewriting of a file's endings as a
    side effect of mutating one line of it.  The exactly-once assertion in
    `apply_batch` is unchanged and still aborts loudly if the translated text
    does not match: a search string that has genuinely rotted must still fail
    loudly rather than being massaged until it matches something.
    """
    return "\r\n" if text.count("\r\n") > text.count("\n") - text.count("\r\n") else "\n"


def to_ending(text, ending):
    """`text` (written with "\\n") rendered in `ending`."""
    return text.replace("\r\n", "\n").replace("\n", ending)


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
        # written with "\n"; matched in whatever this file actually uses
        ending = line_ending(text)
        old_here, new_here = to_ending(old, ending), to_ending(new, ending)
        hits = text.count(old_here)
        if hits != 1:
            raise SystemExit(
                "MUTATION ROTTED: %s\n  in %s (line endings: %s)\n  search text found %d "
                "times, expected exactly 1.\n  The code moved; update this script's search "
                "text before trusting any result from it."
                % (label, filename, repr(ending), hits)
            )
        write(path, text.replace(old_here, new_here, 1))
        print("    applied  %s" % label)


def failing_tests():
    """Run the suite and return (failing test names, total line, ran at all?).

    The third value is the one the closing baseline check needs and used to be
    thrown away (review finding I-2).  `fails` is empty both when the suite is
    GREEN and when it never BUILT, and the two must not be the same answer:
    with only the first two values, a restored tree that no longer compiles
    printed `ALL MUTATIONS CAUGHT, TREE CLEAN` -- a check satisfiable by the
    exact failure mode it exists to rule out, in the script whose subject is
    checks satisfiable by their failure mode.

    `ran` is true only when the suite actually produced a summary line. A
    non-zero return code with a summary line is an ordinary red run (which is
    what a mutation is SUPPOSED to produce); no summary line at all is a build
    failure or a crash, and that is never evidence of anything.
    """
    proc = subprocess.run(
        ["cabal", "test"], cwd=RUNNER, capture_output=True, text=True,
        encoding="utf-8", errors="replace",
    )
    out = proc.stdout + proc.stderr
    fails = re.findall(r"^\s*\d+\) (.+)$", out, re.M)
    total = re.search(r"(\d+) examples?, (\d+) failures?", out)
    if total is None:
        print("    the suite produced no summary line (exit %d) -- it did not run:"
              % proc.returncode)
        for line in out.strip().splitlines()[-8:]:
            print("      | %s" % line)
    return fails, (total.group(0) if total else "DID NOT RUN"), total is not None


def run_batch(name, mutations, expected, originals):
    print("\n=== %s: applying %d mutation(s) ===" % (name, len(mutations)))
    try:
        apply_batch(mutations)
        fails, total, _ran = failing_tests()
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


# ---------------------------------------------------------------------------
# THE SOLO RUNS -- one mutation at a time, with its OWN expectations.
#
# D5, D7 and D8 are here because D5 subsumes the other two (see the module
# docstring).  D9 and D10 are here because they were added in the round
# whose whole subject was per-mutation evidence, and a mutation whose only
# evidence is a batch total is exactly what that round existed to stop.
# ---------------------------------------------------------------------------
SOLO_RUNS = [
    ("D5:", ["an undefined step in a Background is reported ONCE",
             # the subsumption itself, asserted rather than remembered:
             # these three are D7's and D8's laws, and D5 alone reddens
             # them, which is why those two need runs of their own.
             "a background hole is a BAD-VALUE when ANY scenario would run it",
             "a Background in a feature with NO scenarios is still checked",
             "the treatment it gets there is the UNTAGGED one"]),
    ("D7:", ["a background hole is a BAD-VALUE when ANY scenario would run it"]),
    ("D8:", ["a Background in a feature with NO scenarios is still checked",
             "the treatment it gets there is the UNTAGGED one"]),
    ("D9:", ["an EMPTY Background: is a parse error naming the file and line"]),
    ("D10:", ["a degenerate hole written ONLY in a background is caught by the"]),
    # BATCH E's own solo runs (review finding I-1). The batch total is a
    # statement about the UNION, and three subsumptions in E make it
    # silent about individual mutations:
    #
    #   * E5 (cost is `maximum`, not `sum`) alone reddens E6's law as
    #     well as its own -- demonstrated by the reviewer, so `PASS`
    #     would have printed even if E6 were a no-op;
    #   * E19 and E20 redden assertions that live INSIDE tests E11 and
    #     E10 already claim, so they have no expectation of their own by
    #     design and this is the only place they are answered;
    #   * E12 and E13 share the `gateSaid` path, so either could be
    #     carrying the other.
    #
    # E16 and E17 are here for a different reason: they are two of the
    # three mutations that survived round 1, and a fix whose only evidence
    # is a batch total is the shape of defect that let them survive.
    ("E5:", ["a law that judges it TWICE costs twice as much",
             # the subsumption itself, asserted rather than remembered
             "the budget only ever lowers a count"]),
    ("E6:", ["the budget only ever lowers a count"]),
    ("E12:", ["wants the verdict X AND a reason"]),
    ("E13:", ["pins the NAMED set whole"]),
    ("E16:", ["catches a gate that says NOT-STILL and writes the baseline anyway"]),
    ("E17:", ["every judging step builds the invocation its own law names"]),
    ("E19:", ["refuses a gate that found drift somewhere else"]),
    ("E20:", ["refuses two answers that agree"]),
]

# ---------------------------------------------------------------------------
# BATCH E -- R99, THE GOLDEN GATE'S OWN LAWS.
#
# Everything the gate's laws rest on that is NOT the gate itself: the two
# types they quantify over, the order a counterexample narrows along, the
# budget that decides how many times an expensive law runs, and -- the
# largest share -- the readings the Then steps make of the verdict the gate
# prints.
#
# These mutate Haskell and are caught by `cabal test`, with no browser and
# no viewer: that is the point of the fake gate in Spec's "the gate's laws
# are red for the reason they name" block.  A mutation of golden.js itself
# could only be caught by a run that launches Chromium for fifty seconds
# per law, which is why the gate's own falsifiability is demonstrated in
# the report by DRIVING it under conditions rather than by mutating it
# here.
# ---------------------------------------------------------------------------
BATCH_E = [
    (
        "E1: a Ranged universe prints an empty parenthetical when it has no "
        "caveat, so the corpus's own Vocabulary row stops matching its type",
        "Capture.hs",
        '  "whole number from " <> lo <> " to " <> hi <> maybe "" (\\c -> " (" <> c <> ")") m',
        '  "whole number from " <> lo <> " to " <> hi <> " (" <> maybe "" id m <> ")"',
    ),
    (
        "E2: a probe outside the gate's 5x5 grid parses anyway, so <someProbe> "
        "could name a sample that does not exist",
        "Capture.hs",
        '    [(p, "")] | p >= 0 && p < probeCount -> Right (Probe p)',
        '    [(p, "")] | p >= -99 && p < 99 -> Right (Probe p)',
    ),
    (
        "E3: the cameras are alphabetised, which puts the hemisphere before "
        "the levant -- neither the order the views are held in nor the order "
        "the corpus's Vocabulary block states",
        "Capture.hs",
        "gateCameraNames = map gateCameraText [minBound .. maxBound]",
        "gateCameraNames = sort (map gateCameraText [minBound .. maxBound])",
    ),
    (
        "E4: the probe order offers candidates that do not rank lower, so a "
        "counterexample can walk in a circle instead of narrowing",
        "Prop.hs",
        "           , probeRank p' < probeRank p ]",
        "           , True ]",
    ),
    (
        "E5: an iteration's cost is the largest step rather than the sum, so a "
        "law that judges the map twice is budgeted as if it judged it once",
        "Prop.hs",
        "iterationCost defs sc = sum",
        "iterationCost defs sc = maximum . (0 :) $",
    ),
    (
        "E6: the budget may RAISE a count above what was asked for",
        "Prop.hs",
        "  | otherwise = max 1 (min requested (lawSecondsBudget `div` cost))",
        "  | otherwise = max 1 (lawSecondsBudget `div` cost)",
    ),
    (
        # Round 2 (review finding M-4): this used to edit only the HEADER
        # string literal, leaving the data cell intact -- so the mutant still
        # printed the true count and merely mislabelled the column, and the
        # test that caught it read back the same literal that had been
        # changed.  Near-tautological, and not the failure mode the label
        # claims.  The ROW BUILDER is the thing that would have to break for
        # a six-iteration run to read like a hundred-iteration one, so that
        # is what is broken here.
        "E7: the report table drops the iteration count from its ROWS, so a "
        "law that ran 6 of a requested 100 reads exactly like one that ran all 100",
        "Run.hs",
        '       <> " | " <> T.pack (show (srRuns r))\n',
        "",
    ),
    (
        "E8: --check is passed on a BLESS run and withheld on a check run",
        "World.hs",
        '  ++ [ "--check" | not (invBless inv) ]',
        '  ++ [ "--check" | invBless inv ]',
    ),
    (
        "E9: the determinism law compares only the gate's headline, not the "
        "whole verdict -- the defect the feature was written for said "
        "REGRESSION twice with two different failing sets",
        "Steps.hs",
        "        if gvBody va == gvBody vb then Right w",
        "        if gvTag va == gvTag vb then Right w",
    ),
    (
        "E10: 'different verdicts' accepts the all-clear, so a gate that "
        "called a map showing nothing HOLD would satisfy the law",
        "Steps.hs",
        '        if gvTag va == "HOLD" || gvTag vb == "HOLD"',
        '        if gvTag va == "NO-SUCH-VERDICT"',
    ),
    (
        "E11: 'and nowhere else' stops looking anywhere else, so a gate that "
        "reported drift at every probe would satisfy the detection law",
        "Steps.hs",
        "        else if not (null elsewhere)",
        "        else if not (null (drop 99 elsewhere))",
    ),
    (
        "E12: a verdict tag with no reason behind it is accepted, so 'says "
        "the renderer is down' is satisfied by a gate that names a category "
        "and nothing else",
        "Steps.hs",
        "    else if T.null (T.strip (gvDetail v))",
        "    else if T.null (T.strip (gvTag v))",
    ),
    (
        "E13: the missing-stops law checks only that SOMETHING was named, so "
        "a gate that failed while naming the wrong stops would pass",
        "Steps.hs",
        "        if sort (gvMissing v) == sort [gateDroppedStop] then Right w",
        "        if not (null (gvMissing v)) then Right w",
    ),
    (
        "E14: the FIRST verdict line is read instead of the last, so a line "
        "printed before the gate's final word can stand in for the answer",
        "Steps.hs",
        "      v <- either (Left . T.pack) Right (eitherDecodeStrict (TE.encodeUtf8 (last ls)))",
        "      v <- either (Left . T.pack) Right (eitherDecodeStrict (TE.encodeUtf8 (head ls)))",
    ),
    # ---- round 2: the three the reviewer's own mutations found unpinned ----
    #
    # Each of these SURVIVED the first round -- applied, verified on disk,
    # `cabal test` still 359 examples 0 failures.  They are here because a
    # finding that is fixed but not mutated is a finding that can come back.
    (
        "E15: the law budget doubles, so every property law silently costs "
        "twice the wall clock it declares",
        "Prop.hs",
        "lawSecondsBudget = 300",
        "lawSecondsBudget = 600",
    ),
    (
        "E16: the bless law stops checking whether anything was WRITTEN, "
        "keeping only its verdict half -- the one law standing between a view "
        "that will not hold still and the owner's golden fixture",
        "Steps.hs",
        "          else if written",
        "          else if False",
    ),
    (
        "E17: the under-tolerance law repaints FAR, collapsing the two "
        "@property laws into one -- a change under the gate's tolerance and a "
        "change anywhere become the same law",
        "Steps.hs",
        "      \\(p, cam) w -> pure (Right w { pending = Just (repaintCondition p cam RepaintUnder) })",
        "      \\(p, cam) w -> pure (Right w { pending = Just (repaintCondition p cam RepaintFar) })",
    ),
    (
        "E18: a gate-driving step loses its declared cost, so it reports zero "
        "and is given the full 100 iterations of a fifty-second run",
        "Steps.hs",
        "  , costing gateDeadSeconds $",
        "  , id $",
    ),
    (
        "E19: 'and nowhere else' stops checking that the repainted probe "
        "drifted at every stop judged, going blind along the stop axis",
        "Steps.hs",
        "        else if stopsHit /= owed",
        "        else if False && stopsHit /= owed",
    ),
    (
        "E20: 'different verdicts' stops naming WHICH two verdicts, so two "
        "unrelated crashes satisfy a law about telling blank from absent",
        "Steps.hs",
        "        else if Set.fromList [gvTag va, gvTag vb] /= blankAndAbsentVerdicts",
        "        else if False",
    ),
]

# R99's expectations. E8 reddens the whole argv block, because every one of
# those laws reads the same command line; the rest each name one law.
EXPECT_E = [
    "a Ranged universe with no caveat reads as the bare sentence",
    "the grid's ends are in and its neighbours are out",
    "the two cameras are the gate's own two, in the gate's own order",
    "every probe candidate ranks strictly lower",
    # E4 reddens TWO laws, and the second is the point of having it: the
    # generic order law quantifies over every group in the registry, so a
    # new group whose shrinker breaks well-foundedness is caught whether
    # or not anyone remembered to write it a law of its own.
    "every shrink candidate of every group RANKS strictly below",
    "a law that judges it TWICE costs twice as much",
    "the budget only ever lowers a count",
    "the report table carries the skip count in its own column",
    "a check run names the mode and the declared subset",
    "a bless run does NOT pass --check",
    "no declared subset means no --stops at all",
    "a condition is resolved BESIDE the gate",
    "compares the WHOLE verdict",
    "refuses two answers that agree",
    "refuses a gate that found drift somewhere else",
    "wants the verdict X AND a reason",
    "pins the NAMED set whole",
    "reads the LAST verdict line",
    # Round 2's four new laws. E19 and E20 deliberately have NO entry of
    # their own: each reddens a test another mutation already claims (the
    # "nowhere else" law and the "different verdicts" law each grew a new
    # assertion rather than a new test), so the batch total says nothing
    # about them and they are answered by SOLO_RUNS below instead. That is
    # the honest bookkeeping, not a gap.
    "a law that judges the map once gets SIX iterations",
    "catches a gate that says NOT-STILL and writes the baseline anyway",
    "every judging step builds the invocation its own law names",
    "every step that drives the gate declares what it costs",
]

ALL_MUTATIONS = BATCH_A + BATCH_B + BATCH_C + BATCH_D + BATCH_E


def run_one(label_fragment, expected, originals):
    """Apply ONE mutation, by a fragment of its label, and report which laws
    went red.  For a mutation another in its batch subsumes, this is the only
    run that says anything about it."""
    picked = [ m for m in ALL_MUTATIONS if m[0].startswith(label_fragment) ]
    if len(picked) != 1:
        raise SystemExit(
            "run_one: %r matched %d mutations, expected exactly 1. The labels "
            "moved; update SOLO_RUNS before trusting any result from it."
            % (label_fragment, len(picked)))
    return run_batch("SOLO %s" % picked[0][0].split(":")[0], picked, expected,
                     originals)


def run_solo(originals):
    print("\n=== SOLO: each mutation alone, with its own expectations ===")
    results = [ (frag.rstrip(":"), run_one(frag, expected, originals))
                for frag, expected in SOLO_RUNS ]
    print("\n    --- solo summary ---")
    for name, ok in results:
        print("    %-5s %s" % (name, "caught" if ok else "NOT CAUGHT"))
    every = all(ok for _, ok in results)
    print("    SOLO: %d/%d mutations individually caught"
          % (sum(1 for _, ok in results if ok), len(results)))
    return every


PHASES = [
    ("A", "BATCH A (correlations)", BATCH_A, EXPECT_A),
    ("B", "BATCH B (skip discipline + the three new steps)", BATCH_B, EXPECT_B),
    ("C", "BATCH C (fix round 1: the discriminating cases)", BATCH_C, EXPECT_C),
    ("D", "BATCH D (R97: Background)", BATCH_D, EXPECT_D),
    ("E", "BATCH E (R99: the golden gate's own laws)", BATCH_E, EXPECT_E),
]

PHASE_NAMES = [ p[0] for p in PHASES ] + ["SOLO"]


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    wanted = [ a.upper() for a in argv ] or PHASE_NAMES
    unknown = [ w for w in wanted if w not in PHASE_NAMES ]
    if unknown:
        raise SystemExit("unknown phase(s): %s. Known: %s"
                         % (", ".join(unknown), ", ".join(PHASE_NAMES)))

    require_clean()
    originals = {path: read(path) for path in SRC.values()}
    results = []
    try:
        for w in wanted:
            if w == "SOLO":
                results.append(("SOLO", run_solo(originals)))
            else:
                for name, label, muts, expected in PHASES:
                    if name == w:
                        results.append((w, run_batch(label, muts, expected, originals)))
    finally:
        for path, text in originals.items():
            write(path, text)

    print("\n=== baseline: the restored tree must be green again ===")
    fails, total, ran = failing_tests()
    print("    %s" % total)
    # Three things, not one: the suite RAN, it was green, and the files this
    # script rewrites are byte-identical to what git holds. A restored tree
    # that does not compile, or one left with a mutation on disk, must never
    # reach "TREE CLEAN" (review finding I-2).
    baseline_ok = ran and not fails
    if not ran:
        print("    FAIL: the restored tree did not build, so its greenness is unknown")
    elif fails:
        print("    FAIL: the tree did not restore cleanly -- %s" % fails)
    dirty = subprocess.run(
        ["git", "-C", REPO, "status", "--porcelain", "--"] + list(SRC.values()),
        capture_output=True, text=True,
    ).stdout.strip()
    if dirty:
        print("    FAIL: these files are not as git holds them after restore:\n%s" % dirty)
        baseline_ok = False
    else:
        print("    every mutated file is byte-identical to git")

    ok = all(r for _, r in results) and baseline_ok
    print("\n    phases run: %s" % ", ".join(w for w, _ in results))
    print("%s" % ("ALL MUTATIONS CAUGHT, TREE CLEAN" if ok else "MUTATION EVIDENCE FAILED"))
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
