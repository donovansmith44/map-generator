Feature: naming — one thing, one name, one label
  A region is uniquely named: no two regions in a view carry the same
  name, so a promised allotment and the kingdom that later holds it are
  one entity and one label, not two that happen to share a word. A place
  is named once, however many roads pass through it — its identity is the
  town on the ground, not the count of journeys that visit it. Two
  genuinely different places may share a name; the same place drawn twice
  is the fault this feature catches.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |

  @property
  Scenario: no two regions in a view share a name
    When I render pieces all at year <someYear> in style <someStyle> as view
    Then no two region labels of view carry the same name

  @property
  Scenario: a place is named once, however many roads pass through it
    When I render pieces all at year <someYear> in style <someStyle> as view
    Then no place of view is labeled more than once
