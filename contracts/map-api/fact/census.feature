Feature: the census — every disposition, queryable
  The instrument whose absence let a phantom state ship: for any year,
  every standing feature's tenure in one table, pinned WHOLE. The
  promise-as-claim at 1050 BC and Judea-as-held at AD 59 live inside
  these fixtures — and so does everything else standing in those
  years, which is the point: a leaked feature fails the fixture.

  Scenario: the whole census at 1050 BC
    When I GET /api/census?year=-1050
    Then the response equals fixture "census-1050"

  Scenario: the whole census at AD 59
    When I GET /api/census?year=59
    Then the response equals fixture "census-59"

  Scenario: the whole census at the conquest
    When I GET /api/census?year=-1405
    Then the response equals fixture "census-1405"

  @property
  Scenario: the census is deterministic at any year
    When I GET /api/census?year=<someYear> as first
    And I GET /api/census?year=<someYear> as second
    Then first equals second
