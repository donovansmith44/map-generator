Feature: the golden gate — an instrument that can fail, and always says which way
  The gate stands between a refactor and the maps the owner loves, so
  it answers to the standard it enforces. It must be able to fail. It
  must fail for a named reason rather than by hanging, by drifting, or
  by quietly looking at nothing. And it must give the same answer twice
  about the same map — the defect that started this file was an
  instrument that answered HOLD, then REGRESSION, then REGRESSION
  again, no two failing sets alike, against a binary that never
  changed. An instrument nobody can trust teaches its reader to stop
  looking, which is worse than having no instrument at all.

  Vocabulary:
    | camera | any of: levant, hemisphere |
    | probe | whole number from 0 to 24 |

  @property
  Scenario: the same map judged twice gives the same verdict
    When I judge the unchanged map as first
    And I judge the unchanged map as second
    Then first and second are the same verdict, drift for drift

  @property
  Scenario: a change to the picture is caught, wherever it lands
    When I repaint probe <someProbe> of <someCamera> in the drawn map
    And I judge the repainted map as judged
    Then judged reports drift at probe <someProbe> of <someCamera>, and nowhere else

  @property
  Scenario: a change the eye would not see is not called a regression
    When I repaint probe <someProbe> of <someCamera> by less than the gate's tolerance
    And I judge the repainted map as judged
    Then judged holds

  Scenario: a map that never settles is refused, not waited on forever
    When I judge a map that never stops moving
    Then the gate stops and says the view would not hold still

  Scenario: a renderer that has died is called dead, not called drift
    When I judge a map whose renderer has died
    Then the gate stops and says the renderer is down

  Scenario: a blank map and a map that never arrived are different answers
    When I judge a map that draws nothing as blank
    And I judge a map that never loaded as absent
    Then blank and absent are different verdicts

  Scenario: a view that will not hold still is never blessed
    When I bless against a map that never stops moving
    Then nothing is written and the gate says the view would not hold still

  Scenario: every stop the baseline knows is a stop the gate judged
    When I judge a map that offers fewer stops than the baseline holds
    Then the gate stops and says which stops it never saw
