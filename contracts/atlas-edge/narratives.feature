Feature: narratives — the journeys we vendor
  Our parse_narratives reads: per row, id, name, color, and ordered legs.

  Vocabulary:
    | projection | any of: eras, event, land-mask, landmarks, narratives, polities |

  Scenario: the whole narrative book, as we consume it
    When I GET /api/narratives
    Then the consumed projection narratives equals fixture "narratives-consumed"
