Feature: the terms — what a scene may be redistributed under
  Law 6 makes provenance total: every drawn thing names where it came
  from. A name alone does not answer the question a redistributor has,
  which is what that source PERMITS. So a source and its terms travel
  together: every entry of the answer's attribution carries both, and
  the scene declares the distinct terms the whole picture requires.

  THE TERMS FOLLOW THE CONTENT. A server could satisfy a "names its
  terms" law by declaring every licence it has ever heard of on every
  answer, which would be a constant and therefore not an answer. Two
  things forbid that here. The totality law below is stated in both
  directions, so a declared term no drawn source carries fails exactly
  as a carried term left undeclared does. And the terms must MOVE with
  what is drawn: a camera on open ocean draws no river and no tribal
  allotment, so it may not carry their terms.

  The sharper law — one PIECE alone, relief without the political
  ground over it — cannot be stated yet, because only four of the ten
  named piece switches work on the wire (wire-flags.feature owns that
  characterization). A camera is the discriminator available today.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |
    | detail | any of: coarse, fine, ultra |

  @property
  Scenario: every source a scene draws is credited with its own terms
    When I render pieces <somePieces> at year <someYear> in style <someStyle> as sampled
    Then sampled credits every source it draws with that source's own terms

  Scenario: the world's political ground is not public domain
    When I render pieces fills, borders at year -1405 in style canaan as world
    Then world's terms include GPL-3.0-only

  Scenario: open ocean carries neither a river's terms nor an allotment's
    When I render pieces all at year -1405 in style canaan looking at -40.0,-140.0 zoom 1 detail fine as ocean
    Then ocean's terms are exactly public-domain, CC-BY-SA-4.0, GPL-3.0-only
