Feature: detail — how much geometry, never which geometry
  Three tiers a person actually uses: coarse (the world at a glance),
  fine (the working map), ultra (leaning all the way in). The tier
  numbers in the vocabulary are the values the server's own rule
  produces (lod = clamp(radians(zoom/width), 1e-6, 0.01)) at its
  canonical zooms — read from the code and the characterization,
  never tuned.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |
    | detail | any of: coarse, fine, ultra |

  # Characterization: feature-id invariance across lod in [0, 6] holds
  # EXACTLY today. Green, and load-bearing.
  @property
  Scenario: detail changes how much is drawn, never what exists
    When I render pieces <somePieces> at year <someYear> in style <someStyle> detail <someDetail> as one
    And I render pieces <somePieces> at year <someYear> in style <someStyle> detail <someOtherDetail> as other
    Then one and other draw the same features

  # Characterization: vertex monotonicity HOLDS below lod 1e-4 and
  # FAILS above it — 498/917 violations on the 1.5e-3 -> 1e-2 rung;
  # the vertex curve is U-shaped with its minimum at lod 0.01.
  @target
  Scenario: leaning in never loses geometry
    When I render pieces all at year -1405 in style canaan detail coarse as coarse
    And I render pieces all at year -1405 in style canaan detail fine as fine
    And I render pieces all at year -1405 in style canaan detail ultra as ultra
    Then every shared resource has at least as many vertices in fine as in coarse, and in ultra as in fine

  # Characterization: 490/917 features carry MORE vertices at zoom 0.05
  # than at zoom 90 (+31,199 total; worst single case 1388 vs 5),
  # because below-limit rings ship unsimplified. Leaning out must never
  # cost more than leaning in.
  @target
  Scenario: the world at a glance is never heavier than the street corner
    When I render pieces all at year -1405 in style canaan detail coarse as glance
    And I render pieces all at year -1405 in style canaan detail ultra as corner
    Then no shared resource of glance carries more vertices than it does in corner

  Scenario: an unspecified detail is exactly the detail the zoom implies
    When I render pieces all at year -1405 in style canaan looking at 31.5,35.0 zoom 4 as implicit
    And I render pieces all at year -1405 in style canaan looking at 31.5,35.0 zoom 4 detail fine as explicit
    Then implicit equals explicit
