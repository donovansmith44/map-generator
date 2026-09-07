Feature: polities — the eras of governed ground we vendor
  Our parse_polities reads, per row: id, name, from, to, rings,
  color_key, and the verse lists nested under transition and fall
  (transition.verses and fall.verses). color_key and the two verse
  lists are read tolerantly — absent means "no colour key" and "no
  verses", not a parse failure — but they ARE consumed, so a change to
  any of them changes what we render.
  The consumed projection of the whole book is pinned — all rows, all
  coordinates. A silently moved border fails here before it can move
  a pixel of ours.

  Vocabulary:
    | projection | any of: eras, event, land-mask, landmarks, narratives, polities |

  Scenario: the whole polity book, as we consume it
    When I GET /api/polities?from=-4004&to=2000
    Then the consumed projection polities equals fixture "polities-consumed"
