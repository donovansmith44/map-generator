@target
Feature: derivability — the scene tier is a composition of the fact tier
  Spec §4: "every manifest entry must be traceable to `disposition` +
  `borders` answers — contract-tested by sampling." Until `disposition`
  (Stage 2) and `borders` (Stage 3) exist on the wire, this law has
  nothing to sample against and is declared RED rather than absent: the
  two tiers are currently contract-tested in isolation and never against
  each other, and that gap belongs in the corpus, not only in a note.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |

  @target
  Scenario: every manifest entry traces to a disposition and a border
    When I render pieces fills, borders at year -1405 in style canaan as sampled
    Then every entry in sampled traces to a disposition and a border
