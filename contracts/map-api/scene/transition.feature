Feature: the transition — how the map moves between two moments
  The scrubber's animation plan, pinned whole and quantified over the
  moments it claims to connect: borders that move morph, regions that
  rise fade in, regions that fall fade out — for ANY two years, not
  two the author liked. The plan must tell the same story as the
  changes timeline and the two scenes it joins.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |

  Scenario: the conquest's whole animation plan
    When I GET /api/transition?from=-1407&to=-1405&zoom=90.0000&style=canaan
    Then the response equals fixture "transition-conquest"

  Scenario: the exile's whole animation plan
    When I GET /api/transition?from=-590&to=-586&zoom=90.0000&style=canaan
    Then the response equals fixture "transition-exile"

  @property
  Scenario: no time passing means nothing moves
    When I GET /api/transition?from=<someYear>&to=<someYear>&zoom=90.0000&style=<someStyle> as still
    Then still's steps are the empty list

  @property
  Scenario: the same journey twice is the same journey
    When I GET /api/transition?from=<someYear>&to=<someOtherYear>&zoom=90.0000&style=<someStyle> as first
    And I GET /api/transition?from=<someYear>&to=<someOtherYear>&zoom=90.0000&style=<someStyle> as second
    Then first equals second

  @property
  Scenario: the plan and the timeline tell one story, wherever you scrub
    When I GET /api/transition?from=<someYear>&to=<someOtherYear>&zoom=90.0000&style=<someStyle> as plan
    And I GET /api/changes?from=<someYear>&to=<someOtherYear> as story
    Then every fade in plan is a rise or fall in story, and every rise and fall in story has a fade in plan

  @target @property
  Scenario: what fades in arrives, what fades out departs — between any two moments
    When I GET /api/transition?from=<someYear>&to=<someOtherYear>&zoom=90.0000&style=canaan as plan
    And I render pieces all at year <someYear> in style canaan as before
    And I render pieces all at year <someOtherYear> in style canaan as after
    Then every fade-in region of plan is in after and not before, and every fade-out region is in before and not after

  @property
  Scenario: the road back is the road there, reversed
    When I GET /api/transition?from=<someYear>&to=<someOtherYear>&zoom=90.0000&style=<someStyle> as there
    And I GET /api/transition?from=<someOtherYear>&to=<someYear>&zoom=90.0000&style=<someStyle> as back
    Then back is there with every morph reversed and every fade inverted

  @target
  Scenario: a border morphs with its real shape, not a stick figure
    When I GET /api/transition?from=-1407&to=-1405&zoom=90.0000&style=canaan as plan
    And I render pieces all at year -1405 in style canaan as after
    Then every morph of plan carries at least as many points as its border carries vertices in after
