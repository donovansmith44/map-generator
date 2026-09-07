Feature: landmarks — named waters and places we label by
  Our parse_landmarks reads four fields per row: name, kind, lat, and
  lon. The position is consumed, not just the naming — a landmark that
  moves moves our label with it.

  Vocabulary:
    | projection | any of: eras, event, land-mask, landmarks, narratives, polities |

  Scenario: the whole landmark list, as we consume it
    When I GET /api/landmarks
    Then the consumed projection landmarks equals fixture "landmarks-consumed"
