Feature: the scene — a picture composed from pieces
  The scene is a monoid over pieces: any subset renders, absence is the
  identity, and combining parts is the same as rendering the whole. These
  laws are stated over EVERY piece and EVERY dress, because that is what
  the algebra claims — not over the corner of it today's wire can express.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |

  # --- PINNED EXAMPLES: regression anchors. Coordinates fixed of necessity;
  #     a whole-body fixture cannot be blessed against a generated year.

  Scenario: the twelve tribes scene, whole
    When I render pieces ground, water, fills, borders, labels, journeys at year -1405 in style canaan
    Then the response equals fixture "scene-1405-full"

  Scenario: a scene with no labels is still a scene — pinned whole
    When I render pieces ground, water, fills, borders, journeys at year -1405 in style canaan
    Then the response equals fixture "scene-1405-nolabels"

  Scenario: default-totality — an omitted dress is the declared classical default
    When I render pieces ground, water, fills, borders, labels, journeys at year -1405 in no style
    Then the response equals fixture "scene-1405-default-dress"

  # --- LAWS: quantified over every dimension they claim.

  @property
  Scenario: asking for the same map twice gives the same map
    When I render pieces <somePieces> at year <someYear> in style <someStyle> as first
    And I render pieces <somePieces> at year <someYear> in style <someStyle> as second
    Then first equals second

  @property
  Scenario: an empty map stacked onto any map changes nothing
    When I render pieces none at year <someYear> in style <someStyle> as empty
    And I render pieces <somePieces> at year <someYear> in style <someStyle> as some
    Then combining some and empty equals some

  @target @property
  Scenario: turning pieces off only removes things — nothing new appears
    When I render pieces <someSubset> at year <someYear> in style <someStyle> as fewer
    And I render pieces <someSuperset> at year <someYear> in style <someStyle> as more
    Then fewer's resources are a subset of more's resources

  @target @property
  Scenario: building a map in two parts gives the same map as building it in one
    When I render pieces <someA> at year <someYear> in style <someStyle> as sceneA
    And I render pieces <someB> at year <someYear> in style <someStyle> as sceneB
    Then combining sceneA and sceneB equals rendering <someA> plus <someB>

  @target @property
  Scenario: drawing each piece alone and stacking them rebuilds the whole map
    When I render pieces <somePieces> at year <someYear> in style <someStyle> as whole
    Then rendering each piece of <somePieces> alone and combining them equals whole

  @target @property
  Scenario: switching styles repaints the map without moving anything on it
    When I render pieces <somePieces> at year <someYear> in style <someStyle> as dressed
    And I render pieces <somePieces> at year <someYear> in style <someOtherStyle> as redressed
    Then dressed and redressed differ only in dress, never in geometry

  @target @property
  Scenario: everything on the map says which piece put it there
    When I render pieces <somePieces> at year <someYear> in style <someStyle>
    Then every feature entry carries a piece field

  @property
  Scenario: a span of no width is the instant itself
    When I GET /api/scene?year=<someYear>&zoom=90.0000&style=canaan as instant
    And I GET /api/scene?year=<someYear>&to=<someYear>&zoom=90.0000&style=canaan as span
    Then instant equals span
