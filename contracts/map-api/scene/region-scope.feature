Feature: the scene's regions — checked as parts of the whole, not assumed to be
  Asking for one region by name is supposed to be a narrower question than
  asking for the world — the code that answers it calls the narrowing a
  monoid. Only labels actually narrow today: two regions asked for
  together answer with exactly the labels either would answer with alone.
  Borders, claims, and fills do not narrow at all — a single region draws
  the whole map's worth of them, the same set the world itself draws — and
  water, ground, and journeys do not appear for a region at all, regardless
  of what the caller asks for. This feature pins today's answer to which
  is which, so a later change to any of them is a visible law, not a
  silent regression.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |

  @property
  Scenario: two regions asked for together answer with exactly the labels either asked for alone
    When I render pieces all at year <someYear> in style <someStyle> as world
    Then combining world's first and second regions equals asking for both together

  @property
  Scenario: a single region still draws the whole map's borders, claims and fills
    When I render pieces all at year <someYear> in style <someStyle> as world
    Then world's first region draws the whole map's borders, claims and fills

  @property
  Scenario: water, ground and journeys never reach a single region
    When I render pieces all at year <someYear> in style <someStyle> as world
    Then world's first region draws no water, ground or journeys features
