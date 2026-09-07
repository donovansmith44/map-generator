Feature: subjects — what can be asked about at a moment
  The picker's feed: enumeration precedes every question. The WHOLE
  list is the answer — a subjects feed that also carried a leaked era's
  ghost would pass any existential poke; it cannot pass the fixture.

  Scenario: the twelve tribes era, whole
    When I GET /api/subjects?year=-1405
    Then the response equals fixture "subjects-1405"

  Scenario: the tetrarchies era, whole
    When I GET /api/subjects?year=59
    Then the response equals fixture "subjects-59"

  @property
  Scenario: subjects are deterministic at any year
    When I GET /api/subjects?year=<someYear> as first
    And I GET /api/subjects?year=<someYear> as second
    Then first equals second
