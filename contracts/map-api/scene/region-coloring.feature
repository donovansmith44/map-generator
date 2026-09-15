Feature: region coloring — two territories that touch never wear the same paint
  A viewer tells territories apart by color alone; two that touch and
  match read as one. The promise is made once, at load, over every
  territory's whole lifetime rather than the map any single year
  actually shows — this law is checked against the real, live answer,
  not the promise.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |

  @property
  Scenario: no two touching fills share a style
    When I render pieces all at year <someYear> in style <someStyle> as world
    Then no two touching fills of world share a style
