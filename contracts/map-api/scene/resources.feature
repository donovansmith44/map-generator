Feature: resources — geometry by content address
  An id IS its bytes: the same id can never serve two payloads, and a
  batch is exactly its singles.

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC) |
    | style | any of: canaan, parchment, slate |

  Scenario: the same id fetched twice is byte-identical
    When I render pieces fills, borders at year -1405 in style canaan as scene
    Then fetching scene's first resource twice yields identical bytes

  @target
  Scenario: a batch equals its singles
    When I render pieces fills, borders at year -1405 in style canaan as scene
    Then fetching scene's first two resources as a batch equals fetching them singly
