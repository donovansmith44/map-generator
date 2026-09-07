Feature: changes — the narrative between two instants
  The scrubber's stops: the piecewise-constant timeline made visible.

  Scenario: the conquest is a change the timeline knows
    When I GET /api/changes?from=-1407&to=-1405
    Then the response equals fixture "changes-conquest"

  Scenario: an empty span has no changes
    When I GET /api/changes?from=-3000&to=-3000
    Then the response equals fixture "changes-empty"
