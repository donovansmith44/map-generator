Feature: classification fixture
  Fixture for the @target-versus-hard-red classification tests in Spec.hs.
  Do not "fix" these scenarios to pass — the first is meant to fail plain,
  the second is meant to fail as an expected @target red, and the third
  is meant to pass as a met @target.

  Scenario: hard red
    When I GET /api/nothing
    Then the response field missing equals nope

  @target
  Scenario: expected red
    When I GET /api/nothing
    Then the response field missing equals nope

  @target
  Scenario: target already met
    When I GET /api/ok
    Then the response field ok equals yes
