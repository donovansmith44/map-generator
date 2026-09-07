Feature: eras — the named periods that resolve standings
  Our parse_eras reads id and from_year per era; era ids are how
  vendored data declares WHO STANDS WHEN without hardcoded years.

  Vocabulary:
    | projection | any of: eras, event, land-mask, landmarks, narratives, polities |

  Scenario: the whole era table, as we consume it
    When I GET /api/eras
    Then the consumed projection eras equals fixture "eras-consumed"
