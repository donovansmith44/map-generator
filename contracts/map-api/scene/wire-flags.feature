Feature: the scene's wire flags — water, journeys, labels, and ground, checked against what they actually do
  The Gherkin above imagines ten independently switchable pieces. Today's
  wire holds four working switches — water, journeys, labels, ground —
  and three that never move: borders, claims, and fills render on every
  request regardless of what is asked for. This feature does not aspire
  to the ten; it draws from the real four the way the algebra draws from
  all ten, so the same laws are checked against what the wire actually
  does, and the three that never move are pinned as a fact about today
  rather than assumed.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |

  @property
  Scenario: whatever the four real switches are set to, borders, claims and fills never move
    When I render pieces <someWirePieces> at year <someYear> in style <someStyle> as sceneA
    And I render pieces <someOtherWirePieces> at year <someYear> in style <someStyle> as sceneB
    Then sceneA's borders, claims and fills features equal sceneB's features

  @property
  Scenario: the four real switches compose the way the whole algebra claims
    When I render pieces <someWirePieces> at year <someYear> in style <someStyle> as sceneA
    And I render pieces <someOtherWirePieces> at year <someYear> in style <someStyle> as sceneB
    Then combining sceneA and sceneB equals rendering <someWirePieces> plus <someOtherWirePieces>

  @property
  Scenario: turning journeys off reaches past the piece it names
    When I GET /api/scene?year=<someYear>&zoom=90.0000&style=<someStyle> as withJourneys
    And I GET /api/scene?year=<someYear>&zoom=90.0000&style=<someStyle>&journeys=0 as withoutJourneys
    Then every label withJourneys has that withoutJourneys lacks is a label whose own piece says labels, not journeys
