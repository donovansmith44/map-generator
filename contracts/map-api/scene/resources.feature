Feature: resources — geometry by content address
  An id IS its bytes: the same id can never serve two payloads, and a
  batch is exactly its singles.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |

  @property
  Scenario: the same geometry fetched twice is exactly the same bytes
    When I render pieces <somePieces> at year <someYear> in style <someStyle> as scene
    Then fetching scene's first resource twice yields identical bytes

  @target @property
  Scenario: fetching geometry in a batch is the same as fetching it one at a time
    When I render pieces <somePieces> at year <someYear> in style <someStyle> as scene
    Then fetching scene's first two resources as a batch equals fetching them singly
