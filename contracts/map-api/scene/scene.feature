Feature: the scene — a picture composed from pieces
  The scene is a monoid over pieces: any subset renders, absence is the
  identity, and adding a piece back changes nothing else. In v0.1 only
  ground, water, labels, and journeys are toggleable on the wire; the
  rest are always present — a wart this contract records rather than
  hides, retired when the pieces parameter lands.

  Scenario: the twelve tribes scene, whole
    When I render pieces ground, water, fills, borders, labels, journeys at year -1405 in style canaan
    Then the response equals fixture "scene-1405-full"

  Scenario: a scene with no labels is still a scene — pinned whole
    When I render pieces ground, water, fills, borders, journeys at year -1405 in style canaan
    Then the response equals fixture "scene-1405-nolabels"

  Scenario: omission is subtractive, not destructive
    When I render pieces ground, water, fills, borders, labels, journeys at year -1405 in style canaan as full
    And I render pieces fills, borders, labels, journeys at year -1405 in style canaan as noWater
    Then noWater's resources are a subset of full's resources

  Scenario: rendering is deterministic
    When I render pieces ground, water, fills, borders, labels, journeys at year -1405 in style canaan as first
    And I render pieces ground, water, fills, borders, labels, journeys at year -1405 in style canaan as second
    Then first equals second

  @property
  Scenario: determinism holds for any piece subset at any year
    When I render pieces <somePieces> at year <someYear> in style canaan as first
    And I render pieces <somePieces> at year <someYear> in style canaan as second
    Then first equals second

  @target
  Scenario: every manifest entry names its piece
    When I render pieces ground, water, fills, borders, labels, journeys at year -1405 in style canaan
    Then every feature entry carries a piece field

  @target @property
  Scenario: composition — pieces render separately and combine to the whole
    When I render pieces <someA> at year <someYear> in style canaan as sceneA
    And I render pieces <someB> at year <someYear> in style canaan as sceneB
    Then combining sceneA and sceneB equals rendering <someA> plus <someB>

  @target
  Scenario: dress-locality — restyling one piece leaves the others untouched
    When I render pieces ground, fills at year -1405 in style canaan as dressed
    And I render pieces ground, fills at year -1405 in style slate as redressed
    Then dressed and redressed differ only in dress, never in geometry
