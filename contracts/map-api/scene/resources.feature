Feature: resources — geometry by content address
  An id IS its bytes: the same id can never serve two payloads, and a
  batch is exactly its singles.

  Scenario: the same id fetched twice is byte-identical
    When I render pieces fills, borders at year -1405 in style canaan as scene
    Then fetching scene's first resource twice yields identical bytes

  @target
  Scenario: a batch equals its singles
    When I render pieces fills, borders at year -1405 in style canaan as scene
    Then fetching scene's first two resources as a batch equals fetching them singly
