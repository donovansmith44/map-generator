Feature: polities — the eras of governed ground we vendor
  Our parse_polities reads: per row, id, name, from, to, and rings.
  The consumed projection of the whole book is pinned — all rows, all
  coordinates. A silently moved border fails here before it can move
  a pixel of ours.

  Scenario: the whole polity book, as we consume it
    When I GET /api/polities?from=-4004&to=2000
    Then the consumed projection polities equals fixture "polities-consumed"
