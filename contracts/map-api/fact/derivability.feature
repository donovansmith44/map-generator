Feature: derivability — the scene tier is a composition of the fact tier
  Every drawn entry of a scene names the disposition of the fact tier
  that drew it (a census row, by layer and entity) and the borders it
  is made of; a standing buffer names every disposition standing in
  it. The two tiers are contract-tested against each other, by
  sampling.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |

  Scenario: every manifest entry traces to a disposition and a border
    When I render pieces fills, borders at year -1405 in style canaan as sampled
    Then every entry in sampled traces to a disposition and a border

  Scenario: every standing marker traces to a disposition
    When I render pieces markers, journeys at year -1405 in style canaan as sampled
    Then every entry in sampled traces to a disposition and a border
