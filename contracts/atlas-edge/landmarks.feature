Feature: landmarks — named waters and places we label by
  Our parse_landmarks reads name and kind per row.

  Scenario: the whole landmark list, as we consume it
    When I GET /api/landmarks
    Then the consumed projection landmarks equals fixture "landmarks-consumed"
