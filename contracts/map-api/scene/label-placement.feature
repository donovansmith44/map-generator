Feature: label placement — the map says where every name sits, not the viewer
  Where each name sits on the page is part of the map. An answer asked
  at a view says which view it placed its names for, and every name it
  sends arrives with the box its words are drawn in, already settled,
  already clear of its neighbours and inside the page: two viewers
  given the same answer draw the same names in the same places,
  because there is nothing left to decide between the answer and the
  ink. A name that cannot be drawn at that view is not sent.

  A name also stands where the thing it names is. A land's name sits
  within its own region, give or take the overflow its style declares
  in em of the name's size, because the schematic hulls of a promised
  allotment are narrower than the words that name them. A city's name
  says which region it stands in, or that it stands on ground nothing
  claims; a city standing silently nowhere is the failure.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |

  Background:
    When I render pieces all at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as view

  @property
  Scenario: every name on the map says where it sits
    Then every label of view carries a placement

  @property
  Scenario: no two names are printed on top of each other
    Then no two labels of view overlap

  @property
  Scenario: every name that is sent is a name that is drawn
    Then every label of view is legible at the view it was asked for

  @property
  Scenario: a land's name sits on the land it names
    Then every land name of view sits within its own region, give or take the overflow its style declares

  @property
  Scenario: a city stands on ground the map names
    Then every city of view stands in a region it names, or on ground declared unclaimed

  @property
  Scenario: asking for other maps in between changes nothing
    When I render pieces all at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as first
    And I render pieces all at year <someOtherYear> in style <someOtherStyle> looking at <someOtherCenter> zoom <someZoom> as detour
    And I render pieces all at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as again
    Then again equals first

  @property
  Scenario: every name names something that is there
    Then every label of view names something view publishes
