Feature: the answer is the whole map — a viewer draws, it does not decide
  Everything needed to draw the map is in the answer. Where each name
  sits on the page is part of the map, not something a viewer works out
  for itself. Two viewers given the same answer draw the same picture,
  and the same answer drawn twice is the same picture — because there
  is nothing left to decide between the answer and the ink.

  A viewer that decides makes the map a function of its own history.
  That is not a hypothetical: the same year, at the same camera, on an
  unchanged server, draws a different set of names depending on whether
  you had been out to the globe and come back — because the viewer
  remembers where it put a name, and forgets when the name is culled.
  A name is where it is because the map says so, or the map is not the
  whole map.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |

  # Today a label carries an anchor — where the THING is — and the
  # viewer works out where the words go, retaining the offset frame to
  # frame so the text does not jitter. Red until placement is part of
  # the answer.
  @target @property
  Scenario: every name on the map says where it sits
    When I render pieces all at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as view
    Then every label of view carries a placement

  # Placement is only worth sending if it settles the question the
  # viewer was settling for itself. A list of unresolved wishes is not
  # a map.
  @target @property
  Scenario: no two names are printed on top of each other
    When I render pieces all at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as view
    Then no two labels of view overlap

  # The viewer draws what it is sent, all of it. Today it culls again
  # by its own legibility floor, so what arrives and what appears are
  # two different sets — and only the viewer knows the second one.
  @target @property
  Scenario: every name that is sent is a name that is drawn
    When I render pieces all at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as view
    Then every label of view is legible at the view it was asked for

  # Placement belongs to the view, not to the moment of asking. Two
  # identical questions get one answer, or placement has smuggled a
  # clock in.
  @property
  Scenario: the same view puts the names in the same places
    When I render pieces all at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as first
    And I render pieces all at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as second
    Then first equals second

  # The door this law locks. Green today — the server holds nothing
  # between requests — and it must stay green: an answer that depends
  # on what was asked before is not an answer about the map. This is
  # exactly the property the viewer lost.
  @property
  Scenario: asking for other maps in between changes nothing
    When I render pieces all at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as first
    And I render pieces all at year <someOtherYear> in style <someOtherStyle> looking at <someOtherCenter> zoom <someZoom> as detour
    And I render pieces all at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as again
    Then again equals first

  # A name names a thing that is on the map. Placement cannot rescue a
  # name whose subject was never sent — and a viewer deciding placement
  # cannot notice that it happened.
  @property
  Scenario: every name names something that is there
    When I render pieces all at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as view
    Then every label of view names a feature of view
