Feature: label placement — the map says where every name sits, not the viewer
  Where each name sits on the page is part of the map. A label arrives
  with the position it is drawn at, already settled, already clear of
  its neighbours: two viewers given the same answer draw the same
  names in the same places, because there is nothing left to decide
  between the answer and the ink.

  Today a label arrives with an anchor — where the named thing is —
  and the viewer works out where the words go, remembering each offset
  so the text does not jitter, and forgetting it when the name is
  culled. That memory makes the drawn map a function of its own
  history: the same year, at the same camera, on an unchanged server,
  draws a different set of names depending on whether you had been out
  to the globe and come back. The three scenarios that fail here are
  the specification for moving that decision into the answer; the
  two that pass are the door closing behind it.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |

  Background:
    When I render pieces all at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as view

  @target @property
  Scenario: every name on the map says where it sits
    Then every label of view carries a placement

  @target @property
  Scenario: no two names are printed on top of each other
    Then no two labels of view overlap

  @target @property
  Scenario: every name that is sent is a name that is drawn
    Then every label of view is legible at the view it was asked for

  @property
  Scenario: asking for other maps in between changes nothing
    When I render pieces all at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as first
    And I render pieces all at year <someOtherYear> in style <someOtherStyle> looking at <someOtherCenter> zoom <someZoom> as detour
    And I render pieces all at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as again
    Then again equals first

  @property
  Scenario: every name names something that is there
    Then every label of view names a feature of view
